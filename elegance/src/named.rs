// Keys shared with C++ (and other languages) by name: environment keys and layout values whose
// value is a bool, i32, i64, f32, f64, Color or String, kept by Elegance under the name. A C++ key
// and a Rust key match when both name and value type do. Other value types return None ("not a
// named type"), and their keys stay in Rust's own storage.

use std::any::{Any, TypeId};

use elegance_sys as sys;

use crate::color::Color;
use crate::environment::EnvironmentValues;
use crate::error::ffi;
use crate::ffi as cffi;
use crate::prop::{with_prop, Prop};
use crate::view::View;

fn boxed<T: 'static>(value: impl Any) -> Option<T> {
    (Box::new(value) as Box<dyn Any>).downcast::<T>().ok().map(|value| *value)
}

macro_rules! pod_types {
    ($m:ident) => {
        $m!(bool, false, elg_env_get_named_bool, elg_env_set_named_bool, elg_mod_layout_value_bool, elg_layout_child_value_bool);
        $m!(i32, 0, elg_env_get_named_i32, elg_env_set_named_i32, elg_mod_layout_value_i32, elg_layout_child_value_i32);
        $m!(i64, 0, elg_env_get_named_i64, elg_env_set_named_i64, elg_mod_layout_value_i64, elg_layout_child_value_i64);
        $m!(f32, 0.0, elg_env_get_named_f32, elg_env_set_named_f32, elg_mod_layout_value_f32, elg_layout_child_value_f32);
        $m!(f64, 0.0, elg_env_get_named_f64, elg_env_set_named_f64, elg_mod_layout_value_f64, elg_layout_child_value_f64);
    };
}

/// `name`'s value in `environment`: `Some(None)` when it isn't set there; `None` when `T` isn't a
/// named type.
pub(crate) fn env_get<T: 'static>(environment: &EnvironmentValues, name: &str) -> Option<Option<T>> {
    let (env, name) = (environment.as_ptr(), cffi::s(name));
    let t = TypeId::of::<T>();
    macro_rules! get {
        ($t:ty, $zero:expr, $get:ident, $set:ident, $mod:ident, $child:ident) => {
            if t == TypeId::of::<$t>() {
                let mut value: $t = $zero;
                let found = ffi!(sys::$get(env, name, &mut value));
                return Some(found.then(|| boxed::<T>(value)).flatten());
            }
        };
    }
    pod_types!(get);
    if t == TypeId::of::<Color>() {
        let mut value = sys::ElgColor::default();
        let found = ffi!(sys::elg_env_get_named_color(env, name, &mut value));
        return Some(found.then(|| boxed::<T>(Color::from_c(value))).flatten());
    }
    if t == TypeId::of::<String>() {
        let mut found = false;
        let value = cffi::read_string(|sink, ctx| found = ffi!(sys::elg_env_get_named_string(env, name, sink, ctx)));
        return Some(found.then(|| boxed::<T>(value)).flatten());
    }
    None
}

/// Sets `name` in `environment`, or gives `value` back when `T` isn't a named type.
pub(crate) fn env_set<T: 'static>(environment: &mut EnvironmentValues, name: &str, value: Prop<T>) -> Result<(), Prop<T>> {
    let (env, name) = (environment.as_ptr(), cffi::s(name));
    let mut value: Box<dyn Any> = Box::new(value);
    macro_rules! set {
        ($t:ty, $zero:expr, $get:ident, $set:ident, $mod:ident, $child:ident) => {
            value = match value.downcast::<Prop<$t>>() {
                Ok(prop) => {
                    with_prop(*prop, |p| ffi!(sys::$set(env, name, p)));
                    return Ok(());
                }
                Err(value) => value,
            };
        };
    }
    pod_types!(set);
    value = match value.downcast::<Prop<Color>>() {
        Ok(prop) => {
            with_prop(*prop, |p| ffi!(sys::elg_env_set_named_color(env, name, p)));
            return Ok(());
        }
        Err(value) => value,
    };
    value = match value.downcast::<Prop<String>>() {
        Ok(prop) => {
            with_prop(*prop, |p| ffi!(sys::elg_env_set_named_string(env, name, p)));
            return Ok(());
        }
        Err(value) => value,
    };
    Err(*value.downcast::<Prop<T>>().unwrap_or_else(|_| unreachable!("a prop of another type")))
}

/// `view` with `value` under `name` for its parent's layout, or `value` back when `T` isn't a
/// named type.
pub(crate) fn layout_set<T: 'static>(view: View, name: &str, value: T) -> Result<View, (View, T)> {
    let name_c = cffi::s(name);
    let mut value: Box<dyn Any> = Box::new(value);
    macro_rules! set {
        ($t:ty, $zero:expr, $get:ident, $set:ident, $mod:ident, $child:ident) => {
            value = match value.downcast::<$t>() {
                Ok(v) => return Ok(view.modify(|raw| unsafe { sys::$mod(raw, name_c, *v) })),
                Err(value) => value,
            };
        };
    }
    pod_types!(set);
    value = match value.downcast::<Color>() {
        Ok(v) => return Ok(view.modify(|raw| unsafe { sys::elg_mod_layout_value_color(raw, name_c, v.c()) })),
        Err(value) => value,
    };
    value = match value.downcast::<String>() {
        Ok(v) => return Ok(view.modify(|raw| unsafe { sys::elg_mod_layout_value_string(raw, name_c, cffi::s(&v)) })),
        Err(value) => value,
    };
    Err((view, *value.downcast::<T>().unwrap_or_else(|_| unreachable!("a value of another type"))))
}

/// The value under `name` set on `child`: `Some(None)` when there's none; `None` when `T` isn't a
/// named type.
pub(crate) fn layout_get<T: 'static>(child: *const sys::LayoutChild, name: &str) -> Option<Option<T>> {
    let name = cffi::s(name);
    let t = TypeId::of::<T>();
    macro_rules! get {
        ($t:ty, $zero:expr, $get:ident, $set:ident, $mod:ident, $child:ident) => {
            if t == TypeId::of::<$t>() {
                let mut value: $t = $zero;
                let found = ffi!(sys::$child(child, name, &mut value));
                return Some(found.then(|| boxed::<T>(value)).flatten());
            }
        };
    }
    pod_types!(get);
    if t == TypeId::of::<Color>() {
        let mut value = sys::ElgColor::default();
        let found = ffi!(sys::elg_layout_child_value_color(child, name, &mut value));
        return Some(found.then(|| boxed::<T>(Color::from_c(value))).flatten());
    }
    if t == TypeId::of::<String>() {
        let mut found = false;
        let value = cffi::read_string(|sink, ctx| found = ffi!(sys::elg_layout_child_value_string(child, name, sink, ctx)));
        return Some(found.then(|| boxed::<T>(value)).flatten());
    }
    None
}
