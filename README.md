# elegance-rs

Rust bindings to [Elegance](https://github.com/EleganceUI/elegance), with `elg!`, a SwiftUI-style
DSL. Examples are in [elegance-examples](https://github.com/EleganceUI/elegance-examples)
(the `*-rs` folders).

```rust
use elegance::prelude::*;

#[component]
fn Counter() -> impl IntoView {
    let mut count = State::new(0);
    elg! {
        VStack(spacing: 16) {
            Text("{count}").font(Font::system(56.0, .bold))
            HStack {
                Button("−", move || count -= 1)
                Button("+", move || count += 1)
            }
        }
    }
}

fn main() -> Result<std::process::ExitCode, Error> {
    App::new().window(WindowDesc::new("Counter").size(360, 280), Counter).run()
}
```

## Crates

- `elegance-sys`: the raw FFI to Elegance's C API, which lives in Elegance (`c/`).
- `elegance`: the safe API: views, modifiers, reactive state, environment, styles, layouts, custom
  modifiers, effects, gestures, animation, media, GPU images, desktop, previews. Elegance has no
  network API: use a Rust HTTP or WebSocket crate and hand results to the UI with
  `post_to_ui_thread`.
- `elegance-macros`: `elg!`, `#[component]`, and `#[preview]`.
- `cargo-elegance`: `cargo elegance preview`, the preview host runner.
- `elegance-wgpu`: native Rust wgpu → Elegance GPU-image sharing, on wgpu 30.0.1 from
  crates.io. Metal is hardware-tested here, while DX12/Vulkan still require platform
  validation.

## Using it

```toml
[dependencies]
elegance = { git = "https://github.com/EleganceUI/elegance-rs" }
```

On Linux (x86_64, ARM64), macOS (Apple Silicon, Intel), and Windows (x64, ARM64, x86), the build
downloads a prebuilt Elegance: one shared library (`elegance_c`) holding Elegance, its
dependencies, and its C API, which Cargo copies next to your binaries. You need no C++ compiler,
CMake, or vcpkg, only `curl` and `tar` (built into Windows 10+, macOS, and Linux distributions).
Ship the library beside your executable; on Linux, also give the executable an rpath of `$ORIGIN`
(`-C link-args=-Wl,-rpath,$ORIGIN`), or set `LD_LIBRARY_PATH`. The library needs glibc 2.35+ on
Linux and macOS 13.3+.

The library is the one Elegance publishes for the commit pinned in `elegance-sys/elegance-rev`:
Elegance's CI builds it for every commit on its `main` and attaches it to a `c-<commit>` prerelease
there (Elegance's `.github/workflows/c-api.yml`). The build script checks the archive was built
from that commit. When there's none (a target without prebuilts), it builds Elegance from source
as described below.

Elegance improvements reach the bindings through pull requests: once Elegance publishes a newer
commit's library, `.github/workflows/bump-elegance.yml` opens (or updates) one that moves the pin,
and the tests run on it. If the C API changed, push the Rust side to that pull request's branch.
With a git dependency, `cargo update -p elegance` picks up a merged pin; a release keeps the
Elegance it pinned.

| Variable | Effect |
|---|---|
| `ELEGANCE_PREBUILT=0` | never download: build from source |
| `ELEGANCE_PREBUILT=1` | always use a prebuilt library, even beside an Elegance checkout; fail without one |
| `ELEGANCE_NATIVE_DIR` | an extracted archive to use (offline builds, mirrors) |
| `ELEGANCE_PREBUILT_URL` | a folder (`https://`, `file://`) holding `elegance-c-<target>.tar.gz` |

Downloads are cached in the user cache folder (`elegance/c/`) and shared by every project.
Previews (the `dynamic` feature) still build Elegance from source.

### Building from source

The first source build compiles Elegance itself (C++, with CMake and vcpkg), which takes a while:
about half an hour when vcpkg has to build Dawn. `cargo build -vv` shows its progress. Later builds
reuse it. You need Visual Studio's C++ workload, Xcode 26 on macOS, or GCC 14+ on Linux (set
`CC=gcc-14 CXX=g++-14` where it isn't the default), and git; the rest is found or downloaded.

From source, Elegance is linked statically by default: an app is one executable plus the DLLs
Elegance's dependencies come in, which are copied next to it. The `dynamic` feature links
`elegance.dll` instead, which previews need.

### Where Elegance comes from

A source build happens when `ELEGANCE_ROOT` or `ELEGANCE_BUILD_DIR` is set, when there's an
Elegance checkout beside this repository, or when no prebuilt library fits. It needs Elegance's
source:

1. `ELEGANCE_ROOT`, if set;
2. a checkout beside this repository (`../elegance`), for working on both;
3. otherwise a clone of `EleganceUI/elegance` at the pinned commit, in the user cache folder
   (`%LOCALAPPDATA%\elegance\src\<commit>`, `~/.cache/elegance/src/<commit>`), made once and
   shared by every project. `ELEGANCE_GIT_REF` picks another commit, branch, or tag (a branch's
   clone isn't updated: delete its folder to get a newer one).

Then a build of it to link, in this order:

1. `ELEGANCE_BUILD_DIR` (static) / `ELEGANCE_SHARED_BUILD_DIR` (`dynamic`), if set;
2. `build/dev-release-static` / `build/dev-release` in the source tree, if built;
3. otherwise it builds one itself, into `build/cargo-release-static` (or `cargo-release`), and
   rebuilds it when Elegance's sources change. Set `ELEGANCE_AUTO_BUILD=0` to get an error instead.

For that build it uses the first vcpkg that has the baseline Elegance's `vcpkg.json` pins:
`VCPKG_ROOT`, the `VCPKG_ROOT` Elegance's `CMakeUserPresets.json` sets, `vcpkg` on PATH, or a clone
it bootstraps in `ELEGANCE_VCPKG_DIR` (default: the cache folder's `elegance/vcpkg`). CMake 4.3+
and Ninja come from `CMAKE`/`NINJA`, PATH, Visual Studio's bundled copies, or `vcpkg fetch`; MSVC's
environment is found without a developer prompt. The choice of build is made when the build script
runs: after building a tree of your own, `cargo clean -p elegance-sys` to switch to it.

A build of your own must use the release C runtime (`/MD`) like Rust, not a Debug build's `/MDd`
(the C API passes `std::string` and `std::function` to Elegance), and a static one needs
`-DELG_ACCESSKIT_SHARED=ON` (AccessKit's static library is a Rust staticlib, whose own Rust standard
library clashes with the app's).

On macOS, `elegance-sys` stages native dylibs with loader-relative install names and ad-hoc
signs the staged copies before linking and copying them beside Cargo's binaries. Executables
can run directly without an application build script, an rpath, or `DYLD_LIBRARY_PATH`. Keep
the dylibs beside the executable when moving it. The original CMake build is not modified;
distribution still requires your own signing and notarization.

### Working on the bindings

Check out [elegance](https://github.com/EleganceUI/elegance) beside this repository, and
[elegance-examples](https://github.com/EleganceUI/elegance-examples) to try changes; to build the
examples against this checkout instead of GitHub, add to `elegance-examples/.cargo/config.toml`:

```toml
[patch."https://github.com/EleganceUI/elegance-rs"]
elegance = { path = "../elegance-rs/elegance" }
```

## The `elg!` DSL

| DSL | Means |
|---|---|
| `Name(a, b)` | an element; overloads are tuple impls of `ElgNew` |
| `Name(a, key: v)` | the same, then the builder call `.key(v)` |
| `Name { ... }` | its children, on the element's line: a stack's views, a button's label, ... |
| `ForEach(xs, id: \|x\| x.id) { x in ... }` | a content closure per item (also `AsyncImage(url) { phase in ... }`) |
| `if cond { ... } else { ... }` | a reactive `If` |
| `match value { pat => View(), ... }` | the view for the current value, rebuilt when it changes (`Clone + PartialEq` values) |
| `.leading`, `.accent`, `.bold`, `.infinity` | a value of the parameter's type, by name (see below) |
| `"{count} left"` | text that updates when `count` changes |
| `"{size.0}x{size.1}"`, `"{user.name:>8}"`, `"{a + b}"` | fields of state (read reactively) and any expression in a placeholder |
| `{ expr }`, or a variable | any Rust value that is `IntoView` |
| `(layout) { ... }` | a custom `Layout` over the children |
| `.m(a, key: v)` | the modifier `.m_with(a, \|o\| o.key(v))`, e.g. `.frame(max_width: .infinity)` |

### Leading-dot names

`.name` is a value whose type spells the name, and it converts into whatever type has a value
by that name: `.leading` is `HorizontalAlignment::Leading` for a stack and `Alignment::LEADING`
for a frame. Any crate can add names. `#[derive(DotValues)]` on an enum makes its unit
variants names, in snake_case:

```rust
#[derive(Clone, Copy, PartialEq, DotValues)]
enum Density { Compact, Comfortable, ExtraRoomy }

#[component]
fn Row(title: String, #[prop(default = .comfortable)] density: Density) -> impl IntoView { ... }

elg! { Row(title: "Inbox".into(), density: .extra_roomy) }
```

The names work wherever the parameter is a `Density`, an `impl Into<Density>`, or an
`impl IntoProp<Density>`, including on your own structs' builder methods with no attribute.
A parameter of exactly `Density` (not `impl Into<..>`) also works when the method's impl has
`#[elg_builder]`, as `#[component]` props always do. A name the type doesn't have is a compile
error at that name.

### Named arguments

On an element, `Name(a, key: v)` calls the builder method `.key(v)`: `#[component]` makes one
per prop. On a modifier, `.m(a, key: v)` calls `.m_with(a, |o| o.key(v))`, so the names are the
builder methods of the options struct `m_with` takes: `.popover(editing, content, edge: .top)`
is `.popover_with(editing, content, |o| o.edge(.top))`, and `edge` is `PopoverOptions::edge`. Your own modifiers get named arguments the same way: add a `name_with`
method taking `impl FnOnce(Options) -> Options`.

### Custom components and completion

Use `#[component]` on a function, or implement `Component` for a struct. A prop of type
`Prop<T>` is reactive: it takes anything a reactive argument takes (a value, a `State`, a
closure, `"{interpolated}"` text). A `Binding<T>` prop is two-way: pass the parent's `State`.
`components-rs` in elegance-examples shows every kind of prop. On a struct's
builder impl, `#[elg_builder(constructor)]` registers `fn new() -> Self` with `elg!`
and enables typed dot completion; no handwritten `ElgNew<()>` impl is needed.
Use plain `#[elg_builder]` for completion only, including on extension traits.

With rust-analyzer proc macros enabled, `VStack(alignment: .…)` suggests horizontal
alignments, while `.font(.…)` suggests fonts, and a `#[derive(DotValues)]` enum's prop suggests
its variants. Imports and aliases work too.

Property prefixes such as `VStack(spa…)` and `.frame(wid…)` also complete, but may
insert Rust method syntax (`spacing(…)`); change it to the DSL's `spacing: …`.

## Grids

`VGrid`/`HGrid` flow children into fixed, flexible, or adaptive tracks; `Grid` is a table of
`GridRow`s (see `grids-rs` in elegance-examples, and Elegance's Grids guide):

```rust
VGrid([GridItem::adaptive(150)], spacing: 12) {
    ForEach(games, id: |g| g.id) { game in Card(game: game.clone()) }
}
```

## Images

Put the app's own images in `<crate>/assets/` and compile them in with `asset!`, which works
wherever the views run (the app, a preview, an installed copy):

```rust
Image(asset!("images/logo.png")).resizable().frame(96, 96)
```

For images that only exist at runtime (files the user picks, downloads), use
`ImageSource::file(path)` with an absolute path: Elegance resolves relative paths against the
current directory.

## Previews and hot reload

Mark functions with `#[preview]` anywhere in an app; no library/binary split is needed:

```rust
#[preview("Counter")]
fn counter() -> impl IntoView {
    elg! { Counter() }
}

#[preview(root)] // the whole app, for `--root`
fn app() -> impl IntoView {
    elg! { Root() }
}
```

Then run the preview host, which rebuilds and reloads the views as you edit:

```sh
cargo install --git https://github.com/EleganceUI/elegance-rs cargo-elegance
cargo elegance preview            # the previews
cargo elegance preview --root     # the root view
cargo elegance preview -p my-app --bin my-app --release
```

`cargo elegance` builds the crate (its binary, or its library if it has no binary) a second time
as a views module: a cdylib, with `elegance/preview` enabled, in `target/elegance-preview`. That
build links the shared Elegance (`dynamic`), which the module and the host share. Without the
`preview` feature, `#[preview]` functions register nothing, so the app itself carries no preview
code. A reload rebuilds the views, so state in them starts over. `preview-rs` in elegance-examples
is set up this way.

## Effects

Backdrop blur (frosted glass), blur, color adjustments, blend modes, inner shadows and spread,
elliptical and angular gradients, rotation/scale effects, and masks (see `effects-rs` in
elegance-examples):

```rust
Text("Hi").padding(12)
    .background_in(Color::WHITE.opacity(0.5), RoundedRectangle(12))
    .backdrop_blur_in(30, RoundedRectangle(12), |o| o.saturation(1.8).noise(0.02))
tile().grayscale(1).blend_mode(.multiply).rotation_effect(15.0)
backdrop().mask(LinearGradient::colors([Color::BLACK, Color::CLEAR]))
```

### Shared GPU images

`GpuImage` represents externally produced GPU frames; `GpuImageView(&image)` displays
it. For Rust wgpu rendering, `elegance-wgpu::Bridge` creates a compatible producer device,
records frames through its safe render API, and publishes shared GPU memory without CPU pixel
readback. GPU synchronization and frame retirement stay inside the bridge and native framework.

`Bridge::resize` changes subsequent frame dimensions without replacing the image handle.
Use `.on_sized(...)` to send physical-pixel layout dimensions to the producer thread.
Metal/IOSurface, DX12/DXGI, and Vulkan/opaque-FD transports are implemented; unsupported
sharing returns an error rather than falling back to CPU copies. GL/GLES and D3D11 sharing
are not implemented. The old same-device Dawn handle/reservation API has been removed.

## State

State (`State`, `Binding`, `Computed`) is made of `Copy` handles, so `move` closures capture them
without cloning. State lives as long as the views that made it: when an `If` branch switches or a
`ForEach` item goes, the state created inside it is freed, and using a freed handle panics.

```rust
let todos = State::new(vec![todo]);
todos.read().len();                 // a guard (tracked); also todos.get(), todos.with(|t| ..)
todos.peek().len();                 // the same without subscribing
todos.write().push(another);        // notifies when the guard drops; also set(..), update(|t| ..)
editing.toggle();                   // State<bool>
let is_empty = todos.map(Vec::is_empty); // a Computed<bool>
```

Elegance is fine-grained: a component's body runs once, and what updates later is each closure
Elegance runs reactively — a reactive argument, `if`/`match` in `elg!`, a `Computed`, an `effect`.
A read inside such a closure subscribes it wherever the closure was written (in `elg!`, a helper
function, a variable). A read directly in the body, or an expression evaluated once as an
argument (`.disabled(todos.read().is_empty())`), is a one-time value, so pass a closure or a
derived value instead (`.popover` doesn't accept a plain `bool` at all):

```rust
.disabled(move || draft.read().is_empty())
.disabled(draft.map(String::is_empty))
.popover(editing, content)                         // two-way: dismissing sets `editing` to false
.popover(move || todos.read().is_empty(), content) // read-only: shown while the list is empty
.popover(todos.map(Vec::is_empty), content)
```

Two-way arguments (Toggle, Checkbox, TextField, Slider, NumberInput, Picker, `.popover`,
`.selection`, `.focused`, ...) take
`impl IntoBinding<T>`: a `State` or `Binding` (two-way: `Toggle("Wi-Fi", wifi)`), or, read-only,
a `Computed` or a closure.
Reactive one-way arguments take `impl IntoProp<T>`: a value, a `State`/`Binding`/`Computed`, or a
closure.
