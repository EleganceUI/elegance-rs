//! Custom layouts, and the stack layouts to build them from.
//!
//! ```ignore
//! struct Flow { spacing: f32 }
//! impl Layout for Flow {
//!     fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild]) -> Size { ... }
//!     fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild]) { ... }
//! }
//!
//! elg! { (Flow { spacing: 8.0 }) { Text("a") Text("b") } }
//! ```

use std::any::{Any, TypeId};
use std::cell::Cell;
use std::ffi::c_void;
use std::marker::PhantomData;

use elegance_sys as sys;

use crate::dsl::ElgChildren;
use crate::error::ffi;
use crate::ffi as cffi;
use crate::geometry::{Alignment, Axis, HorizontalAlignment, Len, Point, ProposedSize, Rect, Size, VerticalAlignment};
use crate::view::{raw_views, IntoView, View};

/// A child as seen by a layout or layout modifier: it can be measured and placed. Valid during
/// the layout call.
#[repr(transparent)]
pub struct LayoutChild {
    raw: sys::LayoutChild,
}

#[elegance_macros::elg_builder]
impl LayoutChild {
    pub(crate) unsafe fn slice<'a>(ptr: *const sys::LayoutChild, count: usize) -> &'a [LayoutChild] {
        if ptr.is_null() {
            &[]
        } else {
            std::slice::from_raw_parts(ptr as *const LayoutChild, count)
        }
    }
    pub(crate) unsafe fn from_ptr<'a>(ptr: *const sys::LayoutChild) -> &'a LayoutChild {
        &*(ptr as *const LayoutChild)
    }
    fn ptr(&self) -> *const sys::LayoutChild {
        &self.raw
    }

    pub fn size_that_fits(&self, proposal: ProposedSize) -> Size {
        Size::from_c(ffi!(sys::elg_layout_child_size_that_fits(self.ptr(), proposal.c())))
    }
    /// Positions the child's top-left corner; it sizes itself from `proposal`.
    pub fn place(&self, position: Point, proposal: ProposedSize) {
        ffi!(sys::elg_layout_child_place(self.ptr(), position.c(), proposal.c()))
    }
    pub fn layout_priority(&self) -> f32 {
        unsafe { sys::elg_layout_child_priority(self.ptr()) }
    }
    pub fn is_spacer(&self) -> bool {
        unsafe { sys::elg_layout_child_is_spacer(self.ptr()) }
    }
    /// The value set on this child with `.layout_value::<K>(...)`, else `K::default_value()`.
    pub fn value<K: LayoutValueKey>(&self) -> K::Value {
        REQUESTED.with(|r| r.set(Some(TypeId::of::<K>())));
        let data = unsafe { sys::elg_layout_child_rust_value(self.ptr()) };
        REQUESTED.with(|r| r.set(None));
        crate::error::check();
        if data.is_null() {
            return K::default_value();
        }
        let value = unsafe { &*(data as *const LayoutValue) };
        value.value.downcast_ref::<K::Value>().cloned().unwrap_or_else(K::default_value)
    }
}

thread_local! {
    // The key a LayoutChild::value lookup wants: modifiers holding another key don't match.
    static REQUESTED: Cell<Option<TypeId>> = const { Cell::new(None) };
}

/// A key for values set on a child for its parent layout:
/// `struct Rank; impl LayoutValueKey for Rank { type Value = i32; fn default_value() -> i32 { 0 } }`
pub trait LayoutValueKey: 'static {
    type Value: Clone + 'static;
    fn default_value() -> Self::Value;
}

struct LayoutValue {
    key: TypeId,
    value: Box<dyn Any>,
}

pub(crate) fn layout_value<K: LayoutValueKey>(view: View, value: K::Value) -> View {
    let owner = LayoutValue { key: TypeId::of::<K>(), value: Box::new(value) };
    // The owner closure's data is the LayoutValue; calling it answers whether it's the wanted key.
    let data = Box::into_raw(Box::new(owner));
    unsafe extern "C" fn matches(data: *mut c_void, _: *mut c_void, ret: *mut c_void) -> bool {
        let value = &*(data as *const LayoutValue);
        *(ret as *mut bool) = REQUESTED.with(|r| r.get()) == Some(value.key);
        true
    }
    unsafe extern "C" fn drop_value(data: *mut c_void) {
        drop(Box::from_raw(data as *mut LayoutValue));
    }
    let closure = sys::ElgClosure { data: data as *mut c_void, call: Some(matches), drop: Some(drop_value) };
    view.modify(|v| unsafe { sys::elg_mod_rust_layout_value(v, closure) })
}

/// Arranges children: measure them with `size_that_fits`, then place each.
pub trait Layout: 'static {
    /// The size this layout wants for `proposal`.
    fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild]) -> Size;
    /// Places every child within `bounds` (the size it reported, positioned by the parent).
    fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild]);
    /// The axis Spacers inside expand along, if any.
    fn spacer_axis(&self) -> Option<Axis> {
        None
    }

    /// A view laying out `children` with this layout.
    fn view(self, children: Vec<View>) -> View
    where
        Self: Sized,
    {
        layout_view(LayoutImpl::Plain(Box::new(self)), children)
    }
}

/// A layout that computes something from its children once and reuses it across
/// measurement and placement until the children change.
pub trait CachedLayout: 'static {
    type Cache: 'static;
    fn make_cache(&self, children: &[LayoutChild]) -> Self::Cache;
    /// The children changed; by default makes a new cache.
    fn update_cache(&self, cache: &mut Self::Cache, children: &[LayoutChild]) {
        *cache = self.make_cache(children);
    }
    fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild], cache: &mut Self::Cache) -> Size;
    fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild], cache: &mut Self::Cache);
    fn spacer_axis(&self) -> Option<Axis> {
        None
    }

    fn view(self, children: Vec<View>) -> View
    where
        Self: Sized,
    {
        layout_view(LayoutImpl::Cached(Box::new(CachedAdapter(self))), children)
    }
}

// A CachedLayout behind one object-safe interface.
trait AnyCached {
    fn call(&self, op: i32, proposal: ProposedSize, bounds: Rect, children: &[LayoutChild], cache: *mut sys::LayoutCache) -> Size;
    fn spacer_axis(&self) -> Option<Axis>;
}

struct CachedAdapter<L: CachedLayout>(L);

impl<L: CachedLayout> AnyCached for CachedAdapter<L> {
    fn call(&self, op: i32, proposal: ProposedSize, bounds: Rect, children: &[LayoutChild], storage: *mut sys::LayoutCache) -> Size {
        let mut local;
        let cache: &mut L::Cache = if storage.is_null() {
            local = self.0.make_cache(children);
            &mut local
        } else {
            unsafe {
                let data = sys::elg_layout_cache_get(storage);
                if data.is_null() {
                    sys::elg_layout_cache_set(storage, cffi::owner(self.0.make_cache(children)));
                } else if sys::elg_layout_cache_stale(storage) {
                    self.0.update_cache(&mut *(data as *mut L::Cache), children);
                }
                sys::elg_layout_cache_set_stale(storage, false);
                &mut *(sys::elg_layout_cache_get(storage) as *mut L::Cache)
            }
        };
        if op == 0 {
            self.0.size_that_fits(proposal, children, cache)
        } else {
            self.0.place_children(bounds, proposal, children, cache);
            Size::default()
        }
    }
    fn spacer_axis(&self) -> Option<Axis> {
        self.0.spacer_axis()
    }
}

enum LayoutImpl {
    Plain(Box<dyn Layout>),
    Cached(Box<dyn AnyCached>),
}

fn axis_c(axis: Option<Axis>) -> i32 {
    match axis {
        None => -1,
        Some(Axis::Horizontal) => 0,
        Some(Axis::Vertical) => 1,
    }
}

fn layout_view(layout: LayoutImpl, children: Vec<View>) -> View {
    let call = cffi::closure(move |args, ret| unsafe {
        let call = &*(args as *const sys::ElgLayoutCall);
        let proposal = ProposedSize::from_c(call.proposal);
        let bounds = Rect::from_c(call.bounds);
        let children = LayoutChild::slice(call.children, call.count);
        match (&layout, call.op) {
            (LayoutImpl::Plain(l), 0) => *(ret as *mut sys::ElgSize) = l.size_that_fits(proposal, children).c(),
            (LayoutImpl::Plain(l), 1) => l.place_children(bounds, proposal, children),
            (LayoutImpl::Plain(l), _) => *(ret as *mut i32) = axis_c(l.spacer_axis()),
            (LayoutImpl::Cached(l), 0) => *(ret as *mut sys::ElgSize) = l.call(0, proposal, bounds, children, call.cache).c(),
            (LayoutImpl::Cached(l), 1) => {
                l.call(1, proposal, bounds, children, call.cache);
            }
            (LayoutImpl::Cached(l), _) => *(ret as *mut i32) = axis_c(l.spacer_axis()),
        }
    });
    let mut children = raw_views(children);
    View::from_raw(ffi!(sys::elg_rust_layout_view(call, children.as_mut_ptr(), children.len())))
}

/// `(MyLayout { .. }) { children }` in `elg!`.
impl<L: Layout> ElgChildren for L {
    type Out = View;
    fn elg_children(self, children: Vec<View>) -> View {
        self.view(children)
    }
}

/// Wraps a CachedLayout for `(Cached(MyLayout { .. })) { children }` in `elg!`.
pub struct Cached<L: CachedLayout>(pub L);

impl<L: CachedLayout> ElgChildren for Cached<L> {
    type Out = View;
    fn elg_children(self, children: Vec<View>) -> View {
        self.0.view(children)
    }
}

// --- The built-in stack layouts, for composing custom layouts ---

/// VStack's layout: measure and place children with it from a custom layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VStackLayout {
    pub alignment: HorizontalAlignment,
    pub spacing: Len,
}

/// HStack's layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HStackLayout {
    pub alignment: VerticalAlignment,
    pub spacing: Len,
}

/// ZStack's layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ZStackLayout {
    pub alignment: Alignment,
}

macro_rules! stack_layout {
    ($t:ident, $kind:expr, |$s:ident| $alignment:expr, $spacing:expr) => {
        impl $t {
            pub fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild]) -> Size {
                let $s = self;
                let ptr = children.as_ptr() as *const sys::LayoutChild;
                Size::from_c(ffi!(sys::elg_stack_layout_size($kind, $alignment, $spacing, proposal.c(), ptr, children.len())))
            }
            pub fn place_children(&self, bounds: Rect, proposal: ProposedSize, children: &[LayoutChild]) {
                let $s = self;
                let ptr = children.as_ptr() as *const sys::LayoutChild;
                ffi!(sys::elg_stack_layout_place($kind, $alignment, $spacing, bounds.c(), proposal.c(), ptr, children.len()))
            }
        }
        impl ElgChildren for $t {
            type Out = View;
            fn elg_children(self, children: Vec<View>) -> View {
                self.view(children)
            }
        }
    };
}

stack_layout!(VStackLayout, 0, |s| sys::ElgAlignment { horizontal: s.alignment as i32, vertical: 1 }, s.spacing.c());
stack_layout!(HStackLayout, 1, |s| sys::ElgAlignment { horizontal: 1, vertical: s.alignment as i32 }, s.spacing.c());
stack_layout!(ZStackLayout, 2, |s| s.alignment.c(), Len(None).c());

#[elegance_macros::elg_builder]
impl VStackLayout {
    pub fn new(alignment: impl Into<HorizontalAlignment>, spacing: impl Into<Len>) -> Self {
        Self { alignment: alignment.into(), spacing: spacing.into() }
    }
    pub fn view(self, children: Vec<View>) -> View {
        crate::views::VStack::new().alignment(self.alignment).spacing(self.spacing).children(children).into_view()
    }
}

#[elegance_macros::elg_builder]
impl HStackLayout {
    pub fn new(alignment: impl Into<VerticalAlignment>, spacing: impl Into<Len>) -> Self {
        Self { alignment: alignment.into(), spacing: spacing.into() }
    }
    pub fn view(self, children: Vec<View>) -> View {
        crate::views::HStack::new().alignment(self.alignment).spacing(self.spacing).children(children).into_view()
    }
}

#[elegance_macros::elg_builder]
impl ZStackLayout {
    pub fn new(alignment: impl Into<Alignment>) -> Self {
        Self { alignment: alignment.into() }
    }
    pub fn view(self, children: Vec<View>) -> View {
        crate::views::ZStack::new().alignment(self.alignment).children(children).into_view()
    }
}

#[allow(dead_code)]
struct NotSend(PhantomData<*const ()>);
