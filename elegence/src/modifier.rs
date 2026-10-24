//! Modifiers of your own, in each of Elegence's modifier protocols, and the built-in modifier
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

use elegence_sys as sys;

use crate::color::Color;
use crate::environment::EnvironmentValues;
use crate::events::{EventResult, HoverDelay, KeyEvent, PointerEvent, ScrollEvent, TextInputEvent};
use crate::ffi as cffi;
use crate::geometry::{Alignment, ContentMode, EdgeInsets, ProposedSize, Rect, Size};
use crate::layout::LayoutChild;
use crate::prop::{with_prop, IntoProp, Prop};
use crate::shapes::Shape;
use crate::view::{FrameOptions, Shadow, View, ViewExt};

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

impl Effects {
    pub fn clip_to_bounds(&self) -> bool {
        unsafe { sys::elg_effects_clip_to_bounds(self.ptr) }
    }
    pub fn set_clip_to_bounds(&mut self, clip: bool) {
        unsafe { sys::elg_effects_set_clip_to_bounds(self.ptr, clip) }
    }
    /// Clips to a shape filling the view's bounds.
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
    pub fn allows_hit_testing(&self) -> bool {
        unsafe { sys::elg_effects_allows_hit_testing(self.ptr) }
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
    fn scroll(&mut self, event: &ScrollEvent) -> EventResult {
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
    let (in_front, alignment) = (m.placement() == Placement::InFront, m.alignment().c());
    let decoration = cffi::closure(move |_, ret| unsafe { m.decoration().write_into(ret) });
    content.modify(|v| unsafe { sys::elg_mod_rust_decoration_modifier(v, decoration, in_front, alignment) })
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
            2 => result(h.scroll(&ScrollEvent::from_c(&call.scroll))),
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
            _ => *(ret as *mut bool) = h.covers_frame(),
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

pub struct BorderModifier {
    pub color: Color,
    pub width: f32,
}
impl Modifier for BorderModifier {
    fn apply(self, content: View) -> View {
        content.border(self.color, self.width)
    }
}

pub struct ClippedModifier;
impl Modifier for ClippedModifier {
    fn apply(self, content: View) -> View {
        content.clipped()
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

pub struct TapGestureModifier {
    pub count: i32,
    pub action: Box<dyn FnMut()>,
}
impl Modifier for TapGestureModifier {
    fn apply(self, content: View) -> View {
        let count = self.count;
        content.on_tap_gesture_with(self.action, move |o| o.count(count))
    }
}

pub struct HoverModifier {
    pub action: Box<dyn Fn(bool)>,
    pub delay: Option<HoverDelay>,
}
impl Modifier for HoverModifier {
    fn apply(self, content: View) -> View {
        match self.delay {
            Some(delay) => content.on_hover_with(self.action, move |_| delay),
            None => content.on_hover(self.action),
        }
    }
}

pub struct FocusableModifier(pub bool);
impl Modifier for FocusableModifier {
    fn apply(self, content: View) -> View {
        content.focusable(self.0)
    }
}

pub struct KeyPressModifier(pub Box<dyn Fn(&KeyEvent) -> bool>);
impl Modifier for KeyPressModifier {
    fn apply(self, content: View) -> View {
        content.on_key_event(self.0)
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
