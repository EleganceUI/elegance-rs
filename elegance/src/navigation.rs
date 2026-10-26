//! Split views and navigation stacks.

use std::rc::Rc;

use elegance_sys as sys;

use crate::dsl::{ElgChildren, ElgNew};
use crate::error::ffi;
use crate::ffi as cffi;
use crate::geometry::Axis;
use crate::prop::{IntoProp, Prop};
use crate::reactive::{Binding, IntoBinding};
use crate::views::{push_items, untracked, ItemKey};
use crate::view::{raw_views, IntoView, View};

/// Panes side by side with draggable dividers between them.
#[derive(Default)]
pub struct HSplitView {
    panes: Vec<View>,
}

/// Panes stacked top to bottom with draggable dividers.
#[derive(Default)]
pub struct VSplitView {
    panes: Vec<View>,
}

macro_rules! split_view {
    ($t:ident, $axis:expr) => {
        impl $t {
            pub fn new() -> Self {
                Self::default()
            }
            pub fn pane(mut self, pane: impl IntoView) -> Self {
                self.panes.push(pane.into_view());
                self
            }
        }
        impl ElgNew<()> for $t {
            type Out = $t;
            fn elg_new(_: ()) -> $t {
                <$t>::default()
            }
        }
        impl ElgChildren for $t {
            type Out = $t;
            fn elg_children(mut self, children: Vec<View>) -> $t {
                self.panes.extend(children);
                self
            }
        }
        impl IntoView for $t {
            fn into_view(self) -> View {
                let mut panes = raw_views(self.panes);
                View::from_raw(ffi!(sys::elg_split_view($axis as i32, panes.as_mut_ptr(), panes.len())))
            }
        }
    };
}

split_view!(HSplitView, Axis::Horizontal);
split_view!(VSplitView, Axis::Vertical);

/// A column's width range, in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColumnWidth {
    pub min: f32,
    pub ideal: f32,
    pub max: f32,
}

impl ColumnWidth {
    pub const fn new(min: f32, ideal: f32, max: f32) -> Self {
        Self { min, ideal, max }
    }
    fn c(self) -> sys::ElgColumnWidth {
        sys::ElgColumnWidth { min: self.min, ideal: self.ideal, max: self.max }
    }
}

/// A sidebar and a detail (or sidebar, content, and detail) in resizable columns. Its block
/// holds two or three views.
pub struct NavigationSplitView {
    columns: Vec<View>,
    sidebar_visible: Option<Binding<bool>>,
    sidebar_width: ColumnWidth,
    content_width: ColumnWidth,
}

impl Default for NavigationSplitView {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            sidebar_visible: None,
            sidebar_width: ColumnWidth::new(180.0, 240.0, 360.0),
            content_width: ColumnWidth::new(220.0, 300.0, 480.0),
        }
    }
}

impl NavigationSplitView {
    pub fn new(sidebar: impl IntoView, detail: impl IntoView) -> Self {
        Self { columns: vec![sidebar.into_view(), detail.into_view()], ..Default::default() }
    }
    pub fn three_column(sidebar: impl IntoView, content: impl IntoView, detail: impl IntoView) -> Self {
        Self { columns: vec![sidebar.into_view(), content.into_view(), detail.into_view()], ..Default::default() }
    }
    /// Shows and hides the sidebar (and follows the user doing so).
    pub fn sidebar_visible(mut self, visible: impl IntoBinding<bool>) -> Self {
        self.sidebar_visible = Some(visible.into_binding());
        self
    }
    pub fn sidebar_width(mut self, width: ColumnWidth) -> Self {
        self.sidebar_width = width;
        self
    }
    /// The three-column form only.
    pub fn content_width(mut self, width: ColumnWidth) -> Self {
        self.content_width = width;
        self
    }
}

impl ElgNew<()> for NavigationSplitView {
    type Out = NavigationSplitView;
    fn elg_new(_: ()) -> Self {
        Self::default()
    }
}

impl ElgChildren for NavigationSplitView {
    type Out = NavigationSplitView;
    fn elg_children(mut self, children: Vec<View>) -> Self {
        self.columns.extend(children);
        self
    }
}

impl IntoView for NavigationSplitView {
    fn into_view(self) -> View {
        let mut columns = self.columns;
        let (sidebar, content, detail) = match columns.len() {
            2 => {
                let detail = columns.pop().unwrap();
                (columns.pop().unwrap(), None, detail)
            }
            3 => {
                let detail = columns.pop().unwrap();
                let content = columns.pop().unwrap();
                (columns.pop().unwrap(), Some(content), detail)
            }
            n => panic!("elegance: NavigationSplitView takes two or three columns (got {n})"),
        };
        let visible = self.sidebar_visible.map_or(std::ptr::null_mut(), Binding::<bool>::cpp);
        View::from_raw(ffi!(sys::elg_navigation_split_view(
            sidebar.into_raw(),
            content.map_or(std::ptr::null_mut(), View::into_raw),
            detail.into_raw(),
            visible,
            self.sidebar_width.c(),
            self.content_width.c()
        )))
    }
}

/// A stack of pages following `path`: the root, then one page per route. Pushing a route onto
/// the path shows its page; the back button (or Esc) pops it.
///
/// ```ignore
/// NavigationStack(path,destination: |route| Detail(route: route.clone())) { Home() }
/// ```
pub struct NavigationStack<R: 'static> {
    path: Binding<Vec<R>>,
    root: View,
    destination: Option<Rc<dyn Fn(&R) -> View>>,
    root_title: Prop<String>,
    title: Option<Rc<dyn Fn(&R) -> String>>,
    shows_bar: bool,
}

impl<R: PartialEq + Clone + 'static> NavigationStack<R> {
    pub fn new<V: IntoView>(path: impl IntoBinding<Vec<R>>, root: impl IntoView, destination: impl Fn(&R) -> V + 'static) -> Self {
        Self::with_path(path).root(root).destination(destination)
    }
    fn with_path(path: impl IntoBinding<Vec<R>>) -> Self {
        Self { path: path.into_binding(), root: View::empty(), destination: None, root_title: Prop::Value(String::new()), title: None, shows_bar: true }
    }
    pub fn root(mut self, root: impl IntoView) -> Self {
        self.root = root.into_view();
        self
    }
    /// The page for a route.
    pub fn destination<V: IntoView>(mut self, destination: impl Fn(&R) -> V + 'static) -> Self {
        self.destination = Some(Rc::new(move |route| destination(route).into_view()));
        self
    }
    /// The bar's title on the root page.
    pub fn root_title(mut self, title: impl IntoProp<String>) -> Self {
        self.root_title = title.into_prop();
        self
    }
    /// The bar's title on a route's page.
    pub fn title(mut self, title: impl Fn(&R) -> String + 'static) -> Self {
        self.title = Some(Rc::new(title));
        self
    }
    /// The bar with the back button and titles.
    pub fn shows_bar(mut self, shows: bool) -> Self {
        self.shows_bar = shows;
        self
    }
}

impl<R: PartialEq + Clone + 'static, B: IntoBinding<Vec<R>>> ElgNew<(B,)> for NavigationStack<R> {
    type Out = NavigationStack<R>;
    fn elg_new((path,): (B,)) -> Self {
        Self::with_path(path)
    }
}

impl<R: PartialEq + Clone + 'static> ElgChildren for NavigationStack<R> {
    type Out = NavigationStack<R>;
    fn elg_children(self, children: Vec<View>) -> Self {
        self.root(crate::dsl::single(children, "NavigationStack"))
    }
}

impl<R: PartialEq + Clone + 'static> IntoView for NavigationStack<R> {
    fn into_view(self) -> View {
        let path = self.path;
        let destination = self.destination.expect("elegance: NavigationStack needs a destination");
        let pages = cffi::closure(move |_, ret| {
            let routes = path.get(); // a tracked read: the stack follows the path
            push_items(ret, routes, &|index, _| ItemKey::Int(index as i64), &destination);
        });
        let (root_title, title) = (self.root_title, self.title);
        let titles = cffi::closure(move |_, ret| {
            let mut names = vec![match &root_title {
                Prop::Value(v) => v.clone(),
                Prop::Fn(f) => f(),
                _ => String::new(),
            }];
            for route in path.get() {
                names.push(title.as_ref().map(|t| t(&route)).unwrap_or_default());
            }
            for name in &names {
                unsafe { sys::elg_strings_push(ret as *mut sys::Strings, cffi::s(name)) };
            }
        });
        let pop = cffi::closure(move |_, _| {
            let mut routes = untracked(path);
            if routes.pop().is_some() {
                path.set(routes);
            }
        });
        View::from_raw(ffi!(sys::elg_navigation_stack(self.root.into_raw(), pages, titles, pop, self.shows_bar)))
    }
}
