//! Images, icons, and labels.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use elegance_sys as sys;

use crate::dsl::ElgNew;
use crate::error::{self, ffi, Error};
use crate::ffi as cffi;
use crate::font::Weight;
use crate::geometry::ContentMode;
use crate::view::{IntoView, View};

/// Where an image comes from: an icon name (`"gear"`, `"heart-fill"`), a file (PNG, JPEG, SVG,
/// ...), or encoded data in memory: owned, or `'static` bytes such as an [`asset!`](crate::asset)
/// (Elegance copies it when it decodes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImageSource {
    pub system_name: String,
    pub file: PathBuf,
    pub data: Cow<'static, [u8]>,
    /// Pixels per point of a raster image (2 for @2x art).
    pub scale: f32,
}

#[elegance_macros::elg_builder]
impl ImageSource {
    pub fn system(name: impl Into<String>) -> Self {
        Self { system_name: name.into(), scale: 1.0, ..Default::default() }
    }
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self { file: path.into(), scale: 1.0, ..Default::default() }
    }
    pub fn data(data: impl Into<Cow<'static, [u8]>>) -> Self {
        Self { data: data.into(), scale: 1.0, ..Default::default() }
    }
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
    /// The C form; borrows the strings and data. `file` is converted into a string kept in
    /// `scratch`.
    pub(crate) fn c(&self) -> sys::ElgImageSource {
        sys::ElgImageSource {
            system_name: cffi::s(&self.system_name),
            file: self.file.to_str().map(cffi::s).unwrap_or(sys::ElgStr::EMPTY),
            data: if self.data.is_empty() { sys::ElgBytes::EMPTY } else { sys::ElgBytes::new(&self.data) },
            scale: if self.scale > 0.0 { self.scale } else { 1.0 },
        }
    }
}

/// A string is an icon name.
impl From<&str> for ImageSource {
    fn from(name: &str) -> Self {
        ImageSource::system(name)
    }
}
impl From<String> for ImageSource {
    fn from(name: String) -> Self {
        ImageSource::system(name)
    }
}
impl From<&Path> for ImageSource {
    fn from(file: &Path) -> Self {
        ImageSource::file(file)
    }
}
impl From<PathBuf> for ImageSource {
    fn from(file: PathBuf) -> Self {
        ImageSource::file(file)
    }
}

/// An image or icon at its natural size. `.resizable()` makes a raster image take the space
/// it's offered. Icons take the font size and foreground style.
pub struct Image {
    _private: (),
}

impl<S: Into<ImageSource>> ElgNew<(S,)> for Image {
    type Out = View;
    fn elg_new((source,): (S,)) -> View {
        Image(source)
    }
}

/// `Image("gear")` as an argument: `Button(Image("trash"), remove)`.
#[allow(non_snake_case)]
pub fn Image(source: impl Into<ImageSource>) -> View {
    let source = source.into();
    let c = source.c();
    View::from_raw(ffi!(sys::elg_image(&c)))
}

/// What an AsyncImage's content closure sees.
pub enum AsyncImagePhase {
    Loading,
    /// The image (an Image, so `.resizable()` applies).
    Success(View),
    /// Why it failed.
    Failure(String),
}

/// An image downloaded from a URL (cached), with a placeholder while it loads.
///
/// ```ignore
/// AsyncImage(url) { phase in
///     { match phase { AsyncImagePhase::Success(image) => image.clone().resizable(), _ => ProgressView::new().into_view() } }
/// }
/// ```
pub struct AsyncImage {
    url: String,
    scale: f32,
    content_mode: Option<ContentMode>,
    content: Option<Box<dyn Fn(&AsyncImagePhase) -> View>>,
}

#[elegance_macros::elg_builder]
impl AsyncImage {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into(), scale: 1.0, content_mode: None, content: None }
    }
    /// Pixels per point.
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
    /// Resizable, keeping its proportions this way (without a content closure).
    pub fn content_mode(mut self, mode: impl Into<ContentMode>) -> Self {
        self.content_mode = Some(mode.into());
        self
    }
    /// The view for each phase.
    pub fn content<V: IntoView>(mut self, content: impl Fn(&AsyncImagePhase) -> V + 'static) -> Self {
        self.content = Some(Box::new(move |phase| content(phase).into_view()));
        self
    }
    #[doc(hidden)]
    pub fn elg_content<V: IntoView>(self, content: impl Fn(&AsyncImagePhase) -> V + 'static) -> Self {
        self.content(content)
    }
}

impl<S: Into<String>> ElgNew<(S,)> for AsyncImage {
    type Out = AsyncImage;
    fn elg_new((url,): (S,)) -> AsyncImage {
        AsyncImage::new(url)
    }
}

impl IntoView for AsyncImage {
    fn into_view(self) -> View {
        match self.content {
            None => {
                let mode = self.content_mode.map_or(-1, |m| m as i32);
                View::from_raw(ffi!(sys::elg_async_image(cffi::s(&self.url), self.scale, mode)))
            }
            Some(content) => {
                let content = cffi::closure(move |args, ret| unsafe {
                    let phase = &*(args as *const sys::ElgAsyncImagePhase);
                    let phase = match phase.kind {
                        0 => AsyncImagePhase::Loading,
                        1 => AsyncImagePhase::Success(View::copy_from(phase.image)),
                        _ => AsyncImagePhase::Failure(phase.error.as_str().to_owned()),
                    };
                    content(&phase).write_into(ret);
                });
                View::from_raw(ffi!(sys::elg_async_image_content(cffi::s(&self.url), content, self.scale)))
            }
        }
    }
}

/// An icon and a title side by side.
pub struct Label {
    _private: (),
}

impl<S: Into<String>, I: Into<ImageSource>> ElgNew<(S, I)> for Label {
    type Out = View;
    fn elg_new((title, icon): (S, I)) -> View {
        Label(title, icon)
    }
}

#[allow(non_snake_case)]
pub fn Label(title: impl Into<String>, icon: impl Into<ImageSource>) -> View {
    let (title, icon) = (title.into(), icon.into());
    let c = icon.c();
    View::from_raw(ffi!(sys::elg_label(cffi::s(&title), &c)))
}

/// An icon font: a file, or data in memory (copied when registered).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum IconFont {
    #[default]
    None,
    File(PathBuf),
    Data(Vec<u8>),
}

impl IconFont {
    fn c(&self) -> sys::ElgIconFont {
        match self {
            IconFont::None => sys::ElgIconFont { file: sys::ElgStr::EMPTY, data: sys::ElgBytes::EMPTY },
            IconFont::File(path) => sys::ElgIconFont { file: path.to_str().map(cffi::s).unwrap_or(sys::ElgStr::EMPTY), data: sys::ElgBytes::EMPTY },
            IconFont::Data(data) => sys::ElgIconFont { file: sys::ElgStr::EMPTY, data: sys::ElgBytes::new(data) },
        }
    }
    pub fn is_empty(&self) -> bool {
        matches!(self, IconFont::None)
    }
}

impl From<&str> for IconFont {
    fn from(file: &str) -> Self {
        IconFont::File(file.into())
    }
}
impl From<PathBuf> for IconFont {
    fn from(file: PathBuf) -> Self {
        IconFont::File(file)
    }
}
impl From<Vec<u8>> for IconFont {
    fn from(data: Vec<u8>) -> Self {
        IconFont::Data(data)
    }
}
impl From<&[u8]> for IconFont {
    fn from(data: &[u8]) -> Self {
        IconFont::Data(data.to_vec())
    }
}

/// An icon style with its own font: `Image("brand-fill")` uses the "fill" variant.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IconVariant {
    pub name: String,
    pub font: IconFont,
    /// A names map of its own (empty: the set's).
    pub names: PathBuf,
    pub names_data: String,
}

/// An icon set: a font of glyphs, and a map from icon names to code points (lines of
/// `name codepoint`, or CSS like Phosphor's).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IconSetOptions {
    /// The regular weight.
    pub font: IconFont,
    /// More weights (for `.font_weight()`).
    pub weights: Vec<(Weight, IconFont)>,
    pub variants: Vec<IconVariant>,
    pub names: PathBuf,
    /// Or the map's text.
    pub names_data: String,
    /// Replace a set registered under the same name.
    pub replace: bool,
}

/// Registers an icon set; `Image("name:icon")` uses it, or plain names once it's the default.
pub fn register_icon_set(name: &str, options: IconSetOptions) -> Result<(), Error> {
    let weights: Vec<sys::ElgIconWeight> =
        options.weights.iter().map(|(weight, font)| sys::ElgIconWeight { weight: *weight as i32, font: font.c() }).collect();
    let variant_names: Vec<String> = options.variants.iter().map(|v| v.names.to_string_lossy().into_owned()).collect();
    let variants: Vec<sys::ElgIconVariant> = options
        .variants
        .iter()
        .zip(&variant_names)
        .map(|(v, names)| sys::ElgIconVariant { name: cffi::s(&v.name), font: v.font.c(), names: cffi::s(names), names_data: cffi::s(&v.names_data) })
        .collect();
    let names = options.names.to_string_lossy().into_owned();
    unsafe {
        sys::elg_register_icon_set(
            cffi::s(name),
            options.font.c(),
            weights.as_ptr(),
            weights.len(),
            variants.as_ptr(),
            variants.len(),
            cffi::s(&names),
            cffi::s(&options.names_data),
            options.replace,
        )
    };
    error::result()
}

/// Plain icon names (`Image("gear")`) come from this set.
pub fn set_default_icon_set(name: &str) -> Result<(), Error> {
    unsafe { sys::elg_set_default_icon_set(cffi::s(name)) };
    error::result()
}
