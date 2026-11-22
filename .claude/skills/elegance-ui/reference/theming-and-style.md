# Theming and style (Elegance, Rust)

Colors, themes, light/dark, accent, fonts, icons, control styles, window materials, backdrop blur, platform conventions.

## Contents

- [Quick decision table](#quick-decision-table)
- [Colors](#colors): [Color](#color), [Theme colors](#themecolor-and-colortokens), [Fixed vs semantic](#semantic-colors-vs-fixed-colors-guidance)
- [Resolving colors reactively](#resolving-colors-reactively-prop-resolvecolor-resolvestyle-propfrom_env)
- [Themes](#themes): [Theme](#theme), [ColorTokens](#colortokens), [BlurTokens / ThemeBlur](#blurtokens-and-themeblur), [custom theme](#how-to-build-a-custom-theme)
- [Light and dark](#light-and-dark-mode): [ColorScheme](#colorscheme), [follow system](#how-to-follow-system-light-dark-and-accent), [system_accent_color](#system_accent_color)
- [Tint, foreground, font modifiers](#tint-foreground-style-font-modifiers)
- [Default palettes](#default-palettes-themestandard)
- [Shape styles, gradients, backdrop](#shapestyle-gradients-backdropfill)
- [Backdrop blur as a styling tool](#backdrop-blur-as-a-styling-tool)
- [Fonts](#fonts): [Font](#font), [Weight](#weight), [text styles](#text-styles), [register_font*](#registering-fonts), [platform fonts](#platform-default-fonts)
- [Icons](#icons): [Image/Label](#image-and-label-icons), [icon sets](#custom-icon-sets), [other image types](#imagesource-asyncimage-and-the-image-cache)
- [Control styles](#control-styles): [overview](#overview-and-modifiers), [restyle one vs subtree](#restyle-one-control-vs-all-controls-under-a-subtree), [writing a style](#how-to-write-a-custom-style), [each family](#each-style-family), [Any*Style / Into*Style](#anystyle-and-intostyle), [PopupMotion](#popupmotion), [style environment keys](#environment-keys)
- [Environment (theme/scheme keys)](#environment-theme-and-scheme-keys)
- [Window appearance](#window-appearance): [WindowDesc](#windowdesc-appearance-fields), [TitleBar](#titlebar), [WindowMaterial](#windowmaterial), [transparency](#transparency-and-opacity), [WindowControls](#windowcontrols)
- [Platform conventions](#platform-conventions)
- [C++-only / not exposed in Rust](#c-only--not-exposed-in-rust)
- [Doc-vs-code notes](#doc-vs-code-notes)

All items are in `elegance::prelude::*` (via `elegance::*` re-exports) unless noted. Examples use `elg!`; leading-dot values (`.accent`, `.bold`, `.title`) convert to the parameter's type.

---

## Quick decision table

| I want to... | Use |
|---|---|
| Text/background that is right in light and dark | A `ThemeColor` (`.foreground_style(.secondary)`, `.background(.surface)`) |
| A brand color that must stay the same | `Color::hex(0x7C3AED)` (fixed); for light/dark variants, put it in a custom `Theme` |
| Change the accent of one control/subtree | `.tint(color)` |
| Follow the system accent | `.tint(\|\| system_accent_color().unwrap_or(Color::hex(0x0A84FF)))` |
| Force a subtree light/dark | `.color_scheme(.dark)` / closure |
| Restyle all colors for the app | `Theme::standard()` + edits, then `.theme(theme)` at the root |
| Transparent variant of a theme color | `Prop::from_env(\|env\| ShapeStyle::Color(env.color(ThemeColor::Surface).opacity(0.9)))` |
| Different font size/weight | `.font(.title)`, `.font_weight(.bold)`, `Font::system(17.0, Weight::Bold)` |
| Own font file | `register_font(...)` then `Font::custom("Family", size, weight)` |
| Icon | `Image("gear")`, `Label("Title", "gear")` (Phosphor, follows font and foreground) |
| Own icon font | `register_icon_set(name, IconSetOptions { .. })` |
| Different look for every Button in a form | `.button_style(BorderedButtonStyle)` on the container |
| Different look for one button | `.button_style(...)` on that button |
| Completely new look for a control | Implement the `*Style` trait; pass the value to the modifier |
| Frosted glass panel | `.background_in(.glass_regular, shape).backdrop_blur_in(.xl, shape, \|o\| ...)` |
| Whole-window translucent system backdrop | `WindowDesc::material(WindowMaterial::Sidebar)` (macOS, Windows 11 only) |
| Rounded/shaped window, own title bar | `WindowDesc::transparent(true).title_bar(TitleBar::Hidden)` + `WindowControls` |
| Platform-dependent code | `current_platform()` (`Platform::Windows`/`MacOS`/`Linux`/`Mikros`, or `Unknown(i32)`; non-exhaustive) |
| Shortcut that is Cmd on Mac, Ctrl elsewhere | `KeyboardShortcut::new(Key::S)` / `KeyModifiers::primary()` |

---

## Colors

### Color

```rust
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }   // sRGB, straight alpha, 0...1; Default = BLACK
Color::BLACK / Color::WHITE / Color::CLEAR                            // consts
pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self
pub const fn rgb(red: u8, green: u8, blue: u8) -> Self
pub const fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> Self
pub const fn hex(rrggbb: u32) -> Self                                 // Color::hex(0x0A84FF)
pub const fn hex_alpha(rrggbb: u32, alpha: f32) -> Self
pub fn opacity(self, alpha: f32) -> Self                              // MULTIPLIES the existing alpha
pub fn mix(self, other: Color, amount: f32) -> Self                   // linear blend incl. alpha; amount 0 = self, 1 = other
```

Fixed (non-themed) colors. `opacity` multiplies: `Color::BLACK.opacity(0.3)` is black at 30%; `.opacity(0.5)` on a half-transparent color gives 25%. There is no `with_alpha`, `darker`, or `lighter` in Rust: darken with `c.mix(Color::BLACK, 0.2)`, lighten with `c.mix(Color::WHITE, 0.2)`.

```rust
let blue = Color::hex(0x7AA2F7);
let faint = Color::hex_alpha(0x7AA2F7, 0.5);
let shadow = Color::BLACK.opacity(0.3);
let lighter = Color::hex(0x7AA2F7).mix(Color::WHITE, 0.2);
```

### ThemeColor and ColorTokens

```rust
pub enum ThemeColor {
    Background, Surface, SurfaceRaised, Outline, Primary, Secondary, Tertiary,
    Accent, OnAccent, Danger, Success, Warning,
    GlassUltraThin, GlassThin, GlassRegular, GlassThick, GlassUltraThick,
}
```

In `elg!` use the dot form (snake_case): `.background`, `.surface`, `.surface_raised`, `.outline`, `.primary`, `.secondary`, `.tertiary`, `.accent`, `.on_accent`, `.danger`, `.success`, `.warning`, `.glass_ultra_thin`, `.glass_thin`, `.glass_regular`, `.glass_thick`, `.glass_ultra_thick`. In plain Rust, `ThemeColor::SurfaceRaised`. A `ThemeColor` is accepted anywhere a color or shape style is (`foreground_style`, `tint`, `fill`, `background`, `background_in`, style options such as `SwitchToggleStyle::off`).

| ThemeColor | Meaning | Use for |
|---|---|---|
| `Background` | window background | root `.background(.background)`, page canvas |
| `Surface` | cards, panels | grouped content sitting on the background |
| `SurfaceRaised` | controls and panels on surfaces | input boxes, raised buttons, popovers, checkbox empty fill |
| `Outline` | borders, separators | hairlines, borders, slider/progress tracks, switch track off, hover highlight |
| `Primary` | main text and icons | default foreground (`ForegroundKey` default) |
| `Secondary` | supporting text | captions, metadata |
| `Tertiary` | hints, disabled text | placeholders, turned-off text, inactive icons |
| `Accent` | the tint | prominent buttons, selection, focus (`TintKey` default) |
| `OnAccent` | text/icons on the accent | label inside a filled accent button / selected row |
| `Danger` / `Success` / `Warning` | status | destructive actions, confirmations, cautions |
| `GlassUltraThin` .. `GlassUltraThick` | translucent frosted-glass tints, thinnest (most transparent) to thickest; light in light mode, dark in dark | background under `.backdrop_blur*` |

Pick by purpose, not by how it looks: `Secondary` for de-emphasised text, `Outline` for lines, `Surface` vs `SurfaceRaised` by elevation (raised sits on surface).

```rust
Text("Details").foreground_style(.secondary)
RoundedRectangle(12).fill(.surface)
VStack { Text("Content") }.background(.background)
```

```rust
pub struct ColorTokens {          // one scheme's tokens; pub fields, snake_case
    pub background: Color, pub surface: Color, pub surface_raised: Color, pub outline: Color,
    pub primary: Color, pub secondary: Color, pub tertiary: Color,
    pub accent: Color, pub on_accent: Color, pub danger: Color, pub success: Color, pub warning: Color,
    pub glass_ultra_thin: Color, pub glass_thin: Color, pub glass_regular: Color,
    pub glass_thick: Color, pub glass_ultra_thick: Color,
}
pub fn get(&self, token: ThemeColor) -> Color      // also `tokens[ThemeColor::Accent]` (Index)
```

### Semantic colors vs fixed colors: guidance

- Default to `ThemeColor` for UI chrome and text: it flips with light/dark and with a custom `Theme`; you never branch on the scheme.
- Use fixed `Color` for content colors that carry meaning independent of theme (a user-picked label color, a chart series, a brand logo, photo overlays, shadow `Color::BLACK.opacity(0.2)`).
- Need a themed color with tweaks (alpha, mixed)? Resolve it from the environment (next section); do not hard-code a light-mode hex.
- A brand accent that must differ per scheme belongs in the `Theme` (`theme.light.accent`, `theme.dark.accent`), or use `.tint(...)` for a subtree.
- Never compare `Color` values to guess the scheme; read `ColorSchemeKey`/`env.color_scheme()` if you must branch.
- Text on a filled accent: `.on_accent`, not white.

## Resolving colors reactively (Prop, ResolveColor, ResolveStyle, Prop::from_env)

```rust
pub trait ResolveColor: 'static { fn resolve(self, environment: &EnvironmentValues) -> Color; }
pub trait ResolveStyle: 'static { fn resolve_style(self, environment: &EnvironmentValues) -> ShapeStyle; }
Prop::from_env(f: impl Fn(&EnvironmentValues) -> T + 'static) -> Prop<T>     // Prop<T> has `Prop::Theme(ThemeColor)`, `Prop::Value`, `Prop::Env`
pub fn color(&self, token: impl Into<ThemeColor>) -> Color                    // EnvironmentValues::color: tracked read
```

`Color`, `ThemeColor`, dot markers, gradients and `BackdropFill` implement these; so a closure `|| cond.get()` returning a `Color` or `ThemeColor` is a valid reactive color/style argument (`IntoProp<Color>` and `IntoProp<ShapeStyle>` are implemented for `Fn() -> R` where `R: ResolveColor`/`ResolveStyle`). Use `Prop::from_env` when you need the actual themed value (e.g. to apply `.opacity` or `.mix`).

```rust
# let on = State::new(false);
Text("A").background(Prop::from_env(|env| ShapeStyle::Color(env.color(ThemeColor::Surface).opacity(0.9))))
Circle().fill(Prop::from_env(move |env| {
    ShapeStyle::Color(env.color(if on.get() { ThemeColor::Accent } else { ThemeColor::Outline }))
}))
Circle().fill(move || if on.get() { ThemeColor::Accent } else { ThemeColor::Outline })   // simpler when no tweak is needed
```

`env.get::<TintKey>()` also works inside `Prop::from_env`: `Circle().fill(Prop::from_env(|env| ShapeStyle::Color(env.get::<TintKey>().opacity(0.5))))`.

---

## Themes

### Theme

```rust
pub struct Theme { pub light: ColorTokens, pub dark: ColorTokens, pub blur: BlurTokens }
Theme::standard() -> Theme        // const; also Default
pub fn colors(&self, scheme: ColorScheme) -> &ColorTokens
```

A complete set of colors for both schemes plus blur radii. Set with `.theme(theme: Theme)` (a plain value, NOT reactive: unlike `.tint`/`.color_scheme`, it takes no closure; swap themes by making the subtree depend on state, e.g. `.color_scheme(...)` or `match`). Applies to the whole subtree; nested `.theme` overrides, so parts of a window can differ.

### BlurTokens and ThemeBlur

```rust
pub enum ThemeBlur { Xs, Sm, Md, Lg, Xl, Xxl, Xxxl }          // dot: .xs .sm .md .lg .xl .xxl .xxxl (2xl, 3xl)
pub struct BlurTokens { pub xs: f32, pub sm: f32, pub md: f32, pub lg: f32, pub xl: f32, pub xxl: f32, pub xxxl: f32 }
BlurTokens::standard() -> Self    // 4, 8, 12, 16, 24, 40, 64 points; also Default; Index<ThemeBlur>
```

`.blur(.sm)` and `.backdrop_blur(.xl)` resolve a `ThemeBlur` against the theme, so a theme can restyle all frosted glass in one place. Override with `theme.blur = BlurTokens { xl: 32.0, ..BlurTokens::standard() }`.

### How to build a custom theme

```rust
fn brand_theme() -> Theme {
    let mut theme = Theme::standard();
    theme.light.accent = Color::hex(0x7C3AED);
    theme.dark.accent = Color::hex(0xA78BFA);
    theme.dark.background = Color::hex(0x17141F);
    theme
}

let _ = elg! { VStack { Text("Root") }.theme(brand_theme()) };
```

Apply once at the root of each window's content (a window's content is built once). Everything using `ThemeColor` (including built-in controls) follows. Set both `light` and `dark` for each token you change, or one scheme will keep the standard value.

---

## Light and dark mode

### ColorScheme

```rust
pub enum ColorScheme { Light /* default */, Dark }        // dot: .light .dark
.color_scheme(scheme: impl IntoProp<ColorScheme>)          // ViewExt; key: ColorSchemeKey
```

Windows follow the system's appearance and switch along with it (the window sets `ColorSchemeKey` from the system setting). `.color_scheme(...)` forces or overrides it for a subtree; reactive via closure/State.

```rust
# let dark_mode = State::new(true);
VStack { Text("Sidebar") }.color_scheme(.dark)
VStack { Text("Root") }.color_scheme(move || if dark_mode.get() { ColorScheme::Dark } else { ColorScheme::Light })
```

### How to follow system light/dark and accent

- Light/dark: do nothing. Use `ThemeColor`s and the default window; do not call `.color_scheme` at all. For a user override ("System / Light / Dark" setting), wrap the content in a reactive branch that omits `.color_scheme` for "System" (e.g. `match` on the setting), or read the system value with `Environment::<ColorSchemeKey>::new()` above the override and return it from the closure for "System".
- Accent: `.tint(|| system_accent_color().unwrap_or(Color::hex(0x0A84FF)))` at the root. It updates live when the user changes the setting.
- Both are reactive; nothing to poll.

```rust
VStack { Text("Root") }.tint(|| system_accent_color().unwrap_or(Color::hex(0x0A84FF)))
```

### system_accent_color

```rust
pub fn system_accent_color() -> Option<Color>
```

The accent the person chose in system settings, or `None` where there is none or the platform does not say (the C++ header notes Linux desktops only through the settings portal, if at all). Reactive while an App runs (tracked read on the UI thread); outside an App it asks the system each time, untracked. Always provide a fallback.

---

## Tint, foreground style, font modifiers

All are `ViewExt` environment modifiers: set for the view and everything inside; reactive (closure/State/Computed) except `.theme`.

| Modifier | Signature | Key | Default |
|---|---|---|---|
| `.tint` | `fn tint(self, color: impl IntoProp<Color>) -> View` | `TintKey` | theme `accent` |
| `.foreground_style` | `fn foreground_style(self, color: impl IntoProp<Color>) -> View` (also takes `ThemeColor`) | `ForegroundKey` | theme `primary` |
| `.font` | `fn font(self, font: impl IntoProp<Font>) -> View` | `FontKey` | `Font::body()` |
| `.font_weight` | `fn font_weight(self, weight: impl IntoProp<Weight>) -> View` (keeps size/family; icons too) | `FontKey` | |
| `.color_scheme` | see above | `ColorSchemeKey` | system |
| `.theme` | `fn theme(self, theme: Theme) -> View` | `ThemeKey` | `Theme::standard()` |
| `.disabled` | `fn disabled(self, disabled: impl IntoProp<bool>) -> View` | `DisabledKey` | `false` |
| `.environment` | `fn environment<K: EnvironmentKey>(self, value: impl IntoProp<K::Value>) -> View` | any | |

`Text` also has its own `.font`, `.font_weight`, `.bold()`, `.foreground_style` (return `Text`; Text additionally has `.italic(bool)`, `.underline(bool)`, `.strikethrough(bool)`, see the text reference). Tint colors: filled buttons, switches, selected rows, text selection, focus rings, progress fill, slider fill, checkbox fill.

The built-in modifier values for `.modifier(...)` are `TintModifier`, `ForegroundStyleModifier`, `FontModifier`, `ColorSchemeModifier`, `ThemeModifier`, `EnvironmentKeyModifier`, `ButtonStyleModifier` (+ `ToggleStyleModifier`, `CheckboxStyleModifier`, `SliderStyleModifier`, `PickerStyleModifier`, `ProgressStyleModifier`, `TextFieldStyleModifier`, `NumberInputStyleModifier`, `MenuStyleModifier`, `PopoverStyleModifier`, `TooltipStyleModifier`, `RowStyleModifier`, `NavigationStackStyleModifier`), `BackgroundModifier`, `BlurModifier`. Normally use the `ViewExt` methods instead.

```rust
Button("Delete", remove).tint(Color::hex(0xFF453A))
VStack { Text("Form") }.tint(.success)
Text("Title").font(.title).bold()
VStack { Text("A") Image("gear") }.font_weight(.bold)      // each at its own size
```

---

## Default palettes (Theme::standard())

From `Theme::standard()` in color.rs (hex RGB; glass = RGBA components):

| Token | Light | Dark |
|---|---|---|
| background | F5F5F7 | 1C1C1E |
| surface | FFFFFF | 2C2C2E |
| surface_raised | ECECEF | 3A3A3C |
| outline | D1D1D6 | 48484A |
| primary | 1C1C1E | F2F2F7 |
| secondary | 6C6C70 | AEAEB2 |
| tertiary | AEAEB2 | 636366 |
| accent | 0A84FF | 0A84FF |
| on_accent | FFFFFF | FFFFFF |
| danger | FF3B30 | FF453A |
| success | 34C759 | 30D158 |
| warning | FF9F0A | FFD60A |
| glass ultra_thin / thin / regular / thick / ultra_thick (alpha) | rgb(.97,.97,.98) at .20 / .38 / .55 / .72 / .86 | rgb(.11,.11,.12) at .30 / .45 / .60 / .74 / .86 |

Blur: 4, 8, 12, 16, 24, 40, 64 points (xs .. xxxl).

---

## ShapeStyle, gradients, BackdropFill

```rust
pub enum ShapeStyle { Color(Color), Linear(LinearGradient), Radial(RadialGradient),
                      Elliptical(EllipticalGradient), Angular(AngularGradient), Backdrop(BackdropFill) }
// From<Color|LinearGradient|RadialGradient|EllipticalGradient|AngularGradient|BackdropFill> for ShapeStyle
pub struct GradientStop { pub color: Color, pub location: f32 }   // GradientStop::new(color, location), 0...1
```

What `.background`, `.background_in`, `.fill` accept (`impl IntoProp<ShapeStyle>`): a `Color`, `ThemeColor`, gradient, `BackdropFill`, `ShapeStyle`, or a closure returning one. Gradient construction and usage live in components.md (shapes/drawing); constructors for reference: `LinearGradient::new(stops, start, end)`, `LinearGradient::colors(iter)` (top to bottom), `.from_to(start, end)`; `RadialGradient::new(stops, center, start_radius, end_radius)` / `::colors`; `EllipticalGradient::new(stops, center, start_radius_fraction, end_radius_fraction)` / `::colors`; `AngularGradient::new(stops, center, start_angle, end_angle)` / `::colors` / `.angles(start, end)`.

```rust
pub struct BackdropFill { pub blur: f32, pub saturation: f32, pub brightness: f32, pub noise: f32 }   // Default: 0, 1, 0, 0
BackdropFill::new(blur: f32) -> Self;  .saturation(f32)  .brightness(f32)  .noise(f32)
```

A fill showing what is drawn behind it in the window, blurred and adjusted (saturation 1 = unchanged; brightness added per channel -1..1; noise 0..1 grain against banding). Inside an offscreen group (opacity < 1, shadow, blur) it only sees what that group drew behind it, as CSS backdrop-filter does. Use `.background(BackdropFill::new(24.0).saturation(1.8))` when the whole fill is the backdrop; use `.backdrop_blur*` when you also want a tint under it.

---

## Backdrop blur as a styling tool

```rust
fn blur(self, radius: impl IntoProp<f32>) -> View                 // blurs the view's own content
fn blur_with(self, radius: impl IntoProp<f32>, options: impl FnOnce(BlurOptions) -> BlurOptions) -> View   // BlurOptions::opaque(bool): edges stay sharp
fn backdrop_blur(self, radius: impl IntoProp<f32>) -> View        // blurs what is behind the view, inside its bounds
fn backdrop_blur_with(self, radius: impl IntoProp<f32>, options: impl FnOnce(BackdropOptions) -> BackdropOptions) -> View
fn backdrop_blur_in(self, radius: impl IntoProp<f32>, shape: impl Into<Shape>, options: impl FnOnce(BackdropOptions) -> BackdropOptions) -> View
// BackdropOptions: .saturation(impl IntoProp<f32>) (1 unchanged, >1 vivid)  .brightness(...) (-1..1)  .noise(...) (0..1)
```

`radius` is a float in points or a `ThemeBlur` (`.xl`) (reactive, animatable). Like `.background`, the blur goes behind everything it wraps, so a tint goes on before it in the chain (as in the example: `.background_in(.glass_regular, ..)` first, then `.backdrop_blur_in(..)`):

```rust
Text("Now playing").padding(12)
    .background_in(.glass_regular, RoundedRectangle(12))
    .backdrop_blur_in(.xl, RoundedRectangle(12), |o| o.saturation(1.8).noise(0.02))
```

Prefer a `Glass*` `ThemeColor` as the tint (it is light in light mode, dark in dark mode and restylable via the theme); use a fixed `Color::WHITE.opacity(0.5)` only when the look must not change with the scheme. There are no material presets: build frosted panels from glass tints + blur radii. Needs content behind it to be interesting (put it over an image/gradient/scrolling content). For OS-level window blur see [WindowMaterial](#windowmaterial).

---

## Fonts

### Font

```rust
pub struct Font { /* family, size, weight, italic */ }      // Default = Font::body()
pub fn system(size: f32, weight: impl Into<Weight>) -> Self   // platform UI font
pub fn custom(family: impl Into<String>, size: f32, weight: impl Into<Weight>) -> Self   // a registered family
pub fn weight(self, weight: impl Into<Weight>) -> Self
pub fn bold(self) -> Self
pub fn with_size(self, size: f32) -> Self
pub fn italic(self, italic: bool) -> Self                    // italic face of the system font; registered families stay upright (C++ header note)
pub fn family(&self) -> &str   pub fn size(&self) -> f32   pub fn font_weight(&self) -> Weight   pub fn is_italic(&self) -> bool
```

Sizes are `f32` points (`Font::system(17.0, Weight::Regular)`; inside `elg!`, `Font::system(56.0, .bold)`). Showing text in a family that is not registered is an error, reported when the text appears. Prefer a text style (below) over a raw size for consistent typography; use `Font::system` for one-offs (big numbers), `Font::custom` for brand type.

```rust
let big = Font::system(32.0, Weight::Bold);
let light_title = Font::title().weight(Weight::Regular);
let larger = Font::body().with_size(18.0);
Text("Hello").font(Font::custom("Inter", 16.0, Weight::Semibold))
```

### Weight

```rust
pub struct Weight(u16);       // From<u16>; Default = Regular; Ord
Weight::UltraLight(100) Thin(200) Light(300) Regular(400) Medium(500) Semibold(600) Bold(700) Heavy(800) Black(900)
pub const fn new(value: u16) -> Self     // 1..=1000, clamped; Weight::new(550)
pub const fn value(self) -> u16
```

Dot forms in `elg!`: `.ultra_light .thin .light .regular .medium .semibold .bold .heavy .black`. Variable fonts (macOS system font, a variable file you register) draw any weight exactly; other fonts use the closest registered weight.

### Text styles

Static constructors, `Font::large_title()` etc.; dot forms `.large_title .title .title2 .title3 .headline .body .callout .subheadline .footnote .caption .caption2`. Sizes/weights are the same on all platforms (system family differs).

| Style | Size | Weight |
|---|---|---|
| `large_title` | 30 | bold |
| `title` | 24 | semibold |
| `title2` | 20 | semibold |
| `title3` | 17 | semibold |
| `headline` | 15 | semibold |
| `body` | 15 | regular (the default) |
| `callout` | 14 | regular |
| `subheadline` | 13 | regular |
| `footnote` | 12 | regular |
| `caption` | 12 | regular |
| `caption2` | 11 | regular |

### Registering fonts

```rust
pub fn register_font(family: &str, file: impl AsRef<Path>, weight: impl Into<Weight>) -> Result<(), Error>
pub fn register_font_data(family: &str, data: &[u8], weight: impl Into<Weight>) -> Result<(), Error>   // copied
pub fn register_emoji_font(file: impl AsRef<Path>) -> Result<(), Error>    // color emoji font for emoji in any text; register before text is first shaped
```

Register each weight separately under the same family; text uses the closest. Thread-safe. Register before building views that use the family (e.g. at the top of `main`, before `App::run`). `register_font_data` with `include_bytes!` embeds a font in the binary. Characters a font lacks (CJK, symbols) fall back to system fonts; emoji render in color.

```rust
register_font("Inter", "fonts/Inter-Regular.ttf", Weight::Regular)?;
register_font("Inter", "fonts/Inter-SemiBold.ttf", Weight::Semibold)?;
register_font_data("Brand", include_bytes!("../assets/Brand.ttf"), Weight::Regular)?;
```

### Platform default fonts

`Font::system` is the platform UI font: Segoe UI on Windows, SF on macOS, fontconfig's sans-serif on Linux (Elegance loads fontconfig at runtime; if absent it looks for DejaVu, Noto, or Liberation Sans). Right-to-left scripts are not supported yet (docs).

---

## Icons

### Image and Label icons

```rust
pub fn Image(source: impl Into<ImageSource>) -> View          // a &str / String is an icon name
pub fn Label(title: impl IntoProp<String>, icon: impl Into<ImageSource>) -> View
```

Elegance ships the Phosphor set (about 1,500 icons), registered as `phosphor` and the default. An icon is like text: it takes font size (`.font`), weight (`.font_weight`) and color (`.foreground_style`) from the environment and aligns with adjacent text. Name forms: `"<icon>"`, `"<icon>.<variant>"` (`"heart.fill"`), `"<set>.<icon>[.<variant>]"` (`"phosphor.star.duotone"`). Names are Phosphor's own (phosphoricons.com); a mistyped name errors with the closest names. Phosphor thin/light/regular/bold follow the font weight; `fill` and `duotone` are variants. The full icon-name list is not in this repo's docs: take names from phosphoricons.com (kebab-case, e.g. `gear`, `trash`, `bell`, `microphone`, `star`, `heart`).

Prefer `Label` in buttons, menus and lists (spaced correctly); `Image` alone for icon-only buttons, which need `.accessibility_label("...")` for screen readers.

```rust
Image("gear")
Image("heart").foreground_style(.danger).font(.title2)
Image("bell").font_weight(.bold)
Image("heart.fill")
Image("phosphor.star.duotone")
Button(Label("Record", "microphone"), record)
Button(Image("trash"), remove).accessibility_label("Delete")
```

### Custom icon sets

```rust
pub struct IconSetOptions { pub font: IconFont, pub weights: Vec<(Weight, IconFont)>, pub variants: Vec<IconVariant>,
                            pub names: PathBuf, pub names_data: String, pub replace: bool }   // Default
pub struct IconVariant { pub name: String, pub font: IconFont, pub names: PathBuf, pub names_data: String }
pub enum IconFont { None /* default */, File(PathBuf), Data(Vec<u8>) }
// IconFont: From<&str|String|&Path|PathBuf|Vec<u8>|&[u8]>; .is_empty() .file() -> Option<&Path> .data() -> Option<&[u8]>
pub fn register_icon_set(name: &str, options: IconSetOptions) -> Result<(), Error>
pub fn set_default_icon_set(name: &str) -> Result<(), Error>
```

`font` = regular weight; `weights` = more weights (for `.font_weight()`); `variants` = e.g. `fill` with their own font and optionally own `names`; `names`/`names_data` = name-to-codepoint map (CSS from icon webfonts, JSON, or `name codepoint` lines; the map text wins over the file). Names: not empty, no dots. A taken name (including `phosphor`) errors unless `replace: true`. A failed registration leaves sets unchanged. Any thread.

```rust
register_icon_set("brand", IconSetOptions {
    font: IconFont::File("brand-icons.ttf".into()),
    names: "brand-icons.css".into(),
    ..Default::default()
})?;
let _ = elg! { Image("brand.logo") };
set_default_icon_set("brand")?;     // plain names ("logo") now come from "brand"
```

### ImageSource, AsyncImage and the image cache

Images beyond icons (files, asset!, URLs) belong to the media/images reference; signatures for completeness:

```rust
pub struct ImageSource { pub system_name: String, pub file: PathBuf, pub data: Cow<'static, [u8]>, pub scale: f32, pub max_pixels: u64, pub max_side: u32 }
ImageSource::system(name) / ::file(path) / ::data(bytes) / .scale(f32) / .max_pixels(u64) / .max_side(u32)
AsyncImage::new(url) (builders: .disk_cache(bool) .scale(f32) .max_pixels(u64) .max_side(u32) .content_mode(..) .content(|&AsyncImagePhase| ..))
pub enum AsyncImagePhase { Loading, Success(View), Failure(String) }
pub struct ImageCacheOptions { pub disk: bool, pub disk_limit: u64, pub directory: Option<PathBuf> }   // Default: true, 256 MiB, None
pub fn set_image_cache(options: ImageCacheOptions)    // UI thread
pub fn clear_image_cache()                            // UI thread
```

---

## Control styles

### Overview and modifiers

A control's behavior and look are separate: the control handles clicks, hover, focus, keyboard, accessibility; its style only draws. A style applies to every control of its kind inside the view it is set on.

| Control | Modifier (ViewExt) | Built-in styles (default first) | Trait / Configuration / Any / Into / Key |
|---|---|---|---|
| `Button` | `.button_style(impl IntoButtonStyle)` | `ProminentButtonStyle`, `BorderedButtonStyle`, `PlainButtonStyle` | `ButtonStyle` / `ButtonConfiguration` / `AnyButtonStyle` / `IntoButtonStyle` / `ButtonStyleKey` |
| `Toggle` | `.toggle_style(..)` | `SwitchToggleStyle` | `ToggleStyle` / `ToggleConfiguration` / `AnyToggleStyle` / `IntoToggleStyle` / `ToggleStyleKey` |
| `Checkbox` | `.checkbox_style(..)` | `BoxCheckboxStyle` | `CheckboxStyle` / `CheckboxConfiguration` / `AnyCheckboxStyle` / `IntoCheckboxStyle` / `CheckboxStyleKey` |
| `Picker` | `.picker_style(..)` | `SegmentedPickerStyle`, `MenuPickerStyle` | `PickerStyle` / `PickerConfiguration` / `AnyPickerStyle` / `IntoPickerStyle` / `PickerStyleKey` |
| `Slider` | `.slider_style(..)` | `KnobSliderStyle` | `SliderStyle` / `SliderConfiguration` / `AnySliderStyle` / `IntoSliderStyle` / `SliderStyleKey` |
| `NumberInput` | `.number_input_style(..)` | `BoxNumberInputStyle`, `PlainNumberInputStyle` | `NumberInputStyle` / `NumberInputConfiguration` / `AnyNumberInputStyle` / `IntoNumberInputStyle` / `NumberInputStyleKey` |
| `Progress` | `.progress_style(..)` | `LinearProgressStyle`, `CircularProgressStyle` | `ProgressStyle` / `ProgressConfiguration` / `AnyProgressStyle` / `IntoProgressStyle` / `ProgressStyleKey` |
| `TextField`, `TextArea` | `.text_field_style(..)` | `RoundedBorderTextFieldStyle`, `PlainTextFieldStyle` | `TextFieldStyle` / `TextFieldConfiguration` / `AnyTextFieldStyle` / `IntoTextFieldStyle` / `TextFieldStyleKey` |
| popovers | `.popover_style(..)` | `DefaultPopoverStyle`, `PlainPopoverStyle` | `PopoverStyle` / `PopoverConfiguration` / `AnyPopoverStyle` / `IntoPopoverStyle` / `PopoverStyleKey` |
| tooltips | `.tooltip_style(..)` | `DefaultTooltipStyle`, `PlainTooltipStyle` | `TooltipStyle` / `TooltipConfiguration` / `AnyTooltipStyle` / `IntoTooltipStyle` / `TooltipStyleKey` |
| `Menu` | `.menu_style(..)` | `DefaultMenuStyle`, `PlainMenuStyle` | `MenuStyle` / `MenuConfiguration`, `MenuItemConfiguration` / `AnyMenuStyle` / `IntoMenuStyle` / `MenuStyleKey` |
| selectable rows (`.selection(..)`) | `.row_style(..)` | `HighlightRowStyle`, `PlainRowStyle` | `RowStyle` / `RowConfiguration` / `AnyRowStyle` / `IntoRowStyle` / `RowStyleKey` |
| `NavigationStack` | `.navigation_stack_style(..)` | `PlainNavigationStackStyle`, `BarNavigationStackStyle` | `NavigationStackStyle` / `NavigationStackConfiguration` / `AnyNavigationStackStyle` / `IntoNavigationStackStyle` / `NavigationStackStyleKey` |

All modifiers take a built-in value, a custom style value, or an `Any*Style`. Styles can be reactive via the environment key (the key value may be a closure returning an `Any*Style`).

Defaults are the same on every platform: Elegance draws all controls itself (not native widgets); there are no per-platform default styles (design.md decision). Rust built-ins: option-less ones are unit structs passed bare (`.button_style(BorderedButtonStyle)`); ones with options are structs with `::new()` builders and `Default` (`SwitchToggleStyle::new().fills_width(true)`).

### Restyle one control vs all controls under a subtree

The modifier sets an environment value, so scope = the view you put it on and everything inside.

```rust
VStack { Button("OK", || {}) Button("Cancel", || {}) }.button_style(BorderedButtonStyle)   // both buttons
HStack {
    Button("Save", save)                                   // default prominent
    Button("Skip", skip).button_style(PlainButtonStyle)    // just this one
}
```

- One control: put the modifier directly on it.
- Whole form/window: put it on the container (or the window content root; nested modifiers win).
- Popover/tooltip/menu styles apply by where the view that OPENS the popup is, so one `.popover_style(..)` near the root styles every popover in the window.
- A style cannot change a control's input behavior, only its look. For different behavior, build your own view.

```rust
Toggle("Wi-Fi", wifi).toggle_style(SwitchToggleStyle::new().fills_width(true))
Checkbox("Remember me", remember).checkbox_style(BoxCheckboxStyle::new().size(16))
TextField("Search", query).text_field_style(PlainTextFieldStyle)
```

### How to write a custom style

1. A unit struct (or struct with fields) implementing the trait (`make_body(&self, config) -> View`; `SliderStyle` has `make_track`/`make_fill`/`make_thumb`; `MenuStyle` also `make_item`).
2. `make_body` runs ONCE per control, while `Environment` is available: read state inside closures (`move || cfg.is_pressed()`), not eagerly.
3. A configuration is a handle: clones refer to the same control, so `clone()` it for each closure. It gives the control's parts as methods returning `View` (`label()`, `field()`, `content()`...), state as tracked bool reads, and actions.
4. Pass the value to the modifier (`.button_style(PillButtonStyle)`). Any `S: ButtonStyle` implements `IntoButtonStyle` (blanket impl); same for every family except where noted (built-ins with options implement it directly).
5. Use theme colors and `Environment::<TintKey>::new()` so the style follows the theme.

```rust
struct PillButtonStyle;

impl ButtonStyle for PillButtonStyle {
    fn make_body(&self, button: ButtonConfiguration) -> View {
        let tint = Environment::<TintKey>::new();
        let (pressed, enabled) = (button.clone(), button.clone());
        elg! {
            { button.label() }
                .font(.headline)
                .foreground_style(Color::WHITE)
                .padding(horizontal: 18, vertical: 8)
                .background_in(
                    move || if pressed.is_pressed() { tint.get().mix(Color::BLACK, 0.2) } else { tint.get() },
                    Capsule(),
                )
                .opacity(move || if enabled.is_enabled() { 1.0 } else { 0.5 })
        }
        .into_view()
    }
}
let _ = elg! { HStack { Button("Upgrade", upgrade) Button("Later", later) }.button_style(PillButtonStyle) };
```

Wrapping the current style: read it with `Environment::<ButtonStyleKey>::new()` then `.get().make_body(&config)` (each `Any*Style` has `make_body`); `DefaultMenuStyle.into_menu_style().make_body(&menu)` delegates to a built-in.

### Each style family

#### Button

```rust
pub trait ButtonStyle: 'static { fn make_body(&self, button: ButtonConfiguration) -> View; }
ButtonConfiguration: label() -> View; is_pressed() (pointer held, still over), is_hovered(), is_enabled() (false inside .disabled(true)), is_focused()   // -> bool, tracked
```

- `ProminentButtonStyle` (default): filled with the tint, label in `OnAccent`. Primary actions.
- `BorderedButtonStyle`: raised surface with outline, label in the tint. Secondary actions.
- `PlainButtonStyle`: just the label in the surrounding colors (not the tint), dimmed while pressed/disabled. Toolbars, icon buttons, links, rows.

All three are unit structs without options.

#### Toggle

```rust
pub trait ToggleStyle: 'static { fn make_body(&self, toggle: ToggleConfiguration) -> View;
                                 fn animation(&self) -> Option<Animation> { Some(Animation::smooth(0.3)) } }   // None: flip at once
ToggleConfiguration: label() -> View (empty if no label: View::is_empty); is_on(), is_pressed(), is_hovered(), is_enabled(), is_focused(); toggle()
pub struct SwitchToggleStyle { pub on: Option<Prop<Color>>, pub off: Prop<Color>, pub knob: Prop<Color>, pub width: f32, pub height: f32,
                               pub knob_inset: f32, pub spacing: f32, pub fills_width: bool, pub animation: Option<Animation> }
// ::new() / Default; builders: .on(color) .off(color) .knob(color) .size(w, h) .knob_inset(n) .spacing(n) .fills_width(bool) .animation(Option<Animation>)
```

Defaults: on = tint (None), off = `Outline`, knob = white, 40 x 24, inset 2, spacing 8, `fills_width` false, animation `smooth(0.3)`. `fills_width(true)` = label leading, switch trailing across the offered width (settings row).

#### Checkbox

```rust
pub trait CheckboxStyle: 'static { fn make_body(&self, checkbox: CheckboxConfiguration) -> View; fn animation(&self) -> Option<Animation> { Some(Animation::smooth(0.3)) } }
CheckboxConfiguration: label(); is_checked(), is_mixed() (partly checked), is_pressed(), is_hovered(), is_enabled(), is_focused(); toggle() (mixed becomes checked)
pub struct BoxCheckboxStyle { pub size: f32, pub corner_radius: f32, pub fill: Option<Prop<Color>>, pub empty: Prop<Color>, pub border: Prop<Color>,
                              pub border_width: f32, pub mark: Prop<Color>, pub check: Option<View>, pub dash: Option<View>, pub spacing: f32, pub animation: Option<Animation> }
// builders: .size .corner_radius .fill(color) .empty(color) .border(color) .border_width .mark(color) .check(impl IntoView) .dash(impl IntoView) .spacing .animation(Option<Animation>)
```

Defaults: 18, radius 5, fill = tint, empty = `SurfaceRaised`, border = `Outline`, width 1.5, mark = `OnAccent`, spacing 8.

#### Picker

```rust
pub trait PickerStyle: 'static { fn make_body(&self, picker: PickerConfiguration) -> View; }
PickerConfiguration: labels() -> Vec<String>; selected() -> Option<usize> (tracked); select(index: usize)
```

`SegmentedPickerStyle` (default): every option visible as segments in a rounded track, chosen one raised. `MenuPickerStyle`: chosen option on a button, others in a menu it opens (use for many options). Both unit structs.

#### Slider

```rust
pub trait SliderStyle: 'static {
    fn make_track(&self, slider: SliderConfiguration) -> View;
    fn make_fill(&self, slider: SliderConfiguration) -> View { /* default: nothing */ }
    fn make_thumb(&self, slider: SliderConfiguration) -> View;
}
SliderConfiguration: fraction() -> f32 (0..1, tracked); is_pressed() (dragged), is_hovered(), is_enabled(), is_focused()
pub struct KnobSliderStyle { pub track: Prop<Color>, pub fill: Option<Prop<Color>>, pub track_thickness: f32, pub knob_size: f32 }
// ::new(); .track(color) .fill(color) .track_thickness(n) .knob_size(n)
```

The slider places the three parts itself: track gets width less thumb width and picks its height; fill covers track start to thumb center; thumb is centered on the value. Defaults: track `Outline`, fill tint, thickness 4, knob 20. `AnySliderStyle` has `make_track/make_fill/make_thumb(&SliderConfiguration)`.

```rust
struct BarSliderStyle;
impl SliderStyle for BarSliderStyle {
    fn make_track(&self, _: SliderConfiguration) -> View { elg! { RoundedRectangle(3).fill(.outline).frame(height: 6) }.into_view() }
    fn make_fill(&self, _: SliderConfiguration) -> View { elg! { RoundedRectangle(3).fill(.accent).frame(height: 6) }.into_view() }
    fn make_thumb(&self, slider: SliderConfiguration) -> View {
        elg! { RoundedRectangle(2).fill(.accent).frame(4, 16).scale_effect(move || if slider.is_pressed() { 1.25 } else { 1.0 }) }.into_view()
    }
}
```

#### NumberInput

```rust
pub trait NumberInputStyle: 'static { fn make_body(&self, input: NumberInputConfiguration) -> View; }
NumberInputConfiguration: field() -> View (put where the text goes); increment(); decrement();
    can_increment() (below max), can_decrement() (above min), shows_buttons() (NumberInput::shows_buttons), is_focused(), is_hovered(), is_enabled()
pub struct BoxNumberInputStyle { pub background: Prop<Color>, pub border: Prop<Color>, pub focused_border: Option<Prop<Color>>, pub border_width: f32,
                                 pub corner_radius: f32, pub padding: EdgeInsets, pub arrow_size: f32 }
// ::new(); .background .border .focused_border .border_width .corner_radius .padding(impl Into<EdgeInsets>) .arrow_size
```

`BoxNumberInputStyle` (default): raised box with outline (tint while editing), stacked arrows trailing. Defaults: `SurfaceRaised`, `Outline`, focused = tint, 1.0, radius 6, padding symmetric(8, 5), arrows 9. `PlainNumberInputStyle`: just the number (and arrows if shown), for your own box.

#### Progress

```rust
pub trait ProgressStyle: 'static { fn make_body(&self, progress: ProgressConfiguration) -> View; }
ProgressConfiguration: fraction() -> Option<f32>   // 0..1; None = indeterminate; tracked
pub struct LinearProgressStyle { pub track: Prop<Color>, pub fill: Option<Prop<Color>>, pub thickness: f32 }                    // default 'Outline', tint, 6
pub struct CircularProgressStyle { pub track: Prop<Color>, pub fill: Option<Prop<Color>>, pub size: f32, pub line_width: f32 }  // default 'Outline', tint, 20, 3
// Linear: ::new() .track .fill .thickness ; Circular: ::new() .track .fill .size .line_width
```

Linear (default): bar as wide as offered (sweeping segment when indeterminate). Circular: ring clockwise from top (spinning arc when indeterminate); use for inline spinners.

#### TextField / TextArea

```rust
pub trait TextFieldStyle: 'static { fn make_body(&self, field: TextFieldConfiguration) -> View; }
TextFieldConfiguration: field() -> View (the editable field; put it where the text goes); is_focused(), is_hovered(), is_enabled()
```

`RoundedBorderTextFieldStyle` (default): rounded border taking the tint while focused. `PlainTextFieldStyle`: just the text. Both unit structs.

```rust
struct UnderlinedFieldStyle;
impl TextFieldStyle for UnderlinedFieldStyle {
    fn make_body(&self, field: TextFieldConfiguration) -> View {
        let focused = field.clone();
        let line = Rectangle()
            .fill(Prop::from_env(move |env| ShapeStyle::Color(env.color(if focused.is_focused() { ThemeColor::Accent } else { ThemeColor::Outline }))))
            .frame_with(|o| o.height(1));
        elg! { { field.field() }.padding(top: 6, bottom: 6).overlay(line, alignment: .bottom) }.into_view()
    }
}
```

#### Row (selectable lists)

```rust
pub trait RowStyle: 'static { fn make_body(&self, row: RowConfiguration) -> View; }
RowConfiguration: content() -> View; axis() -> Axis; is_selected(), is_hovered(), is_focused() (the row keys act on), is_enabled()
pub struct HighlightRowStyle { pub selected: Option<Prop<Color>>, pub on_selected: Prop<Color>, pub hovered: Prop<Color>, pub corner_radius: f32, pub padding: EdgeInsets }
// ::new(); .selected(color) .on_selected(color) .hovered(color) .corner_radius(n) .padding(impl Into<EdgeInsets>)
```

`HighlightRowStyle` (default): padded rounded highlight: tint while selected (text/icons `OnAccent`), faint `Outline` at half opacity on hover, outlined in the tint when focused; defaults radius 6, padding symmetric(8, 4). `PlainRowStyle`: just the content; show state yourself by reading `Environment::<RowSelectedKey>::new()` (bool, false outside selectable rows).

#### NavigationStack

```rust
pub trait NavigationStackStyle: 'static { fn make_body(&self, stack: NavigationStackConfiguration) -> View; }
NavigationStackConfiguration: pages() -> View; titles() -> Vec<String> (root first); can_go_back() -> bool; back()
pub struct BarNavigationStackStyle { pub height: f32, pub background: Prop<Color>, pub title_font: Font, pub back_color: Option<Prop<Color>>, pub shows_divider: bool }
// ::new(); .height(n) .background(color) .title_font(impl Into<Font>) .back_color(color) .shows_divider(bool)
```

`PlainNavigationStackStyle` (default): just the pages (go back from your own controls or gestures). `BarNavigationStackStyle`: top bar with centered title and, once a page is pushed, a back button named after the page below. Defaults: 44, clear, `Font::headline()`, tint, divider shown.

#### Popover, Tooltip, Menu

```rust
pub trait PopoverStyle: 'static { fn make_body(&self, popover: PopoverConfiguration) -> View; fn motion(&self) -> PopupMotion { PopupMotion::none() } }   // make_body called each time it opens
PopoverConfiguration: content() -> View; is_padded() -> bool (PopoverOptions::padded)
pub trait TooltipStyle: 'static { fn make_body(&self, tooltip: TooltipConfiguration) -> View; fn motion(&self) -> PopupMotion }
TooltipConfiguration: content() -> View
pub trait MenuStyle: 'static { fn make_body(&self, menu: MenuConfiguration) -> View;
                               fn make_item(&self, item: MenuItemConfiguration) -> View { /* default row */ }
                               fn motion(&self) -> PopupMotion }
MenuConfiguration: items() -> View (already stacked)
MenuItemConfiguration: title() -> View; leading() -> View (check/icon, empty if none); trailing() -> View (shortcut or submenu chevron, empty if none);
                       is_highlighted() (hover or keyboard), is_enabled(), is_destructive()
```

`DefaultPopoverStyle`: rounded surface, outline, shadow; padded content inset 12. `PlainPopoverStyle`: just the content. `DefaultTooltipStyle`: small text on rounded surface with outline and shadow. `PlainTooltipStyle`: just content. `DefaultMenuStyle`: surface + outline + shadow, at least 200 points wide. `PlainMenuStyle`: just the items. `MenuStyle::make_item` default: parts in a line, padded, danger color if destructive, dimmed when disabled, rounded highlight. All six built-ins are unit structs and have no `animation`; popups open at once unless `motion()` says otherwise.

```rust
struct CardPopoverStyle;
impl PopoverStyle for CardPopoverStyle {
    fn make_body(&self, popover: PopoverConfiguration) -> View {
        elg! { { popover.content() }.padding(16).background_in(.surface_raised, RoundedRectangle(16))
               .shadow(color: Color::BLACK.opacity(0.2), radius: 24, y: 8) }.into_view()
    }
}
let _ = elg! { VStack { Text("Root") }.popover_style(CardPopoverStyle).tooltip_style(PlainTooltipStyle) };
```

Menu item override example (rows with taller padding and an accent highlight):

```rust
struct RoomyMenuStyle;
impl MenuStyle for RoomyMenuStyle {
    fn make_body(&self, menu: MenuConfiguration) -> View { DefaultMenuStyle.into_menu_style().make_body(&menu) }
    fn make_item(&self, item: MenuItemConfiguration) -> View {
        let (fg, bg) = (item.clone(), item.clone());
        elg! {
            HStack(spacing: 10) { { item.leading() } { item.title() } Spacer(min_length: 32) { item.trailing() } }
                .padding(horizontal: 12, vertical: 9)
                .frame(max_width: .infinity, alignment: .leading)
                .foreground_style(Prop::from_env(move |env| env.color(if fg.is_highlighted() { ThemeColor::OnAccent } else { ThemeColor::Primary })))
                .background_in(Prop::from_env(move |env| ShapeStyle::Color(if bg.is_highlighted() { env.color(ThemeColor::Accent) } else { Color::CLEAR })), RoundedRectangle(6))
        }.into_view()
    }
}
```

### PopupMotion

```rust
pub struct PopupMotion   // Clone
PopupMotion::new(transition: impl Into<Transition>, animation: impl Into<Animation>) -> Self
PopupMotion::none() -> Self                                  // at once (built-in default)
.transition() -> Option<&Transition>   .animation() -> Option<&Animation>   .animates() -> bool
```

Return from a style's `motion()` (or per popup via options' `motion: Option<PopupMotion>`), e.g. `PopupMotion::new(Transition::opacity().combined(Transition::scale_anchored(0.96, Alignment::TOP)), Animation::ease_out(0.15))`. `Any{Popover,Tooltip,Menu}Style::motion(&self) -> PopupMotion` reads a style's. `AnyToggleStyle::animation()` / `AnyCheckboxStyle::animation()` -> `Option<Animation>`.

### AnyStyle and IntoStyle

`Any<X>Style` is a style as the environment holds it (built-in or Rust); it has `make_body(&self, &<X>Configuration) -> View` (Slider: `make_track/make_fill/make_thumb`; Menu: also `make_item`), is `Clone`. `Into<X>Style` has one method `into_<x>_style(self) -> Any<X>Style` (`into_button_style`, `into_toggle_style`, `into_checkbox_style`, `into_picker_style`, `into_slider_style`, `into_number_input_style`, `into_progress_style`, `into_text_field_style`, `into_popover_style`, `into_tooltip_style`, `into_menu_style`, `into_row_style`, `into_navigation_stack_style`). Implemented for the built-ins, for every `S: <X>Style` (blanket impl; Toggle/Checkbox/Slider/Popover/Tooltip/Menu ones also pass `animation()`/`motion()` along), and for `Any<X>Style`. You rarely call these yourself; call `into_*_style()` to get an `Any*Style` for delegation or storing in state.

### Environment keys

`ButtonStyleKey`, `ToggleStyleKey`, `CheckboxStyleKey`, `PickerStyleKey`, `SliderStyleKey`, `NumberInputStyleKey`, `ProgressStyleKey`, `TextFieldStyleKey`, `PopoverStyleKey`, `TooltipStyleKey`, `MenuStyleKey`, `RowStyleKey`, `NavigationStackStyleKey` (value type `Any<X>Style`; prelude exports). Read with `Environment::<ButtonStyleKey>::new().get()` (e.g. to wrap the current style); set via the modifiers.

---

## Environment (theme and scheme keys)

```rust
pub trait EnvironmentKey: 'static { type Value: Clone + 'static; const NAME: Option<&'static str> = None; fn default_value(environment: &EnvironmentValues) -> Self::Value; }
pub struct Environment<K: EnvironmentKey>;   Environment::<K>::new() (while building a view; panics elsewhere); .get() -> K::Value (reactive)
pub struct EnvironmentValues;                // inside Prop::from_env(|env| ..)
  new()  current()  color(token: impl Into<ThemeColor>) -> Color  get::<K>()  set::<K>(value)
  font() -> Font   color_scheme() -> ColorScheme   theme() -> Theme   foreground() -> Color   tint() -> Color
  is_disabled() -> bool   hover_delay() -> HoverDelay   window() -> WindowHandle
```

| Modifier | Key | Default |
|---|---|---|
| `.font` | `FontKey` | `Font::body()` |
| `.foreground_style` | `ForegroundKey` | theme `primary` |
| `.tint` | `TintKey` | theme `accent` |
| `.color_scheme` | `ColorSchemeKey` | the system's setting (set by the window); key's own fallback is `Light` |
| `.theme` | `ThemeKey` | `Theme::standard()` |
| `.disabled` | `DisabledKey` | `false` |
| `.hover_delay` | `HoverDelayKey` | enter 0, exit 0 |
| (window) | `WindowKey` | the window the view is in |
| (selectable rows) | `RowSelectedKey` | `false` |
| style modifiers | `*StyleKey` | default styles |

`color()` is a tracked read: inside a closure it updates when theme/scheme change. Read keys while a view is being built (`#[component]`, a style's `make_body`), then use the reader inside closures. Custom keys: see the environment reference; `NAME` shares bool/i32/i64/f32/f64/String/`Color` keys with C++.

---

## Window appearance

### WindowDesc appearance fields

```rust
pub struct WindowDesc { pub title: String, pub width: i32, pub height: i32, pub resizable: bool, pub min_size: Option<Size>,
    pub background: Option<Color>, pub title_bar: TitleBar, pub system_controls_position: Option<Point>,
    pub transparent: bool, pub material: WindowMaterial, pub opacity: f32, pub fullscreen: bool }
WindowDesc::new(title) .size(i32, i32) .resizable(bool) .min_size(f32, f32) .background(impl Into<Color>) .title_bar(impl Into<TitleBar>)
  .system_controls_position(x: f32, y: f32) .transparent(bool) .material(impl Into<WindowMaterial>) .opacity(f32) .fullscreen(bool)
```

Defaults: title "Elegance", 1280 x 720, resizable, no min size, background = theme background (clear for transparent windows, and where a material is drawn), `TitleBar::System`, not transparent, `WindowMaterial::None`, opacity 1, not fullscreen. `background` is what shows where no view draws.

### TitleBar

```rust
pub enum TitleBar { System /* default */, Hidden, Integrated }      // dot: .system .hidden .integrated
```

| Variant | Behavior | Prefer when |
|---|---|---|
| `System` | platform title bar and frame | default; standard apps |
| `Hidden` | no system title bar; content fills the window, edges still resize; draw your own bar with `.window_drag_area(true)` and `WindowControls` | fully custom chrome, shaped windows (with `transparent`), widgets |
| `Integrated` | content fills window under the system's own buttons (real traffic lights on macOS, movable via `system_controls_position`); platforms whose buttons cannot sit over content keep a normal system title bar | macOS-style unified toolbar; keep content clear of `WindowHandle::system_controls_frame()` |

`.window_drag_area(true)` (ViewExt, `fn window_drag_area(self, enabled: bool) -> View`) marks the part that drags the window; double-click maximizes; controls inside still work.

```rust
#[component]
fn Notes() -> impl IntoView {
    elg! {
        VStack(spacing: 0) {
            HStack { Text("Notes") Spacer() { WindowControls::new() } }.frame(height: 36).window_drag_area(true)
            Text("Content")
        }
    }
}
App::new().window(WindowDesc::new("Notes").title_bar(TitleBar::Hidden), Notes).run()
```

Integrated: reserve space for the buttons and keep it updated (they move or vanish in fullscreen):

```rust
let window = Environment::<WindowKey>::new();
elg! { VStack {
    EmptyView().frame_size(move || { let c = window.get().system_controls_frame(); Size::new(0.0, c.y + c.height) }, .top)
    Text("Search")
}.window_drag_area(true) }
// WindowDesc::new("Music").title_bar(TitleBar::Integrated).system_controls_position(20.0, 27.0)
```

`WindowHandle` (from `WindowKey`) has `close()`, `is_maximized()`, `is_active()` (dim your title bar when inactive), `system_controls_frame() -> Rect`.

### WindowMaterial

```rust
pub enum WindowMaterial { None /* default */, Sidebar, UnderWindow, Mica, Acrylic }     // dot: .sidebar .under_window .mica .acrylic
```

A system-drawn translucent backdrop (desktop and windows below, blurred and tinted). Shows through wherever your content and window background are not opaque (so make big panels `.background(.background)` where you want opaque).

| Material | macOS | Windows 11 (22H2+) |
|---|---|---|
| `Sidebar` | sidebar vibrancy | Mica Alt |
| `UnderWindow` | under-window vibrancy | Mica |
| `Mica` | under-window vibrancy | Mica |
| `Acrylic` | HUD window vibrancy | Acrylic |

Windows 10 and Linux: window stays opaque and draws its background as usual (no error). Do not depend on it for legibility; keep text contrast OK on the opaque fallback. Prefer a material for whole-window chrome (sidebars, Settings-style apps); prefer `backdrop_blur*` for in-window glass that must look the same on every platform.

```rust
let desc = WindowDesc::new("Files").material(WindowMaterial::Sidebar);
App::new().window(desc, Files).run()
```

### Transparency and opacity

- `transparent(true)`: desktop shows through wherever content is not opaque; pair with `TitleBar::Hidden` for rounded/shaped windows and see-through panels. Needs a compositing desktop; otherwise the window stays opaque. Give your root a shape: `.background_in(.surface, RoundedRectangle(16))`.
- `opacity(f32)`: whole window 0...1.
- `fullscreen(bool)`: covers the display, no frame.

```rust
let desc = WindowDesc::new("Widget").size(340, 190).title_bar(TitleBar::Hidden).transparent(true);
App::new().window(desc, Widget).run()   // Widget: ... .background_in(.surface, RoundedRectangle(16))
```

### WindowControls

```rust
WindowControls::new() -> Self;  .style(impl Into<WindowControlsStyle>)  .minimize(bool)  .maximize(bool)      // IntoView; elg!: `{ WindowControls::new() }`
pub enum WindowControlsStyle { Automatic /* default */, Caption, TrafficLights }
```

`Automatic` = platform style (caption buttons on Windows and Linux, traffic lights on macOS). They draw only in `TitleBar::Hidden` windows; with `System`/`Integrated` they are empty, so one layout works everywhere. `Caption` / `TrafficLights` force a look.

---

## Platform conventions

```rust
pub enum Platform { Windows, MacOS, Linux }       // no Mikros variant; anything else counts as Linux
pub fn current_platform() -> Platform             // an ordinary runtime function
```

| Topic | Windows | macOS | Linux |
|---|---|---|---|
| System font (`Font::system`) | Segoe UI | SF | fontconfig sans-serif (fallback DejaVu, Noto, Liberation Sans) |
| Title bar default | system bar | system bar | system bar |
| Integrated title bar | normal system bar | real traffic lights over content | normal system bar |
| Hidden title bar + `WindowControls::new()` | caption buttons (46x32 style; red close hover; per design.md) | traffic lights | caption buttons |
| Shortcut primary modifier | Ctrl | Cmd | Ctrl |
| Window materials | Mica/Mica Alt/Acrylic (Win 11 only) | vibrancy | none (opaque) |
| System accent | `system_accent_color()` | yes | portal-based, often `None` |
| Menu bar (`App::menu_bar`) | top of each window | top of screen; `MenuRole::App` is the app menu | top of each window |
| Settings menu role | `MenuRole::Settings` | Cmd+, (`MenuRole::Settings`) | `MenuRole::Settings` |

Notes:
- Shortcuts: `KeyboardShortcut::new(Key::S)` uses `KeyModifiers::primary()` = Cmd on macOS, Ctrl elsewhere. Add modifiers: `KeyboardShortcut::new(Key::S).modifiers(KeyModifiers::primary() | KeyModifiers::SHIFT)`. `KeyModifiers` constants: `NONE SHIFT CONTROL ALT SUPER` (`SUPER` = Cmd on macOS, Windows key on Windows); `.has(modifier)`; `KeyModifiers::primary()`. In key handlers test `e.modifiers.has(KeyModifiers::primary())` instead of hard-coding Ctrl. Selectable text copies on primary+C, selects all on primary+A.
- `MenuBarItem::shortcut(KeyboardShortcut)`; `MenuRole` gives standard items (`Quit`, `Settings`, `About`, `Undo`, `Redo`, `Cut`, `Copy`, `Paste`, `SelectAll`, `Minimize`, `Zoom`, ...) their standard title/shortcut (e.g. Cmd+, for `Settings` on macOS). Several roles (`Hide`, `HideOthers`, `ShowAll`, `ToggleFullscreen`) are macOS only (left out elsewhere); `App` is macOS's application menu (elsewhere its items join the first menu); `Window` lists open windows on macOS. Full menu API: desktop/menus reference.
- Word-wise caret movement uses Ctrl+arrows (Alt on macOS) (design.md).
- Branch on `current_platform()` for chrome differences (e.g. leading vs trailing window controls, Cmd vs Ctrl labels in hints), at runtime; it is not a compile-time constant.
- Elegance draws all controls itself, so look and metrics are the same on every platform; only fonts, window chrome, materials, and shortcuts follow the platform.
- Mikros: `Platform` has no Mikros variant and the Rust sources mention none. Per the doc comment, any other platform counts as Linux in `current_platform()`; chrome/material behavior there is unverified.

---

## C++-only / not exposed in Rust

- `Color::darker`/`lighter`/`Color::accent` shortcuts in design.md examples: design draft only; Rust (and the shipped C++ `color.h`: `rgb`, `hex`, `opacity`, `mix`, `black/white/clear`) has no such methods. Use `mix`.
- `Font::system(size, weight, design)` (design.md section 9.5): not in the shipped C++ `font.h` or Rust (`Font::system(size, weight)` only).
- C++ `register_font(family, std::span<const unsigned char>, weight)` is Rust's `register_font_data`.
- C++ `EnvironmentValues::get_named<T>` / `set_named<T>` (string-keyed access): no such methods in Rust (grepped `src/*.rs`; `named.rs` is internal); Rust shares keys through `EnvironmentKey::NAME`.
- C++ `Environment<TintKey>()` call syntax vs Rust `Environment::<TintKey>::new()`; C++ `Signal<bool>` readers vs Rust `bool` methods on configurations (call inside closures).
- Theme/token files loaded from JSON/TOML (design.md, marked OPEN/planned): unverified; nothing in Rust.

## Doc-vs-code notes

- Docs (environment.mdx) say `.color_scheme` defaults to "the system's setting"; `ColorSchemeKey::default_value` in Rust returns `Light`. The window sets the key from the system on creation (C++ header: "Windows set this from the system setting"), so views inside windows follow the system; only a bare `EnvironmentValues::new()` reads `Light`.
- `.theme(theme: Theme)` takes a plain value (no closure/State), while `.tint`, `.color_scheme`, `.font`, `.foreground_style` are reactive.
- `lib.rs` doc comment on `ThemeColor::GlassUltraThin` shows `.backdrop_blur_in(.xl, RoundedRectangle(12), |o| o.saturation(1.8).noise(0.02))` while the `backdrop_blur` doc uses a numeric radius `30`: both work (`ThemeBlur` or `f32`).
- styles.mdx's built-in styles table omits NavigationStack styles, but code has `PlainNavigationStackStyle` (default) and `BarNavigationStackStyle`.
