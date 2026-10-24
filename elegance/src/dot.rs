//! Markers behind the DSL's leading-dot shorthand: in `elg!`, `.leading` becomes
//! `elegance::dot::leading`, which converts into whichever type the parameter wants
//! (`HorizontalAlignment::Leading`, `Alignment::LEADING`, `Edges::LEADING`, ...).
//!
//! Each marker is its own type, so the shorthand works where the parameter's type is known. In a
//! closure that returns one of several values (`if on { .accent } else { .secondary }`) write the
//! enum out: `ThemeColor::Accent`.

#![allow(non_camel_case_types)]

use crate::animation::{Animation, Transition};
use crate::app::TitleBar;
use crate::color::{Color, ColorScheme, Material, ThemeColor};
use crate::events::{Cursor, Key};
use crate::font::{Font, Weight};
use crate::geometry::{Alignment, Axis, ContentMode, Edges, HorizontalAlignment, UnitPoint, VerticalAlignment};
use crate::hot_reload::HotReloadShow;
use crate::media::VideoFormat;
use crate::path::{FillRule, LineCap, LineJoin};
use crate::text::TextAlignment;
use crate::view::{AccessibilityRole, BlendMode, PopoverEdge};
use crate::views::{MenuEdge, PickerStyle, ScrollAxes};
use crate::window::WindowControlsStyle;

macro_rules! dots {
    ($( $name:ident { $($target:ty => $value:expr),* $(,)? } )*) => {$(
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct $name;
        $(
            impl From<$name> for $target {
                fn from(_: $name) -> $target {
                    $value
                }
            }
        )*
    )*};
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
    progress { AccessibilityRole => AccessibilityRole::Progress }
    text_field { AccessibilityRole => AccessibilityRole::TextField }
    list { AccessibilityRole => AccessibilityRole::List }
    list_item { AccessibilityRole => AccessibilityRole::ListItem }
    tab { AccessibilityRole => AccessibilityRole::Tab, Key => Key::Tab }
    menu_item { AccessibilityRole => AccessibilityRole::MenuItem }

    // Windows.
    system { TitleBar => TitleBar::System }
    hidden { TitleBar => TitleBar::Hidden }
    automatic { WindowControlsStyle => WindowControlsStyle::Automatic }
    traffic_lights { WindowControlsStyle => WindowControlsStyle::TrafficLights }
    root { HotReloadShow => HotReloadShow::Root }
    previews { HotReloadShow => HotReloadShow::Previews }

    // Cursors.
    arrow { Cursor => Cursor::Arrow }
    text { Cursor => Cursor::Text }
    pointer { Cursor => Cursor::Pointer }
    crosshair { Cursor => Cursor::Crosshair }
    r#move { Cursor => Cursor::Move }
    resize_horizontal { Cursor => Cursor::ResizeHorizontal }
    resize_vertical { Cursor => Cursor::ResizeVertical }
    resize_nwse { Cursor => Cursor::ResizeNwse }
    resize_nesw { Cursor => Cursor::ResizeNesw }
    not_allowed { Cursor => Cursor::NotAllowed }

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
}
