//! The operating system the app was built for.

/// An operating system Elegance runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

/// The platform the app was built for: [`Platform::Windows`], [`Platform::MacOS`] or
/// [`Platform::Linux`] (anything else counts as Linux).
pub fn current_platform() -> Platform {
    match unsafe { elegance_sys::elg_current_platform() } {
        0 => Platform::Windows,
        1 => Platform::MacOS,
        _ => Platform::Linux,
    }
}
