//! Previews: views that Elegance's preview host (`elegance-preview`) shows, and rebuilds and
//! reloads as you edit. Write them anywhere in an app:
//!
//! ```ignore
//! #[preview("Counter")]
//! fn counter() -> impl IntoView {
//!     elg! { Counter() }
//! }
//!
//! // The whole app, for `cargo elegance preview --root`.
//! #[preview(root)]
//! fn app() -> impl IntoView {
//!     elg! { Root() }
//! }
//! ```
//!
//! ```text
//! cargo elegance preview          # the previews
//! cargo elegance preview --root   # the root view
//! ```
//!
//! `cargo elegance preview` builds the app's own binary target as a views module (a DLL) with the
//! `preview` feature, which exports the module entry below and registers the previews. Ordinary
//! builds leave the previews out.

use crate::view::View;

/// A preview, registered by `#[preview]` in preview builds.
#[doc(hidden)]
pub struct PreviewItem {
    pub name: &'static str,
    /// The module's root view (`#[preview(root)]`) rather than one of its previews.
    pub root: bool,
    pub body: fn() -> View,
    pub file: &'static str,
    pub line: u32,
}

inventory::collect!(PreviewItem);

/// `#[preview]`'s registration: an inventory entry in preview builds, nothing otherwise.
#[cfg(feature = "preview")]
#[doc(hidden)]
#[macro_export]
macro_rules! __preview {
    ($name:expr, $root:expr, $function:path) => {
        $crate::__private::inventory::submit! {
            $crate::__private::PreviewItem {
                name: $name,
                root: $root,
                body: || $crate::IntoView::into_view($function()),
                file: ::std::file!(),
                line: ::std::line!(),
            }
        }
    };
}

#[cfg(not(feature = "preview"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __preview {
    ($($tokens:tt)*) => {};
}

// The module entry, in preview builds.
#[cfg(feature = "preview")]
mod entry {
    use std::ffi::c_void;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::OnceLock;

    use elegance_sys as sys;

    use super::PreviewItem;
    use crate::ffi;
    use crate::view::View;

    struct Module {
        root: Option<fn() -> View>,
        previews: Vec<&'static PreviewItem>,
    }

    static MODULE: OnceLock<Module> = OnceLock::new();

    /// The views module's entry, which the preview host looks up. Each loaded copy of a module has
    /// its own statics.
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn elg_module_entry() -> *const c_void {
        module_entry()
    }

    fn module_entry() -> *const c_void {
        let module = MODULE.get_or_init(|| {
            let mut items: Vec<&'static PreviewItem> = inventory::iter::<PreviewItem>.into_iter().collect();
            items.sort_by_key(|p| (p.file, p.line));
            let root = items.iter().find(|p| p.root).map(|p| p.body);
            Module { root, previews: items.into_iter().filter(|p| !p.root).collect() }
        });
        let descs: Vec<sys::ElgPreviewDesc> = module
            .previews
            .iter()
            .map(|p| sys::ElgPreviewDesc { name: ffi::s(p.name), file: ffi::s(p.file), line: p.line as i32 })
            .collect();
        unsafe { sys::elg_rust_module_entry(module.root.is_some(), descs.as_ptr(), descs.len(), build) }
    }

    unsafe extern "C" fn build(index: i32, ret: *mut sys::View) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let module = MODULE.get().expect("elegance: module not loaded");
            let body = if index < 0 {
                module.root.expect("elegance: this module has no root view; mark one with #[preview(root)]")
            } else {
                module.previews[index as usize].body
            };
            body().write_into(ret as *mut c_void);
        }));
        match result {
            Ok(()) => true,
            Err(payload) => {
                ffi::store_panic(payload);
                false
            }
        }
    }
}
