//! Grids: views in columns and rows.
//!
//! `VGrid`/`HGrid` flow their children into tracks (columns of a VGrid, rows of an HGrid),
//! filling one line before the next; tracks are fixed, flexible, or adaptive (as many as fit).
//! `Grid` is a table of `GridRow`s whose columns are as wide as their widest cell.
//!
//! ```ignore
//! VGrid([GridItem::adaptive(150)], spacing: 12) {
//!     ForEach(games, id: |g| g.id) { game in GameCard(game: game.clone()) }
//! }
//!
//! Grid(alignment: .leading, horizontal_spacing: 24) {
//!     GridRow { Text("Game").bold() Text("Size").bold().grid_column_alignment(.trailing) }
//!     Divider()
//!     ForEach(games, id: |g| g.id) { game in
//!         GridRow { Text(game.name) Text(format!("{} GB", game.size)) }
//!     }
//!     GridRow { Text("Sizes include mods.").grid_cell_columns(2) }
//! }
//! ```

use elegance_sys as sys;

use crate::dsl::{ElgChildren, ElgNew};
use crate::error::ffi;
use crate::ffi as cffi;
use crate::geometry::{Alignment, Len, Number, ProposedSize, Rect, Size, VerticalAlignment};
use crate::layout::LayoutChild;
use crate::view::{raw_views, IntoView, View};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GridItemKind {
    /// Exactly `minimum` points.
    Fixed,
    /// A share of the space, within `[minimum, maximum]`.
    #[default]
    Flexible,
    /// As many tracks as fit in its share, each within `[minimum, maximum]`.
    Adaptive,
}

/// How a VGrid column (or HGrid row) is sized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridItem {
    pub kind: GridItemKind,
    pub minimum: f32,
    pub maximum: f32,
    /// Space after this track (and between an adaptive item's tracks); default: the grid's spacing.
    pub spacing: Len,
    /// Where views sit in this track's cells; default: the grid's alignment.
    pub alignment: Option<Alignment>,
}

impl Default for GridItem {
    fn default() -> Self {
        Self::flexible()
    }
}

impl GridItem {
    pub fn fixed(size: impl Number) -> Self {
        let size = size.to_f32();
        Self { kind: GridItemKind::Fixed, minimum: size, maximum: size, spacing: Len(None), alignment: None }
    }
    /// An equal share of what fixed items leave (at least 10 points); limit it with
    /// `.minimum()` / `.maximum()`.
    pub fn flexible() -> Self {
        Self { kind: GridItemKind::Flexible, minimum: 10.0, maximum: f32::INFINITY, spacing: Len(None), alignment: None }
    }
    /// As many tracks as fit at `minimum` or wider; they share the space exactly.
    pub fn adaptive(minimum: impl Number) -> Self {
        Self { kind: GridItemKind::Adaptive, minimum: minimum.to_f32(), ..Self::flexible() }
    }
    pub fn minimum(mut self, minimum: impl Number) -> Self {
        self.minimum = minimum.to_f32();
        self
    }
    pub fn maximum(mut self, maximum: impl Number) -> Self {
        self.maximum = maximum.to_f32();
        self
    }
    pub fn spacing(mut self, spacing: impl Into<Len>) -> Self {
        self.spacing = spacing.into();
        self
    }
    pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = Some(alignment.into());
        self
    }

    fn c(&self) -> sys::ElgGridItem {
        sys::ElgGridItem {
            kind: self.kind as i32,
            minimum: self.minimum,
            maximum: self.maximum,
            spacing: self.spacing.c(),
            has_alignment: self.alignment.is_some(),
            alignment: self.alignment.unwrap_or_default().c(),
        }
    }
}

/// One resolved track: its size, the space after it, and its cells' alignment (if set).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GridTrack {
    pub size: f32,
    pub spacing: f32,
    pub alignment: Option<Alignment>,
}

/// The tracks `items` make in `length` points (`None`: their ideal sizes); items without their
/// own spacing are `spacing` apart (`Len(None)`: the default). For custom layouts.
pub fn resolve_grid_tracks(items: &[GridItem], length: Option<f32>, spacing: impl Into<Len>) -> Vec<GridTrack> {
    let spacing = spacing.into();
    let c: Vec<sys::ElgGridItem> = items.iter().map(GridItem::c).collect();
    let mut tracks = Vec::new();
    let mut each = |args: *mut std::ffi::c_void, _: *mut std::ffi::c_void| unsafe {
        let t = &*(args as *const sys::ElgGridTrack);
        tracks.push(GridTrack {
            size: t.size,
            spacing: t.spacing,
            alignment: t.has_alignment.then(|| alignment_from_c(t.alignment)),
        });
    };
    ffi!(sys::elg_resolve_grid_tracks(c.as_ptr(), c.len(), length.is_some(), length.unwrap_or(0.0), spacing.c(), cffi::borrowed(&mut each)));
    tracks
}

/// The space `tracks` take, with the spacing between them.
pub fn grid_tracks_length(tracks: &[GridTrack]) -> f32 {
    let n = tracks.len();
    tracks.iter().enumerate().map(|(i, t)| t.size + if i + 1 < n { t.spacing } else { 0.0 }).sum()
}

fn alignment_from_c(a: sys::ElgAlignment) -> Alignment {
    use crate::geometry::HorizontalAlignment as H;
    let horizontal = match a.horizontal {
        0 => H::Leading,
        2 => H::Trailing,
        _ => H::Center,
    };
    let vertical = match a.vertical {
        0 => VerticalAlignment::Top,
        2 => VerticalAlignment::Bottom,
        _ => VerticalAlignment::Center,
    };
    Alignment::new(horizontal, vertical)
}

// The C description of a grid; borrows `items`.
fn desc(kind: i32, items: &[sys::ElgGridItem], alignment: Alignment, a: Len, b: Len) -> sys::ElgGridDesc {
    sys::ElgGridDesc { kind, items: items.as_ptr(), item_count: items.len(), alignment: alignment.c(), spacing_a: a.c(), spacing_b: b.c() }
}

fn grid_view(kind: i32, items: &[GridItem], alignment: Alignment, a: Len, b: Len, children: Vec<View>) -> View {
    let c: Vec<sys::ElgGridItem> = items.iter().map(GridItem::c).collect();
    let d = desc(kind, &c, alignment, a, b);
    let mut children = raw_views(children);
    View::from_raw(ffi!(sys::elg_grid(&d, children.as_mut_ptr(), children.len())))
}

macro_rules! line_grid {
    ($(#[$doc:meta])* $name:ident, $tracks:ident, $kind:expr, $layout:ident) => {
        $(#[$doc])*
        pub struct $name {
            $tracks: Vec<GridItem>,
            alignment: Alignment,
            spacing: Len,
            children: Vec<View>,
        }

        impl $name {
            pub fn new(tracks: impl IntoIterator<Item = GridItem>) -> Self {
                Self { $tracks: tracks.into_iter().collect(), alignment: Alignment::CENTER, spacing: Len(None), children: Vec::new() }
            }
            /// Of views in their cells (and of the tracks, when fixed tracks don't fill the grid).
            pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
                self.alignment = alignment.into();
                self
            }
            /// Between lines, and between tracks that don't set their own; default:
            /// [`crate::DEFAULT_STACK_SPACING`].
            pub fn spacing(mut self, spacing: impl Into<Len>) -> Self {
                self.spacing = spacing.into();
                self
            }
            pub fn child(mut self, child: impl IntoView) -> Self {
                self.children.push(child.into_view());
                self
            }
            pub fn children<V: IntoView>(mut self, children: impl IntoIterator<Item = V>) -> Self {
                self.children.extend(children.into_iter().map(IntoView::into_view));
                self
            }
        }

        impl ElgNew<()> for $name {
            type Out = $name;
            fn elg_new(_: ()) -> $name {
                $name::new([GridItem::flexible()])
            }
        }

        impl<I: IntoIterator<Item = GridItem>> ElgNew<(I,)> for $name {
            type Out = $name;
            fn elg_new((tracks,): (I,)) -> $name {
                $name::new(tracks)
            }
        }

        impl ElgChildren for $name {
            type Out = $name;
            fn elg_children(self, children: Vec<View>) -> $name {
                self.children(children)
            }
        }

        impl IntoView for $name {
            fn into_view(self) -> View {
                grid_view($kind, &self.$tracks, self.alignment, self.spacing, Len(None), self.children)
            }
        }

        /// The layout behind the grid, for custom layouts.
        #[derive(Clone, Debug, PartialEq)]
        pub struct $layout {
            pub $tracks: Vec<GridItem>,
            pub alignment: Alignment,
            pub spacing: Len,
        }

        impl $layout {
            pub fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild]) -> Size {
                let c: Vec<sys::ElgGridItem> = self.$tracks.iter().map(GridItem::c).collect();
                let d = desc($kind, &c, self.alignment, self.spacing, Len(None));
                Size::from_c(ffi!(sys::elg_grid_layout_size(&d, proposal.c(), children.as_ptr() as *const sys::LayoutChild, children.len())))
            }
            pub fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild]) {
                let c: Vec<sys::ElgGridItem> = self.$tracks.iter().map(GridItem::c).collect();
                let d = desc($kind, &c, self.alignment, self.spacing, Len(None));
                ffi!(sys::elg_grid_layout_place(&d, bounds.c(), proposal.c(), children.as_ptr() as *const sys::LayoutChild, children.len()))
            }
        }
    };
}

line_grid!(
    /// Children in columns, row after row (ForEach items are cells too). Rows are as tall as
    /// their tallest cell.
    VGrid, columns, 0, VGridLayout
);
line_grid!(
    /// Children in rows, column after column.
    HGrid, rows, 1, HGridLayout
);

/// A table of [`GridRow`]s: columns as wide as their widest cell, rows as tall as their tallest.
/// Other children (a Divider, a heading) take a row of their own, as wide as the grid.
#[derive(Default)]
pub struct Grid {
    alignment: Alignment,
    horizontal_spacing: Len,
    vertical_spacing: Len,
    children: Vec<View>,
}

impl Grid {
    pub fn new() -> Self {
        Self::default()
    }
    /// Of views in their cells.
    pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
    /// Between columns.
    pub fn horizontal_spacing(mut self, spacing: impl Into<Len>) -> Self {
        self.horizontal_spacing = spacing.into();
        self
    }
    /// Between rows.
    pub fn vertical_spacing(mut self, spacing: impl Into<Len>) -> Self {
        self.vertical_spacing = spacing.into();
        self
    }
    pub fn child(mut self, child: impl IntoView) -> Self {
        self.children.push(child.into_view());
        self
    }
}

impl ElgNew<()> for Grid {
    type Out = Grid;
    fn elg_new(_: ()) -> Grid {
        Grid::default()
    }
}

impl ElgChildren for Grid {
    type Out = Grid;
    fn elg_children(mut self, children: Vec<View>) -> Grid {
        self.children.extend(children);
        self
    }
}

impl IntoView for Grid {
    fn into_view(self) -> View {
        grid_view(2, &[], self.alignment, self.horizontal_spacing, self.vertical_spacing, self.children)
    }
}

/// The layout behind [`Grid`], for custom layouts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GridLayout {
    pub alignment: Alignment,
    pub horizontal_spacing: Len,
    pub vertical_spacing: Len,
}

impl GridLayout {
    pub fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild]) -> Size {
        let d = desc(2, &[], self.alignment, self.horizontal_spacing, self.vertical_spacing);
        Size::from_c(ffi!(sys::elg_grid_layout_size(&d, proposal.c(), children.as_ptr() as *const sys::LayoutChild, children.len())))
    }
    pub fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild]) {
        let d = desc(2, &[], self.alignment, self.horizontal_spacing, self.vertical_spacing);
        ffi!(sys::elg_grid_layout_place(&d, bounds.c(), proposal.c(), children.as_ptr() as *const sys::LayoutChild, children.len()))
    }
}

/// One row of a [`Grid`]: each child is a cell. Repeat rows with ForEach (a ForEach inside a
/// GridRow is one cell).
#[derive(Default)]
pub struct GridRow {
    alignment: Option<VerticalAlignment>,
    cells: Vec<View>,
}

impl GridRow {
    pub fn new() -> Self {
        Self::default()
    }
    /// Of this row's cells; default: the grid's.
    pub fn alignment(mut self, alignment: impl Into<VerticalAlignment>) -> Self {
        self.alignment = Some(alignment.into());
        self
    }
    pub fn cell(mut self, cell: impl IntoView) -> Self {
        self.cells.push(cell.into_view());
        self
    }
}

impl ElgNew<()> for GridRow {
    type Out = GridRow;
    fn elg_new(_: ()) -> GridRow {
        GridRow::default()
    }
}

impl ElgChildren for GridRow {
    type Out = GridRow;
    fn elg_children(mut self, children: Vec<View>) -> GridRow {
        self.cells.extend(children);
        self
    }
}

impl IntoView for GridRow {
    fn into_view(self) -> View {
        let alignment = self.alignment.map_or(-1, |a| a as i32);
        let mut cells = raw_views(self.cells);
        View::from_raw(ffi!(sys::elg_grid_row(alignment, cells.as_mut_ptr(), cells.len())))
    }
}
