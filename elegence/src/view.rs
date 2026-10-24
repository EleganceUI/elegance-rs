//! Views, components, and the modifiers every view has.

use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::rc::Rc;

use elegence_sys as sys;

use crate::animation::{Animation, Transition};
use crate::color::{Color, ColorScheme, ShapeStyle, Theme};
use crate::environment::EnvironmentKey;
use crate::error::ffi;
use crate::events::{HoverDelay, Key, KeyEvent};
use crate::ffi as cffi;
use crate::font::{Font, Weight};
use crate::geometry::{Alignment, ContentMode, EdgeInsets, Edges, Len, Number, Point, Size};
use crate::prop::{with_prop, IntoProp};
use crate::reactive::Binding;
use crate::shapes::Shape;
use crate::styles::{IntoButtonStyle, IntoTextFieldStyle, IntoToggleStyle};
use crate::text::Text;

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
        Self { ptr: NonNull::new(ptr).expect("elegence: the shim returned no view") }
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

// --- Modifier options ---

/// What a view is to screen readers, when the automatic choice isn't right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessibilityRole {
    Group,
    Label,
    Heading,
    Button,
    Link,
    Image,
    Toggle,
    Checkbox,
    Slider,
    Progress,
    TextField,
    List,
    ListItem,
    Tab,
    MenuItem,
}

/// Which side of its view a popover opens on (it flips if there isn't room).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PopoverEdge {
    #[default]
    Below,
    Above,
    Trailing,
    Leading,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopoverOptions {
    pub opens: PopoverEdge,
    /// false: the content reaches the panel's edges (images, lists).
    pub padded: bool,
}

impl Default for PopoverOptions {
    fn default() -> Self {
        Self { opens: PopoverEdge::Below, padded: true }
    }
}

impl PopoverOptions {
    pub fn opens(mut self, edge: impl Into<PopoverEdge>) -> Self {
        self.opens = edge.into();
        self
    }
    pub fn padded(mut self, padded: bool) -> Self {
        self.padded = padded;
        self
    }
}

/// A drop shadow: `radius` is the blur radius in points, (x, y) the offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub color: Color,
    pub radius: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for Shadow {
    fn default() -> Self {
        Self { color: Color::BLACK.opacity(0.33), radius: 8.0, x: 0.0, y: 0.0 }
    }
}

impl Shadow {
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }
    pub fn radius(mut self, radius: impl Number) -> Self {
        self.radius = radius.to_f32();
        self
    }
    pub fn x(mut self, x: impl Number) -> Self {
        self.x = x.to_f32();
        self
    }
    pub fn y(mut self, y: impl Number) -> Self {
        self.y = y.to_f32();
        self
    }
    pub(crate) fn c(self) -> sys::ElgShadow {
        sys::ElgShadow { color: self.color.c(), radius: self.radius, x: self.x, y: self.y }
    }
}

/// A frame: fixed `width`/`height`, or a range (`min_*`, `ideal_*`, `max_*`) that takes the
/// proposed size, clamped. Unset dimensions follow the content.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameOptions {
    pub width: Len,
    pub height: Len,
    pub min_width: Len,
    pub ideal_width: Len,
    pub max_width: Len,
    pub min_height: Len,
    pub ideal_height: Len,
    pub max_height: Len,
    pub alignment: Alignment,
}

macro_rules! len_setters {
    ($($field:ident),*) => {$(
        pub fn $field(mut self, length: impl Into<Len>) -> Self {
            self.$field = length.into();
            self
        }
    )*};
}

impl FrameOptions {
    len_setters!(width, height, min_width, ideal_width, max_width, min_height, ideal_height, max_height);
    pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
    fn c(self) -> sys::ElgFrameOptions {
        sys::ElgFrameOptions {
            width: self.width.c(),
            height: self.height.c(),
            min_width: self.min_width.c(),
            ideal_width: self.ideal_width.c(),
            max_width: self.max_width.c(),
            min_height: self.min_height.c(),
            ideal_height: self.ideal_height.c(),
            max_height: self.max_height.c(),
            alignment: self.alignment.c(),
        }
    }
}

/// Where a background or overlay view sits in the modified view's bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DecorationOptions {
    pub alignment: Alignment,
}

impl DecorationOptions {
    pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapOptions {
    /// 2: a double tap.
    pub count: i32,
}

impl Default for TapOptions {
    fn default() -> Self {
        Self { count: 1 }
    }
}

impl TapOptions {
    pub fn count(mut self, count: i32) -> Self {
        self.count = count;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AspectRatioOptions {
    /// width / height; `None`: the content's own (an image's).
    pub ratio: Option<f32>,
    pub mode: ContentMode,
}

impl Default for AspectRatioOptions {
    fn default() -> Self {
        Self { ratio: None, mode: ContentMode::Fit }
    }
}

impl AspectRatioOptions {
    pub fn ratio(mut self, ratio: impl Number) -> Self {
        self.ratio = Some(ratio.to_f32());
        self
    }
    pub fn mode(mut self, mode: impl Into<ContentMode>) -> Self {
        self.mode = mode.into();
        self
    }
}

/// What a view does with things dragged in from other apps. Handlers get what was dropped and
/// where (in the view's coordinates); returning false means it wasn't wanted.
#[derive(Default)]
pub struct DropOptions {
    pub files: Option<Box<dyn Fn(Vec<PathBuf>, Point) -> bool>>,
    pub text: Option<Box<dyn Fn(String, Point) -> bool>>,
    /// Set while a drag the view can take is over it (for a highlight).
    pub targeted: Option<Box<dyn Fn(bool)>>,
}

impl DropOptions {
    pub fn files(mut self, f: impl Fn(Vec<PathBuf>, Point) -> bool + 'static) -> Self {
        self.files = Some(Box::new(f));
        self
    }
    pub fn text(mut self, f: impl Fn(String, Point) -> bool + 'static) -> Self {
        self.text = Some(Box::new(f));
        self
    }
    pub fn targeted(mut self, f: impl Fn(bool) + 'static) -> Self {
        self.targeted = Some(Box::new(f));
        self
    }
}

// --- Modifiers ---

/// The modifiers every view has. In `elg!` a modifier with named arguments,
/// `.frame(max_width: .infinity)`, calls the `_with` form with a closure that sets them.
pub trait ViewExt: IntoView + Sized {
    // Layout.

    /// Space around the content, on every edge.
    fn padding(self, length: impl Number) -> View {
        let l = length.to_f32();
        self.into_view().modify(|v| unsafe { sys::elg_mod_padding(v, l, l, l, l) })
    }
    /// Space on some edges: `.padding_edges(Edges::HORIZONTAL, 12)`.
    fn padding_edges(self, edges: impl Into<Edges>, length: impl Number) -> View {
        let (edges, l) = (edges.into(), length.to_f32());
        let on = |edge| if edges.contains(edge) { l } else { 0.0 };
        let insets = EdgeInsets { top: on(Edges::TOP), leading: on(Edges::LEADING), bottom: on(Edges::BOTTOM), trailing: on(Edges::TRAILING) };
        self.padding_insets(insets)
    }
    fn padding_insets(self, insets: impl Into<EdgeInsets>) -> View {
        let i = insets.into();
        self.into_view().modify(|v| unsafe { sys::elg_mod_padding(v, i.top, i.leading, i.bottom, i.trailing) })
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
        self.into_view().modify(|v| unsafe { sys::elg_mod_frame(v, &o) })
    }
    /// A fixed size that follows state, laid out again whenever it changes.
    fn frame_size(self, size: impl IntoProp<Size>, alignment: impl Into<Alignment>) -> View {
        let a = alignment.into().c();
        let view = self.into_view();
        with_prop(size.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_frame_size(v, p, a) }))
    }
    /// Proposes nothing on the given axes, so the content takes its ideal size.
    fn fixed_size(self) -> View {
        self.fixed_size_axes(true, true)
    }
    fn fixed_size_axes(self, horizontal: bool, vertical: bool) -> View {
        self.into_view().modify(|v| unsafe { sys::elg_mod_fixed_size(v, horizontal, vertical) })
    }
    /// Sizes the view to an aspect ratio (width / height) within what's proposed.
    fn aspect_ratio(self, ratio: impl Number, mode: impl Into<ContentMode>) -> View {
        self.aspect_ratio_with(|o| o.ratio(ratio).mode(mode))
    }
    /// The view's own aspect ratio (an image's): `.aspect_ratio(mode: .fill)` in `elg!`.
    fn aspect_ratio_with(self, options: impl FnOnce(AspectRatioOptions) -> AspectRatioOptions) -> View {
        let o = options(AspectRatioOptions::default());
        self.into_view()
            .modify(|v| unsafe { sys::elg_mod_aspect_ratio(v, o.ratio.is_some(), o.ratio.unwrap_or(0.0), o.mode as i32) })
    }
    /// Raster images: take whatever size they're offered. Panics on other views.
    fn resizable(self) -> View {
        self.into_view().modify(|v| unsafe { sys::elg_mod_resizable(v) })
    }
    /// Stacks give space to higher priorities first. Default 0.
    fn layout_priority(self, priority: impl Number) -> View {
        let p = priority.to_f32();
        self.into_view().modify(|v| unsafe { sys::elg_mod_layout_priority(v, p) })
    }
    /// Stacking order among siblings: higher draws in front and is hit first.
    fn z_index(self, z: impl IntoProp<f32>) -> View {
        let view = self.into_view();
        with_prop(z.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_z_index(v, p) }))
    }
    /// Moves the content when drawn, without affecting layout (animated in animated updates).
    fn offset(self, x: impl IntoProp<f32>, y: impl IntoProp<f32>) -> View {
        let view = self.into_view();
        with_prop(x.into_prop(), |px| with_prop(y.into_prop(), |py| view.modify(|v| unsafe { sys::elg_mod_offset(v, px, py) })))
    }

    // Grids.

    /// A Grid cell that spans `count` columns.
    fn grid_cell_columns(self, count: i32) -> View {
        self.into_view().modify(|v| unsafe { sys::elg_mod_grid_cell_columns(v, count) })
    }
    /// Where this view sits in its grid cell, instead of its row's, column's, or grid's alignment.
    fn grid_cell_alignment(self, alignment: impl Into<Alignment>) -> View {
        let a = alignment.into().c();
        self.into_view().modify(|v| unsafe { sys::elg_mod_grid_cell_alignment(v, a) })
    }
    /// The horizontal alignment of every cell in this cell's Grid column.
    fn grid_column_alignment(self, alignment: impl Into<crate::geometry::HorizontalAlignment>) -> View {
        let a = alignment.into() as i32;
        self.into_view().modify(|v| unsafe { sys::elg_mod_grid_column_alignment(v, a) })
    }

    // Decoration.

    /// A color or gradient filling the view's bounds. Reactive arguments work.
    fn background(self, style: impl IntoProp<ShapeStyle>) -> View {
        let view = self.into_view();
        with_prop(style.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_background(v, p) }))
    }
    /// A color or gradient filling a shape behind the view: `.background_in(.accent, Capsule())`.
    fn background_in(self, style: impl IntoProp<ShapeStyle>, shape: impl Into<Shape>) -> View {
        let (view, shape) = (self.into_view(), shape.into());
        with_prop(style.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_background_shape(v, p, shape.as_ptr()) }))
    }
    /// A view behind this one, sized to it.
    fn background_view(self, background: impl IntoView) -> View {
        self.background_view_with(background, |o| o)
    }
    fn background_view_with(self, background: impl IntoView, options: impl FnOnce(DecorationOptions) -> DecorationOptions) -> View {
        let (b, a) = (background.into_view(), options(DecorationOptions::default()).alignment.c());
        self.into_view().modify(|v| unsafe { sys::elg_mod_background_view(v, b.into_raw(), a) })
    }
    /// A view in front of this one, sized to it.
    fn overlay(self, overlay: impl IntoView) -> View {
        self.overlay_with(overlay, |o| o)
    }
    fn overlay_with(self, overlay: impl IntoView, options: impl FnOnce(DecorationOptions) -> DecorationOptions) -> View {
        let (o, a) = (overlay.into_view(), options(DecorationOptions::default()).alignment.c());
        self.into_view().modify(|v| unsafe { sys::elg_mod_overlay(v, o.into_raw(), a) })
    }
    /// A border drawn inside the view's bounds.
    fn border(self, color: impl Into<Color>, width: impl Number) -> View {
        let (c, w) = (color.into().c(), width.to_f32());
        self.into_view().modify(|v| unsafe { sys::elg_mod_border(v, c, w) })
    }

    // Effects.

    /// Clips the content to the view's bounds.
    fn clipped(self) -> View {
        self.into_view().modify(|v| unsafe { sys::elg_mod_clipped(v) })
    }
    /// Clips the content to a shape filling the view's bounds.
    fn clip_shape(self, shape: impl Into<Shape>) -> View {
        let shape = shape.into();
        self.into_view().modify(|v| unsafe { sys::elg_mod_clip_shape(v, shape.as_ptr()) })
    }
    /// Shorthand for `clip_shape(RoundedRectangle(radius))`.
    fn corner_radius(self, radius: impl Number) -> View {
        let r = radius.to_f32();
        self.into_view().modify(|v| unsafe { sys::elg_mod_corner_radius(v, r) })
    }
    /// Fades the view as a group.
    fn opacity(self, opacity: impl IntoProp<f32>) -> View {
        let view = self.into_view();
        with_prop(opacity.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_opacity(v, p) }))
    }
    /// A blurred shadow of the view's content.
    fn shadow(self, shadow: Shadow) -> View {
        let s = shadow.c();
        self.into_view().modify(|v| unsafe { sys::elg_mod_shadow(v, s) })
    }
    /// `.shadow(radius: 4, y: 2)` in `elg!`.
    fn shadow_with(self, shadow: impl FnOnce(Shadow) -> Shadow) -> View {
        self.shadow(shadow(Shadow::default()))
    }
    /// Hit tests against this shape (filling the bounds) instead of the drawn content.
    fn content_shape(self, shape: impl Into<Shape>) -> View {
        let shape = shape.into();
        self.into_view().modify(|v| unsafe { sys::elg_mod_content_shape(v, shape.as_ptr()) })
    }
    /// false: the view and everything in it ignore the pointer.
    fn allows_hit_testing(self, enabled: bool) -> View {
        self.into_view().modify(|v| unsafe { sys::elg_mod_allows_hit_testing(v, enabled) })
    }

    // Events.

    /// Runs `action` on a tap. The innermost tap gesture wins.
    fn on_tap_gesture(self, action: impl FnMut() + 'static) -> View {
        self.on_tap_gesture_with(action, |o| o)
    }
    /// `.on_tap_gesture(open, count: 2)` in `elg!`.
    fn on_tap_gesture_with(self, action: impl FnMut() + 'static, options: impl FnOnce(TapOptions) -> TapOptions) -> View {
        let count = options(TapOptions::default()).count;
        let action = cffi::action(action);
        self.into_view().modify(|v| unsafe { sys::elg_mod_on_tap_gesture(v, action, count) })
    }
    /// Called with true when the pointer has rested on the view for the hover delay, and false
    /// when it leaves.
    fn on_hover(self, action: impl Fn(bool) + 'static) -> View {
        let action = cffi::consumer(action);
        self.into_view().modify(|v| unsafe { sys::elg_mod_on_hover(v, action, std::ptr::null()) })
    }
    fn on_hover_with(self, action: impl Fn(bool) + 'static, delay: impl FnOnce(HoverDelay) -> HoverDelay) -> View {
        let (action, delay) = (cffi::consumer(action), delay(HoverDelay::default()).c());
        self.into_view().modify(|v| unsafe { sys::elg_mod_on_hover(v, action, &delay) })
    }
    /// Files or text dragged from other apps can be dropped here.
    fn on_drop(self, options: DropOptions) -> View {
        let files = match options.files {
            Some(f) => cffi::closure(move |args, ret| unsafe {
                let arg = &*(args as *const sys::ElgDropFiles);
                let paths = std::slice::from_raw_parts(arg.paths, arg.count).iter().map(|p| PathBuf::from(p.as_str())).collect();
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
        let targeted = options.targeted.map(|f| cffi::consumer(move |b: bool| f(b))).unwrap_or(sys::ElgClosure::NONE);
        self.into_view().modify(|v| unsafe { sys::elg_mod_on_drop(v, files, text, targeted) })
    }
    /// `.on_drop(files: |paths, at| ...)` in `elg!`.
    fn on_drop_with(self, options: impl FnOnce(DropOptions) -> DropOptions) -> View {
        self.on_drop(options(DropOptions::default()))
    }

    // Accessibility.

    /// What it's called (an icon-only button needs one).
    fn accessibility_label(self, label: impl IntoProp<String>) -> View {
        let view = self.into_view();
        with_prop(label.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_accessibility_label(v, p) }))
    }
    /// What it holds, as words ("3 of 10").
    fn accessibility_value(self, value: impl IntoProp<String>) -> View {
        let view = self.into_view();
        with_prop(value.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_accessibility_value(v, p) }))
    }
    /// What happens if it's used, when the label doesn't say.
    fn accessibility_hint(self, hint: impl IntoProp<String>) -> View {
        let view = self.into_view();
        with_prop(hint.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_accessibility_hint(v, p) }))
    }
    /// Leaves it (and everything inside) out: decoration.
    fn accessibility_hidden(self, hidden: bool) -> View {
        self.into_view().modify(|v| unsafe { sys::elg_mod_accessibility_hidden(v, hidden) })
    }
    fn accessibility_role(self, role: impl Into<AccessibilityRole>) -> View {
        let role = role.into() as i32;
        self.into_view().modify(|v| unsafe { sys::elg_mod_accessibility_role(v, role) })
    }
    /// One element for everything inside, labeled with their text.
    fn accessibility_element(self) -> View {
        self.into_view().modify(|v| unsafe { sys::elg_mod_accessibility_element(v) })
    }
    /// What a screen reader's default action does, for views made clickable with gestures.
    fn accessibility_action(self, action: impl FnMut() + 'static) -> View {
        let action = cffi::action(action);
        self.into_view().modify(|v| unsafe { sys::elg_mod_accessibility_action(v, action) })
    }

    /// The hover delay for on_hover handlers and controls inside.
    fn hover_delay(self, delay: HoverDelay) -> View {
        let d = delay.c();
        self.into_view().modify(|v| unsafe { sys::elg_mod_hover_delay(v, d) })
    }
    fn hover_delay_with(self, delay: impl FnOnce(HoverDelay) -> HoverDelay) -> View {
        self.hover_delay(delay(HoverDelay::default()))
    }

    /// Pressing and dragging here moves the window (custom title bars).
    fn window_drag_area(self, enabled: bool) -> View {
        self.into_view().modify(|v| unsafe { sys::elg_mod_window_drag_area(v, enabled) })
    }

    // Popups.

    /// Right-click opens the menu `menu` builds, usually a `Menu { ... }`.
    fn context_menu<V: IntoView>(self, menu: impl Fn() -> V + 'static) -> View {
        let menu = cffi::closure(move |_, ret| unsafe { menu().into_view().write_into(ret) });
        self.into_view().modify(|v| unsafe { sys::elg_mod_context_menu(v, menu) })
    }
    /// A tooltip shown after the pointer rests on the view.
    fn help(self, text: impl Into<String>) -> View {
        let text: String = text.into();
        self.into_view().modify(|v| unsafe { sys::elg_mod_help(v, cffi::s(&text)) })
    }
    /// A panel of `content` beside the view, shown while `is_presented` is true.
    fn popover<V: IntoView>(self, is_presented: impl Into<Binding<bool>>, content: impl Fn() -> V + 'static) -> View {
        self.popover_with(is_presented, content, |o| o)
    }
    fn popover_with<V: IntoView>(
        self,
        is_presented: impl Into<Binding<bool>>,
        content: impl Fn() -> V + 'static,
        options: impl FnOnce(PopoverOptions) -> PopoverOptions,
    ) -> View {
        let o = options(PopoverOptions::default());
        let binding = is_presented.into().cpp();
        let content = cffi::closure(move |_, ret| unsafe { content().into_view().write_into(ret) });
        self.into_view().modify(|v| unsafe { sys::elg_mod_popover(v, binding, content, o.opens as i32, o.padded) })
    }

    // Keyboard and focus.

    /// true: the view takes keyboard focus. false: it and everything inside never do.
    fn focusable(self, focusable: bool) -> View {
        self.into_view().modify(|v| unsafe { sys::elg_mod_focusable(v, focusable) })
    }
    /// Two-way focus: true while the view has focus; setting it true focuses the view.
    fn focused(self, is_focused: impl Into<Binding<bool>>) -> View {
        let binding = is_focused.into().cpp();
        self.into_view().modify(|v| unsafe { sys::elg_mod_focused(v, binding) })
    }
    /// Runs `action` when `key` is pressed while the view (or something inside) has focus.
    fn on_key_press(self, key: impl Into<Key>, action: impl FnMut() + 'static) -> View {
        let (key, action) = (key.into() as i32, cffi::action(action));
        self.into_view().modify(|v| unsafe { sys::elg_mod_on_key_press(v, key, action) })
    }
    /// Sees every key press while focused; return true if handled.
    fn on_key_event(self, action: impl Fn(&KeyEvent) -> bool + 'static) -> View {
        let action = cffi::closure(move |args, ret| unsafe {
            let event = KeyEvent::from_c(&*(args as *const sys::ElgKeyEvent));
            *(ret as *mut bool) = action(&event);
        });
        self.into_view().modify(|v| unsafe { sys::elg_mod_on_key_event(v, action) })
    }

    // Environment: inherited by everything inside.

    fn font_weight(self, weight: impl Into<Weight>) -> View {
        let w = weight.into() as i32;
        self.into_view().modify(|v| unsafe { sys::elg_mod_font_weight(v, w) })
    }
    fn font(self, font: impl IntoProp<Font>) -> View {
        let view = self.into_view();
        with_prop(font.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_font(v, p) }))
    }
    /// The color of text and unfilled shapes. Also takes a theme color.
    fn foreground_style(self, color: impl IntoProp<Color>) -> View {
        let view = self.into_view();
        with_prop(color.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_foreground_style(v, p) }))
    }
    /// The accent color of controls.
    fn tint(self, color: impl IntoProp<Color>) -> View {
        let view = self.into_view();
        with_prop(color.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_tint(v, p) }))
    }
    /// Light or dark, for theme colors inside.
    fn color_scheme(self, scheme: impl IntoProp<ColorScheme>) -> View {
        let view = self.into_view();
        with_prop(scheme.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_color_scheme(v, p) }))
    }
    fn theme(self, theme: Theme) -> View {
        let t = theme.c();
        self.into_view().modify(|v| unsafe { sys::elg_mod_theme(v, &t) })
    }
    /// Controls inside stop responding and look disabled.
    fn disabled(self, disabled: impl IntoProp<bool>) -> View {
        let view = self.into_view();
        with_prop(disabled.into_prop(), |p| view.modify(|v| unsafe { sys::elg_mod_disabled(v, p) }))
    }
    /// The style of every Button inside.
    fn button_style(self, style: impl IntoButtonStyle) -> View {
        let style = style.into_button_style();
        self.into_view().modify(|v| unsafe { sys::elg_mod_button_style(v, style.as_ptr()) })
    }
    /// The style of every Toggle inside.
    fn toggle_style(self, style: impl IntoToggleStyle) -> View {
        let style = style.into_toggle_style();
        self.into_view().modify(|v| unsafe { sys::elg_mod_toggle_style(v, style.as_ptr()) })
    }
    /// The style of every TextField inside.
    fn text_field_style(self, style: impl IntoTextFieldStyle) -> View {
        let style = style.into_text_field_style();
        self.into_view().modify(|v| unsafe { sys::elg_mod_text_field_style(v, style.as_ptr()) })
    }
    /// Sets any environment key: `.environment::<Compact>(true)`.
    fn environment<K: EnvironmentKey>(self, value: impl IntoProp<K::Value>) -> View {
        let value = value.into_prop();
        crate::modifier::environment_modifier(self.into_view(), move |environment| K::write(environment, value.clone()))
    }

    // Animation.

    /// Changes inside this view animate with `animation` whenever `value` (a plain value, State,
    /// or closure) changed in the same update.
    fn animation(self, animation: impl Into<Animation>, value: impl crate::animation::AnimationValue) -> View {
        let animation = animation.into();
        let watch = cffi::producer(value.watcher());
        self.into_view().modify(|v| unsafe { sys::elg_mod_animation(v, animation.as_ptr(), watch) })
    }
    /// How this view enters and leaves when an If/ForEach adds or removes it with an animation.
    fn transition(self, transition: impl Into<Transition>) -> View {
        let t = transition.into();
        self.into_view().modify(|v| unsafe { sys::elg_mod_transition(v, t.as_ptr()) })
    }
    /// false: this view and everything inside snap to new positions and sizes.
    fn animates_layout(self, enabled: bool) -> View {
        self.into_view().modify(|v| unsafe { sys::elg_mod_animates_layout(v, enabled) })
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
