//! Colors, theme tokens, gradients, and shape styles.

use elegence_sys as sys;

use crate::geometry::UnitPoint;

/// A color in linear-light RGBA components 0...1 (as `elg::Color`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl Color {
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const CLEAR: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 1.0)
    }
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> Self {
        Self { r: red as f32 / 255.0, g: green as f32 / 255.0, b: blue as f32 / 255.0, a: alpha }
    }
    /// `Color::hex(0x0A84FF)`
    pub const fn hex(rrggbb: u32) -> Self {
        Self::hex_alpha(rrggbb, 1.0)
    }
    pub const fn hex_alpha(rrggbb: u32, alpha: f32) -> Self {
        Self::rgba((rrggbb >> 16) as u8, (rrggbb >> 8) as u8, rrggbb as u8, alpha)
    }
    pub fn opacity(self, alpha: f32) -> Self {
        Self { a: self.a * alpha, ..self }
    }
    pub fn mix(self, other: Color, amount: f32) -> Self {
        Self {
            r: self.r + (other.r - self.r) * amount,
            g: self.g + (other.g - self.g) * amount,
            b: self.b + (other.b - self.b) * amount,
            a: self.a + (other.a - self.a) * amount,
        }
    }
    pub(crate) fn c(self) -> sys::ElgColor {
        sys::ElgColor { r: self.r, g: self.g, b: self.b, a: self.a }
    }
    pub(crate) fn from_c(c: sys::ElgColor) -> Self {
        Self { r: c.r, g: c.g, b: c.b, a: c.a }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}

/// Semantic colors, resolved against the theme and color scheme where they're used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeColor {
    /// Window background.
    Background,
    /// Cards, panels.
    Surface,
    /// Controls and panels on surfaces.
    SurfaceRaised,
    /// Borders, separators.
    Outline,
    /// Main text and icons.
    Primary,
    /// Supporting text.
    Secondary,
    /// Hints, disabled text.
    Tertiary,
    /// The tint: prominent buttons, selection.
    Accent,
    /// Text and icons on the accent.
    OnAccent,
    Danger,
    Success,
    Warning,
}

/// One color scheme's tokens.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorTokens {
    pub background: Color,
    pub surface: Color,
    pub surface_raised: Color,
    pub outline: Color,
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub danger: Color,
    pub success: Color,
    pub warning: Color,
}

impl ColorTokens {
    pub fn get(&self, token: ThemeColor) -> Color {
        match token {
            ThemeColor::Background => self.background,
            ThemeColor::Surface => self.surface,
            ThemeColor::SurfaceRaised => self.surface_raised,
            ThemeColor::Outline => self.outline,
            ThemeColor::Primary => self.primary,
            ThemeColor::Secondary => self.secondary,
            ThemeColor::Tertiary => self.tertiary,
            ThemeColor::Accent => self.accent,
            ThemeColor::OnAccent => self.on_accent,
            ThemeColor::Danger => self.danger,
            ThemeColor::Success => self.success,
            ThemeColor::Warning => self.warning,
        }
    }
}

impl std::ops::Index<ThemeColor> for ColorTokens {
    type Output = Color;
    fn index(&self, token: ThemeColor) -> &Color {
        match token {
            ThemeColor::Background => &self.background,
            ThemeColor::Surface => &self.surface,
            ThemeColor::SurfaceRaised => &self.surface_raised,
            ThemeColor::Outline => &self.outline,
            ThemeColor::Primary => &self.primary,
            ThemeColor::Secondary => &self.secondary,
            ThemeColor::Tertiary => &self.tertiary,
            ThemeColor::Accent => &self.accent,
            ThemeColor::OnAccent => &self.on_accent,
            ThemeColor::Danger => &self.danger,
            ThemeColor::Success => &self.success,
            ThemeColor::Warning => &self.warning,
        }
    }
}

/// Colors for both color schemes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub light: ColorTokens,
    pub dark: ColorTokens,
}

impl Default for Theme {
    fn default() -> Self {
        Self::standard()
    }
}

impl Theme {
    pub fn colors(&self, scheme: ColorScheme) -> &ColorTokens {
        match scheme {
            ColorScheme::Light => &self.light,
            ColorScheme::Dark => &self.dark,
        }
    }

    /// Elegence's default theme.
    pub const fn standard() -> Self {
        Self {
            light: ColorTokens {
                background: Color::hex(0xF5F5F7),
                surface: Color::hex(0xFFFFFF),
                surface_raised: Color::hex(0xECECEF),
                outline: Color::hex(0xD1D1D6),
                primary: Color::hex(0x1C1C1E),
                secondary: Color::hex(0x6C6C70),
                tertiary: Color::hex(0xAEAEB2),
                accent: Color::hex(0x0A84FF),
                on_accent: Color::WHITE,
                danger: Color::hex(0xFF3B30),
                success: Color::hex(0x34C759),
                warning: Color::hex(0xFF9F0A),
            },
            dark: ColorTokens {
                background: Color::hex(0x1C1C1E),
                surface: Color::hex(0x2C2C2E),
                surface_raised: Color::hex(0x3A3A3C),
                outline: Color::hex(0x48484A),
                primary: Color::hex(0xF2F2F7),
                secondary: Color::hex(0xAEAEB2),
                tertiary: Color::hex(0x636366),
                accent: Color::hex(0x0A84FF),
                on_accent: Color::WHITE,
                danger: Color::hex(0xFF453A),
                success: Color::hex(0x30D158),
                warning: Color::hex(0xFFD60A),
            },
        }
    }

    pub(crate) fn c(&self) -> sys::ElgTheme {
        // Same layout: 24 colors of four f32s.
        unsafe { std::mem::transmute::<Theme, sys::ElgTheme>(*self) }
    }
    pub(crate) fn from_c(c: sys::ElgTheme) -> Self {
        unsafe { std::mem::transmute::<sys::ElgTheme, Theme>(c) }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GradientStop {
    pub color: Color,
    /// 0...1 along the gradient.
    pub location: f32,
}

impl GradientStop {
    pub const fn new(color: Color, location: f32) -> Self {
        Self { color, location }
    }
}

fn even_stops(colors: impl IntoIterator<Item = Color>) -> Vec<GradientStop> {
    let colors: Vec<Color> = colors.into_iter().collect();
    let n = colors.len();
    colors
        .into_iter()
        .enumerate()
        .map(|(i, color)| GradientStop::new(color, if n > 1 { i as f32 / (n - 1) as f32 } else { 0.0 }))
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    pub stops: Vec<GradientStop>,
    pub start: UnitPoint,
    pub end: UnitPoint,
}

impl LinearGradient {
    pub fn new(stops: Vec<GradientStop>, start: UnitPoint, end: UnitPoint) -> Self {
        Self { stops, start, end }
    }
    /// Evenly spaced colors, top to bottom.
    pub fn colors(colors: impl IntoIterator<Item = Color>) -> Self {
        Self::new(even_stops(colors), UnitPoint::TOP, UnitPoint::BOTTOM)
    }
    pub fn from_to(mut self, start: UnitPoint, end: UnitPoint) -> Self {
        self.start = start;
        self.end = end;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RadialGradient {
    pub stops: Vec<GradientStop>,
    pub center: UnitPoint,
    pub start_radius: f32,
    pub end_radius: f32,
}

impl RadialGradient {
    pub fn new(stops: Vec<GradientStop>, center: UnitPoint, start_radius: f32, end_radius: f32) -> Self {
        Self { stops, center, start_radius, end_radius }
    }
    /// Evenly spaced colors from the center out to 100 points.
    pub fn colors(colors: impl IntoIterator<Item = Color>) -> Self {
        Self::new(even_stops(colors), UnitPoint::CENTER, 0.0, 100.0)
    }
}

/// Colors blend outward from `center` along ellipses shaped like the filled bounds. The radii are
/// fractions of the bounds' size: 0.5 reaches the middle of each edge.
#[derive(Clone, Debug, PartialEq)]
pub struct EllipticalGradient {
    pub stops: Vec<GradientStop>,
    pub center: UnitPoint,
    pub start_radius_fraction: f32,
    pub end_radius_fraction: f32,
}

impl EllipticalGradient {
    pub fn new(stops: Vec<GradientStop>, center: UnitPoint, start_radius_fraction: f32, end_radius_fraction: f32) -> Self {
        Self { stops, center, start_radius_fraction, end_radius_fraction }
    }
    /// Evenly spaced colors from the center out to the edges.
    pub fn colors(colors: impl IntoIterator<Item = Color>) -> Self {
        Self::new(even_stops(colors), UnitPoint::CENTER, 0.0, 0.5)
    }
}

/// Colors sweep around `center` (a conic gradient), clockwise from `start_angle` to `end_angle`,
/// in degrees; 0 points trailing (right), 90 down.
#[derive(Clone, Debug, PartialEq)]
pub struct AngularGradient {
    pub stops: Vec<GradientStop>,
    pub center: UnitPoint,
    pub start_angle: f32,
    pub end_angle: f32,
}

impl AngularGradient {
    pub fn new(stops: Vec<GradientStop>, center: UnitPoint, start_angle: f32, end_angle: f32) -> Self {
        Self { stops, center, start_angle, end_angle }
    }
    /// Evenly spaced colors, a full turn around the center.
    pub fn colors(colors: impl IntoIterator<Item = Color>) -> Self {
        Self::new(even_stops(colors), UnitPoint::CENTER, 0.0, 360.0)
    }
    pub fn angles(mut self, start: f32, end: f32) -> Self {
        self.start_angle = start;
        self.end_angle = end;
        self
    }
}

/// Frosted glass: the shape shows what's drawn behind it in the window, blurred, with its colors
/// boosted, under a translucent tint and a little grain. The presets follow the color scheme:
///
/// ```ignore
/// Text("Now playing").padding(12).background_in(Material::REGULAR, RoundedRectangle(12))
/// Rectangle().fill(.thin_material)
/// ```
///
/// Inside an offscreen group (opacity below 1, a shadow, a blur, ...) a material sees only what
/// the group drew behind it, as CSS's backdrop-filter does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// Blur radius in points.
    pub blur: f32,
    /// Of what shows through; 1 keeps the colors.
    pub saturation: f32,
    pub tint: Color,
    /// The tint in dark mode; `None` uses `tint` in both.
    pub dark_tint: Option<Color>,
    /// Grain, 0...1, so smooth gradients don't band.
    pub noise: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self::REGULAR
    }
}

impl Material {
    const fn preset(blur: f32, light_alpha: f32, dark_alpha: f32) -> Self {
        Self {
            blur,
            saturation: 1.8,
            tint: Color::new(0.97, 0.97, 0.98, light_alpha),
            dark_tint: Some(Color::new(0.11, 0.11, 0.12, dark_alpha)),
            noise: 0.02,
        }
    }
    pub const ULTRA_THIN: Material = Material::preset(24.0, 0.2, 0.3);
    pub const THIN: Material = Material::preset(28.0, 0.38, 0.45);
    pub const REGULAR: Material = Material::preset(30.0, 0.55, 0.6);
    pub const THICK: Material = Material::preset(36.0, 0.72, 0.74);
    pub const ULTRA_THICK: Material = Material::preset(40.0, 0.86, 0.86);

    pub fn blur(mut self, radius: f32) -> Self {
        self.blur = radius;
        self
    }
    pub fn saturation(mut self, saturation: f32) -> Self {
        self.saturation = saturation;
        self
    }
    /// The tint in both color schemes.
    pub fn tint(mut self, tint: impl Into<Color>) -> Self {
        self.tint = tint.into();
        self.dark_tint = None;
        self
    }
    pub fn dark_tint(mut self, tint: impl Into<Color>) -> Self {
        self.dark_tint = Some(tint.into());
        self
    }
    pub fn noise(mut self, noise: f32) -> Self {
        self.noise = noise;
        self
    }
}

/// How a shape or background is filled: a color, a gradient, or a material.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeStyle {
    Color(Color),
    Linear(LinearGradient),
    Radial(RadialGradient),
    Elliptical(EllipticalGradient),
    Angular(AngularGradient),
    Material(Material),
}

impl From<Color> for ShapeStyle {
    fn from(c: Color) -> Self {
        ShapeStyle::Color(c)
    }
}
impl From<LinearGradient> for ShapeStyle {
    fn from(g: LinearGradient) -> Self {
        ShapeStyle::Linear(g)
    }
}
impl From<RadialGradient> for ShapeStyle {
    fn from(g: RadialGradient) -> Self {
        ShapeStyle::Radial(g)
    }
}
impl From<EllipticalGradient> for ShapeStyle {
    fn from(g: EllipticalGradient) -> Self {
        ShapeStyle::Elliptical(g)
    }
}
impl From<AngularGradient> for ShapeStyle {
    fn from(g: AngularGradient) -> Self {
        ShapeStyle::Angular(g)
    }
}
impl From<Material> for ShapeStyle {
    fn from(m: Material) -> Self {
        ShapeStyle::Material(m)
    }
}

fn c_stops(stops: &[GradientStop]) -> Vec<sys::ElgGradientStop> {
    stops.iter().map(|s| sys::ElgGradientStop { color: s.color.c(), location: s.location }).collect()
}

impl ShapeStyle {
    /// Calls `f` with the C form (valid for the call).
    pub(crate) fn with_c<R>(&self, f: impl FnOnce(&sys::ElgShapeStyle) -> R) -> R {
        let mut c = sys::ElgShapeStyle {
            kind: 0,
            color: sys::ElgColor::default(),
            stops: std::ptr::null(),
            stop_count: 0,
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
            r0: 0.0,
            r1: 0.0,
            blur: 0.0,
            saturation: 0.0,
            noise: 0.0,
            has_dark_tint: false,
            dark_tint: sys::ElgColor::default(),
        };
        let stops = match self {
            ShapeStyle::Color(color) => {
                c.color = color.c();
                Vec::new()
            }
            ShapeStyle::Linear(g) => {
                c.kind = 1;
                (c.x0, c.y0, c.x1, c.y1) = (g.start.x, g.start.y, g.end.x, g.end.y);
                c_stops(&g.stops)
            }
            ShapeStyle::Radial(g) => {
                c.kind = 2;
                (c.x0, c.y0, c.r0, c.r1) = (g.center.x, g.center.y, g.start_radius, g.end_radius);
                c_stops(&g.stops)
            }
            ShapeStyle::Elliptical(g) => {
                c.kind = 3;
                (c.x0, c.y0, c.r0, c.r1) = (g.center.x, g.center.y, g.start_radius_fraction, g.end_radius_fraction);
                c_stops(&g.stops)
            }
            ShapeStyle::Angular(g) => {
                c.kind = 4;
                (c.x0, c.y0, c.r0, c.r1) = (g.center.x, g.center.y, g.start_angle, g.end_angle);
                c_stops(&g.stops)
            }
            ShapeStyle::Material(m) => {
                c.kind = 5;
                (c.blur, c.saturation, c.noise, c.color) = (m.blur, m.saturation, m.noise, m.tint.c());
                if let Some(dark) = m.dark_tint {
                    (c.has_dark_tint, c.dark_tint) = (true, dark.c());
                }
                Vec::new()
            }
        };
        c.stops = stops.as_ptr();
        c.stop_count = stops.len();
        f(&c)
    }
}
