//! Text and styled text.

use std::ptr::NonNull;

use elegance_sys as sys;

use crate::color::Color;
use crate::dsl::ElgNew;
use crate::error::ffi;
use crate::ffi as cffi;
use crate::font::{Font, Weight};
use crate::prop::{with_prop, IntoProp};
use crate::view::{IntoView, View};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAlignment {
    #[default]
    Leading,
    Center,
    Trailing,
}

/// A run of text in the environment's font and foreground. `Text("Count: {count}")` in `elg!`
/// re-renders when `count` changes.
pub struct Text {
    ptr: NonNull<sys::Text>,
}

impl Clone for Text {
    fn clone(&self) -> Self {
        Self::from_raw(unsafe { sys::elg_text_clone(self.as_ptr()) })
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        unsafe { sys::elg_text_free(self.ptr.as_ptr()) }
    }
}

#[elegance_macros::elg_builder]
impl Text {
    pub fn new(content: impl IntoProp<String>) -> Self {
        with_prop(content.into_prop(), |p| Self::from_raw(ffi!(sys::elg_text(p))))
    }

    fn from_raw(ptr: *mut sys::Text) -> Self {
        Self { ptr: NonNull::new(ptr).expect("elegance: no text") }
    }
    pub(crate) fn as_ptr(&self) -> *const sys::Text {
        self.ptr.as_ptr()
    }

    pub fn font(self, font: impl Into<Font>) -> Text {
        let f = font.into();
        let c = f.c();
        Self::from_raw(ffi!(sys::elg_text_font(self.as_ptr(), &c)))
    }
    pub fn font_weight(self, weight: impl Into<Weight>) -> Text {
        Self::from_raw(ffi!(sys::elg_text_font_weight(self.as_ptr(), weight.into() as i32)))
    }
    pub fn bold(self) -> Text {
        Self::from_raw(ffi!(sys::elg_text_bold(self.as_ptr())))
    }
    pub fn foreground_style(self, color: impl IntoProp<Color>) -> Text {
        with_prop(color.into_prop(), |p| Self::from_raw(ffi!(sys::elg_text_foreground_style(self.as_ptr(), p))))
    }
    /// At most this many lines, ending with an ellipsis.
    pub fn line_limit(self, lines: i32) -> Text {
        Self::from_raw(ffi!(sys::elg_text_line_limit(self.as_ptr(), lines)))
    }
    pub fn multiline_text_alignment(self, alignment: impl Into<TextAlignment>) -> Text {
        Self::from_raw(ffi!(sys::elg_text_multiline_alignment(self.as_ptr(), alignment.into() as i32)))
    }
}

impl<S: IntoProp<String>> ElgNew<(S,)> for Text {
    type Out = Text;
    fn elg_new((content,): (S,)) -> Text {
        Text::new(content)
    }
}

/// `Text("...")` as an argument: `Button(Text("Save").bold(), save)`.
#[allow(non_snake_case)]
pub fn Text(content: impl IntoProp<String>) -> Text {
    Text::new(content)
}

/// How a run of styled text looks; unset parts come from the environment.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunStyle {
    pub weight: Option<Weight>,
    pub italic: bool,
    /// A registered family (e.g. monospace for code); empty: the font's.
    pub family: String,
    pub color: Option<Color>,
    /// A rounded highlight behind it (mentions, inline code).
    pub background: Option<Color>,
    pub underline: bool,
    pub strikethrough: bool,
    /// Never broken across lines (implied by a background).
    pub keep_together: bool,
}

#[elegance_macros::elg_builder]
impl RunStyle {
    pub fn weight(mut self, weight: impl Into<Weight>) -> Self {
        self.weight = Some(weight.into());
        self
    }
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }
    pub fn family(mut self, family: impl Into<String>) -> Self {
        self.family = family.into();
        self
    }
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }
    pub fn background(mut self, color: impl Into<Color>) -> Self {
        self.background = Some(color.into());
        self
    }
    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }
    pub fn strikethrough(mut self, strikethrough: bool) -> Self {
        self.strikethrough = strikethrough;
        self
    }
    pub fn keep_together(mut self, keep_together: bool) -> Self {
        self.keep_together = keep_together;
        self
    }
    /// The C form; borrows `family`.
    pub(crate) fn c(&self) -> sys::ElgRunStyle {
        sys::ElgRunStyle {
            has_weight: self.weight.is_some(),
            weight: self.weight.unwrap_or_default() as i32,
            italic: self.italic,
            family: cffi::s(&self.family),
            has_color: self.color.is_some(),
            color: self.color.unwrap_or_default().c(),
            has_background: self.background.is_some(),
            background: self.background.unwrap_or_default().c(),
            underline: self.underline,
            strikethrough: self.strikethrough,
            keep_together: self.keep_together,
        }
    }
}

/// One run of a [`StyledText`].
pub struct TextRun {
    pub text: String,
    pub style: RunStyle,
    /// Clicking the run (links, mentions); none: not clickable.
    pub on_tap: Option<Box<dyn FnMut()>>,
    /// A view inline with the text (an icon, a chip), sized relative to the font.
    pub attachment: Option<View>,
    pub attachment_size: f32,
}

#[elegance_macros::elg_builder]
impl TextRun {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), style: RunStyle::default(), on_tap: None, attachment: None, attachment_size: 1.25 }
    }
    pub fn style(mut self, style: RunStyle) -> Self {
        self.style = style;
        self
    }
    pub fn on_tap(mut self, action: impl FnMut() + 'static) -> Self {
        self.on_tap = Some(Box::new(action));
        self
    }
    pub fn attachment(mut self, view: impl IntoView, size: f32) -> Self {
        self.attachment = Some(view.into_view());
        self.attachment_size = size;
        self
    }
}

impl<S: Into<String>> From<S> for TextRun {
    fn from(text: S) -> Self {
        TextRun::new(text)
    }
}

/// Runs of differently styled text laid out as one paragraph.
pub struct StyledText(pub Vec<TextRun>);

impl IntoView for StyledText {
    fn into_view(self) -> View {
        // The C runs borrow the texts and styles, which live until the call returns.
        let mut texts = Vec::new();
        let mut styles = Vec::new();
        let mut runs = Vec::new();
        for run in self.0 {
            texts.push(run.text);
            styles.push(run.style);
            runs.push(sys::ElgTextRun {
                text: sys::ElgStr::EMPTY,
                style: RunStyle::default().c(),
                on_tap: cffi::optional_action(run.on_tap),
                take_attachment: run.attachment.map(View::into_raw).unwrap_or(std::ptr::null_mut()),
                attachment_size: run.attachment_size,
            });
        }
        for ((c, text), style) in runs.iter_mut().zip(&texts).zip(&styles) {
            c.text = cffi::s(text);
            c.style = style.c();
        }
        View::from_raw(ffi!(sys::elg_styled_text(runs.as_ptr(), runs.len())))
    }
}

/// A styled range of a TextField's text (from its `styler`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSpan {
    /// Byte offsets into the text.
    pub begin: usize,
    pub end: usize,
    pub style: RunStyle,
    /// Shown instead of the text (a mention chip's name).
    pub label: Option<String>,
}
