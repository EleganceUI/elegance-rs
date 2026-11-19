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
pub const fn current_platform() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOS
    } else {
        Platform::Linux
    }
}
