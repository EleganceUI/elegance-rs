//! Reactive arguments: a plain value, or anything that produces one.
//!
//! ```ignore
//! .opacity(0.5)                                         // static
//! .opacity(fade)                                        // a State<f32>
//! .opacity(move || if hovered.get() { 1.0 } else { 0.6 }) // derived
//! .foreground_style(.accent)                            // resolved against the environment
//! ```
//!
//! A reactive argument becomes one effect that updates one property of one element, so only that
//! property changes when the state it reads does.

use std::any::Any;
use std::ffi::c_void;
use std::rc::Rc;

use elegence_sys as sys;

use crate::color::{
    AngularGradient, Color, ColorScheme, EllipticalGradient, LinearGradient, Material, RadialGradient, ShapeStyle, Theme,
    ThemeColor,
};
use crate::dot;
use crate::environment::EnvironmentValues;
use crate::events::HoverDelay;
use crate::ffi;
use crate::font::Font;
use crate::geometry::Size;
use crate::reactive::{Binding, Computed, State};

/// A reactive argument (`elg::Prop<T>`).
pub enum Prop<T: 'static> {
    Value(T),
    Fn(Rc<dyn Fn() -> T>),
    /// Needs the environment where the prop is used (theme colors, ...).
    Env(Rc<dyn Fn(&EnvironmentValues) -> T>),
    /// A theme color (for `Prop<Color>` and `Prop<ShapeStyle>`).
    Theme(ThemeColor),
}

impl<T: Clone + 'static> Clone for Prop<T> {
    fn clone(&self) -> Self {
        match self {
            Prop::Value(v) => Prop::Value(v.clone()),
            Prop::Fn(f) => Prop::Fn(f.clone()),
            Prop::Env(f) => Prop::Env(f.clone()),
            Prop::Theme(t) => Prop::Theme(*t),
        }
    }
}

impl<T: 'static> Prop<T> {
    pub fn from_fn(f: impl Fn() -> T + 'static) -> Self {
        Prop::Fn(Rc::new(f))
    }
    pub fn from_env(f: impl Fn(&EnvironmentValues) -> T + 'static) -> Self {
        Prop::Env(Rc::new(f))
    }
    pub fn is_reactive(&self) -> bool {
        !matches!(self, Prop::Value(_))
    }
    /// Whether the value depends on the environment it's used in (e.g. a theme color).
    pub fn needs_environment(&self) -> bool {
        matches!(self, Prop::Env(_) | Prop::Theme(_))
    }
    /// The value if it's static.
    pub fn static_value(&self) -> Option<&T> {
        match self {
            Prop::Value(v) => Some(v),
            _ => None,
        }
    }
    /// The current value (a tracked read for reactive props), resolved in `environment`.
    pub fn get(&self, environment: &EnvironmentValues) -> T
    where
        T: Clone,
    {
        match self {
            Prop::Value(v) => v.clone(),
            Prop::Fn(f) => f(),
            Prop::Env(f) => f(environment),
            Prop::Theme(token) => {
                let color = environment.color(*token);
                let any: Box<dyn Any> = Box::new(color);
                match any.downcast::<T>() {
                    Ok(value) => *value,
                    Err(_) => {
                        let style: Box<dyn Any> = Box::new(ShapeStyle::Color(color));
                        *style.downcast::<T>().unwrap_or_else(|_| unreachable!("a theme color prop of a non-color type"))
                    }
                }
            }
        }
    }
}

/// Anything usable as a reactive argument of type `T`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` can't be used as a reactive `{T}` argument",
    label = "expected a `{T}`, a State/Binding/Computed of one, or a closure returning one"
)]
pub trait IntoProp<T: 'static> {
    fn into_prop(self) -> Prop<T>;
}

impl<T: 'static> IntoProp<T> for T {
    fn into_prop(self) -> Prop<T> {
        Prop::Value(self)
    }
}

impl<T: 'static> IntoProp<T> for Prop<T> {
    fn into_prop(self) -> Prop<T> {
        self
    }
}

impl<T: Clone + 'static> IntoProp<T> for State<T> {
    fn into_prop(self) -> Prop<T> {
        Prop::from_fn(move || self.get())
    }
}

impl<T: Clone + 'static> IntoProp<T> for Binding<T> {
    fn into_prop(self) -> Prop<T> {
        Prop::from_fn(move || self.get())
    }
}

impl<T: Clone + 'static> IntoProp<T> for Computed<T> {
    fn into_prop(self) -> Prop<T> {
        Prop::from_fn(move || self.get())
    }
}

macro_rules! number_props {
    ($($t:ty),*) => {$(
        impl IntoProp<f32> for $t {
            fn into_prop(self) -> Prop<f32> {
                Prop::Value(self as f32)
            }
        }
    )*};
}
number_props!(f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl IntoProp<String> for &str {
    fn into_prop(self) -> Prop<String> {
        Prop::Value(self.to_owned())
    }
}

impl IntoProp<String> for &String {
    fn into_prop(self) -> Prop<String> {
        Prop::Value(self.clone())
    }
}

impl IntoProp<String> for std::borrow::Cow<'_, str> {
    fn into_prop(self) -> Prop<String> {
        Prop::Value(self.into_owned())
    }
}

// Closures returning exactly the type, so literals in them infer it (`|| 0.5` is an f32 here).
macro_rules! closure_props {
    ($($t:ty),*) => {$(
        impl<F: Fn() -> $t + 'static> IntoProp<$t> for F {
            fn into_prop(self) -> Prop<$t> {
                Prop::from_fn(self)
            }
        }
    )*};
}
closure_props!(f32, bool, Size, Font, ColorScheme, Theme, HoverDelay);

impl<F: Fn() -> R + 'static, R: Into<String>> IntoProp<String> for F {
    fn into_prop(self) -> Prop<String> {
        Prop::from_fn(move || self().into())
    }
}

/// A value that becomes a color in an environment: a [`Color`], a [`ThemeColor`], or a dot
/// marker such as `.accent`.
pub trait ResolveColor: 'static {
    fn resolve(self, environment: &EnvironmentValues) -> Color;
}

impl ResolveColor for Color {
    fn resolve(self, _: &EnvironmentValues) -> Color {
        self
    }
}

impl ResolveColor for ThemeColor {
    fn resolve(self, environment: &EnvironmentValues) -> Color {
        environment.color(self)
    }
}

/// A value that becomes a shape style in an environment.
pub trait ResolveStyle: 'static {
    fn resolve_style(self, environment: &EnvironmentValues) -> ShapeStyle;
}

impl ResolveStyle for ShapeStyle {
    fn resolve_style(self, _: &EnvironmentValues) -> ShapeStyle {
        self
    }
}
impl ResolveStyle for Color {
    fn resolve_style(self, _: &EnvironmentValues) -> ShapeStyle {
        ShapeStyle::Color(self)
    }
}
impl ResolveStyle for ThemeColor {
    fn resolve_style(self, environment: &EnvironmentValues) -> ShapeStyle {
        ShapeStyle::Color(environment.color(self))
    }
}
impl ResolveStyle for LinearGradient {
    fn resolve_style(self, _: &EnvironmentValues) -> ShapeStyle {
        ShapeStyle::Linear(self)
    }
}
impl ResolveStyle for RadialGradient {
    fn resolve_style(self, _: &EnvironmentValues) -> ShapeStyle {
        ShapeStyle::Radial(self)
    }
}
impl ResolveStyle for EllipticalGradient {
    fn resolve_style(self, _: &EnvironmentValues) -> ShapeStyle {
        ShapeStyle::Elliptical(self)
    }
}
impl ResolveStyle for AngularGradient {
    fn resolve_style(self, _: &EnvironmentValues) -> ShapeStyle {
        ShapeStyle::Angular(self)
    }
}
impl ResolveStyle for Material {
    fn resolve_style(self, _: &EnvironmentValues) -> ShapeStyle {
        ShapeStyle::Material(self)
    }
}

// Closures may return theme colors, so color and style closures resolve in the environment.
impl<F: Fn() -> R + 'static, R: ResolveColor> IntoProp<Color> for F {
    fn into_prop(self) -> Prop<Color> {
        Prop::from_env(move |environment| self().resolve(environment))
    }
}

impl<F: Fn() -> R + 'static, R: ResolveStyle> IntoProp<ShapeStyle> for F {
    fn into_prop(self) -> Prop<ShapeStyle> {
        Prop::from_env(move |environment| self().resolve_style(environment))
    }
}

impl IntoProp<Color> for ThemeColor {
    fn into_prop(self) -> Prop<Color> {
        Prop::Theme(self)
    }
}

impl IntoProp<ShapeStyle> for ThemeColor {
    fn into_prop(self) -> Prop<ShapeStyle> {
        Prop::Theme(self)
    }
}

impl IntoProp<ShapeStyle> for Color {
    fn into_prop(self) -> Prop<ShapeStyle> {
        Prop::Value(ShapeStyle::Color(self))
    }
}

impl IntoProp<ShapeStyle> for LinearGradient {
    fn into_prop(self) -> Prop<ShapeStyle> {
        Prop::Value(ShapeStyle::Linear(self))
    }
}

impl IntoProp<ShapeStyle> for RadialGradient {
    fn into_prop(self) -> Prop<ShapeStyle> {
        Prop::Value(ShapeStyle::Radial(self))
    }
}

impl IntoProp<ShapeStyle> for EllipticalGradient {
    fn into_prop(self) -> Prop<ShapeStyle> {
        Prop::Value(ShapeStyle::Elliptical(self))
    }
}

impl IntoProp<ShapeStyle> for AngularGradient {
    fn into_prop(self) -> Prop<ShapeStyle> {
        Prop::Value(ShapeStyle::Angular(self))
    }
}

impl IntoProp<ShapeStyle> for Material {
    fn into_prop(self) -> Prop<ShapeStyle> {
        Prop::Value(ShapeStyle::Material(self))
    }
}

impl IntoProp<ShapeStyle> for State<Color> {
    fn into_prop(self) -> Prop<ShapeStyle> {
        Prop::from_fn(move || ShapeStyle::Color(self.get()))
    }
}

macro_rules! theme_markers {
    ($($marker:ident),*) => {$(
        impl IntoProp<Color> for dot::$marker {
            fn into_prop(self) -> Prop<Color> {
                Prop::Theme(self.into())
            }
        }
        impl IntoProp<ShapeStyle> for dot::$marker {
            fn into_prop(self) -> Prop<ShapeStyle> {
                Prop::Theme(self.into())
            }
        }
        impl ResolveColor for dot::$marker {
            fn resolve(self, environment: &EnvironmentValues) -> Color {
                environment.color(ThemeColor::from(self))
            }
        }
        impl ResolveStyle for dot::$marker {
            fn resolve_style(self, environment: &EnvironmentValues) -> ShapeStyle {
                ShapeStyle::Color(environment.color(ThemeColor::from(self)))
            }
        }
    )*};
}
theme_markers!(
    background, surface, surface_raised, outline, primary, secondary, tertiary, accent, on_accent, danger, success,
    warning
);

macro_rules! color_markers {
    ($($marker:ident),*) => {$(
        impl IntoProp<Color> for dot::$marker {
            fn into_prop(self) -> Prop<Color> {
                Prop::Value(self.into())
            }
        }
        impl IntoProp<ShapeStyle> for dot::$marker {
            fn into_prop(self) -> Prop<ShapeStyle> {
                Prop::Value(ShapeStyle::Color(self.into()))
            }
        }
        impl ResolveColor for dot::$marker {
            fn resolve(self, _: &EnvironmentValues) -> Color {
                self.into()
            }
        }
    )*};
}
color_markers!(black, white, clear);

macro_rules! material_markers {
    ($($marker:ident),*) => {$(
        impl IntoProp<ShapeStyle> for dot::$marker {
            fn into_prop(self) -> Prop<ShapeStyle> {
                Prop::Value(ShapeStyle::Material(self.into()))
            }
        }
        impl ResolveStyle for dot::$marker {
            fn resolve_style(self, _: &EnvironmentValues) -> ShapeStyle {
                ShapeStyle::Material(self.into())
            }
        }
    )*};
}
material_markers!(ultra_thin_material, thin_material, regular_material, thick_material, ultra_thick_material);

macro_rules! value_markers {
    ($t:ty: $($marker:ident),*) => {$(
        impl IntoProp<$t> for dot::$marker {
            fn into_prop(self) -> Prop<$t> {
                Prop::Value(self.into())
            }
        }
    )*};
}
value_markers!(Font: large_title, title, title2, title3, headline, body, callout, subheadline, footnote, caption, caption2);
value_markers!(ColorScheme: light, dark);

// --- To C ---

/// Prop value types the shim knows.
pub(crate) trait PropValue: Clone + 'static {
    /// Calls `f` with a pointer to the C form.
    fn with_c<R>(&self, f: impl FnOnce(*const c_void) -> R) -> R;
    /// Writes into a C++ `T` (a reactive closure's result).
    unsafe fn write(&self, ret: *mut c_void);
}

macro_rules! pod_value {
    ($t:ty, $c:ty, |$v:ident| $to_c:expr) => {
        impl PropValue for $t {
            fn with_c<R>(&self, f: impl FnOnce(*const c_void) -> R) -> R {
                let $v = self;
                let c: $c = $to_c;
                f(&c as *const $c as *const c_void)
            }
            unsafe fn write(&self, ret: *mut c_void) {
                let $v = self;
                *(ret as *mut $c) = $to_c;
            }
        }
    };
}

pod_value!(f32, f32, |v| *v);
pod_value!(bool, bool, |v| *v);
pod_value!(Color, sys::ElgColor, |v| v.c());
pod_value!(Size, sys::ElgSize, |v| v.c());
pod_value!(ColorScheme, i32, |v| *v as i32);
pod_value!(Theme, sys::ElgTheme, |v| v.c());
pod_value!(HoverDelay, sys::ElgHoverDelay, |v| v.c());

impl PropValue for String {
    fn with_c<R>(&self, f: impl FnOnce(*const c_void) -> R) -> R {
        let c = ffi::s(self);
        f(&c as *const sys::ElgStr as *const c_void)
    }
    unsafe fn write(&self, ret: *mut c_void) {
        ffi::write_string(ret, self);
    }
}

impl PropValue for Font {
    fn with_c<R>(&self, f: impl FnOnce(*const c_void) -> R) -> R {
        let c = self.c();
        f(&c as *const sys::ElgFont as *const c_void)
    }
    unsafe fn write(&self, ret: *mut c_void) {
        let c = self.c();
        sys::elg_font_assign(ret as *mut sys::Font, &c);
    }
}

impl PropValue for ShapeStyle {
    fn with_c<R>(&self, f: impl FnOnce(*const c_void) -> R) -> R {
        self.with_c(|c| f(c as *const sys::ElgShapeStyle as *const c_void))
    }
    unsafe fn write(&self, ret: *mut c_void) {
        ShapeStyle::with_c(self, |c| sys::elg_shape_style_assign(ret as *mut sys::ShapeStyle, c));
    }
}

/// Calls `f` with the C form of `prop` (static values stay valid for the call).
pub(crate) fn with_prop<T: PropValue, R>(prop: Prop<T>, f: impl FnOnce(sys::ElgProp) -> R) -> R {
    match prop {
        Prop::Value(value) => {
            value.with_c(|ptr| f(sys::ElgProp { kind: 0, theme: 0, value: ptr, fn_: sys::ElgClosure::NONE }))
        }
        Prop::Fn(function) => f(sys::ElgProp {
            kind: 1,
            theme: 0,
            value: std::ptr::null(),
            fn_: ffi::closure(move |_, ret| unsafe { function().write(ret) }),
        }),
        Prop::Env(function) => f(sys::ElgProp {
            kind: 2,
            theme: 0,
            value: std::ptr::null(),
            fn_: ffi::closure(move |args, ret| {
                let environment = unsafe { EnvironmentValues::borrowed(args) };
                unsafe { function(&environment).write(ret) }
            }),
        }),
        Prop::Theme(token) => f(sys::ElgProp {
            kind: 3,
            theme: token as i32,
            value: std::ptr::null(),
            fn_: sys::ElgClosure::NONE,
        }),
    }
}
