//! Raw bindings to Elegance's C API: the header (`include/elg-c/elegance.h`) of the Elegance commit
//! `elegance-rev` pins, which `generate.py` turns into `bindings.rs` with bindgen. Use the
//! `elegance` crate; everything here is unsafe.
//!
//! Conventions (see the header): pointer arguments named `take_*`, and arrays of view pointers,
//! are consumed; other pointers are borrowed; returned pointers are owned by the caller. A C++
//! exception becomes a pending error, read with [`elg_take_error`].

#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case, clippy::missing_safety_doc)]

include!("bindings.rs");

mod layouts;
pub use layouts::{LAYOUTS, Layout};

pub mod gpu_image;

// The handle types by the names the `elegance` crate gives them.
pub type View = ElgView;
pub type Text = ElgText;
pub type Shape = ElgShape;
pub type Path = ElgPath;
pub type Animation = ElgAnimation;
pub type Transition = ElgTransition;
pub type EnvironmentValues = ElgEnvironmentValues;
pub type WindowHandle = ElgWindowHandle;
pub type App = ElgApp;
pub type BindingBool = ElgBindingBool;
pub type BindingF32 = ElgBindingF32;
pub type BindingI32 = ElgBindingI32;
pub type BindingString = ElgBindingString;
pub type BindingF64 = ElgBindingF64;
pub type ScrollPosition = ElgScrollPosition;
pub type GraphicsContext = ElgGraphicsContext;
pub type ButtonStyle = ElgButtonStyle;
pub type ToggleStyle = ElgToggleStyle;
pub type CheckboxStyle = ElgCheckboxStyle;
pub type NumberInputStyle = ElgNumberInputStyle;
pub type ProgressStyle = ElgProgressStyle;
pub type TextFieldStyle = ElgTextFieldStyle;
pub type PickerStyle = ElgPickerStyle;
pub type SliderStyle = ElgSliderStyle;
pub type NavigationStackStyle = ElgNavigationStackStyle;
pub type PopoverStyle = ElgPopoverStyle;
pub type TooltipStyle = ElgTooltipStyle;
pub type MenuStyle = ElgMenuStyle;
pub type RowStyle = ElgRowStyle;
pub type ButtonConfiguration = ElgButtonConfiguration;
pub type ToggleConfiguration = ElgToggleConfiguration;
pub type CheckboxConfiguration = ElgCheckboxConfiguration;
pub type NumberInputConfiguration = ElgNumberInputConfiguration;
pub type ProgressConfiguration = ElgProgressConfiguration;
pub type TextFieldConfiguration = ElgTextFieldConfiguration;
pub type PickerConfiguration = ElgPickerConfiguration;
pub type SliderConfiguration = ElgSliderConfiguration;
pub type NavigationStackConfiguration = ElgNavigationStackConfiguration;
pub type PopoverConfiguration = ElgPopoverConfiguration;
pub type TooltipConfiguration = ElgTooltipConfiguration;
pub type MenuConfiguration = ElgMenuConfiguration;
pub type MenuItemConfiguration = ElgMenuItemConfiguration;
pub type RowConfiguration = ElgRowConfiguration;
pub type LayoutCache = ElgLayoutCache;
pub type Effects = ElgEffects;
pub type Sound = ElgSound;
pub type AudioPlayer = ElgAudioPlayer;
pub type AudioStream = ElgAudioStream;
pub type AudioRecorder = ElgAudioRecorder;
pub type Camera = ElgCamera;
pub type VideoPlayer = ElgVideoPlayer;
pub type VideoFeed = ElgVideoFeed;
pub type TrayIcon = ElgTrayIcon;
pub type Timer = ElgTimer;
pub type PropF32 = ElgPropF32;
pub type PropColor = ElgPropColor;
pub type Theme = ElgTheme;
pub type ModuleEntry = ElgModuleEntry;
pub type CppString = ElgCppString;
pub type TextSpans = ElgTextSpans;
pub type ForEachItems = ElgForEachItems;
pub type Strings = ElgStrings;
pub type Font = ElgCppFont;
pub type ShapeStyle = ElgCppShapeStyle;
pub type OptFloat = ElgOptFloat;
pub type ItemKeys = ElgItemKeys;

pub type LayoutChild = ElgLayoutChild;

impl ElgStr {
    pub fn new(s: &str) -> Self {
        Self {
            ptr: s.as_ptr().cast(),
            len: s.len(),
        }
    }
    pub const EMPTY: Self = Self {
        ptr: std::ptr::null(),
        len: 0,
    };
    /// # Safety
    /// The pointer must be valid UTF-8 for `len` bytes, for `'a`.
    pub unsafe fn as_str<'a>(&self) -> &'a str {
        unsafe {
            if self.ptr.is_null() {
                ""
            } else {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.ptr.cast(), self.len))
            }
        }
    }
}

impl ElgBytes {
    pub fn new(b: &[u8]) -> Self {
        Self {
            ptr: b.as_ptr(),
            len: b.len(),
        }
    }
    pub const EMPTY: Self = Self {
        ptr: std::ptr::null(),
        len: 0,
    };
    /// # Safety
    /// The pointer must be valid for `len` bytes, for `'a`.
    pub unsafe fn as_slice<'a>(&self) -> &'a [u8] {
        unsafe {
            if self.ptr.is_null() {
                &[]
            } else {
                std::slice::from_raw_parts(self.ptr, self.len)
            }
        }
    }
}

impl ElgClosure {
    pub const NONE: Self = Self {
        data: std::ptr::null_mut(),
        call: None,
        drop: None,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{c_char, c_void};

    unsafe extern "C" fn append(ctx: *mut c_void, ptr: *const c_char, len: usize) {
        let text = unsafe { &mut *ctx.cast::<String>() };
        let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) };
        text.push_str(std::str::from_utf8(bytes).unwrap());
    }

    /// The library is the one these declarations were made for.
    #[test]
    fn abi_version_matches_the_header() {
        assert_eq!(unsafe { elg_abi_version() }, ELG_ABI_VERSION);
    }

    /// Every struct the library reports has, in Rust, its size, its alignment, and each of its
    /// fields at its offset, in order; and every struct Rust declares is one it reports.
    #[test]
    fn struct_layouts_match_the_library() {
        let mut text = String::new();
        for layouts in [
            elg_layouts_core,
            elg_layouts_views,
            elg_layouts_ext,
            elg_layouts_platform,
            elg_layouts_styles,
        ] {
            unsafe { layouts(Some(append), (&raw mut text).cast()) };
        }
        let mut reported = Vec::new();
        for line in text.lines() {
            let mut words = line.split(' ');
            let name = words.next().unwrap();
            let size: usize = words.next().unwrap().parse().unwrap();
            let align: usize = words.next().unwrap().parse().unwrap();
            let fields: Vec<(&str, usize)> = words
                .map(|field| {
                    let (field, offset) = field.split_once(':').unwrap();
                    (field, offset.parse().unwrap())
                })
                .collect();
            let (_, rust_size, rust_align, rust_fields) = LAYOUTS
                .iter()
                .find(|(rust, ..)| *rust == name)
                .unwrap_or_else(|| panic!("the library reports {name}, which Rust doesn't declare"));
            assert_eq!((size, align), (*rust_size, *rust_align), "{name}'s size and alignment");
            assert_eq!(fields, rust_fields.to_vec(), "{name}'s fields and their offsets");
            reported.push(name);
        }
        for (name, ..) in LAYOUTS {
            assert!(reported.contains(name), "Rust declares {name}, which the library doesn't report");
        }
    }
}
