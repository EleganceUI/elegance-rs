//! The application: windows and the event loop.

use std::cell::RefCell;
use std::process::ExitCode;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};

use elegance_sys as sys;

use crate::color::Color;
use crate::error::{self, Error};
use crate::events::Cursor;
use crate::ffi as cffi;
use crate::geometry::Size;
use crate::view::IntoView;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TitleBar {
    /// The platform's title bar and frame.
    #[default]
    System,
    /// None: the content fills the window. Draw your own title bar with `.window_drag_area()`
    /// and `WindowControls()`; edges still resize.
    Hidden,
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
            transparent: false,
            opacity: 1.0,
            fullscreen: false,
        }
    }
}

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
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    #[track_caller]
    pub fn new() -> Self {
        let ptr = unsafe { sys::elg_app_new() };
        if let Err(error) = error::result() {
            panic!("elegance: couldn't start: {error}");
        }
        Self { inner: Rc::new(AppPtr(NonNull::new(ptr).expect("elegance: no app"))) }
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
            transparent: desc.transparent,
            opacity: desc.opacity,
            fullscreen: desc.fullscreen,
        };
        crate::error::ffi!(sys::elg_app_window(self.ptr(), &c, content));
        self
    }

    /// Opens the windows and runs the event loop until the last window closes or `quit` is
    /// called. Errors raised by Elegance come back here after a clean shutdown; a panic in a
    /// callback resumes here.
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

/// Sets the mouse pointer's shape until something else sets it. UI thread.
pub fn set_cursor(cursor: impl Into<Cursor>) {
    crate::error::ffi!(sys::elg_set_cursor(cursor.into() as i32));
}
