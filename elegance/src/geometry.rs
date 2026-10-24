//! Points, sizes, rectangles, alignment, and lengths.

use std::ops::BitOr;

use elegance_sys as sys;

/// `f32::INFINITY`, for flexible frames: `.frame(max_width: .infinity)`.
pub const INFINITY: f32 = f32::INFINITY;

/// The spacing stacks use when none is given.
pub const DEFAULT_STACK_SPACING: f32 = 8.0;

/// Any number used as a length in points.
pub trait Number: Copy {
    fn to_f32(self) -> f32;
}

macro_rules! numbers {
    ($($t:ty),*) => {$(
        impl Number for $t {
            fn to_f32(self) -> f32 {
                self as f32
            }
        }
    )*};
}
numbers!(f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl Number for crate::dot::infinity {
    fn to_f32(self) -> f32 {
        f32::INFINITY
    }
}

/// An optional length (`OptionalLength`): unset, or any number of points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Len(pub Option<f32>);

impl<N: Number> From<N> for Len {
    fn from(n: N) -> Self {
        Len(Some(n.to_f32()))
    }
}

impl From<Option<f32>> for Len {
    fn from(n: Option<f32>) -> Self {
        Len(n)
    }
}

impl Len {
    pub(crate) fn c(self) -> sys::ElgOptLen {
        sys::ElgOptLen { has: self.0.is_some(), value: self.0.unwrap_or(0.0) }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub(crate) fn c(self) -> sys::ElgPoint {
        sys::ElgPoint { x: self.x, y: self.y }
    }
    pub(crate) fn from_c(p: sys::ElgPoint) -> Self {
        Self { x: p.x, y: p.y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
    pub(crate) fn c(self) -> sys::ElgSize {
        sys::ElgSize { width: self.width, height: self.height }
    }
    pub(crate) fn from_c(s: sys::ElgSize) -> Self {
        Self { width: s.width, height: s.height }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
    pub fn min_x(&self) -> f32 {
        self.x
    }
    pub fn min_y(&self) -> f32 {
        self.y
    }
    pub fn max_x(&self) -> f32 {
        self.x + self.width
    }
    pub fn max_y(&self) -> f32 {
        self.y + self.height
    }
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }
    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
    pub fn inset(&self, amount: f32) -> Rect {
        Rect::new(
            self.x + amount,
            self.y + amount,
            (self.width - 2.0 * amount).max(0.0),
            (self.height - 2.0 * amount).max(0.0),
        )
    }
    pub(crate) fn c(self) -> sys::ElgRect {
        sys::ElgRect { x: self.x, y: self.y, width: self.width, height: self.height }
    }
    pub(crate) fn from_c(r: sys::ElgRect) -> Self {
        Self { x: r.x, y: r.y, width: r.width, height: r.height }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    pub const fn all(radius: f32) -> Self {
        Self { top_left: radius, top_right: radius, bottom_right: radius, bottom_left: radius }
    }
    pub(crate) fn array(self) -> [f32; 4] {
        [self.top_left, self.top_right, self.bottom_right, self.bottom_left]
    }
}

impl<N: Number> From<N> for CornerRadii {
    fn from(radius: N) -> Self {
        CornerRadii::all(radius.to_f32())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeInsets {
    pub top: f32,
    pub leading: f32,
    pub bottom: f32,
    pub trailing: f32,
}

impl EdgeInsets {
    pub const fn all(length: f32) -> Self {
        Self { top: length, leading: length, bottom: length, trailing: length }
    }
    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self { top: vertical, leading: horizontal, bottom: vertical, trailing: horizontal }
    }
    pub fn top(mut self, length: impl Number) -> Self {
        self.top = length.to_f32();
        self
    }
    pub fn leading(mut self, length: impl Number) -> Self {
        self.leading = length.to_f32();
        self
    }
    pub fn bottom(mut self, length: impl Number) -> Self {
        self.bottom = length.to_f32();
        self
    }
    pub fn trailing(mut self, length: impl Number) -> Self {
        self.trailing = length.to_f32();
        self
    }
    pub fn horizontal(self, length: impl Number) -> Self {
        self.leading(length).trailing(length)
    }
    pub fn vertical(self, length: impl Number) -> Self {
        self.top(length).bottom(length)
    }
    pub fn all_edges(self, length: impl Number) -> Self {
        self.horizontal(length).vertical(length)
    }
}

impl<N: Number> From<N> for EdgeInsets {
    fn from(length: N) -> Self {
        EdgeInsets::all(length.to_f32())
    }
}

/// A set of edges: `Edges::TOP | Edges::LEADING`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Edges(pub u32);

impl Edges {
    pub const TOP: Edges = Edges(1);
    pub const LEADING: Edges = Edges(2);
    pub const BOTTOM: Edges = Edges(4);
    pub const TRAILING: Edges = Edges(8);
    pub const HORIZONTAL: Edges = Edges(2 | 8);
    pub const VERTICAL: Edges = Edges(1 | 4);
    pub const ALL: Edges = Edges(15);

    pub fn contains(self, edge: Edges) -> bool {
        self.0 & edge.0 != 0
    }
}

impl BitOr for Edges {
    type Output = Edges;
    fn bitor(self, other: Edges) -> Edges {
        Edges(self.0 | other.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HorizontalAlignment {
    Leading,
    #[default]
    Center,
    Trailing,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VerticalAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Alignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

impl Alignment {
    pub const fn new(horizontal: HorizontalAlignment, vertical: VerticalAlignment) -> Self {
        Self { horizontal, vertical }
    }
    pub const CENTER: Self = Self::new(HorizontalAlignment::Center, VerticalAlignment::Center);
    pub const TOP: Self = Self::new(HorizontalAlignment::Center, VerticalAlignment::Top);
    pub const BOTTOM: Self = Self::new(HorizontalAlignment::Center, VerticalAlignment::Bottom);
    pub const LEADING: Self = Self::new(HorizontalAlignment::Leading, VerticalAlignment::Center);
    pub const TRAILING: Self = Self::new(HorizontalAlignment::Trailing, VerticalAlignment::Center);
    pub const TOP_LEADING: Self = Self::new(HorizontalAlignment::Leading, VerticalAlignment::Top);
    pub const TOP_TRAILING: Self = Self::new(HorizontalAlignment::Trailing, VerticalAlignment::Top);
    pub const BOTTOM_LEADING: Self = Self::new(HorizontalAlignment::Leading, VerticalAlignment::Bottom);
    pub const BOTTOM_TRAILING: Self = Self::new(HorizontalAlignment::Trailing, VerticalAlignment::Bottom);

    pub(crate) fn c(self) -> sys::ElgAlignment {
        sys::ElgAlignment { horizontal: self.horizontal as i32, vertical: self.vertical as i32 }
    }
}

/// A size offered by a parent during layout: `None` asks for the ideal size, 0 for the minimum,
/// and infinity for the maximum.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProposedSize {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl ProposedSize {
    pub const fn new(width: Option<f32>, height: Option<f32>) -> Self {
        Self { width, height }
    }
    pub const fn unspecified() -> Self {
        Self { width: None, height: None }
    }
    pub const fn zero() -> Self {
        Self { width: Some(0.0), height: Some(0.0) }
    }
    pub const fn infinite() -> Self {
        Self { width: Some(f32::INFINITY), height: Some(f32::INFINITY) }
    }
    pub fn replacing_unspecified(&self, fallback: Size) -> Size {
        Size::new(self.width.unwrap_or(fallback.width), self.height.unwrap_or(fallback.height))
    }
    pub(crate) fn c(self) -> sys::ElgProposedSize {
        sys::ElgProposedSize {
            has_width: self.width.is_some(),
            width: self.width.unwrap_or(0.0),
            has_height: self.height.is_some(),
            height: self.height.unwrap_or(0.0),
        }
    }
    pub(crate) fn from_c(p: sys::ElgProposedSize) -> Self {
        Self { width: p.has_width.then_some(p.width), height: p.has_height.then_some(p.height) }
    }
}

impl From<Size> for ProposedSize {
    fn from(size: Size) -> Self {
        Self { width: Some(size.width), height: Some(size.height) }
    }
}

/// A point in a view's bounds, in fractions of its size: (0, 0) is the top-leading corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitPoint {
    pub x: f32,
    pub y: f32,
}

impl Default for UnitPoint {
    fn default() -> Self {
        Self::CENTER
    }
}

impl UnitPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub const CENTER: Self = Self::new(0.5, 0.5);
    pub const TOP: Self = Self::new(0.5, 0.0);
    pub const BOTTOM: Self = Self::new(0.5, 1.0);
    pub const LEADING: Self = Self::new(0.0, 0.5);
    pub const TRAILING: Self = Self::new(1.0, 0.5);
    pub const TOP_LEADING: Self = Self::new(0.0, 0.0);
    pub const TOP_TRAILING: Self = Self::new(1.0, 0.0);
    pub const BOTTOM_LEADING: Self = Self::new(0.0, 1.0);
    pub const BOTTOM_TRAILING: Self = Self::new(1.0, 1.0);

    pub(crate) fn c(self) -> sys::ElgPoint {
        sys::ElgPoint { x: self.x, y: self.y }
    }
    pub(crate) fn from_c(p: sys::ElgPoint) -> Self {
        Self { x: p.x, y: p.y }
    }
}

/// How content with an aspect ratio fills space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContentMode {
    /// All of it shows: as large as fits.
    #[default]
    Fit,
    /// It covers the space: as small as covers it (overflowing on one axis; clip it).
    Fill,
}

/// Where a child of size `size` goes in `container` for `alignment`.
pub fn aligned_position(container: Rect, size: Size, alignment: Alignment) -> Point {
    Point::from_c(unsafe { sys::elg_aligned_position(container.c(), size.c(), alignment.c()) })
}
