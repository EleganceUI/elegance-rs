//! Fonts and font registration.

use std::path::Path;

use elegance_sys as sys;

use crate::error::{self, Error};
use crate::ffi::{self, s};

/// A font weight: the named weights, and any between them (`Weight::new(550)`), from 1 to 1000.
/// Variable fonts (macOS's system font, a variable file passed to [`register_font`]) draw each
/// weight exactly; other fonts use their closest weight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Weight(u16);

#[allow(non_upper_case_globals)]
impl Weight {
    pub const UltraLight: Weight = Weight(100);
    pub const Thin: Weight = Weight(200);
    pub const Light: Weight = Weight(300);
    pub const Regular: Weight = Weight(400);
    pub const Medium: Weight = Weight(500);
    pub const Semibold: Weight = Weight(600);
    pub const Bold: Weight = Weight(700);
    pub const Heavy: Weight = Weight(800);
    pub const Black: Weight = Weight(900);

    /// A weight from 1 (thinnest) to 1000 (heaviest); values outside are clamped.
    pub const fn new(value: u16) -> Self {
        Weight(if value < 1 { 1 } else if value > 1000 { 1000 } else { value })
    }
    /// 1...1000; 400 is regular, 700 bold.
    pub const fn value(self) -> u16 {
        self.0
    }
    pub(crate) fn from_c(value: i32) -> Self {
        Weight::new(value.clamp(1, 1000) as u16)
    }
    pub(crate) fn c(self) -> i32 {
        i32::from(self.0)
    }
}

impl Default for Weight {
    fn default() -> Self {
        Weight::Regular
    }
}

impl From<u16> for Weight {
    fn from(value: u16) -> Self {
        Weight::new(value)
    }
}

/// A font: a family (empty: the system font), size in points, weight, and italic.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    family: String,
    size: f32,
    weight: Weight,
    italic: bool,
}

impl Default for Font {
    fn default() -> Self {
        Self::body()
    }
}

#[elegance_macros::elg_builder]
impl Font {
    pub fn system(size: f32, weight: impl Into<Weight>) -> Self {
        Self { family: String::new(), size, weight: weight.into(), italic: false }
    }
    /// A registered family (see [`register_font`]).
    pub fn custom(family: impl Into<String>, size: f32, weight: impl Into<Weight>) -> Self {
        Self { family: family.into(), size, weight: weight.into(), italic: false }
    }
    pub fn weight(mut self, weight: impl Into<Weight>) -> Self {
        self.weight = weight.into();
        self
    }
    pub fn bold(self) -> Self {
        self.weight(Weight::Bold)
    }
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }
    pub fn family(&self) -> &str {
        &self.family
    }
    pub fn size(&self) -> f32 {
        self.size
    }
    pub fn font_weight(&self) -> Weight {
        self.weight
    }
    pub fn is_italic(&self) -> bool {
        self.italic
    }

    pub fn large_title() -> Self {
        Self::system(30.0, Weight::Bold)
    }
    pub fn title() -> Self {
        Self::system(24.0, Weight::Semibold)
    }
    pub fn title2() -> Self {
        Self::system(20.0, Weight::Semibold)
    }
    pub fn title3() -> Self {
        Self::system(17.0, Weight::Semibold)
    }
    pub fn headline() -> Self {
        Self::system(15.0, Weight::Semibold)
    }
    pub fn body() -> Self {
        Self::system(15.0, Weight::Regular)
    }
    pub fn callout() -> Self {
        Self::system(14.0, Weight::Regular)
    }
    pub fn subheadline() -> Self {
        Self::system(13.0, Weight::Regular)
    }
    pub fn footnote() -> Self {
        Self::system(12.0, Weight::Regular)
    }
    pub fn caption() -> Self {
        Self::system(12.0, Weight::Regular)
    }
    pub fn caption2() -> Self {
        Self::system(11.0, Weight::Regular)
    }

    pub(crate) fn c(&self) -> sys::ElgFont {
        sys::ElgFont { family: s(&self.family), size: self.size, weight: self.weight.c(), italic: self.italic }
    }

    pub(crate) fn from_parts(family: String, size: f32, weight: i32, italic: bool) -> Self {
        Self { family, size, weight: Weight::from_c(weight), italic }
    }
}

/// Registers a font file under `family` for this weight (others are registered separately; the
/// closest registered weight is used). Thread-safe.
pub fn register_font(family: &str, file: impl AsRef<Path>, weight: impl Into<Weight>) -> Result<(), Error> {
    let file = ffi::path_str(file.as_ref());
    unsafe { sys::elg_register_font_file(s(family), s(&file), weight.into().c()) };
    error::result()
}

/// The same from font data in memory (copied).
pub fn register_font_data(family: &str, data: &[u8], weight: impl Into<Weight>) -> Result<(), Error> {
    unsafe { sys::elg_register_font_data(s(family), sys::ElgBytes::new(data), weight.into().c()) };
    error::result()
}

/// A color emoji font, used for emoji in any text. Register before text is first shaped.
pub fn register_emoji_font(file: impl AsRef<Path>) -> Result<(), Error> {
    let file = ffi::path_str(file.as_ref());
    unsafe { sys::elg_register_emoji_font(s(&file)) };
    error::result()
}
