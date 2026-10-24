//! File dialogs, notifications, and tray icons. Callbacks run on the UI thread; call these on
//! the UI thread of a running App.

use std::path::PathBuf;
use std::ptr::NonNull;

use elegence_sys as sys;

use crate::error::{self, ffi, Error};
use crate::ffi as cffi;
use crate::image::ImageSource;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileFilter {
    /// "Images"
    pub name: String,
    /// "png", "jpg"
    pub extensions: Vec<String>,
}

impl FileFilter {
    pub fn new(name: impl Into<String>, extensions: &[&str]) -> Self {
        Self { name: name.into(), extensions: extensions.iter().map(|e| e.to_string()).collect() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenFilesOptions {
    pub filters: Vec<FileFilter>,
    /// Where it opens (empty: the system's choice).
    pub directory: PathBuf,
    pub multiple: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveFileOptions {
    pub filters: Vec<FileFilter>,
    pub directory: PathBuf,
    /// The suggested file name.
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChooseFolderOptions {
    pub directory: PathBuf,
    pub multiple: bool,
}

fn with_filters<R>(filters: &[FileFilter], f: impl FnOnce(&[sys::ElgFileFilter]) -> R) -> R {
    let extensions: Vec<Vec<sys::ElgStr>> = filters.iter().map(|f| f.extensions.iter().map(|e| cffi::s(e)).collect()).collect();
    let c: Vec<sys::ElgFileFilter> = filters
        .iter()
        .zip(&extensions)
        .map(|(filter, ext)| sys::ElgFileFilter { name: cffi::s(&filter.name), extensions: ext.as_ptr(), extension_count: ext.len() })
        .collect();
    f(&c)
}

fn paths_closure(done: impl FnOnce(Vec<PathBuf>) + 'static) -> sys::ElgClosure {
    let done = std::cell::RefCell::new(Some(done));
    cffi::closure(move |args, _| unsafe {
        let paths = &*(args as *const sys::ElgPaths);
        let paths = std::slice::from_raw_parts(paths.paths, paths.count).iter().map(|p| PathBuf::from(p.as_str())).collect();
        if let Some(done) = done.borrow_mut().take() {
            done(paths);
        }
    })
}

/// Shows an open dialog; `done` gets the chosen files (none if cancelled).
pub fn choose_files(options: OpenFilesOptions, done: impl FnOnce(Vec<PathBuf>) + 'static) {
    let directory = cffi::path_str(&options.directory);
    with_filters(&options.filters, |filters| {
        ffi!(sys::elg_choose_files(filters.as_ptr(), filters.len(), cffi::s(&directory), options.multiple, paths_closure(done)))
    });
}

/// Shows a save dialog; `done` gets the chosen path (None if cancelled).
pub fn choose_save_file(options: SaveFileOptions, done: impl FnOnce(Option<PathBuf>) + 'static) {
    let directory = cffi::path_str(&options.directory);
    with_filters(&options.filters, |filters| {
        ffi!(sys::elg_choose_save_file(
            filters.as_ptr(),
            filters.len(),
            cffi::s(&directory),
            cffi::s(&options.name),
            paths_closure(move |paths| done(paths.into_iter().next()))
        ))
    });
}

pub fn choose_folders(options: ChooseFolderOptions, done: impl FnOnce(Vec<PathBuf>) + 'static) {
    let directory = cffi::path_str(&options.directory);
    ffi!(sys::elg_choose_folders(cffi::s(&directory), options.multiple, paths_closure(done)));
}

/// A system notification.
#[derive(Default)]
pub struct Notification {
    pub title: String,
    pub body: String,
    /// When the notification is clicked.
    pub on_click: Option<Box<dyn FnMut()>>,
}

impl Notification {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self { title: title.into(), body: body.into(), on_click: None }
    }
    pub fn on_click(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

pub fn notify(notification: Notification) {
    ffi!(sys::elg_notify(cffi::s(&notification.title), cffi::s(&notification.body), cffi::optional_action(notification.on_click)));
}

/// An item of a tray icon's menu.
#[derive(Default)]
pub struct TrayMenuItem {
    /// Empty: a separator.
    pub title: String,
    /// When chosen.
    pub action: Option<Box<dyn FnMut()>>,
    /// A check box, initially checked or not.
    pub checked: Option<bool>,
    /// Check boxes: when chosen, with the new state.
    pub toggled: Option<Box<dyn Fn(bool)>>,
    pub enabled: bool,
    pub submenu: Vec<TrayMenuItem>,
}

impl TrayMenuItem {
    pub fn new(title: impl Into<String>, action: impl FnMut() + 'static) -> Self {
        Self { title: title.into(), action: Some(Box::new(action)), enabled: true, ..Default::default() }
    }
    pub fn separator() -> Self {
        Self { enabled: true, ..Default::default() }
    }
    pub fn check_box(title: impl Into<String>, checked: bool, toggled: impl Fn(bool) + 'static) -> Self {
        Self { title: title.into(), checked: Some(checked), toggled: Some(Box::new(toggled)), enabled: true, ..Default::default() }
    }
    pub fn submenu(title: impl Into<String>, items: Vec<TrayMenuItem>) -> Self {
        Self { title: title.into(), submenu: items, enabled: true, ..Default::default() }
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

// The C items of a menu; titles and submenus are borrowed from `items`, which outlive it.
struct CMenu {
    items: Vec<sys::ElgTrayMenuItem>,
    _submenus: Vec<CMenu>,
}

fn c_menu(items: Vec<TrayMenuItem>, titles: &mut Vec<String>) -> CMenu {
    let mut c = Vec::new();
    let mut submenus = Vec::new();
    for item in items {
        let sub = c_menu(item.submenu, titles);
        titles.push(item.title);
        let title = titles.last().unwrap();
        c.push(sys::ElgTrayMenuItem {
            // Strings in `titles` don't move: their buffers stay put as the Vec grows.
            title: cffi::s(title),
            action: cffi::optional_action(item.action),
            checked: item.checked.map_or(-1, i32::from),
            toggled: item.toggled.map(|f| cffi::consumer(move |b: bool| f(b))).unwrap_or(sys::ElgClosure::NONE),
            enabled: item.enabled,
            submenu: sub.items.as_ptr(),
            submenu_count: sub.items.len(),
        });
        submenus.push(sub);
    }
    CMenu { items: c, _submenus: submenus }
}

#[derive(Default)]
pub struct TrayIconOptions {
    /// A file or data image; empty: a plain dot in the theme's accent color.
    pub icon: ImageSource,
    pub tooltip: String,
    pub menu: Vec<TrayMenuItem>,
}

/// An icon in the system tray (notification area) with a menu. It's removed when the last clone
/// goes.
pub struct TrayIcon {
    ptr: NonNull<sys::TrayIcon>,
}

impl Clone for TrayIcon {
    fn clone(&self) -> Self {
        Self { ptr: NonNull::new(unsafe { sys::elg_tray_icon_clone(self.ptr.as_ptr()) }).unwrap() }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe { sys::elg_tray_icon_free(self.ptr.as_ptr()) }
    }
}

impl TrayIcon {
    pub fn new(options: TrayIconOptions) -> Result<TrayIcon, Error> {
        let mut titles = Vec::new();
        let menu = c_menu(options.menu, &mut titles);
        let icon = options.icon.c();
        let ptr = unsafe { sys::elg_tray_icon_new(&icon, cffi::s(&options.tooltip), menu.items.as_ptr(), menu.items.len()) };
        error::result()?;
        Ok(Self { ptr: NonNull::new(ptr).unwrap() })
    }
    pub fn set_tooltip(&self, tooltip: &str) {
        ffi!(sys::elg_tray_icon_set_tooltip(self.ptr.as_ptr(), cffi::s(tooltip)));
    }
    pub fn set_menu(&self, menu: Vec<TrayMenuItem>) {
        let mut titles = Vec::new();
        let menu = c_menu(menu, &mut titles);
        ffi!(sys::elg_tray_icon_set_menu(self.ptr.as_ptr(), menu.items.as_ptr(), menu.items.len()));
    }
}
