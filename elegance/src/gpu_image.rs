//! Images other GPU code renders (see `elegance-wgpu`), shared with Elegance without CPU copies.

use std::ptr::NonNull;
use std::sync::Arc;

use elegance_sys as sys;

use crate::dsl::ElgNew;
use crate::error::{ffi, Error};
use crate::geometry::ContentMode;
use crate::view::{IntoView, View};

/// Where the producer's first row goes when displaying an [`GpuImage`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GpuImageOrigin {
    #[default]
    TopLeft,
    BottomLeft,
}

/// An externally produced GPU image with variable frame dimensions and a fixed origin.
///
/// Each accepted frame may have different nonzero dimensions. The shared source keeps its
/// identity and updates its reactive natural size asynchronously on the UI thread.
///
/// Clones share one synchronized native handle. Publication is available only through the
/// unsafe native API in `elegance_sys::gpu_image`; there is no safe generic publish.
/// Views retain their source independently. Stop/join producers and remove views before
/// renderer shutdown, retaining callback state until all accepted frames are released.
#[derive(Clone)]
pub struct GpuImage {
    inner: Arc<GpuImageInner>,
}

struct GpuImageInner {
    ptr: NonNull<sys::gpu_image::GpuImage>,
}

// SAFETY: native publish/clear calls on a live handle are synchronized. Arc keeps the
// handle alive across safe calls and destroys it only after the last owner is dropped.
unsafe impl Send for GpuImageInner {}
unsafe impl Sync for GpuImageInner {}

impl Drop for GpuImageInner {
    fn drop(&mut self) {
        unsafe { sys::gpu_image::elg_gpu_image_destroy(self.ptr.as_ptr()) }
    }
}

impl GpuImage {
    /// Creates an image with an initial natural size of `width` × `height` pixels (both nonzero).
    /// Does not publish or import a frame; accepted frames may have different dimensions.
    pub fn new(width: u32, height: u32, origin: GpuImageOrigin) -> Result<Self, Error> {
        use sys::gpu_image as raw;
        let origin = match origin {
            GpuImageOrigin::TopLeft => raw::ORIGIN_TOP_LEFT,
            GpuImageOrigin::BottomLeft => raw::ORIGIN_BOTTOM_LEFT,
        };
        let ptr = NonNull::new(unsafe { raw::elg_gpu_image_create(width, height, origin) })
            .ok_or_else(|| {
                Error::new(
                    crate::ErrorKind::Other,
                    "could not create external GPU image",
                )
            })?;
        Ok(Self {
            inner: Arc::new(GpuImageInner { ptr }),
        })
    }

    /// Returns a borrowed native handle for unsafe low-level interop.
    ///
    /// Do not destroy it. Keep an owning clone alive for every raw call; the pointer does
    /// not extend its lifetime. Raw use is unsafe and must honor the native publication,
    /// resource lifetime, callback, and GPU synchronization contract. Callbacks must not
    /// reenter this API, including `clear`. Destroying the handle is not a callback drain.
    pub fn as_raw(&self) -> *mut sys::gpu_image::GpuImage {
        self.inner.ptr.as_ptr()
    }

    /// Removes the current frame. Release callbacks do not imply GPU completion;
    /// views/snapshots may retain frames, so this is not a drain.
    pub fn clear(&self) {
        unsafe { sys::gpu_image::elg_gpu_image_clear(self.as_raw()) }
    }
}

/// An [`GpuImage`], fitted into (or filling, clipped to) the offered space, on black.
pub struct GpuImageView {
    image: GpuImage,
    mode: ContentMode,
}

#[elegance_macros::elg_builder]
impl GpuImageView {
    pub fn new(image: &GpuImage) -> Self {
        Self {
            image: image.clone(),
            mode: ContentMode::Fit,
        }
    }
    pub fn mode(mut self, mode: impl Into<ContentMode>) -> Self {
        self.mode = mode.into();
        self
    }
}

impl ElgNew<(&GpuImage,)> for GpuImageView {
    type Out = GpuImageView;
    fn elg_new((image,): (&GpuImage,)) -> Self {
        Self::new(image)
    }
}

impl ElgNew<(GpuImage,)> for GpuImageView {
    type Out = GpuImageView;
    fn elg_new((image,): (GpuImage,)) -> Self {
        Self {
            image,
            mode: ContentMode::Fit,
        }
    }
}

impl IntoView for GpuImageView {
    fn into_view(self) -> View {
        View::from_raw(ffi!(sys::gpu_image::elg_gpu_image_view(
            self.image.as_raw(),
            self.mode as i32
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_is_send_sync_and_clone() {
        fn assert_traits<T: Send + Sync + Clone>() {}
        assert_traits::<GpuImage>();
    }

    #[test]
    fn create_clone_and_clear() {
        let image = GpuImage::new(16, 16, GpuImageOrigin::BottomLeft).unwrap();
        let clone = image.clone();
        assert_eq!(image.as_raw(), clone.as_raw());
        drop(image);
        clone.clear();
    }

    #[test]
    fn rejects_invalid_dimensions() {
        assert!(GpuImage::new(0, 16, GpuImageOrigin::TopLeft).is_err());
        assert!(GpuImage::new(16, 0, GpuImageOrigin::TopLeft).is_err());
    }

    // Compile both owned and borrowed DSL overloads without requiring a live UI renderer.
    #[allow(dead_code)]
    fn dsl_views(image: GpuImage) -> impl IntoView {
        crate::elg! {
            crate::VStack {
                GpuImageView(&image, mode: .fit)
                GpuImageView(image, mode: .fill)
            }
        }
    }
}
