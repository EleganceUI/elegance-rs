//! Built-in views: stacks, controls, structure (If, ForEach), scrolling, lists, and menus.
//!
//! Each is an element for `elg!` (see [`crate::dsl`]) and a builder for plain Rust:
//! `VStack::new().spacing(8).child(Text::new("Hi"))`.

use std::ffi::c_void;
use std::ops::Range;
use std::ptr::NonNull;
use std::rc::Rc;

use elegance_sys as sys;

use crate::dsl::{exactly_one, single, ElgChildren, ElgNew};
use crate::error::ffi;
use crate::events::KeyEvent;
use crate::ffi as cffi;
use crate::geometry::{Alignment, HorizontalAlignment, Len, Number, VerticalAlignment};
use crate::image::ImageSource;
use crate::prop::{with_prop, IntoProp, Prop};
use crate::reactive::{untrack, Binding, Computed, IntoBinding, State};
use crate::text::{Text, TextSpan};
use crate::view::{raw_views, IntoView, View};

/// A label argument: a title string or any view.
pub enum LabelArg {
    Title(String),
    View(View),
}

/// Titles (`&str`, `String`) or views, for controls' labels.
pub trait IntoLabel {
    fn into_label(self) -> LabelArg;
}

impl IntoLabel for &str {
    fn into_label(self) -> LabelArg {
        LabelArg::Title(self.to_owned())
    }
}
impl IntoLabel for String {
    fn into_label(self) -> LabelArg {
        LabelArg::Title(self)
    }
}
impl IntoLabel for &String {
    fn into_label(self) -> LabelArg {
        LabelArg::Title(self.clone())
    }
}
impl<V: IntoView> IntoLabel for V {
    fn into_label(self) -> LabelArg {
        LabelArg::View(self.into_view())
    }
}

impl LabelArg {
    fn into_view(self) -> View {
        match self {
            LabelArg::Title(title) => Text::new(title).into_view(),
            LabelArg::View(view) => view,
        }
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
                View::from_raw(ffi!(sys::elg_button_title(cffi::s(&title), action)))
            }
            Some(LabelArg::View(label)) => {
                View::from_raw(ffi!(sys::elg_button(label.into_raw(), action)))
            }
            None => View::from_raw(ffi!(sys::elg_button(View::empty().into_raw(), action))),
        }
    }
}

/// A text editor bound to a string: typing, IME, selection, clipboard, undo. `TextField` and
/// `SecureField` (bullets, no copying) share this builder.
pub struct TextFieldBuilder<const SECURE: bool> {
    placeholder: String,
    text: Binding<String>,
    on_submit: Option<Box<dyn FnMut()>>,
    max_lines: i32,
    styler: Option<Box<dyn Fn(&str) -> Vec<TextSpan>>>,
    on_key: Option<Box<dyn Fn(&KeyEvent) -> bool>>,
}

/// `TextField("Name", name, on_submit: save)`
pub type TextField = TextFieldBuilder<false>;
/// A TextField for passwords.
pub type SecureField = TextFieldBuilder<true>;

impl<const SECURE: bool> TextFieldBuilder<SECURE> {
    pub fn new(placeholder: impl Into<String>, text: impl IntoBinding<String>) -> Self {
        Self {
            placeholder: placeholder.into(),
            text: text.into_binding(),
            on_submit: None,
            max_lines: 1,
            styler: None,
            on_key: None,
        }
    }
    /// Enter.
    pub fn on_submit(mut self, action: impl FnMut() + 'static) -> Self {
        self.on_submit = Some(Box::new(action));
        self
    }
    /// More than 1: the field wraps and grows up to this many lines, then scrolls. Shift+Enter
    /// starts a new line.
    pub fn max_lines(mut self, lines: i32) -> Self {
        self.max_lines = lines;
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

impl<const SECURE: bool, P: Into<String>, B: IntoBinding<String>> ElgNew<(P, B)>
    for TextFieldBuilder<SECURE>
{
    type Out = Self;
    fn elg_new((placeholder, text): (P, B)) -> Self {
        Self::new(placeholder, text)
    }
}

impl<const SECURE: bool> IntoView for TextFieldBuilder<SECURE> {
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
        let options = sys::ElgTextFieldOptions {
            on_submit: cffi::optional_action(self.on_submit),
            max_lines: self.max_lines,
            styler,
            on_key,
        };
        View::from_raw(ffi!(sys::elg_text_field(
            cffi::s(&self.placeholder),
            self.text.cpp(),
            &options,
            SECURE
        )))
    }
}

/// A value in [min, max] picked by dragging a knob along a track.
pub struct Slider {
    value: Binding<f32>,
    min: f32,
    max: f32,
    step: f32,
    on_editing_changed: Option<Box<dyn Fn(bool)>>,
}

#[elegance_macros::elg_builder]
impl Slider {
    pub fn new(value: impl IntoBinding<f32>) -> Self {
        Self {
            value: value.into_binding(),
            min: 0.0,
            max: 1.0,
            step: 0.0,
            on_editing_changed: None,
        }
    }
    pub fn min(mut self, min: impl Number) -> Self {
        self.min = min.to_f32();
        self
    }
    pub fn max(mut self, max: impl Number) -> Self {
        self.max = max.to_f32();
        self
    }
    /// 0: continuous.
    pub fn step(mut self, step: impl Number) -> Self {
        self.step = step.to_f32();
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
        View::from_raw(ffi!(sys::elg_slider(
            self.value.cpp(),
            self.min,
            self.max,
            self.step,
            changed
        )))
    }
}

/// The label, then - and + buttons that change `value` by `step` within [min, max].
pub struct Stepper {
    label: LabelArg,
    value: Binding<i32>,
    min: i32,
    max: i32,
    step: i32,
}

impl Stepper {
    pub fn new(label: impl IntoLabel, value: impl IntoBinding<i32>) -> Self {
        Self {
            label: label.into_label(),
            value: value.into_binding(),
            min: i32::MIN,
            max: i32::MAX,
            step: 1,
        }
    }
    pub fn min(mut self, min: i32) -> Self {
        self.min = min;
        self
    }
    pub fn max(mut self, max: i32) -> Self {
        self.max = max;
        self
    }
    pub fn step(mut self, step: i32) -> Self {
        self.step = step;
        self
    }
}

impl<L: IntoLabel, B: IntoBinding<i32>> ElgNew<(L, B)> for Stepper {
    type Out = Stepper;
    fn elg_new((label, value): (L, B)) -> Stepper {
        Stepper::new(label, value)
    }
}

impl IntoView for Stepper {
    fn into_view(self) -> View {
        let value = self.value.cpp();
        match self.label {
            LabelArg::Title(title) => View::from_raw(ffi!(sys::elg_stepper_title(
                cffi::s(&title),
                value,
                self.min,
                self.max,
                self.step
            ))),
            LabelArg::View(label) => View::from_raw(ffi!(sys::elg_stepper(
                label.into_raw(),
                value,
                self.min,
                self.max,
                self.step
            ))),
        }
    }
}

/// A bar filled to `fraction` (0...1), or indeterminate activity without one.
#[derive(Default)]
pub struct ProgressView {
    fraction: Option<Prop<f32>>,
}

#[elegance_macros::elg_builder]
impl ProgressView {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn fraction(mut self, fraction: impl IntoProp<f32>) -> Self {
        self.fraction = Some(fraction.into_prop());
        self
    }
}

impl ElgNew<()> for ProgressView {
    type Out = ProgressView;
    fn elg_new(_: ()) -> ProgressView {
        ProgressView::default()
    }
}

impl<P: IntoProp<f32>> ElgNew<(P,)> for ProgressView {
    type Out = ProgressView;
    fn elg_new((fraction,): (P,)) -> ProgressView {
        ProgressView::default().fraction(fraction)
    }
}

impl IntoView for ProgressView {
    fn into_view(self) -> View {
        match self.fraction {
            Some(fraction) => with_prop(fraction, |p| View::from_raw(ffi!(sys::elg_progress(p)))),
            None => View::from_raw(ffi!(sys::elg_progress_indeterminate())),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PickerStyle {
    /// Every option visible, as segments.
    #[default]
    Segmented,
    /// The chosen option on a button; the others in a menu.
    Menu,
}

/// One of several options; `selection` is the chosen option's value.
///
/// `Picker(size,[(Size::S, "S"), (Size::M, "M"), (Size::L, "L")], style: .menu)`
pub struct Picker<T: 'static> {
    selection: Binding<T>,
    options: Vec<(T, String)>,
    style: PickerStyle,
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
            style: PickerStyle::Segmented,
        }
    }
    pub fn style(mut self, style: impl Into<PickerStyle>) -> Self {
        self.style = style.into();
        self
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
        View::from_raw(ffi!(sys::elg_picker(
            labels.as_ptr(),
            labels.len(),
            selected,
            select,
            self.style as i32
        )))
    }
}

/// An on/off control bound to `is_on`; its look comes from the ToggleStyle (a switch by default).
pub struct Toggle {
    label: LabelArg,
    is_on: Binding<bool>,
}

impl Toggle {
    pub fn new(label: impl IntoLabel, is_on: impl IntoBinding<bool>) -> Self {
        Self {
            label: label.into_label(),
            is_on: is_on.into_binding(),
        }
    }
}

impl<L: IntoLabel, B: IntoBinding<bool>> ElgNew<(L, B)> for Toggle {
    type Out = Toggle;
    fn elg_new((label, is_on): (L, B)) -> Toggle {
        Toggle::new(label, is_on)
    }
}

impl IntoView for Toggle {
    fn into_view(self) -> View {
        let is_on = self.is_on.cpp();
        match self.label {
            LabelArg::Title(title) => {
                View::from_raw(ffi!(sys::elg_toggle_title(cffi::s(&title), is_on)))
            }
            LabelArg::View(label) => View::from_raw(ffi!(sys::elg_toggle(label.into_raw(), is_on))),
        }
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

unsafe extern "C" fn items_equal<T: PartialEq>(a: *mut c_void, b: *mut c_void) -> bool {
    std::panic::catch_unwind(|| *(a as *const Rc<T>) == *(b as *const Rc<T>)).unwrap_or(false)
}

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
    shows_indicators: bool,
    position: Option<ScrollPosition>,
    sticks_to_end: bool,
}

#[elegance_macros::elg_builder]
impl ScrollView {
    pub fn new(content: impl IntoView) -> Self {
        Self {
            content: content.into_view(),
            axes: ScrollAxes::Vertical,
            shows_indicators: true,
            position: None,
            sticks_to_end: false,
        }
    }
    pub fn axes(mut self, axes: impl Into<ScrollAxes>) -> Self {
        self.axes = axes.into();
        self
    }
    /// Scroll bars, shown while scrolling or hovered.
    pub fn shows_indicators(mut self, shows: bool) -> Self {
        self.shows_indicators = shows;
        self
    }
    pub fn position(mut self, position: ScrollPosition) -> Self {
        self.position = Some(position);
        self
    }
    /// Chat logs: starts at the end, and stays there as the content grows while scrolled to it.
    pub fn sticks_to_end(mut self, sticks: bool) -> Self {
        self.sticks_to_end = sticks;
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
        let position = position_ptr(&self.position);
        View::from_raw(ffi!(sys::elg_scroll_view(
            self.content.into_raw(),
            self.axes as i32,
            self.shows_indicators,
            position,
            self.sticks_to_end
        )))
    }
}

/// Like VStack over a ForEach, but only items near the visible part of the enclosing
/// ScrollView are built. For long lists.
#[derive(Default)]
pub struct LazyVStack {
    alignment: HorizontalAlignment,
    spacing: Len,
    for_each: Option<View>,
}

/// Like HStack over a ForEach, built lazily.
#[derive(Default)]
pub struct LazyHStack {
    alignment: VerticalAlignment,
    spacing: Len,
    for_each: Option<View>,
}

macro_rules! lazy_stack {
    ($t:ident, $align:ty, $vertical:expr) => {
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
                View::from_raw(ffi!(sys::elg_lazy_stack(
                    $vertical,
                    self.alignment as i32,
                    self.spacing.c(),
                    for_each.into_raw()
                )))
            }
        }
    };
}

lazy_stack!(LazyVStack, HorizontalAlignment, true);
lazy_stack!(LazyHStack, VerticalAlignment, false);

/// A scrolling list of rows (lazy, with separators and a hover highlight) on a rounded surface.
/// Its block holds one ForEach.
#[derive(Default)]
pub struct List {
    for_each: Option<View>,
    position: Option<ScrollPosition>,
}

impl List {
    pub fn new(for_each: impl IntoView) -> Self {
        Self {
            for_each: Some(for_each.into_view()),
            position: None,
        }
    }
    pub fn position(mut self, position: ScrollPosition) -> Self {
        self.position = Some(position);
        self
    }
}

impl ElgNew<()> for List {
    type Out = List;
    fn elg_new(_: ()) -> List {
        List::default()
    }
}

impl ElgChildren for List {
    type Out = List;
    fn elg_children(mut self, children: Vec<View>) -> List {
        self.for_each = Some(exactly_one(children, "List"));
        self
    }
}

impl IntoView for List {
    fn into_view(self) -> View {
        let for_each = self.for_each.expect("elegance: List needs a ForEach");
        let position = position_ptr(&self.position);
        View::from_raw(ffi!(sys::elg_list(for_each.into_raw(), position)))
    }
}

// --- Menus ---

/// A thin line separating groups: horizontal in a VStack (and menus), full width.
pub struct Divider {
    _private: (),
}

impl ElgNew<()> for Divider {
    type Out = View;
    fn elg_new(_: ()) -> View {
        View::from_raw(ffi!(sys::elg_divider()))
    }
}

#[allow(non_snake_case)]
pub fn Divider() -> View {
    Divider::elg_new(())
}

/// A menu command. Choosing it closes the menu, then runs `action`.
pub struct MenuItem {
    title: String,
    action: Box<dyn FnMut()>,
    shortcut: String,
    destructive: bool,
    checked: Option<Prop<bool>>,
    image: Option<ImageSource>,
}

impl MenuItem {
    pub fn new(title: impl Into<String>, action: impl FnMut() + 'static) -> Self {
        Self {
            title: title.into(),
            action: Box::new(action),
            shortcut: String::new(),
            destructive: false,
            checked: None,
            image: None,
        }
    }
    /// A hint shown on the right, e.g. "Ctrl+R".
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = shortcut.into();
        self
    }
    /// Shown in the danger color.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
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

impl<S: Into<String>, F: FnMut() + 'static> ElgNew<(S, F)> for MenuItem {
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
        let make = |checked: *const sys::ElgProp| {
            View::from_raw(ffi!(sys::elg_menu_item(
                cffi::s(&self.title),
                action,
                cffi::s(&self.shortcut),
                self.destructive,
                checked,
                image_ptr
            )))
        };
        match self.checked {
            Some(checked) => with_prop(checked, |p| make(&p)),
            None => make(std::ptr::null()),
        }
    }
}

/// A menu item that opens another menu beside it.
pub struct Submenu {
    _private: (),
}

impl<S: Into<String>, F: Fn() -> V + 'static, V: IntoView> ElgNew<(S, F)> for Submenu {
    type Out = View;
    fn elg_new((title, menu): (S, F)) -> View {
        let title = title.into();
        let menu = cffi::closure(move |_, ret| unsafe { menu().into_view().write_into(ret) });
        View::from_raw(ffi!(sys::elg_submenu(cffi::s(&title), menu)))
    }
}

/// The menu panel: items (MenuItem, Submenu, Divider, or any view) stacked in a styled panel.
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

/// Which side of its button a MenuButton's menu opens on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MenuEdge {
    #[default]
    Below,
    /// For buttons near the bottom, like a message box's.
    Above,
}

/// A button that opens the menu `menu` builds.
pub struct MenuButton {
    label: LabelArg,
    menu: Rc<dyn Fn() -> View>,
    opens: MenuEdge,
    shows_indicator: bool,
}

#[elegance_macros::elg_builder]
impl MenuButton {
    pub fn new<V: IntoView>(label: impl IntoLabel, menu: impl Fn() -> V + 'static) -> Self {
        Self {
            label: label.into_label(),
            menu: Rc::new(move || menu().into_view()),
            opens: MenuEdge::Below,
            shows_indicator: true,
        }
    }
    pub fn opens(mut self, edge: impl Into<MenuEdge>) -> Self {
        self.opens = edge.into();
        self
    }
    /// The chevron after the label.
    pub fn shows_indicator(mut self, shows: bool) -> Self {
        self.shows_indicator = shows;
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
        let label = self.label.into_view();
        View::from_raw(ffi!(sys::elg_menu_button(
            label.into_raw(),
            menu,
            self.opens as i32,
            self.shows_indicator
        )))
    }
}

// --- Helpers for other modules ---

/// The values of a navigation path (a tracked read), for NavigationStack.
pub(crate) fn untracked<T: Clone + 'static>(binding: Binding<T>) -> T {
    untrack(|| binding.get())
}
