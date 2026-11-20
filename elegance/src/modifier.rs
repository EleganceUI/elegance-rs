//! Modifiers of your own, in each of Elegance's modifier protocols, and the built-in modifier
//! types for `.modifier(...)`.
//!
//! ```ignore
//! struct Card;
//! impl ViewModifier for Card {
//!     fn body(&self, content: View) -> View {
//!         content.padding(16).background_in(.surface, RoundedRectangle(12))
//!     }
//! }
//! Text("Hello").modifier(Card)
//! ```

use std::ffi::c_void;

use elegance_sys as sys;

use crate::color::Color;
use crate::environment::EnvironmentValues;
use crate::events::{
    ClickOptions, EventPhase, EventResult, HoverDelay, PixelSize, KeyEvent, MagnifyEvent, PointerEvent, PointerStyle, RotateEvent,
    ScrollEvent, TextInputEvent,
};
use crate::ffi as cffi;
use crate::geometry::{Alignment, ContentMode, EdgeInsets, Point, ProposedSize, Rect, Size, UnitPoint};
use crate::gestures::{
    DragGesture, Gesture, GestureOptions, LongPressGesture, MagnifyGesture, RotateGesture, TwoFingerDoubleTapGesture,
};
use crate::layout::LayoutChild;
use crate::prop::{with_prop, IntoProp, Prop};
use crate::shapes::Shape;
use crate::view::{BlendMode, ColorFilter, FrameOptions, Shadow, View, ViewExt};

/// Anything `.modifier()` takes.
pub trait Modifier {
    fn apply(self, content: View) -> View;
}

/// Composes other modifiers. Most modifiers are this kind. `body` runs once, when the view is
/// mounted.
pub trait ViewModifier: 'static {
    fn body(&self, content: View) -> View;
}

/// Takes part in layout, wrapping one child (padding, frame, ...).
pub trait LayoutModifier: 'static {
    fn size_that_fits(&self, proposal: ProposedSize, content: &LayoutChild) -> Size;
    /// `bounds` has the size this modifier reported for `proposal`, positioned by the parent.
    fn place(&self, bounds: Rect, proposal: ProposedSize, content: &LayoutChild);
    /// Overrides the content's layout priority when set.
    fn layout_priority(&self) -> Option<f32> {
        None
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Placement {
    /// A background.
    #[default]
    Behind,
    /// An overlay.
    InFront,
    /// Not drawn: its alpha masks the content.
    Mask,
}

/// Adds a secondary view sized to the content, behind or in front of it. It doesn't affect
/// layout.
pub trait DecorationModifier: 'static {
    fn decoration(&self) -> View;
    fn placement(&self) -> Placement;
    fn alignment(&self) -> Alignment {
        Alignment::CENTER
    }
}

/// Changes inherited values for everything inside the modified view.
pub trait EnvironmentModifier: 'static {
    fn apply(&self, values: &mut EnvironmentValues);
}

/// How the modified view's content is drawn and hit tested (clip, opacity, shadow, offset,
/// z-index, hit testing), not its layout. Valid during `EffectModifier::apply`.
pub struct Effects {
    ptr: *mut sys::Effects,
}

#[elegance_macros::elg_builder]
impl Effects {
    /// Clips to a shape filling the view's bounds (`Rectangle()`: to the bounds).
    pub fn set_clip_shape(&mut self, shape: Option<Shape>) {
        unsafe { sys::elg_effects_set_clip_shape(self.ptr, shape.as_ref().map_or(std::ptr::null(), Shape::as_ptr)) }
    }
    pub fn set_opacity(&mut self, opacity: impl IntoProp<f32>) {
        with_prop(opacity.into_prop(), |p| crate::error::ffi!(sys::elg_effects_set_opacity(self.ptr, p)))
    }
    pub fn set_shadow(&mut self, shadow: Option<Shadow>) {
        let c = shadow.map(Shadow::c);
        unsafe { sys::elg_effects_set_shadow(self.ptr, c.as_ref().map_or(std::ptr::null(), |s| s as *const sys::ElgShadow)) }
    }
    /// Moves the drawn content (and where it's hit) without affecting layout.
    pub fn set_offset(&mut self, x: impl IntoProp<f32>, y: impl IntoProp<f32>) {
        with_prop(x.into_prop(), |px| with_prop(y.into_prop(), |py| crate::error::ffi!(sys::elg_effects_set_offset(self.ptr, px, py))))
    }
    /// Stacking order among siblings, when set.
    pub fn set_z_index(&mut self, z: Option<Prop<f32>>) {
        match z {
            Some(z) => with_prop(z, |p| crate::error::ffi!(sys::elg_effects_set_z_index(self.ptr, &p))),
            None => crate::error::ffi!(sys::elg_effects_set_z_index(self.ptr, std::ptr::null())),
        }
    }
    /// Hit test this shape (filling the bounds) instead of the drawn content.
    pub fn set_content_shape(&mut self, shape: Option<Shape>) {
        unsafe { sys::elg_effects_set_content_shape(self.ptr, shape.as_ref().map_or(std::ptr::null(), Shape::as_ptr)) }
    }
    /// A shadow inside the content, from its edges inward.
    pub fn set_inner_shadow(&mut self, shadow: Option<Shadow>) {
        let c = shadow.map(Shadow::c);
        unsafe {
            sys::elg_effects_set_inner_shadow(self.ptr, c.as_ref().map_or(std::ptr::null(), |s| s as *const sys::ElgShadow))
        }
    }
    /// Gaussian blur radius in points; `opaque`: edges stay sharp.
    pub fn set_blur(&mut self, radius: Option<Prop<f32>>, opaque: bool) {
        match radius {
            Some(r) => with_prop(r, |p| crate::error::ffi!(sys::elg_effects_set_blur(self.ptr, &p, opaque))),
            None => crate::error::ffi!(sys::elg_effects_set_blur(self.ptr, std::ptr::null(), opaque)),
        }
    }
    pub fn set_color_filter(&mut self, filter: Option<ColorFilter>) {
        match filter {
            Some(f) => f.with_c(|kind, amount, color| {
                crate::error::ffi!(sys::elg_effects_set_color_filter(self.ptr, kind, amount, color))
            }),
            None => with_prop(Prop::Value(1.0), |amount| {
                with_prop(Prop::Value(Color::WHITE), |color| {
                    crate::error::ffi!(sys::elg_effects_set_color_filter(self.ptr, -1, amount, color))
                })
            }),
        }
    }
    pub fn blend_mode(&self) -> BlendMode {
        BlendMode::from_c(unsafe { sys::elg_effects_blend_mode(self.ptr) })
    }
    pub fn set_blend_mode(&mut self, mode: BlendMode) {
        unsafe { sys::elg_effects_set_blend_mode(self.ptr, mode as i32) }
    }
    /// Drawn offscreen first even when nothing else needs it.
    pub fn compositing_group(&self) -> bool {
        unsafe { sys::elg_effects_compositing_group(self.ptr) }
    }
    pub fn set_compositing_group(&mut self, group: bool) {
        unsafe { sys::elg_effects_set_compositing_group(self.ptr, group) }
    }
    /// Rotation of the drawn content, in degrees clockwise, about the anchor.
    pub fn set_rotation(&mut self, degrees: Option<Prop<f32>>) {
        match degrees {
            Some(d) => with_prop(d, |p| crate::error::ffi!(sys::elg_effects_set_rotation(self.ptr, &p))),
            None => crate::error::ffi!(sys::elg_effects_set_rotation(self.ptr, std::ptr::null())),
        }
    }
    /// Horizontal and vertical scale of the drawn content about the anchor (1: as is); each is
    /// unset by `None`, independently of the other.
    pub fn set_scale(&mut self, x: Option<Prop<f32>>, y: Option<Prop<f32>>) {
        fn with_optional(prop: Option<Prop<f32>>, f: impl FnOnce(*const sys::ElgProp)) {
            match prop {
                Some(prop) => with_prop(prop, |p| f(&p)),
                None => f(std::ptr::null()),
            }
        }
        let ptr = self.ptr;
        with_optional(x, |px| with_optional(y, |py| crate::error::ffi!(sys::elg_effects_set_scale(ptr, px, py))));
    }
    /// What rotation and scale pivot around.
    pub fn anchor(&self) -> UnitPoint {
        UnitPoint::from_c(unsafe { sys::elg_effects_anchor(self.ptr) })
    }
    pub fn set_anchor(&mut self, anchor: UnitPoint) {
        unsafe { sys::elg_effects_set_anchor(self.ptr, anchor.c()) }
    }
    pub fn allows_hit_testing(&self) -> bool {
        unsafe { sys::elg_effects_allows_hit_testing(self.ptr) }
    }

    // Reading what the effects already have, so a modifier can build on it. Props read the
    // values in the environment where they're used (tracked reads, reactive as they are).

    /// The clip shape, if any.
    pub fn clip_shape(&self) -> Option<Shape> {
        let mut shape = std::ptr::null_mut();
        crate::error::ffi!(sys::elg_effects_clip_shape(self.ptr, &mut shape)).then(|| Shape::from_raw(shape))
    }
    /// The hit-test shape, if any.
    pub fn content_shape(&self) -> Option<Shape> {
        let mut shape = std::ptr::null_mut();
        crate::error::ffi!(sys::elg_effects_content_shape(self.ptr, &mut shape)).then(|| Shape::from_raw(shape))
    }
    /// The opacity, to multiply: `effects.set_opacity(Prop::from_env(move |env| opacity.get(env) * 0.5))`.
    pub fn opacity(&self) -> Prop<f32> {
        Prop::<f32>::from_handle(crate::error::ffi!(sys::elg_effects_opacity(self.ptr))).unwrap_or(Prop::Value(1.0))
    }
    pub fn shadow(&self) -> Option<Shadow> {
        let mut shadow = sys::ElgShadow::default();
        crate::error::ffi!(sys::elg_effects_shadow(self.ptr, &mut shadow)).then(|| Shadow::from_c(shadow))
    }
    pub fn inner_shadow(&self) -> Option<Shadow> {
        let mut shadow = sys::ElgShadow::default();
        crate::error::ffi!(sys::elg_effects_inner_shadow(self.ptr, &mut shadow)).then(|| Shadow::from_c(shadow))
    }
    /// The blur radius in points, if there's a blur.
    pub fn blur(&self) -> Option<Prop<f32>> {
        let mut radius = std::ptr::null_mut();
        crate::error::ffi!(sys::elg_effects_blur(self.ptr, &mut radius)).then(|| Prop::<f32>::from_handle(radius)).flatten()
    }
    /// The blurred content fills its bounds: edges stay sharp.
    pub fn blur_opaque(&self) -> bool {
        crate::error::ffi!(sys::elg_effects_blur_opaque(self.ptr))
    }
    pub fn color_filter(&self) -> Option<ColorFilter> {
        let (mut kind, mut amount, mut color) = (0, std::ptr::null_mut(), std::ptr::null_mut());
        if !crate::error::ffi!(sys::elg_effects_color_filter(self.ptr, &mut kind, &mut amount, &mut color)) {
            return None;
        }
        let amount = Prop::<f32>::from_handle(amount).unwrap_or(Prop::Value(1.0));
        let color = Prop::<Color>::from_handle(color).unwrap_or(Prop::Value(Color::WHITE));
        Some(match kind {
            0 => ColorFilter::Brightness(amount),
            1 => ColorFilter::Contrast(amount),
            2 => ColorFilter::Saturation(amount),
            3 => ColorFilter::Grayscale(amount),
            4 => ColorFilter::HueRotation(amount),
            5 => ColorFilter::Sepia(amount),
            6 => ColorFilter::Invert(amount),
            7 => ColorFilter::Multiply(color),
            _ => ColorFilter::LuminanceToAlpha,
        })
    }
    /// The rotation in degrees, clockwise about the anchor, if set.
    pub fn rotation(&self) -> Option<Prop<f32>> {
        let mut degrees = std::ptr::null_mut();
        crate::error::ffi!(sys::elg_effects_rotation(self.ptr, &mut degrees)).then(|| Prop::<f32>::from_handle(degrees)).flatten()
    }
    /// The horizontal and vertical scale about the anchor, each if set.
    pub fn scale(&self) -> (Option<Prop<f32>>, Option<Prop<f32>>) {
        let (mut x, mut y) = (std::ptr::null_mut(), std::ptr::null_mut());
        crate::error::ffi!(sys::elg_effects_scale(self.ptr, &mut x, &mut y));
        (Prop::<f32>::from_handle(x), Prop::<f32>::from_handle(y))
    }
    /// The offset of the drawn content.
    pub fn offset(&self) -> (Prop<f32>, Prop<f32>) {
        let (mut x, mut y) = (std::ptr::null_mut(), std::ptr::null_mut());
        crate::error::ffi!(sys::elg_effects_offset(self.ptr, &mut x, &mut y));
        (Prop::<f32>::from_handle(x).unwrap_or(Prop::Value(0.0)), Prop::<f32>::from_handle(y).unwrap_or(Prop::Value(0.0)))
    }
    /// The stacking order among siblings, if set.
    pub fn z_index(&self) -> Option<Prop<f32>> {
        let mut z = std::ptr::null_mut();
        crate::error::ffi!(sys::elg_effects_z_index(self.ptr, &mut z)).then(|| Prop::<f32>::from_handle(z)).flatten()
    }
    /// Also calls `action` with the view's size in physical pixels, after the handler the effects
    /// already have: after the first layout and whenever the size changes, on the UI thread,
    /// before paint.
    pub fn add_on_size_change(&mut self, action: impl Fn(PixelSize) + 'static) {
        let action = cffi::consumer(move |size: sys::ElgPixelSize| action(PixelSize { width: size.width, height: size.height }));
        crate::error::ffi!(sys::elg_effects_add_on_size_change(self.ptr, action))
    }
    pub fn set_allows_hit_testing(&mut self, enabled: bool) {
        unsafe { sys::elg_effects_set_allows_hit_testing(self.ptr, enabled) }
    }
}

/// Changes how the modified view's content is drawn and hit tested.
pub trait EffectModifier: 'static {
    fn apply(&self, effects: &mut Effects);
}

/// Handles input for a mounted view. Every method has a default that ignores the event.
#[allow(unused_variables)]
pub trait EventHandler: 'static {
    /// `frame` is the view's frame in window coordinates.
    fn pointer(&mut self, event: &PointerEvent, frame: Rect) -> EventResult {
        EventResult::IGNORED
    }
    fn hover(&mut self, inside: bool) -> EventResult {
        EventResult::IGNORED
    }
    /// The pointer's shape at `location` (window points) over this view. Asked innermost first,
    /// and of the view a press is captured by while it is; `Automatic` leaves it to the views
    /// around. For styles that depend on where the pointer is (a canvas's resize handles); a
    /// fixed one is [`ViewExt::pointer_style`](crate::ViewExt::pointer_style).
    fn pointer_style(&self, location: Point) -> PointerStyle {
        PointerStyle::Automatic
    }
    /// Wheel and trackpad scrolling: goes to the handlers under the pointer, innermost first,
    /// until one consumes it. `frame` as for `pointer`.
    fn scroll(&mut self, event: &ScrollEvent, frame: Rect) -> EventResult {
        EventResult::IGNORED
    }
    /// Trackpad gestures: the handler under the pointer that consumes `Began` gets the rest of
    /// the gesture, innermost first.
    fn magnify(&mut self, event: &MagnifyEvent) -> EventResult {
        EventResult::IGNORED
    }
    fn rotate(&mut self, event: &RotateEvent) -> EventResult {
        EventResult::IGNORED
    }
    /// A two-finger double tap (usually: zoom to fit).
    fn two_finger_double_tap(&mut self, location: Point) -> EventResult {
        EventResult::IGNORED
    }
    fn key(&mut self, event: &KeyEvent) -> EventResult {
        EventResult::IGNORED
    }
    fn focusable(&self) -> bool {
        false
    }
    fn focus(&mut self, focused: bool) -> EventResult {
        EventResult::IGNORED
    }
    fn accepts_text_input(&self) -> bool {
        false
    }
    fn text_input(&mut self, event: &TextInputEvent) -> EventResult {
        EventResult::IGNORED
    }
    /// The hover delay for this handler, instead of the environment's.
    fn hover_delay(&self) -> Option<HoverDelay> {
        None
    }
    /// Hit everywhere in the view's frame, not just where its content draws.
    fn covers_frame(&self) -> bool {
        false
    }
    /// The phase in which it hears the events that have two ([`EventPhase`]): pointer downs, ups
    /// and hovers, scrolling, keys and text input.
    fn phase(&self) -> EventPhase {
        EventPhase::Bubble
    }
}

/// Handles input for the modified view: makes one EventHandler per mounted view.
pub trait EventModifier: 'static {
    fn make_handler(&self) -> Box<dyn EventHandler>;
}

// --- Applying Rust modifiers ---

impl<M: ViewModifier> Modifier for M {
    fn apply(self, content: View) -> View {
        view_modifier(content, self)
    }
}

/// Passes a [`LayoutModifier`] to `.modifier()`.
pub struct AsLayout<M>(pub M);
/// Passes a [`DecorationModifier`] to `.modifier()`.
pub struct AsDecoration<M>(pub M);
/// Passes an [`EnvironmentModifier`] to `.modifier()`.
pub struct AsEnvironment<M>(pub M);
/// Passes an [`EffectModifier`] to `.modifier()`.
pub struct AsEffect<M>(pub M);
/// Passes an [`EventModifier`] to `.modifier()`.
pub struct AsEvent<M>(pub M);

impl<M: LayoutModifier> Modifier for AsLayout<M> {
    fn apply(self, content: View) -> View {
        layout_modifier(content, self.0)
    }
}
impl<M: DecorationModifier> Modifier for AsDecoration<M> {
    fn apply(self, content: View) -> View {
        decoration_modifier(content, self.0)
    }
}
impl<M: EnvironmentModifier> Modifier for AsEnvironment<M> {
    fn apply(self, content: View) -> View {
        let m = self.0;
        environment_modifier(content, move |values| m.apply(values))
    }
}
impl<M: EffectModifier> Modifier for AsEffect<M> {
    fn apply(self, content: View) -> View {
        effect_modifier(content, self.0)
    }
}
impl<M: EventModifier> Modifier for AsEvent<M> {
    fn apply(self, content: View) -> View {
        event_modifier(content, self.0)
    }
}

fn view_modifier(content: View, m: impl ViewModifier) -> View {
    let body = cffi::closure(move |args, ret| unsafe {
        let content = View::from_raw(args as *mut sys::View);
        m.body(content).write_into(ret);
    });
    content.modify(|v| unsafe { sys::elg_mod_rust_view_modifier(v, body) })
}

fn layout_modifier(content: View, m: impl LayoutModifier) -> View {
    let layout = cffi::closure(move |args, ret| unsafe {
        let call = &*(args as *const sys::ElgLayoutModifierCall);
        match call.op {
            0 => {
                let child = LayoutChild::from_ptr(call.content);
                *(ret as *mut sys::ElgSize) = m.size_that_fits(ProposedSize::from_c(call.proposal), child).c();
            }
            1 => {
                let child = LayoutChild::from_ptr(call.content);
                m.place(Rect::from_c(call.bounds), ProposedSize::from_c(call.proposal), child);
            }
            _ => {
                let priority = m.layout_priority();
                *(ret as *mut sys::ElgOptLen) = sys::ElgOptLen { has: priority.is_some(), value: priority.unwrap_or(0.0) };
            }
        }
    });
    content.modify(|v| unsafe { sys::elg_mod_rust_layout_modifier(v, layout) })
}

fn decoration_modifier(content: View, m: impl DecorationModifier) -> View {
    let (placement, alignment) = (m.placement() as i32, m.alignment().c());
    let decoration = cffi::closure(move |_, ret| unsafe { m.decoration().write_into(ret) });
    content.modify(|v| unsafe { sys::elg_mod_rust_decoration_modifier(v, decoration, placement, alignment) })
}

pub(crate) fn environment_modifier(content: View, apply: impl Fn(&mut EnvironmentValues) + 'static) -> View {
    let apply = cffi::closure(move |args, _| {
        let mut values = unsafe { EnvironmentValues::borrowed(args) };
        apply(&mut values);
    });
    content.modify(|v| unsafe { sys::elg_mod_rust_environment_modifier(v, apply) })
}

fn effect_modifier(content: View, m: impl EffectModifier) -> View {
    let apply = cffi::closure(move |args, _| {
        let mut effects = Effects { ptr: args as *mut sys::Effects };
        m.apply(&mut effects);
    });
    content.modify(|v| unsafe { sys::elg_mod_rust_effect_modifier(v, apply) })
}

fn event_modifier(content: View, m: impl EventModifier) -> View {
    let make = cffi::closure(move |_, ret| unsafe {
        let handler = std::cell::RefCell::new(m.make_handler());
        *(ret as *mut sys::ElgClosure) = cffi::closure(move |args, ret| handle_event(&handler, args, ret));
    });
    content.modify(|v| unsafe { sys::elg_mod_rust_event_modifier(v, make) })
}

fn handle_event(handler: &std::cell::RefCell<Box<dyn EventHandler>>, args: *mut c_void, ret: *mut c_void) {
    unsafe {
        let call = &*(args as *const sys::ElgEventCall);
        let mut h = handler.borrow_mut();
        let result = |r: EventResult| *(ret as *mut sys::ElgEventResult) = r.c();
        match call.op {
            0 => result(h.pointer(&PointerEvent::from_c(&call.pointer), Rect::from_c(call.frame))),
            1 => result(h.hover(call.flag)),
            2 => result(h.scroll(&ScrollEvent::from_c(&call.scroll), Rect::from_c(call.frame))),
            3 => result(h.key(&KeyEvent::from_c(&call.key))),
            4 => *(ret as *mut bool) = h.focusable(),
            5 => result(h.focus(call.flag)),
            6 => *(ret as *mut bool) = h.accepts_text_input(),
            7 => result(h.text_input(&TextInputEvent::from_c(&call.text))),
            8 => {
                let delay = h.hover_delay();
                *(ret as *mut sys::ElgOptHoverDelay) =
                    sys::ElgOptHoverDelay { has: delay.is_some(), delay: delay.unwrap_or_default().c() };
            }
            9 => *(ret as *mut bool) = h.covers_frame(),
            10 => *(ret as *mut i32) = h.pointer_style(Point::from_c(call.location)) as i32,
            11 => result(h.magnify(&MagnifyEvent::from_c(&call.magnify))),
            12 => result(h.rotate(&RotateEvent::from_c(&call.rotate))),
            13 => result(h.two_finger_double_tap(Point::from_c(call.location))),
            14 => *(ret as *mut i32) = h.phase() as i32,
            _ => {}
        }
    }
}

// --- Built-in modifier types, as values ---

/// `.modifier(PaddingModifier(EdgeInsets::all(8)))` == `.padding(8)`.
pub struct PaddingModifier(pub EdgeInsets);
impl Modifier for PaddingModifier {
    fn apply(self, content: View) -> View {
        content.padding_insets(self.0)
    }
}

pub struct FrameModifier(pub FrameOptions);
impl Modifier for FrameModifier {
    fn apply(self, content: View) -> View {
        let options = self.0;
        content.frame_with(move |_| options)
    }
}

pub struct FixedSizeModifier {
    pub horizontal: bool,
    pub vertical: bool,
}
impl Modifier for FixedSizeModifier {
    fn apply(self, content: View) -> View {
        content.fixed_size_axes(self.horizontal, self.vertical)
    }
}

pub struct AspectRatioModifier {
    pub ratio: Option<f32>,
    pub mode: ContentMode,
}
impl Modifier for AspectRatioModifier {
    fn apply(self, content: View) -> View {
        let (ratio, mode) = (self.ratio, self.mode);
        content.aspect_ratio_with(move |_| crate::view::AspectRatioOptions { ratio, mode })
    }
}

pub struct LayoutPriorityModifier(pub f32);
impl Modifier for LayoutPriorityModifier {
    fn apply(self, content: View) -> View {
        content.layout_priority(self.0)
    }
}

pub struct BackgroundModifier {
    pub view: View,
    pub alignment: Alignment,
}
impl Modifier for BackgroundModifier {
    fn apply(self, content: View) -> View {
        let alignment = self.alignment;
        content.background_view_with(self.view, move |o| o.alignment(alignment))
    }
}

pub struct OverlayModifier {
    pub view: View,
    pub alignment: Alignment,
}
impl Modifier for OverlayModifier {
    fn apply(self, content: View) -> View {
        let alignment = self.alignment;
        content.overlay_with(self.view, move |o| o.alignment(alignment))
    }
}

pub struct ClipShapeModifier(pub Shape);
impl Modifier for ClipShapeModifier {
    fn apply(self, content: View) -> View {
        content.clip_shape(self.0)
    }
}

pub struct OpacityModifier(pub Prop<f32>);
impl Modifier for OpacityModifier {
    fn apply(self, content: View) -> View {
        content.opacity(self.0)
    }
}

pub struct ShadowModifier(pub Shadow);
impl Modifier for ShadowModifier {
    fn apply(self, content: View) -> View {
        content.shadow(self.0)
    }
}

pub struct ZIndexModifier(pub Prop<f32>);
impl Modifier for ZIndexModifier {
    fn apply(self, content: View) -> View {
        content.z_index(self.0)
    }
}

pub struct OffsetModifier {
    pub x: Prop<f32>,
    pub y: Prop<f32>,
}
impl Modifier for OffsetModifier {
    fn apply(self, content: View) -> View {
        content.offset(self.x, self.y)
    }
}

pub struct ContentShapeModifier(pub Shape);
impl Modifier for ContentShapeModifier {
    fn apply(self, content: View) -> View {
        content.content_shape(self.0)
    }
}

pub struct AllowsHitTestingModifier(pub bool);
impl Modifier for AllowsHitTestingModifier {
    fn apply(self, content: View) -> View {
        content.allows_hit_testing(self.0)
    }
}

pub struct ClickModifier {
    pub action: Box<dyn Fn(&crate::events::ClickEvent)>,
    pub options: ClickOptions,
}
impl Modifier for ClickModifier {
    fn apply(self, content: View) -> View {
        let options = self.options;
        content.on_click_with(self.action, move |_| options)
    }
}

/// `enter` as soon as the pointer comes over the view, `leave` as soon as it goes.
pub struct PointerBoundaryModifier {
    pub enter: Box<dyn FnMut()>,
    pub leave: Box<dyn FnMut()>,
}
impl Modifier for PointerBoundaryModifier {
    fn apply(self, content: View) -> View {
        content.on_pointer_enter(self.enter).on_pointer_leave(self.leave)
    }
}

/// Hover intent; the delay comes from the environment (`HoverDelayModifier`).
pub struct HoverModifier(pub Box<dyn Fn(bool)>);
impl Modifier for HoverModifier {
    fn apply(self, content: View) -> View {
        content.on_hover_change(self.0)
    }
}

/// `.on_pointer(action)` as a value, in either phase.
pub struct PointerModifier {
    pub action: Box<dyn Fn(&PointerEvent) -> bool>,
    pub phase: EventPhase,
}
impl PointerModifier {
    /// In the bubble phase.
    pub fn new(action: impl Fn(&PointerEvent) -> bool + 'static) -> Self {
        Self { action: Box::new(action), phase: EventPhase::Bubble }
    }
}
impl Modifier for PointerModifier {
    fn apply(self, content: View) -> View {
        match self.phase {
            EventPhase::Bubble => content.on_pointer(self.action),
            EventPhase::Capture => content.on_pointer_capture(self.action),
        }
    }
}

/// `.on_scroll(action)` as a value, in either phase.
pub struct ScrollModifier {
    pub action: Box<dyn Fn(&ScrollEvent) -> bool>,
    pub phase: EventPhase,
}
impl ScrollModifier {
    /// In the bubble phase.
    pub fn new(action: impl Fn(&ScrollEvent) -> bool + 'static) -> Self {
        Self { action: Box::new(action), phase: EventPhase::Bubble }
    }
}
impl Modifier for ScrollModifier {
    fn apply(self, content: View) -> View {
        match self.phase {
            EventPhase::Bubble => content.on_scroll(self.action),
            EventPhase::Capture => content.on_scroll_capture(self.action),
        }
    }
}

/// `.on_text_input(action)` as a value, in either phase.
pub struct TextInputModifier {
    pub action: Box<dyn Fn(&TextInputEvent) -> bool>,
    pub phase: EventPhase,
}
impl TextInputModifier {
    /// In the bubble phase.
    pub fn new(action: impl Fn(&TextInputEvent) -> bool + 'static) -> Self {
        Self { action: Box::new(action), phase: EventPhase::Bubble }
    }
}
impl Modifier for TextInputModifier {
    fn apply(self, content: View) -> View {
        match self.phase {
            EventPhase::Bubble => content.on_text_input(self.action),
            EventPhase::Capture => content.on_text_input_capture(self.action),
        }
    }
}

pub struct FocusableModifier(pub bool);
impl Modifier for FocusableModifier {
    fn apply(self, content: View) -> View {
        content.focusable(self.0)
    }
}

/// Shows the content only where `view` (sized and aligned like an overlay) is opaque
/// (`.mask_view()` as a value).
pub struct MaskModifier {
    pub view: View,
    pub alignment: Alignment,
}
impl Modifier for MaskModifier {
    fn apply(self, content: View) -> View {
        let alignment = self.alignment;
        content.mask_view_with(self.view, move |o| o.alignment(alignment))
    }
}

pub struct InnerShadowModifier(pub Shadow);
impl Modifier for InnerShadowModifier {
    fn apply(self, content: View) -> View {
        content.inner_shadow(self.0)
    }
}

pub struct BlurModifier {
    /// In points.
    pub radius: Prop<f32>,
    /// The content fills its bounds: edges stay sharp instead of fading out.
    pub opaque: bool,
}
impl Modifier for BlurModifier {
    fn apply(self, content: View) -> View {
        let opaque = self.opaque;
        content.blur_with(self.radius, move |o| o.opaque(opaque))
    }
}

pub struct ColorFilterModifier(pub ColorFilter);
impl Modifier for ColorFilterModifier {
    fn apply(self, content: View) -> View {
        content.color_filter(self.0)
    }
}

pub struct BlendModeModifier(pub BlendMode);
impl Modifier for BlendModeModifier {
    fn apply(self, content: View) -> View {
        content.blend_mode(self.0)
    }
}

pub struct CompositingGroupModifier;
impl Modifier for CompositingGroupModifier {
    fn apply(self, content: View) -> View {
        content.compositing_group()
    }
}

/// Turns the drawn content `degrees` clockwise about `anchor`.
pub struct RotationModifier {
    pub degrees: Prop<f32>,
    pub anchor: UnitPoint,
}
impl Modifier for RotationModifier {
    fn apply(self, content: View) -> View {
        let anchor = self.anchor;
        content.rotation_effect_with(self.degrees, move |o| o.anchor(anchor))
    }
}

/// Scales the drawn content about `anchor`, per axis.
pub struct ScaleModifier {
    pub x: Prop<f32>,
    pub y: Prop<f32>,
    pub anchor: UnitPoint,
}
impl Modifier for ScaleModifier {
    fn apply(self, content: View) -> View {
        let anchor = self.anchor;
        content.scale_effect_xy_with(self.x, self.y, move |o| o.anchor(anchor))
    }
}

pub struct DragGestureModifier {
    pub gesture: DragGesture,
    pub options: GestureOptions,
}
impl Modifier for DragGestureModifier {
    fn apply(self, content: View) -> View {
        self.gesture.attach(content, self.options)
    }
}

pub struct LongPressGestureModifier {
    pub gesture: LongPressGesture,
    pub options: GestureOptions,
}
impl Modifier for LongPressGestureModifier {
    fn apply(self, content: View) -> View {
        self.gesture.attach(content, self.options)
    }
}

pub struct MagnifyGestureModifier(pub MagnifyGesture);
impl Modifier for MagnifyGestureModifier {
    fn apply(self, content: View) -> View {
        content.recognize(self.0)
    }
}

pub struct RotateGestureModifier(pub RotateGesture);
impl Modifier for RotateGestureModifier {
    fn apply(self, content: View) -> View {
        content.recognize(self.0)
    }
}

pub struct TwoFingerDoubleTapModifier(pub TwoFingerDoubleTapGesture);
impl Modifier for TwoFingerDoubleTapModifier {
    fn apply(self, content: View) -> View {
        content.recognize(self.0)
    }
}

pub struct PointerStyleModifier(pub Prop<PointerStyle>);
impl Modifier for PointerStyleModifier {
    fn apply(self, content: View) -> View {
        content.pointer_style(self.0)
    }
}

/// Sets a layout value on the view for its parent's layout (`.layout_value::<K>(v)` as a value).
pub struct LayoutValueModifier<K: crate::layout::LayoutValueKey>(pub K::Value);
impl<K: crate::layout::LayoutValueKey> Modifier for LayoutValueModifier<K> {
    fn apply(self, content: View) -> View {
        content.layout_value::<K>(self.0)
    }
}

/// Key downs and ups while the view (or something inside) has focus, in either phase.
pub struct KeyEventModifier {
    pub action: Box<dyn Fn(&KeyEvent) -> bool>,
    pub phase: EventPhase,
}
impl KeyEventModifier {
    /// In the bubble phase.
    pub fn new(action: impl Fn(&KeyEvent) -> bool + 'static) -> Self {
        Self { action: Box::new(action), phase: EventPhase::Bubble }
    }
}
impl Modifier for KeyEventModifier {
    fn apply(self, content: View) -> View {
        match self.phase {
            EventPhase::Bubble => content.on_key(self.action),
            EventPhase::Capture => content.on_key_capture(self.action),
        }
    }
}

/// Sets one environment key (`.environment::<K>(v)` as a value).
pub struct EnvironmentKeyModifier<K: crate::environment::EnvironmentKey>(pub Prop<K::Value>);
impl<K: crate::environment::EnvironmentKey> Modifier for EnvironmentKeyModifier<K> {
    fn apply(self, content: View) -> View {
        content.environment::<K>(self.0)
    }
}

pub type FontModifier = EnvironmentKeyModifier<crate::environment::FontKey>;
pub type ForegroundStyleModifier = EnvironmentKeyModifier<crate::environment::ForegroundKey>;
pub type TintModifier = EnvironmentKeyModifier<crate::environment::TintKey>;
pub type ColorSchemeModifier = EnvironmentKeyModifier<crate::environment::ColorSchemeKey>;
pub type ThemeModifier = EnvironmentKeyModifier<crate::environment::ThemeKey>;
pub type DisabledModifier = EnvironmentKeyModifier<crate::environment::DisabledKey>;
pub type ButtonStyleModifier = EnvironmentKeyModifier<crate::environment::ButtonStyleKey>;
pub type HoverDelayModifier = EnvironmentKeyModifier<crate::environment::HoverDelayKey>;
pub type ToggleStyleModifier = EnvironmentKeyModifier<crate::environment::ToggleStyleKey>;
pub type TextFieldStyleModifier = EnvironmentKeyModifier<crate::environment::TextFieldStyleKey>;
pub type CheckboxStyleModifier = EnvironmentKeyModifier<crate::environment::CheckboxStyleKey>;
pub type NumberInputStyleModifier = EnvironmentKeyModifier<crate::environment::NumberInputStyleKey>;
pub type ProgressStyleModifier = EnvironmentKeyModifier<crate::environment::ProgressStyleKey>;
pub type PickerStyleModifier = EnvironmentKeyModifier<crate::environment::PickerStyleKey>;
pub type SliderStyleModifier = EnvironmentKeyModifier<crate::environment::SliderStyleKey>;
pub type NavigationStackStyleModifier = EnvironmentKeyModifier<crate::environment::NavigationStackStyleKey>;
pub type PopoverStyleModifier = EnvironmentKeyModifier<crate::environment::PopoverStyleKey>;
pub type TooltipStyleModifier = EnvironmentKeyModifier<crate::environment::TooltipStyleKey>;
pub type MenuStyleModifier = EnvironmentKeyModifier<crate::environment::MenuStyleKey>;
pub type RowStyleModifier = EnvironmentKeyModifier<crate::environment::RowStyleKey>;