//! Shapes: views that draw an outline filling their bounds.

use std::ptr::NonNull;

use elegance_sys as sys;

use crate::color::{Color, ShapeStyle};
use crate::dsl::ElgNew;
use crate::error::ffi;
use crate::ffi as cffi;
use crate::geometry::{CornerRadii, Number, Rect};
use crate::path::{FillRule, Path, StrokeStyle};
use crate::prop::{with_prop, IntoProp};

/// A shape view. It takes all the space it's offered.
pub struct Shape {
    ptr: NonNull<sys::Shape>,
}

impl Clone for Shape {
    fn clone(&self) -> Self {
        Self::from_raw(unsafe { sys::elg_shape_clone(self.as_ptr()) })
    }
}

impl Drop for Shape {
    fn drop(&mut self) {
        unsafe { sys::elg_shape_free(self.ptr.as_ptr()) }
    }
}

#[elegance_macros::elg_builder]
impl Shape {
    fn from_raw(ptr: *mut sys::Shape) -> Self {
        Self { ptr: NonNull::new(ptr).expect("elegance: no shape") }
    }
    pub(crate) fn as_ptr(&self) -> *const sys::Shape {
        self.ptr.as_ptr()
    }

    /// Fills the shape with a color or gradient (reactive arguments work).
    pub fn fill(self, style: impl IntoProp<ShapeStyle>) -> Shape {
        with_prop(style.into_prop(), |p| Self::from_raw(ffi!(sys::elg_shape_fill(self.as_ptr(), p))))
    }
    /// Strokes the outline, centered on it.
    pub fn stroke(self, color: impl IntoProp<Color>, line_width: impl Number) -> Shape {
        self.stroke_style(color, StrokeStyle::default().width(line_width.to_f32()))
    }
    pub fn stroke_style(self, color: impl IntoProp<Color>, style: StrokeStyle) -> Shape {
        let c = style.c();
        with_prop(color.into_prop(), |p| Self::from_raw(ffi!(sys::elg_shape_stroke(self.as_ptr(), p, &c))))
    }
    /// Strokes inside the outline, so the stroke stays within the bounds.
    pub fn stroke_border(self, color: impl IntoProp<Color>, line_width: impl Number) -> Shape {
        let w = line_width.to_f32();
        with_prop(color.into_prop(), |p| Self::from_raw(ffi!(sys::elg_shape_stroke_border(self.as_ptr(), p, w))))
    }
}

// The shape elements: `Rectangle()`, `RoundedRectangle(8)`, ... in `elg!`.

pub struct Rectangle {
    _private: (),
}
impl ElgNew<()> for Rectangle {
    type Out = Shape;
    fn elg_new(_: ()) -> Shape {
        Shape::from_raw(unsafe { sys::elg_rectangle() })
    }
}

pub struct RoundedRectangle {
    _private: (),
}
impl<R: Into<CornerRadii>> ElgNew<(R,)> for RoundedRectangle {
    type Out = Shape;
    fn elg_new((radii,): (R,)) -> Shape {
        let r = radii.into();
        Shape::from_raw(ffi!(sys::elg_rounded_rectangle(r.top_left, r.top_right, r.bottom_right, r.bottom_left)))
    }
}

pub struct Capsule {
    _private: (),
}
impl ElgNew<()> for Capsule {
    type Out = Shape;
    fn elg_new(_: ()) -> Shape {
        Shape::from_raw(unsafe { sys::elg_capsule() })
    }
}

pub struct Circle {
    _private: (),
}
impl ElgNew<()> for Circle {
    type Out = Shape;
    fn elg_new(_: ()) -> Shape {
        Shape::from_raw(unsafe { sys::elg_circle() })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PathShapeOptions {
    /// The part of the path's coordinates that fills the bounds (SVG viewBox); none: the path is
    /// in points.
    pub view_box: Option<Rect>,
    pub fill_rule: FillRule,
}

/// A fixed path as a shape.
pub struct PathShape {
    _private: (),
}
impl ElgNew<(Path,)> for PathShape {
    type Out = Shape;
    fn elg_new((path,): (Path,)) -> Shape {
        Self::elg_new((path, PathShapeOptions::default()))
    }
}
impl ElgNew<(Path, PathShapeOptions)> for PathShape {
    type Out = Shape;
    fn elg_new((path, options): (Path, PathShapeOptions)) -> Shape {
        let view_box = options.view_box.map(Rect::c);
        let view_box_ptr = view_box.as_ref().map_or(std::ptr::null(), |r| r as *const sys::ElgRect);
        Shape::from_raw(ffi!(sys::elg_path_shape(path.as_ptr(), view_box_ptr, options.fill_rule as i32)))
    }
}

/// A shape drawn by a function of its bounds.
pub struct CustomShape {
    _private: (),
}
impl<F: Fn(Rect) -> Path + 'static> ElgNew<(F,)> for CustomShape {
    type Out = Shape;
    fn elg_new((path,): (F,)) -> Shape {
        Self::elg_new((path, FillRule::NonZero))
    }
}
impl<F: Fn(Rect) -> Path + 'static, R: Into<FillRule>> ElgNew<(F, R)> for CustomShape {
    type Out = Shape;
    fn elg_new((path, rule): (F, R)) -> Shape {
        let path = cffi::closure(move |args, ret| unsafe {
            let bounds = Rect::from_c(*(args as *const sys::ElgRect));
            path(bounds).write_into(ret);
        });
        Shape::from_raw(ffi!(sys::elg_custom_shape(path, rule.into() as i32)))
    }
}

// The same as functions, for arguments: `.clip_shape(Circle())`, `.background_in(.accent, Capsule())`.

#[allow(non_snake_case)]
pub fn Rectangle() -> Shape {
    Rectangle::elg_new(())
}
#[allow(non_snake_case)]
pub fn RoundedRectangle(radii: impl Into<CornerRadii>) -> Shape {
    RoundedRectangle::elg_new((radii,))
}
#[allow(non_snake_case)]
pub fn Capsule() -> Shape {
    Capsule::elg_new(())
}
#[allow(non_snake_case)]
pub fn Circle() -> Shape {
    Circle::elg_new(())
}
#[allow(non_snake_case)]
pub fn PathShape(path: Path, options: PathShapeOptions) -> Shape {
    PathShape::elg_new((path, options))
}
#[allow(non_snake_case)]
pub fn CustomShape(path: impl Fn(Rect) -> Path + 'static, fill_rule: impl Into<FillRule>) -> Shape {
    CustomShape::elg_new((path, fill_rule))
}
