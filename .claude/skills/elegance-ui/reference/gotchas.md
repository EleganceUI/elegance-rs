# Gotchas

Things that fail, panic, or silently do nothing when building with Elegance in Rust. Each item was taken from the code
(`elegance-rs/elegance/src`), the docs, or the component/modifier/state reference files in this skill (which cite the
code). Items marked (inferred) are conclusions from reading the macro/source, not run.

## Contents

- [Threading](#threading)
- [Reactivity that silently does not update](#reactivity-that-silently-does-not-update)
- [Ownership and lifetimes](#ownership-and-lifetimes)
- [Errors: kinds, panics, the error handler](#errors-kinds-panics-the-error-handler)
- [API shape surprises](#api-shape-surprises)
- [Layout surprises](#layout-surprises)
- [Things that fail silently](#things-that-fail-silently)
- [Performance](#performance)
- [Build and platform](#build-and-platform)
- [Docs that disagree with the code](#docs-that-disagree-with-the-code)

## Threading

| Rule | Detail |
| --- | --- |
| All your code runs on the UI thread | view bodies, effects, event handlers, timers, tasks, dialog and notification callbacks. Drawing is on a separate render thread; you never touch it. |
| `State`, `Binding`, `Computed`, `Timer`, `Signal` are `!Send` | the compiler stops you from moving them into `std::thread::spawn` or `post_to_ui_thread` closures. |
| `State::set` / `write` off the UI thread panic | message `elegance: reactive state was used off the UI thread`. Not reachable from safe code because of `!Send`. |
| Hand results back with `post_to_ui_thread(task)` | callable from any thread; `task: impl FnOnce() + Send + 'static`; runs between events in order. Send plain data (`String`, `Vec`, `Result`), then write state inside the task. Wrapping a state-capturing callback in `send_wrapper::SendWrapper` (external crate) is the docs' pattern. |
| `every`/`after` off the UI thread panic | and need a running App. |
| No networking in Elegance | use `ureq`, `reqwest`, `tungstenite`, ...; a worker thread plus `post_to_ui_thread`. There is no `task(async fn)`. |
| A late task can outlive its view | check `state.is_alive()` before writing (see below). |
| `AppHandle` (from `App::handle()`) | Clone, can quit the app from a closure that cannot borrow the `App`. |

## Reactivity that silently does not update

1. Reading in a component body or plain function gives one value and never updates: `Text(count.get().to_string())`,
   `.opacity(x.get())`. Pass the handle, a closure, a `.map(..)`, or `"{count}"`.
2. Only a lone string literal with `{..}` interpolation inside `elg!` is reactive. `Text(format!("{x}"))` is not.
3. Reactive arguments are `IntoProp<T>`: a value, `State`, `Binding`, `Computed` or closure. A closure must return exactly
   the prop's type (`Progress` takes `Option<f32>`, so `|| 0.4_f32` does not compile; return `Some(0.4)`).
   Two branches returning `Color` and `ThemeColor` do not unify; convert one.
4. Not everything is reactive. Stack `spacing`/`alignment` (take `Into<Len>` / alignments), `Text::line_limit` (plain
   `i32`), `.theme(Theme)` (plain value) are one-shot. Check the signature: `impl IntoProp<..>` means reactive.
5. Two-way arguments (`impl IntoBinding<T>`: `Toggle`, `Checkbox`, `TextField`, `Slider`, `NumberInput`, `Picker`,
   `.popover`, `.selection`, `.focused`) take `State` or `Binding` (read/write) or `Computed`/closure (read-only). A plain
   `bool`/`String` is not accepted. A closure makes the control read-only: typing does not change it.
6. `.animation(anim, value)` needs the watched value to be a closure, `State`, `Computed` or `Binding`; a plain value does
   not compile (the docs say a plain value works; the code does not implement it).
7. Writing a value equal to the current one (`State::new`, needs `PartialEq`) notifies nothing. `write()`/`update` always
   notify. Types without `PartialEq` need `State::always_notify`.
8. A `ForEach` item whose value changed is rebuilt and loses state created in its content. Keep state in the parent, or use
   `ForEach::content_signal`. Item ids must be unique and stable; an index as id rebuilds rows when the list shifts.
9. Effects: `effect()` outside view building panics; effects that write what they read loop and abort after 100,000 runs;
   do not mirror derived values into state with an effect, use `computed`/`.map`.
10. A guard held across a conflicting access panics: `let g = s.read(); s.set(..)`. Keep guards temporary.
11. A plain function returning a view runs immediately and sees its caller's environment; a `#[component]` runs where it
    is placed. Use a component when the view reads the environment (`Environment::<K>::new()` panics outside a view body).
12. Reading environment/state directly in a body (`compact.get()`) is one-shot; read inside a closure.
13. Closures capturing state need `move`. Handles are `Copy`, so you can keep using them afterwards.
14. `count += 1` needs `let mut count` (the operator takes `&mut self`); `count.update(|c| *c += 1)` or `count.set(..)` does not.

## Ownership and lifetimes

- `State`/`Binding`/`Computed` are `Copy` handles into a thread-local arena owned by the view that created them. State
  created before `App::run` lives for the whole program. State created inside an `if` branch, `match` arm, `ForEach` item
  or window is freed when that goes away.
- Using a handle after that panics: `elegance: reactive state was used after the views that created it were removed`.
  Guard late callbacks with `state.is_alive()`; cancel timers in `on_disappear`.
- `Timer` drop does NOT cancel. A timer started while building views/in an effect/in `on_appear` ends with its view; one
  started from a click handler keeps running until it fires or is cancelled.
- Views are values: `.clone()` them to reuse (`{ badge.clone() }` in `elg!`). `elg!` blocks accept any `IntoView`.
- Component props: `Prop<T>` takes reactive inputs, `Binding<T>` is two-way (pass the parent's `State`).
- `asset!("path")` embeds `<crate>/assets/path` into the binary (include_bytes). `ImageSource::file(path)` resolves a
  relative path against the current directory, not the executable: use an absolute path for runtime files.
- `elegance-wgpu` has unsafe requirements: read its README before using `GpuImage`.

## Errors: kinds, panics, the error handler

`elegance::Error` has `kind()` and `message()`; fallible calls (loading a font/sound, opening a camera, file dialogs'
setup, etc.) return `Result<_, Error>`.

| `ErrorKind` | Meaning | Typical cause |
| --- | --- | --- |
| `Usage` | API misuse (the C++ `UsageError`) | icon name that does not exist, SVG path that does not parse, `.resizable()` on a non-image, NaN timer delay, state used off the UI thread, wrong child count |
| `Resource` | file or resource could not be loaded (`ResourceError`) | missing font/sound/image file |
| `Gpu` | no suitable GPU, out of memory, device lost unrecoverably | headless machine, driver |
| `Platform` | the OS could not do it (`PlatformError`) | notifications, tray, audio, camera unsupported on this platform |
| `Elegance` | any other Elegance error | |
| `Other` | not an Elegance error: a panic or a foreign C++ exception | check `AppError::panicked()` |

- Usage mistakes panic with a message that says what and where (they are bugs, not for handling). Functions that return
  `()` and hit a platform failure panic through the internal `ffi!` wrapper (inferred); `Result`-returning ones return
  `Err`.
- When your own code panics (component body, effect, event handler, task, timer, `on_appear`/`on_disappear`, layout, canvas
  draw) Elegance logs it to stderr and the app keeps running. Install a handler to change that:

```rust
App::new()
    .window(WindowDesc::new("App"), Root)
    .on_error(|error| {
        eprintln!("{:?} failed: {}", error.code(), error.message());
        if error.panicked() { ErrorAction::Quit } else { ErrorAction::Resume }
    })
    .run()   // Result<ExitCode, Error>; after Quit, run() resumes the panic or returns the Err
```

| `AppError` method | Returns |
| --- | --- |
| `message()` | the panic message or error text |
| `panicked()` | true if Rust code panicked |
| `kind()` | `ErrorKind` (`Other` for panics) |
| `code()` | `AppCode`: `Body`, `Effect`, `EventHandler`, `Task`, `Timer`, `LifecycleAction`, `Layout`, `Drawing`, `Callback` |

- The handler runs on the UI thread between events. Repeated identical failures of the same kind in the same or next frame
  are only counted ("... 42 times in a row"), and the handler is not called again for them.
- A failed event handler ends the event as handled (press ends, gestures reset). A failed body shows nothing in its
  place. A failed canvas draws nothing that frame. A failed layout gives its view no size until invalidated. A repeating
  timer keeps going.
- If the handler itself panics it is only logged. Errors Elegance raises outside your code come back from `run()` as `Err`
  after a clean shutdown.
- `main` should return `Result<std::process::ExitCode, Error>` and use `?` for `Sound::load(..)` etc.

## API shape surprises

- Rust has no default arguments. `Animation::spring(response, damping_fraction)` and `smooth(duration)` need arguments;
  `.padding(..)`, `focusable(bool)`, `accessibility_hidden(bool)`, `repeat_count(n, autoreverses)` too. Overloads are
  suffixed: `_with` (options builder), `_in` (with a shape), `_view`, `_edges`, `_xy`.
- Named arguments in `elg!` are builder calls: `Name(a, key: v)` is `Name::elg_new((a,)).key(v)`; `.m(a, key: v)` is
  `.m_with(a, |o| o.key(v))`. Unknown leading-dot names are compile errors.
- Leading-dot values resolve by the parameter type: `.leading` is `HorizontalAlignment::Leading` on a stack and
  `Alignment::LEADING` on a frame. `.infinity` for frames. `elg!` interpolation uses `{x}` of `State` via `Display`.
- `LazyVStack`/`LazyHStack` take exactly one child view in `elg!` (the `ForEach`); more panic. `NavigationSplitView` needs
  2 or 3 children, otherwise panics.
- `ForEach(items, id: |x| x.id) { x in .. }` takes an `ItemsSource` (a `State<Vec<T>>` in the docs); the id must implement
  `IntoItemKey` (integers, `String`, `&str`, `ItemKey`; see components.md).
- `TextArea` has no `.secure`/`.line_limit`; only `TextField` does. Text truncation is end-ellipsis only; no
  head/middle/no-wrap options exist.
- Shapes: Rust has marker types and a concrete `Shape` struct, not a `Shape` trait. Use `CustomShape`/`PathShape` for
  custom outlines.
- Modifier named-argument completion: `BlurOptions`, `ClickOptions`, `HoverDelay`, `DropOptions` lack `#[elg_builder]`;
  named arguments still compile but leading-dot completion may not resolve for them (inferred). Use the explicit form.
- `on_sized` is deprecated: use `on_size_change`.
- There is no `help` modifier (docs list one); use `.tooltip`.
- There is no clipboard API, no drag source (only `on_drop` receiving), no SVG image file source (SVG path data via
  `Path::svg`), no runtime opening of extra windows in Rust (unverified; see components.md).
- Menu items' `shortcut` is only a text hint; real key bindings come from `KeyboardShortcut` on `MenuBarItem`.

## Layout surprises

- A view is only as big as its content and is centered: fill space with `.frame(max_width: .infinity, max_height: .infinity,
  alignment: ..)` (or `Spacer`). Frames limit, they do not force: a child larger than the frame still overflows.
- Modifier order matters: `.padding(8).background(c)` colors the padding; `.background(c).padding(8)` does not. Read a chain
  inside out.
- `Spacer` expands only along the stack's main axis; inside a `ScrollView` the content gets unbounded space on the scroll
  axis (use `LazyVStack`, not `VStack`, for long lists).
- HStack aligns Top/Center/Bottom only; there is no text-baseline alignment despite what `layout.mdx` says.
- Text wraps when the width is bounded and truncates with an ellipsis when `line_limit` is hit.
- Shapes fill what they are offered; give them a `.frame(..)`.
- Gaps (a `Spacer` or empty area between children) take no clicks unless the container has `.content_shape(Rectangle())`
  (drawing.mdx, input.mdx). For `fixed_size` and `layout_priority`, see modifiers.md.

## Things that fail silently

| Symptom | Cause |
| --- | --- |
| Text/color/opacity never changes | one-shot read in the body (see Reactivity 1, 2) |
| Image does not appear | missing or unreadable file or unknown icon name: icon names panic (Usage), files log `[elg:warning] image: can't read ...` on stderr; run from a terminal |
| Image blurry / wrong size | forgot `.resizable()`; `asset!` path relative to `<crate>/assets/` |
| Control does nothing when clicked | `.disabled(true)` ancestor; `.allows_hit_testing(false)`; a view with only a clear background has no hit area, use `.content_shape(..)` |
| Window background/transparency not as expected | `WindowMaterial` support is platform-specific (see theming-and-style.md) |
| Repeated callback errors vanish after the first | identical failures are only counted (see Errors) |
| GPU validation errors | logged as `[elg:error] webgpu: ...` in every build, the app goes on |
| `#[preview]` seems ignored in `cargo run` | by design: previews register only in preview builds |
| Linux: app fails to start with missing `libaccesskit.so` | add the rpath config (see preview.md Troubleshooting) |
| Dismissing a popover does nothing | passed a `Computed`/closure (read-only) instead of a `State`/`Binding` |

## Performance

- Elegance redraws only damaged regions and presents no frame when nothing visible changed. Reactive closures are fine
  grained; prefer many small reactive closures to one large `if` that rebuilds big subtrees.
- Use `LazyVStack`/`LazyHStack` or `VGrid`/`HGrid` inside `ScrollView` for long lists; plain stacks build every child.
- `ForEach` with stable ids keeps row views and their state; changing ids rebuilds rows.
- Effects that need extra drawing passes cost more: blur and backdrop blur, masks, blend modes, compositing groups,
  non-uniform scaled content (rendered offscreen). Use them deliberately; the inspector (F12) points out such views.
- Do heavy work off the UI thread and post results; the UI thread runs view code and event handlers.
- `Computed` is lazy and recomputes only when read after a dependency changed; derive instead of duplicating state.
- Use the image cache controls (`set_image_cache`, `clear_image_cache`; env `ELG_IMAGE_CACHE=off`) for `AsyncImage`-heavy UIs.
- Each hot reload stays in memory until the host quits.

## Build and platform

- Rust 1.88+ (edition 2024) per README (the let-chains in the crates need it); `elegance-wgpu` README says 1.93+; no
  `rust-version` in Cargo.toml.
- Prebuilt targets: Windows MSVC x86_64/aarch64, macOS arm64/x86_64 (13.3+), Linux x86_64/aarch64 (glibc 2.35). Other
  targets build Elegance from source (CMake, vcpkg; long first build).
- `Platform` is `#[non_exhaustive]`: `Windows`, `MacOS`, `Linux`, `Mikros`, and `Unknown(i32)` for a value the
  binding doesn't name (later library versions may add platforms), so match with a `_` arm. On Mikros the C++ side
  reports `PlatformError` for notifications, tray, audio output/recording, camera and video playback (from the gap
  analysis of the milestone-15 worktree headers).
- Mikros (`x86_64-unknown-mikros`): elegance-sys links the system's `eleganceui.0.dlib` dynamically and nothing else
  (no prebuilt, no CMake); `ELEGANCE_ROOT`, `ELEGANCE_BUILD_DIR`, `ELEGANCE_NATIVE_DIR`, `ELEGANCE_PREBUILT*`, ... and
  the `dynamic`/`preview` features fail the build there. The linker must find the library (the sysroot, or `-L`).
- Linux/macOS/Windows differences for dialogs, notification, tray: see components.md (Linux behaviour unverified).
- Preview needs a source build and a display; see preview.md.

## Docs that disagree with the code

| Docs say | Code says |
| --- | --- |
| `.animation(.., plain_value)` is allowed | `AnimationValue` implemented for closures, `State`, `Computed`, `Binding` only |
| `Animation::spring()` no arguments (design.md/animation.mdx) | `spring(response, damping_fraction)` |
| state reads off-thread throw in debug builds | in Rust, writes always panic; handles are `!Send` |
| `resolve_grid_tracks` is C++-only (grids.mdx) | exported by Rust, with `grid_tracks_length` and `GridTrack` |
| `elegance::desktop` items are not in the prelude (desktop.mdx) | `lib.rs` re-exports them in the prelude |
| `help` popup modifier (views-and-modifiers.mdx) | not in Rust nor C++; use `tooltip` |
| `SplitView` view (views-and-modifiers.mdx) | Rust has `HSplitView` and `VSplitView` |
| `ColorSchemeKey` defaults to the system setting | `default_value` is `Light`; the window sets the real one |
| native library `elegance_c` is shipped beside the exe (installation.mdx) | statically linked; only AccessKit (and Dawn's DXC DLLs on Windows) ship beside it |
| `ELG_SHOW_DAMAGE`, `ELG_FULL_REDRAW` (debugging.mdx) | names not found in the C++ sources checked |
| `elegance-wgpu` README: `unsafe render` | `Bridge::render` is safe with a restricted closure; `render_raw` is the unsafe one |
