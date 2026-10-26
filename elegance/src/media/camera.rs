//! Cameras and their views.

use std::ptr::NonNull;

use elegance_sys as sys;

use super::audio::devices;
use super::error_string;
use crate::dsl::ElgNew;
use crate::error::{ffi, Error};
use crate::ffi as cffi;
use crate::geometry::{ContentMode, Size};
use crate::view::{IntoView, View};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CameraDevice {
    pub id: String,
    pub name: String,
}

pub fn camera_devices() -> Vec<CameraDevice> {
    devices(sys::elg_camera_devices)
        .into_iter()
        .map(|d| CameraDevice {
            id: d.id,
            name: d.name,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraOptions {
    /// Empty: the first camera.
    pub device_id: String,
    pub width: u32,
    pub height: u32,
    /// The preview shows a mirror image, as people expect of themselves.
    pub mirrored: bool,
}

impl Default for CameraOptions {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            width: 1280,
            height: 720,
            mirrored: true,
        }
    }
}

/// A camera feed for `CameraView`, and snapshots. Make it on the UI thread.
pub struct Camera {
    ptr: NonNull<sys::Camera>,
}
handle!(Camera, Camera, elg_camera_clone, elg_camera_free);

impl Camera {
    pub fn new(options: CameraOptions) -> Result<Camera, Error> {
        Self::from_raw(unsafe {
            sys::elg_camera_new(
                cffi::s(&options.device_id),
                options.width,
                options.height,
                options.mirrored,
            )
        })
    }
    pub fn start(&self) {
        ffi!(sys::elg_camera_start(self.as_ptr(), true));
    }
    pub fn stop(&self) {
        ffi!(sys::elg_camera_start(self.as_ptr(), false));
    }
    pub fn is_running(&self) -> bool {
        ffi!(sys::elg_camera_is_running(self.as_ptr()))
    }
    /// Pixels of the frames; zero until the first one.
    pub fn size(&self) -> Size {
        Size::from_c(ffi!(sys::elg_camera_size(self.as_ptr())))
    }
    pub fn error(&self) -> Option<String> {
        error_string(|sink, ctx| unsafe { sys::elg_camera_error(self.as_ptr(), sink, ctx) })
    }
    pub fn mirrored(&self) -> bool {
        unsafe { sys::elg_camera_mirrored(self.as_ptr()) }
    }
    /// The latest frame as PNG bytes, if there's one.
    pub fn snapshot(&self) -> Option<Vec<u8>> {
        let mut result = Vec::new();
        let mut each = |args: *mut std::ffi::c_void, _: *mut std::ffi::c_void| unsafe {
            result.extend_from_slice((*(args as *const sys::ElgBytes)).as_slice());
        };
        ffi!(sys::elg_camera_snapshot(
            self.as_ptr(),
            cffi::borrowed(&mut each)
        ))
        .then_some(result)
    }
}

/// Shows a camera's feed.
pub struct CameraView {
    camera: Camera,
    mode: ContentMode,
}

#[elegance_macros::elg_builder]
impl CameraView {
    pub fn new(camera: &Camera) -> Self {
        Self {
            camera: camera.clone(),
            mode: ContentMode::Fill,
        }
    }
    pub fn mode(mut self, mode: impl Into<ContentMode>) -> Self {
        self.mode = mode.into();
        self
    }
}

impl ElgNew<(&Camera,)> for CameraView {
    type Out = CameraView;
    fn elg_new((camera,): (&Camera,)) -> Self {
        CameraView::new(camera)
    }
}

impl ElgNew<(Camera,)> for CameraView {
    type Out = CameraView;
    fn elg_new((camera,): (Camera,)) -> Self {
        CameraView {
            camera,
            mode: ContentMode::Fill,
        }
    }
}

impl IntoView for CameraView {
    fn into_view(self) -> View {
        View::from_raw(ffi!(sys::elg_camera_view(
            self.camera.as_ptr(),
            self.mode as i32
        )))
    }
}
