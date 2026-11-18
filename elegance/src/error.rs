//! Errors from Elegance (`elg::Error` and its subclasses).

use std::fmt;
use std::panic::resume_unwind;

use elegance_sys as sys;

use crate::ffi as cffi;

/// What went wrong, mirroring Elegance's exception classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// API misuse (`elg::UsageError`): e.g. state written off the UI thread.
    Usage,
    /// A file or resource couldn't be loaded (`elg::ResourceError`).
    Resource,
    /// The GPU failed (`elg::GpuError`).
    Gpu,
    /// The platform layer failed (`elg::PlatformError`).
    Platform,
    /// Another `elg::Error`.
    Elegance,
    /// A C++ exception that isn't an `elg::Error`.
    Other,
}

/// An error raised by Elegance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl ErrorKind {
    // As elg_take_error() numbers them; 1 (a panic) is Other.
    pub(crate) fn from_c(kind: i32) -> Self {
        match kind {
            2 => ErrorKind::Usage,
            3 => ErrorKind::Resource,
            4 => ErrorKind::Gpu,
            5 => ErrorKind::Platform,
            6 => ErrorKind::Elegance,
            _ => ErrorKind::Other,
        }
    }
}

/// The kind of app code that a panic or an error escaped while the app ran (`elg::AppCode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppCode {
    /// A view's body, while the view was mounted.
    Body,
    /// An effect, or the value of a reactive argument.
    Effect,
    /// An event handler, or a control's, menu's or shortcut's action.
    EventHandler,
    /// A task posted to the UI thread.
    Task,
    /// A timer's action.
    Timer,
    /// `on_appear` or `on_disappear`.
    LifecycleAction,
    /// A layout, a layout modifier, or a size observer.
    Layout,
    /// A canvas's draw function.
    Drawing,
    /// Another callback.
    Callback,
}

impl AppCode {
    fn from_c(code: i32) -> Self {
        match code {
            0 => AppCode::Body,
            1 => AppCode::Effect,
            2 => AppCode::EventHandler,
            3 => AppCode::Task,
            4 => AppCode::Timer,
            5 => AppCode::LifecycleAction,
            6 => AppCode::Layout,
            7 => AppCode::Drawing,
            _ => AppCode::Callback,
        }
    }
}

/// What the app's error handler decides (`App::on_error`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorAction {
    /// The app goes on.
    Resume,
    /// The run ends: `App::run` returns the error, or resumes the panic.
    Quit,
}

/// A panic or an error that escaped the app's code while the app ran (`App::on_error`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppError {
    kind: ErrorKind,
    message: String,
    code: AppCode,
    panicked: bool,
}

impl AppError {
    pub(crate) fn from_c(error: &sys::ElgAppError, panic_message: Option<&str>) -> Self {
        let panicked = error.kind == 1;
        let message = match panic_message {
            Some(message) if panicked => message.to_owned(),
            _ => unsafe { error.message.as_str() }.to_owned(),
        };
        Self { kind: ErrorKind::from_c(error.kind), message, code: AppCode::from_c(error.code), panicked }
    }
    /// The error's kind; `Other` for a panic.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
    /// What the panic or the error said.
    pub fn message(&self) -> &str {
        &self.message
    }
    /// The kind of app code it escaped.
    pub fn code(&self) -> AppCode {
        self.code
    }
    /// Whether Rust code panicked.
    pub fn panicked(&self) -> bool {
        self.panicked
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// The pending error, if a shim call failed. A Rust panic that unwound through C++ resumes here.
pub(crate) fn take() -> Option<Error> {
    let mut kind = 0;
    let message = cffi::read_string(|sink, ctx| kind = unsafe { sys::elg_take_error(sink, ctx) });
    let kind = match kind {
        0 => return None,
        1 => {
            if let Some(payload) = cffi::take_panic() {
                resume_unwind(payload);
            }
            ErrorKind::Other
        }
        kind => ErrorKind::from_c(kind),
    };
    Some(Error { kind, message })
}

/// After a shim call: panics with the error, as the C++ API would throw.
#[track_caller]
pub(crate) fn check() {
    if unsafe { sys::elg_error_pending() }
        && let Some(error) = take() {
            panic!("elegance: {error}");
        }
}

/// After a shim call whose failure is part of the API (loading files, ...).
pub(crate) fn result() -> Result<(), Error> {
    if unsafe { sys::elg_error_pending() }
        && let Some(error) = take() {
            return Err(error);
        }
    Ok(())
}

/// Calls the shim and checks for an error.
macro_rules! ffi {
    ($call:expr_2021) => {{
        #[allow(unused_unsafe)]
        let result = unsafe { $call };
        $crate::error::check();
        result
    }};
}

pub(crate) use ffi;
