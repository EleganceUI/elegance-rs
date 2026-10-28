//! Values inherited down the view tree: font, colors, theme, styles, and your own keys.
//!
//! ```ignore
//! struct Compact;
//! impl EnvironmentKey for Compact {
//!     type Value = bool;
//!     fn default_value(_: &EnvironmentValues) -> bool { false }
//! }
//!
//! content.environment::<Compact>(true)          // set it for everything inside
//! let compact = Environment::<Compact>::new();  // read it while building a view
//! Text(move || if compact.get() { "S" } else { "Small" })
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

use elegance_sys as sys;

use crate::color::{Color, ColorScheme, Theme, ThemeColor};
use crate::error::ffi;
use crate::events::HoverDelay;
use crate::ffi as cffi;
use crate::font::Font;
use crate::prop::{with_prop, Prop};
use crate::styles::{AnyButtonStyle, AnyTextFieldStyle, AnyToggleStyle};
use crate::window::WindowHandle;

/// The inherited values where a view is (`elg::EnvironmentValues`).
pub struct EnvironmentValues {
    ptr: NonNull<sys::EnvironmentValues>,
    owned: bool,
}

impl Clone for EnvironmentValues {
    fn clone(&self) -> Self {
        Self { ptr: NonNull::new(unsafe { sys::elg_env_clone(self.ptr.as_ptr()) }).unwrap(), owned: true }
    }
}

impl Drop for EnvironmentValues {
    fn drop(&mut self) {
        if self.owned {
            unsafe { sys::elg_env_free(self.ptr.as_ptr()) }
        }
    }
}

#[elegance_macros::elg_builder]
impl EnvironmentValues {
    /// Borrows C++ environment values for the duration of a callback.
    pub(crate) unsafe fn borrowed(ptr: *mut c_void) -> Self {
        Self { ptr: NonNull::new(ptr as *mut sys::EnvironmentValues).expect("no environment"), owned: false }
    }

    pub(crate) fn as_ptr(&self) -> *mut sys::EnvironmentValues {
        self.ptr.as_ptr()
    }

    /// A copy of the environment of the view being built. Panics outside a view's body.
    #[track_caller]
    pub fn current() -> Self {
        Self { ptr: NonNull::new(ffi!(sys::elg_env_current())).unwrap(), owned: true }
    }

    /// A theme color as it is here (theme and color scheme). A tracked read.
    pub fn color(&self, token: impl Into<ThemeColor>) -> Color {
        Color::from_c(ffi!(sys::elg_env_color(self.as_ptr(), token.into() as i32)))
    }

    pub fn get<K: EnvironmentKey>(&self) -> K::Value {
        K::read(self)
    }

    /// Sets a value for everything inside (in an EnvironmentModifier).
    pub fn set<K: EnvironmentKey>(&mut self, value: impl crate::prop::IntoProp<K::Value>) {
        K::write(self, value.into_prop())
    }

    pub fn font(&self) -> Font {
        self.get::<FontKey>()
    }
    pub fn color_scheme(&self) -> ColorScheme {
        self.get::<ColorSchemeKey>()
    }
    pub fn theme(&self) -> Theme {
        self.get::<ThemeKey>()
    }
    pub fn foreground(&self) -> Color {
        self.get::<ForegroundKey>()
    }
    pub fn tint(&self) -> Color {
        self.get::<TintKey>()
    }
    pub fn is_disabled(&self) -> bool {
        self.get::<DisabledKey>()
    }
    pub fn hover_delay(&self) -> HoverDelay {
        self.get::<HoverDelayKey>()
    }
    pub fn window(&self) -> WindowHandle {
        self.get::<WindowKey>()
    }
}

/// A key for an environment value. Implement it for your own keys; `read` and `write` have
/// defaults that store the value for Rust (the built-in keys override them).
pub trait EnvironmentKey: 'static {
    type Value: Clone + 'static;
    fn default_value(environment: &EnvironmentValues) -> Self::Value;

    #[doc(hidden)]
    fn read(environment: &EnvironmentValues) -> Self::Value {
        rust_values(environment)
            .and_then(|values| values.get(&TypeId::of::<Self>()).cloned())
            .and_then(|value| value.downcast::<Prop<Self::Value>>().ok())
            .map(|prop| prop.get(environment))
            .unwrap_or_else(|| Self::default_value(environment))
    }

    #[doc(hidden)]
    fn write(environment: &mut EnvironmentValues, value: Prop<Self::Value>) {
        let mut values = rust_values(environment).map(|values| (*values).clone()).unwrap_or_default();
        values.insert(TypeId::of::<Self>(), Rc::new(value));
        ffi!(sys::elg_env_set_rust_values(environment.as_ptr(), cffi::owner(Rc::new(values))));
    }
}

type RustValues = Rc<HashMap<TypeId, Rc<dyn Any>>>;

fn rust_values(environment: &EnvironmentValues) -> Option<RustValues> {
    let data = ffi!(sys::elg_env_rust_values(environment.as_ptr()));
    (!data.is_null()).then(|| unsafe { (*(data as *const RustValues)).clone() })
}

/// Reads an environment value where a view is built (`elg::Environment<Key>`): it keeps that
/// environment, and reading is reactive.
pub struct Environment<K: EnvironmentKey> {
    environment: EnvironmentValues,
    _key: PhantomData<K>,
}

impl<K: EnvironmentKey> Clone for Environment<K> {
    fn clone(&self) -> Self {
        Self { environment: self.environment.clone(), _key: PhantomData }
    }
}

impl<K: EnvironmentKey> Environment<K> {
    /// Call while building a view (in a body). Panics elsewhere.
    #[track_caller]
    pub fn new() -> Self {
        Self { environment: EnvironmentValues::current(), _key: PhantomData }
    }
    pub fn get(&self) -> K::Value {
        self.environment.get::<K>()
    }
}

impl<K: EnvironmentKey> Default for Environment<K> {
    fn default() -> Self {
        Self::new()
    }
}

fn static_only<T>(prop: Prop<T>, key: &str) -> T {
    match prop {
        Prop::Value(value) => value,
        _ => panic!("elegance: the {key} environment value can't be reactive"),
    }
}

/// The font of text inside.
pub struct FontKey;
impl EnvironmentKey for FontKey {
    type Value = Font;
    fn default_value(_: &EnvironmentValues) -> Font {
        Font::body()
    }
    fn read(environment: &EnvironmentValues) -> Font {
        let (mut size, mut weight, mut italic) = (0.0, 0, false);
        let family = cffi::read_string(|sink, ctx| unsafe {
            sys::elg_env_font(environment.as_ptr(), sink, ctx, &mut size, &mut weight, &mut italic)
        });
        crate::error::check();
        Font::from_parts(family, size, weight, italic)
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<Font>) {
        with_prop(value, |p| ffi!(sys::elg_env_set_font(environment.as_ptr(), p)))
    }
}

/// Light or dark, for theme colors inside.
pub struct ColorSchemeKey;
impl EnvironmentKey for ColorSchemeKey {
    type Value = ColorScheme;
    fn default_value(_: &EnvironmentValues) -> ColorScheme {
        ColorScheme::Light
    }
    fn read(environment: &EnvironmentValues) -> ColorScheme {
        if ffi!(sys::elg_env_color_scheme(environment.as_ptr())) == 1 {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        }
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<ColorScheme>) {
        with_prop(value, |p| ffi!(sys::elg_env_set_color_scheme(environment.as_ptr(), p)))
    }
}

pub struct ThemeKey;
impl EnvironmentKey for ThemeKey {
    type Value = Theme;
    fn default_value(_: &EnvironmentValues) -> Theme {
        Theme::standard()
    }
    fn read(environment: &EnvironmentValues) -> Theme {
        let mut theme = Theme::standard().c();
        ffi!(sys::elg_env_theme(environment.as_ptr(), &mut theme));
        Theme::from_c(theme)
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<Theme>) {
        with_prop(value, |p| ffi!(sys::elg_env_set_theme(environment.as_ptr(), p)))
    }
}

/// The color of text and unfilled shapes.
pub struct ForegroundKey;
impl EnvironmentKey for ForegroundKey {
    type Value = Color;
    fn default_value(environment: &EnvironmentValues) -> Color {
        environment.color(ThemeColor::Primary)
    }
    fn read(environment: &EnvironmentValues) -> Color {
        Color::from_c(ffi!(sys::elg_env_foreground(environment.as_ptr())))
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<Color>) {
        with_prop(value, |p| ffi!(sys::elg_env_set_foreground(environment.as_ptr(), p)))
    }
}

/// The accent color of controls.
pub struct TintKey;
impl EnvironmentKey for TintKey {
    type Value = Color;
    fn default_value(environment: &EnvironmentValues) -> Color {
        environment.color(ThemeColor::Accent)
    }
    fn read(environment: &EnvironmentValues) -> Color {
        Color::from_c(ffi!(sys::elg_env_tint(environment.as_ptr())))
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<Color>) {
        with_prop(value, |p| ffi!(sys::elg_env_set_tint(environment.as_ptr(), p)))
    }
}

pub struct DisabledKey;
impl EnvironmentKey for DisabledKey {
    type Value = bool;
    fn default_value(_: &EnvironmentValues) -> bool {
        false
    }
    fn read(environment: &EnvironmentValues) -> bool {
        ffi!(sys::elg_env_disabled(environment.as_ptr()))
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<bool>) {
        with_prop(value, |p| ffi!(sys::elg_env_set_disabled(environment.as_ptr(), p)))
    }
}

pub struct HoverDelayKey;
impl EnvironmentKey for HoverDelayKey {
    type Value = HoverDelay;
    fn default_value(_: &EnvironmentValues) -> HoverDelay {
        HoverDelay::default()
    }
    fn read(environment: &EnvironmentValues) -> HoverDelay {
        HoverDelay::from_c(ffi!(sys::elg_env_hover_delay(environment.as_ptr())))
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<HoverDelay>) {
        with_prop(value, |p| ffi!(sys::elg_env_set_hover_delay(environment.as_ptr(), p)))
    }
}

/// The window a view is in.
pub struct WindowKey;
impl EnvironmentKey for WindowKey {
    type Value = WindowHandle;
    fn default_value(_: &EnvironmentValues) -> WindowHandle {
        WindowHandle::none()
    }
    fn read(environment: &EnvironmentValues) -> WindowHandle {
        WindowHandle::from_raw(ffi!(sys::elg_env_window(environment.as_ptr())))
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<WindowHandle>) {
        let window = static_only(value, "window");
        ffi!(sys::elg_env_set_window(environment.as_ptr(), window.as_ptr()))
    }
}

/// Inside a selectable row ([`crate::ViewExt::selection`]): whether it's selected; false
/// elsewhere. For rows that show their state themselves (`PlainRowStyle`):
///
/// ```ignore
/// let selected = Environment::<RowSelectedKey>::new();
/// Text(mail.sender).foreground_style(move || if selected.get() { ThemeColor::OnAccent } else { ThemeColor::Secondary })
/// ```
pub struct RowSelectedKey;
impl EnvironmentKey for RowSelectedKey {
    type Value = bool;
    fn default_value(_: &EnvironmentValues) -> bool {
        false
    }
    fn read(environment: &EnvironmentValues) -> bool {
        ffi!(sys::elg_env_row_selected(environment.as_ptr()))
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<bool>) {
        with_prop(value, |p| ffi!(sys::elg_env_set_row_selected(environment.as_ptr(), p)))
    }
}

pub struct ButtonStyleKey;
impl EnvironmentKey for ButtonStyleKey {
    type Value = AnyButtonStyle;
    fn default_value(environment: &EnvironmentValues) -> AnyButtonStyle {
        Self::read(environment)
    }
    fn read(environment: &EnvironmentValues) -> AnyButtonStyle {
        AnyButtonStyle::from_raw(ffi!(sys::elg_env_button_style(environment.as_ptr())))
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<AnyButtonStyle>) {
        let style = static_only(value, "button style");
        ffi!(sys::elg_env_set_button_style(environment.as_ptr(), style.as_ptr()))
    }
}

pub struct ToggleStyleKey;
impl EnvironmentKey for ToggleStyleKey {
    type Value = AnyToggleStyle;
    fn default_value(environment: &EnvironmentValues) -> AnyToggleStyle {
        Self::read(environment)
    }
    fn read(environment: &EnvironmentValues) -> AnyToggleStyle {
        AnyToggleStyle::from_raw(ffi!(sys::elg_env_toggle_style(environment.as_ptr())))
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<AnyToggleStyle>) {
        let style = static_only(value, "toggle style");
        ffi!(sys::elg_env_set_toggle_style(environment.as_ptr(), style.as_ptr()))
    }
}

pub struct TextFieldStyleKey;
impl EnvironmentKey for TextFieldStyleKey {
    type Value = AnyTextFieldStyle;
    fn default_value(environment: &EnvironmentValues) -> AnyTextFieldStyle {
        Self::read(environment)
    }
    fn read(environment: &EnvironmentValues) -> AnyTextFieldStyle {
        AnyTextFieldStyle::from_raw(ffi!(sys::elg_env_text_field_style(environment.as_ptr())))
    }
    fn write(environment: &mut EnvironmentValues, value: Prop<AnyTextFieldStyle>) {
        let style = static_only(value, "text field style");
        ffi!(sys::elg_env_set_text_field_style(environment.as_ptr(), style.as_ptr()))
    }
}
