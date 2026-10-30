# elegance-rs

Rust bindings for [Elegance](https://github.com/EleganceUI/elegance), a cross-platform UI
framework. Build desktop interfaces with a SwiftUI-style `elg!` DSL, composable views, and
fine-grained reactive state.

- **Declarative UI:** layouts, reusable components, typed modifiers, and named arguments.
- **Reactive state:** one-way properties, two-way bindings, and derived values.
- **Desktop tooling:** live previews and hot reload with `cargo elegance preview`.
- **Rendering:** images, gradients, effects, gestures, animation, and optional wgpu integration.

[Getting started](#getting-started) · [Core concepts](#core-concepts) ·
[Examples](https://github.com/EleganceUI/elegance-examples) ·
[Native build guide](docs/building.md) · [Contributing](#contributing)

## Getting started

### Requirements

Use a current stable Rust toolchain. The workspace uses Rust edition 2024.
Prebuilt native libraries are available for these targets:

| Platform | Architectures | Minimum runtime |
| --- | --- | --- |
| Linux | x86_64, ARM64 | glibc 2.35+ |
| macOS | Apple Silicon, Intel | macOS 13.3+ |
| Windows (MSVC) | x64, ARM64 | — |

By default, builds without a local Elegance checkout or source-build override download the
pinned **static** native archives using `curl` and `tar`; no C++ toolchain is needed.
Both prebuilt and source builds link Elegance and its C API into the final Rust executable
by default. Shared dependencies such as AccessKit remain separate runtime files. The `dynamic`
feature uses shared Elegance for previews and hot reload.
See the [native build guide](docs/building.md) for build selection and toolchain requirements.

### Installation

Create a binary crate with `cargo new counter`, then add the Git dependency to `Cargo.toml`:

```toml
[dependencies]
elegance = { git = "https://github.com/EleganceUI/elegance-rs" }
```

### A counter application

Replace `src/main.rs` with:

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

Run the application with `cargo run`.

**Linux:** add a loader-relative rpath to `.cargo/config.toml` before running:

```toml
[target.'cfg(target_os = "linux")']
rustflags = ["-C", "link-args=-Wl,-rpath,$ORIGIN"]
```

There is no `elegance_c` shared library to distribute. Cargo copies remaining shared runtime
dependencies beside the executable. Keep those libraries with your application
when distributing it. See the [native build guide](docs/building.md) for offline builds, source
builds, library overrides, and packaging requirements.

## Core concepts

### The `elg!` DSL

`elg!` combines view construction with ordinary Rust expressions. Named arguments map to
builder methods, and child blocks describe view content.

| Syntax | Meaning |
| --- | --- |
| `Name(a, b)` | Construct an element through an `ElgNew` tuple implementation. |
| `Name(a, key: v)` | Construct an element, then call its `.key(v)` builder method. |
| `Name { ... }` | Supply children, such as a stack's views or a button's label. |
| `ForEach(xs, id: \|x\| x.id) { x in ... }` | Build content for each item. |
| `if cond { ... } else { ... }` | Select a view reactively. |
| `match value { pat => View(), ... }` | Rebuild the selected view when a `Clone + PartialEq` value changes. |
| `.leading`, `.accent`, `.bold`, `.infinity` | Resolve a named value using the parameter's expected type. |
| `"{count} left"` | Interpolate text reactively. |
| `"{size.0}x{size.1}"`, `"{user.name:>8}"`, `"{a + b}"` | Read state fields, apply formatting, or evaluate expressions in text. |
| `{ expr }` or a variable | Insert a Rust value implementing `IntoView`. |
| `(layout) { ... }` | Apply a custom `Layout` to children. |
| `.m(a, key: v)` | Call `.m_with(a, \|o\| o.key(v))`, as in `.frame(max_width: .infinity)`. |

Content closures also support APIs such as `AsyncImage(url) { phase in ... }`.

### State and reactivity

`State`, `Binding`, and `Computed` are `Copy` handles, so `move` closures can capture them
without cloning. A component's body runs once; reactive closures update independently when
the state they read changes.

```rust
let todos = State::new(vec![todo]);
todos.read().len();                     // Tracked read guard; also get() and with(...).
todos.peek().len();                     // Read without subscribing.
todos.write().push(another);            // Notify when the write guard drops.
editing.toggle();                      // Available on State<bool>.
let is_empty = todos.map(Vec::is_empty); // Derived Computed<bool>.
```

Reads inside reactive arguments, `elg!` conditionals, computed values, and effects subscribe
the enclosing reactive closure, including reads made through helper functions. Reads directly
in the component body are one-time values. For a modifier to update, pass a closure or a
derived value rather than an eagerly evaluated expression:

```rust
.disabled(move || draft.read().is_empty())
.disabled(draft.map(String::is_empty))
.popover(editing, content)                         // Dismissing writes false to editing.
.popover(move || todos.read().is_empty(), content) // Read-only visibility.
.popover(todos.map(Vec::is_empty), content)
```

- **One-way properties** (`impl IntoProp<T>`) accept a value, `State`, `Binding`, `Computed`,
  or closure.
- **Two-way arguments** (`impl IntoBinding<T>`) accept a `State` or `Binding` for read/write
  access, or a `Computed` or closure for read-only access. These include `Toggle`, `Checkbox`,
  `TextField`, `Slider`, `NumberInput`, `Picker`, `.popover`, `.selection`, and `.focused`.
  Unlike one-way properties, they do not accept plain values such as `bool`.

State is scoped to the views that created it. Removing an `If` branch or `ForEach` item frees
its local state; using a handle after that state is freed panics.

Elegance does not provide a network API. Use a Rust HTTP or WebSocket library and deliver
results to the UI with `post_to_ui_thread`.

### Reusable components

Annotate a function with `#[component]`, or implement `Component` for a struct. Function
props receive builder methods automatically:

- `Prop<T>` accepts reactive inputs, including values, state, closures, and interpolated text.
- `Binding<T>` supports two-way updates; pass the parent's `State`.

For a struct's builder implementation, `#[elg_builder(constructor)]` registers `fn new() -> Self`
with `elg!` and enables typed dot completion without a handwritten `ElgNew<()>` implementation.
Use `#[elg_builder]` for completion only, including on extension traits.

See [`components-rs`](https://github.com/EleganceUI/elegance-examples/tree/main/components-rs)
for examples of component props.

### Leading-dot values and named arguments

Leading-dot values are resolved by type: `.leading` can mean `HorizontalAlignment::Leading`
for a stack or `Alignment::LEADING` for a frame. Derive `DotValues` to expose an enum's unit
variants as snake_case names:

```rust
#[derive(Clone, Copy, PartialEq, DotValues)]
enum Density {
    Compact,
    Comfortable,
    ExtraRoomy,
}
```

A component prop of type `Density` can then receive `density: .extra_roomy`, and
`#[prop(default = .comfortable)]` can supply its default. These names also work with
`impl Into<Density>` and `impl IntoProp<Density>`. For a builder method taking exactly
`Density`, annotate its implementation with `#[elg_builder]`; component props already have
this support. Unknown names produce compile errors.

Named modifier arguments use an options builder. For example,
`.popover(editing, content, edge: .top)` expands to
`.popover_with(editing, content, |o| o.edge(.top))`, where `edge` is `PopoverOptions::edge`.
Custom modifiers can support the same syntax by providing a `name_with` method that takes
`impl FnOnce(Options) -> Options`.

With rust-analyzer's proc macros enabled, leading-dot completion uses the expected parameter
type, including imported names, aliases, and enums deriving `DotValues`. Property-name
completion may insert method syntax such as `spacing(…)`; change it to `spacing: …` in the DSL.

## Layout and rendering

### Grids

`VGrid` and `HGrid` arrange children in fixed, flexible, or adaptive tracks. `Grid` provides
a table layout using `GridRow`:

```rust
VGrid([GridItem::adaptive(150)], spacing: 12) {
    ForEach(games, id: |g| g.id) { game in Card(game: game.clone()) }
}
```

### Images

Place application assets in `<crate>/assets/` and embed them with `asset!`. Embedded assets
work in the application, previews, and installed copies:

```rust
Image(asset!("images/logo.png")).resizable().frame(96, 96)
```

For runtime files, such as user-selected images or downloads, use `ImageSource::file(path)`
with an absolute path. Relative paths are resolved against the current working directory.

### Effects

Effects include backdrop blur, color adjustments, blend modes, inner shadows and spread,
elliptical and angular gradients, transforms, and masks:

```rust
Text("Hi").padding(12)
    .background_in(Color::WHITE.opacity(0.5), RoundedRectangle(12))
    .backdrop_blur_in(30, RoundedRectangle(12), |o| o.saturation(1.8).noise(0.02))
tile().grayscale(1).blend_mode(.multiply).rotation_effect(15.0)
backdrop().mask(LinearGradient::colors([Color::BLACK, Color::CLEAR]))
```

### GPU images

`GpuImage` represents externally produced GPU frames; `GpuImageView(&image)` displays them.
The optional [`elegance-wgpu`](elegance-wgpu/README.md) crate integrates native Rust wgpu
30.0.1 with shared GPU memory and no CPU pixel readback. The bridge manages synchronization
and frame retirement. `Bridge::resize` preserves the image handle; `.on_sized(...)` can forward
physical-pixel layout dimensions to a producer thread.

Metal/IOSurface, DX12/DXGI, and Vulkan/opaque-FD transports are implemented. **Only Metal has
been hardware-tested**; native DX12 and Vulkan execution still require validation. Unsupported
sharing returns an error rather than falling back to CPU copies. GL/GLES and D3D11 sharing
are not implemented. See the bridge's [documentation and safety requirements](elegance-wgpu/README.md)
before integrating a renderer.

## Previews and hot reload

Annotate functions with `#[preview]` anywhere in an application; no library/binary split is
required:

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

Install the preview runner, then run it from your application's directory:

```sh
cargo install --git https://github.com/EleganceUI/elegance-rs cargo-elegance
cargo elegance preview
cargo elegance preview --root
cargo elegance preview -p my-app --bin my-app --release
```

The runner rebuilds and reloads views as you edit. It builds the binary (or library, if there
is no binary) as a views module (`cdylib`) with `elegance/preview` enabled, under
`target/elegance-preview`. The module and host share a dynamically linked Elegance runtime,
which requires a [source build](docs/building.md#building-from-source).

Reloads reset view state. Without the `preview` feature, preview functions register nothing
and the application carries no preview code.

## Examples

The [elegance-examples](https://github.com/EleganceUI/elegance-examples) repository contains
runnable Rust applications in its `*-rs` directories. Start with `components-rs` for component
props, `grids-rs` for layouts, `effects-rs` for rendering effects, or `preview-rs` for previews.

## Workspace crates

| Crate | Purpose |
| --- | --- |
| [`elegance`](elegance) | Safe API for views, state, environment, styles, layouts, modifiers, gestures, animation, media, and desktop integration. |
| [`elegance-sys`](elegance-sys) | Raw FFI bindings to Elegance's C API, maintained in the native framework's `c/` directory. |
| [`elegance-macros`](elegance-macros) | The `elg!` DSL and component, preview, and builder macros. |
| [`cargo-elegance`](cargo-elegance) | Preview host runner for `cargo elegance preview`. |
| [`elegance-wgpu`](elegance-wgpu) | Optional native wgpu integration for shared GPU images. |

## Contributing

Bug reports and pull requests are welcome. [Open an issue](https://github.com/EleganceUI/elegance-rs/issues)
with a minimal reproduction, your platform, Rust version, and relevant build output.

For native framework changes, check out [Elegance](https://github.com/EleganceUI/elegance)
beside this repository. To test these bindings with the examples, add the following to
`elegance-examples/.cargo/config.toml` in a sibling examples checkout:

```toml
[patch."https://github.com/EleganceUI/elegance-rs"]
elegance = { path = "../elegance-rs/elegance" }
```

Run the same lint and test commands used by CI from this repository:

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs on Linux, macOS, and Windows with `ELEGANCE_PREBUILT=1`. Local builds beside an Elegance
checkout use source builds by default; set that variable to match CI. Linux runs also need
the rpath configuration shown in [Getting started](#getting-started).

See the [native build guide](docs/building.md) for source and build overrides and the
[native revision update process](docs/building.md#updating-the-native-revision).

## License

The Rust workspace is licensed under the
[Mozilla Public License 2.0 (MPL-2.0)](https://www.mozilla.org/MPL/2.0/), as declared in
[`Cargo.toml`](Cargo.toml). Native Elegance and third-party dependencies are subject to their
respective licenses.
