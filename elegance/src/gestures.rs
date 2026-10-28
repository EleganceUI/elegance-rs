//! Gestures for [`ViewExt::recognize`](crate::ViewExt::recognize): drags, long presses, and
//! trackpad pinches, rotations and two-finger double taps. Each is built up with the callbacks it
//! needs:
//!
//! ```ignore
//! .recognize(DragGesture::new()
//!     .on_began(move |_| start.set(offset.get()))
//!     .on_changed(move |e| offset.set(start.get() + e.translation)))
//! ```
//!
//! A pointer gesture watches presses on its view (and inside it) without taking them. Once it
//! recognizes (a drag moves far enough, a long press is held long enough) it takes the press
//! over: whatever had it, such as a button or an `on_click`, is cancelled and doesn't fire. The
//! innermost gesture that recognizes wins. Controls that track the pointer themselves (sliders,
//! text selection) keep their presses from gestures around them. [`GestureOptions`] change that
//! for a gesture: it can go first, or run alongside the others.

use std::rc::Rc;

use elegance_sys as sys;

use crate::events::{DragEvent, MagnifyEvent, MouseButton, RotateEvent};
use crate::ffi as cffi;
use crate::geometry::{Number, Point};
use crate::view::View;

/// How a pointer gesture (drag, long press) competes with others for a press.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GesturePriority {
    /// The innermost gesture that recognizes the press takes it.
    #[default]
    Normal,
    /// Offered the press before gestures inside its view, and may take it from controls that
    /// keep their presses (a slider): a row's swipe over the controls in it.
    High,
}

/// `recognize_with`'s options, for drags and long presses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GestureOptions {
    pub priority: GesturePriority,
    /// Recognizes alongside whatever has the press (a gesture, a control), without taking it: the
    /// others aren't cancelled, and it isn't cancelled when they recognize. Priority doesn't apply.
    pub simultaneous: bool,
}

#[elegance_macros::elg_builder]
impl GestureOptions {
    pub fn priority(mut self, priority: impl Into<GesturePriority>) -> Self {
        self.priority = priority.into();
        self
    }
    pub fn simultaneous(mut self, simultaneous: bool) -> Self {
        self.simultaneous = simultaneous;
        self
    }
    fn c(self) -> sys::ElgGestureOptions {
        sys::ElgGestureOptions { priority: self.priority as i32, simultaneous: self.simultaneous }
    }
}

/// A gesture `.recognize()` attaches.
pub trait Gesture {
    #[doc(hidden)]
    fn attach(self, view: View, options: GestureOptions) -> View;
}

type EventFn<E> = Rc<dyn Fn(&E)>;

fn event_closure<E: 'static, C: 'static>(f: Option<EventFn<E>>, convert: fn(&C) -> E) -> sys::ElgClosure {
    match f {
        Some(f) => cffi::closure(move |args, _| f(&convert(unsafe { &*(args as *const C) }))),
        None => sys::ElgClosure::NONE,
    }
}

fn action_closure(f: Option<Rc<dyn Fn()>>) -> sys::ElgClosure {
    match f {
        Some(f) => cffi::closure(move |_, _| f()),
        None => sys::ElgClosure::NONE,
    }
}

// --- Drags ---

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragOptions {
    /// How far the pointer moves (in points) before the drag begins. 0 begins it on the press.
    pub minimum_distance: f32,
    pub button: MouseButton,
    /// Hold, then drag (reordering by touch): seconds the press must be held, within
    /// `hold_tolerance` points, before the drag can begin. It then begins at once (`on_began`
    /// with no translation, for a "lifted" look). A press that moves sooner is left to the rest
    /// (scrolling, clicks). 0: no hold.
    pub hold: f64,
    pub hold_tolerance: f32,
}

impl Default for DragOptions {
    fn default() -> Self {
        Self { minimum_distance: 4.0, button: MouseButton::Left, hold: 0.0, hold_tolerance: 10.0 }
    }
}

#[elegance_macros::elg_builder]
impl DragOptions {
    pub fn minimum_distance(mut self, distance: impl Number) -> Self {
        self.minimum_distance = distance.to_f32();
        self
    }
    pub fn button(mut self, button: impl Into<MouseButton>) -> Self {
        self.button = button.into();
        self
    }
    pub fn hold(mut self, seconds: f64) -> Self {
        self.hold = seconds;
        self
    }
    pub fn hold_tolerance(mut self, distance: impl Number) -> Self {
        self.hold_tolerance = distance.to_f32();
        self
    }
}

/// A drag: began once the pointer moved `minimum_distance` from the press, changed as it moves,
/// ended on the release.
#[derive(Clone, Default)]
pub struct DragGesture {
    options: DragOptions,
    began: Option<EventFn<DragEvent>>,
    changed: Option<EventFn<DragEvent>>,
    ended: Option<EventFn<DragEvent>>,
    cancelled: Option<Rc<dyn Fn()>>,
}

impl DragGesture {
    pub fn new() -> Self {
        Self::default()
    }
    /// `DragGesture::with(|o| o.minimum_distance(8))`.
    pub fn with(options: impl FnOnce(DragOptions) -> DragOptions) -> Self {
        Self { options: options(DragOptions::default()), ..Self::default() }
    }
    /// The drag recognized.
    pub fn on_began(mut self, action: impl Fn(&DragEvent) + 'static) -> Self {
        self.began = Some(Rc::new(action));
        self
    }
    /// The pointer moved (also right after `on_began`).
    pub fn on_changed(mut self, action: impl Fn(&DragEvent) + 'static) -> Self {
        self.changed = Some(Rc::new(action));
        self
    }
    /// The button was released.
    pub fn on_ended(mut self, action: impl Fn(&DragEvent) + 'static) -> Self {
        self.ended = Some(Rc::new(action));
        self
    }
    /// The pointer was lost (the window closed, the view was disabled) before the release.
    pub fn on_cancelled(mut self, action: impl Fn() + 'static) -> Self {
        self.cancelled = Some(Rc::new(action));
        self
    }
}

impl Gesture for DragGesture {
    fn attach(self, view: View, options: GestureOptions) -> View {
        let o = self.options;
        let drag = sys::ElgDragOptions {
            minimum_distance: o.minimum_distance,
            button: o.button.c(),
            hold: o.hold,
            hold_tolerance: o.hold_tolerance,
        };
        let began = event_closure(self.began, DragEvent::from_c);
        let changed = event_closure(self.changed, DragEvent::from_c);
        let ended = event_closure(self.ended, DragEvent::from_c);
        let cancelled = action_closure(self.cancelled);
        let options = options.c();
        view.modify(|v| unsafe { sys::elg_mod_drag_gesture(v, drag, began, changed, ended, cancelled, options) })
    }
}

// --- Long presses ---

#[derive(Clone)]
pub struct LongPressOptions {
    /// Seconds held.
    pub duration: f64,
    /// Points the pointer may wander before the press stops counting.
    pub maximum_distance: f32,
    pub button: MouseButton,
    /// true when a press that could become the long press goes down, false when it lifts, moves
    /// away or is taken by something else: for a "being held" highlight.
    pub pressing: Option<Rc<dyn Fn(bool)>>,
}

impl Default for LongPressOptions {
    fn default() -> Self {
        Self { duration: 0.5, maximum_distance: 10.0, button: MouseButton::Left, pressing: None }
    }
}

#[elegance_macros::elg_builder]
impl LongPressOptions {
    pub fn duration(mut self, seconds: f64) -> Self {
        self.duration = seconds;
        self
    }
    pub fn maximum_distance(mut self, distance: impl Number) -> Self {
        self.maximum_distance = distance.to_f32();
        self
    }
    pub fn button(mut self, button: impl Into<MouseButton>) -> Self {
        self.button = button.into();
        self
    }
    pub fn pressing(mut self, pressing: impl Fn(bool) + 'static) -> Self {
        self.pressing = Some(Rc::new(pressing));
        self
    }
}

/// Runs `action` once the press has been held for the duration. The click it would have been
/// then doesn't happen.
#[derive(Clone)]
pub struct LongPressGesture {
    action: Rc<dyn Fn()>,
    options: LongPressOptions,
}

impl LongPressGesture {
    pub fn new(action: impl Fn() + 'static) -> Self {
        Self { action: Rc::new(action), options: LongPressOptions::default() }
    }
    pub fn options(mut self, options: impl FnOnce(LongPressOptions) -> LongPressOptions) -> Self {
        self.options = options(self.options);
        self
    }
}

impl Gesture for LongPressGesture {
    fn attach(self, view: View, options: GestureOptions) -> View {
        let o = self.options;
        let long_press = sys::ElgLongPressOptions {
            duration: o.duration,
            maximum_distance: o.maximum_distance,
            button: o.button.c(),
        };
        let pressing = match o.pressing {
            Some(f) => cffi::consumer(move |b: bool| f(b)),
            None => sys::ElgClosure::NONE,
        };
        let action = self.action;
        let action = cffi::closure(move |_, _| action());
        let options = options.c();
        view.modify(|v| unsafe { sys::elg_mod_long_press_gesture(v, action, long_press, pressing, options) })
    }
}

// --- Trackpad gestures ---

/// Phased trackpad gestures: pinching ([`MagnifyGesture`]) and turning two fingers
/// ([`RotateGesture`]). The innermost one under the pointer that takes the start gets the rest.
pub struct PhasedGesture<E: 'static> {
    began: Option<EventFn<E>>,
    changed: Option<EventFn<E>>,
    ended: Option<EventFn<E>>,
    cancelled: Option<Rc<dyn Fn()>>,
}

impl<E> Clone for PhasedGesture<E> {
    fn clone(&self) -> Self {
        Self {
            began: self.began.clone(),
            changed: self.changed.clone(),
            ended: self.ended.clone(),
            cancelled: self.cancelled.clone(),
        }
    }
}

impl<E> Default for PhasedGesture<E> {
    fn default() -> Self {
        Self { began: None, changed: None, ended: None, cancelled: None }
    }
}

impl<E: 'static> PhasedGesture<E> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn on_began(mut self, action: impl Fn(&E) + 'static) -> Self {
        self.began = Some(Rc::new(action));
        self
    }
    pub fn on_changed(mut self, action: impl Fn(&E) + 'static) -> Self {
        self.changed = Some(Rc::new(action));
        self
    }
    pub fn on_ended(mut self, action: impl Fn(&E) + 'static) -> Self {
        self.ended = Some(Rc::new(action));
        self
    }
    /// The system took the gesture over.
    pub fn on_cancelled(mut self, action: impl Fn() + 'static) -> Self {
        self.cancelled = Some(Rc::new(action));
        self
    }
}

/// Pinching: `scale` is since the gesture began, `delta` since the last event.
pub type MagnifyGesture = PhasedGesture<MagnifyEvent>;
/// Turning two fingers: degrees clockwise since the gesture began.
pub type RotateGesture = PhasedGesture<RotateEvent>;

impl Gesture for MagnifyGesture {
    fn attach(self, view: View, _: GestureOptions) -> View {
        let began = event_closure(self.began, MagnifyEvent::from_c);
        let changed = event_closure(self.changed, MagnifyEvent::from_c);
        let ended = event_closure(self.ended, MagnifyEvent::from_c);
        let cancelled = action_closure(self.cancelled);
        view.modify(|v| unsafe { sys::elg_mod_magnify_gesture(v, began, changed, ended, cancelled) })
    }
}

impl Gesture for RotateGesture {
    fn attach(self, view: View, _: GestureOptions) -> View {
        let began = event_closure(self.began, RotateEvent::from_c);
        let changed = event_closure(self.changed, RotateEvent::from_c);
        let ended = event_closure(self.ended, RotateEvent::from_c);
        let cancelled = action_closure(self.cancelled);
        view.modify(|v| unsafe { sys::elg_mod_rotate_gesture(v, began, changed, ended, cancelled) })
    }
}

/// A two-finger double tap on a trackpad (usually: zoom to fit). `action` gets where it was, in
/// window coordinates.
#[derive(Clone)]
pub struct TwoFingerDoubleTapGesture(Rc<dyn Fn(Point)>);

impl TwoFingerDoubleTapGesture {
    pub fn new(action: impl Fn(Point) + 'static) -> Self {
        Self(Rc::new(action))
    }
}

impl Gesture for TwoFingerDoubleTapGesture {
    fn attach(self, view: View, _: GestureOptions) -> View {
        let action = self.0;
        let action = cffi::consumer(move |p: sys::ElgPoint| action(Point::from_c(p)));
        view.modify(|v| unsafe { sys::elg_mod_on_two_finger_double_tap(v, action) })
    }
}
