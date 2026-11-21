# Elegance (Rust) view modifiers reference

Every method of `ViewExt` (the trait all views get), the options structs behind the `_with` variants, the generic `.modifier(...)` entry and the built-in modifier value types. Source of truth: `elegance-rs/elegance/src/view.rs` (trait `ViewExt`), `view/options.rs`, `modifier.rs`.

## Table of contents

- [How modifiers work](#how-modifiers-work)
- [Modifier order matters](#modifier-order-matters)
- [Argument kinds](#argument-kinds)
- [The `_with` options-builder variants](#the-_with-options-builder-variants)
- [Sizing and layout](#sizing-and-layout)
  - [padding](#padding-padding_edges-padding_insets-padding_with)
  - [frame](#frame-frame_with-frame_size)
  - [fixed_size](#fixed_size-fixed_size_axes)
  - [aspect_ratio](#aspect_ratio-aspect_ratio_with)
  - [resizable](#resizable)
  - [layout_priority](#layout_priority)
  - [z_index](#z_index)
  - [layout_value](#layout_value)
  - [Grid cells](#grid-cells-grid_cell_columns-grid_cell_alignment-grid_column_alignment)
  - [Backgrounds and overlays](#backgrounds-and-overlays)
- [Alignment](#alignment)
- [Appearance](#appearance)
  - [foreground_style, tint, font, font_weight](#foreground_style-tint-font-font_weight)
  - [color_scheme and theme](#color_scheme-and-theme)
  - [opacity](#opacity)
  - [Shadows](#shadows-shadow-inner_shadow)
  - [Borders and clipping](#borders-and-clipping-border-clipped-clip_shape-corner_radius)
  - [mask](#mask-mask_view)
  - [Blur and backdrop blur](#blur-and-backdrop-blur)
  - [Color filters](#color-filters)
  - [blend_mode and compositing_group](#blend_mode-and-compositing_group)
- [Transforms](#transforms-offset-rotation_effect-scale_effect)
- [Animation and transitions](#animation-and-transitions)
- [Interaction](#interaction)
  - [Enabling, hit testing, content shape](#enabling-hit-testing-content-shape)
  - [Clicks and activation](#clicks-and-activation)
  - [Gestures](#gestures)
  - [Raw pointer and scroll events](#raw-pointer-and-scroll-events)
  - [Hover, pointer boundary, pointer style](#hover-pointer-boundary-pointer-style)
  - [Focus](#focus)
  - [Keyboard and text input](#keyboard-and-text-input)
  - [Drops](#drops)
  - [Selection](#selection-selection-selection_with)
  - [window_drag_area](#window_drag_area)
  - [Event phases (capture and bubble)](#event-phases-capture-and-bubble)
- [Lifecycle and size observers](#lifecycle-and-size-observers)
- [Control-style modifiers](#control-style-modifiers)
- [Tooltips, popovers, menus](#tooltips-popovers-menus)
- [Accessibility](#accessibility)
- [Environment modifiers](#environment-modifiers-environment)
- [Generic entry points: modifier, apply](#generic-entry-points-modifier-apply)
  - [Modifier traits](#modifier-traits)
  - [Built-in modifier value types](#built-in-modifier-value-types)
- [Supporting types](#supporting-types)
- [C++-only / not exposed in Rust](#c-only--not-exposed-in-rust)
- [Checklist of every ViewExt method](#checklist-of-every-viewext-method)

## How modifiers work

- `ViewExt` is implemented for everything that is `IntoView` (`impl<T: IntoView> ViewExt for T {}`). Every modifier takes `self` and returns a new `View`. Nothing mutates; the result of a modifier is a general `View`.
- `Text` and the shapes have inherent methods of their own (`.bold()`, `.fill()`, `.line_limit()` ...). Call those first; after any `ViewExt` modifier the value is a plain `View` and they are gone. See text/shapes reference files.
- In `elg!`, a modifier with named arguments `.m(a, key: v)` expands to `.m_with(a, |o| o.key(v))`. Outside `elg!`, call `m_with` yourself.
- Leading-dot values (`.leading`, `.infinity`, `.accent`, `.top_leading`) are only valid inside `elg!` and are converted to the parameter's type.
- `View` is cheap to clone (`Clone`); `&View` is `IntoView`; `()` and `Option<V>` are `IntoView` too.

```rust
elg! {
    Text("Save")
        .font(.headline)
        .foreground_style(.white)
        .padding(horizontal: 16, vertical: 8)
        .background_in(.accent, Capsule())
        .shadow(radius: 6, y: 2)
}
```

## Modifier order matters

Each modifier wraps everything written before it. Read a chain top to bottom and picture each step wrapping the previous.

| Chain | Result |
| --- | --- |
| `.padding(8).background(c)` | background covers the padding |
| `.background(c).padding(8)` | background hugs the content; padding is outside it |
| `.frame(40, 40).background(c)` | background is 40x40 |
| `.background(c).frame(40, 40)` | background is the content's size, centered in a 40x40 frame |
| `.frame(max_width: .infinity)` on a Button | makes room around it, the button stays label-sized. Put the frame on the Button's label instead: `Button(action: f) { Text("Go").frame(max_width: .infinity) }` |
| `.corner_radius(8).padding(4)` | clips the content, then pads |
| `.padding(12).background_in(tint, RoundedRectangle(12)).backdrop_blur_in(30, RoundedRectangle(12), ..)` | `.backdrop_blur*` goes behind everything it wraps, like `.background`: put the tint on before it |
| `.on_click(..).disabled(true)` vs `.disabled(true).on_click(..)` | `disabled` switches off handlers inside it. Put it outside (after) the handlers it must disable |
| `.border(..)` after `.padding` | border is drawn inside the padded bounds |
| `.offset(..)`, `.rotation_effect(..)`, `.scale_effect(..)` | move only drawing, not layout or hit testing; they do not change what surrounds them |
| `.animation(a, v)` | animates changes inside the view it wraps (everything before it), not what is added after it |
| `.on_size_change(..)` | place inside any padding/frame whose extra space should not count in the observed size |

When a result surprises you, check order first.

## Argument kinds

Four kinds of argument appear in signatures. Look at the parameter type.

| Parameter type | You may pass | Notes |
| --- | --- | --- |
| `impl IntoProp<T>` (reactive) | a plain `T`; a `State<T>`, `Binding<T>`, `Computed<T>`; a closure `move \|\| -> T` (`Fn() -> T + 'static`) | Reactive: only that property of that one view updates when the source changes. Pass the State or a closure, never a value you already read (`state.get()` freezes it). For `f32` props, integer and float literals work (`.opacity(1)`, `.padding(8)`). For `String` props `&str`, `&String`, `Cow<str>`, or a closure returning `Into<String>` |
| `impl IntoProp<Color>` | `Color`, a `ThemeColor` (`.accent`, `.secondary`, looked up in the theme where the view is), closure returning a color or theme color, `State<Color>` | |
| `impl IntoProp<ShapeStyle>` | `Color`, `ThemeColor`, `LinearGradient`, `RadialGradient`, `EllipticalGradient`, `AngularGradient`, closure | Used by `background`, `mask` |
| `impl IntoBinding<bool>` | `State<bool>`, `Binding<bool>`; a read-only `Computed<bool>` or closure `move \|\| -> bool` | Two-way when given a State/Binding (the modifier writes back). A closure/Computed is read-only: dismissal closes it until the condition next becomes true (see `popover`) |
| `impl Number` | `f32`, `f64`, `i8..i64`, `isize`, `u8..u64`, `usize`, and `.infinity` in `elg!` | Plain, static numbers (not reactive): `border` width, `layout_priority`, `corner_radius`, `aspect_ratio` ratio, `Shadow` fields |
| `impl Into<Len>` | any `Number`, or `Option<f32>`; `Len(None)` is "unset" | `frame` dimensions. `.infinity` allowed |
| `impl Into<Alignment>` / `Into<Shape>` / `Into<ContentMode>` / `Into<BlendMode>` / `Into<Key>` | the enum/struct or a leading-dot name in `elg!` (`.top_leading`, `.fill`, `.multiply`) | `Shape` accepts `Rectangle()`, `RoundedRectangle(r)`, `Capsule()`, `Circle()`, `PathShape(..)`, `CustomShape(..)` |
| `bool` / `i32` / `f64` plain | literal only | `fixed_size_axes`, `accessibility_hidden`, `focusable`, `allows_hit_testing`, `animates_layout`, `window_drag_area`, `grid_cell_columns` are NOT reactive |
| `impl Fn(&Event) + 'static` / `FnMut()` | closures | Handlers. `FnMut()` for no-arg actions (`on_appear`, `on_activate`, `on_key_press`, `on_pointer_enter`, `accessibility_action`); `Fn(..)` for ones that take an event. Bool-returning ones (`on_key`, `on_pointer`, `on_scroll`, `on_text_input`) return `true` to consume |
| `impl FnOnce(Options) -> Options` | closure used by every `*_with` | `\|o\| o.width(10).height(20)` |
| `impl IntoView` / `impl Fn() -> V + 'static` | a view, or a builder closure | `overlay`, `background_view`, `mask_view` take a view value; `context_menu`, `tooltip_view`, `floating`, `popover` take a builder closure that runs each time it opens |

```rust
# let (hovered, compact) = (State::new(false), State::new(false));
elg! {
    Text("Row")
        .opacity(move || if hovered.get() { 1.0 } else { 0.6 })   // closure
        .padding(move || if compact.get() { 4.0 } else { 12.0 })  // closure
        .foreground_style(.secondary)                             // theme color, resolved where the view is
        .disabled(hovered)                                        // State<bool>
        .frame(40, 40)                                            // plain numbers (Len)
}
```

## The `_with` options-builder variants

| Base | `_with` form | Options type |
| --- | --- | --- |
| `padding` family | `padding_with(\|e: EdgeInsets\| ..)` | `EdgeInsets` |
| `frame` | `frame_with` | `FrameOptions` |
| `aspect_ratio` | `aspect_ratio_with` | `AspectRatioOptions` |
| `background_view`, `overlay`, `mask_view` | `background_view_with`, `overlay_with`, `mask_view_with` | `DecorationOptions` |
| `shadow`, `inner_shadow` | `shadow_with`, `inner_shadow_with` | `Shadow` |
| `blur` | `blur_with` | `BlurOptions` |
| `backdrop_blur` | `backdrop_blur_with`, `backdrop_blur_in` | `BackdropOptions` |
| `rotation_effect`, `scale_effect`, `scale_effect_xy` | `*_with` | `AnchorOptions` |
| `on_click` | `on_click_with` | `ClickOptions` |
| `recognize` | `recognize_with` | `GestureOptions` |
| `on_drop` | `on_drop_with` | `DropOptions` |
| `hover_delay` | `hover_delay_with` | `HoverDelay` |
| `tooltip` | `tooltip_with`, `tooltip_view(content, options)` | `TooltipOptions` |
| `popover` | `popover_with` | `PopoverOptions` |
| `floating` | takes the options closure directly | `FloatingOptions` |
| `selection` | `selection_with(selection, on_activate)` | (a closure, not an options struct) |

Options structs are listed with all fields in [Supporting types](#supporting-types).

---

## Sizing and layout

### padding, padding_edges, padding_insets, padding_with

```rust
fn padding(self, length: impl IntoProp<f32>) -> View
fn padding_edges(self, edges: impl Into<Edges>, length: impl IntoProp<f32>) -> View
fn padding_insets(self, insets: impl IntoProp<EdgeInsets>) -> View
fn padding_with(self, insets: impl FnOnce(EdgeInsets) -> EdgeInsets) -> View
```

Space around the content. `padding` is every edge (`Edges::ALL`); `padding_edges` selects edges; `padding_insets` takes an `EdgeInsets` (static or reactive); `padding_with` is what `elg!` uses for named arguments (`top`, `leading`, `bottom`, `trailing`, `horizontal`, `vertical`, `all_edges`; each `impl Number`).

- `Edges` constants: `TOP`, `LEADING`, `BOTTOM`, `TRAILING`, `HORIZONTAL`, `VERTICAL`, `ALL`; combine with `|`. `EdgeInsets::all(f32)`, `EdgeInsets::symmetric(h, v)`.
- Default: there is no argument-less padding in Rust (C++ `.padding()` defaults to 16; Rust always needs a length).
- Prefer `padding` for uniform, `padding_edges` for one axis/side, `padding_with` (named args) for mixed.
- Padding is laid out; it grows the view. Apply before `background` if the background should cover it.

```rust
Text("B").padding(8)                         // every edge
Text("C").padding_edges(.horizontal, 12)     // leading + trailing
Text("D").padding(top: 4, bottom: 12)        // padding_with
Text("E").padding(horizontal: 16, vertical: 8)
Text("F").padding(move || if compact.get() { 4.0 } else { 12.0 })
```

### frame, frame_with, frame_size

```rust
fn frame(self, width: impl Into<Len>, height: impl Into<Len>) -> View
fn frame_with(self, options: impl FnOnce(FrameOptions) -> FrameOptions) -> View
fn frame_size(self, size: impl IntoProp<Size>, alignment: impl Into<Alignment>) -> View
```

`frame(w, h)` gives a fixed size (unset dimensions follow the content) and centers the content. `frame_with` / named args take `FrameOptions`: `width`, `height`, `min_width`, `ideal_width`, `max_width`, `min_height`, `ideal_height`, `max_height` (each `impl Into<Len>`) and `alignment`. A range frame takes the offered size, clamped to the range. `frame_size` is a fixed size that follows state (reactive `Size`) and relayouts when it changes.

- Prefer `frame_with`/named for flexible frames (`max_width: .infinity`), plain `frame` for fixed sizes, `frame_size` when the size is state.
- Default alignment: `Alignment::CENTER`.
- To widen a Button, frame its label (see [order](#modifier-order-matters)).

```rust
Circle().fill(.accent).frame(40, 40)
Text("Title").frame(width: 200, alignment: .leading)
VStack { Text("Top left") }.frame(max_width: .infinity, max_height: .infinity, alignment: .top_leading)
```

### fixed_size, fixed_size_axes

```rust
fn fixed_size(self) -> View
fn fixed_size_axes(self, horizontal: bool, vertical: bool) -> View
```

Proposes nothing on the chosen axes, so the content takes its ideal size (text that must never wrap). `fixed_size()` is both axes.

```rust
Text("Never wraps").fixed_size()
Text("One line tall, wraps wide").fixed_size_axes(false, true)
```

### aspect_ratio, aspect_ratio_with

```rust
fn aspect_ratio(self, ratio: impl Number, mode: impl Into<ContentMode>) -> View
fn aspect_ratio_with(self, options: impl FnOnce(AspectRatioOptions) -> AspectRatioOptions) -> View
```

Sizes the view to width/height within what is offered. `AspectRatioOptions`: `ratio` (`impl Number`; unset uses the content's own, e.g. an image's), `mode` (`ContentMode::Fit` default = all visible, `Fill` = covers and overflows; clip after). `aspect_ratio_with` with only `mode` keeps the image's own ratio.

```rust
Rectangle().fill(.surface).aspect_ratio(16.0 / 9.0, .fit)
Image(ImageSource::file(cover)).resizable().aspect_ratio(mode: .fill).frame(120, 120).clipped()
```

### resizable

```rust
fn resizable(self) -> View
```

Raster images take whatever size they are offered (otherwise images keep their own size). Panics on views that are not raster images. Call it directly on the `Image`, before other modifiers.

### layout_priority

```rust
fn layout_priority(self, priority: impl Number) -> View
```

Stacks give space to higher priorities first. Default 0. Static number.

```rust
HStack {
    Text("A long title that might need to be cut short").line_limit(1)
    Text("12:30").layout_priority(1) // never cut
}
```

### z_index

```rust
fn z_index(self, z: impl IntoProp<f32>) -> View
```

Stacking order among siblings: higher draws in front and is hit first. Reactive.

### layout_value

```rust
fn layout_value<K: crate::layout::LayoutValueKey>(self, value: K::Value) -> View
```

Attaches a value for the parent custom `Layout` to read with `LayoutChild::value::<K>()`. Not reactive (plain `K::Value`). Define the key with `LayoutValueKey` (`type Value`, `const NAME: Option<&'static str>`, `fn default_value() -> Self::Value`). See the custom layouts reference. Value form: `LayoutValueModifier<K>(pub K::Value)`.

```rust
struct Rank;
impl LayoutValueKey for Rank { type Value = i32; fn default_value() -> i32 { 0 } }
Text("x").layout_value::<Rank>(2)
```

### Grid cells: grid_cell_columns, grid_cell_alignment, grid_column_alignment

```rust
fn grid_cell_columns(self, count: i32) -> View
fn grid_cell_alignment(self, alignment: impl Into<Alignment>) -> View
fn grid_column_alignment(self, alignment: impl Into<crate::geometry::HorizontalAlignment>) -> View
```

Only meaningful inside a `Grid`/`GridRow`: span `count` columns; align this cell instead of its row/column/grid alignment; set the horizontal alignment for every cell in this cell's column. Details in the grids reference.

```rust
Text("Title").grid_cell_columns(2)
Text("Price").grid_column_alignment(.trailing)
```

### Backgrounds and overlays

```rust
fn background(self, style: impl IntoProp<ShapeStyle>) -> View
fn background_in(self, style: impl IntoProp<ShapeStyle>, shape: impl Into<Shape>) -> View
fn background_view(self, background: impl IntoView) -> View
fn background_view_with(self, background: impl IntoView, options: impl FnOnce(DecorationOptions) -> DecorationOptions) -> View
fn overlay(self, overlay: impl IntoView) -> View
fn overlay_with(self, overlay: impl IntoView, options: impl FnOnce(DecorationOptions) -> DecorationOptions) -> View
fn border(self, color: impl IntoProp<Color>, width: impl Number) -> View
fn border_in(self, color: impl IntoProp<Color>, width: impl Number, shape: impl Into<Shape>) -> View
```

| Modifier | What | Use when |
| --- | --- | --- |
| `background(style)` | color or gradient filling the view's bounds (reactive) | simple fills |
| `background_in(style, shape)` | fill clipped to a shape behind the view | capsule/rounded/circle backgrounds |
| `background_view(view)` / `_with` | any view behind, sized to the content | images, stacks behind |
| `overlay(view)` / `overlay_with` | any view in front, sized to the content | badges, strokes: `Rectangle().stroke_border(..)` |
| `border(color, width)` | border drawn inside the bounds (reactive color) | plain borders |
| `border_in(color, width, shape)` | border along a shape | rounded/capsule borders |

Backgrounds and overlays never affect layout. `DecorationOptions` has one field, `alignment: Alignment` (default `CENTER`); in `elg!`: `.overlay(view, alignment: .top_trailing)`.

```rust
Text("3").padding(6).background_in(.danger, Circle()).overlay(Circle().stroke_border(.white, 2))
Image("bell").overlay(Circle().fill(.danger).frame(8, 8), alignment: .top_trailing)
Text("Tag").padding(6).border_in(.outline, 1, Capsule())
```

## Alignment

There is no standalone alignment modifier. Alignment is a parameter:

| Where | How |
| --- | --- |
| `frame` / `frame_with` / `frame_size` | `alignment:` of the content inside the frame |
| `background_view_with`, `overlay_with`, `mask_view_with` | `DecorationOptions.alignment` |
| `rotation_effect_with`, `scale_effect_with`, `scale_effect_xy_with` | `AnchorOptions.anchor` (a `UnitPoint`, pivot) |
| `grid_cell_alignment`, `grid_column_alignment` | grid cells |
| stacks | `VStack(alignment: .leading, ..)` (see stacks reference) |

Types: `Alignment` has constants `CENTER`, `TOP`, `BOTTOM`, `LEADING`, `TRAILING`, `TOP_LEADING`, `TOP_TRAILING`, `BOTTOM_LEADING`, `BOTTOM_TRAILING` and `Alignment::new(HorizontalAlignment, VerticalAlignment)`; in `elg!` write `.top_leading` etc. `UnitPoint` (`CENTER` default, `TOP`, ..., `UnitPoint::new(x, y)`, 0..1 of bounds).

---

## Appearance

### foreground_style, tint, font, font_weight

```rust
fn foreground_style(self, color: impl IntoProp<Color>) -> View
fn tint(self, color: impl IntoProp<Color>) -> View
fn font(self, font: impl IntoProp<Font>) -> View
fn font_weight(self, weight: impl IntoProp<Weight>) -> View
```

Environment modifiers: inherited by everything inside. `foreground_style`: color of text and unfilled shapes (also a theme color; default theme `primary`). `tint`: accent color of controls (default theme `accent`). `font`: default `Font::body`. `font_weight`: weight only, keeps size and family. All reactive.

- Prefer on a container to style all children; use `Text`'s own methods for one text.

```rust
VStack { Text("A") Text("B") }.font(.headline).foreground_style(.secondary).tint(Color::hex(0x7AA2F7))
```

### color_scheme and theme

```rust
fn color_scheme(self, scheme: impl IntoProp<ColorScheme>) -> View
fn theme(self, theme: Theme) -> View
```

`color_scheme`: Light or Dark for theme colors inside (default: the system's). Reactive. `theme(Theme)`: replaces the theme for everything inside (default `Theme::standard()`); not reactive. See theming-and-style.md.

### opacity

```rust
fn opacity(self, opacity: impl IntoProp<f32>) -> View
```

Fades the view as a group (0..1). Reactive and animatable. Forces offscreen rendering of the group.

### Shadows: shadow, inner_shadow

```rust
fn shadow(self, shadow: Shadow) -> View
fn shadow_with(self, shadow: impl FnOnce(Shadow) -> Shadow) -> View
fn inner_shadow(self, shadow: Shadow) -> View
fn inner_shadow_with(self, shadow: impl FnOnce(Shadow) -> Shadow) -> View
```

`shadow`: blurred drop shadow of the content. `inner_shadow`: cast inside the content from its edges inward (pressed-in, embossed). `Shadow` fields (all static, builder methods of the same names; `color` takes `impl Into<Color>`, the rest `impl Number`):

| Field | Type | Default (`shadow`) | Default (`inner_shadow_with` starting point) |
| --- | --- | --- | --- |
| `color` | `Color` | `Color::BLACK.opacity(0.33)` | `Color::BLACK.opacity(0.25)` |
| `radius` | `f32` | 8 | 6 |
| `x` | `f32` | 0 | 0 |
| `y` | `f32` | 0 | 2 |
| `spread` | `f32` | 0 | 0 |

`spread` grows (negative: shrinks) single shapes (rect, rounded rect, capsule, circle) before blurring; ignored for other content. Stack several by chaining `.shadow(..)`.

```rust
Text("Card").shadow(radius: 6, y: 2)
Text("Card").shadow(Shadow::default().color(Color::BLACK.opacity(0.2)).radius(12).y(4))
Text("Well").inner_shadow(radius: 4, y: 2)
```

### Borders and clipping: border, clipped, clip_shape, corner_radius

```rust
fn clipped(self) -> View
fn clip_shape(self, shape: impl Into<Shape>) -> View
fn corner_radius(self, radius: impl Number) -> View
```

(`border`/`border_in` are under [Backgrounds and overlays](#backgrounds-and-overlays).)

- `clipped()`: clip to the view's bounds. Use after `aspect_ratio(mode: .fill)`.
- `clip_shape(shape)`: clip to a shape filling the bounds (`Circle()` avatars).
- `corner_radius(r)`: shorthand for `clip_shape(RoundedRectangle(r))`. Not reactive.
- Clipping does not change the hit area; use `content_shape` for that.

```rust
Image(ImageSource::file(p)).resizable().frame(48, 48).clip_shape(Circle())
VStack { Text("Card") }.corner_radius(16)
```

### mask, mask_view

```rust
fn mask(self, style: impl IntoProp<ShapeStyle>) -> View
fn mask_view(self, mask: impl IntoView) -> View
fn mask_view_with(self, mask: impl IntoView, options: impl FnOnce(DecorationOptions) -> DecorationOptions) -> View
```

Show the content only where the mask is opaque (only its alpha matters). `mask(style)`: a color/gradient filling the bounds; `mask_view`: a view sized and aligned like an overlay. A mask also limits where the view can be clicked: the content is hit only where the mask itself would be hit; the mask never takes the click.

```rust
Text("Fades out").mask(LinearGradient::colors([Color::BLACK, Color::CLEAR]))
```

### Blur and backdrop blur

```rust
fn blur(self, radius: impl IntoProp<f32>) -> View
fn blur_with(self, radius: impl IntoProp<f32>, options: impl FnOnce(BlurOptions) -> BlurOptions) -> View
fn backdrop_blur(self, radius: impl IntoProp<f32>) -> View
fn backdrop_blur_with(self, radius: impl IntoProp<f32>, options: impl FnOnce(BackdropOptions) -> BackdropOptions) -> View
fn backdrop_blur_in(self, radius: impl IntoProp<f32>, shape: impl Into<Shape>, options: impl FnOnce(BackdropOptions) -> BackdropOptions) -> View
```

- `blur`: Gaussian blur of the content, `radius` in points, reactive and animatable. `BlurOptions.opaque: bool` (builder `opaque(bool)`, default false): content fills its bounds, so edges stay sharp instead of fading.
- `backdrop_blur*`: frosted glass: blurs what the window draws behind the view, inside its bounds (or inside `shape`). `BackdropOptions` (all reactive `IntoProp<f32>`): `saturation` (default 1), `brightness` (default 0), `noise` (grain 0..1, default 0). `radius` accepts a `ThemeBlur` as well.
- A `.blur` after a backdrop blur blurs the view and its backdrop together. Inside an offscreen group (opacity below 1, shadow, blur, `compositing_group`) the backdrop only includes what that group drew.

```rust
Text("Now playing").padding(12)
    .background_in(Color::WHITE.opacity(0.5), RoundedRectangle(12))
    .backdrop_blur_in(30, RoundedRectangle(12), |o| o.saturation(1.8).noise(0.02))
Text("x").blur(12, opaque: true)
Text("y").backdrop_blur(30, saturation: 1.8)
```

### Color filters

```rust
fn color_filter(self, filter: ColorFilter) -> View
fn brightness(self, amount: impl IntoProp<f32>) -> View
fn contrast(self, amount: impl IntoProp<f32>) -> View
fn saturation(self, amount: impl IntoProp<f32>) -> View
fn grayscale(self, amount: impl IntoProp<f32>) -> View
fn hue_rotation(self, degrees: impl IntoProp<f32>) -> View
fn sepia(self, amount: impl IntoProp<f32>) -> View
fn color_invert(self) -> View
fn color_invert_amount(self, amount: impl IntoProp<f32>) -> View
fn color_multiply(self, color: impl IntoProp<Color>) -> View
fn luminance_to_alpha(self) -> View
```

Applied to the content's pixels in sRGB with straight alpha (like CSS filter functions). All amounts reactive and animatable. The shorthands each build a `ColorFilter` variant; use `color_filter(ColorFilter::..)` with a prebuilt value.

| Modifier / `ColorFilter` variant | Amount |
| --- | --- |
| `brightness` / `Brightness(Prop<f32>)` | -1 black ... 0 unchanged ... 1 white (added per channel) |
| `contrast` / `Contrast` | 0 flat gray, 1 unchanged, above 1 more contrast |
| `saturation` / `Saturation` | 0 gray, 1 unchanged, above 1 more vivid |
| `grayscale` / `Grayscale` | 0 unchanged ... 1 fully gray |
| `hue_rotation` / `HueRotation` | degrees around the color wheel |
| `sepia` / `Sepia` | 0 unchanged ... 1 fully sepia |
| `color_invert` (amount 1), `color_invert_amount` / `Invert` | 0 unchanged ... 1 inverted |
| `color_multiply` / `Multiply(Prop<Color>)` | multiplies every pixel (alpha too) by the color |
| `luminance_to_alpha` / `LuminanceToAlpha` | brightness becomes opacity (white stays, black vanishes); for masks |

```rust
Image(ImageSource::file(p)).resizable().grayscale(move || if disabled.get() { 1.0 } else { 0.0 })
Text("x").color_filter(ColorFilter::Sepia(Prop::Value(0.5)))
```

### blend_mode and compositing_group

```rust
fn blend_mode(self, mode: impl Into<BlendMode>) -> View
fn compositing_group(self) -> View
```

`blend_mode`: how the view's pixels combine with what is behind (W3C compositing modes). `BlendMode` variants: `Normal` (default), `Multiply`, `Darken`, `ColorBurn`, `PlusDarker`, `Screen`, `Lighten`, `ColorDodge`, `PlusLighter`, `Overlay`, `SoftLight`, `HardLight`, `Difference`, `Exclusion`, `Hue`, `Saturation`, `Color`, `Luminosity`. `compositing_group`: draws the content offscreen first, then composites it as one image, so blend modes inside blend only with each other and a backdrop blur inside sees only what the group drew. Use it to keep a blend mode from reaching the rest of the window.

```rust
Text("Multiply").blend_mode(.multiply)
ZStack { Image("a") Image("b").blend_mode(.screen) }.compositing_group()
```

---

## Transforms: offset, rotation_effect, scale_effect

```rust
fn offset(self, x: impl IntoProp<f32>, y: impl IntoProp<f32>) -> View
fn rotation_effect(self, degrees: impl IntoProp<f32>) -> View
fn rotation_effect_with(self, degrees: impl IntoProp<f32>, options: impl FnOnce(AnchorOptions) -> AnchorOptions) -> View
fn scale_effect(self, scale: impl IntoProp<f32>) -> View
fn scale_effect_with(self, scale: impl IntoProp<f32>, options: impl FnOnce(AnchorOptions) -> AnchorOptions) -> View
fn scale_effect_xy(self, x: impl IntoProp<f32>, y: impl IntoProp<f32>) -> View
fn scale_effect_xy_with(self, x: impl IntoProp<f32>, y: impl IntoProp<f32>, options: impl FnOnce(AnchorOptions) -> AnchorOptions) -> View
```

All move only what is drawn: layout and hit testing are unaffected. All reactive and animatable.

- `offset(x, y)`: points (drags; animates in animated updates).
- `rotation_effect(degrees)`: clockwise about the center.
- `scale_effect(s)`: uniform scale stays sharp (vector). `scale_effect_xy` scales per axis (rendered offscreen first, so prefer uniform).
- `AnchorOptions.anchor: UnitPoint` (default `UnitPoint::CENTER`): pivot, e.g. `anchor: .top_leading`.
- For layout-affecting size changes use `frame`; for position changes that should relayout use stacks/padding.

```rust
Text("Dragged").offset(drag_x, drag_y).z_index(1)
Text("x").rotation_effect(45, anchor: .top_leading)
Text("y").scale_effect(1.2, anchor: .bottom)
Text("z").scale_effect_xy(1.0, move || squash.get())
```

---

## Animation and transitions

```rust
fn animation(self, animation: impl Into<Animation>, value: impl crate::animation::AnimationValue) -> View
fn transition(self, transition: impl Into<Transition>) -> View
fn animates_layout(self, enabled: bool) -> View
```

- `animation(a, value)`: changes inside this view animate with `a` whenever `value` changed in the same update. `value`: a plain `PartialEq` value, `State<T>`, `Computed<T>`, `Binding<T>`, or a closure `Fn() -> T` (`AnimationValue`). Prefer it over `with_animation` when the view should always animate on that value, wherever the change comes from.
- `with_animation(animation, || changes)` (free function, not a modifier): wrap a state change so everything it affects animates, including views entering/leaving.
- `transition(t)`: how the view enters and leaves when `If`/`ForEach` adds or removes it in an animated update. Default: fade (`Transition::opacity()`).
- `animates_layout(false)`: this view and everything inside snap to new positions/sizes (colors, opacity, transitions inside still animate). Default true. Not reactive.

Types (details in state-and-reactivity.md):

- `Animation`: `linear(d)`, `ease_in(d)`, `ease_out(d)`, `ease_in_out(d)`, `spring(response, damping_fraction)`, `smooth(d)`, `custom(curve, d)`, `timing_curve(x1, y1, x2, y2, d)`; `.delay(s)`, `.repeat_count(n, autoreverses)`, `.repeat_forever(autoreverses)`.
- `Transition`: `opacity()`, `identity()`, `scale(from)`, `scale_anchored(from, anchor)`, `offset(x, y)`, `move_edge(edges)`, `slide()`, `asymmetric(insertion, removal)`; `.combined(other)`.
- `Pose` (opacity, scale, x, y, anchor_x, anchor_y): a transition's partway state, from `Transition::pose(progress, removal, size)`.

```rust
Circle()
    .fill(move || if on.get() { Color::hex(0x9ECE6A) } else { Color::hex(0x565F89) })
    .offset(move || if on.get() { 40.0 } else { 0.0 }, 0.0)
    .animation(Animation::spring(0.35, 0.7), on)

Button("Toggle", move || with_animation(Animation::spring(0.5, 0.825), || expanded.toggle()))

Text("Saved").transition(Transition::move_edge(.top).combined(Transition::opacity()))
Text("Loading").rotation_effect(move || if spinning.get() { 360.0 } else { 0.0 })
    .animation(Animation::linear(1.0).repeat_forever(false), spinning)
```

---

## Interaction

### Enabling, hit testing, content shape

```rust
fn disabled(self, disabled: impl IntoProp<bool>) -> View
fn allows_hit_testing(self, enabled: bool) -> View
fn content_shape(self, shape: impl Into<Shape>) -> View
```

- `disabled`: controls inside stop responding and look disabled; handlers and gestures inside (clicks, gestures, keys, typing, focus, context menus, drops) switch off; hover, tooltips and scrolling keep working. A press on a disabled view goes to views around it. Disabling mid-drag cancels the drag; focus moves on. Environment value (`DisabledKey`, default false); reactive. Put it after (outside) the handlers it disables.
- `allows_hit_testing(false)`: the view and everything in it ignore the pointer (clicks fall through to what is behind). Not reactive.
- `content_shape(shape)`: hit test against this shape (filling the bounds) instead of the drawn content: `Rectangle()` makes the whole frame clickable (gaps in an `HStack` with spacers).

```rust
Text("Card").on_click(move |_| open()).disabled(locked)
HStack { Text("Rate") Spacer() }.content_shape(Rectangle()).on_click(move |_| rate())
Text("Overlay").allows_hit_testing(false)
```

### Clicks and activation

```rust
fn on_click(self, action: impl Fn(&ClickEvent) + 'static) -> View
fn on_click_with(self, action: impl Fn(&ClickEvent) + 'static, options: impl FnOnce(ClickOptions) -> ClickOptions) -> View
fn on_double_click(self, action: impl Fn(&ClickEvent) + 'static) -> View
fn on_activate(self, action: impl FnMut() + 'static) -> View
```

- `on_click`: pointer click (pressed and released inside, left button by default). Innermost click handler wins. `ClickEvent` has the location (view coordinates), button, modifier keys, click count. Pointer only.
- `ClickOptions`: `button: MouseButton` (default `Left`; `.middle`, `.right`), `count: i32` (default 1; 2 = double click, the first click still goes to outer handlers).
- `on_double_click`: `on_click` with `count` 2.
- `on_activate`: the view's action however it is used: left click, Space or Enter while focused, or a screen reader's default action. Makes the view focusable; ignored while disabled. Prefer over `on_click` for anything that acts like a control; add an accessibility role and label.

```rust
Text("Row").on_click(move |_| select()).on_double_click(move |_| open())
Text("Tab").on_click(move |_| close_tab(), button: .middle)
Canvas(|_| {}).on_click(|click| add_point(click.location))
Text("Album").on_activate(open).accessibility_label("Blue Train")
```

### Gestures

```rust
fn recognize(self, gesture: impl Gesture) -> View
fn recognize_with(self, gesture: impl Gesture, options: impl FnOnce(GestureOptions) -> GestureOptions) -> View
fn on_drag(self, action: impl Fn(&DragEvent) + 'static) -> View
fn on_long_press(self, action: impl Fn() + 'static) -> View
fn on_magnify(self, action: impl Fn(&MagnifyEvent) + 'static) -> View
fn on_rotate(self, action: impl Fn(&RotateEvent) + 'static) -> View
fn on_two_finger_double_tap(self, action: impl Fn(Point) + 'static) -> View
```

- `recognize(gesture)`: attach any gesture (`DragGesture`, `LongPressGesture`, `MagnifyGesture`, `RotateGesture`, `TwoFingerDoubleTapGesture`; `PhasedGesture` is the began/changed/ended callback interface they share). Use it when you need `on_began`/`on_ended`/`on_cancelled`, or gesture options.
- Shorthands: `on_drag` = `recognize(DragGesture::new().on_changed(a))`; `on_long_press` = `recognize(LongPressGesture::new(a))`; `on_magnify`, `on_rotate` = `on_changed`; `on_two_finger_double_tap(Point)` gets the window-coordinates location.
- `GestureOptions` (for drags and long presses): `priority: GesturePriority` (`Normal` default: innermost recognizing gesture takes the press; `High`: offered the press before gestures inside, may take it from controls that keep presses such as a slider) and `simultaneous: bool` (default false: recognize alongside whatever has the press without taking it; priority then does not apply).
- Platform: rotate and two-finger double tap come from Mac trackpads; pinch (`on_magnify`) also works on other platforms that report it.
- Gesture builder types (`DragOptions`: `minimum_distance`, `button`, `hold`, `hold_tolerance`; `LongPressOptions`) belong to the input/gestures reference.

```rust
Text("Photo")
    .recognize(
        MagnifyGesture::new()
            .on_began(move |_| start.set(zoom.get()))
            .on_changed(move |e| zoom.set((start.get() * e.scale).clamp(0.5, 8.0))),
    )
    .on_rotate(move |e| angle.set(e.degrees))
    .on_two_finger_double_tap(move |_| zoom.set(1.0))
Text("Row").recognize_with(swipe_gesture, priority: .high)
```

### Raw pointer and scroll events

```rust
fn on_pointer(self, action: impl Fn(&PointerEvent) -> bool + 'static) -> View
fn on_pointer_capture(self, action: impl Fn(&PointerEvent) -> bool + 'static) -> View
fn on_scroll(self, action: impl Fn(&ScrollEvent) -> bool + 'static) -> View
fn on_scroll_capture(self, action: impl Fn(&ScrollEvent) -> bool + 'static) -> View
```

Raw events in the view's own coordinates. Return `true` to consume. Consuming a pointer down captures the pointer (moves and the release then come here wherever it goes). For drawing and custom controls; prefer `on_click` and gestures otherwise. A consumed `on_scroll` stops scroll views around it from scrolling (return false to let them). `*_capture` variants see the event on the way in, before views inside (see [Event phases](#event-phases-capture-and-bubble)). `PointerEvent` kinds: down, up, move while captured, hover moves, cancel (`PointerEventKind`).

```rust
Canvas(|_| {}).on_pointer(move |e| { cursor.set(e.location); true })
Canvas(|_| {}).on_scroll(move |e| { if e.modifiers.has(KeyModifiers::primary()) { zoom_by(e.dy, e.location); true } else { false } })
```

### Hover, pointer boundary, pointer style

```rust
fn on_pointer_enter(self, action: impl FnMut() + 'static) -> View
fn on_pointer_leave(self, action: impl FnMut() + 'static) -> View
fn on_hover_change(self, action: impl Fn(bool) + 'static) -> View
fn hover_delay(self, delay: HoverDelay) -> View
fn hover_delay_with(self, delay: impl FnOnce(HoverDelay) -> HoverDelay) -> View
fn pointer_style(self, style: impl IntoProp<PointerStyle>) -> View
```

- `on_pointer_enter` / `on_pointer_leave`: fire immediately as the pointer crosses the hit area: for highlights.
- `on_hover_change(bool)`: hover intent: true after the pointer rests for the hover delay, false when it leaves (after the exit delay): for previews that should not flash as the pointer passes. Default delay 0.
- `hover_delay(HoverDelay)`: environment value for `on_hover_change` handlers and submenus inside (default enter 0, exit 0 seconds). `HoverDelay { enter: f64, exit: f64 }`, builders `enter(seconds)`, `exit(seconds)`. Controls' own highlights and enter/leave never wait.
- `pointer_style(style)`: pointer shape, like CSS `cursor`. Innermost view that sets one wins; views inside may set their own (text fields set the I-beam). Reactive. `PointerStyle`: `Automatic` (default; defers to views around), `Arrow`, `Pointer` (hand), `Text`, `Crosshair`, `Move`, `Wait`, `Progress`, `ResizeHorizontal`, `ResizeVertical`, `ResizeNwse`, `ResizeNesw`, `NotAllowed`. For a style depending on pointer location implement `EventHandler::pointer_style`.

```rust
Text("Row")
    .background(move || if hovered.get() { Color::hex(0x7AA2F7).opacity(0.1) } else { Color::CLEAR })
    .on_pointer_enter(move || hovered.set(true))
    .on_pointer_leave(move || hovered.set(false))
    .pointer_style(.pointer)
Text("Tile").on_hover_change(show_preview).hover_delay(HoverDelay::default().enter(0.5))
Text("Tile").on_hover_change(show_preview).hover_delay_with(|d| d.enter(0.5))
```

### Focus

```rust
fn focusable(self, focusable: bool) -> View
fn focused(self, is_focused: impl IntoBinding<bool>) -> View
```

- `focusable(true)`: the view takes keyboard focus (Tab order). `focusable(false)`: it and everything inside never do. Controls are focusable already; `on_activate` and `on_text_input` also make the view focusable. Not reactive.
- `focused(binding)`: two-way: true while the view has focus; setting it true focuses the view. Pass a `State<bool>` or `Binding<bool>`. A view inside a scroll view is scrolled into sight when it gains focus.

```rust
VStack {
    TextField("Search", query).focused(search_focused)
    Button("Find", move || search_focused.set(true))
}
Canvas(|_| {}).focusable(true).on_key(move |e| { /* .. */ false })
```

### Keyboard and text input

```rust
fn on_key_press(self, key: impl Into<Key>, action: impl FnMut() + 'static) -> View
fn on_key_press_capture(self, key: impl Into<Key>, action: impl FnMut() + 'static) -> View
fn on_key(self, action: impl Fn(&KeyEvent) -> bool + 'static) -> View
fn on_key_capture(self, action: impl Fn(&KeyEvent) -> bool + 'static) -> View
fn on_text_input(self, action: impl Fn(&TextInputEvent) -> bool + 'static) -> View
fn on_text_input_capture(self, action: impl Fn(&TextInputEvent) -> bool + 'static) -> View
```

Keys go to the focused view first, then each view around it until one handles them.

- `on_key_press(Key, action)`: runs when `key` goes down while the view (or something inside) has focus. `Key` variants: `A`..`Z`, `Digit0`..`Digit9`, `Enter`, `Escape`, `Backspace`, `Tab`, `Space`, `ForwardDelete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Left`, `Right`, `Up`, `Down`, `F1`..`F12`, `Menu`, `Unknown`.
- `on_key`: every key down and up (and repeats); return true if handled, else it bubbles out. `KeyEvent` has `kind` (`KeyEventKind::Down`/`Up`), `key`, `modifiers` (`KeyModifiers`; `KeyModifiers::primary()` is Cmd on macOS, Ctrl elsewhere).
- `on_text_input`: makes the view focusable; receives typed text, including an IME's compositions in progress (`TextInputEvent.kind`: `TextInputKind::Commit`, ...; `.text`). Keys still go to `on_key`. Return true if handled.
- `*_capture`: see before the focused view inside (consumes). `on_text_input_capture` does not make the view focusable.

```rust
Text("Editor").on_key_press(Key::Escape, close)
Canvas(|_| {}).focusable(true).on_key(move |e| {
    if e.kind == KeyEventKind::Down && e.key == Key::S && e.modifiers.has(KeyModifiers::primary()) { save(); return true; }
    false
})
Canvas(|_| {}).on_text_input(|e| { if e.kind == TextInputKind::Commit { insert(&e.text); } true })
```

### Drops

```rust
fn on_drop(self, options: DropOptions) -> View
fn on_drop_with(self, options: impl FnOnce(DropOptions) -> DropOptions) -> View
```

Accept files or text dragged in from other apps. `DropOptions` (builder methods, all `'static` closures): `files(Fn(Vec<PathBuf>, Point))`, `text(Fn(String, Point))`, `targeted(Fn(bool))` (true while something the view could take is dragged over it: for a highlight). A view without a `files`/`text` handler does not take that kind. A drop goes to one view: the innermost enabled one under the pointer with a handler for it (topmost among overlapping siblings). Handlers return nothing. Location is in the view's coordinates.

```rust
Text("Drop files here").on_drop(
    DropOptions::default()
        .files(|files, _at| import(files))
        .text(|text, _| paste(text))
        .targeted(move |over| targeted.set(over)),
)
```

### Selection: selection, selection_with

```rust
fn selection<S: crate::views::Selection>(self, selection: S) -> View
fn selection_with<S: crate::views::Selection>(self, selection: S, on_activate: impl Fn(&S::Key) + 'static) -> View
```

Makes the rows of a stack over a `ForEach` selectable by their ForEach keys. Put it on the `VStack`/`HStack`/`LazyVStack`/`LazyHStack` itself (its other children are not rows). Each row is drawn by the environment's `RowStyle` (`.row_style(..)`). `Selection` is implemented for `State<Option<K>>` and `Binding<Option<K>>` (single selection) and `State<BTreeSet<K>>` and `Binding<BTreeSet<K>>` (multi selection (Ctrl/Cmd+click, Shift+click, Ctrl/Cmd+A)); keys are `SelectionKey` (the ForEach key types). `selection_with` adds `on_activate`: Enter, or a double click on a row. Lists reference: lists.md.

```rust
let open = State::new(None::<i64>);
ScrollView(LazyVStack(ForEach(mails, &Mail::id, MailRow)).selection(open))
```

### window_drag_area

```rust
fn window_drag_area(self, enabled: bool) -> View
```

Pressing and dragging here moves the window (custom title bars). Not reactive. See windows reference for `TitleBar`.

### Event phases (capture and bubble)

Events with two phases (pointer down/up/hover, scroll, keys, text input) run capture handlers outermost-first, then bubble handlers innermost-first; the first handler that consumes the event, in either phase, ends it. `EventPhase { Capture, Bubble (default) }`. Capture variants (`on_pointer_capture`, `on_scroll_capture`, `on_key_capture`, `on_key_press_capture`, `on_text_input_capture`) let a parent see events before the views inside.

---

## Lifecycle and size observers

```rust
fn on_appear(self, action: impl FnMut() + 'static) -> View
fn on_disappear(self, action: impl FnMut() + 'static) -> View
fn on_size_change(self, action: impl Fn(PixelSize) + 'static) -> View
#[deprecated] fn on_sized(self, action: impl Fn(PixelSize) + 'static) -> View
```

- `on_appear`: runs once the view is added (its window opened, an `If`/`ForEach` showed it, or a lazy stack built it as it scrolled near), after the update that added it and before that update is drawn. It runs as the view's own: timers and effects started in it end with the view. Pair with `animation(..repeat_forever..)` to start looping animations.
- `on_disappear`: runs once the view is gone (after its exit transition, or when its window closed). Only for views that appeared.
- `on_size_change(PixelSize)`: the view's actual layout size in physical pixels (display scale included), called on the UI thread after the first layout and whenever the physical size changes. Repeated sizes are suppressed per mount; dimensions are rounded up and exclude visual transforms; zero sizes are reported. Runs after layout and before painting: queue the size (channel, `Cell`) as renderer input; do not change layout or rebuild views synchronously from it. Place it inside any padding/frame whose extra space should not count. `PixelSize { width: u32, height: u32 }`.
- `on_sized`: deprecated former name of `on_size_change`; do not use.

```rust
Text("Hello").on_appear(move || started.set(true)).on_disappear(cleanup)
VideoView(feed).on_size_change(move |size| pending.set(Some(size)))
```

---

## Control-style modifiers

Each sets an environment value: the style of every such control inside the view. Argument types are `impl IntoXStyle` (built-in styles, your own style types, or `AnyXStyle`). Built-in styles and writing your own: see theming-and-style.md.

| Modifier | Signature | Affects |
| --- | --- | --- |
| `button_style` | `fn button_style(self, style: impl IntoButtonStyle) -> View` | every Button |
| `toggle_style` | `fn toggle_style(self, style: impl IntoToggleStyle) -> View` | every Toggle |
| `checkbox_style` | `fn checkbox_style(self, style: impl IntoCheckboxStyle) -> View` | every Checkbox |
| `number_input_style` | `fn number_input_style(self, style: impl IntoNumberInputStyle) -> View` | every NumberInput |
| `progress_style` | `fn progress_style(self, style: impl IntoProgressStyle) -> View` | every Progress |
| `text_field_style` | `fn text_field_style(self, style: impl IntoTextFieldStyle) -> View` | every TextField and TextArea |
| `picker_style` | `fn picker_style(self, style: impl IntoPickerStyle) -> View` | every Picker |
| `slider_style` | `fn slider_style(self, style: impl IntoSliderStyle) -> View` | every Slider |
| `navigation_stack_style` | `fn navigation_stack_style(self, style: impl IntoNavigationStackStyle) -> View` | every NavigationStack |
| `popover_style` | `fn popover_style(self, style: impl IntoPopoverStyle) -> View` | popovers opened from inside |
| `tooltip_style` | `fn tooltip_style(self, style: impl IntoTooltipStyle) -> View` | tooltips shown from inside |
| `menu_style` | `fn menu_style(self, style: impl IntoMenuStyle) -> View` | menus opened from inside |
| `row_style` | `fn row_style(self, style: impl IntoRowStyle) -> View` | selectable rows inside (`.selection()`) |

Not reactive (styles are plain values). Built-in style names (full list in theming-and-style.md): `PlainButtonStyle`, `BorderedButtonStyle`, `ProminentButtonStyle`, `SwitchToggleStyle`, `BoxCheckboxStyle`, `BoxNumberInputStyle`, `PlainNumberInputStyle`, `LinearProgressStyle`, `CircularProgressStyle`, `PlainTextFieldStyle`, `RoundedBorderTextFieldStyle`, `MenuPickerStyle`, `SegmentedPickerStyle`, `KnobSliderStyle`, `BarNavigationStackStyle`, `PlainNavigationStackStyle`, `DefaultPopoverStyle`, `PlainPopoverStyle`, `DefaultTooltipStyle`, `PlainTooltipStyle`, `DefaultMenuStyle`, `PlainMenuStyle`, `HighlightRowStyle`, `PlainRowStyle`.

```rust
VStack { Button("Save", save) Button("Cancel", cancel) }.button_style(ProminentButtonStyle)
```

---

## Tooltips, popovers, menus

Component details (Menu, MenuItem, Submenu, popover content, styles) are in components.md; here are the modifiers.

```rust
fn context_menu<V: IntoView>(self, menu: impl Fn() -> V + 'static) -> View
fn tooltip(self, text: impl IntoProp<String>) -> View
fn tooltip_with(self, text: impl IntoProp<String>, options: impl FnOnce(TooltipOptions) -> TooltipOptions) -> View
fn tooltip_view<V: IntoView>(self, content: impl Fn() -> V + 'static, options: impl FnOnce(TooltipOptions) -> TooltipOptions) -> View
fn floating<V: IntoView>(self, is_presented: impl IntoBinding<bool>, content: impl Fn() -> V + 'static, options: impl FnOnce(FloatingOptions) -> FloatingOptions) -> View
fn floating_container(self) -> View
fn popover<V: IntoView>(self, is_presented: impl IntoBinding<bool>, content: impl Fn() -> V + 'static) -> View
fn popover_with<V: IntoView>(self, is_presented: impl IntoBinding<bool>, content: impl Fn() -> V + 'static, options: impl FnOnce(PopoverOptions) -> PopoverOptions) -> View
```

| Modifier | What | Prefer when |
| --- | --- | --- |
| `context_menu(builder)` | right click opens the menu the closure builds, usually `Menu { .. }` | per-item menus; also disabled views do not open it |
| `tooltip(text)` | text tooltip after the pointer rests (delay 0.6 s default); static or reactive | icon buttons |
| `tooltip_with(text, opts)` | same with options (`tooltip("Settings", delay: 0.3)`) | custom delay/placement |
| `tooltip_view(builder, opts)` | tooltip of any content in the environment's `TooltipStyle`. Screen readers do not see it | rich tooltips; always pass an options closure (`\|o\| o`) |
| `popover(shown, builder)` | panel of content beside the view while `shown` | anchored panels |
| `popover_with(.., opts)` | same with options (`edge: .trailing`) | placement/padding/motion |
| `floating(shown, builder, opts)` | content floating above everything by the view, with no chrome | custom popups; you control dismissal/placement |
| `floating_container()` | marks where `FloatingBoundary::Container` content inside stays | clipped, scroll-following popups |

`popover` binding: with a `State<bool>`/`Binding<bool>`, dismissing it sets it back to false. With a closure or `Computed` (`move || todos.read().is_empty()`, `todos.map(Vec::is_empty)`) it is shown while the condition holds, and dismissing closes it until the condition next becomes true. The content builder runs each time it opens.

Options (all have builder methods of the same names):

| Struct | Fields | Defaults |
| --- | --- | --- |
| `TooltipOptions` | `placement: FloatingPlacement`, `boundary: FloatingBoundary`, `delay: f64` (seconds), `motion: Option<PopupMotion>` | spacing 4 below the view; `Screen`; 0.6 s; style's motion |
| `PopoverOptions` | `placement: FloatingPlacement`, `boundary: FloatingBoundary`, `padded: Prop<bool>` (`IntoProp<bool>`; false: content reaches panel edges), `motion: Option<PopupMotion>`; extra builder `edge(FloatingEdge)` | centered below, spacing 4; `Screen`; padded true; style's motion |
| `FloatingOptions` | `placement: FloatingPlacement`, `boundary: FloatingBoundary`, `dismiss: FloatingDismiss`, `motion: Option<PopupMotion>`; builders `placement(\|p\| ..)`, `boundary`, `outside_click(bool)`, `escape(bool)`, `motion` | `FloatingPlacement::default()`; `Screen`; dismiss on outside click and Esc; no motion |
| `FloatingPlacement` | `edge: FloatingEdge`, `alignment: FloatingAlignment`, `spacing: f32`, `offset: Point`, `position: Option<Point>` (top-leading corner in anchor coordinates, instead of an edge) | `Bottom`, `Start`, 0, zero, none |
| `FloatingDismiss` | `outside_click: bool` (also makes it take the pointer while open), `escape: bool` | both true |
| `PopupMotion` | `PopupMotion::new(transition, animation)`, `PopupMotion::none()`; accessors `transition()`, `animation()`, `animates()` | none (built-in styles' default) |

Enums: `FloatingEdge { Bottom (default), Top, Trailing, Leading }` (flips to the opposite side when there is no room); `FloatingAlignment { Start (default), Center, End }`; `FloatingBoundary { Screen (default: popup window of its own, may extend past the app window), Window (inside the window, above content), Container (inside the nearest `.floating_container()` view: clipped, moves with scroll) }`.

```rust
Button(action: open) { Image("gear") }.tooltip("Settings")
Image("gear").tooltip("Settings", delay: 0.3)
Text("Item").context_menu(|| Menu { MenuItem("Delete", delete) })
Button("Edit", move || editing.set(true)).popover(editing, || Text("Hello"), edge: .top)
Text("Anchor").floating(shown, || Text("Floating"), |o| o.outside_click(false).boundary(FloatingBoundary::Window))
```

---

## Accessibility

```rust
fn accessibility_label(self, label: impl IntoProp<String>) -> View
fn accessibility_value(self, value: impl IntoProp<String>) -> View
fn accessibility_hint(self, hint: impl IntoProp<String>) -> View
fn accessibility_hidden(self, hidden: bool) -> View
fn accessibility_role(self, role: impl Into<AccessibilityRole>) -> View
fn accessibility_element(self) -> View
fn accessibility_action(self, action: impl FnMut() + 'static) -> View
```

| Modifier | Use |
| --- | --- |
| `accessibility_label` | what it is called; required for icon-only buttons. Reactive |
| `accessibility_value` | what it holds, in words ("3 of 10"). Reactive |
| `accessibility_hint` | what using it does, if the label does not say. Reactive |
| `accessibility_hidden(true)` | leave it and its contents out (decoration; also hide decorative `Separator`s) |
| `accessibility_role(role)` | override the automatic role. `AccessibilityRole`: `Group`, `Label`, `Heading`, `Button`, `Link`, `Image`, `Toggle`, `Checkbox`, `Slider`, `Progress`, `TextField`, `List`, `ListItem`, `Tab`, `MenuItem` |
| `accessibility_element()` | one element for everything inside, labeled with their text (rows read as one line) |
| `accessibility_action(f)` | what a screen reader's default action does, for views made clickable with gestures. Does not run while the view is disabled |

Built-in views already describe themselves. Text that is not valid UTF-8 is cleaned up (replacement character). Not yet supported: announcing changes, text cursor/selection in fields, reduced-motion and high-contrast settings.

```rust
Button(action: remove) { Image("trash") }.accessibility_label("Delete")
ZStack(alignment: .leading) { Capsule().fill(track) Circle().fill(.white).frame(28, 28) }
    .frame(52, 32)
    .on_activate(move || dark.toggle())
    .accessibility_element()
    .accessibility_role(.toggle)
    .accessibility_label("Dark mode")
    .accessibility_value(move || if dark.get() { "On" } else { "Off" })
```

---

## Environment modifiers: environment

```rust
fn environment<K: EnvironmentKey>(self, value: impl IntoProp<K::Value>) -> View
```

Sets any environment key for the view and everything inside (nearest value above wins). `value` is reactive (State, closure). Built-in keys and their dedicated modifiers: `FontKey` (`font`), `ForegroundKey` (`foreground_style`), `TintKey` (`tint`), `ColorSchemeKey` (`color_scheme`), `ThemeKey` (`theme`), `DisabledKey` (`disabled`), `HoverDelayKey` (`hover_delay`), the `*StyleKey`s (the `*_style` modifiers), `RowSelectedKey`, `WindowKey` (set by the window).

Custom keys: implement `EnvironmentKey` (`type Value: Clone + 'static`; `const NAME: Option<&'static str> = None`; `fn default_value(&EnvironmentValues) -> Self::Value`). Give it a `NAME` and a `Value` of `bool`, `i32`, `i64`, `f32`, `f64`, `Color` or `String` to share it by name with C++ keys. Read with `Environment::<Key>::new()` in components (see state-and-reactivity.md / environment docs).

```rust
struct CompactKey;
impl EnvironmentKey for CompactKey {
    type Value = bool;
    const NAME: Option<&'static str> = Some("myapp.compact");
    fn default_value(_: &EnvironmentValues) -> bool { false }
}
Text("Settings").environment::<CompactKey>(true)
Text("Settings").environment::<CompactKey>(compact_state)   // follows the state
```

---

## Generic entry points: modifier, apply

```rust
fn modifier(self, modifier: impl crate::modifier::Modifier) -> View
fn apply<V: IntoView>(self, function: impl FnOnce(View) -> V) -> View
```

- `modifier(m)`: applies any `Modifier`: a built-in modifier value, your own `ViewModifier`, or a wrapped protocol modifier (`AsLayout`, `AsDecoration`, ...). Use for reusable bundles of modifiers.
- `apply(f)`: `v.apply(card) == card(v)`; keeps a chain a chain.

```rust
struct Card;
impl ViewModifier for Card {
    fn body(&self, content: View) -> View {
        content.padding(16).background_in(ThemeColor::Surface, RoundedRectangle(12))
    }
}
Text("Hello").modifier(Card)
Text("Hello").modifier(PaddingModifier(EdgeInsets::all(8)))   // == .padding(8)
let card = |c: View| c.padding(16).background_in(.surface, RoundedRectangle(12));
elg! { Text("Inside a card").apply(card).shadow(Shadow::default()) }
```

### Modifier traits

| Item | Definition | Use |
| --- | --- | --- |
| `Modifier` | `trait Modifier { fn apply(self, content: View) -> View; }` | anything `.modifier()` takes. Blanket-implemented for every `ViewModifier` |
| `ViewModifier` | `trait ViewModifier: 'static { fn body(&self, content: View) -> View; }` | compose other modifiers; `body` runs once, when the view is mounted. Most custom modifiers |
| `LayoutModifier` | `size_that_fits(&self, proposal: ProposedSize, content: &LayoutChild) -> Size`, `place(&self, bounds: Rect, proposal: ProposedSize, content: &LayoutChild)`, optional `layout_priority(&self) -> Option<f32>` | takes part in layout, wrapping one child. Pass as `AsLayout(m)` |
| `DecorationModifier` | `decoration(&self) -> View`, `placement(&self) -> Placement`, optional `alignment(&self) -> Alignment` (default `CENTER`) | a secondary view sized to the content; no layout effect. Pass as `AsDecoration(m)` |
| `EnvironmentModifier` | `apply(&self, values: &mut EnvironmentValues)` | change inherited values for everything inside. Pass as `AsEnvironment(m)` |
| `EffectModifier` | `apply(&self, effects: &mut Effects)` | change drawing and hit testing. Pass as `AsEffect(m)` |
| `EventModifier` | `make_handler(&self) -> Box<dyn EventHandler>` | handle input; one handler per mounted view. Pass as `AsEvent(m)` |
| `AsLayout<M>`, `AsDecoration<M>`, `AsEnvironment<M>`, `AsEffect<M>`, `AsEvent<M>` | tuple wrappers `pub struct AsX<M>(pub M)` | select the protocol when passing a non-`ViewModifier` to `.modifier(..)` |
| `Placement` | `enum Placement { Behind (default), InFront, Mask }` | where a `DecorationModifier`'s view goes: background, overlay, or mask (alpha only, not drawn) |
| `Effects` | handle valid during `EffectModifier::apply`; setters: `set_clip_shape(Option<Shape>)`, `set_opacity(impl IntoProp<f32>)`, `set_shadow(Option<Shadow>)`, `set_inner_shadow(Option<Shadow>)`, `set_offset(x, y)`, `set_z_index(Option<Prop<f32>>)`, `set_content_shape(Option<Shape>)`, `set_blur(Option<Prop<f32>>, opaque: bool)`, `set_color_filter(Option<ColorFilter>)`, `set_blend_mode(BlendMode)`, `set_compositing_group(bool)`, `set_rotation(Option<Prop<f32>>)`, `set_scale(Option<Prop<f32>>, Option<Prop<f32>>)`, `set_anchor(UnitPoint)`, `set_allows_hit_testing(bool)`, `add_on_size_change(Fn(PixelSize))`; plus matching getters | drawing and hit testing, not layout |
| `EventHandler` | trait, every method has an ignoring default: `pointer(&mut self, &PointerEvent, frame: Rect) -> EventResult`, `hover(&mut self, inside: bool)`, `pointer_style(&self, Point) -> PointerStyle`, `scroll(&mut self, &ScrollEvent, frame: Rect)`, `magnify`, `rotate`, `two_finger_double_tap(Point)`, `key(&mut self, &KeyEvent)`, `focusable(&self) -> bool`, `focus(&mut self, bool)`, `accepts_text_input(&self) -> bool`, `text_input(&mut self, &TextInputEvent)`, `hover_delay(&self) -> Option<HoverDelay>`, `covers_frame(&self) -> bool`, `phase(&self) -> EventPhase` | full low-level input for a mounted view. Return `EventResult::IGNORED` (default) or a consumed result |

### Built-in modifier value types

Each equals the corresponding `ViewExt` method, for `.modifier(...)`; useful for storing/composing modifiers as values.

| Type | Fields | Equals |
| --- | --- | --- |
| `PaddingModifier` | `(EdgeInsets)` | `.padding_insets(..)` |
| `FrameModifier` | `(FrameOptions)` | `.frame_with(..)` |
| `FixedSizeModifier` | `horizontal: bool, vertical: bool` | `.fixed_size_axes(..)` |
| `AspectRatioModifier` | `ratio: Option<f32>, mode: ContentMode` | `.aspect_ratio_with(..)` |
| `LayoutPriorityModifier` | `(f32)` | `.layout_priority(..)` |
| `LayoutValueModifier<K>` | `(K::Value)` | `.layout_value::<K>(..)` |
| `ZIndexModifier` | `(Prop<f32>)` | `.z_index(..)` |
| `OffsetModifier` | `x: Prop<f32>, y: Prop<f32>` | `.offset(..)` |
| `BackgroundModifier` | `view: View, alignment: Alignment` | `.background_view_with(..)` |
| `OverlayModifier` | `view: View, alignment: Alignment` | `.overlay_with(..)` |
| `MaskModifier` | `view: View, alignment: Alignment` | `.mask_view_with(..)` |
| `ClipShapeModifier` | `(Shape)` | `.clip_shape(..)` |
| `OpacityModifier` | `(Prop<f32>)` | `.opacity(..)` |
| `ShadowModifier` | `(Shadow)` | `.shadow(..)` |
| `InnerShadowModifier` | `(Shadow)` | `.inner_shadow(..)` |
| `BlurModifier` | `radius: Prop<f32>, opaque: bool` | `.blur_with(..)` |
| `ColorFilterModifier` | `(ColorFilter)` | `.color_filter(..)` |
| `BlendModeModifier` | `(BlendMode)` | `.blend_mode(..)` |
| `CompositingGroupModifier` | unit struct | `.compositing_group()` |
| `RotationModifier` | `degrees: Prop<f32>, anchor: UnitPoint` | `.rotation_effect_with(..)` |
| `ScaleModifier` | `x: Prop<f32>, y: Prop<f32>, anchor: UnitPoint` | `.scale_effect_xy_with(..)` |
| `ContentShapeModifier` | `(Shape)` | `.content_shape(..)` |
| `AllowsHitTestingModifier` | `(bool)` | `.allows_hit_testing(..)` |
| `FocusableModifier` | `(bool)` | `.focusable(..)` |
| `ClickModifier` | `action: Box<dyn Fn(&ClickEvent)>, options: ClickOptions` | `.on_click_with(..)` |
| `PointerBoundaryModifier` | `enter: Box<dyn FnMut()>, leave: Box<dyn FnMut()>` | `.on_pointer_enter` + `.on_pointer_leave` |
| `HoverModifier` | `(Box<dyn Fn(bool)>)` | `.on_hover_change(..)` |
| `PointerModifier` | `action: Box<dyn Fn(&PointerEvent) -> bool>, phase: EventPhase`; `PointerModifier::new(action)` (bubble) | `.on_pointer` / `.on_pointer_capture` |
| `ScrollModifier` | `action: Box<dyn Fn(&ScrollEvent) -> bool>, phase: EventPhase`; `ScrollModifier::new(action)` | `.on_scroll` / `.on_scroll_capture` |
| `TextInputModifier` | `action: Box<dyn Fn(&TextInputEvent) -> bool>, phase: EventPhase`; `TextInputModifier::new(action)` | `.on_text_input` / `.on_text_input_capture` |
| `KeyEventModifier` | `action: Box<dyn Fn(&KeyEvent) -> bool>, phase: EventPhase`; `KeyEventModifier::new(action)` | `.on_key` / `.on_key_capture` |
| `DragGestureModifier` | `gesture: DragGesture, options: GestureOptions` | `.recognize_with(drag, ..)` |
| `LongPressGestureModifier` | `gesture: LongPressGesture, options: GestureOptions` | `.recognize_with(long_press, ..)` |
| `MagnifyGestureModifier` | `(MagnifyGesture)` | `.recognize(..)` |
| `RotateGestureModifier` | `(RotateGesture)` | `.recognize(..)` |
| `TwoFingerDoubleTapModifier` | `(TwoFingerDoubleTapGesture)` | `.recognize(..)` |
| `PointerStyleModifier` | `(Prop<PointerStyle>)` | `.pointer_style(..)` |
| `EnvironmentKeyModifier<K>` | `(Prop<K::Value>)` | `.environment::<K>(..)` |

Type aliases of `EnvironmentKeyModifier<..Key>` (value is `Prop<..>` of the key's value): `FontModifier` (`.font`), `ForegroundStyleModifier` (`.foreground_style`), `TintModifier` (`.tint`), `ColorSchemeModifier` (`.color_scheme`), `ThemeModifier` (`.theme`), `DisabledModifier` (`.disabled`), `HoverDelayModifier` (`.hover_delay`), `ButtonStyleModifier`, `ToggleStyleModifier`, `TextFieldStyleModifier`, `CheckboxStyleModifier`, `NumberInputStyleModifier`, `ProgressStyleModifier`, `PickerStyleModifier`, `SliderStyleModifier`, `NavigationStackStyleModifier`, `PopoverStyleModifier`, `TooltipStyleModifier`, `MenuStyleModifier`, `RowStyleModifier` (the matching `*_style` modifiers).

Not every `ViewExt` method has a value type (e.g. `corner_radius`, `border`, `tooltip`, `popover`, `on_appear`, `animation`, `transition`, accessibility modifiers); use the method.

---

## Supporting types

| Type | Fields / variants | Notes |
| --- | --- | --- |
| `FrameOptions` | `width`, `height`, `min_width`, `ideal_width`, `max_width`, `min_height`, `ideal_height`, `max_height`: `Len`; `alignment: Alignment` | all unset by default; builder methods of the same names (`impl Into<Len>`, `impl Into<Alignment>`) |
| `Len` | `Len(pub Option<f32>)` | from any `Number` or `Option<f32>`; `INFINITY` / `.infinity` for flexible frames |
| `DecorationOptions` | `alignment: Alignment` | background/overlay/mask placement |
| `AspectRatioOptions` | `ratio: Option<f32>` (None: content's own), `mode: ContentMode` (default `Fit`) | |
| `BlurOptions` | `opaque: bool` | |
| `BackdropOptions` | `saturation: Prop<f32>` (1), `brightness: Prop<f32>` (0), `noise: Prop<f32>` (0) | |
| `AnchorOptions` | `anchor: UnitPoint` (center) | rotation/scale pivot |
| `Shadow` | `color: Color`, `radius`, `x`, `y`, `spread` (`f32`) | see [Shadows](#shadows-shadow-inner_shadow) |
| `ClickOptions` | `button: MouseButton`, `count: i32` | |
| `HoverDelay` | `enter: f64`, `exit: f64` | seconds |
| `GestureOptions` | `priority: GesturePriority`, `simultaneous: bool` | |
| `DropOptions` | `files`, `text`, `targeted` (`Option<Box<dyn Fn..>>`) | |
| `TooltipOptions`, `PopoverOptions`, `FloatingOptions`, `FloatingPlacement`, `FloatingDismiss`, `PopupMotion`, `FloatingEdge`, `FloatingAlignment`, `FloatingBoundary` | see [Tooltips, popovers, menus](#tooltips-popovers-menus) | |
| `AccessibilityRole` | see [Accessibility](#accessibility) | |
| `BlendMode`, `ColorFilter` | see [Appearance](#appearance) | |
| `EdgeInsets`, `Edges`, `Alignment`, `UnitPoint`, `ContentMode`, `Axis`, `Size`, `Point`, `Rect`, `ProposedSize` | geometry | in the geometry reference |

Named arguments in `elg!` compile for every options struct that has builder methods of the field names (the docs use `on_click(.., button: .middle)`, `blur(12, opaque: true)`, `on_drop` via `DropOptions::default().files(..)`). Structs marked `#[elg_builder]` in the source (`FrameOptions`, `DecorationOptions`, `AspectRatioOptions`, `AnchorOptions`, `BackdropOptions`, `Shadow`, `EdgeInsets`, `GestureOptions`, `FloatingOptions`, `TooltipOptions`, `PopoverOptions`, `FloatingPlacement`) also give leading-dot completion of enum-valued arguments; `BlurOptions`, `ClickOptions`, `HoverDelay` and `DropOptions` are plain builders without that attribute (in the version read).

---

## C++-only / not exposed in Rust

- C++ `padding()` with no argument (16 points). Rust always takes a length.
- C++ `Animation::spring()` with no arguments and `repeat_*` autoreverse default true; Rust requires explicit arguments (`spring(response, damping)`, `repeat_count(n, autoreverses)`).
- C++ `focusable()` with no argument defaults to true; Rust `focusable(bool)` is explicit.
- C++ `accessibility_hidden()` takes no argument; Rust `accessibility_hidden(bool)`.
- C++ `with_modifier` / `*_impl` helpers are internal.
- No `.help(...)` modifier exists in either language, although a table in `guides/views-and-modifiers.mdx` lists `help` under Popups; use `tooltip`.
- C++ view-level modifiers all have Rust counterparts (set compared mechanically: only internal `*_impl`/`with_modifier` differ).

## Checklist of every ViewExt method

padding, padding_edges, padding_insets, padding_with, frame, frame_with, frame_size, fixed_size, fixed_size_axes, aspect_ratio, aspect_ratio_with, resizable, layout_priority, z_index, offset, grid_cell_columns, grid_cell_alignment, grid_column_alignment, background, background_in, background_view, background_view_with, overlay, overlay_with, border, border_in, clipped, clip_shape, corner_radius, opacity, shadow, shadow_with, inner_shadow, inner_shadow_with, blur, blur_with, backdrop_blur, backdrop_blur_with, backdrop_blur_in, color_filter, brightness, contrast, saturation, grayscale, hue_rotation, sepia, color_invert, color_invert_amount, color_multiply, luminance_to_alpha, blend_mode, compositing_group, rotation_effect, rotation_effect_with, scale_effect, scale_effect_with, scale_effect_xy, scale_effect_xy_with, mask, mask_view, mask_view_with, content_shape, allows_hit_testing, on_click, on_click_with, on_double_click, on_appear, on_disappear, on_activate, recognize, recognize_with, on_drag, on_long_press, on_magnify, on_rotate, on_two_finger_double_tap, on_pointer, on_pointer_capture, on_scroll, on_scroll_capture, pointer_style, on_size_change, on_sized, on_pointer_enter, on_pointer_leave, on_hover_change, on_drop, on_drop_with, accessibility_label, accessibility_value, accessibility_hint, accessibility_hidden, accessibility_role, accessibility_element, accessibility_action, hover_delay, hover_delay_with, window_drag_area, context_menu, tooltip, tooltip_with, tooltip_view, floating, floating_container, popover, popover_with, focusable, focused, on_key_press, on_key_press_capture, on_key, on_key_capture, on_text_input, on_text_input_capture, selection, selection_with, font_weight, font, foreground_style, tint, color_scheme, theme, disabled, button_style, toggle_style, checkbox_style, number_input_style, progress_style, text_field_style, picker_style, slider_style, navigation_stack_style, popover_style, tooltip_style, menu_style, row_style, environment, animation, transition, animates_layout, layout_value, modifier, apply
