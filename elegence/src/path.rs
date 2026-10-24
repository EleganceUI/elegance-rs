//! Vector paths and stroke styles.

use std::ptr::NonNull;

use elegence_sys as sys;

use crate::error::{self, ffi, Error};
use crate::ffi as cffi;
use crate::geometry::{Point, Rect};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FillRule {
    /// Inside if the outline winds around the point at all (overlaps stay filled).
    #[default]
    NonZero,
    /// Inside if a ray from the point crosses the outline an odd number of times (holes).
    EvenOdd,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub width: f32,
    pub cap: LineCap,
    pub join: LineJoin,
    /// Miter joins longer than this many widths become bevels.
    pub miter_limit: f32,
    /// On, off, on, off... lengths; empty: solid.
    pub dash: Vec<f32>,
    /// How far into the pattern the stroke starts.
    pub dash_phase: f32,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self { width: 1.0, cap: LineCap::Butt, join: LineJoin::Miter, miter_limit: 4.0, dash: Vec::new(), dash_phase: 0.0 }
    }
}

impl StrokeStyle {
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    pub fn cap(mut self, cap: impl Into<LineCap>) -> Self {
        self.cap = cap.into();
        self
    }
    pub fn join(mut self, join: impl Into<LineJoin>) -> Self {
        self.join = join.into();
        self
    }
    pub fn miter_limit(mut self, limit: f32) -> Self {
        self.miter_limit = limit;
        self
    }
    pub fn dash(mut self, dash: impl Into<Vec<f32>>) -> Self {
        self.dash = dash.into();
        self
    }
    pub fn dash_phase(mut self, phase: f32) -> Self {
        self.dash_phase = phase;
        self
    }
    /// The C form; borrows `dash`.
    pub(crate) fn c(&self) -> sys::ElgStrokeStyle {
        sys::ElgStrokeStyle {
            width: self.width,
            cap: self.cap as i32,
            join: self.join as i32,
            miter_limit: self.miter_limit,
            dash: self.dash.as_ptr(),
            dash_count: self.dash.len(),
            dash_phase: self.dash_phase,
        }
    }
}

/// A flattened subpath.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Polyline {
    pub points: Vec<Point>,
    pub closed: bool,
}

/// An outline made of lines and curves, in points.
pub struct Path {
    ptr: NonNull<sys::Path>,
}

impl Clone for Path {
    fn clone(&self) -> Self {
        Self::from_raw(unsafe { sys::elg_path_clone(self.as_ptr()) })
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        unsafe { sys::elg_path_free(self.ptr.as_ptr()) }
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sys::elg_path_eq(self.as_ptr(), other.as_ptr()) }
    }
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Path").field("bounds", &self.bounds()).finish()
    }
}

impl Path {
    pub fn new() -> Self {
        Self::from_raw(unsafe { sys::elg_path_new() })
    }

    fn from_raw(ptr: *mut sys::Path) -> Self {
        Self { ptr: NonNull::new(ptr).expect("elegence: no path") }
    }
    pub(crate) fn as_ptr(&self) -> *const sys::Path {
        self.ptr.as_ptr()
    }
    fn as_mut_ptr(&mut self) -> *mut sys::Path {
        self.ptr.as_ptr()
    }

    /// Writes this path into a closure's C++ Path result.
    pub(crate) unsafe fn write_into(&self, ret: *mut std::ffi::c_void) {
        sys::elg_path_assign(ret as *mut sys::Path, self.as_ptr());
    }

    pub fn move_to(&mut self, point: Point) -> &mut Self {
        unsafe { sys::elg_path_move_to(self.as_mut_ptr(), point.c()) };
        self
    }
    pub fn line_to(&mut self, point: Point) -> &mut Self {
        unsafe { sys::elg_path_line_to(self.as_mut_ptr(), point.c()) };
        self
    }
    pub fn quad_to(&mut self, control: Point, end: Point) -> &mut Self {
        unsafe { sys::elg_path_quad_to(self.as_mut_ptr(), control.c(), end.c()) };
        self
    }
    pub fn cubic_to(&mut self, control1: Point, control2: Point, end: Point) -> &mut Self {
        unsafe { sys::elg_path_cubic_to(self.as_mut_ptr(), control1.c(), control2.c(), end.c()) };
        self
    }
    /// An SVG-style elliptical arc to `end`.
    pub fn arc_to(&mut self, rx: f32, ry: f32, rotation_degrees: f32, large_arc: bool, sweep: bool, end: Point) -> &mut Self {
        unsafe { sys::elg_path_arc_to(self.as_mut_ptr(), rx, ry, rotation_degrees, large_arc, sweep, end.c()) };
        self
    }
    pub fn close(&mut self) -> &mut Self {
        unsafe { sys::elg_path_close(self.as_mut_ptr()) };
        self
    }
    pub fn add_rect(&mut self, rect: Rect) -> &mut Self {
        unsafe { sys::elg_path_add_rect(self.as_mut_ptr(), rect.c()) };
        self
    }
    pub fn add_rounded_rect(&mut self, rect: Rect, radius: f32) -> &mut Self {
        unsafe { sys::elg_path_add_rounded_rect(self.as_mut_ptr(), rect.c(), radius) };
        self
    }
    pub fn add_ellipse(&mut self, rect: Rect) -> &mut Self {
        unsafe { sys::elg_path_add_ellipse(self.as_mut_ptr(), rect.c()) };
        self
    }
    pub fn add_circle(&mut self, center: Point, radius: f32) -> &mut Self {
        unsafe { sys::elg_path_add_circle(self.as_mut_ptr(), center.c(), radius) };
        self
    }
    pub fn add_path(&mut self, other: &Path) -> &mut Self {
        unsafe { sys::elg_path_add_path(self.as_mut_ptr(), other.as_ptr()) };
        self
    }

    /// Parses SVG path data (`M 0 0 L 10 10 Z`).
    pub fn svg(data: &str) -> Result<Path, Error> {
        let ptr = unsafe { sys::elg_path_svg(cffi::s(data)) };
        error::result()?;
        Ok(Self::from_raw(ptr))
    }

    pub fn is_empty(&self) -> bool {
        unsafe { sys::elg_path_empty(self.as_ptr()) }
    }
    pub fn bounds(&self) -> Rect {
        Rect::from_c(unsafe { sys::elg_path_bounds(self.as_ptr()) })
    }
    pub fn contains(&self, point: Point, rule: impl Into<FillRule>) -> bool {
        unsafe { sys::elg_path_contains(self.as_ptr(), point.c(), rule.into() as i32) }
    }
    /// Scaled, then moved.
    pub fn transformed(&self, scale_x: f32, scale_y: f32, dx: f32, dy: f32) -> Path {
        Self::from_raw(unsafe { sys::elg_path_transformed(self.as_ptr(), scale_x, scale_y, dx, dy) })
    }
    /// The subpaths as polylines, curves split until within `tolerance` points.
    pub fn flatten(&self, tolerance: f32) -> Vec<Polyline> {
        let mut lines = Vec::new();
        let mut each = |args: *mut std::ffi::c_void, _: *mut std::ffi::c_void| unsafe {
            let line = &*(args as *const sys::ElgPolyline);
            let points = std::slice::from_raw_parts(line.points, line.count).iter().map(|p| Point::from_c(*p)).collect();
            lines.push(Polyline { points, closed: line.closed });
        };
        ffi!(sys::elg_path_flatten(self.as_ptr(), tolerance, cffi::borrowed(&mut each)));
        lines
    }
    /// The outline of this path stroked with `style`, as a fillable path.
    pub fn stroked(&self, style: &StrokeStyle, tolerance: f32) -> Path {
        let c = style.c();
        Self::from_raw(ffi!(sys::elg_path_stroked(self.as_ptr(), &c, tolerance)))
    }
}
