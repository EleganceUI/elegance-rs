//! Built-in views: stacks, controls, structure (If, ForEach), scrolling, lists, and menus.
//!
//! Each is an element for `elg!` (see [`crate::dsl`]) and a builder for plain Rust:
//! `VStack::new().spacing(8).child(Text::new("Hi"))`.

use std::ffi::c_void;
use std::ops::Range;
use std::ptr::NonNull;
use std::rc::Rc;

use elegance_sys as sys;

use crate::color::{Color, ThemeColor};
use crate::dsl::{exactly_one, single, ElgChildren, ElgNew};
use crate::error::ffi;
use crate::events::KeyEvent;
use crate::ffi as cffi;
use crate::geometry::{Alignment, Axis, HorizontalAlignment, Len, Number, VerticalAlignment};
use crate::image::ImageSource;
use crate::prop::{with_prop, IntoProp, OwnedProp, Prop};
use crate::reactive::{untrack, Binding, Computed, IntoBinding, State};
use crate::text::TextSpan;
use crate::view::{c_motion, raw_views, FloatingEdge, FloatingPlacement, IntoView, PopupMotion, View};

/// A label argument: a title (static or reactive) or any view.
pub enum LabelArg {
    Title(Prop<String>),
    View(View),
}

/// Titles (`&str`, `String`) or views, for controls' labels. A reactive title goes through the
/// control's `title` method: `Button::new("", save).title(move || format!("Save {}", name.get()))`.
pub trait IntoLabel {
    fn into_label(self) -> LabelArg;
}

impl IntoLabel for &str {
    fn into_label(self) -> LabelArg {
        LabelArg::Title(Prop::Value(self.to_owned()))
    }
}
impl IntoLabel for String {
    fn into_label(self) -> LabelArg {
        LabelArg::Title(Prop::Value(self))
    }
}
impl IntoLabel for &String {
    fn into_label(self) -> LabelArg {
        LabelArg::Title(Prop::Value(self.clone()))
    }
}
impl<V: IntoView> IntoLabel for V {
    fn into_label(self) -> LabelArg {
        LabelArg::View(self.into_view())
    }
}

// --- Stacks ---

/// Children stacked top to bottom.
#[derive(Default)]
pub struct VStack {
    alignment: HorizontalAlignment,
    spacing: Len,
    children: Vec<View>,
}

/// Children side by side, leading to trailing.
#[derive(Default)]
pub struct HStack {
    alignment: VerticalAlignment,
    spacing: Len,
    children: Vec<View>,
}

/// Children drawn on top of each other, first child at the back.
#[derive(Default)]
pub struct ZStack {
    alignment: Alignment,
    children: Vec<View>,
}

macro_rules! children_methods {
    ($t:ty) => {
        impl $t {
            pub fn child(mut self, child: impl IntoView) -> Self {
                self.children.push(child.into_view());
                self
            }
            pub fn children<V: IntoView>(mut self, children: impl IntoIterator<Item = V>) -> Self {
                self.children
                    .extend(children.into_iter().map(IntoView::into_view));
                self
            }
        }
        impl ElgNew<()> for $t {
            type Out = $t;
            fn elg_new(_: ()) -> $t {
                <$t>::default()
            }
        }
        impl ElgChildren for $t {
            type Out = $t;
            fn elg_children(self, children: Vec<View>) -> $t {
                self.children(children)
            }
        }
    };
}

children_methods!(VStack);
children_methods!(HStack);
children_methods!(ZStack);

#[elegance_macros::elg_builder]
impl VStack {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn alignment(mut self, alignment: impl Into<HorizontalAlignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
    /// Default: [`crate::DEFAULT_STACK_SPACING`].
    pub fn spacing(mut self, spacing: impl Into<Len>) -> Self {
        self.spacing = spacing.into();
        self
    }
}

impl IntoView for VStack {
    fn into_view(self) -> View {
        let mut children = raw_views(self.children);
        View::from_raw(ffi!(sys::elg_vstack(
            self.alignment as i32,
            self.spacing.c(),
            children.as_mut_ptr(),
            children.len()
        )))
    }
}

#[elegance_macros::elg_builder]
impl HStack {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn alignment(mut self, alignment: impl Into<VerticalAlignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
    pub fn spacing(mut self, spacing: impl Into<Len>) -> Self {
        self.spacing = spacing.into();
        self
    }
}

impl IntoView for HStack {
    fn into_view(self) -> View {
        let mut children = raw_views(self.children);
        View::from_raw(ffi!(sys::elg_hstack(
            self.alignment as i32,
            self.spacing.c(),
            children.as_mut_ptr(),
            children.len()
        )))
    }
}

#[elegance_macros::elg_builder]
impl ZStack {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
}

impl IntoView for ZStack {
    fn into_view(self) -> View {
        let mut children = raw_views(self.children);
        View::from_raw(ffi!(sys::elg_zstack(
            self.alignment.c(),
            children.as_mut_ptr(),
            children.len()
        )))
    }
}

/// Flexible space that expands along its stack's axis.
#[derive(Default)]
pub struct Spacer {
    min_length: Len,
}

#[elegance_macros::elg_builder]
impl Spacer {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn min_length(mut self, length: impl Into<Len>) -> Self {
        self.min_length = length.into();
        self
    }
}

#[allow(non_snake_case)]
pub fn Spacer() -> Spacer {
    Spacer::default()
}

impl ElgNew<()> for Spacer {
    type Out = Spacer;
    fn elg_new(_: ()) -> Spacer {
        Spacer::default()
    }
}

impl<L: Into<Len>> ElgNew<(L,)> for Spacer {
    type Out = Spacer;
    fn elg_new((min_length,): (L,)) -> Spacer {
        Spacer::default().min_length(min_length)
    }
}

impl IntoView for Spacer {
    fn into_view(self) -> View {
        View::from_raw(ffi!(sys::elg_spacer(self.min_length.c())))
    }
}

/// Takes no space and draws nothing.
pub struct EmptyView {
    _private: (),
}

impl ElgNew<()> for EmptyView {
    type Out = View;
    fn elg_new(_: ()) -> View {
        View::from_raw(ffi!(sys::elg_empty_view()))
    }
}

#[allow(non_snake_case)]
pub fn EmptyView() -> View {
    EmptyView::elg_new(())
}

// --- Controls ---

/// A push button. Its look comes from the ButtonStyle (`.button_style()`).
///
/// `Button("Save", save)`, or `Button(action: save) { Label("Save", "floppy-disk") }`.
#[derive(Default)]
pub struct Button {
    label: Option<LabelArg>,
    action: Option<Box<dyn FnMut()>>,
}

impl Button {
    pub fn new(label: impl IntoLabel, action: impl FnMut() + 'static) -> Self {
        Self {
            label: Some(label.into_label()),
            action: Some(Box::new(action)),
        }
    }
    pub fn action(mut self, action: impl FnMut() + 'static) -> Self {
        self.action = Some(Box::new(action));
        self
    }
    pub fn label(mut self, label: impl IntoLabel) -> Self {
        self.label = Some(label.into_label());
        self
    }
    /// A title, static or reactive.
    pub fn title(mut self, title: impl IntoProp<String>) -> Self {
        self.label = Some(LabelArg::Title(title.into_prop()));
        self
    }
}

impl<L: IntoLabel, F: FnMut() + 'static> ElgNew<(L, F)> for Button {
    type Out = Button;
    fn elg_new((label, action): (L, F)) -> Button {
        Button::new(label, action)
    }
}

impl ElgNew<()> for Button {
    type Out = Button;
    fn elg_new(_: ()) -> Button {
        Button::default()
    }
}

impl ElgChildren for Button {
    type Out = Button;
    fn elg_children(mut self, children: Vec<View>) -> Button {
        self.label = Some(LabelArg::View(single(children, "Button")));
        self
    }
}

impl IntoView for Button {
    fn into_view(self) -> View {
        let action = cffi::optional_action(self.action);
        match self.label {
            Some(LabelArg::Title(title)) => {
                with_prop(title, |t| View::from_raw(ffi!(sys::elg_button_title(t, action))))
            }
            Some(LabelArg::View(label)) => {
                View::from_raw(ffi!(sys::elg_button(label.into_raw(), action)))
            }
            None => View::from_raw(ffi!(sys::elg_button(View::empty().into_raw(), action))),
        }
    }
}

/// A text editor bound to a string: typing, IME, selection, clipboard, undo. `TextField` and
/// `TextArea` share this builder.
pub struct TextFieldBuilder<const AREA: bool> {
    placeholder: Prop<String>,
    text: Binding<String>,
    on_submit: Option<Box<dyn FnMut()>>,
    secure: Prop<bool>,
    line_limit: Prop<i32>,
    styler: Option<Box<dyn Fn(&str) -> Vec<TextSpan>>>,
    on_key: Option<Box<dyn Fn(&KeyEvent) -> bool>>,
}

/// One line of text, or a few (`line_limit`): `TextField("Name", name, on_submit: save)`.
/// Passwords: `TextField("Password", password, secure: true)`.
pub type TextField = TextFieldBuilder<false>;
/// A multiline editor for notes, messages, and documents: Enter adds a newline, Ctrl+Enter
/// (Cmd+Enter on macOS) submits. It takes the height it's offered and scrolls through longer
/// text (in a ScrollView, it grows with its text instead).
pub type TextArea = TextFieldBuilder<true>;

impl<const AREA: bool> TextFieldBuilder<AREA> {
    pub fn new(placeholder: impl IntoProp<String>, text: impl IntoBinding<String>) -> Self {
        Self {
            placeholder: placeholder.into_prop(),
            text: text.into_binding(),
            on_submit: None,
            secure: Prop::Value(false),
            line_limit: Prop::Value(1),
            styler: None,
            on_key: None,
        }
    }
    /// Enter in a TextField; Ctrl+Enter (Cmd+Enter) in a TextArea.
    pub fn on_submit(mut self, action: impl FnMut() + 'static) -> Self {
        self.on_submit = Some(Box::new(action));
        self
    }
    /// Styles ranges of the text as it's edited; spans are sorted and don't overlap.
    pub fn styler(mut self, styler: impl Fn(&str) -> Vec<TextSpan> + 'static) -> Self {
        self.styler = Some(Box::new(styler));
        self
    }
    /// Sees key presses before the field does; return true to take one.
    pub fn on_key(mut self, on_key: impl Fn(&KeyEvent) -> bool + 'static) -> Self {
        self.on_key = Some(Box::new(on_key));
        self
    }
}

#[elegance_macros::elg_builder]
impl TextFieldBuilder<false> {
    /// Passwords: a bullet per character, no copying, no input method, and one line. Reactive,
    /// to let the user see what they typed: `.secure(move || !reveal.get())`.
    pub fn secure(mut self, secure: impl IntoProp<bool>) -> Self {
        self.secure = secure.into_prop();
        self
    }
    /// More than 1: the field wraps and grows up to this many lines, then scrolls. Shift+Enter
    /// starts a new line. For longer text, use TextArea.
    pub fn line_limit(mut self, lines: impl IntoProp<i32>) -> Self {
        self.line_limit = lines.into_prop();
        self
    }
}

impl<const AREA: bool, P: IntoProp<String>, B: IntoBinding<String>> ElgNew<(P, B)>
    for TextFieldBuilder<AREA>
{
    type Out = Self;
    fn elg_new((placeholder, text): (P, B)) -> Self {
        Self::new(placeholder, text)
    }
}

impl<const AREA: bool> IntoView for TextFieldBuilder<AREA> {
    fn into_view(self) -> View {
        let styler = match self.styler {
            Some(styler) => cffi::closure(move |args, ret| unsafe {
                let text = (*(args as *const sys::ElgStr)).as_str();
                for span in styler(text) {
                    let style = span.style.c();
                    let label = span.label.as_deref().map(cffi::s);
                    let label_ptr = label
                        .as_ref()
                        .map_or(std::ptr::null(), |l| l as *const sys::ElgStr);
                    sys::elg_text_spans_push(
                        ret as *mut sys::TextSpans,
                        span.begin,
                        span.end,
                        &style,
                        label_ptr,
                    );
                }
            }),
            None => sys::ElgClosure::NONE,
        };
        let on_key = match self.on_key {
            Some(on_key) => cffi::closure(move |args, ret| unsafe {
                *(ret as *mut bool) =
                    on_key(&KeyEvent::from_c(&*(args as *const sys::ElgKeyEvent)));
            }),
            None => sys::ElgClosure::NONE,
        };
        let (on_submit, text) = (cffi::optional_action(self.on_submit), self.text.cpp());
        with_prop(self.secure, |secure| {
            with_prop(self.line_limit, |line_limit| {
                let options = sys::ElgTextFieldOptions { on_submit, secure, line_limit, styler, on_key };
                with_prop(self.placeholder, |placeholder| {
                    let make = if AREA { sys::elg_text_area } else { sys::elg_text_field };
                    View::from_raw(ffi!(make(placeholder, text, &options)))
                })
            })
        })
    }
}

/// A value in [min, max] picked by dragging a knob along a track. A new range moves the thumb
/// without writing the value.
pub struct Slider {
    value: Binding<f32>,
    min: Prop<f32>,
    max: Prop<f32>,
    step: Prop<f32>,
    on_editing_changed: Option<Box<dyn Fn(bool)>>,
}

#[elegance_macros::elg_builder]
impl Slider {
    pub fn new(value: impl IntoBinding<f32>) -> Self {
        Self {
            value: value.into_binding(),
            min: Prop::Value(0.0),
            max: Prop::Value(1.0),
            step: Prop::Value(0.0),
            on_editing_changed: None,
        }
    }
    pub fn min(mut self, min: impl IntoProp<f32>) -> Self {
        self.min = min.into_prop();
        self
    }
    pub fn max(mut self, max: impl IntoProp<f32>) -> Self {
        self.max = max.into_prop();
        self
    }
    /// 0: continuous.
    pub fn step(mut self, step: impl IntoProp<f32>) -> Self {
        self.step = step.into_prop();
        self
    }
    /// Called with true when a drag starts and false when it ends.
    pub fn on_editing_changed(mut self, f: impl Fn(bool) + 'static) -> Self {
        self.on_editing_changed = Some(Box::new(f));
        self
    }
}

impl<B: IntoBinding<f32>> ElgNew<(B,)> for Slider {
    type Out = Slider;
    fn elg_new((value,): (B,)) -> Slider {
        Slider::new(value)
    }
}

impl IntoView for Slider {
    fn into_view(self) -> View {
        let changed = self
            .on_editing_changed
            .map(|f| cffi::consumer(move |b: bool| f(b)))
            .unwrap_or(sys::ElgClosure::NONE);
        let value = self.value.cpp();
        with_prop(self.min, |min| {
            with_prop(self.max, |max| {
                with_prop(self.step, |step| View::from_raw(ffi!(sys::elg_slider(value, min, max, step, changed))))
            })
        })
    }
}

/// The value types a NumberInput edits.
pub trait NumberValue: Copy + 'static {
    #[doc(hidden)]
    const INTEGER: bool;
    #[doc(hidden)]
    fn to_f64(self) -> f64;
    #[doc(hidden)]
    fn from_f64(value: f64) -> Self;
}

impl NumberValue for i32 {
    const INTEGER: bool = true;
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn from_f64(value: f64) -> Self {
        value.round() as i32
    }
}
impl NumberValue for f32 {
    const INTEGER: bool = false;
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn from_f64(value: f64) -> Self {
        value as f32
    }
}
impl NumberValue for f64 {
    const INTEGER: bool = false;
    fn to_f64(self) -> f64 {
        self
    }
    fn from_f64(value: f64) -> Self {
        value
    }
}

/// A number you can type, step with the arrows or Up/Down, or scrub by dragging. Text that isn't
/// a number reverts when you leave the field; values stay within [min, max]. Its look comes from
/// the NumberInputStyle (a box with stacked arrows by default). Put a Text beside it for a label.
///
/// `NumberInput(guests, min: 1, max: 20)`, `NumberInput(opacity, min: 0, max: 1, step: 0.05, decimals: 2)`
pub struct NumberInput<T: NumberValue> {
    value: Binding<T>,
    min: Prop<f64>,
    max: Prop<f64>,
    step: Prop<f64>,
    decimals: Prop<i32>,
    format: Option<Box<dyn Fn(f64) -> String>>,
    parse: Option<Box<dyn Fn(&str) -> Option<f64>>>,
    live: Prop<bool>,
    shows_buttons: Prop<bool>,
    scrubs: Prop<bool>,
}

#[elegance_macros::elg_builder]
impl<T: NumberValue> NumberInput<T> {
    pub fn new(value: impl IntoBinding<T>) -> Self {
        Self {
            value: value.into_binding(),
            min: Prop::Value(f64::NEG_INFINITY),
            max: Prop::Value(f64::INFINITY),
            step: Prop::Value(1.0),
            decimals: Prop::Value(-1),
            format: None,
            parse: None,
            live: Prop::Value(false),
            shows_buttons: Prop::Value(true),
            scrubs: Prop::Value(true),
        }
    }
    pub fn min(mut self, min: impl IntoProp<f64>) -> Self {
        self.min = min.into_prop();
        self
    }
    pub fn max(mut self, max: impl IntoProp<f64>) -> Self {
        self.max = max.into_prop();
        self
    }
    /// The arrows, Up/Down (Shift: 10 steps), and each few points scrubbed.
    pub fn step(mut self, step: impl IntoProp<f64>) -> Self {
        self.step = step.into_prop();
        self
    }
    /// Digits after the point; -1: as many as the value needs (up to 6). Integers show none.
    pub fn decimals(mut self, decimals: impl IntoProp<i32>) -> Self {
        self.decimals = decimals.into_prop();
        self
    }
    /// Text for a value, e.g. for units: `.format(|v| format!("{v} px"))`. By default, plain numbers.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Some(Box::new(format));
        self
    }
    /// A value for text (None: not a number). By default, plain numbers with spaces around them.
    pub fn parse(mut self, parse: impl Fn(&str) -> Option<f64> + 'static) -> Self {
        self.parse = Some(Box::new(parse));
        self
    }
    /// Apply each valid number as it's typed. Otherwise typing applies on Enter or when the
    /// field loses focus.
    pub fn live(mut self, live: impl IntoProp<bool>) -> Self {
        self.live = live.into_prop();
        self
    }
    /// The style's increment and decrement buttons.
    pub fn shows_buttons(mut self, shows: impl IntoProp<bool>) -> Self {
        self.shows_buttons = shows.into_prop();
        self
    }
    /// Dragging sideways on the field (while it isn't being edited) changes the value.
    pub fn scrubs(mut self, scrubs: impl IntoProp<bool>) -> Self {
        self.scrubs = scrubs.into_prop();
        self
    }
}

impl<T: NumberValue, B: IntoBinding<T>> ElgNew<(B,)> for NumberInput<T> {
    type Out = NumberInput<T>;
    fn elg_new((value,): (B,)) -> NumberInput<T> {
        NumberInput::new(value)
    }
}

impl<T: NumberValue> IntoView for NumberInput<T> {
    fn into_view(self) -> View {
        let value = self.value;
        let get = cffi::producer(move || value.get().to_f64());
        let set = cffi::consumer(move |v: f64| value.set(T::from_f64(v)));
        let binding = unsafe { sys::elg_binding_f64(get, set) };
        let format = match self.format {
            Some(f) => cffi::closure(move |args, ret| unsafe { cffi::write_string(ret, &f(*(args as *const f64))) }),
            None => sys::ElgClosure::NONE,
        };
        let parse = match self.parse {
            Some(f) => cffi::closure(move |args, ret| unsafe {
                let parsed = f((*(args as *const sys::ElgStr)).as_str());
                *(ret as *mut sys::ElgOptNumber) =
                    sys::ElgOptNumber { has: parsed.is_some(), value: parsed.unwrap_or(0.0) };
            }),
            None => sys::ElgClosure::NONE,
        };
        with_prop(self.min, |min| {
            with_prop(self.max, |max| {
                with_prop(self.step, |step| {
                    with_prop(self.decimals, |decimals| {
                        with_prop(self.live, |live| {
                            with_prop(self.shows_buttons, |shows_buttons| {
                                with_prop(self.scrubs, |scrubs| {
                                    let options = sys::ElgNumberInputOptions {
                                        min, max, step, decimals, format, parse, live, shows_buttons, scrubs,
                                    };
                                    View::from_raw(ffi!(sys::elg_number_input(binding, T::INTEGER, &options)))
                                })
                            })
                        })
                    })
                })
            })
        })
    }
}

/// How far along something is: a fraction (0...1), or, without one (or while it's None),
/// activity of unknown length. Drawn by a ProgressStyle (a bar by default); the fraction animates
/// with the update that changed it. Name it with `.accessibility_label`, or put a Text beside it.
///
/// `Progress(0.4)`, `Progress(move || download.fraction())` (an `Option<f32>`), `Progress()`.
#[derive(Default)]
pub struct Progress {
    fraction: Option<Prop<Option<f32>>>,
}

#[elegance_macros::elg_builder]
impl Progress {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn fraction(mut self, fraction: impl IntoProp<Option<f32>>) -> Self {
        self.fraction = Some(fraction.into_prop());
        self
    }
}

impl ElgNew<()> for Progress {
    type Out = Progress;
    fn elg_new(_: ()) -> Progress {
        Progress::default()
    }
}

impl<P: IntoProp<Option<f32>>> ElgNew<(P,)> for Progress {
    type Out = Progress;
    fn elg_new((fraction,): (P,)) -> Progress {
        Progress::default().fraction(fraction)
    }
}

impl IntoView for Progress {
    fn into_view(self) -> View {
        let fraction = self.fraction.unwrap_or(Prop::Value(None));
        with_prop(fraction, |p| View::from_raw(ffi!(sys::elg_progress(p))))
    }
}

/// One of several options; `selection` is the chosen option's value. Its look comes from the
/// PickerStyle: segments by default, or MenuPickerStyle for a button that opens a menu of them.
///
/// `Picker(size, [(Size::S, "S"), (Size::M, "M"), (Size::L, "L")]).picker_style(MenuPickerStyle)`
pub struct Picker<T: 'static> {
    selection: Binding<T>,
    options: Vec<(T, String)>,
}

#[elegance_macros::elg_builder]
impl<T: PartialEq + Clone + 'static> Picker<T> {
    pub fn new<S: Into<String>>(
        selection: impl IntoBinding<T>,
        options: impl IntoIterator<Item = (T, S)>,
    ) -> Self {
        Self {
            selection: selection.into_binding(),
            options: options
                .into_iter()
                .map(|(value, label)| (value, label.into()))
                .collect(),
        }
    }
}

impl<T, B, I, S> ElgNew<(B, I)> for Picker<T>
where
    T: PartialEq + Clone + 'static,
    B: IntoBinding<T>,
    I: IntoIterator<Item = (T, S)>,
    S: Into<String>,
{
    type Out = Picker<T>;
    fn elg_new((selection, options): (B, I)) -> Picker<T> {
        Picker::new(selection, options)
    }
}

impl<T: PartialEq + Clone + 'static> IntoView for Picker<T> {
    fn into_view(self) -> View {
        let labels: Vec<sys::ElgStr> = self
            .options
            .iter()
            .map(|(_, label)| cffi::s(label))
            .collect();
        let values: Rc<Vec<T>> = Rc::new(
            self.options
                .iter()
                .map(|(value, _)| value.clone())
                .collect(),
        );
        let selection = self.selection;
        let read = values.clone();
        let selected = cffi::producer(move || {
            let current = selection.get();
            read.iter()
                .position(|v| *v == current)
                .unwrap_or(read.len())
        });
        let select = cffi::consumer(move |index: usize| {
            if let Some(value) = values.get(index) {
                selection.set(value.clone());
            }
        });
        View::from_raw(ffi!(sys::elg_picker(labels.as_ptr(), labels.len(), selected, select)))
    }
}

/// An on/off control bound to `is_on`; its look comes from the ToggleStyle (a switch by
/// default). Click, Space, or Enter flips it. `Toggle(is_on)` alone is just the switch, for where
/// a label is beside it already: give it an `.accessibility_label(...)`.
///
/// A plain value doesn't make a binding: `Toggle("Always on", Binding::constant(true))`.
pub struct Toggle {
    label: Option<LabelArg>,
    is_on: Binding<bool>,
}

impl Toggle {
    pub fn new(label: impl IntoLabel, is_on: impl IntoBinding<bool>) -> Self {
        Self { label: Some(label.into_label()), is_on: is_on.into_binding() }
    }
    /// Just the switch.
    pub fn unlabeled(is_on: impl IntoBinding<bool>) -> Self {
        Self { label: None, is_on: is_on.into_binding() }
    }
    /// A title, static or reactive.
    pub fn title(mut self, title: impl IntoProp<String>) -> Self {
        self.label = Some(LabelArg::Title(title.into_prop()));
        self
    }
}

impl<L: IntoLabel, B: IntoBinding<bool>> ElgNew<(L, B)> for Toggle {
    type Out = Toggle;
    fn elg_new((label, is_on): (L, B)) -> Toggle {
        Toggle::new(label, is_on)
    }
}

impl<B: IntoBinding<bool>> ElgNew<(B,)> for Toggle {
    type Out = Toggle;
    fn elg_new((is_on,): (B,)) -> Toggle {
        Toggle::unlabeled(is_on)
    }
}

impl IntoView for Toggle {
    fn into_view(self) -> View {
        let is_on = self.is_on.cpp();
        match self.label {
            Some(LabelArg::Title(title)) => {
                with_prop(title, |t| View::from_raw(ffi!(sys::elg_toggle_title(t, is_on))))
            }
            Some(LabelArg::View(label)) => View::from_raw(ffi!(sys::elg_toggle(label.into_raw(), is_on))),
            None => View::from_raw(ffi!(sys::elg_toggle(std::ptr::null_mut(), is_on))),
        }
    }
}

/// A check box bound to `is_checked`, for choosing something (in a form or a list), where a
/// Toggle switches something on. Its look comes from the CheckboxStyle (a box by default).
/// `Checkbox(is_checked)` alone is just the box.
///
/// `Checkbox("Select all", all, mixed: move || some() && !every())`
pub struct Checkbox {
    label: Option<LabelArg>,
    is_checked: Binding<bool>,
    mixed: Prop<bool>,
}

#[elegance_macros::elg_builder]
impl Checkbox {
    pub fn new(label: impl IntoLabel, is_checked: impl IntoBinding<bool>) -> Self {
        Self { label: Some(label.into_label()), is_checked: is_checked.into_binding(), mixed: Prop::Value(false) }
    }
    /// Just the box.
    pub fn unlabeled(is_checked: impl IntoBinding<bool>) -> Self {
        Self { label: None, is_checked: is_checked.into_binding(), mixed: Prop::Value(false) }
    }
    /// A title, static or reactive.
    pub fn title(mut self, title: impl IntoProp<String>) -> Self {
        self.label = Some(LabelArg::Title(title.into_prop()));
        self
    }
    /// Some of what it stands for is checked: shown with a dash; checking it then checks it
    /// fully. Read by screen readers as mixed.
    pub fn mixed(mut self, mixed: impl IntoProp<bool>) -> Self {
        self.mixed = mixed.into_prop();
        self
    }
}

impl<L: IntoLabel, B: IntoBinding<bool>> ElgNew<(L, B)> for Checkbox {
    type Out = Checkbox;
    fn elg_new((label, is_checked): (L, B)) -> Checkbox {
        Checkbox::new(label, is_checked)
    }
}

impl<B: IntoBinding<bool>> ElgNew<(B,)> for Checkbox {
    type Out = Checkbox;
    fn elg_new((is_checked,): (B,)) -> Checkbox {
        Checkbox::unlabeled(is_checked)
    }
}

impl IntoView for Checkbox {
    fn into_view(self) -> View {
        let is_checked = self.is_checked.cpp();
        let label = self.label;
        with_prop(self.mixed, |mixed| match label {
            Some(LabelArg::Title(title)) => {
                with_prop(title, |t| View::from_raw(ffi!(sys::elg_checkbox_title(t, is_checked, mixed))))
            }
            Some(LabelArg::View(label)) => {
                View::from_raw(ffi!(sys::elg_checkbox(label.into_raw(), is_checked, mixed)))
            }
            None => View::from_raw(ffi!(sys::elg_checkbox(std::ptr::null_mut(), is_checked, mixed))),
        })
    }
}

/// A thin line between groups: horizontal (as wide as offered: in a VStack, a menu, a grid row)
/// or vertical (as tall as offered: in an HStack). Screen readers see a separator.
///
/// `Separator()`, `Separator(.vertical, inset: 4)`
pub struct Separator {
    axis: Axis,
    color: Prop<Color>,
    thickness: f32,
    inset: f32,
}

impl Default for Separator {
    fn default() -> Self {
        Self { axis: Axis::Horizontal, color: Prop::Theme(ThemeColor::Outline), thickness: 1.0, inset: 0.0 }
    }
}

#[elegance_macros::elg_builder]
impl Separator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn axis(mut self, axis: impl Into<Axis>) -> Self {
        self.axis = axis.into();
        self
    }
    pub fn color(mut self, color: impl IntoProp<Color>) -> Self {
        self.color = color.into_prop();
        self
    }
    pub fn thickness(mut self, thickness: impl Number) -> Self {
        self.thickness = thickness.to_f32();
        self
    }
    /// Left clear at both ends.
    pub fn inset(mut self, inset: impl Number) -> Self {
        self.inset = inset.to_f32();
        self
    }
}

#[allow(non_snake_case)]
pub fn Separator() -> Separator {
    Separator::default()
}

impl ElgNew<()> for Separator {
    type Out = Separator;
    fn elg_new(_: ()) -> Separator {
        Separator::default()
    }
}

impl<A: Into<Axis>> ElgNew<(A,)> for Separator {
    type Out = Separator;
    fn elg_new((axis,): (A,)) -> Separator {
        Separator::default().axis(axis)
    }
}

impl IntoView for Separator {
    fn into_view(self) -> View {
        let (axis, thickness, inset) = (self.axis as i32, self.thickness, self.inset);
        with_prop(self.color, |c| View::from_raw(ffi!(sys::elg_separator(axis, c, thickness, inset))))
    }
}

// --- Structure ---

pub(crate) fn make_if(condition: Prop<bool>, then_view: View, else_view: View) -> View {
    with_prop(condition, |p| {
        View::from_raw(ffi!(sys::elg_if(
            p,
            then_view.into_raw(),
            else_view.into_raw()
        )))
    })
}

/// Shows `then_view` while `condition` holds, otherwise `else_view` (nothing by default).
/// Switching branches removes the old branch's views, including state created inside them.
/// In `elg!`, write `if cond { ... } else { ... }`.
pub struct If {
    _private: (),
}

impl<C: IntoProp<bool>, T: IntoView> ElgNew<(C, T)> for If {
    type Out = View;
    fn elg_new((condition, then_view): (C, T)) -> View {
        make_if(condition.into_prop(), then_view.into_view(), View::empty())
    }
}

impl<C: IntoProp<bool>, T: IntoView, E: IntoView> ElgNew<(C, T, E)> for If {
    type Out = View;
    fn elg_new((condition, then_view, else_view): (C, T, E)) -> View {
        make_if(
            condition.into_prop(),
            then_view.into_view(),
            else_view.into_view(),
        )
    }
}

/// Identifies a ForEach item across updates.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ItemKey {
    Int(i64),
    Str(String),
}

/// Values usable as ForEach keys: integers and strings.
pub trait IntoItemKey {
    fn into_item_key(self) -> ItemKey;
}

macro_rules! int_keys {
    ($($t:ty),*) => {$(
        impl IntoItemKey for $t {
            fn into_item_key(self) -> ItemKey {
                ItemKey::Int(self as i64)
            }
        }
        impl IntoItemKey for &$t {
            fn into_item_key(self) -> ItemKey {
                ItemKey::Int(*self as i64)
            }
        }
    )*};
}
int_keys!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl IntoItemKey for String {
    fn into_item_key(self) -> ItemKey {
        ItemKey::Str(self)
    }
}
impl IntoItemKey for &String {
    fn into_item_key(self) -> ItemKey {
        ItemKey::Str(self.clone())
    }
}
impl IntoItemKey for &str {
    fn into_item_key(self) -> ItemKey {
        ItemKey::Str(self.to_owned())
    }
}
impl IntoItemKey for ItemKey {
    fn into_item_key(self) -> ItemKey {
        self
    }
}

/// Where a ForEach's items come from: a Vec, a State/Computed/Binding of one, a closure that
/// returns one (a tracked read), or a range.
pub trait ItemsSource {
    type Item: 'static;
    fn into_items(self) -> Rc<dyn Fn() -> Vec<Self::Item>>;
}

impl<T: Clone + 'static> ItemsSource for Vec<T> {
    type Item = T;
    fn into_items(self) -> Rc<dyn Fn() -> Vec<T>> {
        Rc::new(move || self.clone())
    }
}

impl<T: Clone + 'static> ItemsSource for State<Vec<T>> {
    type Item = T;
    fn into_items(self) -> Rc<dyn Fn() -> Vec<T>> {
        Rc::new(move || self.get())
    }
}

impl<T: Clone + 'static> ItemsSource for Computed<Vec<T>> {
    type Item = T;
    fn into_items(self) -> Rc<dyn Fn() -> Vec<T>> {
        Rc::new(move || self.get())
    }
}

impl<T: Clone + 'static> ItemsSource for Binding<Vec<T>> {
    type Item = T;
    fn into_items(self) -> Rc<dyn Fn() -> Vec<T>> {
        Rc::new(move || self.get())
    }
}

impl<F: Fn() -> Vec<T> + 'static, T: 'static> ItemsSource for F {
    type Item = T;
    fn into_items(self) -> Rc<dyn Fn() -> Vec<T>> {
        Rc::new(self)
    }
}

macro_rules! range_items {
    ($($t:ty),*) => {$(
        impl ItemsSource for Range<$t> {
            type Item = $t;
            fn into_items(self) -> Rc<dyn Fn() -> Vec<$t>> {
                Rc::new(move || self.clone().collect())
            }
        }
    )*};
}
range_items!(i32, i64, u32, usize);

/// A ForEach whose items are their own keys (integers or strings).
pub struct NoKey;

/// A ForEach keyed by a function of its items.
pub struct KeyFn<T>(Rc<dyn Fn(&T) -> ItemKey>);

/// One view per item of a collection, identified by key. When the items change, items whose
/// key and value are unchanged keep their views (and state); changed items are rebuilt, removed
/// ones are removed. Inside a stack, the items are direct children of the stack.
///
/// ```ignore
/// ForEach(todos, id: |t| t.id) { todo in TodoRow(todo: todo.clone()) }
/// ```
pub struct ForEach<T: 'static, K = NoKey> {
    items: Rc<dyn Fn() -> Vec<T>>,
    key: K,
    content: Option<Rc<dyn Fn(&T) -> View>>,
}

impl<T: 'static> ForEach<T, NoKey> {
    pub fn new(items: impl ItemsSource<Item = T>) -> Self {
        Self {
            items: items.into_items(),
            key: NoKey,
            content: None,
        }
    }
}

impl<T: 'static, K> ForEach<T, K> {
    /// The key of an item: an integer or string that stays the same as the item changes.
    pub fn id<R: IntoItemKey>(self, key: impl Fn(&T) -> R + 'static) -> ForEach<T, KeyFn<T>> {
        ForEach {
            items: self.items,
            key: KeyFn(Rc::new(move |item| key(item).into_item_key())),
            content: self.content,
        }
    }
    /// The view for an item.
    pub fn content<V: IntoView>(mut self, content: impl Fn(&T) -> V + 'static) -> Self {
        self.content = Some(Rc::new(move |item| content(item).into_view()));
        self
    }
    /// `{ item in ... }` in `elg!`.
    #[doc(hidden)]
    pub fn elg_content<V: IntoView>(self, content: impl Fn(&T) -> V + 'static) -> Self {
        self.content(content)
    }
}

impl<S: ItemsSource> ElgNew<(S,)> for ForEach<S::Item, NoKey> {
    type Out = ForEach<S::Item, NoKey>;
    fn elg_new((items,): (S,)) -> Self {
        ForEach::new(items)
    }
}

impl<T: IntoItemKey + Clone + PartialEq + 'static> IntoView for ForEach<T, NoKey> {
    fn into_view(self) -> View {
        let content = self
            .content
            .expect("elegance: ForEach needs content: ForEach(items) { item in ... }");
        for_each_view(
            self.items,
            Rc::new(|item: &T| item.clone().into_item_key()),
            content,
        )
    }
}

impl<T: PartialEq + 'static> IntoView for ForEach<T, KeyFn<T>> {
    fn into_view(self) -> View {
        let content = self
            .content
            .expect("elegance: ForEach needs content: ForEach(items, id: ...) { item in ... }");
        for_each_view(self.items, self.key.0, content)
    }
}

unsafe extern "C" fn items_equal<T: PartialEq>(a: *mut c_void, b: *mut c_void) -> bool { unsafe {
    std::panic::catch_unwind(|| *(a as *const Rc<T>) == *(b as *const Rc<T>)).unwrap_or(false)
}}

/// Converts items to ForEach entries and pushes them into a C++ vector (`ret`).
pub(crate) fn push_items<T: PartialEq + 'static>(
    ret: *mut c_void,
    values: Vec<T>,
    key: &dyn Fn(usize, &T) -> ItemKey,
    content: &Rc<dyn Fn(&T) -> View>,
) {
    let keys: Vec<ItemKey> = values.iter().enumerate().map(|(i, v)| key(i, v)).collect();
    let items: Vec<sys::ElgForEachItem> = values
        .into_iter()
        .zip(&keys)
        .map(|(value, key)| {
            let value = Rc::new(value);
            let shown = value.clone();
            let content = content.clone();
            let (key_is_string, int_key, string_key) = match key {
                ItemKey::Int(i) => (false, *i, sys::ElgStr::EMPTY),
                ItemKey::Str(s) => (true, 0, cffi::s(s)),
            };
            sys::ElgForEachItem {
                key_is_string,
                int_key,
                string_key,
                value: cffi::owner(value),
                equals: items_equal::<T>,
                view: cffi::closure(move |_, ret| unsafe { content(&shown).write_into(ret) }),
            }
        })
        .collect();
    unsafe {
        sys::elg_for_each_items_push(ret as *mut sys::ForEachItems, items.as_ptr(), items.len())
    };
}

pub(crate) fn for_each_view<T: PartialEq + 'static>(
    items: Rc<dyn Fn() -> Vec<T>>,
    key: Rc<dyn Fn(&T) -> ItemKey>,
    content: Rc<dyn Fn(&T) -> View>,
) -> View {
    let produce = cffi::closure(move |_, ret| {
        let values = items(); // a tracked read: the ForEach updates when it changes
        push_items(ret, values, &|_, item| key(item), &content);
    });
    View::from_raw(ffi!(sys::elg_for_each(produce)))
}

// --- Scrolling ---

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollAxes {
    #[default]
    Vertical = 1,
    Horizontal = 2,
    Both = 3,
}

/// Scrolls a ScrollView or List from code: make one, pass it in, and call it from event
/// handlers. Clones refer to the same scroll view.
pub struct ScrollPosition {
    ptr: NonNull<sys::ScrollPosition>,
}

impl Clone for ScrollPosition {
    fn clone(&self) -> Self {
        Self {
            ptr: NonNull::new(unsafe { sys::elg_scroll_position_clone(self.ptr.as_ptr()) })
                .unwrap(),
        }
    }
}

impl Drop for ScrollPosition {
    fn drop(&mut self) {
        unsafe { sys::elg_scroll_position_free(self.ptr.as_ptr()) }
    }
}

impl Default for ScrollPosition {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollPosition {
    pub fn new() -> Self {
        Self {
            ptr: NonNull::new(ffi!(sys::elg_scroll_position_new())).unwrap(),
        }
    }
    pub fn scroll_to_top(&self, animated: bool) {
        ffi!(sys::elg_scroll_position_to_top(self.ptr.as_ptr(), animated));
    }
    pub fn scroll_to_end(&self, animated: bool) {
        ffi!(sys::elg_scroll_position_to_end(self.ptr.as_ptr(), animated));
    }
    /// Scrolls the item with this ForEach key (in a lazy stack or List inside) to the top.
    pub fn scroll_to(&self, id: impl IntoItemKey) {
        match id.into_item_key() {
            ItemKey::Int(i) => ffi!(sys::elg_scroll_position_to_int(self.ptr.as_ptr(), i)),
            ItemKey::Str(s) => ffi!(sys::elg_scroll_position_to_string(
                self.ptr.as_ptr(),
                cffi::s(&s)
            )),
        }
    }
    fn as_ptr(&self) -> *const sys::ScrollPosition {
        self.ptr.as_ptr()
    }
}

fn position_ptr(position: &Option<ScrollPosition>) -> *const sys::ScrollPosition {
    position
        .as_ref()
        .map_or(std::ptr::null(), ScrollPosition::as_ptr)
}

/// Shows its content at its ideal size along the scroll axes, clipped to the space it takes.
/// Several views in its block stack vertically.
pub struct ScrollView {
    content: View,
    axes: ScrollAxes,
    shows_indicators: Prop<bool>,
    position: Option<ScrollPosition>,
    sticks_to_end: Prop<bool>,
}

#[elegance_macros::elg_builder]
impl ScrollView {
    pub fn new(content: impl IntoView) -> Self {
        Self {
            content: content.into_view(),
            axes: ScrollAxes::Vertical,
            shows_indicators: Prop::Value(true),
            position: None,
            sticks_to_end: Prop::Value(false),
        }
    }
    pub fn axes(mut self, axes: impl Into<ScrollAxes>) -> Self {
        self.axes = axes.into();
        self
    }
    /// Scroll bars, shown while scrolling or hovered.
    pub fn shows_indicators(mut self, shows: impl IntoProp<bool>) -> Self {
        self.shows_indicators = shows.into_prop();
        self
    }
    pub fn position(mut self, position: ScrollPosition) -> Self {
        self.position = Some(position);
        self
    }
    /// Chat logs: starts at the end, and stays there as the content grows while scrolled to it.
    pub fn sticks_to_end(mut self, sticks: impl IntoProp<bool>) -> Self {
        self.sticks_to_end = sticks.into_prop();
        self
    }
}

impl ElgNew<()> for ScrollView {
    type Out = ScrollView;
    fn elg_new(_: ()) -> ScrollView {
        ScrollView::new(View::empty())
    }
}

impl<V: IntoView> ElgNew<(V,)> for ScrollView {
    type Out = ScrollView;
    fn elg_new((content,): (V,)) -> ScrollView {
        ScrollView::new(content)
    }
}

impl ElgChildren for ScrollView {
    type Out = ScrollView;
    fn elg_children(mut self, children: Vec<View>) -> ScrollView {
        self.content = single(children, "ScrollView");
        self
    }
}

impl IntoView for ScrollView {
    fn into_view(self) -> View {
        let (position, content, axes) = (position_ptr(&self.position), self.content, self.axes as i32);
        with_prop(self.shows_indicators, |shows| {
            with_prop(self.sticks_to_end, |sticks| {
                View::from_raw(ffi!(sys::elg_scroll_view(content.into_raw(), axes, shows, position, sticks)))
            })
        })
    }
}

/// Lines between a lazy stack's items, in the middle of the spacing:
/// `LazyVStack(spacing: 0, separators: StackSeparators::default().inset(16))`.
#[derive(Clone)]
pub struct StackSeparators {
    pub color: Prop<Color>,
    /// From the leading edge (the top, in a LazyHStack).
    pub inset: f32,
    pub thickness: f32,
}

impl Default for StackSeparators {
    fn default() -> Self {
        Self { color: Prop::Theme(ThemeColor::Outline), inset: 0.0, thickness: 1.0 }
    }
}

#[elegance_macros::elg_builder]
impl StackSeparators {
    pub fn color(mut self, color: impl IntoProp<Color>) -> Self {
        self.color = color.into_prop();
        self
    }
    pub fn inset(mut self, inset: impl Number) -> Self {
        self.inset = inset.to_f32();
        self
    }
    pub fn thickness(mut self, thickness: impl Number) -> Self {
        self.thickness = thickness.to_f32();
        self
    }
}

/// Like VStack over a ForEach, but only items near the visible part of the enclosing
/// ScrollView are built. For long lists.
#[derive(Default)]
pub struct LazyVStack {
    alignment: HorizontalAlignment,
    spacing: Len,
    separators: Option<StackSeparators>,
    for_each: Option<View>,
}

/// Like HStack over a ForEach, built lazily.
#[derive(Default)]
pub struct LazyHStack {
    alignment: VerticalAlignment,
    spacing: Len,
    separators: Option<StackSeparators>,
    for_each: Option<View>,
}

macro_rules! lazy_stack {
    ($t:ident, $align:ty, $vertical:expr_2021) => {
        #[elegance_macros::elg_builder]
        impl $t {
            pub fn new(for_each: impl IntoView) -> Self {
                Self {
                    for_each: Some(for_each.into_view()),
                    ..Default::default()
                }
            }
            pub fn alignment(mut self, alignment: impl Into<$align>) -> Self {
                self.alignment = alignment.into();
                self
            }
            pub fn spacing(mut self, spacing: impl Into<Len>) -> Self {
                self.spacing = spacing.into();
                self
            }
            /// Lines between the items.
            pub fn separators(mut self, separators: StackSeparators) -> Self {
                self.separators = Some(separators);
                self
            }
        }
        impl ElgNew<()> for $t {
            type Out = $t;
            fn elg_new(_: ()) -> $t {
                <$t>::default()
            }
        }
        impl ElgChildren for $t {
            type Out = $t;
            fn elg_children(mut self, children: Vec<View>) -> $t {
                self.for_each = Some(exactly_one(children, stringify!($t)));
                self
            }
        }
        impl IntoView for $t {
            fn into_view(self) -> View {
                let for_each =
                    self.for_each
                        .expect(concat!("elegance: ", stringify!($t), " needs a ForEach"));
                // The rest of the color's owner (its static value) stays alive for the call.
                let color;
                let c = match self.separators {
                    Some(s) => {
                        color = OwnedProp::color(s.color);
                        Some(sys::ElgStackSeparators { color: color.c, inset: s.inset, thickness: s.thickness })
                    }
                    None => None,
                };
                let c_ptr = c.as_ref().map_or(std::ptr::null(), |c| c as *const sys::ElgStackSeparators);
                View::from_raw(ffi!(sys::elg_lazy_stack(
                    $vertical,
                    self.alignment as i32,
                    self.spacing.c(),
                    c_ptr,
                    for_each.into_raw()
                )))
            }
        }
    };
}

lazy_stack!(LazyVStack, HorizontalAlignment, true);
lazy_stack!(LazyHStack, VerticalAlignment, false);

// --- Selection ---

/// What `.selection()` binds: one key (`Option<K>`) or any number (`BTreeSet<K>`), as a State
/// or Binding. `K` is the ForEach's key type (integers or strings).
pub trait Selection: 'static {
    type Key: SelectionKey;
    #[doc(hidden)]
    const MULTIPLE: bool;
    #[doc(hidden)]
    fn keys(&self) -> Vec<Self::Key>;
    #[doc(hidden)]
    fn set_keys(&self, keys: Vec<Self::Key>);
}

/// Keys of selectable rows: the ForEach key types.
pub trait SelectionKey: Clone + Ord + 'static {
    #[doc(hidden)]
    const STRING: bool;
    #[doc(hidden)]
    fn to_item_key(&self) -> ItemKey;
    #[doc(hidden)]
    fn from_item_key(key: &ItemKey) -> Self;
}

macro_rules! int_selection_keys {
    ($($t:ty),*) => {$(
        impl SelectionKey for $t {
            const STRING: bool = false;
            fn to_item_key(&self) -> ItemKey {
                ItemKey::Int(*self as i64)
            }
            fn from_item_key(key: &ItemKey) -> Self {
                match key {
                    ItemKey::Int(i) => *i as $t,
                    ItemKey::Str(_) => panic!("elegance: a string key for an integer selection"),
                }
            }
        }
    )*};
}
int_selection_keys!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl SelectionKey for String {
    const STRING: bool = true;
    fn to_item_key(&self) -> ItemKey {
        ItemKey::Str(self.clone())
    }
    fn from_item_key(key: &ItemKey) -> Self {
        match key {
            ItemKey::Str(s) => s.clone(),
            ItemKey::Int(i) => i.to_string(),
        }
    }
}

macro_rules! selections {
    ($($wrapper:ident),*) => {$(
        impl<K: SelectionKey> Selection for $wrapper<Option<K>> {
            type Key = K;
            const MULTIPLE: bool = false;
            fn keys(&self) -> Vec<K> {
                self.get().into_iter().collect()
            }
            fn set_keys(&self, keys: Vec<K>) {
                self.set(keys.into_iter().next());
            }
        }
        impl<K: SelectionKey> Selection for $wrapper<std::collections::BTreeSet<K>> {
            type Key = K;
            const MULTIPLE: bool = true;
            fn keys(&self) -> Vec<K> {
                self.get().into_iter().collect()
            }
            fn set_keys(&self, keys: Vec<K>) {
                self.set(keys.into_iter().collect());
            }
        }
    )*};
}
selections!(State, Binding);

pub(crate) fn selection<S: Selection>(
    view: View,
    selection: S,
    on_activate: Option<impl Fn(&S::Key) + 'static>,
) -> View {
    let selection = Rc::new(selection);
    let read = selection.clone();
    let get = cffi::closure(move |_, ret| {
        for key in read.keys() {
            // A tracked read: the rows follow the selection.
            let item = key.to_item_key();
            let (is_string, int_key, string_key) = match &item {
                ItemKey::Int(i) => (false, *i, sys::ElgStr::EMPTY),
                ItemKey::Str(s) => (true, 0, cffi::s(s)),
            };
            unsafe { sys::elg_item_keys_push(ret as *mut sys::ItemKeys, sys::ElgItemKey { is_string, int_key, string_key }) };
        }
    });
    let set = cffi::closure(move |args, _| {
        let keys = unsafe { &*(args as *const sys::ElgItemKeys) };
        let keys = if keys.keys.is_null() { &[][..] } else { unsafe { std::slice::from_raw_parts(keys.keys, keys.count) } };
        selection.set_keys(keys.iter().map(|k| S::Key::from_item_key(&item_key(k))).collect());
    });
    let activate = match on_activate {
        Some(f) => cffi::closure(move |args, _| f(&S::Key::from_item_key(&item_key(unsafe { &*(args as *const sys::ElgItemKey) })))),
        None => sys::ElgClosure::NONE,
    };
    view.modify(|v| unsafe { sys::elg_mod_selection(v, S::MULTIPLE, S::Key::STRING, get, set, activate) })
}

fn item_key(key: &sys::ElgItemKey) -> ItemKey {
    if key.is_string {
        ItemKey::Str(unsafe { key.string_key.as_str() }.to_owned())
    } else {
        ItemKey::Int(key.int_key)
    }
}

// --- Menus ---

/// A menu command. Choosing it closes the menu, then runs `action`.
pub struct MenuItem {
    title: Prop<String>,
    action: Box<dyn FnMut()>,
    shortcut: Prop<String>,
    destructive: Prop<bool>,
    checked: Option<Prop<bool>>,
    image: Option<ImageSource>,
}

impl MenuItem {
    pub fn new(title: impl IntoProp<String>, action: impl FnMut() + 'static) -> Self {
        Self {
            title: title.into_prop(),
            action: Box::new(action),
            shortcut: Prop::Value(String::new()),
            destructive: Prop::Value(false),
            checked: None,
            image: None,
        }
    }
    /// A hint shown on the right, e.g. "Ctrl+R".
    pub fn shortcut(mut self, shortcut: impl IntoProp<String>) -> Self {
        self.shortcut = shortcut.into_prop();
        self
    }
    /// Shown in the danger color.
    pub fn destructive(mut self, destructive: impl IntoProp<bool>) -> Self {
        self.destructive = destructive.into_prop();
        self
    }
    /// A check mark in front, e.g. for options.
    pub fn checked(mut self, checked: impl IntoProp<bool>) -> Self {
        self.checked = Some(checked.into_prop());
        self
    }
    /// An icon in front.
    pub fn image(mut self, image: impl Into<ImageSource>) -> Self {
        self.image = Some(image.into());
        self
    }
}

impl<S: IntoProp<String>, F: FnMut() + 'static> ElgNew<(S, F)> for MenuItem {
    type Out = MenuItem;
    fn elg_new((title, action): (S, F)) -> MenuItem {
        MenuItem::new(title, action)
    }
}

impl IntoView for MenuItem {
    fn into_view(self) -> View {
        let action = cffi::action(self.action);
        let image = self.image.as_ref().map(ImageSource::c);
        let image_ptr = image
            .as_ref()
            .map_or(std::ptr::null(), |i| i as *const sys::ElgImageSource);
        let checked = self.checked;
        with_prop(self.title, |title| {
            with_prop(self.shortcut, |shortcut| {
                with_prop(self.destructive, |destructive| {
                    let make = |checked: *const sys::ElgProp| {
                        View::from_raw(ffi!(sys::elg_menu_item(
                            title, action, shortcut, destructive, checked, image_ptr
                        )))
                    };
                    match checked {
                        Some(checked) => with_prop(checked, |p| make(&p)),
                        None => make(std::ptr::null()),
                    }
                })
            })
        })
    }
}

/// A menu item that opens another menu beside it (on hover, click, or Right).
pub struct Submenu {
    _private: (),
}

impl<S: IntoProp<String>, F: Fn() -> V + 'static, V: IntoView> ElgNew<(S, F)> for Submenu {
    type Out = View;
    fn elg_new((title, menu): (S, F)) -> View {
        let menu = cffi::closure(move |_, ret| unsafe { menu().into_view().write_into(ret) });
        with_prop(title.into_prop(), |t| View::from_raw(ffi!(sys::elg_submenu(t, menu))))
    }
}

/// The menu panel: items (MenuItem, Submenu, Separator, or any view) stacked in a styled panel.
#[derive(Default)]
pub struct Menu {
    items: Vec<View>,
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn item(mut self, item: impl IntoView) -> Self {
        self.items.push(item.into_view());
        self
    }
}

impl ElgNew<()> for Menu {
    type Out = Menu;
    fn elg_new(_: ()) -> Menu {
        Menu::default()
    }
}

impl ElgChildren for Menu {
    type Out = Menu;
    fn elg_children(mut self, children: Vec<View>) -> Menu {
        self.items.extend(children);
        self
    }
}

impl IntoView for Menu {
    fn into_view(self) -> View {
        let mut items = raw_views(self.items);
        View::from_raw(ffi!(sys::elg_menu(items.as_mut_ptr(), items.len())))
    }
}

/// A button that opens the menu `menu` builds, in a popup window.
pub struct MenuButton {
    label: LabelArg,
    menu: Rc<dyn Fn() -> View>,
    placement: FloatingPlacement,
    shows_indicator: Prop<bool>,
    motion: Option<PopupMotion>,
}

#[elegance_macros::elg_builder]
impl MenuButton {
    pub fn new<V: IntoView>(label: impl IntoLabel, menu: impl Fn() -> V + 'static) -> Self {
        Self {
            label: label.into_label(),
            menu: Rc::new(move || menu().into_view()),
            placement: FloatingPlacement { spacing: 4.0, ..Default::default() },
            shows_indicator: Prop::Value(true),
            motion: None,
        }
    }
    /// A title, static or reactive.
    pub fn title(mut self, title: impl IntoProp<String>) -> Self {
        self.label = LabelArg::Title(title.into_prop());
        self
    }
    /// Where the menu opens: below the button by default, flipping when there isn't room.
    pub fn placement(mut self, placement: impl FnOnce(FloatingPlacement) -> FloatingPlacement) -> Self {
        self.placement = placement(self.placement);
        self
    }
    /// The side it opens on: `.top` for buttons near the bottom, like a message box's.
    pub fn edge(mut self, edge: impl Into<FloatingEdge>) -> Self {
        self.placement.edge = edge.into();
        self
    }
    /// The chevron after the label.
    pub fn shows_indicator(mut self, shows: impl IntoProp<bool>) -> Self {
        self.shows_indicator = shows.into_prop();
        self
    }
    /// How the menu opens and closes; by default the MenuStyle's.
    pub fn motion(mut self, motion: PopupMotion) -> Self {
        self.motion = Some(motion);
        self
    }
}

impl<L: IntoLabel, F: Fn() -> V + 'static, V: IntoView> ElgNew<(L, F)> for MenuButton {
    type Out = MenuButton;
    fn elg_new((label, menu): (L, F)) -> MenuButton {
        MenuButton::new(label, menu)
    }
}

impl IntoView for MenuButton {
    fn into_view(self) -> View {
        let build = self.menu;
        let menu = cffi::closure(move |_, ret| unsafe { build().write_into(ret) });
        let (placement, motion) = (self.placement.c(), c_motion(self.motion.as_ref()));
        let label = self.label;
        with_prop(self.shows_indicator, |shows| match label {
            LabelArg::Title(title) => with_prop(title, |t| {
                View::from_raw(ffi!(sys::elg_menu_button(std::ptr::null_mut(), &t, menu, &placement, shows, motion)))
            }),
            LabelArg::View(label) => View::from_raw(ffi!(sys::elg_menu_button(
                label.into_raw(),
                std::ptr::null(),
                menu,
                &placement,
                shows,
                motion
            ))),
        })
    }
}

// --- Helpers for other modules ---

/// The values of a navigation path (a tracked read), for NavigationStack.
pub(crate) fn untracked<T: Clone + 'static>(binding: Binding<T>) -> T {
    untrack(|| binding.get())
}
