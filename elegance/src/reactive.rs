//! Fine-grained reactive state: [`State`], [`Binding`], [`Computed`], and effects.
//!
//! ```ignore
//! let mut count = State::new(0);
//! elg! { Text("Count: {count}") }        // re-renders that one Text when count changes
//! Button("+", move || count += 1)
//! *count.write() += 1;                   // or count.set(..), count.update(|c| ..)
//! let n = *count.read();                 // or count.get(), count.with(|c| ..)
//! ```
//!
//! Reading state inside a closure Elegance runs reactively (a reactive argument, a computed, an
//! effect) subscribes that closure; writing it re-runs exactly the closures that read it, once per
//! frame. It doesn't matter where the closure was written (in `elg!`, in a helper function, in a
//! variable): what counts is that Elegance calls it. A component's body runs once, so a read there
//! (`if todos.read().is_empty()` outside `elg!`) is a one-time value; pass a closure
//! (`move || todos.read().is_empty()`) or a derived value (`todos.map(Vec::is_empty)`) instead.
//! Everything is UI-thread only, so these handles are neither `Send` nor `Sync`.
//!
//! The handles are `Copy`. Their values live as long as the views that created them: state made
//! while building a view ends when the view is removed (an `If` branch switching, a `ForEach` item
//! going away, the window closing). Using a handle after that panics. State made outside any view
//! (before `App::run`) lives for the whole program.

use std::any::Any;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::ffi::c_void;
use std::fmt;
use std::marker::PhantomData;
use std::ops::{AddAssign, Deref, DerefMut, Not, SubAssign};
use std::rc::Rc;

use elegance_sys as sys;

use crate::error::ffi;
use crate::ffi as cffi;

// --- The arena behind the Copy handles ---

struct Slot {
    generation: u32,
    value: Option<Rc<dyn Any>>,
    node: *mut c_void,
}

#[derive(Default)]
struct Arena {
    slots: Vec<Slot>,
    free: Vec<u32>,
}

thread_local! {
    static ARENA: RefCell<Arena> = RefCell::new(Arena::default());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Handle {
    index: u32,
    generation: u32,
}

const STALE: &str = "elegance: reactive state was used after the views that created it were removed";

fn reserve(value: Rc<dyn Any>) -> Handle {
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        if let Some(index) = arena.free.pop() {
            let slot = &mut arena.slots[index as usize];
            slot.value = Some(value);
            Handle { index, generation: slot.generation }
        } else {
            arena.slots.push(Slot { generation: 0, value: Some(value), node: std::ptr::null_mut() });
            Handle { index: (arena.slots.len() - 1) as u32, generation: 0 }
        }
    })
}

fn set_node(handle: Handle, node: *mut c_void) {
    ARENA.with(|arena| {
        let mut arena = arena.borrow_mut();
        let slot = &mut arena.slots[handle.index as usize];
        if slot.generation == handle.generation {
            slot.node = node;
        }
    })
}

fn free(handle: Handle) {
    // The value is dropped after the arena is released: dropping it may free other slots.
    let value = ARENA
        .try_with(|arena| {
            let mut arena = arena.borrow_mut();
            let slot = &mut arena.slots[handle.index as usize];
            if slot.generation != handle.generation {
                return None;
            }
            slot.generation = slot.generation.wrapping_add(1);
            slot.node = std::ptr::null_mut();
            let value = slot.value.take();
            arena.free.push(handle.index);
            value
        })
        .ok()
        .flatten();
    drop(value);
}

#[track_caller]
fn lookup<T: 'static>(handle: Handle) -> (Rc<T>, *mut c_void) {
    let found = ARENA.with(|arena| {
        let arena = arena.borrow();
        arena
            .slots
            .get(handle.index as usize)
            .filter(|slot| slot.generation == handle.generation)
            .and_then(|slot| slot.value.clone().map(|value| (value, slot.node)))
    });
    let (value, node) = found.expect(STALE);
    (value.downcast::<T>().unwrap_or_else(|_| unreachable!("a reactive handle of the wrong type")), node)
}

pub(crate) fn is_alive(handle: Handle) -> bool {
    ARENA.with(|arena| {
        arena
            .borrow()
            .slots
            .get(handle.index as usize)
            .is_some_and(|slot| slot.generation == handle.generation && slot.value.is_some())
    })
}

// Frees the slot when C++ destroys the node that owns this.
struct SlotOwner(Handle);

impl Drop for SlotOwner {
    fn drop(&mut self) {
        free(self.0);
    }
}

/// A slot owned by a signal node in the current scope.
fn anchored(value: Rc<dyn Any>) -> Handle {
    let handle = reserve(value);
    let node = ffi!(sys::elg_signal_new(cffi::owner(SlotOwner(handle))));
    set_node(handle, node);
    handle
}

// --- State ---

struct StateCell<T> {
    value: RefCell<T>,
    eq: Option<fn(&T, &T) -> bool>,
}

/// A piece of mutable state: a `Copy` handle to a value, so closures can capture it with `move`
/// and still write to it.
pub struct State<T: 'static> {
    handle: Handle,
    _marker: PhantomData<*const T>,
}

impl<T: 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for State<T> {}

impl<T: 'static> PartialEq for State<T> {
    /// Whether both handles refer to the same state.
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<T: 'static> Eq for State<T> {}

impl<T: PartialEq + 'static> State<T> {
    /// Writing a value equal to the current one changes nothing.
    pub fn new(initial: T) -> Self {
        Self::make(initial, Some(<T as PartialEq>::eq))
    }
}

impl<T: Default + PartialEq + 'static> Default for State<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: 'static> State<T> {
    /// State whose type can't be compared: every write notifies.
    pub fn always_notify(initial: T) -> Self {
        Self::make(initial, None)
    }

    fn make(initial: T, eq: Option<fn(&T, &T) -> bool>) -> Self {
        let handle = anchored(Rc::new(StateCell { value: RefCell::new(initial), eq }));
        Self { handle, _marker: PhantomData }
    }

    #[track_caller]
    fn cell(&self) -> (Rc<StateCell<T>>, *mut c_void) {
        lookup(self.handle)
    }

    #[track_caller]
    fn borrow(&self, tracked: bool) -> ReadRef<T> {
        let (cell, node) = self.cell();
        if tracked {
            ffi!(sys::elg_node_track(node));
        }
        let value = cell.value.try_borrow().expect("elegance: state read while it's being written");
        // SAFETY: the guard keeps `cell` alive and drops this borrow first.
        let value = unsafe { std::mem::transmute::<Ref<'_, T>, Ref<'static, T>>(value) };
        ReadRef { value, _owner: cell }
    }

    /// Reads the value, subscribing the running computed/effect/reactive argument:
    /// `todos.read().is_empty()`, `*count.read()`. Drop the guard before writing.
    #[track_caller]
    pub fn read(&self) -> ReadRef<T> {
        self.borrow(true)
    }

    /// Reads without subscribing.
    #[track_caller]
    pub fn peek(&self) -> ReadRef<T> {
        self.borrow(false)
    }

    /// A copy of the value (a tracked read, like `read`).
    #[track_caller]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.read().clone()
    }

    /// Reads the value by reference (subscribing, like `read`).
    #[track_caller]
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.read())
    }

    #[track_caller]
    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.peek())
    }

    /// A value derived from this state, recomputed when it changes:
    /// `.popover(todos.map(Vec::is_empty), ...)`, `.disabled(draft.map(String::is_empty))`.
    pub fn map<R: PartialEq + 'static>(&self, f: impl Fn(&T) -> R + 'static) -> Computed<R> {
        let state = *self;
        Computed::new(move || state.with(&f))
    }

    /// Changes the value through a guard, re-running what read it when the guard is dropped:
    /// `*count.write() += 1`, `todos.write().push(todo)`. UI thread only.
    #[track_caller]
    pub fn write(&self) -> WriteRef<T> {
        let (cell, node) = self.cell();
        ffi!(sys::elg_check_ui_thread());
        let value = cell.value.try_borrow_mut().expect("elegance: state written while it's being read");
        // SAFETY: the guard keeps `cell` alive and drops this borrow first.
        let value = unsafe { std::mem::transmute::<RefMut<'_, T>, RefMut<'static, T>>(value) };
        WriteRef { value: Some(value), node, _owner: cell }
    }

    /// Flips a `bool` (or anything with `!`).
    #[track_caller]
    pub fn toggle(&self)
    where
        T: Not<Output = T> + Clone,
    {
        self.update(|value| *value = !value.clone());
    }

    /// Writes the value, re-running what read it. UI thread only.
    #[track_caller]
    pub fn set(&self, value: T) {
        let (cell, node) = self.cell();
        ffi!(sys::elg_check_ui_thread());
        if let Some(eq) = cell.eq
            && eq(&cell.value.borrow(), &value) {
                return;
            }
        let old = cell.value.replace(value);
        drop(old);
        ffi!(sys::elg_node_notify(node));
    }

    /// Changes the value in place: `count.update(|c| *c += 2)`.
    #[track_caller]
    pub fn update(&self, change: impl FnOnce(&mut T)) {
        change(&mut self.write());
    }

    /// A two-way connection to this state. Controls and `Binding` props take the state itself, so
    /// this is rarely needed.
    pub fn binding(&self) -> Binding<T> {
        Binding { kind: BindingKind::State(self.handle), _marker: PhantomData }
    }

    /// Whether the views that made this state are still there.
    pub fn is_alive(&self) -> bool {
        is_alive(self.handle)
    }
}

impl<T: AddAssign<U> + 'static, U> AddAssign<U> for State<T> {
    fn add_assign(&mut self, amount: U) {
        self.update(|value| *value += amount);
    }
}

impl<T: SubAssign<U> + 'static, U> SubAssign<U> for State<T> {
    fn sub_assign(&mut self, amount: U) {
        self.update(|value| *value -= amount);
    }
}

/// Formats the value (a tracked read): `Text("Count: {count}")`.
impl<T: fmt::Display + 'static> fmt::Display for State<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with(|value| value.fmt(f))
    }
}

impl<T: fmt::Debug + 'static> fmt::Debug for State<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_alive() {
            self.with_untracked(|value| f.debug_tuple("State").field(value).finish())
        } else {
            f.write_str("State(<removed>)")
        }
    }
}

// --- Guards ---

/// A borrowed value, from [`State::read`], [`State::peek`] or [`Computed::read`].
pub struct ReadRef<T: 'static> {
    // Declared first, so it's dropped before what it borrows from.
    value: Ref<'static, T>,
    _owner: Rc<dyn Any>,
}

impl<T: 'static> Deref for ReadRef<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: fmt::Display + 'static> fmt::Display for ReadRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: fmt::Debug + 'static> fmt::Debug for ReadRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

/// A mutably borrowed state value, from [`State::write`]. Dropping it re-runs what read the state.
pub struct WriteRef<T: 'static> {
    value: Option<RefMut<'static, T>>,
    node: *mut c_void,
    _owner: Rc<dyn Any>,
}

impl<T: 'static> Deref for WriteRef<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.value.as_deref().unwrap()
    }
}

impl<T: 'static> DerefMut for WriteRef<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value.as_deref_mut().unwrap()
    }
}

impl<T: 'static> Drop for WriteRef<T> {
    fn drop(&mut self) {
        self.value = None;
        unsafe { sys::elg_node_notify(self.node) };
        // A second panic while unwinding would abort.
        if !std::thread::panicking() {
            crate::error::check();
        }
    }
}

// --- Binding ---

#[derive(Clone, Copy, PartialEq, Eq)]
enum BindingKind {
    State(Handle),
    Custom(Handle),
}

struct BindingCell<T> {
    get: Box<dyn Fn() -> T>,
    set: Box<dyn Fn(T)>,
}

/// Read/write access to a value owned elsewhere (usually a [`State`]). Controls and child views
/// take bindings so they can change their parent's state.
pub struct Binding<T: 'static> {
    kind: BindingKind,
    _marker: PhantomData<*const T>,
}

impl<T: 'static> Clone for Binding<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Binding<T> {}

impl<T: 'static> From<State<T>> for Binding<T> {
    fn from(state: State<T>) -> Self {
        state.binding()
    }
}

impl<T: 'static> Binding<T> {
    /// A binding from a getter and a setter (a field of an item in a list, a conversion, ...).
    pub fn new(get: impl Fn() -> T + 'static, set: impl Fn(T) + 'static) -> Self {
        let handle = anchored(Rc::new(BindingCell { get: Box::new(get), set: Box::new(set) }));
        Self { kind: BindingKind::Custom(handle), _marker: PhantomData }
    }

    /// A binding to a fixed value; writes are ignored. Handy for previews.
    pub fn constant(value: T) -> Self
    where
        T: Clone,
    {
        Self::new(move || value.clone(), |_| {})
    }

    /// A binding that reads `get` (reactively) and ignores writes: a popover shown while a
    /// condition holds, `move || todos.read().is_empty()`.
    pub fn read_only(get: impl Fn() -> T + 'static) -> Self {
        Self::new(get, |_| {})
    }

    #[track_caller]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        match self.kind {
            BindingKind::State(handle) => State::<T> { handle, _marker: PhantomData }.get(),
            BindingKind::Custom(handle) => (lookup::<BindingCell<T>>(handle).0.get)(),
        }
    }

    #[track_caller]
    pub fn set(&self, value: T) {
        match self.kind {
            BindingKind::State(handle) => State::<T> { handle, _marker: PhantomData }.set(value),
            BindingKind::Custom(handle) => (lookup::<BindingCell<T>>(handle).0.set)(value),
        }
    }

    /// Reads, changes, and writes back the value.
    #[track_caller]
    pub fn update(&self, change: impl FnOnce(&mut T))
    where
        T: Clone,
    {
        let mut value = untrack(|| self.get());
        change(&mut value);
        self.set(value);
    }
}

impl<T: AddAssign<U> + Clone + 'static, U> AddAssign<U> for Binding<T> {
    fn add_assign(&mut self, amount: U) {
        self.update(|value| *value += amount);
    }
}

impl<T: SubAssign<U> + Clone + 'static, U> SubAssign<U> for Binding<T> {
    fn sub_assign(&mut self, amount: U) {
        self.update(|value| *value -= amount);
    }
}

impl<T: fmt::Display + Clone + 'static> fmt::Display for Binding<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

/// A binding to a fixed value, for a control that should show `value` and ignore edits:
/// `Toggle("Always on", constant(true))`.
pub fn constant<T: Clone + 'static>(value: T) -> Binding<T> {
    Binding::constant(value)
}

/// Read-only access to a value that can change: a State, Binding, Computed, or closure. Reading
/// it is a tracked read. Pass it anywhere a reactive argument goes.
pub struct Signal<T: 'static>(Rc<dyn Fn() -> T>);

impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: 'static> Signal<T> {
    pub fn new(get: impl Fn() -> T + 'static) -> Self {
        Self(Rc::new(get))
    }
    pub fn get(&self) -> T {
        (self.0)()
    }
}

impl<T: Clone + 'static> From<State<T>> for Signal<T> {
    fn from(state: State<T>) -> Self {
        Self::new(move || state.get())
    }
}

impl<T: Clone + 'static> From<Binding<T>> for Signal<T> {
    fn from(binding: Binding<T>) -> Self {
        Self::new(move || binding.get())
    }
}

impl<T: Clone + 'static> From<Computed<T>> for Signal<T> {
    fn from(computed: Computed<T>) -> Self {
        Self::new(move || computed.get())
    }
}

impl<T: 'static> crate::prop::IntoProp<T> for Signal<T> {
    fn into_prop(self) -> crate::prop::Prop<T> {
        crate::prop::Prop::Fn(self.0)
    }
}

/// What a two-way argument (a popover's `is_presented`, a Toggle's value, ...) can be made from:
/// a [`State`] or [`Binding`], or, read-only, a [`Computed`] or a closure.
///
/// ```ignore
/// .popover(editing, content)                           // dismissing sets `editing` to false
/// .popover(move || todos.read().is_empty(), content)    // shown while the list is empty
/// .popover(todos.map(Vec::is_empty), content)          // the same
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` can't be used as a `Binding<{T}>`",
    label = "expected a State/Binding of a `{T}`, a Computed, or a closure returning one"
)]
pub trait IntoBinding<T: 'static> {
    fn into_binding(self) -> Binding<T>;
}

impl<T: 'static> IntoBinding<T> for State<T> {
    fn into_binding(self) -> Binding<T> {
        self.binding()
    }
}

impl<T: 'static> IntoBinding<T> for Binding<T> {
    fn into_binding(self) -> Binding<T> {
        self
    }
}

impl<T: Clone + 'static> IntoBinding<T> for Computed<T> {
    fn into_binding(self) -> Binding<T> {
        Binding::read_only(move || self.get())
    }
}

impl<T: 'static, F: Fn() -> T + 'static> IntoBinding<T> for F {
    fn into_binding(self) -> Binding<T> {
        Binding::read_only(self)
    }
}

// The C++ bindings controls take.
impl Binding<bool> {
    pub(crate) fn cpp(self) -> *mut sys::BindingBool {
        unsafe { sys::elg_binding_bool(cffi::producer(move || self.get()), cffi::consumer(move |v: bool| self.set(v))) }
    }
}

impl Binding<f32> {
    pub(crate) fn cpp(self) -> *mut sys::BindingF32 {
        unsafe { sys::elg_binding_f32(cffi::producer(move || self.get()), cffi::consumer(move |v: f32| self.set(v))) }
    }

    /// Wraps a binding made by Elegance (a player's time, ...).
    pub(crate) fn from_cpp(ptr: *mut sys::BindingF32) -> Self {
        struct Owned(*mut sys::BindingF32);
        impl Drop for Owned {
            fn drop(&mut self) {
                unsafe { sys::elg_binding_f32_free(self.0) }
            }
        }
        let owned = Rc::new(Owned(ptr));
        let reader = owned.clone();
        Binding::new(move || ffi!(sys::elg_binding_f32_get(reader.0)), move |v| ffi!(sys::elg_binding_f32_set(owned.0, v)))
    }
}

impl Binding<String> {
    pub(crate) fn cpp(self) -> *mut sys::BindingString {
        let get = cffi::closure(move |_, ret| unsafe { cffi::write_string(ret, &self.get()) });
        let set = cffi::closure(move |args, _| self.set(unsafe { cffi::read_cpp_string(args) }));
        unsafe { sys::elg_binding_string(get, set) }
    }
}

// --- Computed ---

struct ComputedCell<T> {
    value: RefCell<Option<T>>,
    version: Cell<u64>,
    function: Box<dyn Fn() -> T>,
}

/// A value derived from state: recomputed lazily, at most once per change, and only when read.
pub struct Computed<T: 'static> {
    handle: Handle,
    _marker: PhantomData<*const T>,
}

impl<T: 'static> Clone for Computed<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Computed<T> {}

impl<T: PartialEq + 'static> Computed<T> {
    pub fn new(function: impl Fn() -> T + 'static) -> Self {
        let cell = Rc::new(ComputedCell { value: RefCell::new(None), version: Cell::new(0), function: Box::new(function) });
        let handle = reserve(cell.clone());
        let owner = SlotOwner(handle);
        // C++ sees a version number that changes exactly when the value does.
        let compute = cffi::closure(move |_, ret| {
            let _owner = &owner;
            let next = (cell.function)();
            let changed = {
                let mut value = cell.value.borrow_mut();
                if value.as_ref() == Some(&next) {
                    false
                } else {
                    *value = Some(next);
                    true
                }
            };
            if changed {
                cell.version.set(cell.version.get() + 1);
            }
            unsafe { *(ret as *mut u64) = cell.version.get() };
        });
        let node = ffi!(sys::elg_computed_new(compute));
        set_node(handle, node);
        Self { handle, _marker: PhantomData }
    }
}

impl<T: 'static> Computed<T> {
    #[track_caller]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    /// Reads the value, computing it first if a dependency changed (a tracked read).
    #[track_caller]
    pub fn read(&self) -> ReadRef<T> {
        let (cell, node) = lookup::<ComputedCell<T>>(self.handle);
        ffi!(sys::elg_node_track(node));
        let value = Ref::map(cell.value.borrow(), |value| value.as_ref().expect(STALE));
        // SAFETY: the guard keeps `cell` alive and drops this borrow first.
        let value = unsafe { std::mem::transmute::<Ref<'_, T>, Ref<'static, T>>(value) };
        ReadRef { value, _owner: cell }
    }

    /// Reads the value by reference (a tracked read, like `read`).
    #[track_caller]
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.read())
    }
}

impl<T: fmt::Display + 'static> fmt::Display for Computed<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with(|value| value.fmt(f))
    }
}

/// `let doubled = computed(move || count.get() * 2);`
pub fn computed<T: PartialEq + 'static>(function: impl Fn() -> T + 'static) -> Computed<T> {
    Computed::new(function)
}

// --- Effects ---

/// Runs `function` now and again whenever state it read changes (after the current event, before
/// the next frame). Call it while building views; it ends when those views are removed.
#[track_caller]
pub fn effect(function: impl Fn() + 'static) {
    ffi!(sys::elg_effect(cffi::closure(move |_, _| function())));
}

/// Reads state inside `function` without subscribing to it.
pub fn untrack<R>(function: impl FnOnce() -> R) -> R {
    run_borrowed(function, |closure| unsafe { sys::elg_untrack(closure) })
}

/// Groups several writes: effects run once, after the outermost batch ends.
pub fn batch<R>(function: impl FnOnce() -> R) -> R {
    run_borrowed(function, |closure| unsafe { sys::elg_batch(closure) })
}

/// Calls `function` through a shim function that runs a borrowed closure.
pub(crate) fn run_borrowed<R>(function: impl FnOnce() -> R, run: impl FnOnce(sys::ElgClosure)) -> R {
    let mut function = Some(function);
    let mut result = None;
    {
        let mut call = |_: *mut c_void, _: *mut c_void| result = Some((function.take().unwrap())());
        run(cffi::borrowed(&mut call));
    }
    crate::error::check();
    result.expect("elegance: a closure didn't run")
}
