# Component catalog

Every view, container, control, drawing primitive, media view, window/app/desktop facility, event and accessibility type
available from Rust (`use elegance::prelude::*;`). Names and signatures were checked against `elegance-rs/elegance/src`.
Modifiers are in [modifiers.md](modifiers.md), state in [state-and-reactivity.md](state-and-reactivity.md), styling in
[theming-and-style.md](theming-and-style.md).

## Table of contents

- Part 1: layout containers, lists/grids/lazy stacks, scroll, navigation, custom layouts and geometry, custom views/components/macros
- Part 2: text, buttons and controls, text input, shapes and drawing, images, media (video/camera/audio), GPU images
- Part 3: floating UI (popovers, tooltips, menus), App and windows, desktop integration (dialogs, tray, notifications, dock), clipboard and drag and drop, accessibility, events and gestures

Quick map: a container -> Part 1; something to read, enter or click -> Part 2; something that opens over or outside the
window -> Part 3.

---

## Part 1: Layout, lists, scroll, navigation, custom layouts, custom views

Groups in this part:

1. [Layout containers](#1-layout-containers)
2. [Lists, grids and lazy stacks](#2-lists-grids-and-lazy-stacks)
3. [Scroll](#3-scroll)
4. [Navigation and split views](#4-navigation-and-split-views)
5. [Custom layouts and geometry types](#5-custom-layouts-and-geometry-types)
6. [Custom views, components and modifiers](#6-custom-views-components-and-modifiers)
7. [Not exposed in Rust / unverified](#7-not-exposed-in-rust--unverified)

Conventions used below: all names come from `use elegance::prelude::*;`. Sizes are points (`f32`). `elg!` is the DSL
(see [the grammar summary](#the-elg-dsl-grammar-summary)). Reactive inputs (closures, `State`, `Computed`) are passed
unread (`move || ...`, `count`), never as `count.get()`, except in `if`/`match` conditions inside `elg!`, which are
re-run reactively already. Modifiers (`.padding`, `.frame`, `.background_in`, ...) are in `modifiers.md`; styles
(`RowStyle`, `NavigationStackStyle`, ...) in the theming/styles file.

---

## 1. Layout containers

### VStack / HStack / ZStack

```rust
pub struct VStack { .. }   // VStack::new() -> Self
pub struct HStack { .. }   // HStack::new() -> Self
pub struct ZStack { .. }   // ZStack::new() -> Self

// builder methods (all three)
fn child(self, child: impl IntoView) -> Self
fn children<V: IntoView>(self, children: impl IntoIterator<Item = V>) -> Self
// VStack
fn alignment(self, alignment: impl Into<HorizontalAlignment>) -> Self   // .leading .center .trailing
fn spacing(self, spacing: impl Into<Len>) -> Self
// HStack
fn alignment(self, alignment: impl Into<VerticalAlignment>) -> Self     // .top .center .bottom
fn spacing(self, spacing: impl Into<Len>) -> Self
// ZStack (no spacing)
fn alignment(self, alignment: impl Into<Alignment>) -> Self             // .top_leading, .center, ...
```

VStack: children top to bottom. HStack: leading to trailing. ZStack: layered, first child at the back.
Defaults: alignment center; spacing unset = `DEFAULT_STACK_SPACING` (8 pt), and none next to a `Spacer`.
A stack serves its least flexible children first (text, images), then the flexible ones (shapes, spacers, views with
an unbounded frame); raise `.layout_priority(n)` on the child that must not be cut.
`spacing` and `alignment` are static (not reactive). There is no baseline alignment (only top/center/bottom), although
the docs prose mentions baselines.

Prefer a plain stack when the children are few and fixed (it builds them all); use a [lazy stack](#lazyvstack--lazyhstack)
in a `ScrollView` for long collections, a [grid](#vgrid--hgrid) for columns that flow or line up, a
[custom `Layout`](#layout-trait) for anything else (flow/wrap, radial).
`ForEach` inside a stack: its items are direct children of the stack (get its spacing/alignment).
In `elg!`, a block with several children is the children list; blocks of `ScrollView` (several children) wrap them in
a VStack.

```rust
elg! {
    VStack(alignment: .leading, spacing: 4) {
        Text("Elegance").font(.headline)
        Text("Native apps, in C++ or Rust").foreground_style(.secondary)
    }
}
elg! {
    ZStack(alignment: .bottom_trailing) {
        Image(ImageSource::file("/path/to/photo.jpg"))
        Text("NEW").padding(4).background(.accent)
    }
}
elg! {
    HStack {
        Text("A long title that might need to be cut short").line_limit(1)
        Text("12:30").layout_priority(1) // the time is never cut
    }
}
```

Outside `elg!`: `VStack::new().alignment(HorizontalAlignment::Leading).spacing(4).child(a).child(b).into_view()`.

### Spacer

```rust
pub struct Spacer { .. }
pub fn Spacer() -> Spacer
Spacer::new() -> Self
fn min_length(self, length: impl Into<Len>) -> Self
// elg!: Spacer()   Spacer(20)  (positional = min_length)
```

Flexible space that expands along its stack's axis (pushes siblings apart). Default `min_length` unset.
Prefer it over fixed `.frame` gaps when something must sit at the far end of a row.

```rust
# let done = || {};
elg! {
    HStack {
        Text("Settings").font(.headline)
        Spacer()
        Button("Done", done)
    }
}
```

### Separator

```rust
pub struct Separator { .. }
pub fn Separator() -> Separator
Separator::new() -> Self
fn axis(self, axis: impl Into<Axis>) -> Self        // default Axis::Horizontal
fn color(self, color: impl IntoProp<Color>) -> Self // default theme Outline
fn thickness(self, thickness: impl Number) -> Self  // default 1.0
fn inset(self, inset: impl Number) -> Self          // clear at both ends, default 0
// elg!: Separator()   Separator(.vertical)   Separator(.vertical, inset: 4)
```

Thin line between groups: horizontal (as wide as offered) in a `VStack`, menu or grid row; vertical (as tall as offered)
in an `HStack` only if you pass `.vertical`. Screen readers see it. In a `Grid`, a Separator takes a row of its own.

```rust
elg! { HStack { Text("Bold") Separator(.vertical, inset: 4) Text("Italic") } }
```

### EmptyView

```rust
pub struct EmptyView { .. }
pub fn EmptyView() -> View   // elg!: EmptyView()
```

Takes no space, draws nothing. Use it as a placeholder `else` branch or `match` arm. `()` and `None` also
`IntoView` as empty, and an `if` without `else` is empty.

### If, and `if` / `match` in elg!

```rust
pub struct If { .. }
pub fn If(condition: impl IntoProp<bool>, then_view: impl IntoView, else_view: impl IntoView) -> View
// elg!: If(cond, then)   If(cond, then, else)      (positional ElgNew impls, else defaults to empty)
// elg!: if cond { A } else if cond2 { B } else { C }
// elg!: match value { Pat => View, Pat if guard => View, _ => View }
```

Reactive conditional. In `elg!` write `if`/`else`: the condition is wrapped in `move || -> bool { cond }`, so reading
state in it (`items.read().is_empty()`, `on.get()`) is tracked. Outside `elg!`, pass a `State<bool>`, `Computed<bool>`,
or closure to `If(...)`; use `()` for "nothing". `match` evaluates the scrutinee reactively (`move || value`), requires
the value `Clone + PartialEq + 'static`, clones it into the arm (bindings by value), and rebuilds the arm's view when the
value changes. Each arm is one view (`=> View()` or `=> { View() }`).
Switching branches removes the old branch's views and the state created in them; inside `with_animation` they get
their `.transition`. In a stack a branch acts like a direct child; an empty branch takes no space.
Prefer `if`/`match` to showing/hiding with `.opacity(0)` when the hidden view should not exist (or hold state).

```rust
# #[derive(Clone, PartialEq)] enum Status { Loading, Failed(String), Ready }
# let status = State::new(Status::Loading);
# let items = State::new(Vec::<String>::new());
elg! {
    VStack {
        if items.read().is_empty() {
            Text("Nothing here yet")
        } else {
            Text(move || format!("{} items", items.read().len()))
        }
        match status.get() {
            Status::Loading => Progress(),
            Status::Failed(message) => Text(message).foreground_style(.danger),
            Status::Ready => Text("Ready"),
        }
    }
}
// outside elg!:
let v = If(move || items.with(Vec::is_empty), Text("Nothing here"), ());
```

### ForEach

```rust
pub struct ForEach<T: 'static, K = NoKey> { .. }
ForEach::new(items: impl ItemsSource<Item = T>) -> ForEach<T, NoKey>
fn id<R: IntoItemKey>(self, key: impl Fn(&T) -> R + 'static) -> ForEach<T, KeyFn<T>>
fn content<V: IntoView>(self, content: impl Fn(&T) -> V + 'static) -> Self
fn content_signal<V: IntoView>(self, content: impl Fn(Signal<T>) -> V + 'static) -> Self   // T: Clone + PartialEq
// elg!: ForEach(items) { item in View }        ForEach(items, id: |t| t.id) { item in View }
pub trait ItemsSource { type Item: 'static; fn into_items(self) -> Rc<dyn Fn() -> Vec<Self::Item>>; }
pub trait IntoItemKey { fn into_item_key(self) -> ItemKey; }
pub enum ItemKey { Int(i64), Str(String) }
```

One view per item, identified by key. `ItemsSource` is implemented for `Vec<T>`, `State<Vec<T>>`, `Computed<Vec<T>>`,
`Binding<Vec<T>>`, any `Fn() -> Vec<T>` (a tracked read), and ranges `Range<i32|i64|u32|usize>`. `IntoItemKey` is
implemented for all integer types (and `&` of them), `String`, `&String`, `&str`, and `ItemKey`. The content closure gets
`&T` (so `.clone()` for owned props; in `elg!` the `item in` block is `elg_content`).
Items must be `Clone + PartialEq`. When the collection changes: unchanged (key + value equal) items keep their views and
state; changed items are rebuilt; removed items are removed. Duplicate keys still work but those items rebuild every
update (a warning is logged once).
Without `id:` the items are their own keys (items must then be integers or strings: `ForEach(0..6)`, `ForEach(vec!["a","b"])`).
`id:` can return any `IntoItemKey` (an `i64`, `String`, ...). With `.content_signal` each key's view lives as long as the key
and reads the item through a `Signal<T>` (keeps state on edits). Missing content panics at build.
Filter/sort by passing a `computed(...)` as the items. Inside a `GridRow` a ForEach is a single cell; inside a `Grid` a
ForEach of `GridRow`s repeats rows. Inside a lazy stack, it must be the only child.

```rust
# #[derive(Clone, PartialEq)] struct Todo { id: i64, title: String, done: bool }
# let todos = State::new(Vec::<Todo>::new());
# let hide_done = State::new(false);
let visible = computed(move || {
    todos.read().iter().filter(|t| !(hide_done.get() && t.done)).cloned().collect::<Vec<_>>()
});
elg! {
    VStack {
        ForEach(visible, id: |t| t.id) { todo in Text(todo.title.clone()) }
        ForEach(vec!["Red", "Green", "Blue"]) { name in Text(*name) }
    }
}
// keep each item's view (and its state) across edits:
let rows = ForEach::new(todos).id(|t| t.id).content_signal(|todo| Text::new(move || todo.get().title));
```

### StackSeparators

```rust
#[derive(Clone)] pub struct StackSeparators { pub color: Prop<Color>, pub inset: f32, pub thickness: f32 }
StackSeparators::default()          // color theme Outline, inset 0, thickness 1
fn color(self, color: impl IntoProp<Color>) -> Self
fn inset(self, inset: impl Number) -> Self        // from the leading edge (top in a LazyHStack)
fn thickness(self, thickness: impl Number) -> Self
```

Lines between a lazy stack's items, centered in the spacing; used through `LazyVStack(separators: ...)`. See
[lazy stacks](#lazyvstack--lazyhstack). Only lazy stacks take it (a plain VStack uses `Separator()`).

### Stack alignment, spacing and defaults

```rust
pub const DEFAULT_STACK_SPACING: f32 = 8.0;
pub fn default_stack_spacing() -> f32   // the library's own value (currently 8)
```

| Option | VStack | HStack | ZStack | LazyVStack | LazyHStack | VGrid/HGrid | Grid |
|---|---|---|---|---|---|---|---|
| `alignment` type | `HorizontalAlignment` | `VerticalAlignment` | `Alignment` | `HorizontalAlignment` | `VerticalAlignment` | `Alignment` (default center) | `Alignment` |
| `spacing` | `Len` | `Len` | none | `Len` | `Len` | `Len` | `horizontal_spacing`, `vertical_spacing` |

`Len(None)` (the default) means "use `DEFAULT_STACK_SPACING`". Any number converts (`spacing: 12`, `spacing: 0`).
For custom layouts, `default_stack_spacing()` is the value to use for "no spacing given".

---

## 2. Lists, grids and lazy stacks

There is no separate List view: a list is a `LazyVStack` inside a `ScrollView` (with `.selection` and `.row_style`).

| Need | Use |
|---|---|
| few fixed children | `VStack` / `HStack` / `ZStack` |
| short dynamic collection (tens) | `VStack { ForEach(..) }` (builds all rows; `ForEach` inside a stack) |
| long collection (hundreds to tens of thousands), scrolling | `ScrollView { LazyVStack { ForEach(..) } }` (builds only rows near the viewport) |
| cards/swatches that fill the width, as many columns as fit | `VGrid([GridItem::adaptive(150)], ..)` |
| table whose columns line up on widest cell (forms, settings, data) | `Grid { GridRow { .. } .. }` |
| selectable rows | `.selection(..)` on the stack |
| wrap/flow, anything else | custom [`Layout`](#layout-trait) |

### LazyVStack / LazyHStack

```rust
pub struct LazyVStack { .. }   pub struct LazyHStack { .. }
LazyVStack::new(for_each: impl IntoView) -> Self      // LazyHStack::new the same
fn alignment(self, alignment: impl Into<HorizontalAlignment>) -> Self   // LazyHStack: VerticalAlignment
fn spacing(self, spacing: impl Into<Len>) -> Self
fn separators(self, separators: StackSeparators) -> Self
// elg!: LazyVStack(spacing: 4, separators: StackSeparators::default().inset(16)) { ForEach(..) { .. } }
```

Like a stack over a `ForEach`, but only items near the visible part of the enclosing `ScrollView` are built; rows far
out of sight are destroyed (with their state: keep important state outside the rows). Handles tens of thousands of
items; keeps scroll position stable when rows above change size or are inserted; supports
`ScrollPosition::scroll_to(key)`. Its content must be exactly one `ForEach` (in `elg!`, exactly one view in the block, else
panic; the positional `elg!` form takes none, use the block). Put it inside a `ScrollView` (LazyVStack for vertical,
LazyHStack with `ScrollView(axes: .horizontal)`); rows bring their own padding/background.

```rust
# #[derive(Clone, PartialEq)] struct Contact { id: i64, name: String }
# let contacts = State::new(Vec::<Contact>::new());
elg! {
    ScrollView {
        LazyVStack(spacing: 0, separators: StackSeparators::default().inset(16)) {
            ForEach(contacts, id: |c| c.id) { contact in
                Text(contact.name.clone()).padding(horizontal: 16, vertical: 10)
            }
        }
        .background_in(.surface, RoundedRectangle(10))
    }
}
```

### Selecting rows: Selection, SelectionKey

```rust
fn selection<S: Selection>(self, selection: S) -> View                                       // ViewExt
fn selection_with<S: Selection>(self, selection: S, on_activate: impl Fn(&S::Key) + 'static) -> View
pub trait Selection: 'static { type Key: SelectionKey; .. }   // State<Option<K>>, Binding<Option<K>>, State<BTreeSet<K>>, Binding<BTreeSet<K>>
pub trait SelectionKey: Clone + Ord + 'static                 // all integer types and String (the ForEach key types)
```

`.selection(state)` on a `VStack`, `HStack`, `LazyVStack` or `LazyHStack` that wraps a `ForEach` makes its rows selectable
by ForEach key: `State<Option<K>>` = one row, `State<BTreeSet<K>>` = any number (Ctrl/Cmd-click toggles,
Shift-click/arrows ranges, Ctrl/Cmd+A all). `K` must be the type of the `ForEach` key (an `id: |m| m.id` returning `i64`
means `Option<i64>`; `ForEach(0..100)` means `i32`). `selection_with` adds `on_activate` (Enter or double click, gets
the key). Put `.selection` on the stack itself so other children (a header) are not rows. The stack is one Tab stop; arrows,
Home/End move; the selected row scrolls into view; screen readers see a list box. Keys not in the list are ignored (remove
a deleted item's key yourself). Clicks on controls inside a row go to the control.
Row appearance comes from the environment's `RowStyle`: `.row_style(HighlightRowStyle::new().corner_radius(0))`,
`PlainRowStyle` for no highlight; a row reads `Environment::<RowSelectedKey>::new()` to know whether it is selected.
See the theming/styles file for `RowStyle`, `HighlightRowStyle`, `PlainRowStyle`, `RowSelectedKey`.

```rust
# use std::collections::BTreeSet;
# #[derive(Clone, PartialEq)] struct Mail { id: i64, subject: String }
# let mails = State::new(Vec::<Mail>::new());
# fn show(id: i64) {}
let open = State::new(None::<i64>);               // one row
let chosen = State::new(BTreeSet::<i64>::new());  // any number
elg! {
    VStack {
        ScrollView {
            LazyVStack { ForEach(mails, id: |m| m.id) { mail in Text(mail.subject.clone()) } }
                .selection_with(open, |id| show(*id))
        }
        ScrollView {
            LazyVStack { ForEach(mails, id: |m| m.id) { mail in Text(mail.subject.clone()) } }
                .selection(chosen)
        }
    }
}
```

(The doc comment on `selection` in view.rs shows a C++-style `ForEach(mails, &Mail::id, MailRow)` call; in Rust use
`ForEach(mails, id: |m| m.id) { .. }` as above.)

### VGrid / HGrid

```rust
pub struct VGrid { .. }   // columns        pub struct HGrid { .. }   // rows
VGrid::new(columns: impl IntoIterator<Item = GridItem>) -> Self      // HGrid::new(rows: ..)
fn alignment(self, alignment: impl Into<Alignment>) -> Self           // default Alignment::CENTER
fn spacing(self, spacing: impl Into<Len>) -> Self                      // between lines and tracks without their own
fn child(self, child: impl IntoView) -> Self
fn children<V: IntoView>(self, children: impl IntoIterator<Item = V>) -> Self
// elg!: VGrid([GridItem::adaptive(150)], alignment: .top, spacing: 12) { children / ForEach }    VGrid() = one flexible track
```

Children flow into tracks (VGrid: columns, left to right then next row; HGrid: rows, top to bottom then next column);
`ForEach` items are cells too. Each row is as tall as its tallest cell; every cell offers its whole size to its view, so a
flexible view (colour, unbounded frame) fills its cell. Prefer over `Grid` for photo walls, card libraries,
palettes, anything responsive to width (adaptive tracks). `HGrid` usually lives in a horizontal `ScrollView`.

```rust
# #[derive(Clone, PartialEq)] struct Game { id: i64, name: String }
# let games = State::new(Vec::<Game>::new());
# let (name, email) = (State::new(String::new()), State::new(String::new()));
elg! {
    VStack {
        // as many columns of at least 150 pt as fit; they fill the width
        VGrid([GridItem::adaptive(150)], spacing: 12) {
            ForEach(games, id: |g| g.id) { game in Text(game.name.clone()) }
        }
        // a label column and a column taking the rest
        VGrid([GridItem::fixed(120).alignment(.trailing).spacing(16), GridItem::flexible().alignment(.leading)]) {
            Text("Name")  TextField("", name)
            Text("Email") TextField("", email)
        }
    }
}
```

### GridItem, GridItemKind, GridTrack

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GridItemKind { Fixed, #[default] Flexible, Adaptive }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridItem { pub kind: GridItemKind, pub minimum: f32, pub maximum: f32, pub spacing: Len, pub alignment: Option<Alignment> }
GridItem::fixed(size: impl Number) -> Self           // exactly `size`
GridItem::flexible() -> Self                          // equal share of what fixed leave; minimum 10, maximum infinity
GridItem::adaptive(minimum: impl Number) -> Self     // as many tracks as fit at >= minimum; they share the space exactly
fn minimum(self, minimum: impl Number) -> Self
fn maximum(self, maximum: impl Number) -> Self
fn spacing(self, spacing: impl Into<Len>) -> Self       // space after this track (between an adaptive item's tracks)
fn alignment(self, alignment: impl Into<Alignment>) -> Self   // of views in this track's cells
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GridTrack { pub size: f32, pub spacing: f32, pub alignment: Option<Alignment> }   // a resolved track
```

An adaptive item makes at most 10,000 tracks; with less room than one minimum you get a single narrower track.
`GridItem::default()` is `flexible()`.

### Grid and GridRow

```rust
pub struct Grid { .. }        Grid::new() -> Self
fn alignment(self, alignment: impl Into<Alignment>) -> Self           // of views in cells
fn horizontal_spacing(self, spacing: impl Into<Len>) -> Self           // between columns
fn vertical_spacing(self, spacing: impl Into<Len>) -> Self             // between rows
fn child(self, child: impl IntoView) -> Self      fn children<V: IntoView>(self, children: impl IntoIterator<Item = V>) -> Self
pub struct GridRow { .. }     GridRow::new() -> Self
fn alignment(self, alignment: impl Into<VerticalAlignment>) -> Self    // of this row's cells; default: the grid's
fn cell(self, cell: impl IntoView) -> Self        fn cells<V: IntoView>(self, cells: impl IntoIterator<Item = V>) -> Self
// elg!: Grid(alignment: .leading, horizontal_spacing: 24) { GridRow(alignment: .top) { cell cell } .. }
```

A table of `GridRow`s: each column as wide as its widest cell, each row as tall as its tallest. Anything that is not a
`GridRow` (Separator, heading) takes its own row as wide as the grid. Repeat rows with `ForEach` whose content returns
a `GridRow`. Extra width goes to columns with flexible cells; less width narrows columns proportionally and text wraps.
Cell modifiers (`ViewExt`, see modifiers.md): `.grid_cell_columns(n: i32)` (span n columns),
`.grid_column_alignment(alignment)` (whole column, usually on the header cell),
`.grid_cell_alignment(alignment)` (this view in its cell). Prefer `Grid` for forms and data that must line up;
`VGrid` for flowing collections.

```rust
# #[derive(Clone, PartialEq)] struct Game { id: i64, name: String, mods: i32 }
# let games = State::new(Vec::<Game>::new());
elg! {
    Grid(alignment: .leading, horizontal_spacing: 24) {
        GridRow {
            Text("Game").font(.headline)
            Text("Mods").font(.headline).grid_column_alignment(.trailing)
        }
        Separator()
        ForEach(games, id: |g| g.id) { game in
            GridRow { Text(game.name.clone()) Text(format!("{}", game.mods)) }
        }
        GridRow { Text("Mods include disabled ones.").grid_cell_columns(2) }
    }
}
```

### GridLayout, VGridLayout, HGridLayout, resolve_grid_tracks, grid_tracks_length

```rust
pub struct VGridLayout { pub columns: Vec<GridItem>, pub alignment: Alignment, pub spacing: Len }   // HGridLayout: `rows`
VGridLayout::new(columns: impl IntoIterator<Item = GridItem>, alignment: impl Into<Alignment>, spacing: impl Into<Len>) -> Self
pub struct GridLayout { pub alignment: Alignment, pub horizontal_spacing: Len, pub vertical_spacing: Len }   // Copy
GridLayout::new(alignment: impl Into<Alignment>, horizontal_spacing: impl Into<Len>, vertical_spacing: impl Into<Len>) -> Self
fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild]) -> Size
fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild])
// all three also implement `Layout` (view(children) builds the native grid)
pub fn resolve_grid_tracks(items: &[GridItem], length: Option<f32>, spacing: impl Into<Len>) -> Vec<GridTrack>
pub fn grid_tracks_length(tracks: &[GridTrack]) -> f32
```

The layouts behind the grid views, for use inside your own `Layout` (call `size_that_fits`/`place_children` on them with
your children). They also implement `Layout` (usable as `(layout) { .. }`), but `VGrid`/`HGrid`/`Grid` are the normal way to
get a native grid. `Default` of `VGridLayout`/`HGridLayout` is one flexible track,
centered, default spacing. `resolve_grid_tracks(items, Some(width), spacing)` gives the tracks (`None` = ideal sizes);
`grid_tracks_length` is their total including spacing.

```rust
let tracks = resolve_grid_tracks(&[GridItem::fixed(40), GridItem::flexible()], Some(300.0), 8);
let total = grid_tracks_length(&tracks);
```

---

## 3. Scroll

### ScrollView

```rust
pub struct ScrollView { .. }
ScrollView::new(content: impl IntoView) -> Self
fn axes(self, axes: impl Into<ScrollAxes>) -> Self                  // default Vertical
fn shows_indicators(self, shows: impl IntoProp<bool>) -> Self        // default true
fn position(self, position: ScrollPosition) -> Self
fn sticks_to_end(self, sticks: impl IntoProp<bool>) -> Self          // default false
// elg!: ScrollView { content }   ScrollView(content)   ScrollView(axes: .horizontal, shows_indicators: false) { .. }
```

Shows its content at its ideal size along the scroll axes, clipped to the space it is offered (it takes all of it).
Several views in its block stack vertically (VStack, default spacing). Scrolls with wheel, trackpad, scroll bars, and keys
(arrows, Page Up/Down, Home/End when unhandled by the focused view); Tab scrolls focused views into sight; Shift+wheel scrolls
sideways. Trackpad inertia/rubber-banding on macOS. A scroll that starts over the page keeps scrolling the page.
`sticks_to_end`: chat/log behaviour, starts at the end and follows growth while scrolled to the end; scrolling up stops
following until you return to the bottom.
`LazyVStack`/`LazyHStack` should be directly inside (they need the viewport).

```rust
elg! {
    ScrollView(axes: .horizontal, shows_indicators: false) {
        HStack { /* cards */ }
    }
}
elg! {
    ScrollView(sticks_to_end: true) {
        LazyVStack { ForEach(messages, id: |m| m.id) { m in Text(m.text.clone()) } }
    }
}
```

### ScrollAxes

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollAxes { #[default] Vertical, Horizontal, Both }   // dots: .vertical .horizontal .both
```

### ScrollPosition

```rust
pub struct ScrollPosition { .. }     // Clone (clones refer to the same scroll view), Default
ScrollPosition::new() -> Self
fn scroll_to_top(&self, animated: bool)
fn scroll_to_end(&self, animated: bool)
fn scroll_to(&self, id: impl IntoItemKey)     // scrolls the item with this ForEach key (in a lazy stack inside) to the top
```

Scroll from code: make one, give a clone to `ScrollView(position: ..)`, call it from handlers (`animated` is required).
`position` takes the value (not a closure/State), so clone it for each handler.

```rust
let position = ScrollPosition::new();
elg! {
    VStack {
        ScrollView(position: position.clone()) {
            LazyVStack { ForEach(messages, id: |m| m.id) { m in Text(m.text.clone()) } }
        }
        HStack {
            Button("Newest", { let p = position.clone(); move || p.scroll_to_end(true) })
            Button("Top", { let p = position.clone(); move || p.scroll_to_top(true) })
            Button("Pinned", move || position.scroll_to(pinned_id.get())) // a ForEach key
        }
    }
}
```

Scroll-related modifiers (`.on_scroll`, `.on_scroll_capture`, the `ScrollModifier` value, `ScrollEvent`) live in modifiers.md
and the input reference.

---

## 4. Navigation and split views

### HSplitView / VSplitView, SplitViewOptions

```rust
pub struct HSplitView { .. }   pub struct VSplitView { .. }     // ::new() -> Self
fn pane(self, pane: impl IntoView) -> Self
fn dividers(self, options: impl FnOnce(SplitViewOptions) -> SplitViewOptions) -> Self
// elg!: HSplitView { Sidebar() Editor() }        HSplitView(dividers: |o| o.divider(.clear).thickness(0)) { .. }
#[derive(Clone)] pub struct SplitViewOptions { pub divider: Prop<Color>, pub active_divider: Option<Prop<Color>>, pub thickness: f32, pub active_thickness: f32 }
SplitViewOptions::default()   // divider theme Outline, active_divider None (tint), thickness 1.0, active_thickness 3.0
fn divider(self, color: impl IntoProp<Color>) -> Self
fn active_divider(self, color: impl IntoProp<Color>) -> Self   // while hovered/dragged
fn thickness(self, thickness: impl Number) -> Self
fn active_thickness(self, thickness: impl Number) -> Self
```

Panes side by side (H) or stacked (V) with draggable dividers (grab area slightly wider than the line). A pane's limits
come from its own frame (`Sidebar().frame(min_width: 180, ideal_width: 240, max_width: 400)`); panes start at ideal size;
when the split view resizes, the pane with the highest `layout_priority` (ties: the last) takes the difference. A pane that is an
`if` can come and go. Prefer `NavigationSplitView` for the standard sidebar/detail app shell; use split views for editors,
inspectors, consoles. Each pane is a block child in `elg!`.

### NavigationSplitView, ColumnWidth

```rust
pub struct NavigationSplitView { .. }
NavigationSplitView::new(sidebar: impl IntoView, detail: impl IntoView) -> Self
NavigationSplitView::three_column(sidebar: impl IntoView, content: impl IntoView, detail: impl IntoView) -> Self
fn sidebar_visible(self, visible: impl IntoBinding<bool>) -> Self
fn sidebar_transition(self, transition: impl Into<Transition>) -> Self   // default slides out to the leading edge
fn sidebar_background(self, color: impl IntoProp<Color>) -> Self          // default theme Surface; Color::CLEAR = none
fn sidebar_width(self, width: ColumnWidth) -> Self      // default (180, 240, 360)
fn content_width(self, width: ColumnWidth) -> Self      // three-column only; default (220, 300, 480)
fn detail_width(self, width: ColumnWidth) -> Self       // default (200, 0, INFINITY); ideal 0 = the rest
fn dividers(self, options: impl FnOnce(SplitViewOptions) -> SplitViewOptions) -> Self
#[derive(Clone, Copy, Debug, Default, PartialEq)] pub struct ColumnWidth { pub min: f32, pub ideal: f32, pub max: f32 }
ColumnWidth::new(min: f32, ideal: f32, max: f32) -> Self   // const fn
// elg!: NavigationSplitView(sidebar_width: ColumnWidth::new(200.0, 260.0, 400.0)) { Sidebar() Detail() }   (2 or 3 views in the block)
```

Sidebar + detail (or sidebar, content, detail) in resizable columns: an `HSplitView` of framed columns; the detail takes the
rest of the width. The block must hold exactly two or three views (else panic). `sidebar_visible` is two-way (follows the
user hiding it); toggle it inside `with_animation` to animate. Sidebar content sits top-leading, detail centered.

```rust
# let show_sidebar = State::new(true);
elg! {
    NavigationSplitView(sidebar_visible: show_sidebar) {
        ScrollView { LazyVStack { ForEach(folders, id: |f| f.id) { f in Text(f.name.clone()) } } }
        Text("Select a note")
    }
}
```

### NavigationStack, PageTransition

```rust
pub struct NavigationStack<R: 'static> { .. }      // R: PartialEq + Clone + 'static
NavigationStack::new<V: IntoView>(path: impl IntoBinding<Vec<R>>, root: impl IntoView, destination: impl Fn(&R) -> V + 'static) -> Self
fn root(self, root: impl IntoView) -> Self
fn destination<V: IntoView>(self, destination: impl Fn(&R) -> V + 'static) -> Self
fn root_title(self, title: impl IntoProp<String>) -> Self
fn title(self, title: impl Fn(&R) -> String + 'static) -> Self
fn transition(self, transition: impl Into<PageTransition>) -> Self
fn animation(self, animation: Option<Animation>) -> Self          // default Some(Animation::smooth(0.35)); None = at once
fn back_gestures(self, enabled: bool) -> Self                       // default true
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PageTransition { #[default] Slide, Fade, None }          // dots: .slide .fade .none
// elg!: NavigationStack(path, destination: |route: &Route| Detail(..), root_title: "Home") { Home() }
```

Pages driven by `path` (a `State<Vec<R>>` or `Binding<Vec<R>>`): the root, then one page per route; push by adding to the
path, pop by removing (a deep link is just a path). `destination` is required (panics at build if missing). Pages under the top
one stay alive (keep state and scroll position); a page is rebuilt when its route changes (`PartialEq`). Alt+Left, and a
two-finger right swipe on a macOS trackpad, go back. By default nothing frames the pages; add
`.navigation_stack_style(BarNavigationStackStyle::new())` for a title bar with a back button (see the styles file for
`NavigationStackStyle`, `BarNavigationStackStyle`, `PlainNavigationStackStyle`). `animation` applies to pushes/pops that are
not already inside `with_animation`. Annotate the closure's route type in `elg!` (`|route: &Route|`).
In `elg!` the block holds the root page (one view; several are wrapped in a VStack). Prefer this over `if`/`match` page
switching when you need back gestures, titles, or kept state.

```rust
# #[derive(Clone, PartialEq)] enum Route { Detail(i64) }
let path: State<Vec<Route>> = State::new(Vec::new());
elg! {
    NavigationStack(path, destination: |route: &Route| match route {
        Route::Detail(id) => Text(format!("Detail {id}")),
    }, root_title: "Home") {
        Button("Open", move || path.update(|p| p.push(Route::Detail(1))))
    }
    .navigation_stack_style(BarNavigationStackStyle::new().height(38))
}
```

---

## 5. Custom layouts and geometry types

### Layout trait

```rust
pub trait Layout: 'static {
    fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild]) -> Size;
    fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild]);
    fn spacer_axis(&self) -> Option<Axis> { None }                       // which way Spacers inside grow
    fn view(self, children: Vec<View>) -> View where Self: Sized;        // provided
}
```

Implement it for your struct, then use it in `elg!` as `(MyLayout { .. }) { children }` (parentheses around the layout
expression, children in a block) or call `MyLayout { .. }.view(vec_of_views)`. Every `Layout` is an `ElgChildren`
(blanket impl). Contract: `size_that_fits` returns the size you want for a proposal (measure children with
`child.size_that_fits(..)` as often as needed); `place_children` places every child with `child.place(position, proposal)`
(`bounds` has the size you reported, positioned by your parent). It can be called several times per pass: keep it cheap, or
use [`CachedLayout`](#cachedlayout-cached). A panic while measuring/placing is reported once and the view then has no size
until its layout is invalidated.
Prefer a custom layout when stacks/grids cannot express the arrangement (flow, radial, masonry); otherwise compose stacks.

```rust
struct Flow { spacing: f32 }
impl Flow {
    fn arrange(&self, max_width: f32, children: &[LayoutChild], bounds: Option<Rect>) -> Size {
        let (mut x, mut y, mut row_height, mut width) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
        for child in children {
            let size = child.size_that_fits(ProposedSize::unspecified()); // its ideal size
            if x > 0.0 && x + size.width > max_width {
                x = 0.0;
                y += row_height + self.spacing;
                row_height = 0.0;
            }
            if let Some(bounds) = bounds {
                child.place(Point::new(bounds.x + x, bounds.y + y), ProposedSize::new(Some(size.width), Some(size.height)));
            }
            x += size.width + self.spacing;
            width = width.max(x - self.spacing);
            row_height = row_height.max(size.height);
        }
        Size::new(width, y + row_height)
    }
}
impl Layout for Flow {
    fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild]) -> Size {
        self.arrange(proposal.width.unwrap_or(f32::INFINITY), children, None)
    }
    fn place_children(&self, bounds: Rect, _: ProposedSize, children: &[LayoutChild]) {
        self.arrange(bounds.width, children, Some(bounds));
    }
}
let tags = elg! { (Flow { spacing: 6.0 }) { Text("C++") Text("Vulkan") Text("UI") } };
let flow = Flow { spacing: 8.0 }.view(tag_views); // tag_views: Vec<View>
```

### LayoutChild

```rust
pub struct LayoutChild { .. }       // valid only during the layout call
fn size_that_fits(&self, proposal: ProposedSize) -> Size
fn place(&self, position: Point, proposal: ProposedSize)     // positions the child's top-left; it sizes itself from `proposal`
fn layout_priority(&self) -> f32
fn is_spacer(&self) -> bool
fn value<K: LayoutValueKey>(&self) -> K::Value               // set with `.layout_value::<K>(v)`, else K::default_value()
```

### LayoutValueKey

```rust
pub trait LayoutValueKey: 'static {
    type Value: Clone + 'static;
    const NAME: Option<&'static str> = None;   // stable unique name to share with C++ (types: bool i32 i64 f32 f64 Color String)
    fn default_value() -> Self::Value;
}
// on a view (ViewExt):
fn layout_value<K: LayoutValueKey>(self, value: K::Value) -> View
```

Per-child data for the parent layout (like a column index). Layout values are plain values: not closures or State. A key
with a `NAME` and one of the shared types matches a C++ key with that name and type; any other key is Rust's own.

```rust
struct Column;
impl LayoutValueKey for Column { type Value = i32; fn default_value() -> i32 { 0 } }
let v = MyGrid.view(vec![Text::new("Name").layout_value::<Column>(0), Text::new("Email").layout_value::<Column>(1)]);
// in the layout: child.value::<Column>()
```

### CachedLayout, Cached

```rust
pub trait CachedLayout: 'static {
    type Cache: 'static;
    fn make_cache(&self, children: &[LayoutChild]) -> Self::Cache;
    fn update_cache(&self, cache: &mut Self::Cache, children: &[LayoutChild]) { /* default: make_cache again */ }
    fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild], cache: &mut Self::Cache) -> Size;
    fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild], cache: &mut Self::Cache);
    fn spacer_axis(&self) -> Option<Axis> { None }
    fn view(self, children: Vec<View>) -> View where Self: Sized;
}
pub struct Cached<L: CachedLayout>(pub L);     // elg!: (Cached(MyLayout { .. })) { children }
```

Use instead of `Layout` when you compute something costly from the children that does not depend on the proposal (their ideal
sizes). The cache is made on first layout, kept across measure/place and frames, and refreshed through `update_cache` when
children change. A `CachedLayout` is not a `Layout` (no blanket `ElgChildren`): in `elg!` wrap it in `Cached(..)`, or call `.view(children)`.

```rust
struct CachedFlow;
impl CachedLayout for CachedFlow {
    type Cache = Vec<Size>;
    fn make_cache(&self, children: &[LayoutChild]) -> Vec<Size> {
        children.iter().map(|c| c.size_that_fits(ProposedSize::unspecified())).collect()
    }
    fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild], cache: &mut Vec<Size>) -> Size { Size::default() }
    fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild], cache: &mut Vec<Size>) {}
}
let v = elg! { (Cached(CachedFlow)) { Text("a") Text("b") } };
```

### VStackLayout, HStackLayout, ZStackLayout

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)] pub struct VStackLayout { pub alignment: HorizontalAlignment, pub spacing: Len }
#[derive(Clone, Copy, Debug, Default, PartialEq)] pub struct HStackLayout { pub alignment: VerticalAlignment, pub spacing: Len }
#[derive(Clone, Copy, Debug, Default, PartialEq)] pub struct ZStackLayout { pub alignment: Alignment }
VStackLayout::new(alignment: impl Into<HorizontalAlignment>, spacing: impl Into<Len>) -> Self   // HStackLayout::new(VerticalAlignment, Len); ZStackLayout::new(Alignment)
fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild]) -> Size
fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild])
fn spacer_axis(&self) -> Option<Axis>
fn view(self, children: Vec<View>) -> View        // the native stack
// each also implements `Layout`
```

The built-in stacks' own measuring/placing, so a custom layout can hand work to them (for example place a header with
`VStackLayout` and delegate the rest).

### ProposedSize, aligned_position

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)] pub struct ProposedSize { pub width: Option<f32>, pub height: Option<f32> }
ProposedSize::new(width: Option<f32>, height: Option<f32>) -> Self
ProposedSize::unspecified() / zero() / infinite() -> Self        // const fns
fn replacing_unspecified(&self, fallback: Size) -> Size
impl From<Size> for ProposedSize
pub fn aligned_position(container: Rect, size: Size, alignment: Alignment) -> Point
```

The space a parent offers: `None` = "your ideal size", `Some(0.0)` = smallest, `Some(INFINITY)` = largest. Offering
`ProposedSize::unspecified()` is how you ask a child for its ideal size. `aligned_position` returns where a child of `size`
goes in `container` for an `Alignment`. Layout modifiers (`LayoutModifier`, see [custom modifiers](#viewmodifier-and-custom-modifiers))
use the same API.

### Geometry quick table

| Type | Definition (fields / constructors) | Notes |
|---|---|---|
| `Point` | `{ x, y: f32 }`; `Point::new(x, y)` | `Default` = 0,0 |
| `Size` | `{ width, height: f32 }`; `Size::new(w, h)` | |
| `Rect` | `{ x, y, width, height: f32 }`; `Rect::new(x, y, w, h)` | `min_x() min_y() max_x() max_y() center() -> Point size() -> Size inset(amount) -> Rect` |
| `EdgeInsets` | `{ top, leading, bottom, trailing: f32 }`; `all(l)`, `symmetric(h, v)` | builder: `.top(n) .leading(n) .bottom(n) .trailing(n) .horizontal(n) .vertical(n) .all_edges(n)`; `From<Number>` = all sides |
| `Edges` | `Edges(u32)` bit set; `TOP LEADING BOTTOM TRAILING HORIZONTAL VERTICAL ALL` | `a \| b`; `contains(edge)` (true if any bit shared); dots `.top .leading .horizontal .all` |
| `CornerRadii` | `{ top_left, top_right, bottom_right, bottom_left }`; `CornerRadii::all(r)` | `From<Number>` |
| `Len` | `Len(pub Option<f32>)` | "optional length"; `From<Number>`, `From<Option<f32>>`; `Len(None)` = default/unset |
| `Number` | `trait Number: Copy { fn to_f32(self) -> f32 }` | ints, `f32`, `f64`, and the `.infinity` dot; args typed `impl Number` take any numeric literal |
| `Axis` | `Horizontal`, `Vertical` | dots `.horizontal .vertical` |
| `HorizontalAlignment` | `Leading`, `Center` (default), `Trailing` | dots `.leading .center .trailing` |
| `VerticalAlignment` | `Top`, `Center` (default), `Bottom` | dots `.top .center .bottom` |
| `Alignment` | `{ horizontal, vertical }`; `Alignment::new(h, v)` | consts `CENTER TOP BOTTOM LEADING TRAILING TOP_LEADING TOP_TRAILING BOTTOM_LEADING BOTTOM_TRAILING`; dots `.top_leading` etc. |
| `UnitPoint` | `{ x, y: f32 }` fractions of a view's size; `UnitPoint::new(x, y)` | consts `CENTER TOP BOTTOM LEADING TRAILING TOP_LEADING ...`; (0,0) is top-leading; used for anchors |
| `ContentMode` | `Fit` (default, all content shows), `Fill` (covers, may overflow: clip) | dots `.fit .fill` |
| `INFINITY` | `pub const INFINITY: f32 = f32::INFINITY` | for flexible frames: `.frame(max_width: .infinity)` |
| `DEFAULT_STACK_SPACING` | `pub const ... : f32 = 8.0` | see [stack defaults](#stack-alignment-spacing-and-defaults) |

`Size`, `Point` and `Rect` are plain value structs (all `Clone, Copy, Debug, Default, PartialEq`).

---

## 6. Custom views, components and modifiers

### View, IntoView, ViewExt

```rust
pub struct View { .. }                      // a cheaply cloned description of UI; nothing exists until mounted
View::empty() -> Self      fn is_empty(&self) -> bool
View::component<V: IntoView>(body: impl Fn() -> V + 'static) -> Self   // body runs each time it is mounted
pub trait IntoView { fn into_view(self) -> View; }
pub trait ViewExt: IntoView + Sized { /* every modifier: .padding .frame .background_in ... .modifier(m) .apply(f) .layout_value::<K>(v) .selection(..) */ }
impl<T: IntoView> ViewExt for T {}
```

`IntoView` is implemented for `View`, `&View`, `()` (empty), `Option<V: IntoView>`, every built-in view/builder, `Text`,
shapes, and every `Component`. `ViewExt` gives all modifiers to anything `IntoView`; each modifier returns a new `View`
(it wraps; order matters: `.padding(8).background(c)` fills the padding, `.background(c).padding(8)` does not). Views are
values: keep them, clone them (`{ badge.clone() }` in `elg!`), reuse them.
Plain function vs component: a function returning a view runs immediately and sees the caller's environment; a
`#[component]` body runs when placed, so it sees the environment where it ends up and re-runs per mount. `.apply(f)`
passes the view to your function (`v.apply(card) == card(v)`) so a long chain stays one chain.

```rust
fn badge(text: &str, color: Color) -> View {
    elg! {
        Text(text.to_owned())
            .font(Font::caption().bold())
            .foreground_style(Color::WHITE)
            .padding(horizontal: 8, vertical: 3)
            .background_in(color, Capsule())
    }
    .into_view()
}
fn expandable_section(title: &str, content: View) -> View {
    let open = State::new(false); // lives as long as the views built here
    elg! {
        VStack(alignment: .leading) {
            Button(title.to_owned(), move || with_animation(Animation::smooth(0.4), || open.toggle()))
                .button_style(PlainButtonStyle)
            if open.get() { { content.clone() } }
        }
    }
    .into_view()
}
```

### Component and #[component]

```rust
pub trait Component: 'static { fn body(&self) -> impl IntoView; }      // any Component is IntoView
#[component]
fn Name(prop: Type, #[prop(default)] a: Type, #[prop(default = expr)] b: Type, opt: Option<T>, reactive: Prop<T>, shared: Binding<T>) -> impl IntoView { .. }
```

`#[component]` (no arguments) turns a function into a struct `Name` that is a `Component`. Rules, from the macro:

| Rule | Detail |
|---|---|
| Function shape | not generic; parameters are plain identifiers (`title: String`), no `self`; must return a view (`-> impl IntoView`) |
| Props | every parameter; type must be `Clone + 'static`. Set by name in `elg!` (`Name(id: 1, title: t)`) or builder calls (`Name::new().title(..)`) |
| Required props | a missing one is a compile error (it shows as `Missing` in the type); `Name::new()` and `Name()` start with all required props missing |
| Setter parameter type | a plain `T` prop: `impl Into<T>` (so `"x"` works for `String`, `.leading` for DotValues types, a `State<T>` converts to `Binding<T>`); a `Prop<T>` prop: `impl IntoProp<T>` |
| `#[prop(default)]` | `Default::default()` when not set |
| `#[prop(default = expr)]` | any expr; may be a leading-dot name (`#[prop(default = .compact)]`) or use dots (`Font::system(12.0, .bold)`); for `Prop<T>` props it is converted with `IntoProp` (`default = "Untitled"` works) |
| `Option<T>` props | default `None` without any attribute; the setter takes `impl Into<Option<T>>`, so `note: "unread".to_string()` or `note: Some(..)` both work |
| `Prop<T>` props | reactive: accepts a value, `State`, `Computed`, `Binding`, a closure `|| T`, `"{text}"` interpolation. Pass it on to any argument typed `impl IntoProp<T>` (`Prop<T>` implements it); `Prop::try_get()` reads it directly when it needs no environment |
| `Binding<T>` props | two-way data: pass a `State<T>` (converts) or a `Binding<T>` (`state.binding()`); closures do not convert |
| Visibility | struct, `new` and setters take the function's visibility (`pub fn` -> public component) |
| Children block | not generated: for `Name { views }` write the struct and `impl Component` + `ElgChildren` by hand |
| Body runs | each time the component is mounted, with the props cloned in |
| Body panics | reported as `AppCode::Body`; debug builds show a red caption, release shows nothing |

```rust
#[derive(Clone, PartialEq)]
struct Contact { id: i64, name: String, avatar_url: String }

#[component]
fn ContactRow(contact: Contact, #[prop(default = 36.0)] size: f32) -> impl IntoView {
    elg! {
        HStack {
            AsyncImage(contact.avatar_url.clone(), content_mode: .fill).frame(size, size).clip_shape(Circle())
            Text(contact.name.clone())
            Spacer()
        }
    }
}
elg! {
    ScrollView { LazyVStack { ForEach(contacts, id: |c| c.id) { c in ContactRow(contact: c.clone()) } } }
}

#[component]
fn Counter(label: Prop<String>, #[prop(default = 0)] start: i32, value: Binding<i32>) -> impl IntoView { /* .. */ }
elg! { Counter(label: "Count", value: my_state) }   // my_state: State<i32>
```

Hand-written component with a children block (implement `Component`, `elegance::dsl::ElgNew<()>` and `elegance::dsl::ElgChildren`;
the `#[component]` macro cannot take a block):

```rust
#[derive(Clone)]
struct Card { children: Vec<View> }
impl Component for Card {
    fn body(&self) -> impl IntoView {
        elg! { { VStack::new().children(self.children.clone()) }.padding(16).background_in(.surface, RoundedRectangle(12)) }
    }
}
impl ElgChildren for Card {
    type Out = Card;
    fn elg_children(mut self, children: Vec<View>) -> Card { self.children = children; self }
}
impl ElgNew<()> for Card {
    type Out = Card;
    fn elg_new(_: ()) -> Card { Card { children: Vec::new() } }
}
// elg! { Card { Text("a") Text("b") } }
```

### #[elg_builder]

```rust
#[elg_builder]                  // on an inherent impl or a trait: adds typed dot-shorthand helpers to pub methods
#[elg_builder(constructor)]     // also implements ElgNew<()> for the impl's `fn new() -> Self`
```

Use it on your own builder types and extension traits so `.name` arguments resolve by the parameter's type (completion and
exact type matching), and so `_with` option methods give named arguments. Details from the macro:

| Aspect | Behaviour |
|---|---|
| Targets | inherent impls (`impl Type { .. }`) and trait definitions; anything else is an error |
| Helpers | for each `pub` method parameter typed `impl Into<T>`, `impl IntoProp<T>` (-> `Prop<T>`), `impl Number` (-> `f32`) or a concrete type, a hidden `__elg_dot_<method>_<index>` function is added |
| `_with` methods | `fn name_with(self, options: impl FnOnce(O) -> O)` (same type in and out) also registers `O` as the options type, so `.name(radius: 24)` in `elg!` expands to `.name_with(|o| o.radius(24))` and `o.radius(.name)` dot-values resolve |
| `constructor` | requires `fn new() -> Self` with no args, not async/unsafe/generic, in a non-trait inherent impl; enables `Name()` and named props in `elg!`. It does **not** implement `Component` or `IntoView`: you still do that. Don't use it if `ElgNew<()>` already exists |
| Plain `#[elg_builder]` | does not register a constructor |

The crate's own views use it on their impls. For an options-style modifier:

```rust
#[derive(Clone, Copy)]
pub struct CardOptions { radius: f32 }
#[elg_builder]
impl CardOptions {
    pub fn radius(mut self, radius: impl Number) -> Self { self.radius = radius.to_f32(); self }
}
struct Card { radius: f32 }
impl ViewModifier for Card {
    fn body(&self, content: View) -> View {
        elg! { { content }.padding(20).background_in(.surface, RoundedRectangle(self.radius)) }.into_view()
    }
}
#[elg_builder]
pub trait CardExt: IntoView + Sized {
    fn card(self) -> View { self.card_with(|o| o) }
    fn card_with(self, options: impl FnOnce(CardOptions) -> CardOptions) -> View {
        self.into_view().modifier(Card { radius: options(CardOptions { radius: 16.0 }).radius })
    }
}
impl<V: IntoView> CardExt for V {}
// elg! { Text("Hello").card(radius: 24) }   ==   Text("Hello").card_with(|o| o.radius(24))
```

### DotValues derive

```rust
#[derive(Clone, Copy, PartialEq, DotValues)]
enum Density { Compact, Comfortable, ExtraRoomy }     // .compact .comfortable .extra_roomy
```

Makes the enum's unit variants leading-dot names in snake_case (`TopLeading` -> `.top_leading`, `Digit0` -> `.digit0`)
wherever a `Density`, `impl Into<Density>` or `impl IntoProp<Density>` (and `Prop<Density>`) is expected in `elg!`, including
`#[component]` props and `#[prop(default = .compact)]`. Requires: an enum, non-generic, `Clone` (the dot value is a clone);
non-unit variants are skipped; `.self/.super/.crate` not allowed. It adds `From<Dot>` and `IntoProp` impls and a hidden
`DensityDotValues` struct.

```rust
#[component]
fn Tile(#[prop(default = .comfortable)] density: Density, tone: Prop<Density>) -> impl IntoView { /* .. */ }
elg! { Tile(density: .compact, tone: .extra_roomy) }
```

### ViewModifier and custom modifiers

```rust
pub trait Modifier { fn apply(self, content: View) -> View; }       // what `.modifier(..)` takes; implemented for every ViewModifier
pub trait ViewModifier: 'static { fn body(&self, content: View) -> View; }
// on a view:  fn modifier(self, modifier: impl Modifier) -> View     fn apply<V: IntoView>(self, function: impl FnOnce(View) -> V) -> View
```

A `ViewModifier` packages other modifiers under one name; `body` runs once when the view is mounted (so it can read the
environment). Apply it with `.modifier(Card { radius: 16.0 })`; for `.card()` syntax add an extension trait (above).
For a function-style one, `.apply(card_fn)` needs nothing else.

The other modifier protocols plug into layout, drawing, inherited values and input; `.modifier(..)` takes a
`ViewModifier` directly, and the other kinds must be wrapped so it knows which you mean:

| Protocol trait | You implement | Wrap with | Types |
|---|---|---|---|
| `LayoutModifier` | `size_that_fits(&self, proposal, content: &LayoutChild) -> Size`, `place(&self, bounds, proposal, content: &LayoutChild)`, optional `layout_priority() -> Option<f32>` | `AsLayout(m)` | wraps one child (use `aligned_position`) |
| `DecorationModifier` | `decoration(&self) -> View`, `placement(&self) -> Placement`, optional `alignment(&self) -> Alignment` | `AsDecoration(m)` | `Placement::{Behind, InFront, Mask}` |
| `EnvironmentModifier` | `apply(&self, values: &mut EnvironmentValues)` | `AsEnvironment(m)` | changes what views inside inherit |
| `EffectModifier` | `apply(&self, effects: &mut Effects)` | `AsEffect(m)` | `Effects::set_opacity/set_clip_shape/set_shadow/set_offset/...` |
| `EventModifier` | `make_handler(&self) -> Box<dyn EventHandler>` | `AsEvent(m)` | `EventHandler` has default-ignoring methods `pointer hover scroll key focus text_input ...` |

```rust
struct MinSize { width: f32, height: f32 }
impl LayoutModifier for MinSize {
    fn size_that_fits(&self, proposal: ProposedSize, content: &LayoutChild) -> Size {
        let size = content.size_that_fits(proposal);
        Size::new(size.width.max(self.width), size.height.max(self.height))
    }
    fn place(&self, bounds: Rect, proposal: ProposedSize, content: &LayoutChild) {
        let size = content.size_that_fits(proposal);
        content.place(aligned_position(bounds, size, Alignment::CENTER), proposal);
    }
}
elg! { Text("OK").modifier(AsLayout(MinSize { width: 44.0, height: 44.0 })) }
```

The built-in modifiers also exist as values for `.modifier(..)` (`PaddingModifier`, `FrameModifier`, `BackgroundModifier`,
`ScrollModifier`, `LayoutValueModifier<K>`, `RowStyleModifier`, ... re-exported from `elegance::modifier`); they are listed in
modifiers.md.

### The elg! DSL grammar summary

```text
node     := element | `{` expr `}` mods | `(` layout_expr `)` `{` node* `}` mods | if | match
element  := Path(args)? { block }? mods          // a bare path without ( ) or { } is an expression (a variable holding a view)
args     := (expr | ident `:` expr),*             // positional first, then `key: value`
block    := node*  |  pat `in` node               // `{ item in View }` for ForEach/AsyncImage (calls elg_content)
mods     := (`.` name (`::<T>`)? args?)*          // `.m(a, key: v)` == `.m_with(a, |o| o.key(v))`
if       := if cond { node } (else if .. | else { node })?
match    := match expr { pat (if guard)? => node | { node } , ... }
```

| DSL | Expands to / means |
|---|---|
| `Name(a, b)` | `Name::elg_new((a, b))`; overloads are tuple impls of `ElgNew` (e.g. `Spacer(20)`, `If(c, a, b)`) |
| `Name(a, key: v)` | the same, then the builder call `.key(v)` (`VStack(spacing: 4)`) |
| `Name { c1 c2 }` | `.elg_children(vec![c1, c2])` (`ElgChildren`): a stack's views, a button's label, a ScrollView's content, a NavigationStack root |
| `ForEach(xs, id: \|x\| x.id) { x in .. }` | `.id(..).elg_content(\|x\| ..)`; the closure gets `&T` |
| `if cond { .. } else { .. }` | reactive `If` (cond in `move \|\| -> bool`) |
| `match v { P => View, .. }` | reactive `match_view`: rebuilt when `v` changes (`v: Clone + PartialEq`) |
| `.leading`, `.accent`, `.bold` | leading-dot shorthand, converted to the parameter's type (`DotValues`, `Into`, `IntoProp`) |
| `"{count} left"` | a reactive string `move \|\| format!(..)` (formatted once where a plain `String` is needed) |
| `{ expr }` | any Rust value that is `IntoView` (children lists too: put variables in braces) |
| `(layout) { .. }` | custom `Layout` (or `Cached(..)`) over the children |
| `.m(a, key: v)` | modifier with named options: `.m_with(a, \|o\| o.key(v))` |
| `$name` | compile error: pass the state itself |

Compute values before `elg!` rather than inside blocks. A `{` on a later line starts the next node
(an embedded expression), so an element's `{ children }` must open on the element's line. Parse errors are reported
individually and the rest still expands. Use `move` closures for handlers; `State`/`Binding`/`Computed` are `Copy`, others
need `.clone()`.

---

## 7. Not exposed in Rust / unverified

C++ items in this scope with no Rust counterpart found in `/workspaces/mikros/elegance-rs/elegance/src`:

- `LayoutChild::value_named<T>(name)` (C++ docs, custom-layouts guide): Rust only has `value::<K>()` (named keys are via
  `LayoutValueKey::NAME`). Not found in `include/elg/*.h` either (docs mention it; header search for `value_named` empty).
- The C++ `SplitView` named in the views table of `views-and-modifiers.mdx`: Rust has `HSplitView`/`VSplitView` only.
- `NavigationStackOptions`, `NavigationSplitViewOptions`, `VGridOptions`, `HGridOptions`, `GridOptions`, `GridRowOptions`,
  `ScrollViewOptions`, `SeparatorOptions` (C++ option structs): Rust uses builder methods/named `elg!` arguments instead.
- `GridItem::with_alignment` / `with_spacing` (C++ names): Rust is `.alignment()` / `.spacing()`.

Unverified: the hand-written `Component` with a children block (shape taken from the traits in `view.rs` and `dsl.rs` and the
test's `Card`; no repo example found), and whether `NavigationSplitView(sidebar_width: ..)`
named-arg form compiles in `elg!` (it follows the `Name(key: v)` rule onto the existing builder methods; no test uses it).

---

## Part 2: Text, controls, input, drawing, images, media

# Components 2: text, controls, text input, shapes, images, media, GPU images

Rust (`elegance` crate, `use elegance::prelude::*;`) reference for content views. All names below are exported by the prelude unless noted. Styles of controls: see theming-and-style.md.

## Contents

- [Text](#text)
- [Buttons and controls](#buttons-and-controls)
- [Text input](#text-input)
- [Shapes and drawing](#shapes-and-drawing)
- [Images](#images)
- [Media](#media)
- [GPU images](#gpu-images)
- [C++-only / not exposed in Rust](#c-only--not-exposed-in-rust)

Conventions: reactive inputs take `impl IntoProp<T>` (a plain value, a `State`/`Computed`/`Binding`, or a closure `move || ...`); never pass a read value if it should update. Controls that edit a value take `impl IntoBinding<T>` (usually a `State<T>`). The platform enum is `Platform { Windows, MacOS, Linux }` (no Mikros variant; `current_platform()` reports anything else as Linux).

---

## Text

### Text
`Text::new(content: impl IntoProp<String>) -> Text`; `Text("...")` in `elg!`; function form `Text(content)` for arguments (`Button(Text("Save").bold(), save)`).

A run of text in the environment's font and foreground. Takes the width its words need; given less, wraps between words and, out of lines, ends with an ellipsis. Newlines start paragraphs. Invalid UTF-8 is shown as U+FFFD. Right-to-left scripts are not supported yet.

Builder methods (all return `Text`, so they chain until a generic `ViewExt` modifier turns it into a `View`):

| Method | Notes |
|---|---|
| `.font(impl IntoProp<Font>)` | static or reactive; unregistered family is an error when the text appears |
| `.font_weight(impl IntoProp<Weight>)` | |
| `.bold()` | |
| `.foreground_style(impl IntoProp<Color>)` | `.secondary`, `Color::hex(..)`, ... |
| `.line_limit(lines: i32)` | at most this many lines, ending in an ellipsis (plain `i32`, not reactive; 0 or less: no limit) |
| `.multiline_text_alignment(impl Into<TextAlignment>)` | how wrapped lines line up; default `.leading` |
| `.selectable(impl IntoProp<bool>)` | pointer selection + copy (drag, double click word, triple click paragraph, Shift+click extend, primary-mod+C copy, +A select all). One selection per window. Not inside a button/control. Not for styled text. |

Prefer it when: showing a static or reactive string in one style. Use `StyledText` when runs differ in style/click/inline views.

```rust
# let (body, caption) = ("A long article", "A caption");
# let count = State::new(3);
Text("Hello")
Text("{count} unread")                       // reactive: see Interpolation
Text(body).line_limit(3)
Text(caption).multiline_text_alignment(.center)
Text("Title").font(.title).bold().foreground_style(.secondary)
Text(body).selectable(true)
```

Truncation/wrapping: the only controls are `line_limit` and `multiline_text_alignment`; truncation is always an ellipsis at the end (no head/middle truncation, no tight-spacing or no-wrap option exists in the Rust crate or C++ `text.h`).

### Interpolation
In `elg!`, a whole string-literal argument containing `{name}` becomes a reactive string: `"{count} left"` expands to `move || format!("{count} left")` (wrapped in `Interpolated`, which implements `IntoProp<String>` and `IntoLabel`). Expressions such as `{size.0}` and format specs (`{x:.1}`) work. States are read by reference through a tracked read.

- Works where a `String` prop is taken: `Text`, accessibility labels, titles, and as a control label (becomes a reactive `Text`).
- Where a plain `String` is needed (`impl Into<String>`), it is formatted once, not reactively.
- Outside `elg!` write the closure yourself: `Text(move || format!("{} items", n.get()))`.
- `$name` is an error: pass the state itself.

```rust
let count = State::new(0);
elg! { VStack { Text("Count: {count}") Button("{count} clicks", move || count += 1) } }
```

### StyledText
`pub struct StyledText(pub Vec<TextRun>)` (implements `IntoView`; in `elg!` put it in braces: `{ message }`).

Runs of differently styled text laid out as one paragraph that wraps as a whole. Anything a run does not set comes from the environment. Selection does not work for it.

```rust
# let open_profile = || {};
let message = StyledText(vec![
    TextRun::new("Hey "),
    TextRun::new("@Ana")
        .style(RunStyle::default().color(Color::hex(0x7AA2F7)).background(Color::hex(0x2B3A67)))
        .on_tap(open_profile),
    TextRun::new("this").style(RunStyle::default().weight(Weight::Bold)),
]);
let _ = elg! { { message } };
```

### TextRun
```rust
pub struct TextRun { pub text: String, pub style: RunStyle, pub on_tap: Option<Box<dyn FnMut()>>,
                     pub attachment: Option<View>, pub attachment_size: f32 }
TextRun::new(text: impl Into<String>) -> Self        // attachment_size defaults to 1.25
.style(RunStyle) -> Self
.on_tap(action: impl FnMut() + 'static) -> Self      // clickable run (links, mentions)
.attachment(view: impl IntoView, size: f32) -> Self  // inline view (icon, chip) sized relative to the font
impl<S: Into<String>> From<S> for TextRun
```

### RunStyle
```rust
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunStyle { pub weight: Option<Weight>, pub italic: bool, pub family: String,
    pub color: Option<Color>, pub background: Option<Color>, pub underline: bool,
    pub strikethrough: bool, pub keep_together: bool }
.weight(impl Into<Weight>) .italic(bool) .family(impl Into<String>) .color(impl Into<Color>)
.background(impl Into<Color>) .underline(bool) .strikethrough(bool) .keep_together(bool)
```
`family`: a registered family (empty: the font's, e.g. a monospace family for code). `background`: rounded highlight; implies `keep_together` (never broken across lines). Unset parts come from the environment.

### TextSpan
```rust
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSpan { pub begin: usize, pub end: usize, pub style: RunStyle, pub label: Option<String> }
```
A styled range of a `TextField`/`TextArea`'s text, returned from `.styler(...)`: `begin`/`end` are byte offsets; `label` is shown instead of the text (a mention chip's name). See Text input.

### TextAlignment
`enum TextAlignment { Leading (default), Center, Trailing }`. Used by `.multiline_text_alignment(.center)`.

### Font (basics)
`Font` is `Font::system(size: f32, weight: impl Into<Weight>)`, `Font::custom(family, size, weight)`, text styles `Font::large_title() title() title2() title3() headline() body() callout() subheadline() footnote() caption() caption2()`, and builders `.weight(w) .bold() .with_size(f32) .italic(bool)`. `Weight` constants: `UltraLight Thin Light Regular Medium Semibold Bold Heavy Black`, or `Weight::new(1..=1000)`. Registration: `register_font(family: &str, file: impl AsRef<Path>, weight)`, `register_font_data(family: &str, data: &[u8], weight)`, `register_emoji_font(file)` (all `Result<(), Error>`). Fonts and foreground are environment values inherited by everything inside a container: `VStack{..}.font(.callout)`. Prefer text styles (`.title`) over raw sizes. Details, theme tokens and environment: see theming-and-style.md.

```rust
Text("Title").font(.title).bold()
let f = Font::system(32.0, Weight::Bold);
register_font("Inter", "fonts/Inter-Regular.ttf", Weight::Regular)?;
```

---

## Buttons and controls

Controls handle hover/press feedback, keyboard focus, keyboard activation and screen readers themselves. Look comes from styles (`.button_style(..)`, `.toggle_style(..)`, `.checkbox_style(..)`, `.slider_style(..)`, `.picker_style(..)`, `.progress_style(..)`, `.number_input_style(..)`, `.menu_style(..)`, e.g. `BorderedButtonStyle`, `PlainButtonStyle`, `SwitchToggleStyle::new().fills_width(true)`, `MenuPickerStyle`, `CircularProgressStyle::new()`): see theming-and-style.md. `.tint(color)` and `.disabled(impl IntoProp<bool>)` also work on containers.

### IntoLabel
`pub trait IntoLabel { fn into_label(self) -> LabelArg }` plus `pub enum LabelArg { Title(Prop<String>), View(View) }` (the latter is `elegance::views::LabelArg`, not in the prelude). What `Button`, `Toggle`, `Checkbox`, `MenuButton` accept as a label: `&str`, `String`, `&String`, any `IntoView` (a `Text`, `Label`, `Image`, any view), `Prop<String>`, `State<String>`, `Binding<String>`, `Computed<String>`, and interpolated strings. A reactive plain-closure title cannot be a label directly; use the control's `.title(impl IntoProp<String>)` or a `Text(move || ..)` view as the label.

### Button
```rust
Button::new(label: impl IntoLabel, action: impl FnMut() + 'static) -> Button
.action(impl FnMut() + 'static)  .label(impl IntoLabel)  .title(impl IntoProp<String>)
```
`Button("Save", save)`, or with a view label as the block: `Button(action: save) { Label("Save", "floppy-disk") }`. Runs the action on click, or Return/Space while focused. Default look fills with the tint; `BorderedButtonStyle` / `PlainButtonStyle` for quieter ones. Icon-only buttons need `.accessibility_label("...")`.

```rust
# let (save, remove, toggle) = (|| {}, || {}, || {});
# let playing = State::new(false);
Button("Save", save)
Button(Label("Share", "share-network"), save)
Button(Image("trash"), remove).accessibility_label("Delete")
Button(Text(move || if playing.get() { "Pause" } else { "Play" }), toggle)
Button("Submit", save).disabled(move || !playing.get())
```

### Toggle
```rust
Toggle::new(label: impl IntoLabel, is_on: impl IntoBinding<bool>) -> Toggle
Toggle::unlabeled(is_on: impl IntoBinding<bool>) -> Toggle   // `Toggle(is_on)` in elg!
.title(impl IntoProp<String>)
```
A switch that turns something on (Wi-Fi). Click/Space/Return flips it. Prefer `Toggle` for "on/off setting", `Checkbox` for "choose this option / list item". A plain value is not a binding: use `Binding::constant(true)`. Unlabeled: add `.accessibility_label(..)` (wrap in braces in `elg!`: `{ Toggle::unlabeled(wifi) }.accessibility_label("Wi-Fi")`).

### Checkbox
```rust
Checkbox::new(label: impl IntoLabel, is_checked: impl IntoBinding<bool>) -> Checkbox
Checkbox::unlabeled(is_checked: impl IntoBinding<bool>) -> Checkbox
.title(impl IntoProp<String>)
.mixed(impl IntoProp<bool>)    // dash shown; checking it then checks fully
```
```rust
# let (wifi, remember) = (State::new(true), State::new(false));
elg! { VStack { Toggle("Wi-Fi", wifi) Checkbox("Remember me", remember) } }
let all = Binding::new(move || files.read().iter().all(|f| f.selected), move |on| { /* select all */ });
elg! { Checkbox("Select all", all, mixed: move || some() && !all.get()) }
```

### Slider
```rust
Slider::new(value: impl IntoBinding<f32>) -> Slider
.min(impl IntoProp<f32>)   // default 0
.max(impl IntoProp<f32>)   // default 1
.step(impl IntoProp<f32>)  // 0: continuous
.on_editing_changed(impl Fn(bool) + 'static)   // true when a drag starts, false when it ends
```
Value in [min, max] by dragging or keys (arrows, PageUp/Down, Home/End). As wide as offered. A new range moves the thumb without writing the value. The value type is `f32`: bind a `State<f32>`.
```rust
# let (volume, size, brightness) = (State::new(0.5_f32), State::new(12.0_f32), State::new(1.0_f32));
elg! { VStack {
    Slider(volume)
    Slider(size, min: 8, max: 72, step: 1)
    Slider(brightness, on_editing_changed: move |dragging| if !dragging { /* save */ })
} }
```

### Picker
```rust
Picker::<T>::new<S: Into<String>>(selection: impl IntoBinding<T>, options: impl IntoIterator<Item = (T, S)>) -> Picker<T>
// T: PartialEq + Clone + 'static
```
One of several options; each option is a `(value, label)`. Default look is segments; `.picker_style(MenuPickerStyle)` shows the chosen option on a button that opens a menu (pick it for many options). If the bound value matches no option, nothing is selected.
```rust
#[derive(Clone, Copy, PartialEq)] enum Size { Small, Medium, Large }
let size = State::new(Size::Medium);
elg! { Picker(size, [(Size::Small, "S"), (Size::Medium, "M"), (Size::Large, "L")]).picker_style(MenuPickerStyle) }
```

### Selection (and SelectionKey)
```rust
pub trait Selection: 'static { type Key: SelectionKey; ... }
pub trait SelectionKey: Clone + Ord + 'static   // integers (i8..isize, u8..usize) and String
```
Not a control: the bound value type for the `ViewExt::selection(selection)` / `selection_with(selection, on_activate: impl Fn(&S::Key))` modifier on a stack over a `ForEach` (rows selectable; the keys are the ForEach's keys). Implemented for `State<Option<K>>`/`Binding<Option<K>>` (single) and `State<BTreeSet<K>>`/`Binding<BTreeSet<K>>` (multiple). Hidden `#[doc(hidden)]` items are internal. For lists, see the lists reference.
```rust
let open = State::new(None::<i64>);
ScrollView(LazyVStack(ForEach(mails, &Mail::id, MailRow)).selection(open))
```

### Progress
```rust
Progress::new() -> Progress
.fraction(impl IntoProp<Option<f32>>)
```
`Progress()` = activity of unknown length; `Progress(0.4)` = 40% bar; `Progress(move || Some(x))` / `Progress(move || (total > 0).then(|| d / total))` for a fraction that may be unknown (`None` shows indeterminate). Bar by default; `.progress_style(CircularProgressStyle::new().size(14).line_width(2))` or `LinearProgressStyle`. No label of its own: put a `Text` beside it or `.accessibility_label`. An indeterminate one animates while on screen, so remove it when done. Note: a closure returning a plain `f32` is not accepted; wrap in `Some`.

### NumberInput
```rust
NumberInput::<T: NumberValue>::new(value: impl IntoBinding<T>) -> NumberInput<T>
.min(impl IntoProp<f64>)       // default -inf
.max(impl IntoProp<f64>)       // default +inf
.step(impl IntoProp<f64>)      // default 1 (Shift: ten steps)
.decimals(impl IntoProp<i32>)  // default -1 (automatic)
.format(impl Fn(f64) -> String + 'static)
.parse(impl Fn(&str) -> Option<f64> + 'static)
.live(impl IntoProp<bool>)          // default false (applies on Return/blur)
.shows_buttons(impl IntoProp<bool>) // default true
.scrubs(impl IntoProp<bool>)        // default true (drag sideways)
pub trait NumberValue: Copy + 'static   // implemented for i32, f32, f64
```
Typed, stepped (arrows, Up/Down) or scrubbed number. Non-numbers revert on leaving the field; values stay in [min, max]. Put a `Text` beside it for a label. Style: `.number_input_style(..)` (see theming-and-style.md).
```rust
let guests = State::new(2);
let opacity = State::new(1.0_f64);
elg! { VStack {
    HStack { Text("Guests") NumberInput(guests, min: 1, max: 12) }
    NumberInput(opacity, min: 0, max: 1, step: 0.05, decimals: 2)
} }
```

### Menu, MenuButton, MenuItem, Submenu (in-window menu views)
```rust
MenuButton::new<V: IntoView>(label: impl IntoLabel, menu: impl Fn() -> V + 'static) -> MenuButton
  .title(impl IntoProp<String>)  .placement(impl FnOnce(FloatingPlacement) -> FloatingPlacement)
  .edge(impl Into<FloatingEdge>)  .shows_indicator(impl IntoProp<bool>)  .motion(PopupMotion)
Menu::new() -> Menu;  .item(impl IntoView) -> Menu;   // `Menu { ... }` in elg! takes children
MenuItem::new(title: impl IntoProp<String>, action: impl FnMut() + 'static) -> MenuItem
MenuItem::without_action(title: impl IntoProp<String>) -> MenuItem   // choosing it only closes the menus
  .shortcut(impl IntoProp<String>)   // hint text only, e.g. "Ctrl+R"
  .destructive(impl IntoProp<bool>)  .checked(impl IntoProp<bool>)  .image(impl Into<ImageSource>)
Submenu(title: impl IntoProp<String>, menu: impl Fn() -> V + 'static) -> View   // `Submenu("More") { ... }` form: ElgNew<(S, F)>
```
`MenuButton` opens the menu its closure builds, in a popup window (below the button, flipping when no room; `.edge(.top)` for buttons near the bottom). `Menu` is the panel; items can be `MenuItem`, `Submenu`, `Separator()` or any view. Choosing an item closes the menu, then runs its action. `MenuItem` has no `#[elg_builder]` dot helpers (plain method calls). The window menu bar, `MenuBarItem`, `TrayMenuItem` and context menus belong to the app/menus reference.
```rust
elg! { MenuButton("Actions", || elg! {
    Menu {
        MenuItem("Rename", rename).shortcut("F2")
        Separator()
        Submenu("Share") { Menu { MenuItem("Copy link", copy) } }
        MenuItem("Delete", remove).destructive(true)
    }
}) }
```

### Label
`Label(title: impl IntoProp<String>, icon: impl Into<ImageSource>) -> View` (also `Label("Record", "microphone")` in elg!). An icon plus a title side by side, spaced right for buttons, menus and lists. Prefer it over `HStack{Image Text}` for a titled icon. See Images.

### Separator and layout controls
`Separator` is a thin line (`Separator()`, `Separator(.vertical, inset: 4)`; `.axis .color .thickness .inset`). It and the stacks belong to the layout reference; listed here only because menus use them.

---

## Text input

### TextField and TextArea
```rust
pub type TextField = TextFieldBuilder<false>;   // one line (or a few)
pub type TextArea  = TextFieldBuilder<true>;    // multiline editor
TextField::new(placeholder: impl IntoProp<String>, text: impl IntoBinding<String>) -> Self   // TextArea::new likewise
// both:
.on_submit(action: impl FnMut() + 'static)
.styler(impl Fn(&str) -> Vec<TextSpan> + 'static)    // style ranges as edited; spans sorted, non-overlapping, byte offsets
.on_key(impl Fn(&KeyEvent) -> bool + 'static)        // sees key presses before the field; return true to take one
// TextField only (not on TextArea):
.secure(impl IntoProp<bool>)       // password: bullets, no copy, no IME, one line; reactive to reveal
.line_limit(impl IntoProp<i32>)    // >1: wraps and grows to that many lines, then scrolls; Shift+Enter = newline
```
Typing, IME (CJK), mouse and keyboard selection, word/line selection, clipboard and undo are built in. Only valid UTF-8 is held; broken sequences become U+FFFD and are written back only when the user edits.

- `TextField`: Enter submits.
- `TextArea`: Enter adds a newline, Ctrl+Enter (Cmd+Enter on macOS) submits; takes the height offered and scrolls; inside a `ScrollView` it grows with its text instead.
- Prefer `TextField` for names/search/chat boxes (with `line_limit`), `TextArea` for notes and documents.
- Style: `.text_field_style(..)` (`PlainTextFieldStyle`, `RoundedBorderTextFieldStyle`, see theming-and-style.md).

```rust
# let (query, password, note) = (State::new(String::new()), State::new(String::new()), State::new(String::new()));
let reveal = State::new(false);
elg! { VStack {
    TextField("Search", query, on_submit: move || search(query.get()))
    TextField("Password", password, secure: move || !reveal.get())
    TextArea("Notes", note).frame(height: 200)
} }
```

### Focus (by mention)
Controls take focus on click and Tab/Shift+Tab walk them in order with a focus ring. `ViewExt` (view modifiers, covered in the input/modifiers reference): `.focusable(bool)` (false: it and everything inside never take focus), `.focused(impl IntoBinding<bool>)` (two-way; setting true focuses the view), `.on_key(..)`, `.on_key_press(key, action)`, `.on_key_press_capture`. Focused views inside a scroll view are scrolled into sight.
```rust
let editing = State::new(false);
elg! { VStack { TextField("Title", title).focused(editing)  Button("Rename", move || editing.set(true)) } }
```

### TextInputEvent and TextInputKind
```rust
pub enum TextInputKind { Commit (default) /* insert the text */, Composition /* IME in-progress text, replacing the previous one; "" ends it */ }
pub struct TextInputEvent { pub kind: TextInputKind, pub text: String, pub cursor: usize /* Composition: IME caret, byte offset into text */ }
ViewExt::on_text_input(impl Fn(&TextInputEvent) -> bool + 'static) -> View          // makes the view focusable
ViewExt::on_text_input_capture(impl Fn(&TextInputEvent) -> bool + 'static) -> View  // sees text meant for a focused view inside; does not make it focusable
TextInputModifier::new(impl Fn(&TextInputEvent) -> bool + 'static)                   // modifier value form, for `.modifier(..)`
```
For custom text-receiving views (a canvas editor). Return true to consume the text. Key events (`KeyEvent`, `Key`, `KeyModifiers`) are separate from typed text.
```rust
Canvas(|_| {}).on_text_input(|e| {
    if e.kind == TextInputKind::Commit { insert(&e.text); }
    true
})
```

---

## Shapes and drawing

When to use which:

| Need | Use |
|---|---|
| A solid or gradient rectangle/pill/circle as a standalone view | a shape view: `Rectangle()`, `Capsule()`, ... `.fill(..)` `.frame(..)` |
| A fill behind an existing view (tag, card, chip) | `view.background(style)` (its bounds) or `view.background_in(style, shape)` |
| Clip an existing view | `.clip_shape(shape)`, `.corner_radius(r)`; hit-test region `.content_shape(shape)` |
| A frosted-glass panel | `.backdrop_blur(radius)` / `.backdrop_blur_in(radius, shape, \|o\| ...)` with `BackdropFill` semantics |
| Vector artwork (icons, SVG path) | `PathShape` (fixed path, view box scales it) |
| A shape depending on its frame | `CustomShape(\|r: Rect\| Path, rule)` |
| Charts, plots, animations, many primitives | `Canvas(\|g\| ...)` immediate-mode drawing |

Shape views take all the space offered: give them a frame or use them in a background. With neither fill nor stroke a shape uses the environment's foreground color. Fills and stroke colors can be reactive and animate.

### Shape
`pub struct Shape` (a view; clonable; `impl IntoView`; also accepted anywhere `impl Into<Shape>` is, such as `.background_in`, `.clip_shape`, `.content_shape`, `.backdrop_blur_in`).
```rust
.fill(style: impl IntoProp<ShapeStyle>) -> Shape                    // color or gradient (Color, LinearGradient, ... convert)
.stroke(color: impl IntoProp<Color>, line_width: impl Number) -> Shape        // centered on the outline
.stroke_style(color: impl IntoProp<Color>, style: StrokeStyle) -> Shape       // caps, joins, dashes
.stroke_border(color: impl IntoProp<Color>, line_width: impl Number) -> Shape // inside the outline, stays within bounds
```
```rust
Circle().fill(.accent).frame(48, 48)
RoundedRectangle(12).fill(.surface).stroke_border(.outline, 1)
Circle().stroke_style(.accent, StrokeStyle::default().width(3.0).cap(LineCap::Round).dash(vec![6.0, 4.0]))
Text("Tag").padding(6).background_in(.surface_raised, Capsule())
Image(ImageSource::file("/path/avatar.jpg")).resizable().frame(48, 48).clip_shape(Circle())
```

### Rectangle, RoundedRectangle, Circle, Capsule
```rust
Rectangle() -> Shape                                       // also an elg! element
RoundedRectangle(radii: impl Into<CornerRadii>) -> Shape   // a number (all corners) or CornerRadii { top_left, top_right, bottom_right, bottom_left }
Capsule() -> Shape                                         // fully rounded ends
Circle() -> Shape                                          // largest circle that fits, centered
```
Drawn as signed distance fields (cheap, crisp at any size). `RoundedRectangle(CornerRadii { top_left: 12.0, top_right: 12.0, ..Default::default() })`.

### Shape trait
There is no `Shape` trait in the Rust crate: `Shape` is a concrete struct (above) and the elements are zero-sized marker types (`Rectangle`, `RoundedRectangle`, `Circle`, `Capsule`, `PathShape`, `CustomShape`) implementing `ElgNew<..>` with `Out = Shape`. Do not write `impl Shape for ...`; build a `CustomShape` instead.

### PathShape and PathShapeOptions
```rust
PathShape(path: Path) -> Shape                                  // in elg!: PathShape(path) / PathShape(path, options)
PathShape(path: Path, options: PathShapeOptions) -> Shape       // function form
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PathShapeOptions { pub view_box: Option<Rect>, pub fill_rule: FillRule }
```
A fixed path as a shape. With `view_box` (SVG viewBox) the path is scaled to fit the frame and centered; `None`: path is in points. Paths use stencil-and-cover with multisampled edges, cached while unchanged.
```rust
PathShape(heart, PathShapeOptions { view_box: Some(Rect::new(0.0, 0.0, 24.0, 24.0)), ..Default::default() })
    .fill(Color::hex(0xBB9AF7)).frame(48, 48)
PathShape(ring, PathShapeOptions { fill_rule: FillRule::EvenOdd, ..Default::default() }).fill(.accent)
```

### CustomShape
```rust
CustomShape(path: impl Fn(Rect) -> Path + 'static) -> Shape      // elg! form, non-zero fill (ElgNew<(F,)>)
CustomShape(path: impl Fn(Rect) -> Path + 'static, fill_rule: impl Into<FillRule>) -> Shape
```
Builds its path from the frame it gets and rebuilds on size change (bounds `Rect` have `.width`/`.height` fields).
```rust
CustomShape(|r: Rect| {
    let mut p = Path::new();
    p.move_to(Point::new(0.0, r.height)).line_to(Point::new(r.width / 2.0, 0.0))
     .line_to(Point::new(r.width, r.height)).close();
    p
}, FillRule::NonZero).fill(.accent)
```

### Path
```rust
Path::new() -> Path;  Path::svg(data: &str) -> Result<Path, Error>    // all SVG path commands, abs+rel; errors name the position
// builders (each returns &mut Self, so chain on a `let mut`):
.move_to(Point) .line_to(Point) .quad_to(control, end) .cubic_to(c1, c2, end)
.arc_to(rx: f32, ry: f32, rotation_degrees: f32, large_arc: bool, sweep: bool, end: Point)   // SVG-style
.close() .add_rect(Rect) .add_rounded_rect(Rect, radius: f32) .add_ellipse(Rect)
.add_circle(center: Point, radius: f32) .add_path(&Path)
// queries:
.is_empty() -> bool  .bounds() -> Rect  .contains(Point, rule: impl Into<FillRule>) -> bool
.transformed(scale_x, scale_y, dx, dy: f32) -> Path          // scaled, then moved
.flatten(tolerance: f32) -> Vec<Polyline>
.stroked(&StrokeStyle, tolerance: f32) -> Path               // the stroke outline as a fillable path
```
`Clone`, `PartialEq`, `Default`, `Debug`. Coordinates are points.
```rust
let mut triangle = Path::new();
triangle.move_to(Point::new(0.0, 20.0)).line_to(Point::new(20.0, 20.0)).line_to(Point::new(10.0, 0.0)).close();
let heart = Path::svg("M12 21s-7.5-4.6-9.6-9.3C.9 8.4 3.2 4.5 6.9 4.5z")?;
```

### Polyline
`#[derive(Clone, Debug, Default, PartialEq)] pub struct Polyline { pub points: Vec<Point>, pub closed: bool }`: a flattened subpath, the result of `Path::flatten`.

### StrokeStyle, LineCap, LineJoin, FillRule
```rust
pub struct StrokeStyle { pub width: f32 /*1*/, pub cap: LineCap, pub join: LineJoin, pub miter_limit: f32 /*4*/,
                         pub dash: Vec<f32>, pub dash_phase: f32 }          // Default provided
.width(f32) .cap(impl Into<LineCap>) .join(impl Into<LineJoin>) .miter_limit(f32) .dash(impl Into<Vec<f32>>) .dash_phase(f32)
enum LineCap  { Butt (default), Round, Square }
enum LineJoin { Miter (default), Round, Bevel }
enum FillRule { NonZero (default) /* overlaps stay filled */, EvenOdd /* holes */ }
```
Dash lists on, off, on, off lengths (an odd count is used twice over, as in SVG/CSS); empty is solid; at most 100,000 dashes per path. Used by `Shape::stroke_style`, `GraphicsContext::stroke_path`, `Path::stroked`.

### Canvas
```rust
Canvas(draw: impl Fn(&mut GraphicsContext) + 'static) -> View     // also an elg! element
```
Immediate mode: the closure runs each time the canvas is drawn (not when built), with the final frame and window scale. Takes all the space offered; coordinates are points from the top-left. It redraws only when a state it read changes, size/scale/theme changes, it moves by a fraction of a pixel, or every frame if it read `time()` or called `request_frame()`. Plain non-state data read in the closure does not trigger redraws: keep drawn data in state. A panic reports to the app's error handler and leaves that frame's drawing out. Prefer shapes for static boxes/pills (cheaper, accessible layout), `Canvas` for charts, plots, particle effects, custom widgets (combine with `.focusable().on_key(..)` / `.on_text_input(..)`).

### GraphicsContext
`pub struct GraphicsContext<'a>`: valid only during the draw call (passed as `&mut GraphicsContext`).
```rust
.size() -> Size        .scale() -> f32 /* pixels per point */      .time() -> f64 /* seconds, frame clock */
.request_frame()       // draw again next frame (continuous animation)
.fill_rect(Rect, color: impl Into<Color>, radii: impl Into<CornerRadii>)           // radii: a number or CornerRadii; 0.0 for square
.stroke_rect(Rect, color: impl Into<Color>, width: f32, radii: impl Into<CornerRadii>)
.draw_rect(Rect, radii: impl Into<CornerRadii>, fill: impl Into<Color>, border: impl Into<Color>, border_width: f32)  // fill + border in one draw
.fill_path(&Path, style: impl Into<ShapeStyle>, rule: impl Into<FillRule>)
.stroke_path(&Path, color: impl Into<Color>, style: &StrokeStyle)
.push_clip(Rect, radii: impl Into<CornerRadii>)  .pop_clip()                        // clip until the matching pop
```
Constructed only by `Canvas`.
```rust
# let graph = Path::new();
Canvas(move |g| {
    let size = g.size();
    g.fill_rect(Rect::new(0.0, 0.0, size.width, size.height), Color::hex(0x1A1B26), 12.0);
    g.stroke_path(&graph, Color::hex(0x7AA2F7), &StrokeStyle::default().width(2.0).join(LineJoin::Round));
    g.request_frame();   // only if animating, with g.time()
})
.frame(height: 120)
```

### Gradients and GradientStop
```rust
GradientStop::new(color: Color, location: f32 /* 0..1 */) -> GradientStop     // fields color, location (const fn)
LinearGradient::new(stops: Vec<GradientStop>, start: UnitPoint, end: UnitPoint)
LinearGradient::colors(colors: impl IntoIterator<Item = Color>)   // evenly spaced; then .from_to(start: UnitPoint, end: UnitPoint)
RadialGradient::new(stops, center: UnitPoint, start_radius: f32, end_radius: f32)   // radii in points
RadialGradient::colors(colors)       // center, 0 to 100 points
EllipticalGradient::new(stops, center: UnitPoint, start_radius_fraction: f32, end_radius_fraction: f32)  // fractions of the bounds' size; 0.5 reaches mid-edge
EllipticalGradient::colors(colors)   // center, 0 to 0.5
AngularGradient::new(stops, center: UnitPoint, start_angle: f32, end_angle: f32)   // degrees, clockwise; 0 = trailing (right), 90 = down
AngularGradient::colors(colors)      // center, 0..360; then .angles(start: f32, end: f32)
```
Fill anything a color fills (`.fill`, `.background`, `.background_in`, `GraphicsContext::fill_path`, `.mask`). `UnitPoint::new(x, y)` / constants such as `UnitPoint::CENTER`, `UnitPoint::TOP`: (0,0) is top-leading, (1,1) bottom-trailing. Colors interpolate in sRGB with premultiplied alpha (CSS-like).
```rust
Circle().fill(LinearGradient::colors([Color::hex(0xF7768E), Color::hex(0xFF9E64)]))
Rectangle().fill(LinearGradient::new(
    vec![GradientStop::new(Color::BLACK, 0.0), GradientStop::new(Color::CLEAR, 0.6)],
    UnitPoint::new(0.5, 0.0), UnitPoint::new(0.5, 1.0)))
Circle().fill(RadialGradient::new(vec![GradientStop::new(Color::WHITE, 0.0), GradientStop::new(Color::hex(0x7AA2F7), 1.0)],
    UnitPoint::new(0.5, 0.5), 0.0, 60.0))
```

### ShapeStyle
```rust
pub enum ShapeStyle { Color(Color), Linear(LinearGradient), Radial(RadialGradient),
                      Elliptical(EllipticalGradient), Angular(AngularGradient), Backdrop(BackdropFill) }
```
`From` impls exist for `Color`, each gradient and `BackdropFill`; so does `IntoProp<ShapeStyle>` (theme colors like `.accent` too). It is the parameter type of `.fill`, `.background`, `.background_in`, `.mask`. Code that reads a fill's value gets this enum.

### BackdropFill
```rust
BackdropFill::new(blur: f32) -> Self   // what is behind, blurred by `blur` points
.saturation(f32)   // 1 keeps colors
.brightness(f32)   // added per channel: -1 black .. 1 white
.noise(f32)        // 0..1 grain against banding
```
Frosted glass. Normally used through `ViewExt::backdrop_blur(radius)`, `backdrop_blur_with(..)`, `backdrop_blur_in(radius, shape, |o| o.saturation(1.8).noise(0.02))`. It goes behind everything it wraps, so put a tint `.background_in(..)` before it. Inside an offscreen group (opacity < 1, shadow, blur) it only sees what that group drew behind it.
```rust
Text("Now playing").padding(12)
    .background_in(Color::WHITE.opacity(0.5), RoundedRectangle(12))
    .backdrop_blur_in(30, RoundedRectangle(12), |o| o.saturation(1.8).noise(0.02))
```

---

## Images

### Image
`Image(source: impl Into<ImageSource>) -> View` (also `Image("gear")` in `elg!`).

Shows an icon, a raster picture from a file, or encoded data in memory. Natural size = pixels / `scale`. Decoded in the background; the size is known immediately. A file that cannot be read shows nothing and logs a warning. Icons behave like text: they take the surrounding font size, weight and foreground. Raster: `.resizable()` (directly on the image, first) makes it take the offered size; combine with `.aspect_ratio(mode: .fill | .fit)` / `.aspect_ratio_with(|o| o.mode(ContentMode::Fit))`, `.frame(..)`, `.clipped()`, `.clip_shape(Circle())`. `.resizable()` panics on non-raster views.

Formats: PNG, JPEG, GIF (first frame), BMP, TIFF, ICO always; WebP, HEIF, AVIF where the system has codecs. EXIF orientation is honored. SVG pictures: not supported as image files (no SVG source variant in `ImageSource`); for vector art parse path data with `Path::svg` and show it with `PathShape`, or use an icon font set.

```rust
Image("gear")                                           // Phosphor icon (default set)
Image("heart").foreground_style(.danger).font(.title2)
Image("bell").font_weight(.bold)                         // Phosphor bold style
Image("heart.fill")                                      // variant
Image("phosphor.star.duotone")                           // set.icon.variant
Image(ImageSource::file("/path/photo.jpg")).resizable().aspect_ratio(mode: .fill).frame(160, 100).clipped()
Image(ImageSource::file("/path/logo@2x.png").scale(2.0))
Image(asset!("images/logo.png"))
```

### ImageSource
```rust
#[derive(Clone, Debug, PartialEq)]
pub struct ImageSource { pub system_name: String, pub file: PathBuf, pub data: Cow<'static, [u8]>,
                         pub scale: f32 /* 1 */, pub max_pixels: u64 /* 0: 2^27 */, pub max_side: u32 /* 0: 8192 */ }
ImageSource::system(name: impl Into<String>)   // icon name "<set>.<icon>[.<variant>]" or "<icon>[.<variant>]" in the default set
ImageSource::file(path: impl Into<PathBuf>)    // relative paths resolve against the current directory at run time
ImageSource::data(data: impl Into<Cow<'static, [u8]>>)   // encoded bytes (copied when decoded)
.scale(f32)  .max_pixels(u64)  .max_side(u32)
impl From<&str>/From<String> (icon name), From<&Path>/From<PathBuf> (file)
```
`max_side`: longer sides are scaled down while decoding (size in points unchanged; never below 8192 by GPU limit). `max_pixels`: more pixels than this and the image is refused like an unreadable file; lower it for untrusted sources. Prefer `asset!` for images shipped with the app; absolute `file` paths for user-chosen files.

### asset! macro
`asset!("images/logo.png") -> ImageSource` (path relative to `<crate>/assets/`, literal only). Compiles the bytes in with `include_bytes!` so it works in the app, previews and installed copies (unlike relative file paths). Use `Image(asset!("..."))`.

### AsyncImage and AsyncImagePhase
```rust
AsyncImage::new(url: impl Into<String>) -> AsyncImage            // AsyncImage("https://...") in elg!
.disk_cache(bool)        // false: memory only (private images whose server lacks no-store). default true
.scale(f32)              // pixels per point
.max_pixels(u64)  .max_side(u32)
.content_mode(impl Into<ContentMode>)   // resizable, keeps proportions (without a content closure)
.content<V: IntoView>(impl Fn(&AsyncImagePhase) -> V + 'static)   // trailing closure block in elg!
pub enum AsyncImagePhase { Loading, Success(View) /* an Image: .resizable() applies */, Failure(String) }
```
Downloads over HTTP(S) when first shown. Default: progress indicator while loading, then the image, or a placeholder icon with the error as tooltip on failure; without `content_mode` shown at natural size. Cached in memory and on disk (see `set_image_cache`). Fails on: 30 s timeout, over 5 redirects, header over 64 KB, image over 64 MB, https-to-http redirects. Uses the system proxy on macOS/Windows; on Linux only `HTTPS_PROXY`/`http_proxy`/`NO_PROXY` (HTTP proxies only). For anything else (auth, SOCKS) download the bytes yourself and use `Image(ImageSource::data(..))`.
```rust
AsyncImage("https://example.com/avatar.png", content_mode: .fill).frame(48, 48).clip_shape(Circle())
AsyncImage(url, disk_cache: false) { phase in
    { match phase {
        AsyncImagePhase::Loading => Progress::new().into_view(),
        AsyncImagePhase::Success(image) => image.clone().resizable().aspect_ratio_with(|o| o.mode(ContentMode::Fit)),
        AsyncImagePhase::Failure(error) => Text::new(error.clone()).foreground_style(ThemeColor::Danger).into_view(),
    } }
}
```

### ImageCacheOptions, set_image_cache, clear_image_cache
```rust
pub struct ImageCacheOptions { pub disk: bool /* true */, pub disk_limit: u64 /* bytes, 256 MiB */, pub directory: Option<PathBuf> }
pub fn set_image_cache(options: ImageCacheOptions)    // UI thread; running downloads keep the cache they started with
pub fn clear_image_cache()                            // memory and disk (e.g. on sign-out); UI thread
```
Disk cache follows Cache-Control/Expires, revalidates with ETag/Last-Modified, shows stale copies when offline, evicts least recently used. Default folder: `<user cache folder>/Elegance/<exe name>/Images`. `ELG_IMAGE_CACHE=off` disables it regardless of the app.
```rust
set_image_cache(ImageCacheOptions { disk_limit: 64 << 20, ..Default::default() });
set_image_cache(ImageCacheOptions { disk: false, ..Default::default() });
```

### Label (with an icon)
`Label(title: impl IntoProp<String>, icon: impl Into<ImageSource>) -> View`. See the controls section; `Button(Label("Share", "share-network"), share)`.

### Icon sets: IconFont, IconVariant, IconSetOptions, register_icon_set, set_default_icon_set
Phosphor (about 1,500 icons) is registered as the default set `phosphor`; names are Phosphor's (mistyped names list the closest). Thin/light/regular/bold follow the font weight; `fill` and `duotone` are variants.
```rust
pub enum IconFont { None (default), File(PathBuf), Data(Vec<u8>) }
  .is_empty() .file() -> Option<&Path> .data() -> Option<&[u8]>;  From<&str|String|&Path|PathBuf> (file), From<Vec<u8>|&[u8]> (data)
pub struct IconVariant { pub name: String /* no dots, not empty */, pub font: IconFont, pub names: PathBuf, pub names_data: String }
pub struct IconSetOptions { pub font: IconFont /* regular */, pub weights: Vec<(Weight, IconFont)>, pub variants: Vec<IconVariant>,
                            pub names: PathBuf, pub names_data: String, pub replace: bool }   // Default provided
pub fn register_icon_set(name: &str, options: IconSetOptions) -> Result<(), Error>
pub fn set_default_icon_set(name: &str) -> Result<(), Error>
```
The names map is lines of `name codepoint`, or CSS (like Phosphor's / web icon fonts), or `names_data` as text. A set's name must be unique; `replace: true` to re-register a taken name (including `phosphor`). Registration may be called from any thread; a failed registration leaves sets unchanged.
```rust
register_icon_set("brand", IconSetOptions {
    font: IconFont::File("brand-icons.ttf".into()),
    names: "brand-icons.css".into(),
    ..Default::default()
})?;
let _ = elg! { Image("brand.logo") };
set_default_icon_set("brand")?;   // plain names now come from "brand"
```

---

## Media

All handles (`VideoPlayer`, `VideoFeed`, `Camera`, `Sound`, `AudioPlayer`, `AudioStream`, `AudioRecorder`) are cheap `Clone` handles: clones control the same object; playback stops when the last clone and last view are gone. Constructors return `Result<_, Error>`. Reactive reads (`is_playing`, `time`, ...) are for the UI thread and update views that show them. In `elg!` closures that need the handle, clone it first (`let p2 = player.clone();`). Platform back ends: Media Foundation (Windows), AVFoundation (macOS), GStreamer (Linux; needed at Elegance build time, otherwise videos do not play and `error()` says why). MP4/H.264 works everywhere; HEVC/VP9/AV1 where the system plays them. Audio files: WAV, FLAC, MP3.

Naming: the Rust module is `elegance::media` (`audio.rs`, `camera.rs`, `video.rs`; everything re-exported in the prelude). The C++ headers are `elg/audio.h`, `elg/video.h` and `elg/capture.h` (audio recording and camera live in `capture.h`; the guide page is `capture.mdx`, "Microphone and camera"). In Rust `AudioRecorder`, `Camera`, `CameraView` are in the same `media` module as the rest.

Access permission: macOS asks the first time; Windows via Settings > Privacy; Linux cameras usually need the `video` group. When denied, `error()` says so: show it in the UI.

### VideoPlayer
```rust
VideoPlayer::new(source: &str) -> Result<VideoPlayer, Error>                 // file path or http(s) URL
VideoPlayer::file(file: impl AsRef<Path>) -> Result<VideoPlayer, Error>
.play() .pause() .toggle() .stop() /* pause + back to start */ .seek(seconds: f64)
.set_volume(f32 /*0..1, clamped*/) .set_muted(bool) .set_looping(bool)
.is_playing() .is_ready() /* duration+size known */ .is_muted() .is_looping() -> bool
.time() -> f64  .duration() -> f64  .volume() -> f32
.natural_size() -> Size  // pixels; zero until ready
.error() -> Option<String>   // missing file, unsupported format, network
.progress_binding() -> Binding<f32>  // position 0..1, seeks when written: feeds a Slider
.volume_binding() -> Binding<f32>
```
Loads in the background (`is_ready()` starts false). One player may be shown in several windows at once.

### VideoView
```rust
VideoView::new(player: &VideoPlayer) -> Self;   .mode(impl Into<ContentMode>) -> Self    // default Fit
VideoView(&player) / VideoView(player) in elg!
```
Shows the player's frames: fit (default) or `.mode(.fill)` (crops); uncovered area is black; alone it is the video's size. Combine with `.aspect_ratio(16.0 / 9.0, .fit)` and `.clip_shape(..)`.

### VideoControls
`VideoControls(player: &VideoPlayer) -> View` (and `VideoControls(&player)` / `VideoControls(player)` in `elg!`). Ready-made bar: play/pause, scrubber, time, mute; white on a translucent black capsule; the time reads "Can't play this video" on error. Layer it over a `VideoView` in a `ZStack`. Build your own from the readers/bindings instead if you need a custom design.
```rust
let player = VideoPlayer::new("assets/intro.mp4")?;
let _ = elg! {
    ZStack(alignment: .bottom) {
        VideoView(&player).aspect_ratio(16.0 / 9.0, .fit).clip_shape(RoundedRectangle(12))
        VideoControls(&player).padding(10)
    }
};
player.play();
// custom controls
let (p1, p2, p3) = (player.clone(), player.clone(), player.clone());
let _ = elg! { HStack {
    Button(action: move || p1.toggle()) { if p2.is_playing() { Image("pause") } else { Image("play") } }
        .accessibility_label("Play or pause")
    Slider(player.progress_binding())
    Text(move || format!("{:.0} s", p3.time()))
} };
```

### VideoFeed, VideoFormat, VideoFeedView
```rust
VideoFeed::new() -> Result<VideoFeed, Error>          // Send + Sync, PartialEq; push from any thread
.push(format: impl Into<VideoFormat>, width: u32, height: u32, pixels: &[u8], full_range: bool)   // full_range: YUV 0..255 instead of 16..235
.clear()           // black until the next push
.size() -> Size    // reactive, pixels; zero before the first frame
.has_frame() -> bool
pub enum VideoFormat { Bgra (default), Nv12, I420 }   // packed top to bottom, no row padding
VideoFeedView::new(feed: &VideoFeed) -> Self;  .mode(impl Into<ContentMode>);  .mirrored(bool)   // default Fit, not mirrored
```
Use for frames your app produces (own decoder, remote camera). YUV to RGB happens on the GPU. Frames must fit the GPU's largest texture (at least 8192); larger ones are not shown (warning logged once). Use `.mirrored(true)` for self-view of a camera.
```rust
let feed = VideoFeed::new()?;
let _ = elg! { VideoFeedView(&feed).aspect_ratio(16.0 / 9.0, .fit) };
feed.push(VideoFormat::Nv12, 1280, 720, &pixels, false);   // from any thread
```

### Camera, CameraView, CameraDevice, CameraOptions, camera_devices
```rust
pub struct CameraDevice { pub id: String, pub name: String }
pub fn camera_devices() -> Vec<CameraDevice>
pub struct CameraOptions { pub device_id: String /* empty: first camera */, pub width: u32 /*1280*/, pub height: u32 /*720*/, pub mirrored: bool /*true*/ }
Camera::new(options: CameraOptions) -> Result<Camera, Error>     // create on the UI thread
.start() .stop() .is_running() -> bool  .size() -> Size  /* pixels, zero until first frame */
.error() -> Option<String>  .mirrored() -> bool
.snapshot() -> Option<Vec<u8>>        // latest frame as PNG bytes, not mirrored; None before the first frame
CameraView::new(camera: &Camera) -> Self;  .mode(impl Into<ContentMode>)    // default Fill (crops); .fit shows all
```
The camera delivers its closest size to the one requested; virtual cameras work. Mirrored preview by default (turn off in options). Frames go to the GPU without CPU copies.
```rust
let camera = Camera::new(CameraOptions::default())?;
let c = camera.clone();
let _ = elg! { CameraView(&camera).frame(320, 180).clip_shape(RoundedRectangle(12)) };
if let Some(png) = camera.snapshot() { std::fs::write("snapshot.png", &png)?; /* or Image(ImageSource::data(png)) */ }
```

### Sound and PlayOptions
```rust
Sound::load(file: impl AsRef<Path>) -> Result<Sound, Error>       // decoded into memory; Send + Sync
Sound::load_data(data: &[u8]) -> Result<Sound, Error>             // encoded audio in memory (copied)
.play()  .play_with(PlayOptions)  .duration() -> f64
pub struct PlayOptions { pub volume: f32 /*1.0, 0..1 and beyond amplifies*/, pub pitch: f32 /*1.0; speed and pitch, 2 = octave up*/, pub pan: f32 /*0.0; -1 left .. 1 right*/ }
```
For UI clicks and game effects: each `play` starts afresh, so plays overlap. Choose `AudioPlayer` for long music.
```rust
let click = Sound::load("assets/click.wav")?;
let quiet = click.clone();
elg! { HStack {
    Button("Tap", move || click.play())
    Button("Quiet tap", move || quiet.play_with(PlayOptions { volume: 0.4, pitch: 1.2, pan: -0.5 }))
} }
```

### AudioPlayer
```rust
AudioPlayer::new(file: impl AsRef<Path>) -> Result<AudioPlayer, Error>   // streams from the file: long files start at once, low memory
.play() .pause() .toggle() .stop() /* also rewinds */ .seek(seconds: f64) .set_volume(f32) .set_looping(bool)
.is_playing() .is_looping() -> bool   .time() -> f64  .duration() -> f64  .volume() -> f32
.time_binding() -> Binding<f32>    // position in seconds; seeks when written; use Slider(music.time_binding(), max: music.duration() as f32)
.volume_binding() -> Binding<f32>
```
```rust
let music = AudioPlayer::new("assets/theme.mp3")?;
let (m1, m2, m3) = (music.clone(), music.clone(), music.clone());
let _ = elg! { VStack {
    Button(Text(move || if m1.is_playing() { "Pause" } else { "Play" }), move || m2.toggle())
    Slider(music.time_binding(), max: music.duration() as f32)
    Slider(music.volume_binding())
    Text(move || format!("{:.0} / {:.0} s", m3.time(), m3.duration()))
} };
```

### AudioStream and AudioStreamOptions
```rust
pub struct AudioStreamOptions { pub device_id: String /* empty: default output, followed on change */, pub sample_rate: u32 /*48000*/, pub channels: u32 /*2*/ }
AudioStream::new(options: AudioStreamOptions) -> Result<AudioStream, Error>               // push mode; Send + Sync
AudioStream::with_render(options, render: impl Fn(&mut [f32]) + Send + 'static) -> Result<AudioStream, Error>   // pull mode, audio thread
.write(samples: &[f32])   // interleaved, -1..1, any thread; over about a second queued: oldest dropped
.clear() .start() .stop() .queued_frames() -> usize .set_volume(f32)
.error() -> Option<String> .sample_rate() -> u32 .channels() -> u32
```
Starts playing as soon as it is made. For audio your app produces (calls, synths, decoders). The render closure is `Fn`: keep state in a `Mutex`/atomics; keep it short. A panic silences the stream and the panic reaches the app's error handler. If a given `device_id` is disconnected the stream plays nowhere and `error()` says so.
```rust
let out = AudioStream::new(AudioStreamOptions { channels: 2, ..Default::default() })?;
out.write(decoded);
let phase = std::sync::Mutex::new(0.0_f32);
let tone = AudioStream::with_render(AudioStreamOptions { channels: 1, ..Default::default() }, move |buffer| {
    let mut phase = phase.lock().unwrap();
    for s in buffer.iter_mut() { *s = 0.2 * phase.sin(); *phase += 2.0 * std::f32::consts::PI * 440.0 / 48000.0; }
})?;
```

### AudioRecorder and AudioRecorderOptions
```rust
pub struct AudioRecorderOptions { pub device_id: String /* empty: default input */, pub sample_rate: u32 /*48000*/, pub channels: u32 /*1*/,
                                  pub keeps_samples: bool /*true; false: only on_samples sees audio*/ }
AudioRecorder::new(options) -> Result<AudioRecorder, Error>        // UI thread
AudioRecorder::with_samples(options, on_samples: impl Fn(&[f32]) + Send + 'static) -> Result<AudioRecorder, Error>  // audio thread: keep short, never touch views/state
.start() /* continues */ .stop() /* pauses */ .toggle() .clear() /* drops the recording */
.is_recording() -> bool  .duration() -> f64  .level() -> f32 /* recent peak 0..1 */  .error() -> Option<String>
.sample_rate() -> u32 .channels() -> u32
.samples() -> Vec<f32>   // interleaved (L R L R ...)
.save_wav(file: impl AsRef<Path>) -> Result<(), Error>  .wav() -> Vec<u8>  .sound() -> Result<Sound, Error>
```
```rust
let recorder = AudioRecorder::new(AudioRecorderOptions::default())?;
let (r1, r2, r3, r4) = (recorder.clone(), recorder.clone(), recorder.clone(), recorder.clone());
let _ = elg! { VStack {
    Button(Text(move || if r1.is_recording() { "Stop" } else { "Record" }), move || r2.toggle())
    Progress(move || Some(r3.level()))
    Text(move || format!("{:.1} s", r4.duration()))
} };
recorder.save_wav("memo.wav")?;
let streaming = AudioRecorder::with_samples(AudioRecorderOptions { keeps_samples: false, ..Default::default() },
    move |block| { let _ = tx.send(block.to_vec()); })?;
```

### AudioDevice, audio_input_devices, audio_output_devices
```rust
pub struct AudioDevice { pub id: String, pub name: String, pub is_default: bool }
pub fn audio_output_devices() -> Vec<AudioDevice>
pub fn audio_input_devices() -> Vec<AudioDevice>
```
`id` is what `AudioStreamOptions::device_id` / `AudioRecorderOptions::device_id` take. Sound output follows the system default output and switches when it changes (headphones). Without an output device playing does nothing and logs once.

---

## GPU images

For pictures that other GPU code (a `wgpu` renderer) produces and Elegance shows without CPU pixel copies. `GpuImage`, `GpuImageOrigin`, `GpuImageView` are in the prelude and re-exported by the `elegance-wgpu` crate.

### GpuImage
```rust
#[derive(Clone)] pub struct GpuImage   // Send + Sync; clones share one native handle
GpuImage::new(width: u32, height: u32, origin: GpuImageOrigin) -> Result<GpuImage, Error>   // initial natural size, both nonzero; no renderer needed; does not publish a frame
.flipped() -> bool            // created with GpuImageOrigin::BottomLeft
.clear()                      // removes the current frame; not a GPU drain
.as_raw() -> *mut elegance_sys::gpu_image::GpuImage    // borrowed native handle for unsafe interop; keep an owning clone alive; do not destroy
```
Accepted frames may change size (any nonzero dimensions); the natural size updates asynchronously on the UI thread. There is no safe generic publish in `elegance`: publishing goes through the unsafe `elegance_sys::gpu_image` API, or the safe `elegance-wgpu` `Bridge`. Stop/join producers and remove views before renderer shutdown.

### GpuImageOrigin
`enum GpuImageOrigin { TopLeft (default), BottomLeft }`: where the producer's first row goes when displayed.

### GpuImageView
```rust
GpuImageView::new(image: &GpuImage) -> Self;   .mode(impl Into<ContentMode>) -> Self    // default Fit
GpuImageView(&image) / GpuImageView(image), then optional mode: .fit | .fill, in elg!
```
Shows the image fitted into (or filling, clipped to) the offered space, on black. Hold the `GpuImage` somewhere long-lived (the view retains it independently).
```rust
let image = GpuImage::new(640, 480, GpuImageOrigin::TopLeft)?;
elg! { GpuImageView(&image, mode: .fit) }
```

### elegance-wgpu crate (separate crate `elegance_wgpu`)
Bridges native `wgpu` 30.0.1 (pinned exactly `=30.0.1`; use `elegance_wgpu::wgpu` or depend on `wgpu = "=30.0.1"` so Cargo unifies; Rust 1.93+) to `GpuImage`. No CPU pixel copy; unsupported paths fail with an error, with no fallback. Same physical GPU must be used on both sides (no adapter negotiation).

| Platform | Path | Status per README |
|---|---|---|
| macOS | wgpu Metal to Dawn Metal (IOSurface + shared events) | implemented, tested on real GPU |
| Windows | wgpu DX12 to Dawn D3D12 (shared DXGI textures + fences) | implemented, only cross-check validated |
| Linux | wgpu Vulkan to Dawn Vulkan (opaque-FD memory + binary semaphores) | implemented, only cross-check validated |

Not implemented: D3D11, OpenGL/GLES, cross-adapter sharing, DMA-BUF, YUV/multi-plane, device-loss recovery. Other OS targets get a placeholder `Bridge` whose constructors return `Error::Unsupported`.

Public API (from `elegance-wgpu/src`):
```rust
pub use elegance::{GpuImage, GpuImageOrigin, GpuImageView};  pub use wgpu;
pub use render::{RenderAttachments, RenderFrame, RenderPassOptions};
#[derive(Clone, Copy, Debug)]
pub struct Config { pub width: u32, pub height: u32, pub origin: GpuImageOrigin,
                    pub format: wgpu::TextureFormat /* Bgra8Unorm or Rgba8Unorm; RGB bytes sRGB-encoded, straight alpha */,
                    pub max_frames: usize /* displayed + in-flight + retiring + quarantined */ }
pub enum Error { Unsupported, InvalidConfig, Busy, Device(String), Allocation, Publish(u32) }   // Display + std::error::Error
pub struct Bridge   // per-platform type
impl Bridge {
    pub async fn new(adapter: &wgpu::Adapter, config: Config) -> Result<Self, Error>        // own device + private queue on the adapter
    pub unsafe fn from_device(device: wgpu::Device, queue: wgpu::Queue, config: Config) -> Result<Self, Error>  // attach an existing renderer; strict external-sync contract; Linux needs sharing extensions and a reserved second queue
    pub fn device(&self) -> &wgpu::Device
    pub fn image(&self) -> &GpuImage                 // cloneable UI source: GpuImageView::new(bridge.image())
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), Error>
    pub fn clear(&self)
    pub fn in_flight(&self) -> usize   pub fn quarantined(&self) -> usize
    pub fn copy_from(&mut self, source: &wgpu::Texture) -> Result<(), Error>     // GPU copy; source: same device, same size/format, single-sample, COPY_SRC; submit its writes first
    pub fn clear_color(&mut self, color: wgpu::Color) -> Result<(), Error>
    pub fn render(&mut self, encode: impl FnOnce(&mut RenderFrame<'_>)) -> Result<(), Error>   // safe, restricted frame API; records into the shareable frame, no copy
    pub unsafe fn render_raw(&mut self, encode: impl FnOnce(&wgpu::Device, &mut wgpu::CommandEncoder, &wgpu::Texture)) -> Result<(), Error>  // raw access; strict safety contract (README)
}
// RenderFrame<'a>: .size() -> wgpu::Extent3d
//   .write_buffer(&mut self, buffer: &wgpu::Buffer, offset: u64, data: &[u8])
//   .render_pass_with(&mut self, attachments: &mut RenderAttachments, clear: wgpu::Color, encode: impl FnOnce(&mut wgpu::RenderPass<'_>))
//   .render_pass(&mut self, options: RenderPassOptions<'_>, encode: impl FnOnce(&mut wgpu::RenderPass<'_>))
// RenderAttachments::new(sample_count: u32, depth_format: Option<wgpu::TextureFormat>)   // caches MSAA/depth attachments
// RenderPassOptions<'a> (Default): pub label: Option<&'a str>, pub clear: wgpu::Color, pub multisample: Option<&'a wgpu::TextureView>,
//                                  pub depth_stencil: Option<wgpu::RenderPassDepthStencilAttachment<'a>>
```
`render` was read from `elegance-wgpu/src/render.rs` (the README's "unsafe render" wording is stale: `render` is a safe method; `render_raw` is the unsafe one).
Rules: a full budget returns `Error::Busy` (retry later, keep showing the last frame; retained UI snapshots can occupy every slot). Frames are freshly allocated and freed on completion (no persistent pool yet). Stop/join producers, `clear()` and remove views before renderer shutdown; `Drop` is not a GPU drain. Presentation stays Elegance's FIFO/VSync path; publication is not display-paced. A runnable app is `wgpu-direct-rs` in elegance-examples (not available locally).
```rust
use elegance_wgpu::{wgpu, Bridge, Config, GpuImageOrigin};
let bridge = Bridge::new(&adapter, Config { width: 640, height: 480, origin: GpuImageOrigin::TopLeft,
    format: wgpu::TextureFormat::Bgra8Unorm, max_frames: 3 }).await?;
bridge.clear_color(wgpu::Color::RED)?;                   // after Elegance's render device is initialized
let view = elg! { GpuImageView(bridge.image(), mode: .fit) };   // bridge.image() is &GpuImage
```
(`GpuImageView` has an `ElgNew<(&GpuImage,)>` overload, so passing the reference is fine.)

---

## C++-only / not exposed in Rust

- No Rust `Shape` trait (C++ `shapes.h` has `class Shape : View`); Rust has the concrete `Shape` struct only.
- `ImageSource` has no SVG option in either language doc surface; SVG support is limited to path data (`Path::svg`).
- C++ aggregate-initialized options (`{.on_submit = ..}`, `{.min = 8}`) become named args in `elg!` (`on_submit: ..`, `min: 8`) or builder methods in Rust.
- C++ `Image({.system_name = "microphone"})` is `Image("microphone")` in Rust.
- C++ `AudioStreamOptions.on_render` and `AudioRecorderOptions.on_samples` are the separate constructors `AudioStream::with_render` and `AudioRecorder::with_samples` in Rust.
- C++ `ContentMode::fill` etc. are `.fill` / `ContentMode::Fill`.
- Native `elg::GpuImageView` and the C `gpu_image.h` API (`elg_gpu_image_publish`) are reachable in Rust only through `elegance_sys::gpu_image` (unsafe) or `elegance-wgpu`.
- Selection is `Selection` trait + `.selection(..)`; there is no list/`Selection` control view.
- Mikros: nothing in these modules mentions it; platform back ends listed above are Windows/macOS/Linux only.

---

## Part 3: Floating UI, app and windows, desktop, events, accessibility

# Elegance (Rust): popups, app and windows, desktop, input, accessibility

Reference for floating UI (popover, tooltip, menus), `App` and windows, desktop integration, drag and drop, accessibility, and input/event/gesture types. All names are from elegance-rs (`elegance` crate); everything below is re-exported at the crate root and in `elegance::prelude` (via `items`) unless noted.

## Contents
- [Floating UI](#floating-ui): popover, floating, tooltip, context menu, Menu/MenuButton/MenuItem/Submenu
- [App and windows](#app-and-windows)
- [Desktop integration](#desktop-integration)
- [Clipboard and drag and drop](#clipboard-and-drag-and-drop)
- [Accessibility](#accessibility)
- [Events and input types](#events-and-input-types)
- [C++-only / not exposed in Rust](#c-only--not-exposed-in-rust)
- [Doc-vs-code notes](#doc-vs-code-notes)

---

## Floating UI

Choosing:

| Need | Use |
|---|---|
| Short label on hover | `.tooltip("text")` (or `.tooltip_view` for rich content; screen readers do not see tooltip views) |
| Panel of arbitrary content beside a view, bound to a bool | `.popover(binding, content)` |
| Positioned content with no panel/shadow, custom dismissal | `.floating(binding, content, opts)` |
| List of commands | `Menu { MenuItem ... }` in `MenuButton` (click) or `.context_menu` (right-click) |
| Choose one value from a list | `Picker` with `MenuPickerStyle` (see controls reference) |
| Separate OS-level surface with its own title/size | a window: `App::window` (windows cannot be opened after `run()` starts, see below) |

All popup builders (`content`, `menu`) are `Fn() -> V` closures re-run each time the popup opens, so they reflect current state. Popovers, tooltips and menus are drawn by the environment's `PopoverStyle`, `TooltipStyle`, `MenuStyle` (`.popover_style(..)`, `.tooltip_style(..)`, `.menu_style(..)`; see styles reference).

### popover

```rust
fn popover<V: IntoView>(self, is_presented: impl IntoBinding<bool>, content: impl Fn() -> V + 'static) -> View
fn popover_with<V: IntoView>(self, is_presented: impl IntoBinding<bool>, content: impl Fn() -> V + 'static,
    options: impl FnOnce(PopoverOptions) -> PopoverOptions) -> View
```
A styled panel beside the view while the binding is true. Pass a `State<bool>` (dismissal writes false back) or a closure/`Computed` (shown while the condition holds; dismissing closes it until the condition next becomes true). Click outside, Esc, or switching apps dismisses it. Default placement: centered below, spacing 4, flips when there is no room; content is padded.
Prefer it over `floating` when you want the standard panel chrome; over a window for transient, anchored UI. In `elg!`, trailing `key: v` args go to `popover_with`.

```rust
let editing = State::new(false);
elg! {
    Button("Edit…", move || editing.set(true)).popover(editing, move || elg! {
        VStack {
            TextField("Name", name)
            Button("Done", move || editing.set(false))
        }
        .frame(width: 240)
    })
}
// options: .popover(editing, content, edge: .trailing, padded: false, motion: grow.clone())
```
A `MenuButton` inside a popover stacks over it; choosing an item closes both.

### PopoverOptions

```rust
pub struct PopoverOptions { pub placement: FloatingPlacement, pub boundary: FloatingBoundary,
    pub padded: Prop<bool>, pub motion: Option<PopupMotion> }
fn placement(self, impl FnOnce(FloatingPlacement) -> FloatingPlacement) -> Self
fn edge(self, edge: impl Into<FloatingEdge>) -> Self
fn boundary(self, boundary: impl Into<FloatingBoundary>) -> Self
fn padded(self, padded: impl IntoProp<bool>) -> Self   // default true; false: content reaches panel edges
fn motion(self, motion: PopupMotion) -> Self           // None: the PopoverStyle's
```
Defaults: placement alignment `Center`, spacing 4.0; boundary `Screen`.

### floating

```rust
fn floating<V: IntoView>(self, is_presented: impl IntoBinding<bool>, content: impl Fn() -> V + 'static,
    options: impl FnOnce(FloatingOptions) -> FloatingOptions) -> View
fn floating_container(self) -> View
```
Content above everything by the view while the binding is true, no panel or shadow. The options closure is a required 3rd argument (pass `|o| o` for defaults). Size follows content; it follows the view when the view moves. When dismissed (Esc / outside press) the binding is set false (even for a constant binding); it reopens only after the binding has been false then true.
`floating_container()` marks the view that `FloatingBoundary::Container` content inside it stays within (map callouts, content in a scroll view). Without a marked container, `Container` acts like `Window`.

```rust
Text("Avatar").floating(card_shown, || elg! { Text("Profile card") }, |o| {
    o.placement(|p| p.edge(FloatingEdge::Trailing).alignment(FloatingAlignment::Center).spacing(8))
        .boundary(FloatingBoundary::Window)
        .outside_click(false)
})
```

### FloatingOptions

```rust
pub struct FloatingOptions { pub placement: FloatingPlacement, pub boundary: FloatingBoundary,
    pub dismiss: FloatingDismiss, pub motion: Option<PopupMotion> }   // Default: Bottom/Start, Screen, dismiss both true, no motion
fn placement(self, impl FnOnce(FloatingPlacement) -> FloatingPlacement) -> Self
fn boundary(self, boundary: impl Into<FloatingBoundary>) -> Self
fn outside_click(self, closes: bool) -> Self
fn escape(self, closes: bool) -> Self
fn motion(self, motion: PopupMotion) -> Self
```

### FloatingDismiss

```rust
pub struct FloatingDismiss { pub outside_click: bool, pub escape: bool }   // Default: both true
```
`outside_click: true`: a press outside closes it and, while open, presses elsewhere only close it (the rest of the window is blocked). `false`: the window stays usable; it closes when the binding goes false or on Esc. `escape`: Esc closes unless something focused handles it first. Set through `FloatingOptions::outside_click` / `escape` builders.

### FloatingPlacement

```rust
pub struct FloatingPlacement { pub edge: FloatingEdge, pub alignment: FloatingAlignment,
    pub spacing: f32, pub offset: Point, pub position: Option<Point> }
fn edge(self, impl Into<FloatingEdge>) -> Self
fn alignment(self, impl Into<FloatingAlignment>) -> Self
fn spacing(self, impl Number) -> Self      // negative: overlapping
fn offset(self, impl Into<Point>) -> Self  // applied after placement
fn position(self, impl Into<Point>) -> Self // explicit: content's top-leading corner in the anchor's coordinates, instead of an edge
```
Used inside the `placement(|p| ...)` closures. `Default` is Bottom / Start / 0 spacing; popover and tooltip options start with spacing 4.

### FloatingEdge

`enum FloatingEdge { Bottom (default), Top, Trailing, Leading }`. The side of the anchor it opens on; flips to the opposite side when there is no room. In `elg!`: `.bottom`, `.top`, `.trailing`, `.leading`.

### FloatingAlignment

`enum FloatingAlignment { Start (default), Center, End }`. Along the chosen edge: lined up with anchor start (leading/top), center, or end.

### FloatingBoundary

`enum FloatingBoundary { Screen (default), Window, Container }`.
- `Screen`: own popup window; may extend past the app window; kept on the display. On Wayland the compositor may move/limit popup windows: use `Window` where exact placement matters.
- `Window`: drawn inside the app's window above content, kept within it.
- `Container`: inside the nearest `.floating_container()` view (else window): clipped, under outside views, moves with it when it scrolls; Esc and outside presses still close it from anywhere.
Accepted by `popover`, `tooltip`, `floating`.

### PopupMotion

```rust
fn PopupMotion::new(transition: impl Into<Transition>, animation: impl Into<Animation>) -> Self
fn PopupMotion::none() -> Self                    // at once, no animation (built-in styles' default)
fn transition(&self) -> Option<&Transition>
fn animation(&self) -> Option<&Animation>
fn animates(&self) -> bool
```
`Clone`. Opening/closing transition run with an animation; plays both ways and a popup stays on screen (not taking input) until animated out. Pass as `.motion(..)` option of `floating`, `popover`, `tooltip`, or `MenuButton`; without one, the style's `motion()` applies.

```rust
let grow = PopupMotion::new(
    Transition::opacity().combined(Transition::scale_anchored(0.95, Alignment::TOP)),
    Animation::ease_out(0.15),
);
elg! { Button("Details", move || showing.set(true)).popover(showing, details, motion: grow.clone()) }
```

### AnchorOptions

```rust
pub struct AnchorOptions { pub anchor: UnitPoint }   // Default: center
fn anchor(self, anchor: impl Into<UnitPoint>) -> Self
```
Options for rotation/scale effects (pivot point of the view's bounds), not popup placement despite the name; documented with the effect modifiers in modifiers.md. Listed here only because it is in the floating/options scope.

### tooltip

```rust
fn tooltip(self, text: impl IntoProp<String>) -> View
fn tooltip_with(self, text: impl IntoProp<String>, options: impl FnOnce(TooltipOptions) -> TooltipOptions) -> View
fn tooltip_view<V: IntoView>(self, content: impl Fn() -> V + 'static,
    options: impl FnOnce(TooltipOptions) -> TooltipOptions) -> View
```
Appears after the pointer rests on the view (`delay`, 0.6 s default) and goes when it leaves or presses. `tooltip_view` takes any content (the options closure is required; pass `|o| o`); screen readers do not see it, so give icon-only controls an `accessibility_label` as well. In-window tooltips let the pointer through. Static or reactive text.

```rust
Button(Image("gear"), settings).tooltip("Settings")
Text("Avatar").tooltip_view(|| elg! { HStack { Image("user") Text("Ada") } },
    |o| o.placement(|p| p.edge(FloatingEdge::Trailing).spacing(6)).delay(0.3))
Text("x").tooltip("Hi", delay: 0.3)          // elg! form of tooltip_with
```

### TooltipOptions

```rust
pub struct TooltipOptions { pub placement: FloatingPlacement, pub boundary: FloatingBoundary,
    pub delay: f64, pub motion: Option<PopupMotion> }
fn placement(self, impl FnOnce(FloatingPlacement) -> FloatingPlacement) -> Self
fn boundary(self, boundary: impl Into<FloatingBoundary>) -> Self
fn delay(self, seconds: f64) -> Self
fn motion(self, motion: PopupMotion) -> Self
```
Defaults: below the view, spacing 4.0, `Screen`, delay 0.6, motion from `TooltipStyle`.

### context_menu

```rust
fn context_menu<V: IntoView>(self, menu: impl Fn() -> V + 'static) -> View
```
Opens the menu the closure builds (usually `Menu { ... }`) on right-click, or Shift+F10 / the Menu key while the view has focus. Rebuilt each time it opens. Disabled views do not open it.

```rust
Text("notes.txt").context_menu(move || elg! {
    Menu {
        MenuItem("Open", open)
        MenuItem("Pin", toggle_pin, checked: pinned)
        Separator()
        MenuItem("Delete", remove, destructive: true)
    }
})
```

### Menu

```rust
pub struct Menu;  Menu::new() -> Menu;  fn item(self, item: impl IntoView) -> Self
```
The menu panel: `MenuItem`, `Submenu`, `Separator`, or any view as children (`Menu { ... }` in `elg!`). Choosing an item closes the menu, then runs its action. Keyboard: Up/Down move, Enter chooses, Esc closes, Right/Left enter/leave submenus.

### MenuItem

```rust
fn MenuItem::new(title: impl IntoProp<String>, action: impl FnMut() + 'static) -> MenuItem
fn MenuItem::without_action(title: impl IntoProp<String>) -> MenuItem   // choosing only closes the menus
fn shortcut(self, shortcut: impl IntoProp<String>) -> Self   // a text hint on the right, e.g. "Ctrl+R" (not a KeyboardShortcut, does not bind a key)
fn destructive(self, destructive: impl IntoProp<bool>) -> Self   // danger color
fn checked(self, checked: impl IntoProp<bool>) -> Self           // check mark; reactive
fn image(self, image: impl Into<ImageSource>) -> Self            // icon in front (elg!: image: "copy")
```
`MenuItem("Title", action, key: v)` in `elg!`. Disable with `.disabled(..)`. Menu-bar style real shortcuts use `KeyboardShortcut` on `MenuBarItem` (see App section), not `MenuItem`.

### Submenu

```rust
fn Submenu<V: IntoView>(title: impl IntoProp<String>, menu: impl Fn() -> V + 'static) -> View
```
Item that opens another `Menu` beside it (hover, click, or Right). The builder runs each time it opens.

### MenuButton

```rust
fn MenuButton::new<V: IntoView>(label: impl IntoLabel, menu: impl Fn() -> V + 'static) -> MenuButton
fn title(self, title: impl IntoProp<String>) -> Self
fn placement(self, impl FnOnce(FloatingPlacement) -> FloatingPlacement) -> Self
fn edge(self, edge: impl Into<FloatingEdge>) -> Self      // .top near the window bottom
fn shows_indicator(self, shows: impl IntoProp<bool>) -> Self   // the chevron, default true
fn motion(self, motion: PopupMotion) -> Self
```
A button that opens the menu below itself (spacing 4, flips when no room). Prefer it for a toolbar/overflow menu; use `.context_menu` for right-click.

```rust
MenuButton(Label("Sort", "sort-ascending"), move || elg! {
    Menu {
        MenuItem("Name", move || sort.set(Sort::Name), checked: move || sort.get() == Sort::Name)
        Submenu("Move to", move || elg! { Menu { MenuItem("Archive", archive) } })
    }
})
MenuButton("Attach", attach_menu, edge: .top, shows_indicator: false)
```

### KeyboardShortcut (in menus)

See [KeyboardShortcut](#keyboardshortcut) in App and windows. In-window `MenuItem` takes only a text hint; `KeyboardShortcut` is used by `MenuBarItem::shortcut`.

---

## App and windows

Guide: windows.mdx, desktop.mdx (menu bar). Window content is built once, when the window opens.

### App

```rust
App::new() -> App                       // panics on ABI mismatch or startup failure; Default impl exists
fn window(&self, desc: WindowDesc, content: impl IntoView + 'static) -> &Self
fn menu_bar(&self, menus: Vec<MenuBarItem>) -> &Self
fn on_error(&self, handler: impl FnMut(&AppError) -> ErrorAction + 'static) -> &Self
fn run(&self) -> Result<ExitCode, Error>
fn run_code(&self) -> Result<i32, Error>
fn quit(&self)
fn handle(&self) -> AppHandle
```
Builder methods take `&self` and return `&Self`, so chain them or call separately. There is no `tray` method (use `TrayIcon::new`) and no method to open windows at runtime.
- `window(desc, content)`: `content` is a component fn or any view; turned into the root view once on the UI thread when the window opens. Windows open when `run()` starts.
- `menu_bar(menus)`: macOS: top-of-screen menu bar (without one, a minimal app menu). Windows: a bar in each window with a system title bar. Linux: bar drawn at the top of each window. Shortcuts work while a window of the app has the keyboard and no view took the key. Callable while the app runs: replaces the menus.
- `on_error(handler)`: hears each panic/error escaping app code (bodies, effects, handlers, tasks, timers) on the UI thread between events, after logging. No handler: app continues. Return `ErrorAction::Resume` to go on or `ErrorAction::Quit` to end the run (`run` then resumes the panic or returns the error). Callable before or during `run`. (Types `AppError`, `ErrorAction` detailed in the errors reference.)
- `run()`: call on the main thread, one app at a time. Returns when the last window closes or `quit()` is called. `run_code()` returns the exit code as `i32`; `run()` maps it to `ExitCode` (clamped 0..=255). On Windows `run()` sets up OLE on the main thread (needed for drag and drop, clipboard, file dialogs).
- `quit()`: makes `run` return after the current event; UI thread only. From inside closures use an `AppHandle` (closures cannot borrow `App`).

```rust
fn main() -> Result<std::process::ExitCode, Error> {
    let app = App::new();
    app.on_error(|e| { eprintln!("{}", e.message()); ErrorAction::Resume });
    app.window(WindowDesc::new("Editor").size(1200, 800), Editor)
        .window(WindowDesc::new("Palette").size(300, 500), Palette);
    app.run()
}
```

Lifecycle:
- Multiple windows: call `window` once per window before `run`; each gets its own content. A view finds its window through `Environment::<WindowKey>::new()` (`WindowKey` is in the prelude).
- Closing: the user closing a window, or `WindowHandle::close()`, closes it; the app ends when the last window closes (or on `quit()`/`MenuRole::Quit`).
- Opening windows at runtime: not supported by the underlying library (C++ `App::window` throws `UsageError` while running). The Rust `window` doc says only "opens a window when the app runs"; I found no runtime-open API in elegance-rs. Use popovers/floating content for transient UI, or open all windows up front.
- Lifetime hooks: `.on_appear(..)` / `.on_disappear(..)` fire when a window's content appears / the window closes.
- Work on other threads: use `post_to_ui_thread`.

### AppHandle

```rust
#[derive(Clone)] pub struct AppHandle;   fn quit(&self)
```
From `App::handle()`. Holds a weak reference; quits the app if it still exists. Safe to move into closures.

```rust
let quit = app.handle();
Button("Quit", move || quit.quit())
```

### post_to_ui_thread

```rust
pub fn post_to_ui_thread(task: impl FnOnce() + Send + 'static)
```
Runs `task` on the UI thread soon, between events, where state may be written. Callable from any thread; tasks run in posting order. Use it to hand results of your own threads back to state.

### version

`pub fn version() -> &'static str`: Elegance's version, e.g. `"0.1.0"`.

### WindowDesc

```rust
pub struct WindowDesc { pub title: String, pub width: i32, pub height: i32, pub resizable: bool,
    pub min_size: Option<Size>, pub background: Option<Color>, pub title_bar: TitleBar,
    pub system_controls_position: Option<Point>, pub transparent: bool,
    pub material: WindowMaterial, pub opacity: f32, pub fullscreen: bool }
fn WindowDesc::new(title: impl Into<String>) -> Self
fn size(self, width: i32, height: i32) -> Self
fn resizable(self, resizable: bool) -> Self
fn min_size(self, width: f32, height: f32) -> Self
fn background(self, color: impl Into<Color>) -> Self
fn title_bar(self, title_bar: impl Into<TitleBar>) -> Self
fn system_controls_position(self, x: f32, y: f32) -> Self
fn transparent(self, transparent: bool) -> Self
fn material(self, material: impl Into<WindowMaterial>) -> Self
fn opacity(self, opacity: f32) -> Self
fn fullscreen(self, fullscreen: bool) -> Self
```
Defaults: title `"Elegance"`, 1280x720 points, resizable, no min size, background = theme background (clear when transparent / with a material where the platform draws it), `TitleBar::System`, not transparent, `WindowMaterial::None`, opacity 1.0, not fullscreen. Windows follow each display's scale; resizing is live.
- `system_controls_position`: with `TitleBar::Integrated`, first control's top-left in points from the window's top-left. Only macOS draws controls over content, so only macOS uses it.
- `transparent`: desktop shows through where content is not opaque (use with `TitleBar::Hidden` for shaped windows). Needs a compositing desktop; otherwise stays opaque.
- `fullscreen`: covers the display it opens on without a frame; width/height are its size after.

```rust
let desc = WindowDesc::new("Widget").size(340, 190).title_bar(TitleBar::Hidden).transparent(true);
App::new().window(desc, Widget).run()
```

### TitleBar

`enum TitleBar { System (default), Hidden, Integrated }`
- `System`: platform title bar and frame.
- `Hidden`: no title bar; content fills the window; draw your own with `.window_drag_area(true)` and `WindowControls`; edges still resize.
- `Integrated`: content fills the window and the system's own controls stay over it where the platform has them (macOS traffic lights; movable via `system_controls_position`). Elsewhere it behaves as `System`. Keep content clear using `WindowHandle::system_controls_frame()`.

`.window_drag_area(self, enabled: bool) -> View` (view modifier): pressing and dragging there moves the window; double-click maximizes.

```rust
elg! {
    VStack(spacing: 0) {
        HStack { Text("Notes") Spacer() { WindowControls::new() } }
            .frame(height: 36)
            .window_drag_area(true)
        Text("Content")
    }
}
// App::new().window(WindowDesc::new("Notes").title_bar(TitleBar::Hidden), Notes).run()
```

### WindowMaterial

`enum WindowMaterial { None (default), Sidebar, UnderWindow, Mica, Acrylic }`. System translucent backdrop behind content; shows through where content/background are not opaque.

| | macOS | Windows 11 (22H2+) |
|---|---|---|
| `Sidebar` | sidebar vibrancy | Mica Alt |
| `UnderWindow` | under-window | Mica |
| `Mica` | under-window | Mica |
| `Acrylic` | HUD window | Acrylic |

Windows 10 and Linux: the window stays opaque and draws its background as usual.

### WindowHandle

```rust
pub struct WindowHandle   // Clone, PartialEq, Eq, Default, Debug
fn WindowHandle::none() -> Self;  fn is_none(&self) -> bool
fn close(&self);  fn minimize(&self);  fn toggle_maximized(&self)
fn is_maximized(&self) -> bool;  fn is_active(&self) -> bool
fn title_bar(&self) -> TitleBar
fn system_controls_frame(&self) -> Rect
```
The window a view is in, from `Environment::<WindowKey>::new()`. Readers (`is_maximized`, `is_active`, `system_controls_frame`) are tracked reads, so reactive closures that call them rerun on change. Methods do nothing on the empty handle (default / outside any window / window closed). `close()` is like the system close button (app ends with the last window). `is_active`: window has keyboard focus. `system_controls_frame`: empty `Rect` where the system draws no controls over content. UI thread only. Clone the environment handle for each closure.

```rust
let window = Environment::<WindowKey>::new();
let (w1, w2) = (window.clone(), window.clone());
elg! { VStack {
    Text("Title").opacity(move || if window.get().is_active() { 1.0 } else { 0.6 })
    Button("Close", move || w1.get().close())
    Text(move || if w2.get().is_maximized() { "Maximized" } else { "Normal" })
} }
```

### WindowControls

```rust
WindowControls::new() -> Self   // also WindowControls() in elg!
fn style(self, style: impl Into<WindowControlsStyle>) -> Self
fn minimize(self, shown: bool) -> Self
fn maximize(self, shown: bool) -> Self
```
Minimize/maximize/close buttons for a `TitleBar::Hidden` window. With `Automatic` style it draws nothing where the system draws its own controls (`System`, `Integrated`), so one layout works everywhere. It is a view (`IntoView`); in `elg!` write `{ WindowControls::new() }` or `WindowControls()`.

### WindowControlsStyle

`enum WindowControlsStyle { Automatic (default), Caption, TrafficLights }`. `Automatic`: caption buttons on Windows and Linux, traffic lights on macOS. `Caption`: min/max/close as on Windows. `TrafficLights`: three colored circles as on macOS.

### MenuBarItem

```rust
pub struct MenuBarItem { pub title: String, pub action: Option<Box<dyn FnMut()>>, pub checked: Option<bool>,
    pub toggled: Option<Box<dyn Fn(bool)>>, pub enabled: bool, pub shortcut: Option<KeyboardShortcut>,
    pub role: MenuRole, pub submenu: Vec<MenuBarItem> }   // Default: enabled true, rest empty
fn MenuBarItem::menu(title: impl Into<String>, items: Vec<MenuBarItem>) -> Self   // top-level menu or submenu
fn MenuBarItem::new(title: impl Into<String>, action: impl FnMut() + 'static) -> Self
fn MenuBarItem::role(role: MenuRole) -> Self        // standard title, shortcut, behavior
fn MenuBarItem::separator() -> Self                 // empty title, no role
fn MenuBarItem::check_box(title: impl Into<String>, checked: bool, toggled: impl Fn(bool) + 'static) -> Self
fn shortcut(self, shortcut: KeyboardShortcut) -> Self
fn with_role(self, role: MenuRole) -> Self
fn enabled(self, enabled: bool) -> Self             // false: grayed, shortcut does nothing
```
Items for `App::menu_bar`. Plain structs (not views): callbacks run on the UI thread; they are not reactive (call `menu_bar` again to replace). An empty `title` uses the role's standard title. A `menu` with `MenuRole::App` is macOS's application menu (elsewhere its items join the first menu). Copy/Select All roles also work on selectable text.

```rust
app.menu_bar(vec![
    MenuBarItem::menu("", vec![
        MenuBarItem::role(MenuRole::About),
        MenuBarItem::separator(),
        MenuBarItem::new("", open_settings).with_role(MenuRole::Settings),
        MenuBarItem::separator(),
        MenuBarItem::role(MenuRole::Quit),
    ]).with_role(MenuRole::App),
    MenuBarItem::menu("File", vec![
        MenuBarItem::new("New Note", new_note).shortcut(KeyboardShortcut::new(Key::N)),
        MenuBarItem::check_box("Autosave", true, |on| set_autosave(on)),
    ]),
    MenuBarItem::menu("Edit", vec![MenuBarItem::role(MenuRole::Undo), MenuBarItem::role(MenuRole::Redo),
        MenuBarItem::separator(), MenuBarItem::role(MenuRole::Cut), MenuBarItem::role(MenuRole::Copy),
        MenuBarItem::role(MenuRole::Paste)]),
    MenuBarItem::menu("Window", vec![MenuBarItem::role(MenuRole::Minimize), MenuBarItem::role(MenuRole::Zoom)])
        .with_role(MenuRole::Window),
]);
```

### MenuRole

`enum MenuRole { None (default), App, Window, Help, About, Settings, Quit, Hide, HideOthers, ShowAll, Undo, Redo, Cut, Copy, Paste, SelectAll, Minimize, Zoom, ToggleFullscreen }`
Items with a role do the standard thing unless they have their own action, with standard title/shortcut if none given.
- Top-level menus: `App` (macOS app menu), `Window` (macOS lists open windows), `Help` (macOS adds search field).
- `About`: macOS standard About panel. `Settings`: Cmd+, on macOS. `Quit`: quits the app.
- `Hide`, `HideOthers`, `ShowAll`, `ToggleFullscreen`: macOS only (left out elsewhere).
- `Undo`, `Redo`, `Cut`, `Copy`, `Paste`, `SelectAll`: send their shortcut to the focused view (text field).
- `Minimize`: active window. `Zoom`: maximize/restore.

### KeyboardShortcut

```rust
pub struct KeyboardShortcut { pub key: Key, pub modifiers: KeyModifiers }   // Copy, Eq, Hash
fn KeyboardShortcut::new(key: Key) -> Self          // key + KeyModifiers::primary() (Cmd on macOS, Ctrl elsewhere)
fn modifiers(self, modifiers: KeyModifiers) -> Self // replaces the modifiers
```
Used by `MenuBarItem::shortcut`; works while the menu is closed. Not accepted by `MenuItem` (in-window menus), which has a text-only `shortcut` hint.

```rust
KeyboardShortcut::new(Key::S)                                                   // Cmd/Ctrl+S
KeyboardShortcut::new(Key::S).modifiers(KeyModifiers::primary() | KeyModifiers::SHIFT)
```

---

## Desktop integration

Guide: desktop.mdx. Call on the UI thread while the app runs; callbacks run on the UI thread (they can change state). Text given to the system (notifications, tray tooltip/menu, filters, file names, window titles, menu bar, badge) must be UTF-8; malformed text is replaced with U+FFFD.
Import: the guide says `use elegance::desktop::*;`; the items are also re-exported at the crate root and prelude.

| Feature | Windows | macOS | Linux |
|---|---|---|---|
| File dialogs | yes | yes | not documented (unverified) |
| `notify` | toast (kept in notification center; needs AppUserModelID, see below) | needs an app bundle, otherwise only logged; asks the user first time | not documented (unverified) |
| `TrayIcon` | notification area | menu bar | no tray: a warning is logged and nothing shows ("when the system has no tray", C++ header); otherwise not documented |
| `set_badge` | small badge on taskbar button (~2 chars) | Dock text | launchers supporting Unity LauncherEntry (Dash to Dock, KDE): numbers only |
| `set_progress` | every window's taskbar button | Dock icon | where the launcher supports it |
| `request_attention` | taskbar button flashes | Dock icon bounces | launcher-dependent |

There is no Mikros variant in `Platform`; `current_platform()` returns Linux for anything not Windows/macOS.

### choose_files / choose_folders / choose_save_file

```rust
pub fn choose_files(options: OpenFilesOptions, done: impl FnOnce(Vec<PathBuf>) + 'static)
pub fn choose_folders(options: ChooseFolderOptions, done: impl FnOnce(Vec<PathBuf>) + 'static)
pub fn choose_save_file(options: SaveFileOptions, done: impl FnOnce(Option<PathBuf>) + 'static)
```
System dialogs over the active window; the app keeps running. `done` gets what was chosen (empty `Vec`/`None` on cancel). The save dialog asks about overwriting but does not create the file. Dialogs still open when `run()` returns are closed and `done` is dropped uncalled. `done` is `FnOnce` and `'static` (move state clones in).

```rust
use elegance::desktop::*;
choose_files(
    OpenFilesOptions { filters: vec![FileFilter::new("Images", &["png", "jpg", "jpeg"])], multiple: true, ..Default::default() },
    move |files| photos.write().extend(files),
);
choose_save_file(
    SaveFileOptions { filters: vec![FileFilter::new("Text", &["txt"])], name: "notes.txt".into(), ..Default::default() },
    move |path| { if let Some(path) = path { save_notes(path); } },
);
choose_folders(ChooseFolderOptions::default(), |folders| { /* ... */ });
```

### OpenFilesOptions

`pub struct OpenFilesOptions { pub filters: Vec<FileFilter>, pub directory: PathBuf, pub multiple: bool }` (`Default`). `directory` empty: system's choice. Struct-literal with `..Default::default()`.

### SaveFileOptions

`pub struct SaveFileOptions { pub filters: Vec<FileFilter>, pub directory: PathBuf, pub name: String }` (`Default`). `name`: suggested file name.

### ChooseFolderOptions

`pub struct ChooseFolderOptions { pub directory: PathBuf, pub multiple: bool }` (`Default`).

### FileFilter

```rust
pub struct FileFilter { pub name: String, pub extensions: Vec<String> }
fn FileFilter::new(name: impl Into<String>, extensions: &[&str]) -> Self   // FileFilter::new("Images", &["png", "jpg"])
```
Extensions without dots. No extensions: all files.

### notify / Notification

```rust
pub struct Notification { pub title: String, pub body: String, pub on_click: Option<Box<dyn FnMut()>> }
fn Notification::new(title: impl Into<String>, body: impl Into<String>) -> Self
fn on_click(self, f: impl FnMut() + 'static) -> Self
pub fn notify(notification: Notification)
```
System notification; it also stays in the notification center. `on_click` runs while the app runs (after `run()` returns, clicks do nothing). Panics when no App is running. macOS: needs an app bundle (e.g. via cargo-bundle); a bare executable only logs. Windows: toasts show under the app's AppUserModelID, which wants a Start menu shortcut; if the app sets none, Elegance derives an id from the executable name and on first notification adds a shortcut to the person's Start menu; if the app sets its own id, its installer must create the shortcut.

```rust
notify(Notification::new("Export finished", "Saved 12 photos").on_click(show_folder));
```

### TrayIcon / TrayIconOptions / TrayMenuItem

```rust
#[derive(Default)] pub struct TrayIconOptions { pub icon: ImageSource, pub tooltip: String, pub menu: Vec<TrayMenuItem> }
fn TrayIcon::new(options: TrayIconOptions) -> Result<TrayIcon, Error>    // Clone; removed when the last clone drops
fn set_tooltip(&self, tooltip: &str);  fn set_menu(&self, menu: Vec<TrayMenuItem>)

pub struct TrayMenuItem { pub title: String, pub action: Option<Box<dyn FnMut()>>, pub checked: Option<bool>,
    pub toggled: Option<Box<dyn Fn(bool)>>, pub enabled: bool, pub submenu: Vec<TrayMenuItem> }
fn TrayMenuItem::new(title: impl Into<String>, action: impl FnMut() + 'static) -> Self
fn TrayMenuItem::separator() -> Self
fn TrayMenuItem::check_box(title: impl Into<String>, checked: bool, toggled: impl Fn(bool) + 'static) -> Self
fn TrayMenuItem::submenu(title: impl Into<String>, items: Vec<TrayMenuItem>) -> Self
fn enabled(self, enabled: bool) -> Self
```
An icon with a menu in the system tray, shown for as long as you keep a clone: hold it somewhere that lasts (`main`). Create it after `App::new()` (needs a running app per C++ docs: `UsageError` when no App is running); the guide creates it before `run()`. Icon: file or data `ImageSource`; decoded at most 256 px a side; empty/unreadable gives a plain blue dot and a log warning. Tray icons are taken down when `run()` returns. There is no `App::tray` method.

```rust
let app = App::new();
let quit = app.handle();
let _tray = TrayIcon::new(TrayIconOptions {
    icon: ImageSource::file("assets/tray.png"),
    tooltip: "Sync".into(),
    menu: vec![
        TrayMenuItem::check_box("Pause syncing", false, move |on| paused.set(on)),
        TrayMenuItem::separator(),
        TrayMenuItem::new("Quit", move || quit.quit()),
    ],
})?;
app.window(WindowDesc::new("Sync"), SyncView).run()
```

### request_attention

`pub fn request_attention(critical: bool)`. While the app is in the background: Dock icon bounces (once, or until active when `critical`) or taskbar button flashes. Nothing happens while the app is in front. Panics when no App is running.

### set_badge

`pub fn set_badge(text: Option<&str>)`. Short label on the app icon, like an unread count; `None` removes it. Panics when no App is running. Linux launchers show only numbers.

### set_progress / ProgressState

```rust
pub fn set_progress(fraction: Option<f64>, state: ProgressState)   // fraction 0..1, clamped; None removes
enum ProgressState { Normal (default), Paused, Error, Indeterminate }
```
Progress bar on the app icon. `Paused`: Windows yellow, elsewhere as normal. `Error`: Windows red, macOS red bar. `Indeterminate`: busy without a fraction: Windows animates, macOS/Linux show it full. Panics when no App is running.

```rust
set_badge(Some("3"));
set_progress(Some(0.4), ProgressState::Normal);
set_progress(None, ProgressState::Normal);
set_progress(Some(0.0), ProgressState::Indeterminate);
request_attention(false);
```

### current_platform / Platform

```rust
pub enum Platform { Windows, MacOS, Linux }   // Clone, Copy, Eq, Hash
pub fn current_platform() -> Platform         // anything else counts as Linux
```
The platform the app was built for; runs at runtime (not const). No Mikros variant. Pair with `KeyModifiers::primary()` for the shortcut modifier.

```rust
if current_platform() == Platform::MacOS { /* traffic-light layout */ }
```

---

## Clipboard and drag and drop

### Clipboard

No clipboard API exists in elegance-rs, nor in the C++ headers (`include/elg/*.h`): grep for "clipboard" finds only the text-field description ("clipboard" among TextField editing features) and the windows.mdx note that Windows needs OLE for it. Copy/paste happens inside `TextField`/`TextArea` and selectable text (`Text::selectable`, Ctrl/Cmd+C copies plain text), and via `MenuRole::Copy/Cut/Paste/SelectAll/Undo/Redo` menu-bar items. There is no programmatic read/write. Status: unavailable in Rust and C++.

### on_drop

```rust
fn on_drop(self, options: DropOptions) -> View
fn on_drop_with(self, options: impl FnOnce(DropOptions) -> DropOptions) -> View   // elg!: .on_drop(files: |paths, at| ..)
```
Accept files or text dragged in from other apps (file manager, browser). A drop goes to one view: the innermost enabled view under the pointer with a handler for that kind (topmost among siblings). Handlers' return values do not matter; locations are in the view's coordinates. Disabled views do not take drops. Windows needs OLE (set up by `run()`).

### DropOptions

```rust
#[derive(Default)] pub struct DropOptions {
    pub files: Option<Box<dyn Fn(Vec<PathBuf>, Point)>>,
    pub text: Option<Box<dyn Fn(String, Point)>>,
    pub targeted: Option<Box<dyn Fn(bool)>> }
fn files(self, f: impl Fn(Vec<PathBuf>, Point) + 'static) -> Self
fn text(self, f: impl Fn(String, Point) + 'static) -> Self
fn targeted(self, f: impl Fn(bool) + 'static) -> Self   // true while a takeable drag is over the view: for a highlight
```
A kind with no handler (`None`) is not accepted.

```rust
let targeted = State::new(false);
elg! {
    Text("Drop files here").on_drop(
        DropOptions::default()
            .files(|files, _at| import(files))
            .text(|text, _| paste(text))
            .targeted(move |over| targeted.set(over)),
    )
}
```

Drag sources (starting a drag out of the app to other apps): not provided in Rust or C++. `on_drag` is only the in-app pointer drag gesture (see Events).

---

## Accessibility

Guide: accessibility.mdx. Elegance exposes every window's contents to screen readers through AccessKit: Narrator/NVDA/JAWS and UI Automation on Windows; VoiceOver on macOS; Orca and other AT-SPI readers on Linux (X11 and Wayland; with no accessibility bus, windows still open without delay). Nothing is computed until a screen reader asks.
Free: `Text` (label), `Button` (named by its label text, pressable), `Toggle` (checkbox/switch + state), `Slider`/`NumberInput` (value, range; settable), `TextField`/`TextArea` (text; secure field is a password), `Progress`, images, `Separator` (hide decorative ones), selectable lists (list box with selected rows), keyboard focus (Tab).
Not yet supported: announcing live changes (no live-region API; there is none in the Rust modifiers), cursor/selection inside text fields, following reduced-motion/high-contrast settings.
Text in labels/values/hints and the window title is cleaned to valid UTF-8 (U+FFFD). Screen readers cannot activate a disabled view (its action, including `accessibility_action`, does not run). Requests are handled like events: state changes apply when the request ends.

Modifiers (all on `ViewExt`):

### accessibility_label

`fn accessibility_label(self, label: impl IntoProp<String>) -> View`. What it is called. Required for icon-only buttons. Reactive.

### accessibility_value

`fn accessibility_value(self, value: impl IntoProp<String>) -> View`. What it holds, in words ("3 of 10", "On"). Reactive: pass a closure.

### accessibility_hint

`fn accessibility_hint(self, hint: impl IntoProp<String>) -> View`. What using it does, if the label does not say.

### accessibility_hidden

`fn accessibility_hidden(self, hidden: bool) -> View`. Leave it and everything inside out (decoration). Takes a plain `bool` (not reactive); the C++ default of `true` does not exist in Rust, so write `true`.

### accessibility_role

`fn accessibility_role(self, role: impl Into<AccessibilityRole>) -> View`. Override the automatic role.

### accessibility_element

`fn accessibility_element(self) -> View`. One element for everything inside, labeled with their text: makes a row read as one line.

### accessibility_action

`fn accessibility_action(self, action: impl FnMut() + 'static) -> View`. What a screen reader's default action (click) does; for views made clickable with gestures. Prefer `on_activate` for custom controls, which covers pointer, keyboard (Tab, Space/Return) and screen readers at once.

### AccessibilityRole

`enum AccessibilityRole { Group, Label, Heading, Button, Link, Image, Toggle, Checkbox, Slider, Progress, TextField, List, ListItem, Tab, MenuItem }` (no `Default`). In `elg!`: `.accessibility_role(.heading)`.

```rust
Button(action: remove) { Image("trash") }.accessibility_label("Delete")

HStack { Image("user") Text(name) Spacer() Text(email) }.accessibility_element()

// custom control
ZStack(alignment: .leading) {
    Capsule().fill(track_color)
    Circle().fill(.white).frame(28, 28).padding(2).offset(knob_x, 0.0)
}
.frame(52, 32)
.on_activate(move || dark.toggle())
.accessibility_element()
.accessibility_role(.toggle)
.accessibility_label("Dark mode")
.accessibility_value(move || if dark.get() { "On" } else { "Off" })
```
Checking: Narrator (Ctrl+Win+Enter), VoiceOver (Cmd+F5), Orca; Accessibility Insights / Windows SDK Inspect; Xcode Accessibility Inspector.

---

## Events and input types

Handlers are attached by modifiers (documented in modifiers.md): `on_click`, `on_click_with`, `on_double_click`, `on_activate`, `recognize`, `recognize_with`, `on_drag`, `on_long_press`, `on_magnify`, `on_rotate`, `on_two_finger_double_tap`, `on_pointer`, `on_pointer_capture`, `on_scroll`, `on_scroll_capture`, `pointer_style`, `on_pointer_enter`, `on_pointer_leave`, `on_hover_change`, `hover_delay`, `hover_delay_with`, `on_key_press`, `on_key_press_capture`, `on_key`, `on_key_capture`, `on_text_input`, `on_text_input_capture`, `focusable`, `focused`, `on_size_change`.

Which handler to choose:

| Goal | Use |
|---|---|
| Pointer click on something (location, button, modifiers, count) | `on_click` / `on_double_click` / `on_click_with(.., button: .right, count: 2)` |
| A view that IS a control (click, Space/Return when focused, screen reader) | `on_activate` + role + label |
| Drag a view, swipe, reorder | `recognize(DragGesture ...)` (`on_drag` for just `on_changed`) |
| Long press | `recognize(LongPressGesture ...)` (`on_long_press` for just an action) |
| Pinch / rotate / two-finger double tap (trackpad) | `MagnifyGesture` / `RotateGesture` / `TwoFingerDoubleTapGesture` (or `on_magnify`/`on_rotate`/`on_two_finger_double_tap`) |
| Highlight on hover, exact | `on_pointer_enter` / `on_pointer_leave` |
| Hover intent (previews, no flicker) | `on_hover_change` + `hover_delay` |
| Custom drawing/controls needing every down/move/up | `on_pointer` (consume a down to capture the pointer) |
| Wheel/trackpad scroll override | `on_scroll` (return true to stop scroll views around) |
| Keyboard shortcuts | `on_key_press(Key::X, action)` for one key; `on_key` for down/up/modifiers/repeat; make the view `focusable(true)` |
| Typed text / IME | `on_text_input` |
| See events before inner views (Esc in a form, outside-press) | the `_capture` handlers |
| Cursor shape | `pointer_style` |

Notes: the innermost click handler wins; a double-click handler lets the first click through to outer single-click handlers. Keys go to the focused view first, then outward. `.disabled()` turns off clicks, gestures, keys, typing, focus, context menus, drops (hover and scrolling still work). Raw handlers return `bool`: true consumes. Removing a view cancels its captured press/gestures and sends pointer-leave.

### Key

`enum Key` (`#[repr(i32)]`, `Default = Unknown`): `Unknown`, `A`..`Z`, `Digit0`..`Digit9`, `Enter`, `Escape`, `Backspace`, `Tab`, `Space`, `ForwardDelete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Left`, `Right`, `Up`, `Down`, `F1`..`F12`, `Menu`. Keys by meaning. Where an `impl Into<Key>` is taken, in `elg!` you can write `Key::Escape`.

### KeyModifiers

```rust
pub struct KeyModifiers(pub u32)   // Copy, Eq, Hash, Default, BitOr
consts: NONE, SHIFT, CONTROL, ALT, SUPER   // SUPER: Cmd on macOS, Windows key on Windows
fn has(self, modifier: KeyModifiers) -> bool
fn primary() -> KeyModifiers               // Cmd on macOS, Ctrl elsewhere
```
Combine with `|`. Use `e.modifiers.has(KeyModifiers::primary())` for shortcuts.

### KeyEvent

`pub struct KeyEvent { pub kind: KeyEventKind, pub key: Key, pub modifiers: KeyModifiers, pub repeat: bool }`. `repeat`: auto-repeat while held. Handler: `on_key(|e: &KeyEvent| -> bool)`.

### KeyEventKind

`enum KeyEventKind { Down (default), Up }`.

```rust
Text("Editor").on_key_press(Key::Escape, close)
Canvas(|_| {}).focusable(true).on_key(move |e| {
    if e.kind == KeyEventKind::Down && e.key == Key::S && e.modifiers.has(KeyModifiers::primary()) {
        save();
        return true; // handled: stops here
    }
    false
})
```

### TextInputEvent / TextInputKind

```rust
pub struct TextInputEvent { pub kind: TextInputKind, pub text: String, pub cursor: usize }  // cursor: IME caret, bytes into text
enum TextInputKind { Commit (default), Composition }   // Composition: in-progress IME text, replaces the previous ("" ends it)
```
`on_text_input` makes the view focusable; keys still go to `on_key`. `_capture` variant does not.

```rust
Canvas(|_| {}).on_text_input(|e| { if e.kind == TextInputKind::Commit { insert(&e.text); } true })
```

### PointerEvent

```rust
pub struct PointerEvent { pub kind: PointerEventKind, pub location: Point, pub button: MouseButton,
    pub click_count: i32, pub modifiers: KeyModifiers, pub window_location: Point }
```
`location` is in the view's own coordinates for `on_pointer` (window coordinates for a custom `EventHandler`); `window_location` is always window coordinates. `click_count`: 2 for the second click of a double click.

### PointerEventKind

`enum PointerEventKind { Down, Up, Move (default), Cancel, Hover }`. `Move` only goes to the handler that captured the pointer. `Cancel`: pointer lost (window closed, another view took a new press): reset pressed state.

```rust
Canvas(|_| {}).on_pointer(move |e| {
    if matches!(e.kind, PointerEventKind::Down | PointerEventKind::Move) { cursor.set(e.location); return true; }
    false
})
```

### ClickEvent

```rust
pub struct ClickEvent { pub location: Point, pub window_location: Point, pub button: MouseButton,
    pub modifiers: KeyModifiers, pub click_count: i32 }
```
`location`: where released, in the view's own coordinates. Left button by default.

### ClickOptions

```rust
pub struct ClickOptions { pub button: MouseButton, pub count: i32 }   // Default: Left, 1
fn button(self, impl Into<MouseButton>) -> Self;  fn count(self, i32) -> Self
```
Used via `on_click_with(action, |o| o.button(..).count(..))`; in `elg!`: `.on_click(open, count: 2)`, `.on_click(menu, button: .right)`.

### MouseButton

`enum MouseButton { Left (default), Middle, Right, Other }`.

### ScrollEvent

```rust
pub struct ScrollEvent { pub location: Point, pub window_location: Point, pub dx: f32, pub dy: f32,
    pub precise: bool, pub modifiers: KeyModifiers, pub phase: GesturePhase, pub momentum: bool, pub time: f64 }
```
`precise`: trackpad-style fine deltas (apply directly) vs wheel notches. `phase`/`momentum`: macOS trackpads report the fingers' gesture, then momentum coasting. `time`: seconds on the source's clock, 0 if unknown. Ctrl+wheel arrives with the modifier set (zoom).

### MagnifyEvent

`pub struct MagnifyEvent { pub location: Point, pub scale: f32, pub delta: f32, pub phase: GesturePhase }`. `scale`: since gesture start (2 = twice as big); `delta`: factor since last event.

### RotateEvent

`pub struct RotateEvent { pub location: Point, pub degrees: f32, pub delta: f32, pub phase: GesturePhase }`. `degrees`: clockwise since start.

### DragEvent

```rust
pub struct DragEvent { pub location: Point, pub start_location: Point, pub translation: Point,
    pub velocity: Point, pub button: MouseButton, pub modifiers: KeyModifiers }
```
Locations in the view's own coordinates; `translation` is measured in the window (so a view moving with the drag does not feed back); `velocity` in points per second.

### GesturePhase

`enum GesturePhase { None (default), Began, Changed, Ended, Cancelled }`. `None`: not part of a gesture (mouse wheel, or a trackpad the platform does not follow). `Cancelled`: the system took the gesture over.

### EventPhase

`enum EventPhase { Capture, Bubble (default) }`. Pointer downs/ups/hovers, scrolling, keys and text input travel in two phases: capture handlers outermost to innermost, then bubble handlers innermost to outermost; the first handler that consumes ends it. Used by custom `EventHandler::phase()`; the `_capture` modifiers use `Capture`.

### EventResult

```rust
pub struct EventResult { pub consumed: bool, pub redraw: bool, pub cancellable: bool }   // Default: all false
consts: IGNORED, CONSUMED, REDRAW (consumed + redraw)
fn cancellable(self) -> Self
```
Return value of custom `EventHandler` methods (see modifiers/custom modifiers reference). `cancellable` with `consumed` on a down: gestures around the view may take the press over and the handler then gets `Cancel`; for presses that are clicks, not for pointer-tracking controls like sliders.

### PointerStyle

`enum PointerStyle { Automatic (default), Arrow, Pointer, Text, Crosshair, Move, Wait, Progress, ResizeHorizontal, ResizeVertical, ResizeNwse, ResizeNesw, NotAllowed }`. Like CSS `cursor`. `Automatic`: views around decide; innermost non-`Automatic` style wins; none: arrow. `Pointer` = hand, `Text` = I-beam, `Wait` = busy and blocked, `Progress` = busy but takes input. Reactive (`IntoProp`).

```rust
Text("Card").on_activate(open).pointer_style(.pointer)
Text("Content").pointer_style(move || if busy.get() { PointerStyle::Progress } else { PointerStyle::Automatic })
```

### PixelSize

`pub struct PixelSize { pub width: u32, pub height: u32 }`. Physical pixels (logical size x display scale, rounded up; excludes transforms, clipping, shadows; zero possible) given to `on_size_change(|size: PixelSize| ..)`, called after layout and on changes, before paint. Queue it (channel/`Cell`) and render at that size; do not change layout or rebuild views from the callback. Place the modifier inside padding/frames whose extra space should not count. `on_sized` is a deprecated alias.

### HoverDelay

```rust
pub struct HoverDelay { pub enter: f64, pub exit: f64 }   // Default: 0.0, 0.0; seconds
fn enter(self, seconds: f64) -> Self;  fn exit(self, seconds: f64) -> Self
```
Used by `.hover_delay(HoverDelay)` / `.hover_delay_with(|d| d.enter(0.3).exit(0.2))` for everything inside (`on_hover_change`, submenus). Pointer enter/leave and controls' highlights never wait.

```rust
Text("Tile").on_hover_change(show_preview).hover_delay(HoverDelay::default().enter(0.5))
```

### Gesture

`pub trait Gesture` (attach is hidden): implemented by `DragGesture`, `LongPressGesture`, `MagnifyGesture`, `RotateGesture`, `TwoFingerDoubleTapGesture`. Attach with `view.recognize(gesture)` or `view.recognize_with(gesture, |o| o.priority(..).simultaneous(..))` (elg!: `.recognize(g, priority: .high)`). A pointer gesture watches presses without taking them; once it recognizes (drag moved far enough, long press held long enough) it takes the press: the button or `on_click` it started on is cancelled. Innermost recognizing gesture wins. Controls that track the pointer (sliders, text selection) keep their presses unless the gesture has `High` priority.

### GestureOptions

```rust
pub struct GestureOptions { pub priority: GesturePriority, pub simultaneous: bool }   // Default: Normal, false
fn priority(self, impl Into<GesturePriority>) -> Self;  fn simultaneous(self, bool) -> Self
```
Apply to drags and long presses only (ignored by Magnify/Rotate/TwoFingerDoubleTap). `simultaneous: true`: recognizes alongside whatever has the press without taking it or being cancelled; priority does not apply.

### GesturePriority

`enum GesturePriority { Normal (default), High }`. `High`: offered the press before gestures inside its view and may take it from controls that keep their presses (row swipe over sliders).

```rust
Text("Row").recognize(DragGesture::new().on_changed(swipe), priority: .high)
Text("Card").recognize(DragGesture::new().on_changed(tilt), simultaneous: true)
```

### DragGesture

```rust
fn DragGesture::new() -> Self
fn DragGesture::with(options: impl FnOnce(DragOptions) -> DragOptions) -> Self
fn on_began(self, impl Fn(&DragEvent) + 'static) -> Self      // drag recognized
fn on_changed(self, impl Fn(&DragEvent) + 'static) -> Self    // pointer moved (also right after on_began)
fn on_ended(self, impl Fn(&DragEvent) + 'static) -> Self      // button released
fn on_cancelled(self, impl Fn() + 'static) -> Self            // pointer lost before release
```
Prefer `on_drag` for a plain drag with only `on_changed`. Callbacks are `Fn`: keep mutable data in `State`. Pattern: remember the start in `on_began`, then set `start + e.translation` in `on_changed`.

```rust
.recognize(
    DragGesture::with(|o| o.minimum_distance(8))
        .on_began(move |_| start.set(*offset.peek()))
        .on_changed(move |e| offset.set(Point::new(start.get().x + e.translation.x, start.get().y + e.translation.y)))
        .on_ended(|e| settle(e.velocity)),
)
// hold, then drag (reorder by touch):
.recognize(DragGesture::with(|o| o.hold(0.4)).on_began(move |_| lifted.set(true))
    .on_changed(move |e| offset.set(e.translation)).on_ended(move |_| lifted.set(false)))
```

### DragOptions

```rust
pub struct DragOptions { pub minimum_distance: f32, pub button: MouseButton, pub hold: f64, pub hold_tolerance: f32 }
// Default: 4.0, Left, 0.0, 10.0
fn minimum_distance(self, impl Number) -> Self   // points before the drag begins; 0 begins on the press
fn button(self, impl Into<MouseButton>) -> Self
fn hold(self, seconds: f64) -> Self              // press must stay put this long (within hold_tolerance) before the drag can begin; then on_began fires at once with no translation; a press that moves sooner is left to scroll views/clicks. 0: no hold
fn hold_tolerance(self, impl Number) -> Self     // points
```

### LongPressGesture

```rust
fn LongPressGesture::new(action: impl Fn() + 'static) -> Self
fn options(self, options: impl FnOnce(LongPressOptions) -> LongPressOptions) -> Self
```
Runs `action` once the press is held for the duration; the click it would have been does not happen. Prefer `on_long_press(action)` when no options are needed.

```rust
Text("Photo").recognize(LongPressGesture::new(show_details)
    .options(|o| o.duration(0.4).pressing(move |held| pressed.set(held))))
```

### LongPressOptions

```rust
pub struct LongPressOptions { pub duration: f64, pub maximum_distance: f32, pub button: MouseButton,
    pub pressing: Option<Rc<dyn Fn(bool)>> }   // Default: 0.5 s, 10.0 pt, Left, None
fn duration(self, seconds: f64) -> Self
fn maximum_distance(self, impl Number) -> Self    // points the pointer may wander
fn button(self, impl Into<MouseButton>) -> Self
fn pressing(self, impl Fn(bool) + 'static) -> Self   // true when a candidate press goes down, false when it lifts/moves away/is taken: "being held" highlight
```

### PhasedGesture

`pub struct PhasedGesture<E: 'static>` with `new()`, `on_began`, `on_changed`, `on_ended` (each `impl Fn(&E) + 'static`), `on_cancelled(impl Fn() + 'static)`. Generic base of `MagnifyGesture` and `RotateGesture`. The innermost one under the pointer that takes the start gets the rest.

### MagnifyGesture

`type MagnifyGesture = PhasedGesture<MagnifyEvent>`. Pinch on a trackpad (or touch screen; other platforms that report it). Event `scale` is since the gesture began. Prefer `on_magnify(action)` for only `on_changed`.

### RotateGesture

`type RotateGesture = PhasedGesture<RotateEvent>`. Two-finger turn, `degrees` clockwise since the start. Comes from Mac trackpads. Prefer `on_rotate(action)`.

### TwoFingerDoubleTapGesture

`TwoFingerDoubleTapGesture::new(action: impl Fn(Point) + 'static)`. Usually "zoom to fit"; the point is in window coordinates. Mac trackpads. Shorthand `on_two_finger_double_tap(|p| ..)`.

```rust
elg! {
    Text("Photo")
        .recognize(MagnifyGesture::new()
            .on_began(move |_| start.set(zoom.get()))
            .on_changed(move |e| zoom.set((start.get() * e.scale).clamp(0.5, 8.0))))
        .on_rotate(move |e| angle.set(e.degrees))
        .on_two_finger_double_tap(move |_| zoom.set(if zoom.get() == 1.0 { 2.0 } else { 1.0 }))
}
```

---

## C++-only / not exposed in Rust

- Runtime window opening: no API in either (C++ `App::window` throws `UsageError` while running).
- Clipboard read/write: not in C++ headers or Rust.
- Drag sources (dragging out to other apps): not in either.
- C++ `accessibility_hidden(bool hidden = true)` has a default; Rust requires the argument.
- The `ELG_ACCESSIBILITY` CMake option (C++ build-time toggle of accessibility) has no Rust counterpart.
- The guide `views-and-modifiers.mdx` lists a `help` popup modifier; no `help` exists in `ViewExt` (grep in elegance-rs/elegance/src) or `view.h` (unverified elsewhere): treat as not available.
- Live-region announcements, custom accessibility actions beyond `accessibility_action`, and accessibility selection/cursor: not present (docs list live announcements as planned).
- Platform::Mikros: no such variant.

## Doc-vs-code notes

- desktop.mdx says the desktop items are in `elegance::desktop` "which the prelude doesn't include". In code, lib.rs re-exports them (`choose_files`, `notify`, `TrayIcon`, ... ) in `items`, which the prelude includes, so both `use elegance::desktop::*` and the prelude work.
- Docs say tray icons need a running app only for notifications etc.; C++ `TrayIcon` throws `UsageError` with no running App, while the desktop guide's Rust example creates `TrayIcon::new` before `app.run()`. The guide example is the compile-checked form, so it works; unverified what a Rust call with no App does.
- `MenuItem::shortcut` is a text hint (`impl IntoProp<String>`), distinct from `KeyboardShortcut` (menu bar only).
- menus.mdx popover/menu snippet `MenuButton("Attach", attach_menu, edge: .top, shows_indicator: false)` uses `edge:` shorthand which maps to `MenuButton::edge`; in C++ the same is `placement.edge`.
