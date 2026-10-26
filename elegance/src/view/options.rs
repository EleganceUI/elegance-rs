//! The options and values [`ViewExt`](super::ViewExt)'s modifiers take.

use std::path::PathBuf;

use elegance_sys as sys;

use crate::color::Color;
use crate::geometry::{Alignment, ContentMode, Len, Number, Point, UnitPoint};
use crate::prop::{with_prop, Prop};

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

/// Which side of its view a popover opens on (it flips if there isn't room).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PopoverEdge {
    #[default]
    Below,
    Above,
    Trailing,
    Leading,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopoverOptions {
    pub opens: PopoverEdge,
    /// false: the content reaches the panel's edges (images, lists).
    pub padded: bool,
}

impl Default for PopoverOptions {
    fn default() -> Self {
        Self {
            opens: PopoverEdge::Below,
            padded: true,
        }
    }
}

#[elegance_macros::elg_builder]
impl PopoverOptions {
    pub fn opens(mut self, edge: impl Into<PopoverEdge>) -> Self {
        self.opens = edge.into();
        self
    }
    pub fn padded(mut self, padded: bool) -> Self {
        self.padded = padded;
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
pub struct TapOptions {
    /// 2: a double tap.
    pub count: i32,
}

impl Default for TapOptions {
    fn default() -> Self {
        Self { count: 1 }
    }
}

impl TapOptions {
    pub fn count(mut self, count: i32) -> Self {
        self.count = count;
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
