//! Animations, animated transactions, and transitions.

use std::cell::{Cell, RefCell};
use std::ptr::NonNull;

use elegance_sys as sys;

use crate::error::ffi;
use crate::ffi as cffi;
use crate::geometry::{Alignment, Edges, Size};
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
    pub(crate) fn from_raw(ptr: *mut sys::Animation) -> Self {
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
    /// A cubic Bézier through (0, 0), (x1, y1), (x2, y2) and (1, 1), like CSS's cubic-bezier():
    /// `Animation::timing_curve(0.55, 0.1, 0.45, 0.9, 1.4)`. Times are clamped to 0...1; progress
    /// outside it overshoots.
    pub fn timing_curve(x1: f64, y1: f64, x2: f64, y2: f64, duration: f64) -> Self {
        Self::from_raw(ffi!(sys::elg_animation_timing_curve(x1, y1, x2, y2, duration)))
    }
    /// The same animation, starting `seconds` later. A negative delay starts partway in, like CSS.
    pub fn delay(&self, seconds: f64) -> Self {
        Self::from_raw(unsafe { sys::elg_animation_delay(self.as_ptr(), seconds) })
    }
    /// Plays `count` times in all (less than 1 counts as 1), landing on the target. With
    /// `autoreverses`, every other time runs backward.
    pub fn repeat_count(&self, count: i32, autoreverses: bool) -> Self {
        Self::from_raw(ffi!(sys::elg_animation_repeat_count(self.as_ptr(), count, autoreverses)))
    }
    /// Plays again and again until the value changes or the view goes away. A spinner turning
    /// forever: rotation 0 to 360, linear, `autoreverses` false.
    pub fn repeat_forever(&self, autoreverses: bool) -> Self {
        Self::from_raw(ffi!(sys::elg_animation_repeat_forever(self.as_ptr(), autoreverses)))
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

/// How a view looks partway through a transition. The default is the view in place: opacity and
/// scale 1, no offset, scaling around the middle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// Multiplies the view's opacity.
    pub opacity: f32,
    /// Multiplies its size.
    pub scale: f32,
    /// Offset in points.
    pub x: f32,
    pub y: f32,
    /// What it scales around, as fractions of its frame (0.5, 0.5: the middle).
    pub anchor_x: f32,
    pub anchor_y: f32,
}

impl Default for Pose {
    fn default() -> Self {
        Self { opacity: 1.0, scale: 1.0, x: 0.0, y: 0.0, anchor_x: 0.5, anchor_y: 0.5 }
    }
}

#[elegance_macros::elg_builder]
impl Transition {
    pub(crate) fn from_raw(ptr: *mut sys::Transition) -> Self {
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
    /// Grows from (and shrinks to) `from` times its size, around its center.
    pub fn scale(from: f32) -> Self {
        Self::scale_anchored(from, Alignment::CENTER)
    }
    /// Grows around the point of its frame `anchor` names: `scale_anchored(0.9, Alignment::TOP)`
    /// grows down from its top edge.
    pub fn scale_anchored(from: f32, anchor: impl Into<Alignment>) -> Self {
        let anchor = anchor.into().c();
        Self::from_raw(unsafe { sys::elg_transition_scale(from, anchor) })
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
        Pose { opacity: p.opacity, scale: p.scale, x: p.x, y: p.y, anchor_x: p.anchor_x, anchor_y: p.anchor_y }
    }
}
