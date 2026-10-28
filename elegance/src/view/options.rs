//! The options and values [`ViewExt`](super::ViewExt)'s modifiers take.

use std::path::PathBuf;

use elegance_sys as sys;

use crate::animation::{Animation, Transition};
use crate::color::Color;
use crate::geometry::{Alignment, ContentMode, Len, Number, Point, UnitPoint};
use crate::prop::{with_prop, IntoProp, Prop};

/// What a view is to screen readers, when the automatic choice isn't right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessibilityRole {
    Group,
    Label,
    Heading,
    Button,
    Link,
    Image,
    Toggle,
    Checkbox,
    Slider,
    Progress,
    TextField,
    List,
    ListItem,
    Tab,
    MenuItem,
}

/// The side of its anchor (the view it's attached to) floating content opens on. It flips to the
/// opposite side when there isn't room.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FloatingEdge {
    #[default]
    Bottom,
    Top,
    Trailing,
    Leading,
}

/// Across that edge: lined up with the anchor's start (leading or top), center, or end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FloatingAlignment {
    #[default]
    Start,
    Center,
    End,
}

/// Where floating content may go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FloatingBoundary {
    /// In a popup window of its own: it can extend past the app's window, and stays on the
    /// display.
    #[default]
    Screen,
    /// Drawn inside the app's window, above its content, and kept within it.
    Window,
    /// Drawn inside the nearest view around it marked `.floating_container()` (else the window):
    /// clipped to it, under views outside it, moving with it as it scrolls.
    Container,
}

/// Where floating content goes, by its anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FloatingPlacement {
    pub edge: FloatingEdge,
    pub alignment: FloatingAlignment,
    /// Between the anchor and the content (negative: overlapping).
    pub spacing: f32,
    /// Then moved by this.
    pub offset: Point,
    /// Instead of an edge: the content's top-leading corner here, in the anchor's coordinates.
    pub position: Option<Point>,
}

#[elegance_macros::elg_builder]
impl FloatingPlacement {
    pub fn edge(mut self, edge: impl Into<FloatingEdge>) -> Self {
        self.edge = edge.into();
        self
    }
    pub fn alignment(mut self, alignment: impl Into<FloatingAlignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
    pub fn spacing(mut self, spacing: impl Number) -> Self {
        self.spacing = spacing.to_f32();
        self
    }
    pub fn offset(mut self, offset: impl Into<Point>) -> Self {
        self.offset = offset.into();
        self
    }
    pub fn position(mut self, position: impl Into<Point>) -> Self {
        self.position = Some(position.into());
        self
    }
    pub(crate) fn c(&self) -> sys::ElgFloatingPlacement {
        sys::ElgFloatingPlacement {
            edge: self.edge as i32,
            alignment: self.alignment as i32,
            spacing: self.spacing,
            offset: self.offset.c(),
            has_position: self.position.is_some(),
            position: self.position.unwrap_or_default().c(),
        }
    }
}

/// How popup content opens and closes: a transition run with an animation.
///
/// ```ignore
/// PopupMotion::new(Transition::opacity().combined(Transition::scale_anchored(0.96, Alignment::TOP)),
///                  Animation::ease_out(0.15))
/// ```
#[derive(Clone)]
pub struct PopupMotion {
    parts: Option<(Transition, Animation)>,
}

impl PopupMotion {
    pub fn new(transition: impl Into<Transition>, animation: impl Into<Animation>) -> Self {
        Self { parts: Some((transition.into(), animation.into())) }
    }
    /// At once, with no animation (the built-in styles' default).
    pub fn none() -> Self {
        Self { parts: None }
    }
}

/// The C form of an optional motion (None: the style's); borrows `motion`.
pub(crate) fn c_motion(motion: Option<&PopupMotion>) -> sys::ElgPopupMotion {
    let parts = motion.and_then(|m| m.parts.as_ref());
    sys::ElgPopupMotion {
        has: motion.is_some(),
        transition: parts.map_or(std::ptr::null(), |(t, _)| t.as_ptr()),
        animation: parts.map_or(std::ptr::null(), |(_, a)| a.as_ptr()),
    }
}

/// What closes floating content (and sets its binding back to false).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatingDismiss {
    /// A press anywhere outside it. While open, it then takes the pointer: presses elsewhere only
    /// close it. false: the rest of the window stays usable.
    pub outside_click: bool,
    /// Esc, unless something focused handles it first.
    pub escape: bool,
}

impl Default for FloatingDismiss {
    fn default() -> Self {
        Self { outside_click: true, escape: true }
    }
}

/// `.floating(...)`'s options.
#[derive(Clone, Default)]
pub struct FloatingOptions {
    pub placement: FloatingPlacement,
    pub boundary: FloatingBoundary,
    pub dismiss: FloatingDismiss,
    /// None: none.
    pub motion: Option<PopupMotion>,
}

#[elegance_macros::elg_builder]
impl FloatingOptions {
    pub fn placement(mut self, placement: impl FnOnce(FloatingPlacement) -> FloatingPlacement) -> Self {
        self.placement = placement(self.placement);
        self
    }
    pub fn boundary(mut self, boundary: impl Into<FloatingBoundary>) -> Self {
        self.boundary = boundary.into();
        self
    }
    pub fn outside_click(mut self, closes: bool) -> Self {
        self.dismiss.outside_click = closes;
        self
    }
    pub fn escape(mut self, closes: bool) -> Self {
        self.dismiss.escape = closes;
        self
    }
    pub fn motion(mut self, motion: PopupMotion) -> Self {
        self.motion = Some(motion);
        self
    }
}

/// `.tooltip(...)`'s options.
#[derive(Clone)]
pub struct TooltipOptions {
    /// Below the view by default.
    pub placement: FloatingPlacement,
    pub boundary: FloatingBoundary,
    /// Seconds the pointer rests on the view before it shows.
    pub delay: f64,
    /// None: the TooltipStyle's.
    pub motion: Option<PopupMotion>,
}

impl Default for TooltipOptions {
    fn default() -> Self {
        Self {
            placement: FloatingPlacement { spacing: 4.0, ..Default::default() },
            boundary: FloatingBoundary::Screen,
            delay: 0.6,
            motion: None,
        }
    }
}

#[elegance_macros::elg_builder]
impl TooltipOptions {
    pub fn placement(mut self, placement: impl FnOnce(FloatingPlacement) -> FloatingPlacement) -> Self {
        self.placement = placement(self.placement);
        self
    }
    pub fn boundary(mut self, boundary: impl Into<FloatingBoundary>) -> Self {
        self.boundary = boundary.into();
        self
    }
    pub fn delay(mut self, seconds: f64) -> Self {
        self.delay = seconds;
        self
    }
    pub fn motion(mut self, motion: PopupMotion) -> Self {
        self.motion = Some(motion);
        self
    }
}

/// `.popover(...)`'s options.
#[derive(Clone)]
pub struct PopoverOptions {
    /// Centered below the view by default.
    pub placement: FloatingPlacement,
    pub boundary: FloatingBoundary,
    /// false: the content reaches the panel's edges (images, lists).
    pub padded: Prop<bool>,
    /// None: the PopoverStyle's.
    pub motion: Option<PopupMotion>,
}

impl Default for PopoverOptions {
    fn default() -> Self {
        Self {
            placement: FloatingPlacement { alignment: FloatingAlignment::Center, spacing: 4.0, ..Default::default() },
            boundary: FloatingBoundary::Screen,
            padded: Prop::Value(true),
            motion: None,
        }
    }
}

#[elegance_macros::elg_builder]
impl PopoverOptions {
    pub fn placement(mut self, placement: impl FnOnce(FloatingPlacement) -> FloatingPlacement) -> Self {
        self.placement = placement(self.placement);
        self
    }
    /// The side it opens on: `.popover(shown, content, edge: .trailing)` in `elg!`.
    pub fn edge(mut self, edge: impl Into<FloatingEdge>) -> Self {
        self.placement.edge = edge.into();
        self
    }
    pub fn boundary(mut self, boundary: impl Into<FloatingBoundary>) -> Self {
        self.boundary = boundary.into();
        self
    }
    pub fn padded(mut self, padded: impl IntoProp<bool>) -> Self {
        self.padded = padded.into_prop();
        self
    }
    pub fn motion(mut self, motion: PopupMotion) -> Self {
        self.motion = Some(motion);
        self
    }
}

/// Adjustments to what `.backdrop_blur()` shows through, after the blur. Each is reactive and
/// animatable.
#[derive(Clone)]
pub struct BackdropOptions {
    /// 0 gray, 1 unchanged, above 1 more vivid.
    pub saturation: Prop<f32>,
    /// Added to each channel: -1 black ... 0 unchanged ... 1 white.
    pub brightness: Prop<f32>,
    /// Grain, 0...1, so smooth gradients don't band.
    pub noise: Prop<f32>,
}

impl Default for BackdropOptions {
    fn default() -> Self {
        Self { saturation: Prop::Value(1.0), brightness: Prop::Value(0.0), noise: Prop::Value(0.0) }
    }
}

#[elegance_macros::elg_builder]
impl BackdropOptions {
    pub fn saturation(mut self, saturation: impl IntoProp<f32>) -> Self {
        self.saturation = saturation.into_prop();
        self
    }
    pub fn brightness(mut self, brightness: impl IntoProp<f32>) -> Self {
        self.brightness = brightness.into_prop();
        self
    }
    pub fn noise(mut self, noise: impl IntoProp<f32>) -> Self {
        self.noise = noise.into_prop();
        self
    }
}

/// A drop shadow: `radius` is the blur radius in points, (x, y) the offset. `spread` grows (or,
/// negative, shrinks) the shape before blurring; it applies to single shapes (rects, rounded
/// rects, capsules, circles), as in Figma, and is ignored for other content. Stack shadows by
/// chaining `.shadow(...)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub color: Color,
    pub radius: f32,
    pub x: f32,
    pub y: f32,
    pub spread: f32,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            color: Color::BLACK.opacity(0.33),
            radius: 8.0,
            x: 0.0,
            y: 0.0,
            spread: 0.0,
        }
    }
}

/// How a view's pixels combine with what's behind them: the W3C compositing modes, as in CSS
/// mix-blend-mode, Figma, and SwiftUI. Blending reads the window behind the view (or, inside an
/// offscreen group, what the group drew behind it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlendMode {
    #[default]
    Normal,
    // Darken
    Multiply,
    Darken,
    ColorBurn,
    PlusDarker,
    // Lighten
    Screen,
    Lighten,
    ColorDodge,
    PlusLighter,
    // Contrast
    Overlay,
    SoftLight,
    HardLight,
    // Inversion
    Difference,
    Exclusion,
    // Component
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendMode {
    pub(crate) fn from_c(mode: i32) -> Self {
        use BlendMode::*;
        const ALL: [BlendMode; 18] = [
            Normal,
            Multiply,
            Darken,
            ColorBurn,
            PlusDarker,
            Screen,
            Lighten,
            ColorDodge,
            PlusLighter,
            Overlay,
            SoftLight,
            HardLight,
            Difference,
            Exclusion,
            Hue,
            Saturation,
            Color,
            Luminosity,
        ];
        ALL.get(mode as usize).copied().unwrap_or_default()
    }
}

/// A color adjustment, applied to the content's pixels in sRGB with straight alpha, as CSS
/// filter functions are. Each amount is reactive and animatable.
#[derive(Clone)]
pub enum ColorFilter {
    /// Adds to each channel: -1 black ... 0 unchanged ... 1 white.
    Brightness(Prop<f32>),
    /// 0 flat gray, 1 unchanged, above 1 more contrast.
    Contrast(Prop<f32>),
    /// 0 gray, 1 unchanged, above 1 more vivid.
    Saturation(Prop<f32>),
    /// 0 unchanged ... 1 fully gray.
    Grayscale(Prop<f32>),
    /// Rotates hues around the color wheel, in degrees.
    HueRotation(Prop<f32>),
    /// 0 unchanged ... 1 fully sepia-toned.
    Sepia(Prop<f32>),
    /// 0 unchanged ... 1 inverted.
    Invert(Prop<f32>),
    /// Multiplies every pixel by the color (alpha too): tints white content, darkens the rest.
    Multiply(Prop<Color>),
    /// Brightness becomes opacity: white stays, black disappears (for masks).
    LuminanceToAlpha,
}

impl ColorFilter {
    /// Calls `f` with the C form: the kind (`elg::ColorFilter::Kind`), the amount, and the color.
    pub(crate) fn with_c<R>(self, f: impl FnOnce(i32, sys::ElgProp, sys::ElgProp) -> R) -> R {
        let (kind, amount, color): (i32, Prop<f32>, Prop<Color>) = match self {
            ColorFilter::Brightness(a) => (0, a, Prop::Value(Color::WHITE)),
            ColorFilter::Contrast(a) => (1, a, Prop::Value(Color::WHITE)),
            ColorFilter::Saturation(a) => (2, a, Prop::Value(Color::WHITE)),
            ColorFilter::Grayscale(a) => (3, a, Prop::Value(Color::WHITE)),
            ColorFilter::HueRotation(a) => (4, a, Prop::Value(Color::WHITE)),
            ColorFilter::Sepia(a) => (5, a, Prop::Value(Color::WHITE)),
            ColorFilter::Invert(a) => (6, a, Prop::Value(Color::WHITE)),
            ColorFilter::Multiply(c) => (7, Prop::Value(1.0), c),
            ColorFilter::LuminanceToAlpha => (8, Prop::Value(1.0), Prop::Value(Color::WHITE)),
        };
        with_prop(amount, |a| with_prop(color, |c| f(kind, a, c)))
    }
}

/// Options for [`ViewExt::blur_with`](super::ViewExt::blur_with).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BlurOptions {
    /// The content fills its bounds: edges stay sharp instead of fading out.
    pub opaque: bool,
}

impl BlurOptions {
    pub fn opaque(mut self, opaque: bool) -> Self {
        self.opaque = opaque;
        self
    }
}

/// Where a rotation or scale effect pivots: a unit point of the view's bounds (the center by
/// default).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnchorOptions {
    pub anchor: UnitPoint,
}

#[elegance_macros::elg_builder]
impl AnchorOptions {
    pub fn anchor(mut self, anchor: impl Into<UnitPoint>) -> Self {
        self.anchor = anchor.into();
        self
    }
}

#[elegance_macros::elg_builder]
impl Shadow {
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }
    pub fn radius(mut self, radius: impl Number) -> Self {
        self.radius = radius.to_f32();
        self
    }
    pub fn x(mut self, x: impl Number) -> Self {
        self.x = x.to_f32();
        self
    }
    pub fn y(mut self, y: impl Number) -> Self {
        self.y = y.to_f32();
        self
    }
    pub fn spread(mut self, spread: impl Number) -> Self {
        self.spread = spread.to_f32();
        self
    }
    pub(crate) fn c(self) -> sys::ElgShadow {
        sys::ElgShadow {
            color: self.color.c(),
            radius: self.radius,
            x: self.x,
            y: self.y,
            spread: self.spread,
        }
    }
}

/// A frame: fixed `width`/`height`, or a range (`min_*`, `ideal_*`, `max_*`) that takes the
/// proposed size, clamped. Unset dimensions follow the content.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameOptions {
    pub width: Len,
    pub height: Len,
    pub min_width: Len,
    pub ideal_width: Len,
    pub max_width: Len,
    pub min_height: Len,
    pub ideal_height: Len,
    pub max_height: Len,
    pub alignment: Alignment,
}

macro_rules! len_setters {
    ($($field:ident),*) => {$(
        #[elegance_macros::elg_builder]
        impl FrameOptions {
            pub fn $field(mut self, length: impl Into<Len>) -> Self {
                self.$field = length.into();
                self
            }
        }
    )*};
}

len_setters!(
    width,
    height,
    min_width,
    ideal_width,
    max_width,
    min_height,
    ideal_height,
    max_height
);

#[elegance_macros::elg_builder]
impl FrameOptions {
    pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
    pub(super) fn c(self) -> sys::ElgFrameOptions {
        sys::ElgFrameOptions {
            width: self.width.c(),
            height: self.height.c(),
            min_width: self.min_width.c(),
            ideal_width: self.ideal_width.c(),
            max_width: self.max_width.c(),
            min_height: self.min_height.c(),
            ideal_height: self.ideal_height.c(),
            max_height: self.max_height.c(),
            alignment: self.alignment.c(),
        }
    }
}

/// Where a background or overlay view sits in the modified view's bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DecorationOptions {
    pub alignment: Alignment,
}

#[elegance_macros::elg_builder]
impl DecorationOptions {
    pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AspectRatioOptions {
    /// width / height; `None`: the content's own (an image's).
    pub ratio: Option<f32>,
    pub mode: ContentMode,
}

impl Default for AspectRatioOptions {
    fn default() -> Self {
        Self {
            ratio: None,
            mode: ContentMode::Fit,
        }
    }
}

#[elegance_macros::elg_builder]
impl AspectRatioOptions {
    pub fn ratio(mut self, ratio: impl Number) -> Self {
        self.ratio = Some(ratio.to_f32());
        self
    }
    pub fn mode(mut self, mode: impl Into<ContentMode>) -> Self {
        self.mode = mode.into();
        self
    }
}

/// What a view does with things dragged in from other apps. Handlers get what was dropped and
/// where (in the view's coordinates); returning false means it wasn't wanted.
#[derive(Default)]
pub struct DropOptions {
    pub files: Option<Box<dyn Fn(Vec<PathBuf>, Point) -> bool>>,
    pub text: Option<Box<dyn Fn(String, Point) -> bool>>,
    /// Set while a drag the view can take is over it (for a highlight).
    pub targeted: Option<Box<dyn Fn(bool)>>,
}

impl DropOptions {
    pub fn files(mut self, f: impl Fn(Vec<PathBuf>, Point) -> bool + 'static) -> Self {
        self.files = Some(Box::new(f));
        self
    }
    pub fn text(mut self, f: impl Fn(String, Point) -> bool + 'static) -> Self {
        self.text = Some(Box::new(f));
        self
    }
    pub fn targeted(mut self, f: impl Fn(bool) + 'static) -> Self {
        self.targeted = Some(Box::new(f));
        self
    }
}
