//! Views, components, and the modifiers every view has.

use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::rc::Rc;

use elegance_sys as sys;

use crate::animation::{Animation, Transition};
use crate::color::{Color, ColorScheme, ShapeStyle, Theme};
use crate::environment::EnvironmentKey;
use crate::error::ffi;
use crate::events::{
    DragEvent, HoverDelay, Key, KeyEvent, MagnifyEvent, PixelSize, PointerEvent, PointerStyle, RotateEvent,
    ScrollEvent,
};
use crate::ffi as cffi;
use crate::font::{Font, Weight};
use crate::geometry::{Alignment, ContentMode, EdgeInsets, Edges, Len, Number, Point, Size};
use crate::prop::{with_prop, IntoProp};
use crate::reactive::IntoBinding;
use crate::shapes::Shape;
use crate::styles::{IntoButtonStyle, IntoTextFieldStyle, IntoToggleStyle};
use crate::text::Text;

mod options;
pub use options::*;

/// A description of UI: an immutable, cheaply cloned value. Nothing is created until a view is
/// mounted in a window.
pub struct View {
    ptr: NonNull<sys::View>,
}

impl Clone for View {
    fn clone(&self) -> Self {
        Self::from_raw(unsafe { sys::elg_view_clone(self.ptr.as_ptr()) })
    }
}

impl Drop for View {
    fn drop(&mut self) {
        unsafe { sys::elg_view_free(self.ptr.as_ptr()) }
    }
}

impl Default for View {
    fn default() -> Self {
        Self::empty()
    }
}

impl View {
    /// An empty view: takes no space and draws nothing.
    pub fn empty() -> Self {
        Self::from_raw(unsafe { sys::elg_view_empty() })
    }

    /// A view whose body runs each time it's mounted (what [`Component`]s become).
    pub fn component<V: IntoView>(body: impl Fn() -> V + 'static) -> Self {
        let body = cffi::closure(move |_, ret| unsafe { body().into_view().write_into(ret) });
        Self::from_raw(ffi!(sys::elg_view_component(body)))
    }

    pub(crate) fn from_raw(ptr: *mut sys::View) -> Self {
        Self {
            ptr: NonNull::new(ptr).expect("elegance: the shim returned no view"),
        }
    }

    pub(crate) fn as_ptr(&self) -> *const sys::View {
        self.ptr.as_ptr()
    }

    pub(crate) fn into_raw(self) -> *mut sys::View {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Moves this view into a closure's C++ `View` result.
    pub(crate) unsafe fn write_into(self, ret: *mut c_void) {
        sys::elg_view_move_into(ret as *mut sys::View, self.into_raw());
    }

    /// Takes ownership of a borrowed C++ view by copying it.
    pub(crate) unsafe fn copy_from(ptr: *const sys::View) -> Self {
        Self::from_raw(sys::elg_view_clone(ptr))
    }

    pub(crate) fn modify(self, f: impl FnOnce(*const sys::View) -> *mut sys::View) -> View {
        let result = f(self.as_ptr());
        crate::error::check();
        View::from_raw(result)
    }
}

pub(crate) fn raw_views(views: Vec<View>) -> Vec<*mut sys::View> {
    views.into_iter().map(View::into_raw).collect()
}

/// Anything that can be shown: views, text, shapes, element builders, and components.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a view",
    label = "not a view",
    note = "views are elements (Text, VStack, ...), View values, and Components; if this is a #[component], a required prop may be missing (shown as `Missing` in its type)"
)]
pub trait IntoView {
    fn into_view(self) -> View;
}

impl IntoView for View {
    fn into_view(self) -> View {
        self
    }
}

impl IntoView for &View {
    fn into_view(self) -> View {
        self.clone()
    }
}

/// Nothing: an empty view.
impl IntoView for () {
    fn into_view(self) -> View {
        View::empty()
    }
}

impl<V: IntoView> IntoView for Option<V> {
    fn into_view(self) -> View {
        match self {
            Some(view) => view.into_view(),
            None => View::empty(),
        }
    }
}

/// A view made of other views. `body` runs each time the component is mounted:
///
/// ```ignore
/// #[derive(Clone)]
/// struct Card { title: String }
///
/// impl Component for Card {
///     fn body(&self) -> impl IntoView {
///         elg! { Text(self.title.clone()).padding(16).background(.surface) }
///     }
/// }
/// ```
///
/// `#[component]` makes one from a function.
pub trait Component: 'static {
    fn body(&self) -> impl IntoView;
}

impl<C: Component> IntoView for C {
    fn into_view(self) -> View {
        let component = Rc::new(self);
        View::component(move || component.body().into_view())
    }
}

// --- Modifiers ---

/// The modifiers every view has. In `elg!` a modifier with named arguments,
/// `.frame(max_width: .infinity)`, calls the `_with` form with a closure that sets them.
#[elegance_macros::elg_builder]
pub trait ViewExt: IntoView + Sized {
    // Layout.

    /// Space around the content, on every edge.
    fn padding(self, length: impl Number) -> View {
        let l = length.to_f32();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_padding(v, l, l, l, l) })
    }
    /// Space on some edges: `.padding_edges(Edges::HORIZONTAL, 12)`.
    fn padding_edges(self, edges: impl Into<Edges>, length: impl Number) -> View {
        let (edges, l) = (edges.into(), length.to_f32());
        let on = |edge| if edges.contains(edge) { l } else { 0.0 };
        let insets = EdgeInsets {
            top: on(Edges::TOP),
            leading: on(Edges::LEADING),
            bottom: on(Edges::BOTTOM),
            trailing: on(Edges::TRAILING),
        };
        self.padding_insets(insets)
    }
    fn padding_insets(self, insets: impl Into<EdgeInsets>) -> View {
        let i = insets.into();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_padding(v, i.top, i.leading, i.bottom, i.trailing) })
    }
    /// `.padding(top: 8, horizontal: 12)` in `elg!`.
    fn padding_with(self, insets: impl FnOnce(EdgeInsets) -> EdgeInsets) -> View {
        self.padding_insets(insets(EdgeInsets::default()))
    }

    /// A fixed size; unset dimensions follow the content.
    fn frame(self, width: impl Into<Len>, height: impl Into<Len>) -> View {
        self.frame_with(|o| o.width(width).height(height))
    }
    /// `.frame(max_width: .infinity, alignment: .top_leading)` in `elg!`.
    fn frame_with(self, options: impl FnOnce(FrameOptions) -> FrameOptions) -> View {
        let o = options(FrameOptions::default()).c();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_frame(v, &o) })
    }
    /// A fixed size that follows state, laid out again whenever it changes.
    fn frame_size(self, size: impl IntoProp<Size>, alignment: impl Into<Alignment>) -> View {
        let a = alignment.into().c();
        let view = self.into_view();
        with_prop(size.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_frame_size(v, p, a) })
        })
    }
    /// Proposes nothing on the given axes, so the content takes its ideal size.
    fn fixed_size(self) -> View {
        self.fixed_size_axes(true, true)
    }
    fn fixed_size_axes(self, horizontal: bool, vertical: bool) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_fixed_size(v, horizontal, vertical) })
    }
    /// Sizes the view to an aspect ratio (width / height) within what's proposed.
    fn aspect_ratio(self, ratio: impl Number, mode: impl Into<ContentMode>) -> View {
        self.aspect_ratio_with(|o| o.ratio(ratio).mode(mode))
    }
    /// The view's own aspect ratio (an image's): `.aspect_ratio(mode: .fill)` in `elg!`.
    fn aspect_ratio_with(
        self,
        options: impl FnOnce(AspectRatioOptions) -> AspectRatioOptions,
    ) -> View {
        let o = options(AspectRatioOptions::default());
        self.into_view().modify(|v| unsafe {
            sys::elg_mod_aspect_ratio(v, o.ratio.is_some(), o.ratio.unwrap_or(0.0), o.mode as i32)
        })
    }
    /// Raster images: take whatever size they're offered. Panics on other views.
    fn resizable(self) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_resizable(v) })
    }
    /// Stacks give space to higher priorities first. Default 0.
    fn layout_priority(self, priority: impl Number) -> View {
        let p = priority.to_f32();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_layout_priority(v, p) })
    }
    /// Stacking order among siblings: higher draws in front and is hit first.
    fn z_index(self, z: impl IntoProp<f32>) -> View {
        let view = self.into_view();
        with_prop(z.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_z_index(v, p) })
        })
    }
    /// Moves the content when drawn, without affecting layout (animated in animated updates).
    fn offset(self, x: impl IntoProp<f32>, y: impl IntoProp<f32>) -> View {
        let view = self.into_view();
        with_prop(x.into_prop(), |px| {
            with_prop(y.into_prop(), |py| {
                view.modify(|v| unsafe { sys::elg_mod_offset(v, px, py) })
            })
        })
    }

    // Grids.

    /// A Grid cell that spans `count` columns.
    fn grid_cell_columns(self, count: i32) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_grid_cell_columns(v, count) })
    }
    /// Where this view sits in its grid cell, instead of its row's, column's, or grid's alignment.
    fn grid_cell_alignment(self, alignment: impl Into<Alignment>) -> View {
        let a = alignment.into().c();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_grid_cell_alignment(v, a) })
    }
    /// The horizontal alignment of every cell in this cell's Grid column.
    fn grid_column_alignment(
        self,
        alignment: impl Into<crate::geometry::HorizontalAlignment>,
    ) -> View {
        let a = alignment.into() as i32;
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_grid_column_alignment(v, a) })
    }

    // Decoration.

    /// A color or gradient filling the view's bounds. Reactive arguments work.
    fn background(self, style: impl IntoProp<ShapeStyle>) -> View {
        let view = self.into_view();
        with_prop(style.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_background(v, p) })
        })
    }
    /// A color or gradient filling a shape behind the view: `.background_in(.accent, Capsule())`.
    fn background_in(self, style: impl IntoProp<ShapeStyle>, shape: impl Into<Shape>) -> View {
        let (view, shape) = (self.into_view(), shape.into());
        with_prop(style.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_background_shape(v, p, shape.as_ptr()) })
        })
    }
    /// A view behind this one, sized to it.
    fn background_view(self, background: impl IntoView) -> View {
        self.background_view_with(background, |o| o)
    }
    fn background_view_with(
        self,
        background: impl IntoView,
        options: impl FnOnce(DecorationOptions) -> DecorationOptions,
    ) -> View {
        let (b, a) = (
            background.into_view(),
            options(DecorationOptions::default()).alignment.c(),
        );
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_background_view(v, b.into_raw(), a) })
    }
    /// A view in front of this one, sized to it.
    fn overlay(self, overlay: impl IntoView) -> View {
        self.overlay_with(overlay, |o| o)
    }
    fn overlay_with(
        self,
        overlay: impl IntoView,
        options: impl FnOnce(DecorationOptions) -> DecorationOptions,
    ) -> View {
        let (o, a) = (
            overlay.into_view(),
            options(DecorationOptions::default()).alignment.c(),
        );
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_overlay(v, o.into_raw(), a) })
    }
    /// A border drawn inside the view's bounds.
    fn border(self, color: impl Into<Color>, width: impl Number) -> View {
        let (c, w) = (color.into().c(), width.to_f32());
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_border(v, c, w) })
    }

    // Effects.

    /// Clips the content to the view's bounds.
    fn clipped(self) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_clipped(v) })
    }
    /// Clips the content to a shape filling the view's bounds.
    fn clip_shape(self, shape: impl Into<Shape>) -> View {
        let shape = shape.into();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_clip_shape(v, shape.as_ptr()) })
    }
    /// Shorthand for `clip_shape(RoundedRectangle(radius))`.
    fn corner_radius(self, radius: impl Number) -> View {
        let r = radius.to_f32();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_corner_radius(v, r) })
    }
    /// Fades the view as a group.
    fn opacity(self, opacity: impl IntoProp<f32>) -> View {
        let view = self.into_view();
        with_prop(opacity.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_opacity(v, p) })
        })
    }
    /// A blurred shadow of the view's content.
    fn shadow(self, shadow: Shadow) -> View {
        let s = shadow.c();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_shadow(v, s) })
    }
    /// `.shadow(radius: 4, y: 2)` in `elg!`.
    fn shadow_with(self, shadow: impl FnOnce(Shadow) -> Shadow) -> View {
        self.shadow(shadow(Shadow::default()))
    }
    /// A shadow cast inside the view's content, from its edges inward (pressed-in, embossed).
    fn inner_shadow(self, shadow: Shadow) -> View {
        let s = shadow.c();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_inner_shadow(v, s) })
    }
    /// `.inner_shadow(radius: 4, y: 2)` in `elg!`; the default is a soft dark shadow from above.
    fn inner_shadow_with(self, shadow: impl FnOnce(Shadow) -> Shadow) -> View {
        let default = Shadow {
            color: Color::BLACK.opacity(0.25),
            radius: 6.0,
            y: 2.0,
            ..Shadow::default()
        };
        self.inner_shadow(shadow(default))
    }
    /// Gaussian blur of the content; `radius` in points. Reactive and animatable.
    fn blur(self, radius: impl IntoProp<f32>) -> View {
        self.blur_with(radius, |o| o)
    }
    /// `.blur(12, opaque: true)` in `elg!`.
    fn blur_with(
        self,
        radius: impl IntoProp<f32>,
        options: impl FnOnce(BlurOptions) -> BlurOptions,
    ) -> View {
        let (view, opaque) = (self.into_view(), options(BlurOptions::default()).opaque);
        with_prop(radius.into_prop(), |r| {
            view.modify(|v| unsafe { sys::elg_mod_blur(v, r, opaque) })
        })
    }
    /// A color adjustment ([`ColorFilter`]); the shorthands below cover each kind.
    fn color_filter(self, filter: ColorFilter) -> View {
        let view = self.into_view();
        filter.with_c(|kind, amount, color| {
            view.modify(|v| unsafe { sys::elg_mod_color_filter(v, kind, amount, color) })
        })
    }
    /// Adds to each channel: -1 black ... 0 unchanged ... 1 white.
    fn brightness(self, amount: impl IntoProp<f32>) -> View {
        self.color_filter(ColorFilter::Brightness(amount.into_prop()))
    }
    /// 0 flat gray, 1 unchanged, above 1 more contrast.
    fn contrast(self, amount: impl IntoProp<f32>) -> View {
        self.color_filter(ColorFilter::Contrast(amount.into_prop()))
    }
    /// 0 gray, 1 unchanged, above 1 more vivid.
    fn saturation(self, amount: impl IntoProp<f32>) -> View {
        self.color_filter(ColorFilter::Saturation(amount.into_prop()))
    }
    /// 0 unchanged ... 1 fully gray.
    fn grayscale(self, amount: impl IntoProp<f32>) -> View {
        self.color_filter(ColorFilter::Grayscale(amount.into_prop()))
    }
    /// Rotates hues around the color wheel by `degrees`.
    fn hue_rotation(self, degrees: impl IntoProp<f32>) -> View {
        self.color_filter(ColorFilter::HueRotation(degrees.into_prop()))
    }
    /// 0 unchanged ... 1 fully sepia-toned.
    fn sepia(self, amount: impl IntoProp<f32>) -> View {
        self.color_filter(ColorFilter::Sepia(amount.into_prop()))
    }
    /// Inverts the colors.
    fn color_invert(self) -> View {
        self.color_invert_amount(1.0)
    }
    /// Inverts the colors partly: 0 unchanged ... 1 inverted.
    fn color_invert_amount(self, amount: impl IntoProp<f32>) -> View {
        self.color_filter(ColorFilter::Invert(amount.into_prop()))
    }
    /// Multiplies every pixel by `color` (alpha too): tints white content, darkens the rest.
    fn color_multiply(self, color: impl IntoProp<Color>) -> View {
        self.color_filter(ColorFilter::Multiply(color.into_prop()))
    }
    /// Brightness becomes opacity: white stays, black disappears (for masks).
    fn luminance_to_alpha(self) -> View {
        self.color_filter(ColorFilter::LuminanceToAlpha)
    }
    /// Blends the view with what's behind it: `.blend_mode(.multiply)`.
    fn blend_mode(self, mode: impl Into<BlendMode>) -> View {
        let mode = mode.into() as i32;
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_blend_mode(v, mode) })
    }
    /// Draws the content offscreen first, then composites it as one image: blend modes inside it
    /// blend with each other only, and a material inside sees only what the group drew.
    fn compositing_group(self) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_compositing_group(v) })
    }
    /// Rotates the drawn content by `degrees` (clockwise) about its center, without affecting
    /// layout or hit testing. Reactive and animatable.
    fn rotation_effect(self, degrees: impl IntoProp<f32>) -> View {
        self.rotation_effect_with(degrees, |o| o)
    }
    /// `.rotation_effect(45, anchor: .top_leading)` in `elg!`.
    fn rotation_effect_with(
        self,
        degrees: impl IntoProp<f32>,
        options: impl FnOnce(AnchorOptions) -> AnchorOptions,
    ) -> View {
        let (view, anchor) = (
            self.into_view(),
            options(AnchorOptions::default()).anchor.c(),
        );
        with_prop(degrees.into_prop(), |d| {
            view.modify(|v| unsafe { sys::elg_mod_rotation_effect(v, d, anchor) })
        })
    }
    /// Scales the drawn content about its center, without affecting layout or hit testing. A
    /// uniform scale stays sharp (vector). Reactive and animatable.
    fn scale_effect(self, scale: impl IntoProp<f32>) -> View {
        self.scale_effect_with(scale, |o| o)
    }
    /// `.scale_effect(1.2, anchor: .bottom)` in `elg!`.
    fn scale_effect_with(
        self,
        scale: impl IntoProp<f32>,
        options: impl FnOnce(AnchorOptions) -> AnchorOptions,
    ) -> View {
        let (view, anchor) = (
            self.into_view(),
            options(AnchorOptions::default()).anchor.c(),
        );
        with_prop(scale.into_prop(), |s| {
            view.modify(|v| unsafe { sys::elg_mod_scale_effect(v, s, std::ptr::null(), anchor) })
        })
    }
    /// Scales each axis separately (rendered offscreen first).
    fn scale_effect_xy(self, x: impl IntoProp<f32>, y: impl IntoProp<f32>) -> View {
        self.scale_effect_xy_with(x, y, |o| o)
    }
    fn scale_effect_xy_with(
        self,
        x: impl IntoProp<f32>,
        y: impl IntoProp<f32>,
        options: impl FnOnce(AnchorOptions) -> AnchorOptions,
    ) -> View {
        let (view, anchor) = (
            self.into_view(),
            options(AnchorOptions::default()).anchor.c(),
        );
        with_prop(x.into_prop(), |x| {
            with_prop(y.into_prop(), |y| {
                view.modify(|v| unsafe { sys::elg_mod_scale_effect(v, x, &y, anchor) })
            })
        })
    }
    /// Shows the content only where `style` is opaque, filling the bounds; its colors don't
    /// matter, only its alpha. A gradient fades the content out:
    /// `.mask(LinearGradient::colors([Color::BLACK, Color::CLEAR]))`.
    fn mask(self, style: impl IntoProp<ShapeStyle>) -> View {
        let view = self.into_view();
        with_prop(style.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_mask_style(v, p) })
        })
    }
    /// Shows the content only where `mask` (sized and aligned like an overlay) is opaque.
    fn mask_view(self, mask: impl IntoView) -> View {
        self.mask_view_with(mask, |o| o)
    }
    fn mask_view_with(
        self,
        mask: impl IntoView,
        options: impl FnOnce(DecorationOptions) -> DecorationOptions,
    ) -> View {
        let (m, a) = (
            mask.into_view(),
            options(DecorationOptions::default()).alignment.c(),
        );
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_mask(v, m.into_raw(), a) })
    }
    /// Hit tests against this shape (filling the bounds) instead of the drawn content.
    fn content_shape(self, shape: impl Into<Shape>) -> View {
        let shape = shape.into();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_content_shape(v, shape.as_ptr()) })
    }
    /// false: the view and everything in it ignore the pointer.
    fn allows_hit_testing(self, enabled: bool) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_allows_hit_testing(v, enabled) })
    }

    // Events.

    /// Receives pointer events over the view's frame. Return true to consume the
    /// event; consuming a press captures its drag and release/cancel events.
    /// Coordinates are in window points, as with a custom EventHandler.
    fn on_pointer_event(self, action: impl Fn(&PointerEvent) -> bool + 'static) -> View {
        crate::modifier::on_pointer_event(self.into_view(), action)
    }

    /// Tracks a left-button drag, capturing the pointer until release or cancellation.
    /// Movement deltas are in window points. Both release and cancellation emit
    /// [`DragEvent::Ended`]. Each mounted view has independent drag state.
    /// Consumes the drag's pointer events, including the initial press.
    fn on_drag(self, action: impl Fn(DragEvent) + 'static) -> View {
        crate::modifier::on_drag(self.into_view(), action)
    }

    /// Receives scrolling over the view's frame. Return true to consume the event,
    /// or false to let an enclosing scroll view handle it.
    fn on_scroll(self, action: impl Fn(&ScrollEvent) -> bool + 'static) -> View {
        crate::modifier::on_scroll(self.into_view(), action)
    }

    /// Runs `action` on a tap. The innermost tap gesture wins.
    fn on_tap_gesture(self, action: impl FnMut() + 'static) -> View {
        self.on_tap_gesture_with(action, |o| o)
    }
    /// `.on_tap_gesture(open, count: 2)` in `elg!`.
    fn on_tap_gesture_with(
        self,
        action: impl FnMut() + 'static,
        options: impl FnOnce(TapOptions) -> TapOptions,
    ) -> View {
        let count = options(TapOptions::default()).count;
        let action = cffi::action(action);
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_on_tap_gesture(v, action, count) })
    }
    /// Pinching on a trackpad over the view (began ... ended). The innermost view handling it
    /// under the pointer takes the gesture.
    fn on_magnify(self, action: impl Fn(&MagnifyEvent) + 'static) -> View {
        let action = cffi::consumer(move |e: sys::ElgMagnifyEvent| action(&MagnifyEvent::from_c(&e)));
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_on_magnify(v, action) })
    }
    /// Turning two fingers on a trackpad over the view (began ... ended).
    fn on_rotate(self, action: impl Fn(&RotateEvent) + 'static) -> View {
        let action = cffi::consumer(move |e: sys::ElgRotateEvent| action(&RotateEvent::from_c(&e)));
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_on_rotate(v, action) })
    }
    /// A two-finger double tap over the view (macOS's smart zoom), with its location.
    fn on_smart_magnify(self, action: impl Fn(Point) + 'static) -> View {
        let action = cffi::consumer(move |p: sys::ElgPoint| action(Point::from_c(p)));
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_on_smart_magnify(v, action) })
    }
    /// The pointer's shape over the view, like CSS's `cursor`: `.pointer_style(.pointer)` over
    /// something to click. Views inside can set their own (a text field's I-beam). Reactive:
    /// `.pointer_style(move || if busy.get() { PointerStyle::Wait } else { PointerStyle::Automatic })`.
    fn pointer_style(self, style: impl IntoProp<PointerStyle>) -> View {
        let view = self.into_view();
        with_prop(style.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_pointer_style(v, p) })
        })
    }
    /// Observes this view's actual layout size in physical pixels, including display-scale changes.
    ///
    /// Called on the UI thread after initial layout and whenever the physical dimensions change.
    /// Repeated sizes are suppressed independently for each mount of the view. Dimensions are
    /// rounded up and exclude visual transforms; zero sizes are reported too.
    ///
    /// This runs after layout and before painting. Queue the size as renderer input (for example,
    /// in a channel or a `Cell`) and render at that size; do not change layout or rebuild views
    /// synchronously from this callback. Place the modifier inside any padding or frame whose
    /// extra space should not be included in the observed size.
    fn on_sized(self, action: impl Fn(PixelSize) + 'static) -> View {
        let action = cffi::consumer(move |size: sys::ElgPixelSize| {
            action(PixelSize {
                width: size.width,
                height: size.height,
            });
        });
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_on_size_change(v, action) })
    }

    /// Called with true when the pointer has rested on the view for the hover delay, and false
    /// when it leaves.
    fn on_hover(self, action: impl Fn(bool) + 'static) -> View {
        let action = cffi::consumer(action);
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_on_hover(v, action, std::ptr::null()) })
    }
    fn on_hover_with(
        self,
        action: impl Fn(bool) + 'static,
        delay: impl FnOnce(HoverDelay) -> HoverDelay,
    ) -> View {
        let (action, delay) = (cffi::consumer(action), delay(HoverDelay::default()).c());
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_on_hover(v, action, &delay) })
    }
    /// Files or text dragged from other apps can be dropped here.
    fn on_drop(self, options: DropOptions) -> View {
        let files = match options.files {
            Some(f) => cffi::closure(move |args, ret| unsafe {
                let arg = &*(args as *const sys::ElgDropFiles);
                let paths = std::slice::from_raw_parts(arg.paths, arg.count)
                    .iter()
                    .map(|p| PathBuf::from(p.as_str()))
                    .collect();
                *(ret as *mut bool) = f(paths, Point::from_c(arg.location));
            }),
            None => sys::ElgClosure::NONE,
        };
        let text = match options.text {
            Some(f) => cffi::closure(move |args, ret| unsafe {
                let arg = &*(args as *const sys::ElgDropText);
                *(ret as *mut bool) = f(arg.text.as_str().to_owned(), Point::from_c(arg.location));
            }),
            None => sys::ElgClosure::NONE,
        };
        let targeted = options
            .targeted
            .map(|f| cffi::consumer(move |b: bool| f(b)))
            .unwrap_or(sys::ElgClosure::NONE);
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_on_drop(v, files, text, targeted) })
    }
    /// `.on_drop(files: |paths, at| ...)` in `elg!`.
    fn on_drop_with(self, options: impl FnOnce(DropOptions) -> DropOptions) -> View {
        self.on_drop(options(DropOptions::default()))
    }

    // Accessibility.

    /// What it's called (an icon-only button needs one).
    fn accessibility_label(self, label: impl IntoProp<String>) -> View {
        let view = self.into_view();
        with_prop(label.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_accessibility_label(v, p) })
        })
    }
    /// What it holds, as words ("3 of 10").
    fn accessibility_value(self, value: impl IntoProp<String>) -> View {
        let view = self.into_view();
        with_prop(value.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_accessibility_value(v, p) })
        })
    }
    /// What happens if it's used, when the label doesn't say.
    fn accessibility_hint(self, hint: impl IntoProp<String>) -> View {
        let view = self.into_view();
        with_prop(hint.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_accessibility_hint(v, p) })
        })
    }
    /// Leaves it (and everything inside) out: decoration.
    fn accessibility_hidden(self, hidden: bool) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_accessibility_hidden(v, hidden) })
    }
    fn accessibility_role(self, role: impl Into<AccessibilityRole>) -> View {
        let role = role.into() as i32;
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_accessibility_role(v, role) })
    }
    /// One element for everything inside, labeled with their text.
    fn accessibility_element(self) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_accessibility_element(v) })
    }
    /// What a screen reader's default action does, for views made clickable with gestures.
    fn accessibility_action(self, action: impl FnMut() + 'static) -> View {
        let action = cffi::action(action);
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_accessibility_action(v, action) })
    }

    /// The hover delay for on_hover handlers and controls inside.
    fn hover_delay(self, delay: HoverDelay) -> View {
        let d = delay.c();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_hover_delay(v, d) })
    }
    fn hover_delay_with(self, delay: impl FnOnce(HoverDelay) -> HoverDelay) -> View {
        self.hover_delay(delay(HoverDelay::default()))
    }

    /// Pressing and dragging here moves the window (custom title bars).
    fn window_drag_area(self, enabled: bool) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_window_drag_area(v, enabled) })
    }

    // Popups.

    /// Right-click opens the menu `menu` builds, usually a `Menu { ... }`.
    fn context_menu<V: IntoView>(self, menu: impl Fn() -> V + 'static) -> View {
        let menu = cffi::closure(move |_, ret| unsafe { menu().into_view().write_into(ret) });
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_context_menu(v, menu) })
    }
    /// A tooltip shown after the pointer rests on the view.
    fn help(self, text: impl Into<String>) -> View {
        let text: String = text.into();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_help(v, cffi::s(&text)) })
    }
    /// A panel of `content` beside the view, shown while `is_presented` is true. With a `State`,
    /// dismissing it sets the state back to false; with a closure or a `Computed`
    /// (`move || todos.read().is_empty()`, `todos.map(Vec::is_empty)`) it's shown while the
    /// condition holds, and dismissing it closes it until the condition next becomes true.
    fn popover<V: IntoView>(
        self,
        is_presented: impl IntoBinding<bool>,
        content: impl Fn() -> V + 'static,
    ) -> View {
        self.popover_with(is_presented, content, |o| o)
    }
    fn popover_with<V: IntoView>(
        self,
        is_presented: impl IntoBinding<bool>,
        content: impl Fn() -> V + 'static,
        options: impl FnOnce(PopoverOptions) -> PopoverOptions,
    ) -> View {
        let o = options(PopoverOptions::default());
        let binding = is_presented.into_binding().cpp();
        let content = cffi::closure(move |_, ret| unsafe { content().into_view().write_into(ret) });
        self.into_view().modify(|v| unsafe {
            sys::elg_mod_popover(v, binding, content, o.opens as i32, o.padded)
        })
    }

    // Keyboard and focus.

    /// true: the view takes keyboard focus. false: it and everything inside never do.
    fn focusable(self, focusable: bool) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_focusable(v, focusable) })
    }
    /// Two-way focus: true while the view has focus; setting it true focuses the view.
    fn focused(self, is_focused: impl IntoBinding<bool>) -> View {
        let binding = is_focused.into_binding().cpp();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_focused(v, binding) })
    }
    /// Runs `action` when `key` is pressed while the view (or something inside) has focus.
    fn on_key_press(self, key: impl Into<Key>, action: impl FnMut() + 'static) -> View {
        let (key, action) = (key.into() as i32, cffi::action(action));
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_on_key_press(v, key, action) })
    }
    /// Sees every key press while focused; return true if handled.
    fn on_key_event(self, action: impl Fn(&KeyEvent) -> bool + 'static) -> View {
        let action = cffi::closure(move |args, ret| unsafe {
            let event = KeyEvent::from_c(&*(args as *const sys::ElgKeyEvent));
            *(ret as *mut bool) = action(&event);
        });
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_on_key_event(v, action) })
    }

    // Environment: inherited by everything inside.

    fn font_weight(self, weight: impl Into<Weight>) -> View {
        let w = weight.into() as i32;
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_font_weight(v, w) })
    }
    fn font(self, font: impl IntoProp<Font>) -> View {
        let view = self.into_view();
        with_prop(font.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_font(v, p) })
        })
    }
    /// The color of text and unfilled shapes. Also takes a theme color.
    fn foreground_style(self, color: impl IntoProp<Color>) -> View {
        let view = self.into_view();
        with_prop(color.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_foreground_style(v, p) })
        })
    }
    /// The accent color of controls.
    fn tint(self, color: impl IntoProp<Color>) -> View {
        let view = self.into_view();
        with_prop(color.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_tint(v, p) })
        })
    }
    /// Light or dark, for theme colors inside.
    fn color_scheme(self, scheme: impl IntoProp<ColorScheme>) -> View {
        let view = self.into_view();
        with_prop(scheme.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_color_scheme(v, p) })
        })
    }
    fn theme(self, theme: Theme) -> View {
        let t = theme.c();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_theme(v, &t) })
    }
    /// Controls inside stop responding and look disabled.
    fn disabled(self, disabled: impl IntoProp<bool>) -> View {
        let view = self.into_view();
        with_prop(disabled.into_prop(), |p| {
            view.modify(|v| unsafe { sys::elg_mod_disabled(v, p) })
        })
    }
    /// The style of every Button inside.
    fn button_style(self, style: impl IntoButtonStyle) -> View {
        let style = style.into_button_style();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_button_style(v, style.as_ptr()) })
    }
    /// The style of every Toggle inside.
    fn toggle_style(self, style: impl IntoToggleStyle) -> View {
        let style = style.into_toggle_style();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_toggle_style(v, style.as_ptr()) })
    }
    /// The style of every TextField inside.
    fn text_field_style(self, style: impl IntoTextFieldStyle) -> View {
        let style = style.into_text_field_style();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_text_field_style(v, style.as_ptr()) })
    }
    /// Sets any environment key: `.environment::<Compact>(true)`.
    fn environment<K: EnvironmentKey>(self, value: impl IntoProp<K::Value>) -> View {
        let value = value.into_prop();
        crate::modifier::environment_modifier(self.into_view(), move |environment| {
            K::write(environment, value.clone())
        })
    }

    // Animation.

    /// Changes inside this view animate with `animation` whenever `value` (a plain value, State,
    /// or closure) changed in the same update.
    fn animation(
        self,
        animation: impl Into<Animation>,
        value: impl crate::animation::AnimationValue,
    ) -> View {
        let animation = animation.into();
        let watch = cffi::producer(value.watcher());
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_animation(v, animation.as_ptr(), watch) })
    }
    /// How this view enters and leaves when an If/ForEach adds or removes it with an animation.
    fn transition(self, transition: impl Into<Transition>) -> View {
        let t = transition.into();
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_transition(v, t.as_ptr()) })
    }
    /// false: this view and everything inside snap to new positions and sizes.
    fn animates_layout(self, enabled: bool) -> View {
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_animates_layout(v, enabled) })
    }

    /// A value for the parent layout to read with `LayoutChild::value::<K>()`.
    fn layout_value<K: crate::layout::LayoutValueKey>(self, value: K::Value) -> View {
        crate::layout::layout_value::<K>(self.into_view(), value)
    }

    /// Applies a modifier, built-in or your own.
    fn modifier(self, modifier: impl crate::modifier::Modifier) -> View {
        modifier.apply(self.into_view())
    }

    /// Function-style helpers: `v.apply(card) == card(v)`.
    fn apply<V: IntoView>(self, function: impl FnOnce(View) -> V) -> View {
        function(self.into_view()).into_view()
    }
}

impl<T: IntoView> ViewExt for T {}

// Text keeps its own font/foreground modifiers (they return Text); the rest come from ViewExt.
impl IntoView for Text {
    fn into_view(self) -> View {
        View::from_raw(unsafe { sys::elg_text_to_view(self.as_ptr()) })
    }
}

impl IntoView for Shape {
    fn into_view(self) -> View {
        View::from_raw(unsafe { sys::elg_shape_to_view(self.as_ptr()) })
    }
}
