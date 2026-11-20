//! Timers: actions on the UI thread after a delay or at an interval.
//!
//! ```ignore
//! Text("{seconds}").on_appear(move || { every(1.0, move || seconds += 1); })  // ends with the view
//! let hide = after(3.0, move || loading.set(false));
//! hide.cancel();
//! ```
//!
//! A timer started while views are built, or in `on_appear`, belongs to that view: it stops when
//! the view goes away. Started anywhere else (a click), it runs until it has fired ([`after`]) or
//! is cancelled ([`every`]). State written in the action takes effect like a click's.

use std::cell::RefCell;
use std::ptr::NonNull;

use elegance_sys as sys;

use crate::error::ffi;
use crate::ffi as cffi;

/// A started timer, from [`after`] or [`every`]. Clones refer to the same one; dropping them
/// doesn't stop it. The default refers to none: it's never active.
#[derive(Default)]
pub struct Timer {
    ptr: Option<NonNull<sys::Timer>>,
}

impl Clone for Timer {
    fn clone(&self) -> Self {
        match self.ptr {
            Some(ptr) => Self { ptr: NonNull::new(unsafe { sys::elg_timer_clone(ptr.as_ptr()) }) },
            None => Self::default(),
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr {
            unsafe { sys::elg_timer_free(ptr.as_ptr()) }
        }
    }
}

impl std::fmt::Debug for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timer").field("active", &self.active()).finish()
    }
}

impl Timer {
    /// Stops it; the action doesn't run again. Fine to call more than once.
    pub fn cancel(&self) {
        if let Some(ptr) = self.ptr {
            unsafe { sys::elg_timer_cancel(ptr.as_ptr()) }
        }
    }
    /// Whether the timer is still going: not cancelled, its view is still there, and for
    /// [`after`], not fired yet.
    pub fn active(&self) -> bool {
        self.ptr.is_some_and(|ptr| unsafe { sys::elg_timer_active(ptr.as_ptr()) })
    }
}

/// Runs `action` once, `seconds` from now (0: as soon as the current event is handled; negative
/// counts as 0; infinity never runs it, and the timer stays active until it's cancelled or its
/// view goes). Panics when `seconds` is NaN, or off the UI thread of a running App.
#[track_caller]
pub fn after(seconds: f64, action: impl FnOnce() + 'static) -> Timer {
    let action = RefCell::new(Some(action));
    let action = cffi::closure(move |_, _| {
        if let Some(action) = action.borrow_mut().take() {
            action();
        }
    });
    Timer { ptr: NonNull::new(ffi!(sys::elg_timer_after(seconds, action))) }
}

/// Runs `action` every `seconds` (at least a millisecond), the first time `seconds` from now. If
/// the app falls behind, missed runs are skipped rather than run back to back. Panics when
/// `seconds` is NaN, or off the UI thread of a running App.
#[track_caller]
pub fn every(seconds: f64, action: impl FnMut() + 'static) -> Timer {
    Timer { ptr: NonNull::new(ffi!(sys::elg_timer_every(seconds, cffi::action(action)))) }
}
