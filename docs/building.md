# Native libraries and source builds

[← README](../README.md)

Elegance's Rust bindings link to the native [Elegance](https://github.com/EleganceUI/elegance)
framework. Linkage depends on the build path, not just whether the `dynamic` feature is enabled:

| Build path | Elegance linkage | Runtime files |
| --- | --- | --- |
| Downloaded prebuilt (default without source overrides or a sibling checkout) | Static C API and Elegance archives linked into the Rust executable | Shared dependencies such as AccessKit; no Elegance shared library. |
| Source build (default features) | Static Elegance library | Any shared dependencies listed by the native build. |
| Source build with `dynamic` (including previews) | Shared Elegance library | Elegance and its shared dependencies. |

Static Elegance linkage does **not** imply a fully static, single-file executable.

## Prebuilt libraries

By default, `elegance-sys` downloads a package containing static `elegance_c`, Elegance, and
native dependency archives, plus a relocatable `elegance-link.txt` manifest. Cargo links these
archives into the final Rust executable in manifest order; the archives are not runtime files
and do not need to ship with the application. This path requires `curl` and `tar`, but no C++
compiler, CMake, or vcpkg.

The manifest separately lists shared runtime dependencies. AccessKit remains shared because
its Rust static library includes a second Rust standard library that conflicts with the
application's. System libraries and frameworks also remain platform dependencies.

The native revision is pinned in [`elegance-sys/elegance-rev`](../elegance-sys/elegance-rev).
Elegance's CI publishes archives for each commit on `main` as a `c-<commit>` prerelease, and
the build script verifies that the archive matches the pinned commit. Downloads are cached
under `elegance/c-static/` in the user cache directory and shared across projects.

Static packages are named `elegance-c-static-<target>.tar.gz` and include `linkage static` in
`BUILDINFO`. Old shared packages are not accepted or silently linked dynamically. Until a static
package is published for the pinned native revision, the normal build falls back to a static
source build; `ELEGANCE_PREBUILT=1` instead fails. Updating the pin to a newly published static
package is required to enable the download-only path for that revision.

| Environment variable | Purpose |
| --- | --- |
| `ELEGANCE_PREBUILT=0` | Disable downloads and build from source. |
| `ELEGANCE_PREBUILT=1` | Require a prebuilt library, even beside an Elegance checkout; fail if unavailable. |
| `ELEGANCE_NATIVE_DIR` | Use an extracted archive, for example for an offline build. |
| `ELEGANCE_PREBUILT_URL` | Use a base URL (`https://` or `file://`) containing `elegance-c-static-<target>.tar.gz`. |

Previews use the `dynamic` feature and build Elegance from source.

### Distributing an application

Ship only the shared runtime dependencies copied beside your executable, not the static
archives. Neither `elegance_c` nor Elegance requires a shared library in default builds.
Linux executables need an rpath for the remaining shared dependencies of `$ORIGIN` (`-C link-args=-Wl,-rpath,$ORIGIN`) or an appropriate `LD_LIBRARY_PATH`.
Prebuilt libraries require glibc 2.35+ on Linux and macOS 13.3+.

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
