# elegence-rust

Rust bindings to Elegence (`../ElegenceMain`), with `elg!`, a SwiftUI-style DSL.

```rust
use elegence::prelude::*;

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

- `elegence-sys`: a C++ shim (`shim/*.cpp`) giving Elegence's C++23 API a C ABI, and the raw FFI.
- `elegence`: the safe API: views, modifiers, reactive state, environment, styles, layouts, custom
  modifiers, animation, media, HTTP/WebSocket, desktop, hot reload.
- `elegence-macros`: `elg!` and `#[component]`.

## Building

The shim passes `std::string` and `std::function` across the DLL boundary, so it must be built with
the same compiler and C runtime as `elegence.dll`. Rust uses the release CRT (`/MD`), so link
against the `dev-release` build of Elegence, not `dev-debug` (`/MDd`):

```bat
:: in a VS developer prompt, in ElegenceMain
cmake --preset dev-release
cmake --build build/dev-release --target elegence
```

Then, here:

```sh
cargo run --example counter
cargo run --example todos
```

`ELEGENCE_ROOT` (default `../ElegenceMain`) and `ELEGENCE_BUILD_DIR` (default
`$ELEGENCE_ROOT/build/dev-release`) point the build elsewhere. The build copies `elegence.dll` and
its dependencies next to the built binaries.

## The `elg!` DSL

| DSL | Means |
|---|---|
| `Name(a, b)` | an element; overloads are tuple impls of `ElgNew` |
| `Name(a, key: v)` | the same, then the builder call `.key(v)` |
| `Name { ... }` | its children, on the element's line: a stack's views, a button's label, ... |
| `ForEach(xs, id: \|x\| x.id) { x in ... }` | a content closure per item (also `AsyncImage(url) { phase in ... }`) |
| `if cond { ... } else { ... }` | a reactive `If` |
| `match value { pat => View(), ... }` | the view for the current value, rebuilt when it changes (`Clone + PartialEq` values) |
| `.leading`, `.accent`, `.bold`, `.infinity` | shorthand for the parameter's type (`elegence::dot`) |
| `$name` | `name.binding()` |
| `"{count} left"` | text that updates when `count` changes |
| `{ expr }`, or a variable | any Rust value that is `IntoView` |
| `(layout) { ... }` | a custom `Layout` over the children |
| `.m(a, key: v)` | the modifier `.m_with(a, \|o\| o.key(v))`, e.g. `.frame(max_width: .infinity)` |

## Grids

`VGrid`/`HGrid` flow children into fixed, flexible, or adaptive tracks; `Grid` is a table of
`GridRow`s (see `examples/grid.rs` and Elegence's Grids guide):

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
`ImageSource::file(path)` with an absolute path: Elegence resolves relative paths against the
current directory.

## Previews and hot reload

A library crate of views (`crate-type = ["cdylib", "rlib"]`) can be a views module for
Elegence's preview host, which shows it and rebuilds and reloads it as you edit:

```rust
// src/lib.rs
#[preview("Counter")]
fn counter() -> impl IntoView {
    elg! { Counter() }
}

elegence::module!(Root); // or elegence::module!() for previews only
```

```sh
# build the host once (in ElegenceMain): cmake --build build/dev-release --target elegence-preview
elegence-preview --module target/debug/my_views.dll --build "cargo build --lib" --watch src
elegence-preview --root ...   # the module's root view instead of the previews
```

`../mod-launcher` is set up this way (`preview.cmd`). Give the library its own name
(`[lib] name = "..._views"`): a library and binary with the same name collide on their `.pdb` files.
A reload rebuilds the views, so state in them starts over.

## State

State (`State`, `Binding`, `Computed`) is made of `Copy` handles, so `move` closures capture them
without cloning. State lives as long as the views that made it: when an `If` branch switches or a
`ForEach` item goes, the state created inside it is freed, and using a freed handle panics.
