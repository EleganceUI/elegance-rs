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
        2 => ErrorKind::Usage,
        3 => ErrorKind::Resource,
        4 => ErrorKind::Gpu,
        5 => ErrorKind::Platform,
        6 => ErrorKind::Elegance,
        _ => ErrorKind::Other,
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
