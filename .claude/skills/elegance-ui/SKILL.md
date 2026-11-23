---
name: elegance-ui
description: Build and edit desktop GUIs with Elegance (elegance-rs, the `elegance` crate, `elg!` macro, `#[component]`, `#[preview]`, `cargo elegance preview`), a declarative, reactive, GPU-rendered, SwiftUI-style UI library with C++ and Rust bindings. Use whenever the task is writing, changing, debugging or previewing a UI, window, view, component, layout, control, theme, animation, or hot reload with Elegance or its Rust binding, or when a Cargo.toml depends on `elegance`, or code uses `use elegance::prelude::*`, `State::new`, `VStack`, `elg!`.
---

# Elegance UI (Rust)

Elegance is a declarative, SwiftUI-style GUI library with fine-grained reactivity, rendered on the GPU (WebGPU/Dawn) in
one process, for Windows, macOS, Linux and Mikros (C++ core; `elegance-rs` is the Rust binding; for Mikros see
gotchas). Views are cheap values that describe UI; modifiers wrap views; your view code runs
once and only the small closures you pass where a value is expected re-run when the `State` they read changes. Version is
0.x: the API can change between releases. The crate is a Git dependency, not on crates.io.

Reference files (read the one you need; do not guess names):

| File | Contents |
| --- | --- |
| [reference/components.md](reference/components.md) | Every view/control/container/drawing/media/window/desktop/event/accessibility type with signatures, when to use, examples |
| [reference/modifiers.md](reference/modifiers.md) | Every view modifier (`ViewExt` method), option structs, argument kinds, ordering |
| [reference/state-and-reactivity.md](reference/state-and-reactivity.md) | `State`/`Binding`/`Computed`/`effect`, props, timers, animation, threads, environment, recipes |
| [reference/theming-and-style.md](reference/theming-and-style.md) | Colors, themes, light/dark, accent, fonts, icons, control styles, window materials |
| [reference/preview.md](reference/preview.md) | `cargo elegance preview`, hot reload, flags, limits, verifying without a display |
| [reference/gotchas.md](reference/gotchas.md) | Threading, lifetimes, errors, silent failures, performance, doc-vs-code mismatches |
| [reference/cpp-vs-rust-gaps.md](reference/cpp-vs-rust-gaps.md) | What exists in C++ but not in Rust, and how C++ docs map to Rust names |

## Setup and commands

Requirements: Rust 1.88+ (edition 2024), `curl` and `tar` (the first build downloads a prebuilt static Elegance for your
target, cached in the user cache dir). Prebuilt targets: Windows MSVC (x86_64, aarch64), macOS (arm64, x86_64; 13.3+),
Linux (x86_64, aarch64; glibc 2.35+). Other targets build Elegance from source (CMake, vcpkg; very long).

There is no project generator in `cargo-elegance` (its commands are `preview`, `build-module`, `help`). Create a normal
binary crate and add the dependency:

```sh
cargo new my-app && cd my-app
```

```toml
# Cargo.toml
[package]
edition = "2024"
[dependencies]
elegance = { git = "https://github.com/EleganceUI/elegance-rs" }
```

Linux only: the executable must find `libaccesskit.so` beside itself; add `.cargo/config.toml`:

```toml
[target.'cfg(target_os = "linux")']
rustflags = ["-C", "link-args=-Wl,-rpath,$ORIGIN"]
```

| Task | Command |
| --- | --- |
| Run | `cargo run` (`--release` for real performance) |
| Type-check (fast, no window) | `cargo check` |
| Test | `cargo test` (type-check-only view tests are possible, see preview.md) |
| Install preview runner (once) | `cargo install --git https://github.com/EleganceUI/elegance-rs cargo-elegance` |
| Preview picker (hot reload) | `cargo elegance preview` |
| Preview the whole app | `cargo elegance preview --root` (needs `#[preview(root)]`) |
| Flags | `-p/--package <n>`, `--bin <n>`, `--manifest-path <f>`, `--release`, `-F/--features <list>`, `--watch <dir>` (repeat) |
| Compile-check the preview build | `cargo elegance build-module` (prints the module path; no window) |
| Embed assets | put files in `<crate>/assets/`, use `asset!("images/logo.png")` |

Preview facts: needs a display; first run builds Elegance from source as a shared library (long); builds your crate a
second time into `target/elegance-preview`; reloads on changes in `src/`, `assets/` and `--watch` dirs; view state resets
on every reload; a failed build keeps the old UI and shows the error. Details in reference/preview.md.
An agent without a display cannot see the UI: verify with `cargo check`, `cargo elegance build-module`, `cargo test`, and
the stderr log, and tell the user what was not visually verified.

## Minimal complete example

Built from the README counter and the compile-checked first-app tutorial pieces (not built in this environment).

```rust
use elegance::prelude::*;

#[derive(Clone, PartialEq)]
struct Todo { id: i64, title: String, done: bool }

#[component]
fn TodoApp() -> impl IntoView {
    let draft = State::new(String::new());
    let mut next_id = State::new(1_i64);
    let todos = State::new(Vec::<Todo>::new());
    let remaining = todos.map(|list| list.iter().filter(|t| !t.done).count());
    let add = move || {
        if draft.read().is_empty() { return; }
        with_animation(Animation::smooth(0.4), || {
            todos.write().push(Todo { id: next_id.get(), title: draft.get(), done: false });
        });
        next_id += 1;
        draft.set(String::new());
    };
    elg! {
        VStack(alignment: .leading, spacing: 12) {
            Text("Todos").font(.large_title)
            HStack {
                TextField("What needs doing?", draft, on_submit: add)
                Button("Add", add).disabled(move || draft.read().is_empty())
            }
            if todos.read().is_empty() {
                Text("Nothing left to do.").foreground_style(.secondary)
            } else {
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(todos, id: |t| t.id) { todo in
                            Text(todo.title.clone()).padding(horizontal: 16, vertical: 10)
                        }
                    }
                }
            }
            Text("{remaining} left")
        }
        .padding(24)
        .frame(max_width: .infinity, max_height: .infinity, alignment: .top_leading)
    }
}

#[preview("Todos")]            // only compiled in preview builds
fn todos_preview() -> impl IntoView { elg! { TodoApp() } }

fn main() -> Result<std::process::ExitCode, Error> {
    App::new().window(WindowDesc::new("Todos").size(480, 560), TodoApp).run()
}
```

## Core mental model

- **Views are values.** `Text("Hi")` draws nothing by itself; placing it in a window builds the real element. Reuse by
  `.clone()`. A `#[component] fn Name(props) -> impl IntoView` runs when placed (so it can read the environment); a plain
  fn returning a view runs immediately where called. Component params become builder args: `Name(title: "x")`.
- **Modifiers wrap.** `.padding(8).background(c)` colors the padding; `.background(c).padding(8)` does not. Read inside out.
- **Reactivity.** `State<T>`, `Binding<T>`, `Computed<T>` are `Copy` handles (capture with `move`). A body runs once; what
  re-runs are closures given to reactive arguments (`impl IntoProp<T>`: a value, state, computed or closure), `Computed`s
  and `effect`s that read state. Reads in the body itself happen once and never update. `Text("{count}")` (a lone string
  literal with `{..}` in `elg!`) is reactive; `Text(format!(..))` and `Text(count.get().to_string())` are not.
  Writes in one event are batched; each affected closure runs once before the next frame.
- **Changing what is shown** (not how it looks): `if`/`match` inside `elg!`, and `ForEach(items, id: |x| x.id) { x in .. }`.
  State created inside a removed branch or `ForEach` item is freed; using its handle later panics (`is_alive()` checks).
- **Two-way controls** (`Toggle`, `Checkbox`, `TextField`, `Slider`, `NumberInput`, `Picker`, `.popover`, `.selection`,
  `.focused`) take `impl IntoBinding<T>`: pass a `State` or `Binding` (read/write); a closure or `Computed` is read-only;
  plain values are not accepted.
- **Layout** is a negotiation: the parent proposes, the child picks its size, the parent places it. A view is as big as
  its content and centered; use `.frame(max_width: .infinity, max_height: .infinity, alignment: ..)` and `Spacer` to fill.
  Stacks share space; `ScrollView` + `LazyVStack` for long lists; grids for tiles.
- **Environment** values flow down (font, foreground, tint, color scheme, theme, disabled, control styles, your own keys).
  Set high, read below with `Environment::<Key>::new()`.
- **Animation:** wrap writes in `with_animation(Animation::smooth(0.4), || ..)`, or use `.animation(anim, watched_value)`
  and `.transition(..)`. Rust has no default arguments: `Animation::spring(response, damping_fraction)`.
- **App/window lifecycle:** one `App` per process; `App::new().window(WindowDesc::new(title).size(w, h), Component).run()`
  returns when the last window closes (`Result<ExitCode, Error>`). `on_error(|e| ..)` decides what happens when your code
  panics (default: logged, app continues). Press F12 in any window for the inspector.
- **Threads:** all code (bodies, handlers, effects, timers, callbacks) runs on the UI thread. State is `!Send`. Do slow work
  on another thread and return plain data with `post_to_ui_thread(move || ..)` (`Send + 'static`). Elegance has no
  networking API; use any Rust HTTP crate.
- **`elg!` syntax:** `Name(a, key: v) { children }`, `.leading`/`.bold`/`.infinity` leading-dot values resolved by the
  parameter type, `.m(a, key: v)` calls `.m_with(a, |o| o.key(v))`, `{ expr }` embeds any `IntoView`, `(layout) { .. }`
  uses a custom `Layout`. Typos in dot names are compile errors.

## Which component for which job

| Need | Use | Not |
| --- | --- | --- |
| Stack children vertically / horizontally / overlapped | `VStack` / `HStack` / `ZStack` | manual offsets |
| Flexible gap or push apart | `Spacer` | fixed `.padding` hacks |
| Divider line | `Separator`, or `separators:` on a lazy stack | |
| Long or unbounded list | `ScrollView { LazyVStack { ForEach(..) } }` (builds visible rows only) | plain `VStack` in `ScrollView` |
| Short fixed list | `VStack` + `ForEach` | |
| Tiles / adaptive columns | `VGrid([GridItem::adaptive(150)], ..)` / `HGrid` | |
| Table with aligned columns | `Grid` + `GridRow` | |
| Sidebar + detail | `NavigationSplitView`; resizable panes `HSplitView`/`VSplitView` | |
| Push/pop screens | `NavigationStack` (+ `PageTransition`) | |
| Show/hide a view reactively | `if`/`match` in `elg!` | `.opacity(0)` to hide (still in tree) |
| One-off custom positioning | custom `Layout` trait (`HStackLayout` etc. are built-ins); `Cached` for expensive ones | |
| Static or reactive text | `Text("..")`, `Text("{x}")`; rich runs `StyledText`/`TextSpan`/`TextRun` | |
| Single-line / multi-line input | `TextField` / `TextArea` (bind a `State<String>`) | |
| Numeric input / slider | `NumberInput` / `Slider` | |
| On/off | `Toggle` (switch) or `Checkbox` | |
| Choose one of N | `Picker` (segmented/menu styles) | |
| Action button | `Button(label, || ..)`; look via `.button_style(..)` | `on_click` on Text (no focus/keyboard/accessibility) |
| Progress | `Progress` (determinate `Some(f)`, or indeterminate) | |
| Menu from a button | `Menu` / `MenuButton` / `MenuItem` / `Submenu`; right-click `context_menu` | |
| Anchored panel, hint | `.popover(state, content)`, `.tooltip(..)` | a new window |
| Basic shape / custom outline | `Rectangle`, `RoundedRectangle`, `Circle`, `Capsule`; `PathShape`/`CustomShape` | `Canvas` for static shapes |
| Charts, per-frame custom drawing | `Canvas` with `GraphicsContext` + `Path` | |
| Gradients / blur / glass | `LinearGradient`..., `.background_in(..)`, `.backdrop_blur_in(..)` | |
| Icon or bundled image | `Image(asset!(..))`, `Image("icon-name")`, `Label` (icon + text) | |
| Remote image | `AsyncImage(url) { phase in .. }` | `Image` + manual fetch |
| Video / camera / sound | `VideoPlayer`/`VideoView`, `Camera`/`CameraView`, `Sound`/`AudioPlayer` | |
| Frames from your own wgpu renderer | `GpuImageView` + `elegance-wgpu` | |
| Window, menu bar, title bar | `WindowDesc`, `MenuBarItem`, `TitleBar`, `WindowMaterial` | |
| Native dialogs, notifications, tray, dock | `choose_files`/`choose_save_file`/`choose_folders`, `notify`, `TrayIcon`, `set_badge`, `set_progress` | |
| Accept dropped files | `.on_drop(DropOptions::default().files(..))` | (no drag source, no clipboard API) |
| Derived value | `state.map(..)` or `computed(..)` | an `effect` that writes state |
| Periodic / delayed work | `every(secs, ..)`, `after(secs, ..)` (cancel with `Timer::cancel`) | threads with sleeps |
| App-wide settings flow | custom `EnvironmentKey` | globals or threading props through |
| Reusable look for a control | control style traits (`ButtonStyle` ... ) + `.button_style(..)` | copy-pasting modifiers |
| Reusable wrapper | `ViewModifier`; reusable view: `#[component]` | |

Full catalogue: reference/components.md. Every modifier: reference/modifiers.md.

## Common mistakes checklist

Each is verified in code or docs (see reference/gotchas.md for more).

1. Reading state in the body (`Text(count.get().to_string())`, `.opacity(x.get())`): one-shot. Pass the handle, a closure,
   `.map(..)` or `"{count}"`.
2. Forgetting `move` on closures that capture state; forgetting `let mut` for `count += 1` (`update`/`set` do not need it).
3. Passing a plain `bool`/`String` to a two-way argument; or a closure/`Computed`, making it read-only (popover cannot dismiss).
4. Touching state from another thread: write only inside `post_to_ui_thread`, send plain data across.
5. Using a state handle after its branch/`ForEach` item was removed (panic); guard late callbacks with `is_alive()`.
6. `Timer` drop does not cancel it; timers started from click handlers outlive the view; cancel them.
7. An `effect` that writes what it reads loops (aborts after 100,000 runs); derive with `computed` instead; `effect()` only
   while building views.
8. Wrong modifier order (`.padding` vs `.background`); forgetting `.frame(max_width: .infinity, ..)` so the view stays small.
9. `.resizable()` is missing on a scaled `Image`; relative file paths resolve against the cwd: use `asset!` or absolute paths.
10. `LazyVStack`/`LazyHStack` hold exactly one child (the `ForEach`); `NavigationSplitView` needs 2 or 3 children.
11. Unstable or duplicate `ForEach` ids (indexes) rebuild rows and lose their state; changed item values also rebuild.
12. `.animation(anim, v)` needs a watched `State`/`Computed`/`Binding`/closure, not a plain value.
13. `Animation::spring()`/`padding()` with no arguments: Rust has no default arguments; see signatures.
14. Calling something C++-only (clipboard, drag source, `help` modifier, runtime window opening): not in Rust; see
    reference/components.md "not exposed".
15. Expecting hot reload to keep state: every reload resets state made in views.
16. Running preview without a source build / display, or with several bins and no `--bin`.
17. Colors: use semantic `ThemeColor` tokens (`.accent`, `.surface`, `.secondary`, ...) so light/dark just work; fixed
    `Color::hex` ignores the scheme. See reference/theming-and-style.md.
18. Errors: usage mistakes panic (they are bugs); loading calls return `Result<_, Error>`; install `App::on_error` to make
    panics fatal in tests/CI.

## Workflow for an agent

1. Look at existing code first (`Cargo.toml`, components, theme). Match its component style and tokens.
2. Look up every API you use in the reference files; if a name is not in them, grep `elegance-rs/elegance/src` (or the
   cargo git checkout of `elegance`) rather than guessing. The generated Rust API docs do not exist.
3. Build small `#[component]`s with sample-data `#[preview]`s; keep logic in plain functions over data so it is testable.
4. `cargo check` after each change; `cargo elegance build-module` before claiming previews work; run `cargo test`.
5. When a display is available: `cargo elegance preview`, then F12 inspector; otherwise state what could not be verified.
6. Do not claim the UI "looks right" without seeing it.

## How far to trust this skill

- Type, method and function names were checked against the `elegance-rs` sources by script: every item the `elegance` crate
  re-exports and every `ViewExt` method appears in the reference files.
- The examples in `reference/components.md` and `reference/state-and-reactivity.md` were written from the docs and the
  sources and were not compile-checked. Some use placeholder variables (`folders`, `messages`, `contacts`, `pinned_id`).
  Treat them as patterns, and let `cargo check` decide.
- Linux behavior of file dialogs, notifications and tray icons, whether Rust can open extra windows at run time, and the
  `elegance-wgpu` backends were not checked. `reference/cpp-vs-rust-gaps.md` lists what the C++ library has that the Rust
  binding doesn't.
- When the code and this skill disagree, the code wins: fix the skill.
