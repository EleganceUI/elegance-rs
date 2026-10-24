//! The window a view is in, and window controls for custom title bars.

use std::ptr::NonNull;

use elegance_sys as sys;

use crate::dsl::ElgNew;
use crate::error::ffi;
use crate::view::{IntoView, View};

/// The window a view is in (`Environment::<WindowKey>`): close, minimize, maximize it.
/// Methods do nothing on the empty handle.
pub struct WindowHandle {
    ptr: Option<NonNull<sys::WindowHandle>>,
}

impl Clone for WindowHandle {
    fn clone(&self) -> Self {
        match self.ptr {
            Some(ptr) => Self::from_raw(unsafe { sys::elg_window_clone(ptr.as_ptr()) }),
            None => Self::none(),
        }
    }
}

impl Drop for WindowHandle {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr {
            unsafe { sys::elg_window_free(ptr.as_ptr()) }
        }
    }
}

impl PartialEq for WindowHandle {
    fn eq(&self, other: &Self) -> bool {
        match (self.ptr, other.ptr) {
            (Some(a), Some(b)) => unsafe { sys::elg_window_eq(a.as_ptr(), b.as_ptr()) },
            (None, None) => true,
            _ => false,
        }
    }
}

impl WindowHandle {
    pub(crate) fn none() -> Self {
        Self { ptr: None }
    }
    pub(crate) fn from_raw(ptr: *mut sys::WindowHandle) -> Self {
        Self { ptr: NonNull::new(ptr) }
    }
    pub(crate) fn as_ptr(&self) -> *const sys::WindowHandle {
        self.ptr.map_or(std::ptr::null(), |p| p.as_ptr())
    }

    pub fn close(&self) {
        if self.ptr.is_some() {
            ffi!(sys::elg_window_close(self.as_ptr()));
        }
    }
    pub fn minimize(&self) {
        if self.ptr.is_some() {
            ffi!(sys::elg_window_minimize(self.as_ptr()));
        }
    }
    pub fn toggle_maximized(&self) {
        if self.ptr.is_some() {
            ffi!(sys::elg_window_toggle_maximized(self.as_ptr()));
        }
    }
    pub fn is_maximized(&self) -> bool {
        self.ptr.is_some() && ffi!(sys::elg_window_is_maximized(self.as_ptr()))
    }
    pub fn is_active(&self) -> bool {
        self.ptr.is_some() && ffi!(sys::elg_window_is_active(self.as_ptr()))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowControlsStyle {
    /// The platform's: caption buttons on Windows and Linux, traffic lights on macOS.
    #[default]
    Automatic,
    /// Minimize / maximize / close buttons, as on Windows.
    Caption,
    /// Three colored circles, as on macOS.
    TrafficLights,
}

/// Minimize, maximize, and close buttons for a window with `TitleBar::Hidden`.
pub struct WindowControls {
    style: WindowControlsStyle,
    minimize: bool,
    maximize: bool,
}

impl Default for WindowControls {
    fn default() -> Self {
        Self { style: WindowControlsStyle::Automatic, minimize: true, maximize: true }
    }
}

#[elegance_macros::elg_builder]
impl WindowControls {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn style(mut self, style: impl Into<WindowControlsStyle>) -> Self {
        self.style = style.into();
        self
    }
    pub fn minimize(mut self, shown: bool) -> Self {
        self.minimize = shown;
        self
    }
    pub fn maximize(mut self, shown: bool) -> Self {
        self.maximize = shown;
        self
    }
}

impl ElgNew<()> for WindowControls {
    type Out = WindowControls;
    fn elg_new(_: ()) -> WindowControls {
        WindowControls::default()
    }
}

impl IntoView for WindowControls {
    fn into_view(self) -> View {
        View::from_raw(ffi!(sys::elg_window_controls(self.style as i32, self.minimize, self.maximize)))
    }
}
