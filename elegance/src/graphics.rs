//! Immediate-mode drawing: [`Canvas`] and [`GraphicsContext`].

use std::marker::PhantomData;

use elegance_sys as sys;

use crate::color::{Color, ShapeStyle};
use crate::dsl::ElgNew;
use crate::error::ffi;
use crate::ffi as cffi;
use crate::geometry::{CornerRadii, Rect, Size};
use crate::path::{FillRule, Path, StrokeStyle};
use crate::view::View;

/// Draws into a canvas's display list, in the canvas's coordinates (points). Valid only during
/// the draw call.
pub struct GraphicsContext<'a> {
    ptr: *mut sys::GraphicsContext,
    _call: PhantomData<&'a mut ()>,
}

#[elegance_macros::elg_builder]
impl GraphicsContext<'_> {
    pub fn size(&self) -> Size {
        Size::from_c(unsafe { sys::elg_gc_size(self.ptr) })
    }
    /// Pixels per point.
    pub fn scale(&self) -> f32 {
        unsafe { sys::elg_gc_scale(self.ptr) }
    }
    /// Seconds on the frame clock, for animation.
    pub fn time(&self) -> f64 {
        unsafe { sys::elg_gc_time(self.ptr) }
    }
    /// Draw again next frame (continuous animation).
    pub fn request_frame(&mut self) {
        unsafe { sys::elg_gc_request_frame(self.ptr) }
    }
    pub fn fill_rect(&mut self, rect: Rect, color: impl Into<Color>, radii: impl Into<CornerRadii>) {
        let radii = radii.into().array();
        ffi!(sys::elg_gc_fill_rect(self.ptr, rect.c(), color.into().c(), radii.as_ptr()));
    }
    pub fn stroke_rect(&mut self, rect: Rect, color: impl Into<Color>, width: f32, radii: impl Into<CornerRadii>) {
        let radii = radii.into().array();
        ffi!(sys::elg_gc_stroke_rect(self.ptr, rect.c(), color.into().c(), width, radii.as_ptr()));
    }
    /// A filled rect with a border, in one draw.
    pub fn draw_rect(&mut self, rect: Rect, radii: impl Into<CornerRadii>, fill: impl Into<Color>, border: impl Into<Color>, border_width: f32) {
        let radii = radii.into().array();
        ffi!(sys::elg_gc_draw_rect(self.ptr, rect.c(), radii.as_ptr(), fill.into().c(), border.into().c(), border_width));
    }
    pub fn fill_path(&mut self, path: &Path, style: impl Into<ShapeStyle>, rule: impl Into<FillRule>) {
        let rule = rule.into() as i32;
        style.into().with_c(|s| ffi!(sys::elg_gc_fill_path(self.ptr, path.as_ptr(), s, rule)));
    }
    pub fn stroke_path(&mut self, path: &Path, color: impl Into<Color>, style: &StrokeStyle) {
        let s = style.c();
        ffi!(sys::elg_gc_stroke_path(self.ptr, path.as_ptr(), color.into().c(), &s));
    }
    /// Clips what's drawn until the matching pop_clip.
    pub fn push_clip(&mut self, rect: Rect, radii: impl Into<CornerRadii>) {
        let radii = radii.into().array();
        ffi!(sys::elg_gc_push_clip(self.ptr, rect.c(), radii.as_ptr()));
    }
    pub fn pop_clip(&mut self) {
        ffi!(sys::elg_gc_pop_clip(self.ptr));
    }
}

/// Immediate-mode drawing: `draw` runs every time the view is drawn. Takes the proposed size.
///
/// ```ignore
/// Canvas(|gc| gc.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0), Color::hex(0xFF9F0A), 6))
/// ```
pub struct Canvas {
    _private: (),
}

impl<F: Fn(&mut GraphicsContext) + 'static> ElgNew<(F,)> for Canvas {
    type Out = View;
    fn elg_new((draw,): (F,)) -> View {
        let draw = cffi::closure(move |args, _| {
            let mut context = GraphicsContext { ptr: args as *mut sys::GraphicsContext, _call: PhantomData };
            draw(&mut context);
        });
        View::from_raw(ffi!(sys::elg_canvas(draw)))
    }
}

#[allow(non_snake_case)]
pub fn Canvas(draw: impl Fn(&mut GraphicsContext) + 'static) -> View {
    Canvas::elg_new((draw,))
}
