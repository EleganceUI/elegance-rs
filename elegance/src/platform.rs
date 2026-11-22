//! The operating system the app was built for.

/// An operating system Elegance runs on.
///
/// Later ABI levels of the library may add platforms. A platform this binding has no name for is
/// [`Platform::Unknown`], which holds the library's value for it; match with a `_` arm for
/// the platforms a later version of this enum may name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    Mikros,
    /// A platform this binding doesn't know, with the value the library gave for it.
    Unknown(i32),
}

impl Platform {
    /// The platform the library's value `value` stands for: [`Platform::Unknown`] for one this
    /// binding doesn't know.
    pub(crate) fn from_raw(value: i32) -> Platform {
        match value {
            0 => Platform::Windows,
            1 => Platform::MacOS,
            2 => Platform::Linux,
            3 => Platform::Mikros,
            other => Platform::Unknown(other),
        }
    }
}

/// The platform the app was built for: [`Platform::Windows`], [`Platform::MacOS`],
/// [`Platform::Linux`] or [`Platform::Mikros`].
///
/// Later ABI levels of the library may add platforms: one this binding doesn't know is returned
/// as [`Platform::Unknown`].
pub fn current_platform() -> Platform {
    Platform::from_raw(unsafe { elegance_sys::elg_current_platform() })
}

#[cfg(test)]
mod tests {
    use super::Platform;

    #[test]
    fn known_values_are_named() {
        assert_eq!(Platform::from_raw(0), Platform::Windows);
        assert_eq!(Platform::from_raw(1), Platform::MacOS);
        assert_eq!(Platform::from_raw(2), Platform::Linux);
        assert_eq!(Platform::from_raw(3), Platform::Mikros);
    }

    #[test]
    fn other_values_stay_unknown() {
        for value in [4, 5, 100, -1, i32::MAX] {
            assert_eq!(Platform::from_raw(value), Platform::Unknown(value));
        }
    }
}
