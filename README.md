# elegance-rust

Rust bindings to Elegance (`../EleganceMain`), with `elg!`, a SwiftUI-style DSL.

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

- `elegance-sys`: a C++ shim (`shim/*.cpp`) giving Elegance's C++23 API a C ABI, and the raw FFI.
- `elegance`: the safe API: views, modifiers, reactive state, environment, styles, layouts, custom
  modifiers, effects, animation, media, GPU images, HTTP/WebSocket, desktop, previews.
- `elegance-macros`: `elg!`, `#[component]`, and `#[preview]`.
- `cargo-elegance`: `cargo elegance preview`, the preview host runner.

## Building

```sh
cargo run --example counter
cargo run --example effects
```

Elegance is linked statically by default: an app is one executable plus the DLLs Elegance's
dependencies come in (copied next to the binaries). `elegance-sys`'s build script picks the
Elegance build to link, in this order:

1. `ELEGANCE_BUILD_DIR` (static) / `ELEGANCE_SHARED_BUILD_DIR` (the `dynamic` feature), if set;
2. the `dev-release-static` / `dev-release` preset's tree in `$ELEGANCE_ROOT/build`, if it's built;
3. otherwise it builds Elegance itself with CMake into `$ELEGANCE_ROOT/build/cargo-release-static`
   (or `cargo-release`). The first build installs Elegance's vcpkg dependencies and takes a while
   (`cargo build -vv` shows its progress); later builds only rebuild what changed in Elegance.

For that build it finds vcpkg with the baseline `vcpkg.json` pins: `VCPKG_ROOT`, the `VCPKG_ROOT`
Elegance's `CMakeUserPresets.json` sets, `vcpkg` on PATH, or else it clones and bootstraps its own
into `ELEGANCE_VCPKG_DIR` (default `%LOCALAPPDATA%\elegance\vcpkg`, `~/.cache/elegance/vcpkg`).
CMake (Elegance needs 4.3+) and Ninja come from `CMAKE`/`NINJA`, PATH, Visual Studio's bundled
copies, or `vcpkg fetch`; MSVC's environment is found without a developer prompt. Set
`ELEGANCE_AUTO_BUILD=0` to get an error instead. `ELEGANCE_ROOT` defaults to `../EleganceMain`.
The choice is made when the build script runs: if you build a preset tree after cargo has fallen
back to its own, `cargo clean -p elegance-sys` to switch to it.

To build the trees yourself (in a VS developer prompt, in EleganceMain):

```bat
cmake --preset dev-release-static && cmake --build build/dev-release-static --target elegance
cmake --preset dev-release && cmake --build build/dev-release --target elegance elegance-preview
```

Two requirements, which the automatic build and the presets handle: the shim passes `std::string`
and `std::function` to Elegance, so Elegance must use the release C runtime (`/MD`) like Rust, not
`dev-debug`'s `/MDd`; and a static Elegance needs `-DELG_ACCESSKIT_SHARED=ON`, since AccessKit's
static library is a Rust staticlib whose own Rust standard library clashes with the app's.

## The `elg!` DSL

| DSL | Means |
|---|---|
| `Name(a, b)` | an element; overloads are tuple impls of `ElgNew` |
| `Name(a, key: v)` | the same, then the builder call `.key(v)` |
| `Name { ... }` | its children, on the element's line: a stack's views, a button's label, ... |
| `ForEach(xs, id: \|x\| x.id) { x in ... }` | a content closure per item (also `AsyncImage(url) { phase in ... }`) |
| `if cond { ... } else { ... }` | a reactive `If` |
| `match value { pat => View(), ... }` | the view for the current value, rebuilt when it changes (`Clone + PartialEq` values) |
| `.leading`, `.accent`, `.bold`, `.infinity` | shorthand for the parameter's type (`elegance::dot`) |
| `$name` | `name.binding()` |
| `"{count} left"` | text that updates when `count` changes |
| `{ expr }`, or a variable | any Rust value that is `IntoView` |
| `(layout) { ... }` | a custom `Layout` over the children |
| `.m(a, key: v)` | the modifier `.m_with(a, \|o\| o.key(v))`, e.g. `.frame(max_width: .infinity)` |

## Grids

`VGrid`/`HGrid` flow children into fixed, flexible, or adaptive tracks; `Grid` is a table of
`GridRow`s (see `examples/grid.rs` and Elegance's Grids guide):

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
cargo install --path cargo-elegance
cargo elegance preview            # the previews
cargo elegance preview --root     # the root view
cargo elegance preview -p my-app --bin my-app --release
```

`cargo elegance` builds the crate (its binary, or its library if it has no binary) a second time
as a views module: a cdylib, with `elegance/preview` enabled, in `target/elegance-preview`. That
build links the shared Elegance (`dynamic`), which the module and the host share. Without the
`preview` feature, `#[preview]` functions register nothing, so the app itself carries no preview
code. A reload rebuilds the views, so state in them starts over. `../mod-launcher` is set up this
way.

## Effects

Materials, blur, color adjustments, blend modes, inner shadows and spread, elliptical and angular
gradients, rotation/scale effects, and masks (see `examples/effects.rs`):

```rust
Text("Hi").padding(12).background_in(Material::THIN, RoundedRectangle(12))
tile().grayscale(1).blend_mode(.multiply).rotation_effect(15.0)
backdrop().mask(LinearGradient::colors([Color::BLACK, Color::CLEAR]))
```

`GpuImage` (`elegance::media`) shows frames another renderer (OpenGL, Vulkan, Direct3D) draws into
shared, triple-buffered textures, e.g. a game or emulator embedded in a view.

## State

State (`State`, `Binding`, `Computed`) is made of `Copy` handles, so `move` closures capture them
without cloning. State lives as long as the views that made it: when an `If` branch switches or a
`ForEach` item goes, the state created inside it is freed, and using a freed handle panics.
