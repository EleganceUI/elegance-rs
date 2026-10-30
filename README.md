<h1 align="center">elegance-rs</h1>

<p align="center">
  <strong>Rust bindings for <a href="https://github.com/EleganceUI/elegance">Elegance</a>: declarative, reactive desktop UI with a SwiftUI-style DSL.</strong>
</p>

<p align="center">
  <a href="https://github.com/EleganceUI/elegance-rs/actions/workflows/rust.yml"><img alt="CI" src="https://github.com/EleganceUI/elegance-rs/actions/workflows/rust.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: MPL-2.0" src="https://img.shields.io/badge/license-MPL--2.0-blue.svg"></a>
  <img alt="Rust 1.88+" src="https://img.shields.io/badge/rust-1.88%2B-orange.svg">
</p>

<p align="center">
  <a href="https://eleganceui.github.io/elegance-docs/">Documentation</a> ·
  <a href="https://github.com/EleganceUI/elegance-examples">Examples</a> ·
  <a href="docs/building.md">Native build guide</a> ·
  <a href="#contributing">Contributing</a>
</p>

---

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

## Features

- **`elg!` DSL.** Layouts, components, modifiers, named arguments, and leading-dot values
  (`.leading`, `.bold`) in a syntax close to SwiftUI, with rust-analyzer completion.
- **Fine-grained reactivity.** `State`, `Binding`, and `Computed` are `Copy` handles. A
  component's body runs once, and only the closures that read a value rerun when it changes.
- **No C++ toolchain.** Elegance comes prebuilt and is linked statically into your executable.
- **Rendering.** Gradients, backdrop blur, blend modes, masks, shadows, animation, gestures,
  images, and video, all on the GPU.
- **Live previews and hot reload** with `cargo elegance preview`.
- **wgpu interop.** Show frames rendered with [wgpu](https://wgpu.rs) without copying pixels
  through the CPU ([`elegance-wgpu`](elegance-wgpu/README.md)).

> [!NOTE]
> Elegance is in early development (0.x). The API can change between releases.

## Installation

elegance-rs is not on crates.io yet. Add it as a Git dependency:

```toml
[dependencies]
elegance = { git = "https://github.com/EleganceUI/elegance-rs" }
```

Then `cargo run`. The first build downloads the prebuilt Elegance for your target. It's cached in
your user cache folder and shared by all your projects.

### Requirements

- Rust 1.88 or newer (edition 2024)
- `curl` and `tar`, which ship with Windows 10+, macOS, and practically every Linux distribution

Prebuilt libraries are published for these targets:

| Platform | Targets | Minimum OS |
| --- | --- | --- |
| Windows (MSVC) | `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` | — |
| macOS | `aarch64-apple-darwin`, `x86_64-apple-darwin` | macOS 13.3 |
| Linux | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` | glibc 2.35 |

On other targets, or when you ask for one, elegance-sys builds Elegance from source with CMake
and vcpkg. See [Building from source](docs/building.md#building-from-source).

### Linking and distribution

Elegance, its C API, and its C++ dependencies are linked **statically** into your executable.
Only a few shared libraries stay separate, and Cargo copies them next to your binary:

| Platform | Ships beside the executable |
| --- | --- |
| Windows | `accesskit.dll`, plus Dawn's DirectX shader compiler (`dxcompiler.dll`, `dxil.dll`) when the package includes it |
| macOS | `libaccesskit.dylib` |
| Linux x86_64 | `libaccesskit.so` |

AccessKit, the accessibility library, stays shared because its static library carries its own
copy of the Rust standard library, which would clash with your application's.

On Linux, the executable needs to find `libaccesskit.so` beside itself. Add this to
`.cargo/config.toml`:

```toml
[target.'cfg(target_os = "linux")']
rustflags = ["-C", "link-args=-Wl,-rpath,$ORIGIN"]
```

On macOS this works without configuration. The [native build guide](docs/building.md) covers
offline builds, source builds, and packaging in detail.

## Guide

### The `elg!` DSL

`elg!` mixes view construction with ordinary Rust expressions. Named arguments map to builder
methods, and blocks supply child views.

| Syntax | Meaning |
| --- | --- |
| `Name(a, b)` | Construct an element (through an `ElgNew` implementation) |
| `Name(a, key: v)` | Construct an element, then call its `.key(v)` builder method |
| `Name { ... }` | Supply children, such as a stack's views or a button's label |
| `ForEach(xs, id: \|x\| x.id) { x in ... }` | Build content for each item |
| `if cond { ... } else { ... }` | Choose a view reactively |
| `match value { pat => View(), ... }` | Rebuild the chosen view when a `Clone + PartialEq` value changes |
| `.leading`, `.accent`, `.bold`, `.infinity` | A named value, resolved from the parameter's type |
| `"{count} left"` | Reactive text interpolation |
| `"{size.0}x{size.1}"`, `"{user.name:>8}"`, `"{a + b}"` | Fields, format specs, and expressions in text |
| `{ expr }` or a variable | Insert any Rust value implementing `IntoView` |
| `(layout) { ... }` | Arrange children with a custom `Layout` |
| `.m(a, key: v)` | Call `.m_with(a, \|o\| o.key(v))`, as in `.frame(max_width: .infinity)` |

Content closures work for APIs such as `AsyncImage(url) { phase in ... }`.

### State and reactivity

```rust
let todos = State::new(vec![todo]);
todos.read().len();                      // Tracked read; also get() and with(...)
todos.peek().len();                      // Read without subscribing
todos.write().push(another);             // Notifies when the guard drops
editing.toggle();                        // On State<bool>
let is_empty = todos.map(Vec::is_empty); // Derived Computed<bool>
```

Reads inside reactive arguments, `elg!` conditionals, computed values, and effects subscribe
the enclosing closure, including reads made through helper functions. Reads in the component
body itself happen once. To make a modifier update, pass a closure or a derived value:

```rust
.disabled(move || draft.read().is_empty())
.disabled(draft.map(String::is_empty))
.popover(editing, content)                         // Dismissing writes false to `editing`
.popover(todos.map(Vec::is_empty), content)        // Read-only visibility
```

- **One-way properties** (`impl IntoProp<T>`) accept a value, `State`, `Binding`, `Computed`,
  or closure.
- **Two-way arguments** (`impl IntoBinding<T>`) accept a `State` or `Binding` for read/write
  access, or a `Computed` or closure for read-only access. `Toggle`, `Checkbox`, `TextField`,
  `Slider`, `NumberInput`, `Picker`, `.popover`, `.selection`, and `.focused` take them. Plain
  values such as `bool` aren't accepted.

State belongs to the view that created it. Removing an `if` branch or a `ForEach` item frees
its state, and using a handle after that panics.

Elegance has no networking API. Use any Rust HTTP or WebSocket library and hand results to the
UI with `post_to_ui_thread`.

### Components

Annotate a function with `#[component]`, or implement `Component` for a struct. Function
parameters become builder methods:

- `Prop<T>` accepts reactive inputs: values, state, closures, and interpolated text.
- `Binding<T>` is two-way; pass the parent's `State`.

On a struct's builder `impl`, `#[elg_builder(constructor)]` registers `fn new() -> Self` with
`elg!` and enables typed dot completion. `#[elg_builder]` alone enables completion, extension
traits included. [`components-rs`](https://github.com/EleganceUI/elegance-examples/tree/main/components-rs)
shows every kind of prop.

### Leading-dot values and named arguments

Leading-dot values resolve by type: `.leading` is `HorizontalAlignment::Leading` for a stack
and `Alignment::LEADING` for a frame. Derive `DotValues` to give an enum's unit variants
snake_case names:

```rust
#[derive(Clone, Copy, PartialEq, DotValues)]
enum Density {
    Compact,
    Comfortable,
    ExtraRoomy,
}
```

A `Density` prop then accepts `density: .extra_roomy`, and `#[prop(default = .comfortable)]`
sets its default. Unknown names are compile errors.

Named modifier arguments go through an options builder:
`.popover(editing, content, edge: .top)` expands to
`.popover_with(editing, content, |o| o.edge(.top))`. Your own modifiers get the same syntax by
providing a `name_with` method that takes `impl FnOnce(Options) -> Options`.

With rust-analyzer's proc-macro support on, leading-dot completion uses the parameter's
expected type. Property-name completion may insert `spacing(…)`; change it to `spacing: …`.

### Layout and rendering

`VGrid` and `HGrid` arrange children in fixed, flexible, or adaptive tracks, and `Grid` lays out
a table with `GridRow`:

```rust
VGrid([GridItem::adaptive(150)], spacing: 12) {
    ForEach(games, id: |g| g.id) { game in Card(game: game.clone()) }
}
```

Put assets in `<crate>/assets/` and embed them with `asset!`; they work in the app, previews,
and installed copies. For runtime files, use `ImageSource::file(path)` with an absolute path.

```rust
Image(asset!("images/logo.png")).resizable().frame(96, 96)
```

Effects include backdrop blur, color adjustments, blend modes, inner shadows, angular and
elliptical gradients, transforms, and masks:

```rust
Text("Hi").padding(12)
    .background_in(Color::WHITE.opacity(0.5), RoundedRectangle(12))
    .backdrop_blur_in(30, RoundedRectangle(12), |o| o.saturation(1.8).noise(0.02))
tile().grayscale(1).blend_mode(.multiply).rotation_effect(15.0)
backdrop().mask(LinearGradient::colors([Color::BLACK, Color::CLEAR]))
```

### GPU images and wgpu

`GpuImageView(&image)` displays frames produced on the GPU by your own renderer. The optional
[`elegance-wgpu`](elegance-wgpu/README.md) crate connects wgpu 30 to it through shared GPU memory,
handling synchronization and frame retirement.

| Backend | Status |
| --- | --- |
| Metal (macOS) | Implemented and tested on hardware |
| Direct3D 12 (Windows) | Implemented, not yet tested on hardware |
| Vulkan (Linux) | Implemented, not yet tested on hardware |

Unsupported configurations return an error rather than falling back to CPU copies. Read the
crate's [documentation and safety requirements](elegance-wgpu/README.md) before integrating it.

## Previews and hot reload

Annotate functions with `#[preview]` anywhere in your application:

```rust
#[preview("Counter")]
fn counter() -> impl IntoView {
    elg! { Counter() }
}

#[preview(root)]
fn app() -> impl IntoView {
    elg! { Root() }
}
```

Install the preview runner and start it from your crate's directory:

```sh
cargo install --git https://github.com/EleganceUI/elegance-rs cargo-elegance
cargo elegance preview                 # pick a preview
cargo elegance preview --root          # the whole app
cargo elegance preview -p my-app --bin my-app --release
```

The runner rebuilds and reloads views as you edit, and reloads reset view state. It builds your
crate as a views module that shares a dynamically linked Elegance with the preview host, so
previews need a [source build](docs/building.md#building-from-source) of Elegance. Without the
`preview` feature, `#[preview]` functions compile to nothing.

## Workspace

| Crate | Description |
| --- | --- |
| [`elegance`](elegance) | The safe API: views, state, environment, styles, layout, modifiers, gestures, animation, media, desktop integration |
| [`elegance-macros`](elegance-macros) | `elg!` and the component, preview, and builder macros |
| [`elegance-sys`](elegance-sys) | Raw FFI bindings to Elegance's C API, and the build script that links it |
| [`elegance-wgpu`](elegance-wgpu) | Optional wgpu interop through shared GPU images |
| [`cargo-elegance`](cargo-elegance) | The `cargo elegance preview` runner |

## Examples

[elegance-examples](https://github.com/EleganceUI/elegance-examples) has runnable Rust apps in its
`*-rs` folders. Good starting points are `hello-rs`, `components-rs` (props), `grids-rs`
(layout), `effects-rs` (rendering), and `preview-rs` (hot reload).

## Contributing

Bug reports and pull requests are welcome.
[Open an issue](https://github.com/EleganceUI/elegance-rs/issues) with a minimal reproduction,
your platform, your Rust version, and the relevant build output.

Before submitting a pull request, run the same checks as CI:

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI sets `ELEGANCE_PREBUILT=1`. A checkout of [Elegance](https://github.com/EleganceUI/elegance)
at `../elegance` makes local builds use it from source instead; set that variable to match CI.

To try your changes in the examples, add this to `.cargo/config.toml` in a sibling
elegance-examples checkout:

```toml
[patch."https://github.com/EleganceUI/elegance-rs"]
elegance = { path = "../elegance-rs/elegance" }
```

Changes to the C API belong in Elegance's [`c/`](https://github.com/EleganceUI/elegance/tree/main/c)
directory. See [Updating the native revision](docs/building.md#updating-the-native-revision).

## License

elegance-rs is licensed under the [Mozilla Public License 2.0](LICENSE). Elegance and its
dependencies are distributed under their own licenses.
