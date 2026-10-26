//! Audio playback, audio and camera capture, and video.
//!
//! Players, recorders, and streams are cheap handles: clones refer to the same one. Their
//! reactive reads (`is_playing`, `time`, ...) are for the UI thread, like any state.

use elegance_sys as sys;

use crate::ffi as cffi;

macro_rules! handle {
    ($name:ident, $raw:ident, $clone:ident, $free:ident) => {
        impl Clone for $name {
            fn clone(&self) -> Self {
                Self {
                    ptr: std::ptr::NonNull::new(unsafe { sys::$clone(self.ptr.as_ptr()) }).unwrap(),
                }
            }
        }
        impl Drop for $name {
            fn drop(&mut self) {
                unsafe { sys::$free(self.ptr.as_ptr()) }
            }
        }
        impl $name {
            fn from_raw(ptr: *mut sys::$raw) -> Result<Self, crate::Error> {
                crate::error::result()?;
                Ok(Self {
                    ptr: std::ptr::NonNull::new(ptr)
                        .ok_or_else(|| crate::Error::new(crate::ErrorKind::Other, "no handle"))?,
                })
            }
            fn as_ptr(&self) -> *const sys::$raw {
                self.ptr.as_ptr()
            }
        }
    };
}

mod audio;
mod camera;
mod video;

pub use audio::*;
pub use camera::*;
pub use video::*;

fn error_string(f: impl FnOnce(sys::ElgStrSink, *mut std::ffi::c_void)) -> Option<String> {
    let message = cffi::read_string(f);
    crate::error::check();
    (!message.is_empty()).then_some(message)
}
