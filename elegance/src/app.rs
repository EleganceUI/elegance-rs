//! The application: windows and the event loop.

use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::ExitCode;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};

use elegance_sys as sys;

use crate::color::Color;
use crate::error::{self, AppError, Error, ErrorAction};
use crate::ffi as cffi;
use crate::geometry::{Point, Size};
use crate::view::IntoView;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TitleBar {
    /// The platform's title bar and frame.
    #[default]
    System,
    /// None: the content fills the window. Draw your own title bar with `.window_drag_area()`
    /// and `WindowControls()`; edges still resize.
    Hidden,
    /// The content fills the window, and the system's own window controls stay over it, where
    /// the platform has them: the traffic lights on macOS (see
    /// [`WindowDesc::system_controls_position`]). Leave room with
    /// [`WindowHandle::system_controls_frame`](crate::WindowHandle::system_controls_frame) and
    /// drag with `.window_drag_area()`. Elsewhere the window keeps its system title bar (as
    /// `System`).
    Integrated,
}

/// How a window opens.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowDesc {
    pub title: String,
    /// Logical points.
    pub width: i32,
    pub height: i32,
    pub resizable: bool,
    /// Smallest size the user can resize to.
    pub min_size: Option<Size>,
    /// Shown where no view draws. Default: the theme's background (clear for transparent windows).
    pub background: Option<Color>,
    pub title_bar: TitleBar,
    /// `TitleBar::Integrated`: where the system's window controls go, as the first one's top-left
    /// corner in points from the window's top-left corner (the others follow it). Default: where
    /// the system puts them. Only macOS draws its controls over the content, so only macOS uses it.
    pub system_controls_position: Option<Point>,
    /// The desktop shows through wherever the content isn't opaque (with `TitleBar::Hidden`).
    pub transparent: bool,
    /// The whole window's opacity, 0...1.
    pub opacity: f32,
    /// Covers the display it opens on, without a frame. width/height are its size after.
    pub fullscreen: bool,
}

impl Default for WindowDesc {
    fn default() -> Self {
        Self {
            title: "Elegance".into(),
            width: 1280,
            height: 720,
            resizable: true,
            min_size: None,
            background: None,
            title_bar: TitleBar::System,
            system_controls_position: None,
            transparent: false,
            opacity: 1.0,
            fullscreen: false,
        }
    }
}

#[elegance_macros::elg_builder]
impl WindowDesc {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), ..Default::default() }
    }
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.min_size = Some(Size::new(width, height));
        self
    }
    pub fn background(mut self, color: impl Into<Color>) -> Self {
        self.background = Some(color.into());
        self
    }
    pub fn title_bar(mut self, title_bar: impl Into<TitleBar>) -> Self {
        self.title_bar = title_bar.into();
        self
    }
    pub fn system_controls_position(mut self, x: f32, y: f32) -> Self {
        self.system_controls_position = Some(Point::new(x, y));
        self
    }
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }
}

struct AppPtr(NonNull<sys::App>);

impl Drop for AppPtr {
    fn drop(&mut self) {
        unsafe { sys::elg_app_free(self.0.as_ptr()) }
    }
}

/// Owns the application lifetime: windows, the event loop, and the render thread.
///
/// ```ignore
/// fn main() -> Result<ExitCode, elegance::Error> {
///     App::new().window(WindowDesc::new("Counter").size(360, 280), Counter).run()
/// }
/// ```
pub struct App {
    inner: Rc<AppPtr>,
    on_error: Rc<RefCell<Option<ErrorHandler>>>,
}

type ErrorHandler = Box<dyn FnMut(&AppError) -> ErrorAction>;

// Hears each report for the app, with or without a handler of the app's: Elegance reported the
// panic and the app goes on, so its payload goes too, or a later failure would resume it. It's kept
// only when the handler ends the run, for `run` to resume.
fn report_closure(slot: Rc<RefCell<Option<ErrorHandler>>>) -> sys::ElgClosure {
    cffi::fallible_closure(move |args, ret| unsafe {
        let error = &*(args as *const sys::ElgAppError);
        let payload = if error.kind == 1 { cffi::take_panic() } else { None };
        let Some(mut handler) = slot.borrow_mut().take() else {
            return true;
        };
        let app_error = AppError::from_c(error, payload.as_deref().and_then(cffi::panic_message));
        let action = catch_unwind(AssertUnwindSafe(|| handler(&app_error)));
        let mut current = slot.borrow_mut();
        if current.is_none() {
            *current = Some(handler); // unless it set another
        }
        match action {
            Ok(ErrorAction::Resume) => true,
            Ok(ErrorAction::Quit) => {
                *(ret as *mut i32) = 1;
                if let Some(payload) = payload {
                    cffi::store_panic(payload);
                }
                true
            }
            // Elegance logs that the handler panicked; the panic hook already printed what with.
            Err(_) => false,
        }
    })
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    #[track_caller]
    pub fn new() -> Self {
        // The library must be the one elegance-sys was generated for: other functions or structs
        // would be called with the wrong arguments.
        let library = unsafe { sys::elg_abi_version() };
        if library != sys::ELG_ABI_VERSION {
            panic!(
                "elegance: the Elegance library has C API version {library}, but elegance-sys was made for version {}",
                sys::ELG_ABI_VERSION
            );
        }
        let ptr = unsafe { sys::elg_app_new() };
        if let Err(error) = error::result() {
            panic!("elegance: couldn't start: {error}");
        }
        let app = Self {
            inner: Rc::new(AppPtr(NonNull::new(ptr).expect("elegance: no app"))),
            on_error: Rc::default(),
        };
        crate::error::ffi!(sys::elg_app_on_error(app.ptr(), report_closure(Rc::clone(&app.on_error))));
        app
    }

    fn ptr(&self) -> *mut sys::App {
        self.inner.0.as_ptr()
    }

    /// Opens a window when the app runs. `content` (a component, or any view) is turned into
    /// the window's root view once, on the UI thread, when the window opens.
    pub fn window(&self, desc: WindowDesc, content: impl IntoView + 'static) -> &Self {
        let content = RefCell::new(Some(content));
        let content = cffi::closure(move |_, ret| unsafe {
            let view = content.borrow_mut().take().map(IntoView::into_view).unwrap_or_default();
            view.write_into(ret);
        });
        let c = sys::ElgWindowDesc {
            title: cffi::s(&desc.title),
            width: desc.width,
            height: desc.height,
            resizable: desc.resizable,
            has_min_size: desc.min_size.is_some(),
            min_size: desc.min_size.unwrap_or_default().c(),
            has_background: desc.background.is_some(),
            background: desc.background.unwrap_or_default().c(),
            title_bar: desc.title_bar as i32,
            has_system_controls_position: desc.system_controls_position.is_some(),
            system_controls_position: desc.system_controls_position.unwrap_or_default().c(),
            transparent: desc.transparent,
            opacity: desc.opacity,
            fullscreen: desc.fullscreen,
        };
        crate::error::ffi!(sys::elg_app_window(self.ptr(), &c, content));
        self
    }

    /// Sets the error handler. It hears each panic and error that escapes the app's code while the
    /// app runs (bodies, effects, handlers, tasks, timers), on the UI thread between events, after
    /// Elegance logged it. Without a handler, the app goes on. Callable before or during `run`.
    ///
    /// The handler returns [`ErrorAction::Resume`] to go on, or [`ErrorAction::Quit`] to end the
    /// run: `run` then resumes the panic or returns the error. What it panics with itself is only
    /// logged.
    pub fn on_error(&self, handler: impl FnMut(&AppError) -> ErrorAction + 'static) -> &Self {
        *self.on_error.borrow_mut() = Some(Box::new(handler));
        self
    }

    /// Opens the windows and runs the event loop until the last window closes or `quit` is
    /// called. A panic or an error in the app's code is logged and goes to `on_error`'s handler,
    /// and the app goes on, unless the handler ends the run: then the panic resumes here, or the
    /// error comes back here. So do Elegance's own errors, after a clean shutdown.
    pub fn run(&self) -> Result<ExitCode, Error> {
        let code = self.run_code()?;
        Ok(ExitCode::from(code.clamp(0, 255) as u8))
    }

    /// Like `run`, with the exit code as a number.
    pub fn run_code(&self) -> Result<i32, Error> {
        let code = unsafe { sys::elg_app_run(self.ptr()) };
        error::result()?;
        Ok(code)
    }

    /// Makes `run` return after the current event. UI thread.
    pub fn quit(&self) {
        crate::error::ffi!(sys::elg_app_quit(self.ptr()));
    }

    /// A handle for quitting from inside the app (closures can't borrow the App).
    pub fn handle(&self) -> AppHandle {
        AppHandle { inner: Rc::downgrade(&self.inner) }
    }
}

/// Quits the app it came from, if it's still there.
#[derive(Clone)]
pub struct AppHandle {
    inner: Weak<AppPtr>,
}

impl AppHandle {
    pub fn quit(&self) {
        if let Some(app) = self.inner.upgrade() {
            crate::error::ffi!(sys::elg_app_quit(app.0.as_ptr()));
        }
    }
}

/// Runs `task` on the UI thread soon, between events, where state may be written. For results
/// of work done on your own threads. Call from any thread; tasks run in the order posted.
pub fn post_to_ui_thread(task: impl FnOnce() + Send + 'static) {
    let task = std::sync::Mutex::new(Some(task));
    let task = cffi::send_closure(move |_, _| {
        if let Some(task) = task.lock().unwrap_or_else(|e| e.into_inner()).take() {
            task();
        }
    });
    crate::error::ffi!(sys::elg_post_to_ui_thread(task));
}

/// Elegance's version, e.g. "0.1.0".
pub fn version() -> &'static str {
    let ptr = unsafe { sys::elg_version() };
    unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use std::ffi::c_void;

    use super::*;
    use crate::error::AppCode;

    // Calls the report closure as Elegance does: (whether it succeeded, what it set `ret` to).
    fn report(closure: &sys::ElgClosure, kind: i32, code: i32) -> (bool, i32) {
        let error = sys::ElgAppError { kind, message: cffi::s("a Rust callback panicked"), code };
        let mut quit = 0i32;
        let call = closure.call.expect("a closure");
        let succeeded = unsafe {
            call(closure.data, &error as *const sys::ElgAppError as *mut c_void, &mut quit as *mut i32 as *mut c_void)
        };
        (succeeded, quit)
    }

    fn free(closure: sys::ElgClosure) {
        unsafe { (closure.drop.expect("owned"))(closure.data) }
    }

    fn panic_with(message: &'static str) {
        cffi::store_panic(Box::new(message));
    }

    #[test]
    fn a_reported_panic_goes_to_the_handler_and_doesnt_resume_later() {
        let slot: Rc<RefCell<Option<ErrorHandler>>> = Rc::default();
        let heard = Rc::new(RefCell::new(Vec::new()));
        let record = Rc::clone(&heard);
        *slot.borrow_mut() = Some(Box::new(move |error: &AppError| {
            record.borrow_mut().push((error.message().to_owned(), error.code(), error.panicked()));
            ErrorAction::Resume
        }));
        let closure = report_closure(Rc::clone(&slot));
        panic_with("boom");
        assert_eq!(report(&closure, 1, 0), (true, 0));
        assert!(cffi::take_panic().is_none());
        assert_eq!(report(&closure, 2, 4), (true, 0)); // a usage error, from a timer
        assert_eq!(
            *heard.borrow(),
            vec![
                ("boom".to_owned(), AppCode::Body, true),
                ("a Rust callback panicked".to_owned(), AppCode::Timer, false),
            ]
        );
        free(closure);
    }

    #[test]
    fn without_a_handler_a_reported_panic_is_dropped() {
        let closure = report_closure(Rc::default());
        panic_with("boom");
        assert_eq!(report(&closure, 1, 2), (true, 0));
        assert!(cffi::take_panic().is_none());
        free(closure);
    }

    #[test]
    fn a_handler_that_quits_keeps_the_panic_for_run() {
        let slot: Rc<RefCell<Option<ErrorHandler>>> = Rc::default();
        *slot.borrow_mut() = Some(Box::new(|_: &AppError| ErrorAction::Quit));
        let closure = report_closure(Rc::clone(&slot));
        panic_with("boom");
        assert_eq!(report(&closure, 1, 0), (true, 1));
        let payload = cffi::take_panic().expect("kept");
        assert_eq!(cffi::panic_message(payload.as_ref()), Some("boom"));
        free(closure);
    }

    #[test]
    fn a_handler_that_panics_fails_the_call_and_leaves_no_panic() {
        let slot: Rc<RefCell<Option<ErrorHandler>>> = Rc::default();
        *slot.borrow_mut() = Some(Box::new(|_: &AppError| panic!("the handler broke")));
        let closure = report_closure(Rc::clone(&slot));
        assert_eq!(report(&closure, 2, 0), (false, 0));
        assert!(cffi::take_panic().is_none());
        assert!(slot.borrow().is_some()); // still the handler
        free(closure);
    }
}
