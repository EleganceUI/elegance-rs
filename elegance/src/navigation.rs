//! Split views and navigation stacks.

use std::rc::Rc;

use elegance_sys as sys;

use crate::animation::{Animation, Transition};
use crate::color::{Color, ThemeColor};
use crate::dsl::{ElgChildren, ElgNew};
use crate::error::ffi;
use crate::ffi as cffi;
use crate::geometry::{Axis, Edges, Number, INFINITY};
use crate::prop::{IntoProp, OwnedProp, Prop};
use crate::reactive::{Binding, IntoBinding};
use crate::views::{push_items, untracked, ItemKey};
use crate::view::{raw_views, IntoView, View};

/// How a split view's dividers look. A divider is a line that takes space between the panes; it
/// can be grabbed a few points beside the line too.
#[derive(Clone)]
pub struct SplitViewOptions {
    pub divider: Prop<Color>,
    /// While hovered or dragged; None: the tint.
    pub active_divider: Option<Prop<Color>>,
    /// The line.
    pub thickness: f32,
    /// Drawn over the line while hovered or dragged.
    pub active_thickness: f32,
}

impl Default for SplitViewOptions {
    fn default() -> Self {
        Self { divider: Prop::Theme(ThemeColor::Outline), active_divider: None, thickness: 1.0, active_thickness: 3.0 }
    }
}

#[elegance_macros::elg_builder]
impl SplitViewOptions {
    pub fn divider(mut self, color: impl IntoProp<Color>) -> Self {
        self.divider = color.into_prop();
        self
    }
    pub fn active_divider(mut self, color: impl IntoProp<Color>) -> Self {
        self.active_divider = Some(color.into_prop());
        self
    }
    pub fn thickness(mut self, thickness: impl Number) -> Self {
        self.thickness = thickness.to_f32();
        self
    }
    pub fn active_thickness(mut self, thickness: impl Number) -> Self {
        self.active_thickness = thickness.to_f32();
        self
    }
}

/// What a C form points at: keep it until the call.
type Keep = (Option<Box<sys::ElgColor>>, Option<Box<OwnedProp>>);

fn c_split_view_options(o: SplitViewOptions) -> (sys::ElgSplitViewOptions, Keep) {
    let (divider, divider_value) = OwnedProp::color(o.divider).split();
    let active = o.active_divider.map(|color| Box::new(OwnedProp::color(color)));
    let c = sys::ElgSplitViewOptions {
        divider,
        active_divider: active.as_ref().map_or(std::ptr::null(), |p| &p.c),
        thickness: o.thickness,
        active_thickness: o.active_thickness,
    };
    (c, (divider_value, active))
}

/// Panes side by side with draggable dividers between them.
#[derive(Default)]
pub struct HSplitView {
    panes: Vec<View>,
    options: SplitViewOptions,
}

/// Panes stacked top to bottom with draggable dividers.
#[derive(Default)]
pub struct VSplitView {
    panes: Vec<View>,
    options: SplitViewOptions,
}

macro_rules! split_view {
    ($t:ident, $axis:expr_2021) => {
        impl $t {
            pub fn new() -> Self {
                Self::default()
            }
            pub fn pane(mut self, pane: impl IntoView) -> Self {
                self.panes.push(pane.into_view());
                self
            }
            /// How the dividers look: `.dividers(|o| o.divider(.clear).thickness(0))`.
            pub fn dividers(mut self, options: impl FnOnce(SplitViewOptions) -> SplitViewOptions) -> Self {
                self.options = options(self.options);
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
                let (options, _keep) = c_split_view_options(self.options);
                let mut panes = raw_views(self.panes);
                View::from_raw(ffi!(sys::elg_split_view($axis as i32, panes.as_mut_ptr(), panes.len(), &options)))
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

/// A sidebar and a detail (or sidebar, content, and detail) in resizable columns: an HSplitView
/// of framed columns. Its block holds two or three views.
pub struct NavigationSplitView {
    columns: Vec<View>,
    sidebar_visible: Option<Binding<bool>>,
    sidebar_transition: Transition,
    sidebar_background: Prop<Color>,
    sidebar_width: ColumnWidth,
    content_width: ColumnWidth,
    detail_width: ColumnWidth,
    dividers: SplitViewOptions,
}

impl Default for NavigationSplitView {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            sidebar_visible: None,
            sidebar_transition: Transition::move_edge(Edges::LEADING),
            sidebar_background: Prop::Theme(ThemeColor::Surface),
            sidebar_width: ColumnWidth::new(180.0, 240.0, 360.0),
            content_width: ColumnWidth::new(220.0, 300.0, 480.0),
            detail_width: ColumnWidth::new(200.0, 0.0, INFINITY),
            dividers: SplitViewOptions::default(),
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
    /// How the sidebar comes and goes: by default it slides out to the leading edge, and the
    /// other columns take its width.
    pub fn sidebar_transition(mut self, transition: impl Into<Transition>) -> Self {
        self.sidebar_transition = transition.into();
        self
    }
    /// `Color::CLEAR`: none.
    pub fn sidebar_background(mut self, color: impl IntoProp<Color>) -> Self {
        self.sidebar_background = color.into_prop();
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
    /// Ideal 0: the rest of the width.
    pub fn detail_width(mut self, width: ColumnWidth) -> Self {
        self.detail_width = width;
        self
    }
    pub fn dividers(mut self, options: impl FnOnce(SplitViewOptions) -> SplitViewOptions) -> Self {
        self.dividers = options(self.dividers);
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
        let (dividers, _keep_dividers) = c_split_view_options(self.dividers);
        let (sidebar_background, _keep_background) = OwnedProp::color(self.sidebar_background).split();
        let options = sys::ElgNavigationSplitViewOptions {
            take_sidebar_visible: self.sidebar_visible.map_or(std::ptr::null_mut(), Binding::<bool>::cpp),
            sidebar_transition: self.sidebar_transition.as_ptr(),
            sidebar_background,
            sidebar_width: self.sidebar_width.c(),
            content_width: self.content_width.c(),
            detail_width: self.detail_width.c(),
            dividers,
        };
        View::from_raw(ffi!(sys::elg_navigation_split_view(
            sidebar.into_raw(),
            content.map_or(std::ptr::null_mut(), View::into_raw),
            detail.into_raw(),
            &options
        )))
    }
}

/// How a page comes in when pushed and goes when popped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PageTransition {
    /// Over the page underneath, from the trailing edge.
    #[default]
    Slide,
    /// Cross-fades with the page underneath.
    Fade,
    /// At once.
    None,
}

/// A stack of pages following `path`: the root, then one page per route. Pushing a route onto
/// the path shows its page; Alt+Left (or swiping right with two fingers on a macOS trackpad)
/// pops it. What's around the pages is the NavigationStackStyle's: nothing by default;
/// `.navigation_stack_style(BarNavigationStackStyle::new())` adds a bar with the title and a
/// back button.
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
    transition: PageTransition,
    animation: Option<Animation>,
    back_gestures: bool,
}

impl<R: PartialEq + Clone + 'static> NavigationStack<R> {
    pub fn new<V: IntoView>(path: impl IntoBinding<Vec<R>>, root: impl IntoView, destination: impl Fn(&R) -> V + 'static) -> Self {
        Self::with_path(path).root(root).destination(destination)
    }
    fn with_path(path: impl IntoBinding<Vec<R>>) -> Self {
        Self {
            path: path.into_binding(),
            root: View::empty(),
            destination: None,
            root_title: Prop::Value(String::new()),
            title: None,
            transition: PageTransition::Slide,
            animation: Some(Animation::smooth(0.35)),
            back_gestures: true,
        }
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
    /// The root page's title, for the style (a bar shows it, and on the back button once a page
    /// is pushed). Static or reactive.
    pub fn root_title(mut self, title: impl IntoProp<String>) -> Self {
        self.root_title = title.into_prop();
        self
    }
    /// A pushed page's title, for the style.
    pub fn title(mut self, title: impl Fn(&R) -> String + 'static) -> Self {
        self.title = Some(Rc::new(title));
        self
    }
    pub fn transition(mut self, transition: impl Into<PageTransition>) -> Self {
        self.transition = transition.into();
        self
    }
    /// For pushes and pops that aren't animated themselves (`with_animation` picks its own).
    /// None: those change at once.
    pub fn animation(mut self, animation: Option<Animation>) -> Self {
        self.animation = animation;
        self
    }
    /// Alt+Left, and swiping right with two fingers on a trackpad (macOS), go back.
    pub fn back_gestures(mut self, enabled: bool) -> Self {
        self.back_gestures = enabled;
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
        let animation = self.animation.as_ref().map_or(std::ptr::null(), Animation::as_ptr);
        View::from_raw(ffi!(sys::elg_navigation_stack(
            self.root.into_raw(),
            pages,
            titles,
            pop,
            self.transition as i32,
            animation,
            self.back_gestures
        )))
    }
}
