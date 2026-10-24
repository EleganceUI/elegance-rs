//! Hot reload: a view that shows a module (a DLL built from your views) and reloads it when
//! its sources change.

use std::path::PathBuf;

use elegence_sys as sys;

use crate::dsl::ElgNew;
use crate::error::ffi;
use crate::ffi as cffi;
use crate::view::{IntoView, View};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HotReloadShow {
    /// The module's root view.
    #[default]
    Root,
    /// A list of the module's previews and the selected one.
    Previews,
}

/// Shows an Elegence module and rebuilds it (`build_command`) when files in `watch` change.
pub struct HotReload {
    module: PathBuf,
    build_command: String,
    watch: Vec<PathBuf>,
    show: HotReloadShow,
}

impl HotReload {
    pub fn new(module: impl Into<PathBuf>) -> Self {
        Self { module: module.into(), build_command: String::new(), watch: Vec::new(), show: HotReloadShow::Root }
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
}

impl<P: Into<PathBuf>> ElgNew<(P,)> for HotReload {
    type Out = HotReload;
    fn elg_new((module,): (P,)) -> HotReload {
        HotReload::new(module)
    }
}

impl IntoView for HotReload {
    fn into_view(self) -> View {
        let module = cffi::path_str(&self.module);
        let watch: Vec<String> = self.watch.iter().map(|p| cffi::path_str(p)).collect();
        let watch: Vec<sys::ElgStr> = watch.iter().map(|w| cffi::s(w)).collect();
        View::from_raw(ffi!(sys::elg_hot_reload(cffi::s(&module), cffi::s(&self.build_command), watch.as_ptr(), watch.len(), self.show as i32)))
    }
}
