// Closures and strings across the C boundary.
//
// A Rust closure becomes an ElgClosure: a boxed closure plus a trampoline that catches panics.
// A panic is kept in a thread-local and reported to C++ as a failed call; the shim then throws,
// Elegance unwinds as it does for its own errors, and the panic resumes when control comes back
// to Rust (error::check).

use std::any::Any;
use std::cell::RefCell;
use std::ffi::{c_char, c_void};
use std::panic::{catch_unwind, AssertUnwindSafe};

use elegance_sys as sys;

thread_local! {
    static PANIC: RefCell<Option<Box<dyn Any + Send>>> = const { RefCell::new(None) };
}

pub(crate) fn take_panic() -> Option<Box<dyn Any + Send>> {
    PANIC.with(|p| p.borrow_mut().take())
}

pub(crate) fn store_panic(payload: Box<dyn Any + Send>) {
    PANIC.with(|p| {
        let mut slot = p.borrow_mut();
        // Keep the first panic: later ones are usually consequences of it.
        if slot.is_none() {
            *slot = Some(payload);
        }
    });
}

unsafe extern "C" fn call_shared<F: Fn(*mut c_void, *mut c_void)>(data: *mut c_void, args: *mut c_void, ret: *mut c_void) -> bool {
    let f = &*(data as *const F);
    match catch_unwind(AssertUnwindSafe(|| f(args, ret))) {
        Ok(()) => true,
        Err(payload) => {
            store_panic(payload);
            false
        }
    }
}

unsafe extern "C" fn call_borrowed<F: FnMut(*mut c_void, *mut c_void)>(
    data: *mut c_void,
    args: *mut c_void,
    ret: *mut c_void,
) -> bool {
    let f = &mut *(data as *mut F);
    match catch_unwind(AssertUnwindSafe(|| f(args, ret))) {
        Ok(()) => true,
        Err(payload) => {
            store_panic(payload);
            false
        }
    }
}

unsafe extern "C" fn call_nothing(_: *mut c_void, _: *mut c_void, _: *mut c_void) -> bool {
    true
}

unsafe extern "C" fn drop_box<T>(data: *mut c_void) {
    // Drops run from C++ destructors, which can't unwind: report and carry on.
    if catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(data as *mut T)))).is_err() {
        eprintln!("elegance: a value panicked while being dropped by Elegance");
    }
}

/// An owned closure that C++ may call any number of times. `args` and `ret` point at values whose
/// types depend on the call site (see the shim).
pub(crate) fn closure<F>(f: F) -> sys::ElgClosure
where
    F: Fn(*mut c_void, *mut c_void) + 'static,
{
    sys::ElgClosure {
        data: Box::into_raw(Box::new(f)) as *mut c_void,
        call: Some(call_shared::<F>),
        drop: Some(drop_box::<F>),
    }
}

/// Like `closure`, for closures that may run on another thread.
pub(crate) fn send_closure<F>(f: F) -> sys::ElgClosure
where
    F: Fn(*mut c_void, *mut c_void) + Send + 'static,
{
    closure(f)
}

/// A closure borrowed for one synchronous call.
pub(crate) fn borrowed<F>(f: &mut F) -> sys::ElgClosure
where
    F: FnMut(*mut c_void, *mut c_void),
{
    sys::ElgClosure { data: f as *mut F as *mut c_void, call: Some(call_borrowed::<F>), drop: None }
}

/// C++ owns `value` (calls do nothing); it's dropped when C++ lets go. The data pointer is a
/// `*mut T`.
pub(crate) fn owner<T: 'static>(value: T) -> sys::ElgClosure {
    sys::ElgClosure {
        data: Box::into_raw(Box::new(value)) as *mut c_void,
        call: Some(call_nothing),
        drop: Some(drop_box::<T>),
    }
}

/// An action: `FnMut` behind a RefCell (C++ calls through a shared function).
pub(crate) fn action<F: FnMut() + 'static>(f: F) -> sys::ElgClosure {
    let f = RefCell::new(f);
    closure(move |_, _| {
        let mut f = f.try_borrow_mut().expect("an action ran again while it was still running");
        f()
    })
}

pub(crate) fn optional_action<F: FnMut() + 'static>(f: Option<F>) -> sys::ElgClosure {
    f.map(action).unwrap_or(sys::ElgClosure::NONE)
}

/// A closure taking one argument of C type `A` (args: *mut A).
pub(crate) fn consumer<A: Copy + 'static, F: Fn(A) + 'static>(f: F) -> sys::ElgClosure {
    closure(move |args, _| f(unsafe { *(args as *const A) }))
}

/// A closure returning a C value of type `R` (ret: *mut R).
pub(crate) fn producer<R: 'static, F: Fn() -> R + 'static>(f: F) -> sys::ElgClosure {
    closure(move |_, ret| unsafe { std::ptr::write(ret as *mut R, f()) })
}

pub(crate) fn s(text: &str) -> sys::ElgStr {
    sys::ElgStr::new(text)
}

unsafe extern "C" fn string_sink(ctx: *mut c_void, ptr: *const c_char, len: usize) {
    let target = &mut *(ctx as *mut String);
    if !ptr.is_null() {
        target.push_str(&String::from_utf8_lossy(std::slice::from_raw_parts(ptr.cast(), len)));
    }
}

/// Collects a string that C++ sends through a sink.
pub(crate) fn read_string(f: impl FnOnce(sys::ElgStrSink, *mut c_void)) -> String {
    let mut result = String::new();
    f(string_sink, &mut result as *mut String as *mut c_void);
    result
}

/// Writes a string into a closure's `std::string` result.
pub(crate) unsafe fn write_string(ret: *mut c_void, value: &str) {
    sys::elg_string_assign(ret as *mut sys::CppString, s(value));
}

pub(crate) unsafe fn read_cpp_string(arg: *const c_void) -> String {
    sys::elg_string_view(arg as *const sys::CppString).as_str().to_owned()
}

pub(crate) fn path_str(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}
