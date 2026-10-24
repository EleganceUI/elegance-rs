//! Animations, animated transactions, and transitions.

use std::cell::{Cell, RefCell};
use std::ptr::NonNull;

use elegance_sys as sys;

use crate::error::ffi;
use crate::ffi as cffi;
use crate::geometry::{Edges, Size};
use crate::reactive::{run_borrowed, Binding, Computed, State};

/// A timing curve: how an animated change progresses over time.
pub struct Animation {
    ptr: NonNull<sys::Animation>,
}

impl Clone for Animation {
    fn clone(&self) -> Self {
        Self::from_raw(unsafe { sys::elg_animation_clone(self.as_ptr()) })
    }
}

impl Drop for Animation {
    fn drop(&mut self) {
        unsafe { sys::elg_animation_free(self.ptr.as_ptr()) }
    }
}

impl Animation {
    fn from_raw(ptr: *mut sys::Animation) -> Self {
        Self { ptr: NonNull::new(ptr).expect("elegance: no animation") }
    }
    pub(crate) fn as_ptr(&self) -> *const sys::Animation {
        self.ptr.as_ptr()
    }

    pub fn linear(duration: f64) -> Self {
        Self::from_raw(unsafe { sys::elg_animation_linear(duration) })
    }
    pub fn ease_in(duration: f64) -> Self {
        Self::from_raw(unsafe { sys::elg_animation_ease_in(duration) })
    }
    pub fn ease_out(duration: f64) -> Self {
        Self::from_raw(unsafe { sys::elg_animation_ease_out(duration) })
    }
    pub fn ease_in_out(duration: f64) -> Self {
        Self::from_raw(unsafe { sys::elg_animation_ease_in_out(duration) })
    }
    /// A spring: `response` is roughly its duration in seconds; `damping_fraction` 1 doesn't
    /// overshoot, less bounces.
    pub fn spring(response: f64, damping_fraction: f64) -> Self {
        Self::from_raw(unsafe { sys::elg_animation_spring(response, damping_fraction) })
    }
    /// A spring that doesn't bounce (the usual choice for UI).
    pub fn smooth(duration: f64) -> Self {
        Self::from_raw(unsafe { sys::elg_animation_smooth(duration) })
    }
    /// `curve` maps time 0...1 to progress (usually 0...1).
    pub fn custom(curve: impl Fn(f64) -> f64 + 'static, duration: f64) -> Self {
        let curve = cffi::closure(move |args, ret| unsafe { *(ret as *mut f64) = curve(*(args as *const f64)) });
        Self::from_raw(ffi!(sys::elg_animation_custom(curve, duration)))
    }
    /// The same animation, starting `seconds` later.
    pub fn delay(&self, seconds: f64) -> Self {
        Self::from_raw(unsafe { sys::elg_animation_delay(self.as_ptr(), seconds) })
    }
    /// Progress `seconds` after the start, 0...1 (springs may overshoot).
    pub fn progress(&self, seconds: f64) -> f64 {
        ffi!(sys::elg_animation_progress(self.as_ptr(), seconds))
    }
    pub fn finished(&self, seconds: f64) -> bool {
        ffi!(sys::elg_animation_finished(self.as_ptr(), seconds))
    }
}

impl Default for Animation {
    fn default() -> Self {
        Self::ease_in_out(0.35)
    }
}

/// Makes the writes inside `changes` animate: views that change because of them move, resize,
/// fade, and change color with `animation`, and If/ForEach use their transitions.
pub fn with_animation<R>(animation: impl Into<Animation>, changes: impl FnOnce() -> R) -> R {
    let animation = animation.into();
    run_borrowed(changes, |closure| unsafe { sys::elg_with_animation(animation.as_ptr(), closure) })
}

/// A value watched by `.animation(...)`: changes inside the view animate when it changes.
pub trait AnimationValue: 'static {
    #[doc(hidden)]
    fn watcher(self) -> Box<dyn Fn() -> u64>;
}

fn version_of<T: PartialEq + 'static>(read: impl Fn() -> T + 'static) -> Box<dyn Fn() -> u64> {
    let last: RefCell<Option<T>> = RefCell::new(None);
    let version = Cell::new(0u64);
    Box::new(move || {
        let next = read();
        let mut last = last.borrow_mut();
        if last.as_ref() != Some(&next) {
            version.set(version.get() + 1);
            *last = Some(next);
        }
        version.get()
    })
}

impl<F: Fn() -> T + 'static, T: PartialEq + 'static> AnimationValue for F {
    fn watcher(self) -> Box<dyn Fn() -> u64> {
        version_of(self)
    }
}

impl<T: PartialEq + Clone + 'static> AnimationValue for State<T> {
    fn watcher(self) -> Box<dyn Fn() -> u64> {
        version_of(move || self.get())
    }
}

impl<T: PartialEq + Clone + 'static> AnimationValue for Computed<T> {
    fn watcher(self) -> Box<dyn Fn() -> u64> {
        version_of(move || self.get())
    }
}

impl<T: PartialEq + Clone + 'static> AnimationValue for Binding<T> {
    fn watcher(self) -> Box<dyn Fn() -> u64> {
        version_of(move || self.get())
    }
}

/// How a view enters and leaves when an If/ForEach adds or removes it with an animation.
pub struct Transition {
    ptr: NonNull<sys::Transition>,
}

impl Clone for Transition {
    fn clone(&self) -> Self {
        Self::from_raw(unsafe { sys::elg_transition_clone(self.as_ptr()) })
    }
}

impl Drop for Transition {
    fn drop(&mut self) {
        unsafe { sys::elg_transition_free(self.ptr.as_ptr()) }
    }
}

/// How a view looks partway through a transition.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub opacity: f32,
    pub scale: f32,
    /// Offset in points.
    pub x: f32,
    pub y: f32,
}

#[elegance_macros::elg_builder]
impl Transition {
    fn from_raw(ptr: *mut sys::Transition) -> Self {
        Self { ptr: NonNull::new(ptr).expect("elegance: no transition") }
    }
    pub(crate) fn as_ptr(&self) -> *const sys::Transition {
        self.ptr.as_ptr()
    }

    pub fn opacity() -> Self {
        Self::from_raw(unsafe { sys::elg_transition_opacity() })
    }
    /// Appears and disappears at once.
    pub fn identity() -> Self {
        Self::from_raw(unsafe { sys::elg_transition_identity() })
    }
    pub fn scale(from: f32) -> Self {
        Self::from_raw(unsafe { sys::elg_transition_scale(from) })
    }
    pub fn offset(x: f32, y: f32) -> Self {
        Self::from_raw(unsafe { sys::elg_transition_offset(x, y) })
    }
    /// Slides in from (and out to) an edge of the view's bounds.
    pub fn move_edge(edge: impl Into<Edges>) -> Self {
        Self::from_raw(unsafe { sys::elg_transition_move(edge.into().0) })
    }
    /// In from the leading edge, out to the trailing edge.
    pub fn slide() -> Self {
        Self::from_raw(unsafe { sys::elg_transition_slide() })
    }
    pub fn asymmetric(insertion: impl Into<Transition>, removal: impl Into<Transition>) -> Self {
        let (insertion, removal) = (insertion.into(), removal.into());
        Self::from_raw(unsafe { sys::elg_transition_asymmetric(insertion.as_ptr(), removal.as_ptr()) })
    }
    pub fn combined(&self, other: impl Into<Transition>) -> Self {
        let other = other.into();
        Self::from_raw(unsafe { sys::elg_transition_combined(self.as_ptr(), other.as_ptr()) })
    }
    /// The pose at `progress` (0: hidden, 1: shown) of an insertion or removal.
    pub fn pose(&self, progress: f32, removal: bool, size: Size) -> Pose {
        let p = unsafe { sys::elg_transition_pose(self.as_ptr(), progress, removal, size.c()) };
        Pose { opacity: p.opacity, scale: p.scale, x: p.x, y: p.y }
    }
}
