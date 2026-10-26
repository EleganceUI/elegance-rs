//! The DSL's leading-dot shorthand. `.leading` in `elg!` is a [`Dot`] named `leading`: a value
//! whose type spells the name (`Dot<(l, e, a, d, i, n, g)>`), and which converts into whatever
//! type has a value by that name (`HorizontalAlignment::Leading`, `Alignment::LEADING`, ...).
//! Because the name is in the type, any crate can add values: `#[derive(DotValues)]` on an enum
//! makes `.variant_name` work wherever that enum is expected.
//!
//! ```ignore
//! #[derive(Clone, Copy, PartialEq, DotValues)]
//! enum Density { Compact, Comfortable }
//!
//! #[component]
//! fn Row(density: Density) -> impl IntoView { ... }
//!
//! elg! { Row(density: .compact) }
//! ```
//!
//! Where a builder method's parameter type is known (`#[component]` props, `#[elg_builder]`
//! impls), the name is a field of that type's namespace ([`DotValues`]), which also gives
//! rust-analyzer highlighting and completions. Elsewhere it converts through `Into`/`IntoProp`
//! like any value.
//! In a closure that returns one of several values (`if on { .accent } else { .secondary }`)
//! write the type out: `ThemeColor::Accent`.
//!
//! The names Elegance's own types answer to are aliases here (`elegance::dot::leading`).

#![allow(non_camel_case_types)]

use crate::animation::{Animation, Transition};
use crate::app::TitleBar;
use crate::color::{Color, ColorScheme, Material, ShapeStyle, ThemeColor};
use crate::events::{Key, PointerStyle};
use crate::font::{Font, Weight};
use crate::geometry::{
    Alignment, Axis, ContentMode, CornerRadii, EdgeInsets, Edges, HorizontalAlignment, Len,
    UnitPoint, VerticalAlignment,
};
use crate::hot_reload::HotReloadShow;
use crate::media::VideoFormat;
use crate::path::{FillRule, LineCap, LineJoin};
use crate::text::TextAlignment;
use crate::view::{AccessibilityRole, BlendMode, PopoverEdge};
use crate::views::{MenuEdge, PickerStyle, ScrollAxes};
use crate::window::WindowControlsStyle;

use crate::prop::Prop;
use elegance_macros::dots;

/// `.name` in `elg!`: a value whose type is the name, spelled with [`chars`].
pub struct Dot<Name>(std::marker::PhantomData<fn() -> Name>);

impl<Name> Dot<Name> {
    pub const NEW: Self = Self(std::marker::PhantomData);
}

impl<Name> Clone for Dot<Name> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Name> Copy for Dot<Name> {}

impl<Name> Default for Dot<Name> {
    fn default() -> Self {
        Self::NEW
    }
}

impl<Name> PartialEq for Dot<Name> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl<Name> Eq for Dot<Name> {}

impl<Name> std::hash::Hash for Dot<Name> {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

impl<Name> std::fmt::Debug for Dot<Name> {
    // Not `f`: the key `.f` is a name in this module.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Dot")
    }
}

/// The letters names are spelled with: `Dot<(a, b, o, v, e)>` is `.above`. Digits are `_0`..`_9`
/// and `_` is `__`.
#[doc(hidden)]
pub mod chars {
    macro_rules! chars {
        ($($c:ident)*) => {$(
            #[derive(Clone, Copy, Debug)]
            pub enum $c {}
        )*};
    }
    chars!(
        a b c d e f g h i j k l m n o p q r s t u v w x y z
        A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
        _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 __
    );
}

/// The values a leading dot can name where this type is expected, as fields: `.compact` in
/// `elg!` reads the field `compact` (so rust-analyzer highlights and completes it). A field holds
/// the value itself, or, for a type that takes other types' values, a [`Dot`] that converts.
/// `#[derive(DotValues)]` implements it for enums.
pub trait DotValues {
    type Values: Default;

    fn dot_values() -> Self::Values {
        Self::Values::default()
    }
}

/// The namespace lookup `elg!` does at a call site: `(&target).__elg_values(PhantomData::<L>)`
/// picks the parameter type's [`DotValues`] when it has one (this trait), and otherwise `L`, a
/// struct the call site declares holding just the names it uses as [`Dot`]s
/// ([`LocalDotValues`]). Defined once here, not per call site: nested `elg!` calls see the
/// enclosing block's items, and two call sites' traits would make the method ambiguous.
#[doc(hidden)]
pub trait TypedDotValues {
    type Values;
    fn __elg_values<L>(self, local: std::marker::PhantomData<L>) -> Self::Values;
}

impl<T: DotValues + ?Sized> TypedDotValues for &DotTarget<T> {
    type Values = T::Values;
    fn __elg_values<L>(self, _: std::marker::PhantomData<L>) -> T::Values {
        T::dot_values()
    }
}

#[doc(hidden)]
pub trait LocalDotValues {
    fn __elg_values<L: Default>(self, local: std::marker::PhantomData<L>) -> L;
}

impl<T: ?Sized> LocalDotValues for &&DotTarget<T> {
    fn __elg_values<L: Default>(self, _: std::marker::PhantomData<L>) -> L {
        L::default()
    }
}

/// An enum made with `#[derive(DotValues)]`, so a reactive `Prop` of it takes its names too.
pub trait DotEnum: DotValues {}

impl<T: DotEnum> DotValues for Prop<T> {
    type Values = T::Values;
}

/// A zero-sized argument-type witness used to resolve leading-dot namespaces.
///
/// Creating a witness does not require `T: DotValues`: types without a namespace
/// retain global marker lookup instead of acquiring a new bound.
pub struct DotTarget<T: ?Sized>(std::marker::PhantomData<fn() -> *const T>);

impl<T: ?Sized> DotTarget<T> {
    pub const fn new() -> Self {
        Self(std::marker::PhantomData)
    }

    #[doc(hidden)]
    pub fn constrain(&self, _: &T) {}
}

impl<T: ?Sized> Default for DotTarget<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Sentinel for builder arguments without a generated type witness.
#[doc(hidden)]
pub enum NoDotValues {}

/// Resolves a type witness to its typed namespace, or to the global fallback.
///
/// Import this trait and call `(&target).resolve_dot_values()`: Rust selects the
/// `&DotTarget<T>` implementation when `T: DotValues`, otherwise it autoref-borrows
/// the witness for the `&&DotTarget<T>` fallback. Type aliases and imported names
/// therefore work without the macro having to resolve their paths.
pub trait ResolveDotValues {
    type Values;

    fn resolve_dot_values(self) -> Self::Values;
}

impl<T: DotValues + ?Sized> ResolveDotValues for &DotTarget<T> {
    type Values = T::Values;

    fn resolve_dot_values(self) -> Self::Values {
        T::dot_values()
    }
}

impl<T: ?Sized> ResolveDotValues for &&DotTarget<T> {
    type Values = Values;

    fn resolve_dot_values(self) -> Self::Values {
        Values::default()
    }
}

dots! {
    // Alignment, edges, unit points.
    leading {
        HorizontalAlignment => HorizontalAlignment::Leading,
        Alignment => Alignment::LEADING,
        TextAlignment => TextAlignment::Leading,
        Edges => Edges::LEADING,
        UnitPoint => UnitPoint::LEADING,
        PopoverEdge => PopoverEdge::Leading,
    }
    center {
        HorizontalAlignment => HorizontalAlignment::Center,
        VerticalAlignment => VerticalAlignment::Center,
        Alignment => Alignment::CENTER,
        TextAlignment => TextAlignment::Center,
        UnitPoint => UnitPoint::CENTER,
    }
    trailing {
        HorizontalAlignment => HorizontalAlignment::Trailing,
        Alignment => Alignment::TRAILING,
        TextAlignment => TextAlignment::Trailing,
        Edges => Edges::TRAILING,
        UnitPoint => UnitPoint::TRAILING,
        PopoverEdge => PopoverEdge::Trailing,
    }
    top {
        VerticalAlignment => VerticalAlignment::Top,
        Alignment => Alignment::TOP,
        Edges => Edges::TOP,
        UnitPoint => UnitPoint::TOP,
    }
    bottom {
        VerticalAlignment => VerticalAlignment::Bottom,
        Alignment => Alignment::BOTTOM,
        Edges => Edges::BOTTOM,
        UnitPoint => UnitPoint::BOTTOM,
    }
    top_leading { Alignment => Alignment::TOP_LEADING, UnitPoint => UnitPoint::TOP_LEADING }
    top_trailing { Alignment => Alignment::TOP_TRAILING, UnitPoint => UnitPoint::TOP_TRAILING }
    bottom_leading { Alignment => Alignment::BOTTOM_LEADING, UnitPoint => UnitPoint::BOTTOM_LEADING }
    bottom_trailing { Alignment => Alignment::BOTTOM_TRAILING, UnitPoint => UnitPoint::BOTTOM_TRAILING }
    horizontal { Axis => Axis::Horizontal, Edges => Edges::HORIZONTAL, ScrollAxes => ScrollAxes::Horizontal }
    vertical { Axis => Axis::Vertical, Edges => Edges::VERTICAL, ScrollAxes => ScrollAxes::Vertical }
    all { Edges => Edges::ALL }
    both { ScrollAxes => ScrollAxes::Both }
    // Also a Number (so a Len): `.frame(max_width: .infinity)`.
    infinity { f32 => f32::INFINITY }

    fit { ContentMode => ContentMode::Fit }
    fill { ContentMode => ContentMode::Fill }

    // Popups and pickers.
    below { PopoverEdge => PopoverEdge::Below, MenuEdge => MenuEdge::Below }
    above { PopoverEdge => PopoverEdge::Above, MenuEdge => MenuEdge::Above }
    segmented { PickerStyle => PickerStyle::Segmented }
    menu { PickerStyle => PickerStyle::Menu, Key => Key::Menu }

    // Colors.
    background { ThemeColor => ThemeColor::Background }
    surface { ThemeColor => ThemeColor::Surface }
    surface_raised { ThemeColor => ThemeColor::SurfaceRaised }
    outline { ThemeColor => ThemeColor::Outline }
    primary { ThemeColor => ThemeColor::Primary }
    secondary { ThemeColor => ThemeColor::Secondary }
    tertiary { ThemeColor => ThemeColor::Tertiary }
    accent { ThemeColor => ThemeColor::Accent }
    on_accent { ThemeColor => ThemeColor::OnAccent }
    danger { ThemeColor => ThemeColor::Danger }
    success { ThemeColor => ThemeColor::Success }
    warning { ThemeColor => ThemeColor::Warning }
    white { Color => Color::WHITE }
    clear { Color => Color::CLEAR }
    light { ColorScheme => ColorScheme::Light, Weight => Weight::Light }
    dark { ColorScheme => ColorScheme::Dark }

    // Materials (frosted glass): `.background(.thin_material)`.
    ultra_thin_material { Material => Material::ULTRA_THIN }
    thin_material { Material => Material::THIN }
    regular_material { Material => Material::REGULAR }
    thick_material { Material => Material::THICK }
    ultra_thick_material { Material => Material::ULTRA_THICK }

    // Blend modes.
    normal { BlendMode => BlendMode::Normal }
    multiply { BlendMode => BlendMode::Multiply }
    darken { BlendMode => BlendMode::Darken }
    color_burn { BlendMode => BlendMode::ColorBurn }
    plus_darker { BlendMode => BlendMode::PlusDarker }
    screen { BlendMode => BlendMode::Screen }
    lighten { BlendMode => BlendMode::Lighten }
    color_dodge { BlendMode => BlendMode::ColorDodge }
    plus_lighter { BlendMode => BlendMode::PlusLighter }
    overlay { BlendMode => BlendMode::Overlay }
    soft_light { BlendMode => BlendMode::SoftLight }
    hard_light { BlendMode => BlendMode::HardLight }
    difference { BlendMode => BlendMode::Difference }
    exclusion { BlendMode => BlendMode::Exclusion }
    hue { BlendMode => BlendMode::Hue }
    saturation { BlendMode => BlendMode::Saturation }
    color { BlendMode => BlendMode::Color }
    luminosity { BlendMode => BlendMode::Luminosity }

    // Weights and fonts.
    ultra_light { Weight => Weight::UltraLight }
    thin { Weight => Weight::Thin }
    regular { Weight => Weight::Regular }
    medium { Weight => Weight::Medium }
    semibold { Weight => Weight::Semibold }
    bold { Weight => Weight::Bold }
    heavy { Weight => Weight::Heavy }
    black { Weight => Weight::Black, Color => Color::BLACK }
    large_title { Font => Font::large_title() }
    title { Font => Font::title() }
    title2 { Font => Font::title2() }
    title3 { Font => Font::title3() }
    headline { Font => Font::headline() }
    body { Font => Font::body() }
    callout { Font => Font::callout() }
    subheadline { Font => Font::subheadline() }
    footnote { Font => Font::footnote() }
    caption { Font => Font::caption(), WindowControlsStyle => WindowControlsStyle::Caption }
    caption2 { Font => Font::caption2() }

    // Animations and transitions.
    linear { Animation => Animation::linear(0.25) }
    ease_in { Animation => Animation::ease_in(0.25) }
    ease_out { Animation => Animation::ease_out(0.25) }
    ease_in_out { Animation => Animation::ease_in_out(0.35) }
    spring { Animation => Animation::spring(0.5, 0.825) }
    smooth { Animation => Animation::smooth(0.4) }
    opacity { Transition => Transition::opacity() }
    identity { Transition => Transition::identity() }
    scale { Transition => Transition::scale(0.0) }
    slide { Transition => Transition::slide() }

    // Paths.
    non_zero { FillRule => FillRule::NonZero }
    even_odd { FillRule => FillRule::EvenOdd }
    butt { LineCap => LineCap::Butt }
    round { LineCap => LineCap::Round, LineJoin => LineJoin::Round }
    square { LineCap => LineCap::Square }
    miter { LineJoin => LineJoin::Miter }
    bevel { LineJoin => LineJoin::Bevel }

    // Accessibility roles.
    group { AccessibilityRole => AccessibilityRole::Group }
    label { AccessibilityRole => AccessibilityRole::Label }
    heading { AccessibilityRole => AccessibilityRole::Heading }
    button { AccessibilityRole => AccessibilityRole::Button }
    link { AccessibilityRole => AccessibilityRole::Link }
    image { AccessibilityRole => AccessibilityRole::Image }
    toggle { AccessibilityRole => AccessibilityRole::Toggle }
    checkbox { AccessibilityRole => AccessibilityRole::Checkbox }
    slider { AccessibilityRole => AccessibilityRole::Slider }
    progress { AccessibilityRole => AccessibilityRole::Progress, PointerStyle => PointerStyle::Progress }
    text_field { AccessibilityRole => AccessibilityRole::TextField }
    list { AccessibilityRole => AccessibilityRole::List }
    list_item { AccessibilityRole => AccessibilityRole::ListItem }
    tab { AccessibilityRole => AccessibilityRole::Tab, Key => Key::Tab }
    menu_item { AccessibilityRole => AccessibilityRole::MenuItem }

    // Windows.
    system { TitleBar => TitleBar::System }
    hidden { TitleBar => TitleBar::Hidden }
    integrated { TitleBar => TitleBar::Integrated }
    automatic { WindowControlsStyle => WindowControlsStyle::Automatic, PointerStyle => PointerStyle::Automatic }
    traffic_lights { WindowControlsStyle => WindowControlsStyle::TrafficLights }
    root { HotReloadShow => HotReloadShow::Root }
    previews { HotReloadShow => HotReloadShow::Previews }

    // Pointer styles (`automatic` and `progress` are above).
    arrow { PointerStyle => PointerStyle::Arrow }
    text { PointerStyle => PointerStyle::Text }
    pointer { PointerStyle => PointerStyle::Pointer }
    crosshair { PointerStyle => PointerStyle::Crosshair }
    r#move { PointerStyle => PointerStyle::Move }
    wait { PointerStyle => PointerStyle::Wait }
    resize_horizontal { PointerStyle => PointerStyle::ResizeHorizontal }
    resize_vertical { PointerStyle => PointerStyle::ResizeVertical }
    resize_nwse { PointerStyle => PointerStyle::ResizeNwse }
    resize_nesw { PointerStyle => PointerStyle::ResizeNesw }
    not_allowed { PointerStyle => PointerStyle::NotAllowed }

    // Video frame formats.
    bgra { VideoFormat => VideoFormat::Bgra }
    nv12 { VideoFormat => VideoFormat::Nv12 }
    i420 { VideoFormat => VideoFormat::I420 }

    // Keys.
    a { Key => Key::A } b { Key => Key::B } c { Key => Key::C } d { Key => Key::D }
    e { Key => Key::E } f { Key => Key::F } g { Key => Key::G } h { Key => Key::H }
    i { Key => Key::I } j { Key => Key::J } k { Key => Key::K } l { Key => Key::L }
    m { Key => Key::M } n { Key => Key::N } o { Key => Key::O } p { Key => Key::P }
    q { Key => Key::Q } r { Key => Key::R } s { Key => Key::S } t { Key => Key::T }
    u { Key => Key::U } v { Key => Key::V } w { Key => Key::W } x { Key => Key::X }
    y { Key => Key::Y } z { Key => Key::Z }
    digit0 { Key => Key::Digit0 } digit1 { Key => Key::Digit1 } digit2 { Key => Key::Digit2 }
    digit3 { Key => Key::Digit3 } digit4 { Key => Key::Digit4 } digit5 { Key => Key::Digit5 }
    digit6 { Key => Key::Digit6 } digit7 { Key => Key::Digit7 } digit8 { Key => Key::Digit8 }
    digit9 { Key => Key::Digit9 }
    enter { Key => Key::Enter } escape { Key => Key::Escape } backspace { Key => Key::Backspace }
    space { Key => Key::Space } forward_delete { Key => Key::ForwardDelete } insert { Key => Key::Insert }
    home { Key => Key::Home } end { Key => Key::End } page_up { Key => Key::PageUp } page_down { Key => Key::PageDown }
    left { Key => Key::Left } right { Key => Key::Right } up { Key => Key::Up } down { Key => Key::Down }
    f1 { Key => Key::F1 } f2 { Key => Key::F2 } f3 { Key => Key::F3 } f4 { Key => Key::F4 }
    f5 { Key => Key::F5 } f6 { Key => Key::F6 } f7 { Key => Key::F7 } f8 { Key => Key::F8 }
    f9 { Key => Key::F9 } f10 { Key => Key::F10 } f11 { Key => Key::F11 } f12 { Key => Key::F12 }

    // Existing blanket and reactive conversions, without new From implementations.
    namespaces {
        Len => [f32],
        EdgeInsets => [f32],
        CornerRadii => [f32],
        Prop<Color> => [Color, ThemeColor],
        Prop<ShapeStyle> => [Color, ThemeColor, Material],
        Prop<Font> => [Font],
        Prop<ColorScheme> => [ColorScheme],
        Prop<PointerStyle> => [PointerStyle],
    }
}
