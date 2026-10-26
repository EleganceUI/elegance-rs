# The prebuilt native library: the shim and a static Elegance, with all of Elegance's dependencies,
# in one shared library that exports only the C ABI (shim.h and ELG_API's C functions). Rust links
# it with no C++ compiler, CMake, or vcpkg: CI (.github/workflows/native.yml) builds it per target
# from the Elegance commit elegance-rev pins, attaches it to a prerelease named after the hash of
# the shim and that pin, and build.rs downloads it (build_prebuilt.rs).
#
#   cmake -S elegance-sys/native -B build -G Ninja -DELEGANCE_ROOT=<elegance checkout> \
#         -DCMAKE_TOOLCHAIN_FILE=$VCPKG_ROOT/scripts/buildsystems/vcpkg.cmake \
#         -DVCPKG_MANIFEST_DIR=<elegance checkout> -DVCPKG_TARGET_TRIPLET=<triplet> \
#         -DCMAKE_BUILD_TYPE=Release
#   cmake --build build
#   cmake --install build --prefix <dir>
#
# Windows triplets must be *-windows-static: the library carries its own C runtime (/MT), so apps
# need no Visual C++ redistributable. On Linux it links libstdc++ statically, so it only needs glibc.
cmake_minimum_required(VERSION 4.3)
project(elegance_native LANGUAGES CXX)

set(ELEGANCE_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../elegance" CACHE PATH "The Elegance source tree")
if (NOT EXISTS "${ELEGANCE_ROOT}/CMakeLists.txt")
    message(FATAL_ERROR "No Elegance source at ELEGANCE_ROOT (${ELEGANCE_ROOT})")
endif ()

if (MSVC)
    if (NOT VCPKG_TARGET_TRIPLET MATCHES "-static$")
        message(FATAL_ERROR "Use a *-windows-static vcpkg triplet (got ${VCPKG_TARGET_TRIPLET})")
    endif ()
    # Set here, not only in Elegance's directory, so the shim uses the same runtime.
    set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded")
endif ()
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

set(ELG_BUILD_SHARED OFF CACHE BOOL "" FORCE)
set(ELG_STATIC_EXPORTS ON CACHE BOOL "" FORCE)
# Inside a shared library that hides its symbols, AccessKit's Rust standard library can't clash
# with the app's, so its static library is fine (and keeps this one file).
set(ELG_ACCESSKIT_SHARED OFF CACHE BOOL "" FORCE)
add_subdirectory("${ELEGANCE_ROOT}" elegance)

set(shim "${CMAKE_CURRENT_SOURCE_DIR}/../shim")
add_library(elegance_native SHARED
        ${shim}/core.cpp
        ${shim}/views.cpp
        ${shim}/ext.cpp
        ${shim}/platform.cpp
)
# The whole archive, so ELG_API functions the shim doesn't call (elg_gpu_image_*) are exported too.
target_link_libraries(elegance_native PRIVATE "$<LINK_LIBRARY:WHOLE_ARCHIVE,elegance>")
target_compile_definitions(elegance_native PRIVATE ELG_SHIM_SHARED)
set_target_properties(elegance_native PROPERTIES
        CXX_VISIBILITY_PRESET hidden
        VISIBILITY_INLINES_HIDDEN ON
)
if (MSVC)
    target_compile_definitions(elegance_native PRIVATE NOMINMAX)
    target_compile_options(elegance_native PRIVATE /EHsc /permissive- /utf-8 /Zc:__cplusplus)
elseif (APPLE)
    set_target_properties(elegance_native PROPERTIES INSTALL_NAME_DIR "@rpath")
    # Export only the C ABI: the dependencies (Abseil, AccessKit's Rust standard library, ...)
    # aren't built with hidden visibility. Leave room for build.rs to rewrite the install name.
    target_link_options(elegance_native PRIVATE
            "-Wl,-exported_symbol,_elg_*" -Wl,-headerpad_max_install_names -Wl,-dead_strip)
else ()
    # Carry libstdc++ and libgcc; export only the C ABI.
    target_link_options(elegance_native PRIVATE
            -static-libstdc++ -static-libgcc "-Wl,--version-script=${CMAKE_CURRENT_SOURCE_DIR}/exports.map"
            -Wl,--gc-sections -Wl,-z,defs)
    set_property(TARGET elegance_native APPEND PROPERTY LINK_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/exports.map")
endif ()

include(GNUInstallDirs)
install(TARGETS elegance_native
        RUNTIME DESTINATION lib
        LIBRARY DESTINATION lib
        ARCHIVE DESTINATION lib)
# Dawn's Direct3D 12 backend loads the DirectX shader compiler at run time.
if (WIN32)
    install(FILES
            "${VCPKG_INSTALLED_DIR}/${VCPKG_TARGET_TRIPLET}/bin/dxcompiler.dll"
            "${VCPKG_INSTALLED_DIR}/${VCPKG_TARGET_TRIPLET}/bin/dxil.dll"
            DESTINATION lib OPTIONAL)
endif ()
install(FILES "${ELEGANCE_ROOT}/LICENSE" DESTINATION . RENAME LICENSE-elegance)

# What build.rs checks before using an archive: SHA-256 of the shim sources and elegance-rev (the
# pinned Elegance commit), without carriage returns, which build_prebuilt.rs hashes the same way.
# The prerelease tag is native-<its first 12>.
set(native_text "")
foreach (file IN ITEMS ${shim}/shim.h ${shim}/core.cpp ${shim}/views.cpp ${shim}/ext.cpp ${shim}/platform.cpp
                       ${CMAKE_CURRENT_SOURCE_DIR}/elegance-rev)
    file(READ "${file}" text)
    string(REPLACE "\r" "" text "${text}")
    string(APPEND native_text "${text}")
    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${file}")
endforeach ()
string(SHA256 native_hash "${native_text}")
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/native.sha256" "${native_hash}\n")
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/native.sha256" DESTINATION .)

# The hash claims the pinned Elegance, so check ELEGANCE_ROOT is that commit (CI requires it).
option(ELG_NATIVE_REQUIRE_PIN "Fail unless ELEGANCE_ROOT is the commit elegance-rev pins" OFF)
file(STRINGS "${CMAKE_CURRENT_SOURCE_DIR}/elegance-rev" pinned_rev LIMIT_COUNT 1)
find_package(Git QUIET)
if (GIT_FOUND)
    execute_process(COMMAND "${GIT_EXECUTABLE}" -C "${ELEGANCE_ROOT}" rev-parse HEAD
                    OUTPUT_VARIABLE elegance_rev OUTPUT_STRIP_TRAILING_WHITESPACE ERROR_QUIET)
endif ()
if (NOT elegance_rev STREQUAL pinned_rev)
    set(mismatch "ELEGANCE_ROOT is at '${elegance_rev}', but elegance-rev pins ${pinned_rev}")
    if (ELG_NATIVE_REQUIRE_PIN)
        message(FATAL_ERROR "${mismatch}")
    endif ()
    message(WARNING "${mismatch}: the library's hash will claim the pinned commit. Fine for a local "
                    "build used through ELEGANCE_NATIVE_DIR; don't publish it.")
endif ()
