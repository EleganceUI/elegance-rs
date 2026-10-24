//! Styles for buttons, toggles, and text fields: built-in ones, and your own.
//!
//! ```ignore
//! struct Pill;
//! impl ButtonStyle for Pill {
//!     fn make_body(&self, button: ButtonConfiguration) -> View {
//!         let pressed = button.clone();
//!         elg! {
//!             {button.label()}
//!                 .padding_with(|o| o.horizontal(14).vertical(6))
//!                 .background_in(move || if pressed.is_pressed() { ThemeColor::Accent } else { ThemeColor::SurfaceRaised }, Capsule)
//!         }.into_view()
//!     }
//! }
//! ```

use std::ptr::NonNull;

use elegance_sys as sys;

use crate::error::ffi;
use crate::ffi as cffi;
use crate::view::View;

macro_rules! handle {
    ($name:ident, $raw:ident, $clone:ident, $free:ident) => {
        impl Clone for $name {
            fn clone(&self) -> Self {
                Self::from_raw(unsafe { sys::$clone(self.ptr.as_ptr()) })
            }
        }
        impl Drop for $name {
            fn drop(&mut self) {
                unsafe { sys::$free(self.ptr.as_ptr()) }
            }
        }
        impl $name {
            pub(crate) fn from_raw(ptr: *mut sys::$raw) -> Self {
                Self { ptr: NonNull::new(ptr).expect(concat!("elegance: no ", stringify!($name))) }
            }
            pub(crate) fn as_ptr(&self) -> *const sys::$raw {
                self.ptr.as_ptr()
            }
        }
    };
}

// --- Buttons ---

/// What a ButtonStyle sees: the label, and the button's state (tracked reads, so closures
/// that read them update). Clones refer to the same button.
pub struct ButtonConfiguration {
    ptr: NonNull<sys::ButtonConfiguration>,
}
handle!(ButtonConfiguration, ButtonConfiguration, elg_button_config_clone, elg_button_config_free);

impl ButtonConfiguration {
    pub fn label(&self) -> View {
        View::from_raw(unsafe { sys::elg_button_config_label(self.as_ptr()) })
    }
    /// Pressed with the pointer still over it.
    pub fn is_pressed(&self) -> bool {
        ffi!(sys::elg_button_config_state(self.as_ptr(), 0))
    }
    pub fn is_hovered(&self) -> bool {
        ffi!(sys::elg_button_config_state(self.as_ptr(), 1))
    }
    /// false inside `.disabled(true)`.
    pub fn is_enabled(&self) -> bool {
        ffi!(sys::elg_button_config_state(self.as_ptr(), 2))
    }
    /// Has keyboard focus.
    pub fn is_focused(&self) -> bool {
        ffi!(sys::elg_button_config_state(self.as_ptr(), 3))
    }
}

/// How a Button looks. `make_body` runs once per mounted button; react to its state with
/// reactive arguments that read the configuration.
pub trait ButtonStyle: 'static {
    fn make_body(&self, button: ButtonConfiguration) -> View;
}

/// A button style as the environment holds it: a built-in or Rust style. It can draw a button
/// itself, e.g. to wrap the current style.
pub struct AnyButtonStyle {
    ptr: NonNull<sys::ButtonStyle>,
}
handle!(AnyButtonStyle, ButtonStyle, elg_button_style_clone, elg_button_style_free);

impl AnyButtonStyle {
    pub fn make_body(&self, button: &ButtonConfiguration) -> View {
        View::from_raw(ffi!(sys::elg_button_style_make_body(self.as_ptr(), button.as_ptr())))
    }
}

/// Filled with the tint, white label: the default.
pub struct ProminentButtonStyle;
/// A tinted outline.
pub struct BorderedButtonStyle;
/// Just the label (tinted), dimmed while pressed.
pub struct PlainButtonStyle;

/// Anything `.button_style()` takes.
pub trait IntoButtonStyle {
    fn into_button_style(self) -> AnyButtonStyle;
}

impl IntoButtonStyle for AnyButtonStyle {
    fn into_button_style(self) -> AnyButtonStyle {
        self
    }
}
impl IntoButtonStyle for ProminentButtonStyle {
    fn into_button_style(self) -> AnyButtonStyle {
        AnyButtonStyle::from_raw(unsafe { sys::elg_button_style_builtin(0) })
    }
}
impl IntoButtonStyle for BorderedButtonStyle {
    fn into_button_style(self) -> AnyButtonStyle {
        AnyButtonStyle::from_raw(unsafe { sys::elg_button_style_builtin(1) })
    }
}
impl IntoButtonStyle for PlainButtonStyle {
    fn into_button_style(self) -> AnyButtonStyle {
        AnyButtonStyle::from_raw(unsafe { sys::elg_button_style_builtin(2) })
    }
}
impl<S: ButtonStyle> IntoButtonStyle for S {
    fn into_button_style(self) -> AnyButtonStyle {
        let make_body = cffi::closure(move |args, ret| unsafe {
            let button = ButtonConfiguration::from_raw(args as *mut sys::ButtonConfiguration);
            self.make_body(button).write_into(ret);
        });
        AnyButtonStyle::from_raw(ffi!(sys::elg_button_style_rust(make_body)))
    }
}

// --- Toggles ---

/// What a ToggleStyle sees.
pub struct ToggleConfiguration {
    ptr: NonNull<sys::ToggleConfiguration>,
}
handle!(ToggleConfiguration, ToggleConfiguration, elg_toggle_config_clone, elg_toggle_config_free);

impl ToggleConfiguration {
    pub fn label(&self) -> View {
        View::from_raw(unsafe { sys::elg_toggle_config_label(self.as_ptr()) })
    }
    /// A tracked read.
    pub fn is_on(&self) -> bool {
        ffi!(sys::elg_toggle_config_state(self.as_ptr(), 0))
    }
    pub fn is_pressed(&self) -> bool {
        ffi!(sys::elg_toggle_config_state(self.as_ptr(), 1))
    }
    pub fn is_hovered(&self) -> bool {
        ffi!(sys::elg_toggle_config_state(self.as_ptr(), 2))
    }
    pub fn is_enabled(&self) -> bool {
        ffi!(sys::elg_toggle_config_state(self.as_ptr(), 3))
    }
    /// Flips it (animated).
    pub fn toggle(&self) {
        ffi!(sys::elg_toggle_config_toggle(self.as_ptr()))
    }
}

pub trait ToggleStyle: 'static {
    fn make_body(&self, toggle: ToggleConfiguration) -> View;
}

pub struct AnyToggleStyle {
    ptr: NonNull<sys::ToggleStyle>,
}
handle!(AnyToggleStyle, ToggleStyle, elg_toggle_style_clone, elg_toggle_style_free);

impl AnyToggleStyle {
    pub fn make_body(&self, toggle: &ToggleConfiguration) -> View {
        View::from_raw(ffi!(sys::elg_toggle_style_make_body(self.as_ptr(), toggle.as_ptr())))
    }
}

/// A sliding switch after the label: the default.
pub struct SwitchToggleStyle;
/// A check box before the label.
pub struct CheckboxToggleStyle;

pub trait IntoToggleStyle {
    fn into_toggle_style(self) -> AnyToggleStyle;
}

impl IntoToggleStyle for AnyToggleStyle {
    fn into_toggle_style(self) -> AnyToggleStyle {
        self
    }
}
impl IntoToggleStyle for SwitchToggleStyle {
    fn into_toggle_style(self) -> AnyToggleStyle {
        AnyToggleStyle::from_raw(unsafe { sys::elg_toggle_style_builtin(0) })
    }
}
impl IntoToggleStyle for CheckboxToggleStyle {
    fn into_toggle_style(self) -> AnyToggleStyle {
        AnyToggleStyle::from_raw(unsafe { sys::elg_toggle_style_builtin(1) })
    }
}
impl<S: ToggleStyle> IntoToggleStyle for S {
    fn into_toggle_style(self) -> AnyToggleStyle {
        let make_body = cffi::closure(move |args, ret| unsafe {
            let toggle = ToggleConfiguration::from_raw(args as *mut sys::ToggleConfiguration);
            self.make_body(toggle).write_into(ret);
        });
        AnyToggleStyle::from_raw(ffi!(sys::elg_toggle_style_rust(make_body)))
    }
}

// --- Text fields ---

/// What a TextFieldStyle sees: the editable field, and its state.
pub struct TextFieldConfiguration {
    ptr: NonNull<sys::TextFieldConfiguration>,
}
handle!(TextFieldConfiguration, TextFieldConfiguration, elg_text_field_config_clone, elg_text_field_config_free);

impl TextFieldConfiguration {
    pub fn field(&self) -> View {
        View::from_raw(unsafe { sys::elg_text_field_config_field(self.as_ptr()) })
    }
    pub fn is_focused(&self) -> bool {
        ffi!(sys::elg_text_field_config_state(self.as_ptr(), 0))
    }
    pub fn is_hovered(&self) -> bool {
        ffi!(sys::elg_text_field_config_state(self.as_ptr(), 1))
    }
    pub fn is_enabled(&self) -> bool {
        ffi!(sys::elg_text_field_config_state(self.as_ptr(), 2))
    }
}

pub trait TextFieldStyle: 'static {
    fn make_body(&self, field: TextFieldConfiguration) -> View;
}

pub struct AnyTextFieldStyle {
    ptr: NonNull<sys::TextFieldStyle>,
}
handle!(AnyTextFieldStyle, TextFieldStyle, elg_text_field_style_clone, elg_text_field_style_free);

impl AnyTextFieldStyle {
    pub fn make_body(&self, field: &TextFieldConfiguration) -> View {
        View::from_raw(ffi!(sys::elg_text_field_style_make_body(self.as_ptr(), field.as_ptr())))
    }
}

/// A rounded border that takes the tint while focused: the default.
pub struct RoundedBorderTextFieldStyle;
/// Just the text.
pub struct PlainTextFieldStyle;

pub trait IntoTextFieldStyle {
    fn into_text_field_style(self) -> AnyTextFieldStyle;
}

impl IntoTextFieldStyle for AnyTextFieldStyle {
    fn into_text_field_style(self) -> AnyTextFieldStyle {
        self
    }
}
impl IntoTextFieldStyle for RoundedBorderTextFieldStyle {
    fn into_text_field_style(self) -> AnyTextFieldStyle {
        AnyTextFieldStyle::from_raw(unsafe { sys::elg_text_field_style_builtin(0) })
    }
}
impl IntoTextFieldStyle for PlainTextFieldStyle {
    fn into_text_field_style(self) -> AnyTextFieldStyle {
        AnyTextFieldStyle::from_raw(unsafe { sys::elg_text_field_style_builtin(1) })
    }
}
impl<S: TextFieldStyle> IntoTextFieldStyle for S {
    fn into_text_field_style(self) -> AnyTextFieldStyle {
        let make_body = cffi::closure(move |args, ret| unsafe {
            let field = TextFieldConfiguration::from_raw(args as *mut sys::TextFieldConfiguration);
            self.make_body(field).write_into(ret);
        });
        AnyTextFieldStyle::from_raw(ffi!(sys::elg_text_field_style_rust(make_body)))
    }
}
