# State and reactivity (Elegance, Rust)

Purpose: how data changes drive the UI: `State`, `Binding`, `Computed`, effects, reactive props, timers, animation, background work, and the environment. Names come from `elegance-rs/elegance/src/{reactive,prop,timer,animation,app,environment,dsl}.rs`, `view.rs`, and the `elg!` macros. `use elegance::prelude::*;` brings in everything below (the `*Key` types are in the prelude too).

## Table of contents

- [Mental model](#mental-model)
- [How an update happens](#how-an-update-happens)
- [State](#state)
- [ReadRef and WriteRef](#readref-and-writeref)
- [Binding](#binding)
- [IntoBinding](#intobinding)
- [Signal](#signal)
- [Computed and computed](#computed-and-computed)
- [effect](#effect)
- [untrack](#untrack)
- [batch](#batch)
- [Ownership, Copy and lifetime rules](#ownership-copy-and-lifetime-rules)
- [Reactive arguments: Prop and IntoProp](#reactive-arguments-prop-and-intoprop)
- [ResolveColor and ResolveStyle](#resolvecolor-and-resolvestyle)
- [Reactive text, if, match and ForEach in elg!](#reactive-text-if-match-and-foreach-in-elg)
- [How a component body runs](#how-a-component-body-runs)
- [Timers: after, every, Timer](#timers-after-every-timer)
- [Animation](#animation)
  - [Animation constructors](#animation-constructors)
  - [with_animation](#with_animation)
  - [The animation modifier and AnimationValue](#the-animation-modifier-and-animationvalue)
  - [Transition and Pose](#transition-and-pose)
  - [Which things animate](#which-things-animate)
- [Threads: post_to_ui_thread and AppHandle](#threads-post_to_ui_thread-and-apphandle)
- [Environment](#environment)
  - [Environment, EnvironmentKey, EnvironmentValues](#environment-environmentkey-environmentvalues)
  - [Built-in keys](#built-in-keys)
  - [Custom keys](#custom-keys)
  - [Setting and reading](#setting-and-reading)
- [Pitfalls](#pitfalls)
- [Recipes](#recipes)
  - [Background work with loading and error state](#recipe-background-work-with-loading-and-error-state)
  - [Debounce and timers](#recipe-debounce-and-timers)
  - [Animated list](#recipe-animated-list)
  - [Spinner that runs while a view is shown](#recipe-spinner-that-runs-while-a-view-is-shown)
- [C++-only / not exposed in Rust](#c-only--not-exposed-in-rust)
- [Unverified items](#unverified-items)

## Mental model

- Your view code (functions, `#[component]` bodies) runs once. What re-runs is the small closures you give to Elegance where a value is expected (a "reactive argument"), plus computed values and effects.
- Reading state inside a closure that Elegance runs subscribes that closure. Writing the state re-runs exactly those closures.
- All state is UI-thread only. The handles are `Copy` but not `Send` or `Sync`.
- Everything that is not run by Elegance (the component body itself, a plain helper function) reads state once and never updates.

## How an update happens

1. An event handler (click, key, timer action, a `post_to_ui_thread` task) writes state: `count += 1`, `count.set(5)`, `*todos.write() = ...`.
2. The write notifies the state's node. Every subscribed closure (a reactive prop, a `Computed`, an `effect`) is marked dirty and scheduled. `Computed` values are lazy: they recompute only when read.
3. Writes made while handling one event are grouped automatically. When the event handler returns, the scheduled closures run once each, before the next frame is drawn. A closure that reads three states changed by one click runs once.
4. Two events in the same frame are flushed after each, so a closure can run twice in one frame.
5. A state written with a value equal to its current one (`State::new`, which requires `PartialEq`) notifies nothing. `State::always_notify` and `write()` always notify (the guard cannot tell what changed). A `Computed` that recomputes to an equal value does not notify its readers.
6. `batch(|| ...)` groups writes made outside an event (a library callback, a task you run yourself). The outermost batch ends, then effects run immediately. Inside an event handler, effect, computed or layout a batch leaves flushing to the end of that outer context (effects cannot run inside those).
7. `with_animation(anim, || ...)` marks the writes inside as animated: the closures they trigger run inside that transaction, and animatable properties interpolate.
8. Effect loop guard (C++ runtime, reaches Rust as a panic): more than 100,000 effect runs in one flush is reported as "effects keep re-triggering each other; an effect probably writes state it also reads".

## State

```rust
pub struct State<T: 'static>            // Copy + Clone + PartialEq (same handle) + Eq
impl<T: PartialEq + 'static> State<T> { pub fn new(initial: T) -> Self }
impl<T: Default + PartialEq + 'static> Default for State<T>
impl<T: 'static> State<T> {
    pub fn always_notify(initial: T) -> Self
    pub fn read(&self) -> ReadRef<T>                       // tracked
    pub fn peek(&self) -> ReadRef<T>                       // untracked
    pub fn get(&self) -> T where T: Clone                  // tracked copy
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R    // tracked, by reference
    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R
    pub fn map<R: PartialEq + 'static>(&self, f: impl Fn(&T) -> R + 'static) -> Computed<R>
    pub fn write(&self) -> WriteRef<T>                     // UI thread only
    pub fn set(&self, value: T)                            // UI thread only
    pub fn update(&self, change: impl FnOnce(&mut T))
    pub fn toggle(&self) where T: Not<Output = T> + Clone
    pub fn binding(&self) -> Binding<T>
    pub fn is_alive(&self) -> bool
}
impl<T: AddAssign<U> + 'static, U> AddAssign<U> for State<T>   // needs `mut` binding: `let mut count`
impl<T: SubAssign<U> + 'static, U> SubAssign<U> for State<T>
impl<T: Display + 'static> Display for State<T>                // tracked read, so Text("{count}") works
impl<T: Debug + 'static> Debug for State<T>                    // untracked; prints State(<removed>) when dead
```

| Method | Subscribes | Use when |
| --- | --- | --- |
| `get()` | yes | you need an owned copy (`T: Clone`): `count.get() * 2` |
| `read()` | yes | you need to look inside without cloning: `todos.read().is_empty()`, `*count.read()` |
| `with(f)` | yes | one expression by reference, no guard to manage: `todos.with(Vec::is_empty)` |
| `peek()` / `with_untracked(f)` | no | inside an event handler or when a read must not subscribe |
| `set(v)` | n/a | replace the value; no-op when equal (for `State::new`) |
| `update(f)` | n/a | edit in place: `count.update(\|c\| *c += 2)` |
| `write()` | n/a | edit in place through a guard: `todos.write().push(t)`, `*count.write() += 1`; notifies when the guard drops |
| `toggle()` | n/a | flip a bool: `open.toggle()` |
| `map(f)` | n/a | a derived `Computed<R>` from this one state: `draft.map(String::is_empty)` |
| `binding()` | n/a | explicit two-way handle; usually unnecessary because a `State` is accepted wherever a binding is |

Prefer `State::new` for ordinary types. Use `State::always_notify` when `T` has no `PartialEq` (every write notifies). `+=`/`-=` need a `let mut` binding (the operator takes `&mut self`) but they still write the shared value; `count.update(|c| *c += 1)` works without `mut`.

Off-thread rules (checked in code): `write` and `set` call the UI-thread check and panic with `elegance: reactive state was used off the UI thread ...` if called on another thread. Reads are checked only in C++ debug builds. In Rust you cannot get there without unsafe, since handles are `!Send` and the arena is thread-local.

```rust
let mut count = State::new(0);
let name = State::new(String::new());
let todos = State::new(Vec::<String>::new());
count += 1;                          // AddAssign
count.set(5);
todos.write().push("milk".into());   // guard drops at end of statement, then subscribers run
let n = *count.read();
let empty = todos.with(Vec::is_empty);
elg! { Text("Count: {count}") }      // follows count
```

## ReadRef and WriteRef

```rust
pub struct ReadRef<T: 'static>     // Deref<Target = T>, Display, Debug. From State::read/peek, Computed::read
pub struct WriteRef<T: 'static>    // Deref + DerefMut, drop notifies. From State::write
```

- Both hold a `RefCell` borrow. Drop a `ReadRef` before writing the same state, and never hold two `WriteRef`s: reading while a write guard is alive panics (`state read while it's being written`), writing while a read guard is alive panics (`state written while it's being read`).
- `WriteRef` notifies when it is dropped, not at each deref.

```rust
let done = todos.read().iter().filter(|t| t.is_empty()).count();  // guard is a temporary, dropped at `;`
todos.write().clear();
// BAD: let first = todos.read(); todos.write().clear();  // panics: first is still alive
```

## Binding

```rust
pub struct Binding<T: 'static>                  // Copy + Clone
impl<T> From<State<T>> for Binding<T>
impl<T: 'static> Binding<T> {
    pub fn new(get: impl Fn() -> T + 'static, set: impl Fn(T) + 'static) -> Self
    pub fn constant(value: T) -> Self where T: Clone      // writes ignored
    pub fn read_only(get: impl Fn() -> T + 'static) -> Self  // reads reactively, ignores writes
    pub fn get(&self) -> T where T: Clone
    pub fn set(&self, value: T)
    pub fn update(&self, change: impl FnOnce(&mut T)) where T: Clone  // untracked get, change, set
}
impl<T: AddAssign<U> + Clone + 'static, U> AddAssign<U> for Binding<T>   // and SubAssign
impl<T: Display + Clone + 'static> Display for Binding<T>
pub fn constant<T: Clone + 'static>(value: T) -> Binding<T>
```

A `Binding` is read/write access to a value owned elsewhere. Controls that edit a value (`TextField`, `Toggle`, `Slider`, popover `is_presented`) and components that change their parent's state take one. There are no `read`, `with`, `peek`, `map` or `toggle` methods on `Binding`: call `get`/`set`/`update`.

Prefer a `State` over a `Binding` in your own props? No: a component that edits a value takes `Binding<T>` (any `State` converts with `.binding()`), a component that only reads takes `Prop<T>`.

```rust
#[component]
fn VolumeControl(volume: Binding<f32>) -> impl IntoView {
    elg! { HStack { Image("speaker-high")  Slider(volume) } }
}
let volume = State::new(0.5_f32);
elg! { VolumeControl(volume: volume.binding()) }

// Make one by hand: edit part of a value, or convert units.
# #[derive(Clone, PartialEq)] struct Todo { done: bool }
let todos = State::new(vec![Todo { done: false }]);
let index = 0;
let done = Binding::new(
    move || todos.read()[index].done,
    move |value| todos.write()[index].done = value,
);
let percent = Binding::new(move || volume.get() * 100.0, move |p| volume.set(p / 100.0));
elg! { Toggle("Always on", constant(true)) }   // fixed value, edits ignored (previews)
```

## IntoBinding

```rust
pub trait IntoBinding<T: 'static> { fn into_binding(self) -> Binding<T>; }
// implemented for: State<T>, Binding<T>, Computed<T> (read-only), any F: Fn() -> T + 'static (read-only)
```

What two-way arguments (Toggle value, popover `is_presented`, ...) accept. A `Computed` or closure becomes a read-only binding (writes ignored).

```rust
.popover(editing, content)                          // State: dismissing sets editing to false
.popover(move || todos.read().is_empty(), content)  // closure: shown while the list is empty
.popover(todos.map(Vec::is_empty), content)         // Computed: the same
```

Elegance never makes a constant binding for you: passing a plain value to a control that edits it is a compile error.

## Signal

```rust
pub struct Signal<T: 'static>(Rc<dyn Fn() -> T>)     // Clone (not Copy)
impl<T: 'static> Signal<T> { pub fn new(get: impl Fn() -> T + 'static) -> Self;  pub fn get(&self) -> T }
impl<T: Clone> From<State<T>> / From<Binding<T>> / From<Computed<T>> for Signal<T>
impl<T> IntoProp<T> for Signal<T>
```

Read-only access (a tracked read) to anything that can change. In practice it is what `ForEach::content_signal` hands your content closure: `Text(move || t.get().title)`. Rarely needed elsewhere; use `Prop<T>` for props of your own components.

## Computed and computed

```rust
pub struct Computed<T: 'static>                       // Copy + Clone
impl<T: PartialEq + 'static> Computed<T> { pub fn new(function: impl Fn() -> T + 'static) -> Self }
impl<T: 'static> Computed<T> {
    pub fn always_notify(function: impl Fn() -> T + 'static) -> Self
    pub fn get(&self) -> T where T: Clone
    pub fn read(&self) -> ReadRef<T>
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R
}
impl<T: Display + 'static> Display for Computed<T>
pub fn computed<T: PartialEq + 'static>(function: impl Fn() -> T + 'static) -> Computed<T>
```

A value derived from state. Lazy and memoized: recomputed at most once per change and only when read; if it recomputes to an equal value (`PartialEq`), readers do not re-run. All reads are tracked. There is no `peek`, `map` or `is_alive` on `Computed`.

Prefer `state.map(f)` when it depends on one state; prefer `computed(move || ...)` when it reads several. Prefer a plain closure `move || ...` over `computed` when only one place uses it and you don't need memoizing (a closure re-runs for each subscriber; a `Computed` is shared and compares results).

```rust
let visible = computed(move || todos.read().iter().filter(|t| show_done.get() || !t.done).cloned().collect::<Vec<_>>());
let remaining = todos.map(|list| list.iter().filter(|t| !t.done).count());
elg! { Text("{remaining} left") }
```

## effect

```rust
#[track_caller] pub fn effect(function: impl Fn() + 'static)
```

Runs `function` now and again whenever state it read changes (after the current event, before the next frame). Use it to keep something outside the UI in step with state (save settings, window title, kick off a request). Must be called while building views (a component body, a reactive argument); calling it elsewhere (for example in a click handler or before `App::run`) fails with a UsageError panic ("effect() must be called while building views"). It ends when the views that made it are removed. Timers and effects started by a later run belong to the same view.

```rust
effect(move || save_settings(theme.get(), font_size.get()));
```

Never use an effect to compute a value for display (use `computed`/`map`/a closure). An effect that writes state it also reads loops; see [Pitfalls](#pitfalls).

## untrack

```rust
pub fn untrack<R>(function: impl FnOnce() -> R) -> R
```

Reads state inside `function` without subscribing the running closure. Use for "as of" values that must not trigger a re-run.

```rust
Text(move || format!("{} (as of {})", value.get(), untrack(|| updated_at.get())))
```

## batch

```rust
pub fn batch<R>(function: impl FnOnce() -> R) -> R
```

Groups writes so effects run once after the outermost batch. Event handlers are already batched; use it in callbacks that come from outside an event (a `post_to_ui_thread` task is one such place: group several writes there).

```rust
batch(|| { first_name.set("Ada".into()); last_name.set("Lovelace".into()); });
```

## Ownership, Copy and lifetime rules

- `State`, `Binding` and `Computed` are `Copy` handles into a thread-local arena. Capture them with `move`; copies refer to the same value.
- A value lives as long as the view that created it. State created while building a view is freed when that view is removed: an `if`/`If` branch switching, a `match` arm changing, a `ForEach` item going away or being rebuilt, the window closing. State created outside any view (before `App::run`) lives for the whole program.
- Using a handle after its owner is gone panics: `elegance: reactive state was used after the views that created it were removed`. Check `state.is_alive()` first when a late callback (a thread result, a timer you started in a click) might outlive the view.
- Group state in a plain `#[derive(Clone, Copy)] struct Player { playing: State<bool>, volume: State<f32> }` to pass a model around. For values far apart, use the [environment](#environment).
- State that must survive an `if` branch or a `ForEach` rebuild belongs in a parent or outside the view.
- `ForEach` rebuilds an item whose value changed (and drops state created in its content); `ForEach::content_signal` keeps the view while the key exists and gives you a `Signal<T>` that follows the new values.

## Reactive arguments: Prop and IntoProp

```rust
pub enum Prop<T: 'static> { Value(T), Fn(Rc<dyn Fn() -> T>), Env(Rc<dyn Fn(&EnvironmentValues) -> T>), Theme(ThemeColor) }
impl<T: 'static> Prop<T> {
    pub fn from_fn(f: impl Fn() -> T + 'static) -> Self
    pub fn from_env(f: impl Fn(&EnvironmentValues) -> T + 'static) -> Self
    pub fn is_reactive(&self) -> bool
    pub fn needs_environment(&self) -> bool
    pub fn static_value(&self) -> Option<&T>
    pub fn get(&self, environment: &EnvironmentValues) -> T where T: Clone
    pub fn try_get(&self) -> Option<T> where T: Clone         // None when it needs an environment
    pub fn resolved_in(self, environment: &EnvironmentValues) -> Prop<T> where T: Clone
}
pub trait IntoProp<T: 'static> { fn into_prop(self) -> Prop<T>; }
```

A reactive argument is one effect that updates one property of one element. Whenever a modifier or view parameter is `impl IntoProp<T>` it accepts:

| You pass | Result |
| --- | --- |
| plain `T` (and `&str`/`&String`/`Cow<str>` for `String`) | static |
| `State<T>`, `Binding<T>`, `Computed<T>` (`T: Clone`) | follows it |
| `Signal<T>` | follows it |
| closure `move \|\| value` returning exactly the type | reactive; closures are typed per type: `f32, f64, i32, bool, Size, Font, Weight, EdgeInsets, ColorScheme, Theme, HoverDelay, PointerStyle, Option<f32>`, any `R: Into<String>` for `String`, and `R: ResolveColor` / `R: ResolveStyle` for `Color` / `ShapeStyle` |
| numbers (`i32`, `f64`, `u8`, ...) | convert to `f32` props (`.opacity(1)` is fine); `f64` props (number-input bounds) take `f32` and integers too |
| `ThemeColor` or a dot marker (`.accent`) for `Color`/`ShapeStyle` | `Prop::Theme`, resolved in the environment where it is used |
| `"Text {name}"` literal inside `elg!` | an `Interpolated` reactive string (see below) |

Other facts:
- `Prop::Env` / `Prop::Theme` props need the environment where they are used (theme colors follow the color scheme). A closure returning `ThemeColor` or `.secondary` for a color prop becomes a `Prop::Env`.
- Only the listed value types can be reactive. Passing a closure for some other type is a compile error ("can't be used as a reactive `T` argument").
- In `#[component]`, a prop declared `Prop<T>` is reactive and its setter takes `impl IntoProp<T>`. A prop declared as a plain type `T` takes `impl Into<T>` and is read once. Use `Binding<T>` for props the component edits.

```rust
.opacity(0.5)                                              // static
.opacity(fade)                                             // State<f32>
.opacity(move || if hovered.get() { 1.0 } else { 0.6 })   // derived
.foreground_style(move || if selected.get() { ThemeColor::OnAccent } else { ThemeColor::Secondary })
Prop::from_env(|env| ShapeStyle::Color(env.get::<TintKey>().opacity(0.5)))  // from the environment where used
```

## ResolveColor and ResolveStyle

```rust
pub trait ResolveColor: 'static { fn resolve(self, environment: &EnvironmentValues) -> Color; }
pub trait ResolveStyle: 'static { fn resolve_style(self, environment: &EnvironmentValues) -> ShapeStyle; }
```

Implemented for `Color`, `ThemeColor` and the theme-color dot markers (`ResolveColor`); and for `ShapeStyle`, `Color`, `ThemeColor`, `LinearGradient`, `RadialGradient`, `EllipticalGradient`, `AngularGradient`, `BackdropFill` and the dot markers (`ResolveStyle`). They are what lets one closure return either a literal color or a theme color: `.fill(move || if on.get() { Color::hex(0x9ECE6A) } else { ThemeColor::Secondary })` does NOT compile (two types); return one type from both branches (for example make both `Color`s, or both `ThemeColor`). You rarely name these traits yourself.

## Reactive text, if, match and ForEach in elg!

`elg!` rewrites these so they follow state without you writing closures:

| In `elg!` | Expands to | Notes |
| --- | --- | --- |
| `Text("Count: {count}")` (a lone string literal argument with `{...}`) | `Interpolated(move \|\| format!(...))` | reactive string where Elegance takes one (`Text`, accessibility labels, any `IntoProp<String>`); a reactive `Text` as a control's label; formatted once where a plain `String` is required. `{count}` works on `State`, `Computed`, `Binding` and plain values; `{user.name}`, `{size.0}` field chains are read by reference through the root; other expressions `{a + b}` are evaluated inside the closure; `{x:>8}` format specs work; `{{` and `}}` are literal braces |
| `if cond { A } else { B }` | `if_view(move \|\| -> bool { cond }, A, B)` | the condition is a reactive closure, so `if loading.get() {...}` follows `loading`. Switching branches removes the old branch's views and the state they created. No `else`: nothing is shown |
| `match value { pat => view, ... }` | `match_view(move \|\| value, \|v\| match v.clone() {...})` | `value` must be `Clone + PartialEq`. When the value changes, the old arm's view (and its state) is removed and the new one built, as a one-item `ForEach` does |
| `ForEach(items, id: \|t\| t.id) { item in ... }` | `ForEach::new(items).id(...).elg_content(move \|item\| ...)` | `items` may be a `Vec`, `State/Computed/Binding<Vec<T>>`, a closure returning `Vec<T>` (tracked) or an integer range. `item` is `&T`. Items with unchanged key and value keep their view and state; changed values are rebuilt |
| `If(cond, then, else)` outside `elg!` | `pub fn If(condition: impl IntoProp<bool>, then_view: impl IntoView, else_view: impl IntoView) -> View` | `else` can be `()` for nothing |

Plain-`String` places do not follow state: inside `elg!`, `Text(count.get().to_string())` shows one value forever; use `Text("{count}")` or `Text(move || count.get().to_string())`.

## How a component body runs

```rust
#[component]
fn SettingRow(title: String) -> impl IntoView {
    let compact = Environment::<CompactKey>::new();   // runs ONCE, when the row is placed in the tree
    elg! { Text(title.clone()).font(move || if compact.get() { Font::footnote() } else { Font::body() }) }
}
```

- A `#[component]` body (and `Component::body`) runs once when the view is placed, in the environment of where it ends up; it never re-runs on state changes. Only closures inside it re-run. `Component::body(&self)` is called through `View::component(...)`.
- So `if todos.read().is_empty() { ... }` written in plain Rust in the body decides once. Put it in `elg!` (`if` is reactive there) or pass `move || todos.read().is_empty()` / `todos.map(Vec::is_empty)`.
- Create state, `effect`s and timers in the body: they belong to this view and end with it.
- A plain function that returns a view runs before its view is placed, so it sees the caller's environment, not its own. Use a component to read the environment where the view ends up.
- Props: a required prop that is missing is a compile error; `#[prop(default)]` / `#[prop(default = expr)]` set defaults.

## Timers: after, every, Timer

```rust
#[track_caller] pub fn after(seconds: f64, action: impl FnOnce() + 'static) -> Timer
#[track_caller] pub fn every(seconds: f64, action: impl FnMut() + 'static) -> Timer
pub struct Timer                  // Default (never active), Clone (same timer), Debug; !Send
impl Timer { pub fn cancel(&self);  pub fn active(&self) -> bool }
```

- `after`: runs once after `seconds`. 0: as soon as the current event is handled; negative counts as 0; infinity: never fires but stays active until cancelled or its view goes. NaN panics.
- `every`: first run `seconds` from now, interval at least 1 ms. If the app falls behind, missed runs are skipped, not run back to back.
- Thread rules: UI thread only. Both panic when called off the UI thread of a running App. Actions run on the UI thread, and state written there takes effect like a click's. Closures are not `Send`, so capturing `State` is fine.
- Ownership: a timer started while views are built, in a reactive closure, an `effect`, or in `on_appear` belongs to that view and stops when it goes away. A timer started anywhere else (a click handler) runs until it has fired (`after`) or is cancelled (`every`), even after its view is gone: guard with `is_alive()` or `cancel()` in `on_disappear`.
- Dropping a `Timer` does NOT cancel it. Cancel explicitly. `cancel()` is idempotent. `active()` is false once cancelled, fired (for `after`), or its view is gone.

```rust
let mut seconds = State::new(0);
elg! { Text("{seconds}").on_appear(move || { every(1.0, move || seconds += 1); }) }   // ends with the view
let hide = after(3.0, move || loading.set(false));
hide.cancel();
```

## Animation

### Animation constructors

```rust
pub struct Animation          // Clone, Default = ease_in_out(0.35)
impl Animation {
    pub fn linear(duration: f64) -> Self
    pub fn ease_in(duration: f64) -> Self
    pub fn ease_out(duration: f64) -> Self
    pub fn ease_in_out(duration: f64) -> Self
    pub fn spring(response: f64, damping_fraction: f64) -> Self   // response ~ seconds; 1.0 = no overshoot, lower bounces
    pub fn smooth(duration: f64) -> Self                          // spring without bounce (usual UI choice)
    pub fn custom(curve: impl Fn(f64) -> f64 + 'static, duration: f64) -> Self   // time 0..1 -> progress; non-finite result falls back to linear
    pub fn timing_curve(x1: f64, y1: f64, x2: f64, y2: f64, duration: f64) -> Self   // CSS cubic-bezier
    pub fn delay(&self, seconds: f64) -> Self                     // negative starts partway in
    pub fn repeat_count(&self, count: i32, autoreverses: bool) -> Self   // count < 1 counts as 1
    pub fn repeat_forever(&self, autoreverses: bool) -> Self      // until the value changes or the view goes
    pub fn progress(&self, seconds: f64) -> f64
    pub fn finished(&self, seconds: f64) -> bool
}
```

Times are seconds. Rust has no default arguments: `Animation::spring()` does not exist, write `Animation::spring(0.5, 0.825)`. Autoreverses is always explicit (a spinner: `repeat_forever(false)`).

Inside `elg!` the dot values `.linear` (0.25 s), `.ease_in` (0.25), `.ease_out` (0.25), `.ease_in_out` (0.35), `.spring` (0.5, 0.825), `.smooth` (0.4) exist for `Animation` parameters (`dot.rs`). Transitions: `.opacity`, `.identity`, `.scale` (from 0.0), `.slide`.

Prefer `smooth`/`spring` for things people move or toggle, `ease_out` for short appear effects, `linear` for spinners and progress.

### with_animation

```rust
pub fn with_animation<R>(animation: impl Into<Animation>, changes: impl FnOnce() -> R) -> R
```

Explicit animation: everything that changes because of the writes inside `changes` animates (colors, opacity, offsets, sizes, positions; `If`/`ForEach` use transitions). Calls nest; the inner animation applies inside, the outer one again after it returns. A change that arrives while an animation runs starts from the current value, so nothing jumps.

```rust
Button("Toggle", move || with_animation(Animation::spring(0.5, 0.825), || expanded.toggle()))
```

### The animation modifier and AnimationValue

```rust
// ViewExt (view.rs)
fn animation(self, animation: impl Into<Animation>, value: impl AnimationValue) -> View
fn transition(self, transition: impl Into<Transition>) -> View
fn animates_layout(self, enabled: bool) -> View            // false: this view and its content snap to new positions/sizes
// animation.rs
pub trait AnimationValue: 'static            // implemented for: F: Fn() -> T (T: PartialEq), State<T>, Computed<T>, Binding<T> (T: PartialEq + Clone)
```

Implicit animation: `.animation(anim, value)` makes changes inside the view animate whenever `value` changed in the same update, wherever the change came from. Prefer it over `with_animation` when a view should always animate on a value; prefer `with_animation` when only some writes should animate.

```rust
elg! {
    Circle()
        .fill(move || if on.get() { Color::hex(0x9ECE6A) } else { Color::hex(0x565F89) })
        .offset(move || if on.get() { 40.0 } else { 0.0 }, 0.0)
        .animation(Animation::spring(0.35, 0.7), on)
}
```

Doc-vs-code: the doc comment on `.animation` and `animation.mdx` say the value can be "a plain value", but `AnimationValue` has no impl for plain values in the Rust crate. Pass a `State`, `Computed`, `Binding` or closure.

### Transition and Pose

```rust
pub struct Transition                                   // Clone
impl Transition {
    pub fn opacity() -> Self                            // the default (fade)
    pub fn identity() -> Self                           // appears/disappears at once
    pub fn scale(from: f32) -> Self                     // around the center
    pub fn scale_anchored(from: f32, anchor: impl Into<Alignment>) -> Self   // e.g. Alignment::TOP
    pub fn offset(x: f32, y: f32) -> Self
    pub fn move_edge(edge: impl Into<Edges>) -> Self    // slide in from / out to an edge
    pub fn slide() -> Self                              // in from leading, out to trailing
    pub fn asymmetric(insertion: impl Into<Transition>, removal: impl Into<Transition>) -> Self
    pub fn combined(&self, other: impl Into<Transition>) -> Self
    pub fn pose(&self, progress: f32, removal: bool, size: Size) -> Pose   // 0 hidden, 1 shown
}
pub struct Pose { pub opacity: f32, pub scale: f32, pub x: f32, pub y: f32, pub anchor_x: f32, pub anchor_y: f32 }  // Default: 1, 1, 0, 0, 0.5, 0.5
```

A transition decides how a view enters/leaves when `If`/`match`/`ForEach` add or remove it during an animated change (inside `with_animation` or an `.animation`-watched value). Unless set it fades. Set it on the view that appears: `.transition(Transition::move_edge(.top).combined(Transition::opacity()))`. `Pose` is the value `Transition::pose` returns (for custom drawing/testing); you seldom build one.

`PopupMotion::new(transition, animation)` (view/options.rs) takes the same pair for popups; `NavigationStack`'s `sidebar_transition` takes `impl Into<Transition>` (outside this file's scope).

### Which things animate

- Reactive props that are documented "Reactive and animatable": `.blur`, `.backdrop_blur`, `.rotation_effect`, `.scale_effect`; `.offset` (animated in animated updates); also colors and fills (`.fill`, `.foreground_style`, `.background`), opacity, sizes and positions (layout, unless `.animates_layout(false)`), and a `Progress` fraction (from docs `animation.mdx`, `views.rs`).
- Non-reactive (static) values do not animate; make the property a closure/State so a change is observable.
- Views appear/disappear with their transition only when the change is animated.
- Style animations: some styles (toggle, checkbox) carry their own `Option<Animation>` (default `smooth(0.3)`); `None` means at once.
- A `repeat_forever` animation keeps the window drawing frames; it ends when the watched value changes again or the view goes away.

## Threads: post_to_ui_thread and AppHandle

```rust
pub fn post_to_ui_thread(task: impl FnOnce() + Send + 'static)
pub struct AppHandle { .. }                      // Clone; from App::handle()
impl AppHandle { pub fn quit(&self) }            // no-op if the App is gone
// App (app.rs): pub fn quit(&self)  // makes run() return after the current event; UI thread
//               pub fn handle(&self) -> AppHandle
```

- `post_to_ui_thread` is the only supported way to hand work back from another thread. Callable from any thread; tasks run on the UI thread, between events, in the order posted, where state may be written. The task must be `Send + 'static`.
- `State`/`Binding`/`Computed` are not `Send`, so a task cannot capture them directly. The docs wrap the callback in `SendWrapper` (external crate `send_wrapper`, not re-exported by Elegance): the callback travels to the worker thread and back inside a `SendWrapper`, and is only called on the UI thread again. Send only plain data (`String`, `Vec`, `Result`) across.
- `AppHandle::quit` is for closures that cannot borrow the `App`.
- Posted while no App runs, the task waits for the next one (C++ doc, `app.h`).
- A late task can run after the view that owned the state is gone: check `state.is_alive()` first.
- `Timer`s, `effect`s and all state calls stay on the UI thread. There is no `task(async fn)` in the Rust crate (design.md lists it as future). Elegance has no HTTP/WebSocket/JSON library: use `ureq`/`reqwest`/`tungstenite`/`serde_json` as the docs suggest.
- Platform: no per-platform differences are documented for these calls. (`Platform` has `Windows`, `MacOS`, `Linux` only; there is no Mikros variant in the Rust enum, `current_platform` maps anything else to Linux.)

## Environment

Values inherited down the view tree (font, colors, theme, styles, your own keys). A modifier sets a value for a view and everything inside; each view sees the value set nearest above.

### Environment, EnvironmentKey, EnvironmentValues

```rust
pub trait EnvironmentKey: 'static {
    type Value: Clone + 'static;
    const NAME: Option<&'static str> = None;          // share with C++ by name (7 value types)
    fn default_value(environment: &EnvironmentValues) -> Self::Value;
}
pub struct Environment<K: EnvironmentKey>               // Clone; Default
impl<K: EnvironmentKey> Environment<K> {
    #[track_caller] pub fn new() -> Self                // call while building a view; panics elsewhere
    pub fn get(&self) -> K::Value                       // tracked read
}
pub struct EnvironmentValues                            // Clone, Default
impl EnvironmentValues {
    pub fn new() -> Self                                // empty: every key reads its default
    #[track_caller] pub fn current() -> Self            // copy of the env of the view being built; panics outside a body
    pub fn color(&self, token: impl Into<ThemeColor>) -> Color   // tracked
    pub fn get<K: EnvironmentKey>(&self) -> K::Value
    pub fn set<K: EnvironmentKey>(&mut self, value: impl IntoProp<K::Value>)   // inside an EnvironmentModifier
    pub fn font(&self) -> Font          pub fn color_scheme(&self) -> ColorScheme
    pub fn theme(&self) -> Theme        pub fn foreground(&self) -> Color
    pub fn tint(&self) -> Color         pub fn is_disabled(&self) -> bool
    pub fn hover_delay(&self) -> HoverDelay    pub fn window(&self) -> WindowHandle
}
pub trait EnvironmentModifier: 'static { fn apply(&self, values: &mut EnvironmentValues); }  // modifier.rs
pub struct AsEnvironment<M>(pub M);                     // use: view.modifier(AsEnvironment(m))
```

Use `Environment::<K>::new()` in a component/body to read a key reactively. Use `EnvironmentValues` (given to `Prop::from_env` closures and `EnvironmentModifier::apply`) when you are already inside one. `EnvironmentValues::default_value` receives the environment so a default can depend on other keys (`ForegroundKey` default is the theme's `Primary`).

### Built-in keys

Defined in `environment.rs`, all in the prelude. Set with the matching modifier:

| Key | Value | Modifier | Default |
| --- | --- | --- | --- |
| `FontKey` | `Font` | `.font(..)` | `Font::body()` |
| `ColorSchemeKey` | `ColorScheme` | `.color_scheme(..)` | light in `default_value`; the system setting at runtime |
| `ThemeKey` | `Theme` | `.theme(theme)` (static only) | `Theme::standard()` |
| `ForegroundKey` | `Color` | `.foreground_style(..)` | theme `Primary` |
| `TintKey` | `Color` | `.tint(..)` | theme `Accent` |
| `DisabledKey` | `bool` | `.disabled(..)` | `false` |
| `HoverDelayKey` | `HoverDelay` | `.hover_delay(..)` | `HoverDelay::default()` |
| `WindowKey` | `WindowHandle` | set by the window; static only (panics if reactive) | `WindowHandle::none()` |
| `RowSelectedKey` | `bool` | set inside selectable rows (`.selection()`) | `false` |
| `ButtonStyleKey` | `AnyButtonStyle` | `.button_style(..)` | `ProminentButtonStyle` |
| `ToggleStyleKey` | `AnyToggleStyle` | `.toggle_style(..)` | `SwitchToggleStyle` |
| `TextFieldStyleKey` | `AnyTextFieldStyle` | `.text_field_style(..)` | `RoundedBorderTextFieldStyle` |
| `CheckboxStyleKey` | `AnyCheckboxStyle` | `.checkbox_style(..)` | `BoxCheckboxStyle` |
| `NumberInputStyleKey` | `AnyNumberInputStyle` | `.number_input_style(..)` | `BoxNumberInputStyle` |
| `ProgressStyleKey` | `AnyProgressStyle` | `.progress_style(..)` | `LinearProgressStyle` |
| `PickerStyleKey` | `AnyPickerStyle` | `.picker_style(..)` | `SegmentedPickerStyle` |
| `SliderStyleKey` | `AnySliderStyle` | `.slider_style(..)` | `KnobSliderStyle` |
| `NavigationStackStyleKey` | `AnyNavigationStackStyle` | `.navigation_stack_style(..)` | `PlainNavigationStackStyle` |
| `PopoverStyleKey` | `AnyPopoverStyle` | `.popover_style(..)` | `DefaultPopoverStyle` |
| `TooltipStyleKey` | `AnyTooltipStyle` | `.tooltip_style(..)` | `DefaultTooltipStyle` |
| `MenuStyleKey` | `AnyMenuStyle` | `.menu_style(..)` | `DefaultMenuStyle` |
| `RowStyleKey` | `AnyRowStyle` | `.row_style(..)` | `HighlightRowStyle` |

(Modifier names other than the ones in `environment.mdx` and the ones I confirmed in `view.rs`: `.font`, `.foreground_style`, `.tint`, `.color_scheme`, `.theme`, `.disabled`, `.hover_delay`, `.text_field_style`, `.navigation_stack_style`, `.popover_style`, `.tooltip_style`, `.menu_style`, `.row_style`, `.button_style`, `.toggle_style`; `.checkbox_style`, `.number_input_style`, `.progress_style`, `.picker_style`, `.slider_style` follow the same pattern but were not each grepped: unverified.)

Reactivity: a value can follow state: `.tint(move || accent.get())` or `.tint(accent_state)`; only the views using that value update. Style keys may take a closure returning a style. `ThemeKey` and `WindowKey` values must be static.

### Custom keys

```rust
struct CompactKey;
impl EnvironmentKey for CompactKey {
    type Value = bool;
    const NAME: Option<&'static str> = Some("myapp.compact");   // optional: shared with a C++ key of this name
    fn default_value(_: &EnvironmentValues) -> bool { false }
}
```

- `Value` must be `Clone + 'static`. A key without `NAME` is Rust-only. A `NAME` is shared with C++ only when `Value` is one of `bool`, `i32`, `i64`, `f32`, `f64`, `String`, `Color`; with another value type (a `Vec`, an enum, your struct) it still works, but only within Rust and with no error saying it is not shared.
- A key reads `default_value` when unset (also when a key with that name was set with another value type).
- Pick unique, stable names like `"myapp.compact"`; renaming breaks sharing.
- A value that depends on the environment (a theme color, `Prop::from_env`) is resolved against the environment as it was before the set call, so a key never depends on itself.

### Setting and reading

```rust
fn environment<K: EnvironmentKey>(self, value: impl IntoProp<K::Value>) -> View   // ViewExt
```

```rust
let settings = Text::new("Settings").environment::<CompactKey>(true);          // static
let settings = Text::new("Settings").environment::<CompactKey>(compact_state); // follows the state
elg! { Settings().environment::<CompactKey>(move || width.get() < 400.0) }    // closure (a bool closure is a Prop)

#[component]
fn SettingRow(title: String) -> impl IntoView {
    let compact = Environment::<CompactKey>::new();
    elg! { Text(title.clone()).font(move || if compact.get() { Font::footnote() } else { Font::body() }) }
}

// Where a prop closure is given the environment:
Circle().fill(Prop::from_env(|env| ShapeStyle::Color(env.get::<TintKey>().opacity(0.5))))
```

Forms of setting: the `.environment::<K>(v)` modifier (any key, static or reactive); the dedicated modifiers in the table above; an `EnvironmentModifier` via `.modifier(AsEnvironment(m))` (mutate `EnvironmentValues` with `set::<K>(...)`); `ViewExt::modifier(impl Modifier)`. Read with `Environment::<K>::new()` while building a view, or `env.get::<K>()` when handed an `EnvironmentValues`. `Environment::new` outside a body panics. Reading is reactive: read inside closures (`move || compact.get()`), not once in the body.

## Pitfalls

Each item was checked in the code (`reactive.rs`, `timer.rs`, `dsl.rs`, `app.rs`) or in `state.mdx`/`networking.mdx`.

1. Reading state in a component body (or any plain function) does not subscribe; it gives one value. Use `Text("{x}")`, `Text(move || ..)`, a `State`, `.map(..)` or `computed`.
2. Closures that capture state need `move`; the handles are `Copy`, so you can still use the state afterwards. Without `move` the closure borrows a local and the compiler rejects it as `'static`.
3. Handles are `!Send`/`!Sync`: you cannot capture a `State` in a closure given to `std::thread::spawn` or `post_to_ui_thread`. Send plain data and use `SendWrapper` (docs), or poll a channel from `every` (see recipe).
4. Using a handle after the views that created it are removed panics. Callbacks that can outlive the view (thread results, `after` started in a click, `every` started in a click) must check `is_alive()` or cancel in `on_disappear`.
5. A read guard alive while writing the same state panics, and a write guard alive while reading panics. Keep guards temporary: `let n = state.read().len();` is fine, `let g = state.read(); state.set(..)` is not.
6. `State::set`/`write` are UI-thread only and panic elsewhere (`elegance: reactive state was used off the UI thread`).
7. Effects that write state they also read loop; Elegance aborts the flush with a panic after 100,000 effect runs. Avoid writing the state an effect reads; use `computed`/`map` for derived values, `untrack` for reads that must not subscribe.
8. Never compute display values in an effect and write them to state (extra frame of lag, loops). Derive with `computed`.
9. Effects cannot run inside an event handler, effect, computed or layout; a `batch` there only defers. Do not rely on an effect having run synchronously after a `set` in a handler.
10. `effect` outside view building panics ("effect() must be called while building views"). Call it in a component body or reactive closure, not in `main` before the app, not in a click handler.
11. `Timer` drop does not stop it. Timers from a click run past their view. `after`/`every` off the UI thread panic. NaN seconds panics.
12. `State::new` writes equal to the current value do nothing; `write()`/`update` always notify even if nothing changed. Types without `PartialEq` need `State::always_notify`/`Computed::always_notify`.
13. State created inside an `if` branch, `match` arm or `ForEach` item is destroyed with it. A `ForEach` item whose value changed is rebuilt (state lost): use `content_signal` or keep state in the parent.
14. `Text(count.get().to_string())` is one-time. `.opacity(count.get())` is one-time. Pass `count`, `move || count.get()` or `"{count}"`.
15. A closure for a reactive prop must return exactly the prop's type; literals infer (`|| 0.5` is `f32` for an `f32` prop). Only certain types are reactive (see the `IntoProp` table). Two branches returning different color types (`Color` vs `ThemeColor`) do not compile.
16. `.animation(anim, value)` needs a watched `State`/`Computed`/`Binding`/closure, not a plain value (docs say otherwise; the code does not implement it). Static props do not animate.
17. `Animation::spring()` with no arguments does not exist in Rust. `repeat_count`/`repeat_forever` always take `autoreverses`.
18. Environment reads: `Environment::<K>::new()` panics outside a view body; and reading in a component body once (`compact.get()` directly) does not follow changes: read it inside a closure.
19. A plain function that returns a view runs before it is placed and sees its caller's environment. Use a `#[component]` for "where I end up" reads.
20. The reactive `if`/`match` in `elg!` evaluates the condition inside a `move ||` closure: the condition must not move non-`Copy` values out; clone inside. `match` needs `Clone + PartialEq` values and clones the value per arm evaluation.
21. Interpolated strings (`"{x}"`) are a lone string literal argument only; `Text(format!("{x}"))` is not reactive.

## Recipes

### Recipe: background work with loading and error state

Pattern: a `State<Load>` enum (loading / error / value), a worker thread that sends plain data, `post_to_ui_thread` to deliver. `SendWrapper` (external crate) moves the non-`Send` callback; check `is_alive()` when delivered.

```rust
use send_wrapper::SendWrapper;

#[derive(Clone, PartialEq)]
enum Load { Loading, Failed(String), Ready(String) }

// `done` runs on the UI thread.
fn get(url: &str, done: impl FnOnce(Result<String, String>) + 'static) {
    let url = url.to_owned();
    let done = SendWrapper::new(done);
    std::thread::spawn(move || {
        let body = ureq::get(&url)
            .call()
            .and_then(|mut r| r.body_mut().read_to_string())
            .map_err(|e| e.to_string());                 // parse here too, off the UI thread
        post_to_ui_thread(move || done.take()(body));
    });
}

#[component]
fn Quote() -> impl IntoView {
    let state = State::new(Load::Loading);
    let reload = move || {
        state.set(Load::Loading);
        get("https://api.github.com/zen", move |result| {
            if !state.is_alive() { return; }             // the view has gone
            state.set(match result { Ok(text) => Load::Ready(text), Err(e) => Load::Failed(e) });
        });
    };
    elg! {
        VStack {
            match state.get() {
                Load::Loading => Progress(),
                Load::Failed(error) => Text("Failed: {error}"),
                Load::Ready(text) => Text("{text}"),
            }
            Button("Reload", reload)
        }
        .on_appear(reload)
    }
}
```

Notes: `reload` is `Copy` here because it only captures a `State`; `on_appear` takes `FnMut`, `Button` takes the closure as the docs show. A tokio runtime works the same: do the async work on the runtime, call `post_to_ui_thread` with plain data. Without a crate: create an `std::sync::mpsc` channel, give the `Sender` to the worker and poll the `Receiver` from `every(0.05, move || while let Ok(m) = rx.try_recv() { state.set(m) })` started in the component body (the timer ends with the view, so no `is_alive` check; derived from the `every` signature, `FnMut + 'static`, not shown in the docs). Group several writes in `batch` when a task sets more than one state.

### Recipe: debounce and timers

```rust
use std::{cell::RefCell, rc::Rc};

#[component]
fn Search(query: Binding<String>, results: State<String>) -> impl IntoView {
    let debounced = State::new(String::new());
    let pending = Rc::new(RefCell::new(Timer::default()));
    // Runs now and whenever `query` changes; each run cancels the previous timer.
    effect(move || {
        let q = query.get();
        pending.borrow().cancel();
        *pending.borrow_mut() = after(0.3, move || debounced.set(q));
    });
    effect(move || results.set(format!("searching for {}", debounced.get())));
    elg! { TextField("Search", query) }
}
```

Timers started in an effect run belong to the view and stop with it. Other timer patterns:

```rust
// Clock: starts with the view, stops with it.
elg! { Text("{seconds}").on_appear(move || { every(1.0, move || seconds += 1); }) }
// Hide a banner after 3 s; cancel if dismissed early.
let hide = after(3.0, move || banner.set(false));
hide.cancel();
// A click-started interval runs past its view: keep the Timer and cancel it.
let ticker = Rc::new(RefCell::new(Timer::default()));
```

### Recipe: animated list

Wrap the list mutation in `with_animation`; give rows a `.transition`; use a stable `id:` key so rows move rather than rebuild.

```rust
#[derive(Clone, PartialEq)] struct Item { id: i64, name: String }

let items = State::new(Vec::<Item>::new());
let next_id = State::new(0_i64);

let add = move || with_animation(Animation::spring(0.4, 0.8), || {
    let id = next_id.get(); next_id.set(id + 1);
    items.write().push(Item { id, name: format!("Item {id}") });
});
let remove = move |id: i64| with_animation(Animation::smooth(0.3), || {
    items.write().retain(|i| i.id != id);
});

elg! {
    VStack {
        Button("Add", add)
        ForEach(items, id: |i| i.id) { item in
            HStack {
                Text(item.name.clone())
                Button("Remove", { let id = item.id; move || remove(id) })
            }
            .transition(Transition::scale(0.8).combined(Transition::opacity()))
        }
    }
}
```

Notes: removal and insertion animate only because the change is inside `with_animation` (or because a watched `.animation(.., value)` changed). Other rows glide to their new positions (layout animates; `.animates_layout(false)` on a subtree makes it snap). Item values that change are rebuilt: if rows hold state, use `ForEach::new(items).id(|i| i.id).content_signal(|item| ...)` (item is a `Signal<Item>`: `Text(move || item.get().name)`). `ForEach` as a stack child makes the items direct children of the stack.

### Recipe: spinner that runs while a view is shown

```rust
#[component]
fn Spinner() -> impl IntoView {
    let spinning = State::new(false);
    elg! {
        Text("Loading")
            .rotation_effect(move || if spinning.get() { 360.0 } else { 0.0 })
            .animation(Animation::linear(1.0).repeat_forever(false), spinning)
            .on_appear(move || spinning.set(true))
    }
}
```

## C++-only / not exposed in Rust

- `State::operator()` / `Binding::operator()` / `Computed::operator()` and `const T&`-returning `get()` (Rust uses `get()` clone, `read()`/`with()` guards).
- `elg::signal(...)`, `Signal` as a writable source (design.md sketch); Rust `Signal<T>` is read-only. `Store<T>`, `Resource<T>`, `Switch(...).Case(...)`, `task(async fn)`, `elg::post_to_ui` naming: design.md proposals, not in `reactive.h` and not in Rust.
- C++ `Animation::spring()` / `smooth()` with default arguments, `Animation::...repeat_count(n)` with default autoreverses = true.
- `EnvironmentValues::get_named<T>` / `set_named<T>`: by-name access without a key type (C++ only; Rust shares keys through `EnvironmentKey::NAME`).
- C++ `EnvironmentKey` uses `static constexpr std::string_view name`; Rust uses `const NAME: Option<&'static str>`.
- Computed values that throw (exception cached until a source changes), empty `std::function` UsageErrors, `UsageError` on off-thread reads in debug builds.
- `Transition::move(Edges::top)` in C++ is `Transition::move_edge(..)` in Rust.
- `DeferEffects`, `Scope`, `begin_batch`/`end_batch`, `begin_untracked` (internal `elg::detail` in `reactive.h`).

## Unverified items

- `.checkbox_style`, `.number_input_style`, `.progress_style`, `.picker_style`, `.slider_style` modifier names (the matching keys exist in `environment.rs`; I grepped `.text_field_style`, `.navigation_stack_style`, `.popover_style`, `.tooltip_style`, `.menu_style`, `.row_style`, `.button_style`, `.toggle_style` in `view.rs`, not these five).
- Whether `Computed::new` can be called outside any view (before `App::run`) without a scope: `State` outside a view is documented as program-lifetime; `Computed` is not mentioned.
- Debounce and animated-list recipes are assembled from confirmed signatures and doc snippets (`elg!` forms like `Button("Remove", { let id = ..; move || remove(id) })` follow the DSL's block-expression rule but were not compile-checked); the `mpsc` + `every` polling alternative is not in the docs.
- Exact set of properties that animate beyond those documented as "Reactive and animatable" in `view.rs` comments and `animation.mdx`.
- The `Progress` fraction animates per a doc comment in `views.rs` (line ~688).
