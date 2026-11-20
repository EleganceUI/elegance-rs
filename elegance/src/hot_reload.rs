//! Hot reload: a view that shows a module (a DLL built from your views) and reloads it when
//! its sources change. It needs the `dynamic` feature: the module and the app showing it must
//! share one Elegance (elegance.dll), which a statically linked app can't.
//!
//! To preview an app's views, `cargo elegance preview` is simpler (see [`crate::preview`]).

#[cfg(feature = "dynamic")]
use std::path::PathBuf;

#[cfg(feature = "dynamic")]
use elegance_sys as sys;

#[cfg(feature = "dynamic")]
use crate::dsl::ElgNew;
#[cfg(feature = "dynamic")]
use crate::error::ffi;
#[cfg(feature = "dynamic")]
use crate::ffi as cffi;
#[cfg(feature = "dynamic")]
use crate::view::{IntoView, View};

/// A hash of the sizes and alignments of the types an app's host shares with its views module (its
/// stores, the values it puts in the environment), as Elegance's `layout_hash<T...>()`: 64-bit
/// FNV-1a of each type's size and alignment, in order. Give the same hash to
/// `#[preview(root, app_hash = ...)]` and [`HotReload::app_hash`], and a reload that changes those
/// types asks for a restart instead of mounting the module. Any other value both sides compute
/// works too, such as a number the app changes along with those types.
///
/// ```ignore
/// const APP_HASH: u64 = elegance::layout_hash!(Store, Settings);
/// ```
#[macro_export]
macro_rules! layout_hash {
    ($($t:ty),* $(,)?) => {
        $crate::hot_reload::layout_hash(&[$((::std::mem::size_of::<$t>(), ::std::mem::align_of::<$t>())),*])
    };
}

/// [`layout_hash!`]'s hash of `(size, alignment)` pairs, in order.
pub const fn layout_hash(layouts: &[(usize, usize)]) -> u64 {
    const fn mix(mut hash: u64, value: u64) -> u64 {
        let mut byte = 0;
        while byte < 8 {
            hash = (hash ^ ((value >> (byte * 8)) & 0xFF)).wrapping_mul(1099511628211);
            byte += 1;
        }
        hash
    }
    let mut hash: u64 = 14695981039346656037;
    let mut i = 0;
    while i < layouts.len() {
        hash = mix(hash, layouts[i].0 as u64);
        hash = mix(hash, layouts[i].1 as u64);
        i += 1;
    }
    hash
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HotReloadShow {
    /// The module's root view.
    #[default]
    Root,
    /// A list of the module's previews and the selected one.
    Previews,
}

#[cfg(feature = "dynamic")]
/// Shows an Elegance module and rebuilds it (`build_command`) when files in `watch` change.
pub struct HotReload {
    module: PathBuf,
    build_command: String,
    watch: Vec<PathBuf>,
    show: HotReloadShow,
    app_hash: u64,
}

#[cfg(feature = "dynamic")]
#[elegance_macros::elg_builder]
impl HotReload {
    pub fn new(module: impl Into<PathBuf>) -> Self {
        Self { module: module.into(), build_command: String::new(), watch: Vec::new(), show: HotReloadShow::Root, app_hash: 0 }
    }
    pub fn build_command(mut self, command: impl Into<String>) -> Self {
        self.build_command = command.into();
        self
    }
    pub fn watch<P: Into<PathBuf>>(mut self, paths: impl IntoIterator<Item = P>) -> Self {
        self.watch.extend(paths.into_iter().map(Into::into));
        self
    }
    pub fn show(mut self, show: impl Into<HotReloadShow>) -> Self {
        self.show = show.into();
        self
    }
    /// The hash of the types this host shares with the module, the one the module gives
    /// `#[preview(root, app_hash = ...)]` (see [`layout_hash!`]): a load whose hash differs isn't
    /// mounted, and asks for a restart. 0 (the default) for a module that has none.
    pub fn app_hash(mut self, app_hash: u64) -> Self {
        self.app_hash = app_hash;
        self
    }
}

#[cfg(feature = "dynamic")]
impl<P: Into<PathBuf>> ElgNew<(P,)> for HotReload {
    type Out = HotReload;
    fn elg_new((module,): (P,)) -> HotReload {
        HotReload::new(module)
    }
}

#[cfg(feature = "dynamic")]
impl IntoView for HotReload {
    fn into_view(self) -> View {
        let module = cffi::path_str(&self.module);
        let watch: Vec<String> = self.watch.iter().map(|p| cffi::path_str(p)).collect();
        let watch: Vec<sys::ElgStr> = watch.iter().map(|w| cffi::s(w)).collect();
        View::from_raw(ffi!(sys::elg_hot_reload(
            cffi::s(&module),
            cffi::s(&self.build_command),
            watch.as_ptr(),
            watch.len(),
            self.show as i32,
            self.app_hash
        )))
    }
}
