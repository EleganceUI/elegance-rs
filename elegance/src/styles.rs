//! Styles: how each control looks. Every control draws through a style from the environment,
//! set for a subtree with its modifier (`.button_style(...)`, `.toggle_style(...)`, ...). Each
//! family has built-in styles, some with options, and a trait for styles of your own:
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
//!
//! `make_body` runs once per control when it's built, while `Environment` is available. A
//! configuration holds the control's parts as views, its state as tracked reads (use them in
//! reactive arguments), and its actions. Clones of a configuration refer to the same control.

use std::ptr::NonNull;

use elegance_sys as sys;

use crate::animation::Animation;
use crate::color::Color;
use crate::error::ffi;
use crate::ffi as cffi;
use crate::font::Font;
use crate::geometry::{Axis, EdgeInsets, Number};
use crate::prop::{IntoProp, OwnedProp, Prop};
use crate::view::{c_motion, IntoView, PopupMotion, View};

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

// A style family's handle (`AnyXStyle`), as the environment holds it, and the trait for anything
// its modifier takes.
macro_rules! family {
    ($(#[$m:meta])* $any:ident: $raw:ident, $clone:ident, $free:ident, $modifier:ident, $into:ident::$method:ident) => {
        $(#[$m])*
        pub struct $any {
            ptr: NonNull<sys::$raw>,
        }
        handle!($any, $raw, $clone, $free);

        impl $any {
            pub(crate) fn apply(self, view: View) -> View {
                view.modify(|v| unsafe { sys::$modifier(v, self.as_ptr()) })
            }
        }

        #[doc = concat!("Anything `.", stringify!($method), "`'s modifier takes: a built-in style, a style of your own, or an [`", stringify!($any), "`].")]
        pub trait $into {
            fn $method(self) -> $any;
        }
        impl $into for $any {
            fn $method(self) -> $any {
                self
            }
        }
    };
}

// A configuration handle.
macro_rules! configuration {
    ($(#[$m:meta])* $name:ident, $clone:ident, $free:ident) => {
        $(#[$m])*
        pub struct $name {
            ptr: NonNull<sys::$name>,
        }
        handle!($name, $name, $clone, $free);
    };
}

// Tracked state readers: `fn is_hovered(&self) -> bool` for each (name, which).
macro_rules! states {
    ($get:ident: $($(#[$m:meta])* $name:ident = $which:expr_2021),* $(,)?) => {$(
        $(#[$m])*
        pub fn $name(&self) -> bool {
            ffi!(sys::$get(self.as_ptr(), $which))
        }
    )*};
}

// A built-in style without options, as a unit struct.
macro_rules! builtin {
    ($(#[$m:meta])* $name:ident => $into:ident::$method:ident -> $any:ident, $make:expr_2021) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $name;
        impl $into for $name {
            fn $method(self) -> $any {
                $any::from_raw(unsafe { $make })
            }
        }
    };
}

// A Rust style whose trait has `make_body` only.
macro_rules! rust_style {
    ($trait:ident, $config:ident, $into:ident::$method:ident -> $any:ident, $rust:ident) => {
        impl<S: $trait> $into for S {
            fn $method(self) -> $any {
                let make_body = cffi::closure(move |args, ret| unsafe {
                    self.make_body($config::from_raw(args as *mut sys::$config)).write_into(ret);
                });
                $any::from_raw(ffi!(sys::$rust(make_body)))
            }
        }
    };
}

fn view_of(ptr: *mut sys::View) -> View {
    View::from_raw(ptr)
}

fn animation_ptr(animation: &Option<Animation>) -> *const sys::Animation {
    animation.as_ref().map_or(std::ptr::null(), Animation::as_ptr)
}

fn owned_color(color: Option<Prop<Color>>) -> Option<OwnedProp> {
    color.map(OwnedProp::color)
}

// --- Buttons ---

configuration!(
    /// What a ButtonStyle sees: the label, and the button's state.
    ButtonConfiguration, elg_button_config_clone, elg_button_config_free
);

impl ButtonConfiguration {
    pub fn label(&self) -> View {
        view_of(unsafe { sys::elg_button_config_label(self.as_ptr()) })
    }
    states!(elg_button_config_state:
        /// Pressed with the pointer still over it.
        is_pressed = 0,
        is_hovered = 1,
        /// false inside `.disabled(true)`.
        is_enabled = 2,
        /// Has keyboard focus.
        is_focused = 3,
    );
}

/// How a Button looks.
pub trait ButtonStyle: 'static {
    fn make_body(&self, button: ButtonConfiguration) -> View;
}

family!(
    /// A button style as the environment holds it: a built-in or Rust style. It can draw a
    /// button itself, e.g. to wrap the current style.
    AnyButtonStyle: ButtonStyle, elg_button_style_clone, elg_button_style_free, elg_mod_button_style,
    IntoButtonStyle::into_button_style
);

impl AnyButtonStyle {
    pub fn make_body(&self, button: &ButtonConfiguration) -> View {
        view_of(ffi!(sys::elg_button_style_make_body(self.as_ptr(), button.as_ptr())))
    }
}

builtin!(
    /// Filled with the tint, white label: the default.
    ProminentButtonStyle => IntoButtonStyle::into_button_style -> AnyButtonStyle, sys::elg_button_style_builtin(0)
);
builtin!(
    /// A tinted outline.
    BorderedButtonStyle => IntoButtonStyle::into_button_style -> AnyButtonStyle, sys::elg_button_style_builtin(1)
);
builtin!(
    /// Just the label (tinted), dimmed while pressed.
    PlainButtonStyle => IntoButtonStyle::into_button_style -> AnyButtonStyle, sys::elg_button_style_builtin(2)
);
rust_style!(ButtonStyle, ButtonConfiguration, IntoButtonStyle::into_button_style -> AnyButtonStyle, elg_button_style_rust);

// --- Toggles ---

configuration!(
    /// What a ToggleStyle sees.
    ToggleConfiguration, elg_toggle_config_clone, elg_toggle_config_free
);

impl ToggleConfiguration {
    /// Empty (`View::is_empty`) for a toggle made without a label.
    pub fn label(&self) -> View {
        view_of(unsafe { sys::elg_toggle_config_label(self.as_ptr()) })
    }
    states!(elg_toggle_config_state:
        is_on = 0,
        is_pressed = 1,
        is_hovered = 2,
        is_enabled = 3,
        is_focused = 4,
    );
    /// Flips it (with the style's animation).
    pub fn toggle(&self) {
        ffi!(sys::elg_toggle_config_toggle(self.as_ptr()))
    }
}

pub trait ToggleStyle: 'static {
    fn make_body(&self, toggle: ToggleConfiguration) -> View;
    /// How a flip animates; None: at once.
    fn animation(&self) -> Option<Animation> {
        Some(Animation::smooth(0.3))
    }
}

family!(
    AnyToggleStyle: ToggleStyle, elg_toggle_style_clone, elg_toggle_style_free, elg_mod_toggle_style,
    IntoToggleStyle::into_toggle_style
);

impl AnyToggleStyle {
    pub fn make_body(&self, toggle: &ToggleConfiguration) -> View {
        view_of(ffi!(sys::elg_toggle_style_make_body(self.as_ptr(), toggle.as_ptr())))
    }
}

impl<S: ToggleStyle> IntoToggleStyle for S {
    fn into_toggle_style(self) -> AnyToggleStyle {
        let animation = self.animation();
        let make_body = cffi::closure(move |args, ret| unsafe {
            self.make_body(ToggleConfiguration::from_raw(args as *mut sys::ToggleConfiguration)).write_into(ret);
        });
        AnyToggleStyle::from_raw(ffi!(sys::elg_toggle_style_rust(make_body, animation_ptr(&animation))))
    }
}

/// The label, then a sliding switch in the tint: the default.
///
/// `Toggle("Wi-Fi", wifi).toggle_style(SwitchToggleStyle::new().on(.success).fills_width(true))`
#[derive(Clone)]
pub struct SwitchToggleStyle {
    /// The track while on; None: the tint.
    pub on: Option<Prop<Color>>,
    /// The track while off.
    pub off: Prop<Color>,
    pub knob: Prop<Color>,
    pub width: f32,
    pub height: f32,
    /// Between the knob and the track's edge.
    pub knob_inset: f32,
    /// Between the label and the switch.
    pub spacing: f32,
    /// The label at the leading edge and the switch at the trailing one, across the width it's
    /// offered (a settings row). Otherwise it's as wide as the label and switch.
    pub fills_width: bool,
    /// None: flips at once.
    pub animation: Option<Animation>,
}

impl Default for SwitchToggleStyle {
    fn default() -> Self {
        Self {
            on: None,
            off: Prop::Theme(crate::color::ThemeColor::Outline),
            knob: Prop::Value(Color::WHITE),
            width: 40.0,
            height: 24.0,
            knob_inset: 2.0,
            spacing: 8.0,
            fills_width: false,
            animation: Some(Animation::smooth(0.3)),
        }
    }
}

#[elegance_macros::elg_builder]
impl SwitchToggleStyle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn on(mut self, color: impl IntoProp<Color>) -> Self {
        self.on = Some(color.into_prop());
        self
    }
    pub fn off(mut self, color: impl IntoProp<Color>) -> Self {
        self.off = color.into_prop();
        self
    }
    pub fn knob(mut self, color: impl IntoProp<Color>) -> Self {
        self.knob = color.into_prop();
        self
    }
    pub fn size(mut self, width: impl Number, height: impl Number) -> Self {
        (self.width, self.height) = (width.to_f32(), height.to_f32());
        self
    }
    pub fn knob_inset(mut self, inset: impl Number) -> Self {
        self.knob_inset = inset.to_f32();
        self
    }
    pub fn spacing(mut self, spacing: impl Number) -> Self {
        self.spacing = spacing.to_f32();
        self
    }
    pub fn fills_width(mut self, fills: bool) -> Self {
        self.fills_width = fills;
        self
    }
    pub fn animation(mut self, animation: Option<Animation>) -> Self {
        self.animation = animation;
        self
    }
}

impl IntoToggleStyle for SwitchToggleStyle {
    fn into_toggle_style(self) -> AnyToggleStyle {
        // Kept until the call: the static colors' storage.
        let (on, off, knob) = (owned_color(self.on), OwnedProp::color(self.off), OwnedProp::color(self.knob));
        let options = sys::ElgSwitchToggleStyleOptions {
            on: OwnedProp::ptr(&on),
            off: off.c,
            knob: knob.c,
            width: self.width,
            height: self.height,
            knob_inset: self.knob_inset,
            spacing: self.spacing,
            fills_width: self.fills_width,
            animation: animation_ptr(&self.animation),
        };
        AnyToggleStyle::from_raw(ffi!(sys::elg_switch_toggle_style(&options)))
    }
}

// --- Check boxes ---

configuration!(
    /// What a CheckboxStyle sees.
    CheckboxConfiguration, elg_checkbox_config_clone, elg_checkbox_config_free
);

impl CheckboxConfiguration {
    /// Empty (`View::is_empty`) for a check box made without a label.
    pub fn label(&self) -> View {
        view_of(unsafe { sys::elg_checkbox_config_label(self.as_ptr()) })
    }
    states!(elg_checkbox_config_state:
        is_checked = 0,
        /// Some of what it stands for is checked (`Checkbox::mixed`).
        is_mixed = 1,
        is_pressed = 2,
        is_hovered = 3,
        is_enabled = 4,
        is_focused = 5,
    );
    /// Checks or unchecks it; a mixed one becomes checked.
    pub fn toggle(&self) {
        ffi!(sys::elg_checkbox_config_toggle(self.as_ptr()))
    }
}

pub trait CheckboxStyle: 'static {
    fn make_body(&self, checkbox: CheckboxConfiguration) -> View;
    /// How checking and unchecking animate; None: at once.
    fn animation(&self) -> Option<Animation> {
        Some(Animation::smooth(0.3))
    }
}

family!(
    AnyCheckboxStyle: CheckboxStyle, elg_checkbox_style_clone, elg_checkbox_style_free, elg_mod_checkbox_style,
    IntoCheckboxStyle::into_checkbox_style
);

impl AnyCheckboxStyle {
    pub fn make_body(&self, checkbox: &CheckboxConfiguration) -> View {
        view_of(ffi!(sys::elg_checkbox_style_make_body(self.as_ptr(), checkbox.as_ptr())))
    }
}

impl<S: CheckboxStyle> IntoCheckboxStyle for S {
    fn into_checkbox_style(self) -> AnyCheckboxStyle {
        let animation = self.animation();
        let make_body = cffi::closure(move |args, ret| unsafe {
            self.make_body(CheckboxConfiguration::from_raw(args as *mut sys::CheckboxConfiguration)).write_into(ret);
        });
        AnyCheckboxStyle::from_raw(ffi!(sys::elg_checkbox_style_rust(make_body, animation_ptr(&animation))))
    }
}

/// A box, then the label: filled in the tint with a check mark while checked, a dash while
/// mixed. The default.
#[derive(Clone)]
pub struct BoxCheckboxStyle {
    pub size: f32,
    pub corner_radius: f32,
    /// Checked or mixed; None: the tint.
    pub fill: Option<Prop<Color>>,
    /// Unchecked.
    pub empty: Prop<Color>,
    /// Unchecked; checked or mixed, it's the fill.
    pub border: Prop<Color>,
    pub border_width: f32,
    /// The check and the dash.
    pub mark: Prop<Color>,
    /// Shown while checked; None: a check mark.
    pub check: Option<View>,
    /// Shown while mixed; None: a dash.
    pub dash: Option<View>,
    /// Between the box and the label.
    pub spacing: f32,
    pub animation: Option<Animation>,
}

impl Default for BoxCheckboxStyle {
    fn default() -> Self {
        use crate::color::ThemeColor;
        Self {
            size: 18.0,
            corner_radius: 5.0,
            fill: None,
            empty: Prop::Theme(ThemeColor::SurfaceRaised),
            border: Prop::Theme(ThemeColor::Outline),
            border_width: 1.5,
            mark: Prop::Theme(ThemeColor::OnAccent),
            check: None,
            dash: None,
            spacing: 8.0,
            animation: Some(Animation::smooth(0.3)),
        }
    }
}

#[elegance_macros::elg_builder]
impl BoxCheckboxStyle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn size(mut self, size: impl Number) -> Self {
        self.size = size.to_f32();
        self
    }
    pub fn corner_radius(mut self, radius: impl Number) -> Self {
        self.corner_radius = radius.to_f32();
        self
    }
    pub fn fill(mut self, color: impl IntoProp<Color>) -> Self {
        self.fill = Some(color.into_prop());
        self
    }
    pub fn empty(mut self, color: impl IntoProp<Color>) -> Self {
        self.empty = color.into_prop();
        self
    }
    pub fn border(mut self, color: impl IntoProp<Color>) -> Self {
        self.border = color.into_prop();
        self
    }
    pub fn border_width(mut self, width: impl Number) -> Self {
        self.border_width = width.to_f32();
        self
    }
    pub fn mark(mut self, color: impl IntoProp<Color>) -> Self {
        self.mark = color.into_prop();
        self
    }
    pub fn check(mut self, check: impl IntoView) -> Self {
        self.check = Some(check.into_view());
        self
    }
    pub fn dash(mut self, dash: impl IntoView) -> Self {
        self.dash = Some(dash.into_view());
        self
    }
    pub fn spacing(mut self, spacing: impl Number) -> Self {
        self.spacing = spacing.to_f32();
        self
    }
    pub fn animation(mut self, animation: Option<Animation>) -> Self {
        self.animation = animation;
        self
    }
}

impl IntoCheckboxStyle for BoxCheckboxStyle {
    fn into_checkbox_style(self) -> AnyCheckboxStyle {
        let fill = owned_color(self.fill);
        let (empty, border, mark) =
            (OwnedProp::color(self.empty), OwnedProp::color(self.border), OwnedProp::color(self.mark));
        let options = sys::ElgBoxCheckboxStyleOptions {
            size: self.size,
            corner_radius: self.corner_radius,
            fill: OwnedProp::ptr(&fill),
            empty: empty.c,
            border: border.c,
            border_width: self.border_width,
            mark: mark.c,
            take_check: self.check.map_or(std::ptr::null_mut(), View::into_raw),
            take_dash: self.dash.map_or(std::ptr::null_mut(), View::into_raw),
            spacing: self.spacing,
            animation: animation_ptr(&self.animation),
        };
        AnyCheckboxStyle::from_raw(ffi!(sys::elg_box_checkbox_style(&options)))
    }
}

// --- Number inputs ---

configuration!(
    /// What a NumberInputStyle sees: the editable number (put it where the text goes), the steps,
    /// and the state.
    NumberInputConfiguration, elg_number_input_config_clone, elg_number_input_config_free
);

impl NumberInputConfiguration {
    pub fn field(&self) -> View {
        view_of(unsafe { sys::elg_number_input_config_field(self.as_ptr()) })
    }
    states!(elg_number_input_config_state:
        /// Below the maximum.
        can_increment = 0,
        /// Above the minimum.
        can_decrement = 1,
        /// `NumberInput::shows_buttons`.
        shows_buttons = 2,
        /// Being edited.
        is_focused = 3,
        is_hovered = 4,
        is_enabled = 5,
    );
    /// A step up, within the range.
    pub fn increment(&self) {
        ffi!(sys::elg_number_input_config_action(self.as_ptr(), 0))
    }
    pub fn decrement(&self) {
        ffi!(sys::elg_number_input_config_action(self.as_ptr(), 1))
    }
}

pub trait NumberInputStyle: 'static {
    fn make_body(&self, input: NumberInputConfiguration) -> View;
}

family!(
    AnyNumberInputStyle: NumberInputStyle, elg_number_input_style_clone, elg_number_input_style_free,
    elg_mod_number_input_style, IntoNumberInputStyle::into_number_input_style
);

impl AnyNumberInputStyle {
    pub fn make_body(&self, input: &NumberInputConfiguration) -> View {
        view_of(ffi!(sys::elg_number_input_style_make_body(self.as_ptr(), input.as_ptr())))
    }
}

rust_style!(
    NumberInputStyle, NumberInputConfiguration, IntoNumberInputStyle::into_number_input_style -> AnyNumberInputStyle,
    elg_number_input_style_rust
);
builtin!(
    /// Just the number, and the arrows if shown: for putting it in a box of your own.
    PlainNumberInputStyle => IntoNumberInputStyle::into_number_input_style -> AnyNumberInputStyle,
    sys::elg_number_input_style_builtin(std::ptr::null())
);

/// A raised box with an outline (the tint while editing), and stacked arrows at its trailing
/// edge. The default.
#[derive(Clone)]
pub struct BoxNumberInputStyle {
    pub background: Prop<Color>,
    pub border: Prop<Color>,
    /// None: the tint.
    pub focused_border: Option<Prop<Color>>,
    pub border_width: f32,
    pub corner_radius: f32,
    pub padding: EdgeInsets,
    /// The stacked arrows' icons.
    pub arrow_size: f32,
}

impl Default for BoxNumberInputStyle {
    fn default() -> Self {
        use crate::color::ThemeColor;
        Self {
            background: Prop::Theme(ThemeColor::SurfaceRaised),
            border: Prop::Theme(ThemeColor::Outline),
            focused_border: None,
            border_width: 1.0,
            corner_radius: 6.0,
            padding: EdgeInsets::symmetric(8.0, 5.0),
            arrow_size: 9.0,
        }
    }
}

#[elegance_macros::elg_builder]
impl BoxNumberInputStyle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn background(mut self, color: impl IntoProp<Color>) -> Self {
        self.background = color.into_prop();
        self
    }
    pub fn border(mut self, color: impl IntoProp<Color>) -> Self {
        self.border = color.into_prop();
        self
    }
    pub fn focused_border(mut self, color: impl IntoProp<Color>) -> Self {
        self.focused_border = Some(color.into_prop());
        self
    }
    pub fn border_width(mut self, width: impl Number) -> Self {
        self.border_width = width.to_f32();
        self
    }
    pub fn corner_radius(mut self, radius: impl Number) -> Self {
        self.corner_radius = radius.to_f32();
        self
    }
    pub fn padding(mut self, padding: impl Into<EdgeInsets>) -> Self {
        self.padding = padding.into();
        self
    }
    pub fn arrow_size(mut self, size: impl Number) -> Self {
        self.arrow_size = size.to_f32();
        self
    }
}

impl IntoNumberInputStyle for BoxNumberInputStyle {
    fn into_number_input_style(self) -> AnyNumberInputStyle {
        let focused_border = owned_color(self.focused_border);
        let (background, border) = (OwnedProp::color(self.background), OwnedProp::color(self.border));
        let options = sys::ElgBoxNumberInputStyleOptions {
            background: background.c,
            border: border.c,
            focused_border: OwnedProp::ptr(&focused_border),
            border_width: self.border_width,
            corner_radius: self.corner_radius,
            padding: self.padding.c(),
            arrow_size: self.arrow_size,
        };
        AnyNumberInputStyle::from_raw(ffi!(sys::elg_number_input_style_builtin(&options)))
    }
}

// --- Progress ---

configuration!(
    /// What a ProgressStyle sees: how far along it is.
    ProgressConfiguration, elg_progress_config_clone, elg_progress_config_free
);

impl ProgressConfiguration {
    /// 0...1; None: indeterminate (activity of unknown length). A tracked read.
    pub fn fraction(&self) -> Option<f32> {
        let f = ffi!(sys::elg_progress_config_fraction(self.as_ptr()));
        f.has.then_some(f.value)
    }
}

pub trait ProgressStyle: 'static {
    fn make_body(&self, progress: ProgressConfiguration) -> View;
}

family!(
    AnyProgressStyle: ProgressStyle, elg_progress_style_clone, elg_progress_style_free, elg_mod_progress_style,
    IntoProgressStyle::into_progress_style
);

impl AnyProgressStyle {
    pub fn make_body(&self, progress: &ProgressConfiguration) -> View {
        view_of(ffi!(sys::elg_progress_style_make_body(self.as_ptr(), progress.as_ptr())))
    }
}

rust_style!(
    ProgressStyle, ProgressConfiguration, IntoProgressStyle::into_progress_style -> AnyProgressStyle,
    elg_progress_style_rust
);

/// A bar as wide as offered, filled to the fraction, or with a segment sweeping across while
/// indeterminate. The default.
#[derive(Clone)]
pub struct LinearProgressStyle {
    pub track: Prop<Color>,
    /// None: the tint.
    pub fill: Option<Prop<Color>>,
    pub thickness: f32,
}

impl Default for LinearProgressStyle {
    fn default() -> Self {
        Self { track: Prop::Theme(crate::color::ThemeColor::Outline), fill: None, thickness: 6.0 }
    }
}

/// A ring filled clockwise from the top to the fraction, or with an arc spinning while
/// indeterminate.
#[derive(Clone)]
pub struct CircularProgressStyle {
    pub track: Prop<Color>,
    /// None: the tint.
    pub fill: Option<Prop<Color>>,
    /// The ring's diameter.
    pub size: f32,
    pub line_width: f32,
}

impl Default for CircularProgressStyle {
    fn default() -> Self {
        Self { track: Prop::Theme(crate::color::ThemeColor::Outline), fill: None, size: 20.0, line_width: 3.0 }
    }
}

#[elegance_macros::elg_builder]
impl LinearProgressStyle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn track(mut self, color: impl IntoProp<Color>) -> Self {
        self.track = color.into_prop();
        self
    }
    pub fn fill(mut self, color: impl IntoProp<Color>) -> Self {
        self.fill = Some(color.into_prop());
        self
    }
    pub fn thickness(mut self, thickness: impl Number) -> Self {
        self.thickness = thickness.to_f32();
        self
    }
}

#[elegance_macros::elg_builder]
impl CircularProgressStyle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn track(mut self, color: impl IntoProp<Color>) -> Self {
        self.track = color.into_prop();
        self
    }
    pub fn fill(mut self, color: impl IntoProp<Color>) -> Self {
        self.fill = Some(color.into_prop());
        self
    }
    pub fn size(mut self, size: impl Number) -> Self {
        self.size = size.to_f32();
        self
    }
    pub fn line_width(mut self, width: impl Number) -> Self {
        self.line_width = width.to_f32();
        self
    }
}

fn progress_style(kind: i32, track: Prop<Color>, fill: Option<Prop<Color>>, a: f32, b: f32) -> AnyProgressStyle {
    let (track, fill) = (OwnedProp::color(track), owned_color(fill));
    let options = sys::ElgProgressStyleOptions { track: track.c, fill: OwnedProp::ptr(&fill), a, b };
    AnyProgressStyle::from_raw(ffi!(sys::elg_progress_style_builtin(kind, &options)))
}

impl IntoProgressStyle for LinearProgressStyle {
    fn into_progress_style(self) -> AnyProgressStyle {
        progress_style(0, self.track, self.fill, self.thickness, 0.0)
    }
}

impl IntoProgressStyle for CircularProgressStyle {
    fn into_progress_style(self) -> AnyProgressStyle {
        progress_style(1, self.track, self.fill, self.size, self.line_width)
    }
}

// --- Text fields ---

configuration!(
    /// What a TextFieldStyle sees: the editable field, and its state.
    TextFieldConfiguration, elg_text_field_config_clone, elg_text_field_config_free
);

impl TextFieldConfiguration {
    pub fn field(&self) -> View {
        view_of(unsafe { sys::elg_text_field_config_field(self.as_ptr()) })
    }
    states!(elg_text_field_config_state:
        is_focused = 0,
        is_hovered = 1,
        is_enabled = 2,
    );
}

/// How a TextField and a TextArea look.
pub trait TextFieldStyle: 'static {
    fn make_body(&self, field: TextFieldConfiguration) -> View;
}

family!(
    AnyTextFieldStyle: TextFieldStyle, elg_text_field_style_clone, elg_text_field_style_free,
    elg_mod_text_field_style, IntoTextFieldStyle::into_text_field_style
);

impl AnyTextFieldStyle {
    pub fn make_body(&self, field: &TextFieldConfiguration) -> View {
        view_of(ffi!(sys::elg_text_field_style_make_body(self.as_ptr(), field.as_ptr())))
    }
}

builtin!(
    /// A rounded border that takes the tint while focused: the default.
    RoundedBorderTextFieldStyle => IntoTextFieldStyle::into_text_field_style -> AnyTextFieldStyle,
    sys::elg_text_field_style_builtin(0)
);
builtin!(
    /// Just the text.
    PlainTextFieldStyle => IntoTextFieldStyle::into_text_field_style -> AnyTextFieldStyle,
    sys::elg_text_field_style_builtin(1)
);
rust_style!(
    TextFieldStyle, TextFieldConfiguration, IntoTextFieldStyle::into_text_field_style -> AnyTextFieldStyle,
    elg_text_field_style_rust
);

// --- Pickers ---

configuration!(
    /// What a PickerStyle sees: the options' labels, which one is chosen, and choosing one.
    PickerConfiguration, elg_picker_config_clone, elg_picker_config_free
);

impl PickerConfiguration {
    pub fn labels(&self) -> Vec<String> {
        let count = unsafe { sys::elg_picker_config_count(self.as_ptr()) };
        (0..count)
            .map(|i| unsafe { sys::elg_picker_config_label(self.as_ptr(), i).as_str() }.to_owned())
            .collect()
    }
    /// The chosen option's index; None while none is. A tracked read.
    pub fn selected(&self) -> Option<usize> {
        let count = unsafe { sys::elg_picker_config_count(self.as_ptr()) };
        let index = ffi!(sys::elg_picker_config_selected(self.as_ptr()));
        (index < count).then_some(index)
    }
    pub fn select(&self, index: usize) {
        ffi!(sys::elg_picker_config_select(self.as_ptr(), index))
    }
}

pub trait PickerStyle: 'static {
    fn make_body(&self, picker: PickerConfiguration) -> View;
}

family!(
    AnyPickerStyle: PickerStyle, elg_picker_style_clone, elg_picker_style_free, elg_mod_picker_style,
    IntoPickerStyle::into_picker_style
);

impl AnyPickerStyle {
    pub fn make_body(&self, picker: &PickerConfiguration) -> View {
        view_of(ffi!(sys::elg_picker_style_make_body(self.as_ptr(), picker.as_ptr())))
    }
}

builtin!(
    /// Every option visible, as segments in a rounded track; the chosen one is raised. The default.
    SegmentedPickerStyle => IntoPickerStyle::into_picker_style -> AnyPickerStyle, sys::elg_picker_style_builtin(0)
);
builtin!(
    /// The chosen option on a button; the others in a menu it opens, the chosen one checked.
    MenuPickerStyle => IntoPickerStyle::into_picker_style -> AnyPickerStyle, sys::elg_picker_style_builtin(1)
);
rust_style!(PickerStyle, PickerConfiguration, IntoPickerStyle::into_picker_style -> AnyPickerStyle, elg_picker_style_rust);

// --- Sliders ---

configuration!(
    /// What a SliderStyle sees: where the value is and the slider's state.
    SliderConfiguration, elg_slider_config_clone, elg_slider_config_free
);

impl SliderConfiguration {
    /// Where the value is between min and max, 0...1. A tracked read.
    pub fn fraction(&self) -> f32 {
        ffi!(sys::elg_slider_config_fraction(self.as_ptr()))
    }
    states!(elg_slider_config_state:
        /// Being dragged.
        is_pressed = 0,
        is_hovered = 1,
        is_enabled = 2,
        is_focused = 3,
    );
}

/// How a Slider looks, in three parts the slider places itself, so a style doesn't need to know
/// its width:
/// - the track, offered the slider's length less the thumb's width (centered, so the thumb stays
///   inside at either end) and taking its own height;
/// - the fill, over the track from its start to the thumb's center (nothing by default);
/// - the thumb, at its ideal size, centered on the value.
///
/// The slider is as tall as the taller of the track and the thumb. It handles the pointer, keys
/// and accessibility itself; the parts only draw.
pub trait SliderStyle: 'static {
    fn make_track(&self, slider: SliderConfiguration) -> View;
    fn make_fill(&self, slider: SliderConfiguration) -> View {
        let _ = slider;
        View::empty()
    }
    fn make_thumb(&self, slider: SliderConfiguration) -> View;
}

family!(
    AnySliderStyle: SliderStyle, elg_slider_style_clone, elg_slider_style_free, elg_mod_slider_style,
    IntoSliderStyle::into_slider_style
);

impl AnySliderStyle {
    pub fn make_track(&self, slider: &SliderConfiguration) -> View {
        view_of(ffi!(sys::elg_slider_style_make_part(self.as_ptr(), slider.as_ptr(), 0)))
    }
    pub fn make_fill(&self, slider: &SliderConfiguration) -> View {
        view_of(ffi!(sys::elg_slider_style_make_part(self.as_ptr(), slider.as_ptr(), 1)))
    }
    pub fn make_thumb(&self, slider: &SliderConfiguration) -> View {
        view_of(ffi!(sys::elg_slider_style_make_part(self.as_ptr(), slider.as_ptr(), 2)))
    }
}

impl<S: SliderStyle> IntoSliderStyle for S {
    fn into_slider_style(self) -> AnySliderStyle {
        let style = std::rc::Rc::new(self);
        let part = |make: fn(&S, SliderConfiguration) -> View| {
            let style = style.clone();
            cffi::closure(move |args, ret| unsafe {
                make(&style, SliderConfiguration::from_raw(args as *mut sys::SliderConfiguration)).write_into(ret);
            })
        };
        let (track, fill, thumb) = (part(S::make_track), part(S::make_fill), part(S::make_thumb));
        AnySliderStyle::from_raw(ffi!(sys::elg_slider_style_rust(track, fill, thumb)))
    }
}

/// A thin capsule track, filled in the tint up to a round white knob with a soft shadow. The
/// default.
#[derive(Clone)]
pub struct KnobSliderStyle {
    pub track: Prop<Color>,
    /// None: the tint.
    pub fill: Option<Prop<Color>>,
    pub track_thickness: f32,
    pub knob_size: f32,
}

impl Default for KnobSliderStyle {
    fn default() -> Self {
        Self { track: Prop::Theme(crate::color::ThemeColor::Outline), fill: None, track_thickness: 4.0, knob_size: 20.0 }
    }
}

#[elegance_macros::elg_builder]
impl KnobSliderStyle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn track(mut self, color: impl IntoProp<Color>) -> Self {
        self.track = color.into_prop();
        self
    }
    pub fn fill(mut self, color: impl IntoProp<Color>) -> Self {
        self.fill = Some(color.into_prop());
        self
    }
    pub fn track_thickness(mut self, thickness: impl Number) -> Self {
        self.track_thickness = thickness.to_f32();
        self
    }
    pub fn knob_size(mut self, size: impl Number) -> Self {
        self.knob_size = size.to_f32();
        self
    }
}

impl IntoSliderStyle for KnobSliderStyle {
    fn into_slider_style(self) -> AnySliderStyle {
        let (track, fill) = (OwnedProp::color(self.track), owned_color(self.fill));
        let options = sys::ElgKnobSliderStyleOptions {
            track: track.c,
            fill: OwnedProp::ptr(&fill),
            track_thickness: self.track_thickness,
            knob_size: self.knob_size,
        };
        AnySliderStyle::from_raw(ffi!(sys::elg_knob_slider_style(&options)))
    }
}

// --- Navigation stacks ---

configuration!(
    /// What a NavigationStackStyle sees: the pages (put them where they go), their titles, and
    /// going back.
    NavigationStackConfiguration, elg_navigation_stack_config_clone, elg_navigation_stack_config_free
);

impl NavigationStackConfiguration {
    pub fn pages(&self) -> View {
        view_of(unsafe { sys::elg_navigation_stack_config_pages(self.as_ptr()) })
    }
    /// The root's title, then each pushed page's. A tracked read.
    pub fn titles(&self) -> Vec<String> {
        unsafe extern "C" fn push(ctx: *mut std::ffi::c_void, ptr: *const std::ffi::c_char, len: usize) {
            let titles = unsafe { &mut *(ctx as *mut Vec<String>) };
            let bytes = if ptr.is_null() { &[][..] } else { unsafe { std::slice::from_raw_parts(ptr.cast(), len) } };
            titles.push(String::from_utf8_lossy(bytes).into_owned());
        }
        let mut titles = Vec::new();
        ffi!(sys::elg_navigation_stack_config_titles(self.as_ptr(), push, (&raw mut titles).cast()));
        titles
    }
    /// A page is pushed. A tracked read.
    pub fn can_go_back(&self) -> bool {
        ffi!(sys::elg_navigation_stack_config_can_go_back(self.as_ptr()))
    }
    /// Pops the top page.
    pub fn back(&self) {
        ffi!(sys::elg_navigation_stack_config_back(self.as_ptr()))
    }
}

pub trait NavigationStackStyle: 'static {
    fn make_body(&self, stack: NavigationStackConfiguration) -> View;
}

family!(
    AnyNavigationStackStyle: NavigationStackStyle, elg_navigation_stack_style_clone, elg_navigation_stack_style_free,
    elg_mod_navigation_stack_style, IntoNavigationStackStyle::into_navigation_stack_style
);

impl AnyNavigationStackStyle {
    pub fn make_body(&self, stack: &NavigationStackConfiguration) -> View {
        view_of(ffi!(sys::elg_navigation_stack_style_make_body(self.as_ptr(), stack.as_ptr())))
    }
}

rust_style!(
    NavigationStackStyle, NavigationStackConfiguration,
    IntoNavigationStackStyle::into_navigation_stack_style -> AnyNavigationStackStyle, elg_navigation_stack_style_rust
);
builtin!(
    /// Just the pages: go back from your own controls (or the back gestures). The default.
    PlainNavigationStackStyle => IntoNavigationStackStyle::into_navigation_stack_style -> AnyNavigationStackStyle,
    sys::elg_navigation_stack_style_builtin(std::ptr::null())
);

/// A bar over the pages with the top page's title, centered, and once a page is pushed, a back
/// button named after the page underneath.
#[derive(Clone)]
pub struct BarNavigationStackStyle {
    pub height: f32,
    pub background: Prop<Color>,
    pub title_font: Font,
    /// The back button; None: the tint.
    pub back_color: Option<Prop<Color>>,
    /// Under the bar.
    pub shows_divider: bool,
}

impl Default for BarNavigationStackStyle {
    fn default() -> Self {
        Self {
            height: 44.0,
            background: Prop::Value(Color::CLEAR),
            title_font: Font::headline(),
            back_color: None,
            shows_divider: true,
        }
    }
}

#[elegance_macros::elg_builder]
impl BarNavigationStackStyle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn height(mut self, height: impl Number) -> Self {
        self.height = height.to_f32();
        self
    }
    pub fn background(mut self, color: impl IntoProp<Color>) -> Self {
        self.background = color.into_prop();
        self
    }
    pub fn title_font(mut self, font: impl Into<Font>) -> Self {
        self.title_font = font.into();
        self
    }
    pub fn back_color(mut self, color: impl IntoProp<Color>) -> Self {
        self.back_color = Some(color.into_prop());
        self
    }
    pub fn shows_divider(mut self, shows: bool) -> Self {
        self.shows_divider = shows;
        self
    }
}

impl IntoNavigationStackStyle for BarNavigationStackStyle {
    fn into_navigation_stack_style(self) -> AnyNavigationStackStyle {
        let (background, back_color) = (OwnedProp::color(self.background), owned_color(self.back_color));
        let options = sys::ElgBarNavigationStackStyleOptions {
            height: self.height,
            background: background.c,
            title_font: self.title_font.c(),
            back_color: OwnedProp::ptr(&back_color),
            shows_divider: self.shows_divider,
        };
        AnyNavigationStackStyle::from_raw(ffi!(sys::elg_navigation_stack_style_builtin(&options)))
    }
}

// --- Popovers, tooltips, menus ---

configuration!(
    /// What a PopoverStyle wraps in a panel: the popover's content.
    PopoverConfiguration, elg_popover_config_clone, elg_popover_config_free
);

impl PopoverConfiguration {
    pub fn content(&self) -> View {
        view_of(unsafe { sys::elg_popover_config_content(self.as_ptr()) })
    }
    /// `PopoverOptions::padded`: false lets the content reach the edges. A tracked read.
    pub fn is_padded(&self) -> bool {
        ffi!(sys::elg_popover_config_is_padded(self.as_ptr()))
    }
}

pub trait PopoverStyle: 'static {
    /// Called each time it opens.
    fn make_body(&self, popover: PopoverConfiguration) -> View;
    /// How it opens and closes; at once by default.
    fn motion(&self) -> PopupMotion {
        PopupMotion::none()
    }
}

family!(
    AnyPopoverStyle: PopoverStyle, elg_popover_style_clone, elg_popover_style_free, elg_mod_popover_style,
    IntoPopoverStyle::into_popover_style
);

impl AnyPopoverStyle {
    pub fn make_body(&self, popover: &PopoverConfiguration) -> View {
        view_of(ffi!(sys::elg_popover_style_make_body(self.as_ptr(), popover.as_ptr())))
    }
}

impl<S: PopoverStyle> IntoPopoverStyle for S {
    fn into_popover_style(self) -> AnyPopoverStyle {
        let motion = self.motion();
        let make_body = cffi::closure(move |args, ret| unsafe {
            self.make_body(PopoverConfiguration::from_raw(args as *mut sys::PopoverConfiguration)).write_into(ret);
        });
        AnyPopoverStyle::from_raw(ffi!(sys::elg_popover_style_rust(make_body, c_motion(Some(&motion)))))
    }
}

builtin!(
    /// A rounded surface with an outline and a shadow; padded content is inset 12 points. The
    /// default.
    DefaultPopoverStyle => IntoPopoverStyle::into_popover_style -> AnyPopoverStyle, sys::elg_popover_style_builtin(0)
);
builtin!(
    /// Just the content: no panel, padding or shadow.
    PlainPopoverStyle => IntoPopoverStyle::into_popover_style -> AnyPopoverStyle, sys::elg_popover_style_builtin(1)
);

configuration!(
    /// What a TooltipStyle wraps: the tooltip's content (`.tooltip(text)`: the text).
    TooltipConfiguration, elg_tooltip_config_clone, elg_tooltip_config_free
);

impl TooltipConfiguration {
    pub fn content(&self) -> View {
        view_of(unsafe { sys::elg_tooltip_config_content(self.as_ptr()) })
    }
}

pub trait TooltipStyle: 'static {
    /// Called each time it opens.
    fn make_body(&self, tooltip: TooltipConfiguration) -> View;
    /// How it opens and closes; at once by default.
    fn motion(&self) -> PopupMotion {
        PopupMotion::none()
    }
}

family!(
    AnyTooltipStyle: TooltipStyle, elg_tooltip_style_clone, elg_tooltip_style_free, elg_mod_tooltip_style,
    IntoTooltipStyle::into_tooltip_style
);

impl AnyTooltipStyle {
    pub fn make_body(&self, tooltip: &TooltipConfiguration) -> View {
        view_of(ffi!(sys::elg_tooltip_style_make_body(self.as_ptr(), tooltip.as_ptr())))
    }
}

impl<S: TooltipStyle> IntoTooltipStyle for S {
    fn into_tooltip_style(self) -> AnyTooltipStyle {
        let motion = self.motion();
        let make_body = cffi::closure(move |args, ret| unsafe {
            self.make_body(TooltipConfiguration::from_raw(args as *mut sys::TooltipConfiguration)).write_into(ret);
        });
        AnyTooltipStyle::from_raw(ffi!(sys::elg_tooltip_style_rust(make_body, c_motion(Some(&motion)))))
    }
}

builtin!(
    /// Small text on a rounded surface with an outline and a shadow. The default.
    DefaultTooltipStyle => IntoTooltipStyle::into_tooltip_style -> AnyTooltipStyle, sys::elg_tooltip_style_builtin(0)
);
builtin!(
    /// Just the content.
    PlainTooltipStyle => IntoTooltipStyle::into_tooltip_style -> AnyTooltipStyle, sys::elg_tooltip_style_builtin(1)
);

configuration!(
    /// What a MenuStyle wraps in a panel: a Menu's items, stacked (rows span the panel's width).
    MenuConfiguration, elg_menu_config_clone, elg_menu_config_free
);

impl MenuConfiguration {
    pub fn items(&self) -> View {
        view_of(unsafe { sys::elg_menu_config_items(self.as_ptr()) })
    }
}

configuration!(
    /// What a MenuStyle gets for one row (a MenuItem or a Submenu): its parts, laid out by the
    /// style, and its state. The row itself handles clicks, keys and opening submenus.
    MenuItemConfiguration, elg_menu_item_config_clone, elg_menu_item_config_free
);

impl MenuItemConfiguration {
    pub fn title(&self) -> View {
        view_of(unsafe { sys::elg_menu_item_config_view(self.as_ptr(), 0) })
    }
    /// The check mark and icon; empty (`View::is_empty`) when the item has neither.
    pub fn leading(&self) -> View {
        view_of(unsafe { sys::elg_menu_item_config_view(self.as_ptr(), 1) })
    }
    /// The shortcut hint, or a submenu's chevron; empty when there's neither.
    pub fn trailing(&self) -> View {
        view_of(unsafe { sys::elg_menu_item_config_view(self.as_ptr(), 2) })
    }
    states!(elg_menu_item_config_state:
        /// Hovered, or focused with the keyboard.
        is_highlighted = 0,
        is_enabled = 1,
        /// `MenuItem::destructive`.
        is_destructive = 2,
    );
}

pub trait MenuStyle: 'static {
    /// Called each time it opens.
    fn make_body(&self, menu: MenuConfiguration) -> View;
    /// A row: by default the parts in a line, padded, in the danger color if destructive, dimmed
    /// when disabled, on a rounded highlight.
    fn make_item(&self, item: MenuItemConfiguration) -> View {
        DefaultMenuStyle.into_menu_style().make_item(&item)
    }
    /// How it opens and closes; at once by default.
    fn motion(&self) -> PopupMotion {
        PopupMotion::none()
    }
}

family!(
    AnyMenuStyle: MenuStyle, elg_menu_style_clone, elg_menu_style_free, elg_mod_menu_style,
    IntoMenuStyle::into_menu_style
);

impl AnyMenuStyle {
    pub fn make_body(&self, menu: &MenuConfiguration) -> View {
        view_of(ffi!(sys::elg_menu_style_make_body(self.as_ptr(), menu.as_ptr())))
    }
    pub fn make_item(&self, item: &MenuItemConfiguration) -> View {
        view_of(ffi!(sys::elg_menu_style_make_item(self.as_ptr(), item.as_ptr())))
    }
}

impl<S: MenuStyle> IntoMenuStyle for S {
    fn into_menu_style(self) -> AnyMenuStyle {
        let motion = self.motion();
        let style = std::rc::Rc::new(self);
        let body_style = style.clone();
        let make_body = cffi::closure(move |args, ret| unsafe {
            body_style.make_body(MenuConfiguration::from_raw(args as *mut sys::MenuConfiguration)).write_into(ret);
        });
        let make_item = cffi::closure(move |args, ret| unsafe {
            style.make_item(MenuItemConfiguration::from_raw(args as *mut sys::MenuItemConfiguration)).write_into(ret);
        });
        AnyMenuStyle::from_raw(ffi!(sys::elg_menu_style_rust(make_body, make_item, c_motion(Some(&motion)))))
    }
}

builtin!(
    /// A surface with an outline and a shadow, at least 200 points wide. The default.
    DefaultMenuStyle => IntoMenuStyle::into_menu_style -> AnyMenuStyle, sys::elg_menu_style_builtin(0)
);
builtin!(
    /// Just the items.
    PlainMenuStyle => IntoMenuStyle::into_menu_style -> AnyMenuStyle, sys::elg_menu_style_builtin(1)
);

// --- Selectable rows ---

configuration!(
    /// What a RowStyle gets for one selectable row (`.selection()`): the ForEach item's view, and
    /// its state. The row itself handles clicks, keys and accessibility.
    RowConfiguration, elg_row_config_clone, elg_row_config_free
);

impl RowConfiguration {
    pub fn content(&self) -> View {
        view_of(unsafe { sys::elg_row_config_content(self.as_ptr()) })
    }
    /// The stack's: rows usually span across it.
    pub fn axis(&self) -> Axis {
        if unsafe { sys::elg_row_config_axis(self.as_ptr()) } == Axis::Vertical as i32 {
            Axis::Vertical
        } else {
            Axis::Horizontal
        }
    }
    states!(elg_row_config_state:
        is_selected = 0,
        is_hovered = 1,
        /// The row keys act on, while the stack has keyboard focus.
        is_focused = 2,
        is_enabled = 3,
    );
}

pub trait RowStyle: 'static {
    fn make_body(&self, row: RowConfiguration) -> View;
}

family!(
    AnyRowStyle: RowStyle, elg_row_style_clone, elg_row_style_free, elg_mod_row_style, IntoRowStyle::into_row_style
);

impl AnyRowStyle {
    pub fn make_body(&self, row: &RowConfiguration) -> View {
        view_of(ffi!(sys::elg_row_style_make_body(self.as_ptr(), row.as_ptr())))
    }
}

rust_style!(RowStyle, RowConfiguration, IntoRowStyle::into_row_style -> AnyRowStyle, elg_row_style_rust);
builtin!(
    /// Just the content, for rows that show their state themselves (`RowSelectedKey`).
    PlainRowStyle => IntoRowStyle::into_row_style -> AnyRowStyle, sys::elg_row_style_builtin(std::ptr::null())
);

/// The content, padded, on a rounded highlight: the tint while selected (with the text and icons
/// in on_accent), faint while hovered, outlined in the tint while focused. The default.
#[derive(Clone)]
pub struct HighlightRowStyle {
    /// The selected rows' highlight; None: the tint.
    pub selected: Option<Prop<Color>>,
    /// Their text and icons.
    pub on_selected: Prop<Color>,
    /// The hovered row's highlight, at half opacity.
    pub hovered: Prop<Color>,
    pub corner_radius: f32,
    /// Around the content, inside the highlight.
    pub padding: EdgeInsets,
}

impl Default for HighlightRowStyle {
    fn default() -> Self {
        use crate::color::ThemeColor;
        Self {
            selected: None,
            on_selected: Prop::Theme(ThemeColor::OnAccent),
            hovered: Prop::Theme(ThemeColor::Outline),
            corner_radius: 6.0,
            padding: EdgeInsets::symmetric(8.0, 4.0),
        }
    }
}

#[elegance_macros::elg_builder]
impl HighlightRowStyle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn selected(mut self, color: impl IntoProp<Color>) -> Self {
        self.selected = Some(color.into_prop());
        self
    }
    pub fn on_selected(mut self, color: impl IntoProp<Color>) -> Self {
        self.on_selected = color.into_prop();
        self
    }
    pub fn hovered(mut self, color: impl IntoProp<Color>) -> Self {
        self.hovered = color.into_prop();
        self
    }
    pub fn corner_radius(mut self, radius: impl Number) -> Self {
        self.corner_radius = radius.to_f32();
        self
    }
    pub fn padding(mut self, padding: impl Into<EdgeInsets>) -> Self {
        self.padding = padding.into();
        self
    }
}

impl IntoRowStyle for HighlightRowStyle {
    fn into_row_style(self) -> AnyRowStyle {
        let selected = owned_color(self.selected);
        let (on_selected, hovered) = (OwnedProp::color(self.on_selected), OwnedProp::color(self.hovered));
        let options = sys::ElgHighlightRowStyleOptions {
            selected: OwnedProp::ptr(&selected),
            on_selected: on_selected.c,
            hovered: hovered.c,
            corner_radius: self.corner_radius,
            padding: self.padding.c(),
        };
        AnyRowStyle::from_raw(ffi!(sys::elg_row_style_builtin(&options)))
    }
}
