# Native libraries and source builds

[← README](../README.md)

elegance-sys links the native [Elegance](https://github.com/EleganceUI/elegance) framework
through its C API. How it links depends on where Elegance comes from:

| Build path | Elegance linkage | Files to ship beside the executable |
| --- | --- | --- |
| Prebuilt package (the default) | Static: one archive linked into the executable | AccessKit's shared library (and, on Windows, Dawn's shader compiler DLLs if present) |
| Source build | Static Elegance library | Any shared dependencies the native build lists |
| Source build with the `dynamic` feature (previews) | Shared Elegance library | Elegance and its shared dependencies |

Cargo copies the files to ship next to your binaries in `target/<profile>/`. System libraries and
frameworks remain ordinary platform dependencies, so a static Elegance doesn't make a fully
static, single-file executable.

## Prebuilt libraries

By default, elegance-sys downloads Elegance's static package for your target. It holds:

- `lib/libelegance_c.a` (`elegance_c.lib` on Windows): one archive that merges the C API,
  Elegance, and every static dependency, Dawn included
- the shared libraries that stay separate: AccessKit's, because its static library carries its
  own copy of the Rust standard library, which would clash with your application's
- `elegance-link.txt`: what linking the archive takes (system libraries and frameworks, and the
  shared libraries above), with paths relative to the package

Cargo links the archive into the final executable. The archive is not a runtime file and
doesn't ship with your application. This path needs `curl` and `tar`, but no C++ compiler,
CMake, or vcpkg.

The native revision is pinned in [`elegance-sys/elegance-rev`](../elegance-sys/elegance-rev).
Elegance's CI builds the package for every commit on `main` and publishes it as
`elegance-c-static-<target>.tar.gz` in a `c-<commit>` prerelease. The build script checks that
the package's `BUILDINFO` names the pinned commit and `linkage static`, then caches it under
`elegance/c-static/` in the user cache folder, shared by all your projects.

If no static package is published for the pinned revision and target, the build falls back to a
source build; with `ELEGANCE_PREBUILT=1`, it fails instead.

| Environment variable | Purpose |
| --- | --- |
| `ELEGANCE_PREBUILT=0` | Disable downloads and build from source. |
| `ELEGANCE_PREBUILT=1` | Require a prebuilt library, even beside an Elegance checkout; fail if unavailable. |
| `ELEGANCE_NATIVE_DIR` | Use an extracted archive, for example for an offline build. |
| `ELEGANCE_PREBUILT_URL` | Use a base URL (`https://` or `file://`) containing `elegance-c-static-<target>.tar.gz`. |

Previews use the `dynamic` feature and build Elegance from source.

To build the static package yourself, for example for an unsupported target or a modified
Elegance, configure Elegance's `c/` project with `ELG_C_STATIC`, then point
`ELEGANCE_NATIVE_DIR` at the installed result:

```sh
cmake -S c -B build/c-static -G Ninja -DCMAKE_BUILD_TYPE=Release -DELG_C_STATIC=ON \
      -DCMAKE_TOOLCHAIN_FILE=$VCPKG_ROOT/scripts/buildsystems/vcpkg.cmake -DVCPKG_MANIFEST_DIR=.
cmake --build build/c-static
cmake --install build/c-static --prefix build/c-static-dist
```

On Windows, add `-DVCPKG_TARGET_TRIPLET=x64-windows-static-md`: the archive must use the shared
C runtime (`/MD`), as Rust does.

### Distributing an application

Ship the executable together with the shared libraries Cargo copied beside it (AccessKit's).
There is no Elegance shared library to ship.

On Linux, the executable needs `$ORIGIN` in its rpath (`-C link-args=-Wl,-rpath,$ORIGIN`) to
find them, or an appropriate `LD_LIBRARY_PATH`. The Linux package also links libstdc++
statically, so apps need only glibc 2.35 or newer. On macOS the prebuilt libraries need macOS
13.3 or newer.

On macOS, `elegance-sys` stages dylibs with loader-relative install names and ad-hoc signs the
staged copies before linking and copying them beside Cargo's binaries. Executables can run
directly without an application build script, an rpath, or `DYLD_LIBRARY_PATH`. Keep the dylibs
beside the executable when moving it. The original CMake build is not modified; distribution
still requires your own signing and notarization.

## Building from source

A source build is selected when `ELEGANCE_ROOT` or `ELEGANCE_BUILD_DIR` is set, an Elegance
checkout is present beside this repository, or no suitable prebuilt library is available.
You can also request it explicitly with `ELEGANCE_PREBUILT=0`.

### Requirements

- **Windows:** Visual Studio's C++ workload.
- **macOS:** Xcode 26.
- **Linux:** GCC 14+; set `CC=gcc-14 CXX=g++-14` if needed.
- **All platforms:** Git. The build locates or downloads the remaining tools.

The first build compiles Elegance and its C++ dependencies with CMake and vcpkg. Allow
roughly half an hour if vcpkg needs to compile Dawn. Run `cargo build -vv` to see progress;
subsequent builds reuse the native build.

Source builds link Elegance statically by default. Applications still need the shared
libraries supplied by Elegance's dependencies, which are copied beside the executable.
The `dynamic` feature links the shared Elegance library instead; the preview host and views
module use it to share the same native runtime.

### Source selection

The build script locates Elegance's source in this order:

1. `ELEGANCE_ROOT`, if set.
2. A sibling checkout at `../elegance`.
3. A cached clone of `EleganceUI/elegance` at the pinned commit.

Cached clones live under `%LOCALAPPDATA%\elegance\src\<commit>` on Windows or
`~/.cache/elegance/src/<commit>` on Linux. `ELEGANCE_GIT_REF` overrides the revision with a
commit, branch, or tag. Branch clones are not automatically updated; delete the cached clone
to fetch a newer revision.

### Build selection

Once the source is located, the build script uses the first available build:

| Priority | Default (static) | `dynamic` feature |
| --- | --- | --- |
| 1 | `ELEGANCE_BUILD_DIR` | `ELEGANCE_SHARED_BUILD_DIR` |
| 2 | `build/dev-release-static` in the source tree | `build/dev-release` in the source tree |
| 3 | Automatically build in `build/cargo-release-static` | Automatically build in `build/cargo-release` |

Automatic builds are refreshed when Elegance's sources change. Set `ELEGANCE_AUTO_BUILD=0`
to report a missing build instead of creating one. Build selection happens when Cargo runs
the build script: after creating a native build of your own, run `cargo clean -p elegance-sys`
to switch to it.

### Tool discovery

The build uses the first vcpkg installation with the baseline pinned by Elegance's `vcpkg.json`:

1. `VCPKG_ROOT`.
2. The `VCPKG_ROOT` configured in Elegance's `CMakeUserPresets.json`.
3. `vcpkg` on `PATH`.
4. A bootstrapped clone in `ELEGANCE_VCPKG_DIR` (default: `elegance/vcpkg` in the user cache directory).

CMake 4.3+ and Ninja are found through `CMAKE` / `NINJA`, `PATH`, Visual Studio's bundled tools,
or `vcpkg fetch`. The MSVC environment is detected without a developer command prompt.

### Custom native builds

On Windows, use the release C runtime (`/MD`), not the debug runtime (`/MDd`), to match Rust.
The C API passes `std::string` and `std::function` to Elegance, so runtime compatibility matters.
Static builds also require `-DELG_ACCESSKIT_SHARED=ON`: AccessKit's static library is a Rust
`staticlib`, whose bundled Rust standard library would conflict with the application's.

## Updating the native revision

The [`bump-elegance` workflow](../.github/workflows/bump-elegance.yml) opens or updates a pull
request when Elegance publishes a newer commit's library. The pull request updates the pin and
runs the Rust checks. If the C API changed, add the corresponding Rust changes to that branch.

Applications using the Git dependency can pick up a merged update with `cargo update -p elegance`.
A release retains the native revision it pins.
