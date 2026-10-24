//! Views modules: a library of views that Elegence's preview host (`elegence-preview`) loads,
//! shows, and reloads when its sources change.
//!
//! ```ignore
//! // src/lib.rs of a crate with `crate-type = ["cdylib", "rlib"]`
//! #[preview("Counter")]
//! fn counter() -> impl IntoView {
//!     elg! { Counter() }
//! }
//!
//! elegence::module!(Root); // or `elegence::module!();` for previews only
//! ```
//!
//! ```text
//! elegence-preview --module target/debug/my_views.dll --build "cargo build --lib" --watch src
//! ```
//!
//! `--root` shows the module's root view instead of the previews, for working on the whole app.

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;

use elegence_sys as sys;

use crate::ffi;
use crate::view::View;

/// A preview, registered by `#[preview]`.
#[doc(hidden)]
pub struct PreviewItem {
    pub name: &'static str,
    pub body: fn() -> View,
    pub file: &'static str,
    pub line: u32,
}

inventory::collect!(PreviewItem);

static ROOT: OnceLock<Option<fn() -> View>> = OnceLock::new();
static PREVIEWS: OnceLock<Vec<&'static PreviewItem>> = OnceLock::new();

/// The module's entry (`elegence::module!`). Each loaded copy of a module has its own statics.
#[doc(hidden)]
pub fn module_entry(root: Option<fn() -> View>) -> *const c_void {
    let _ = ROOT.set(root);
    let previews = PREVIEWS.get_or_init(|| {
        let mut previews: Vec<&'static PreviewItem> = inventory::iter::<PreviewItem>.into_iter().collect();
        previews.sort_by_key(|p| (p.file, p.line));
        previews
    });
    let descs: Vec<sys::ElgPreviewDesc> = previews
        .iter()
        .map(|p| sys::ElgPreviewDesc { name: ffi::s(p.name), file: ffi::s(p.file), line: p.line as i32 })
        .collect();
    unsafe { sys::elg_rust_module_entry(root.is_some(), descs.as_ptr(), descs.len(), build) }
}

unsafe extern "C" fn build(index: i32, ret: *mut sys::View) -> bool {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let body = if index < 0 {
            ROOT.get().copied().flatten().expect("elegence: this module has no root view")
        } else {
            PREVIEWS.get().expect("elegence: module not loaded")[index as usize].body
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

/// Makes this crate a views module for the preview host: exports its entry point.
/// `module!(Root)` gives it a root view (shown with `elegence-preview --root`); `module!()` has
/// previews only.
#[macro_export]
macro_rules! module {
    () => {
        $crate::module!(@entry ::std::option::Option::None);
    };
    ($root:expr) => {
        $crate::module!(@entry ::std::option::Option::Some(
            (|| $crate::IntoView::into_view($root)) as fn() -> $crate::View
        ));
    };
    (@entry $root:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub extern "C" fn elg_module_entry() -> *const ::std::ffi::c_void {
            $crate::__private::module_entry($root)
        }
    };
}
