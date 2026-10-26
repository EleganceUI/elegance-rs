//! What every backend's `Bridge` shares: the frame budget, configuration checks, wgpu error
//! scopes, and the platform-neutral half of the public API. Each backend keeps its own
//! allocation, publication and retirement, whose safety depends on its native API.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use elegance_sys::gpu_image as raw;

use crate::{Bridge, Config, Error, GpuImage};

/// Frames holding a budget slot (producing, displayed, retiring or quarantined), and how many
/// of those are quarantined.
#[derive(Default)]
pub(crate) struct Counts {
    pub(crate) active: AtomicUsize,
    pub(crate) quarantined: AtomicUsize,
}

/// One frame's budget slot, given back when dropped. A quarantined frame never drops its
/// permit, so it keeps its slot for the life of the process.
pub(crate) struct Permit(pub(crate) Arc<Counts>);

impl Permit {
    /// Takes a slot, or returns `Error::Busy` when all `limit` are in use.
    pub(crate) fn new(counts: &Arc<Counts>, limit: usize) -> Result<Self, Error> {
        let mut active = counts.active.load(Ordering::Acquire);
        loop {
            if active >= limit {
                return Err(Error::Busy);
            }
            match counts.active.compare_exchange_weak(
                active,
                active + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(Self(counts.clone())),
                Err(actual) => active = actual,
            }
        }
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::AcqRel);
    }
}

/// wgpu error scopes around a production step. The native wgpu-core backends resolve scope
/// futures synchronously; an unexpected Pending is an error, never permission to publish or to
/// block the UI.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) struct NativeErrors(Vec<wgpu::ErrorScopeGuard>);

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl NativeErrors {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        Self(vec![
            device.push_error_scope(wgpu::ErrorFilter::OutOfMemory),
            device.push_error_scope(wgpu::ErrorFilter::Internal),
            device.push_error_scope(wgpu::ErrorFilter::Validation),
        ])
    }

    /// Pops the scopes, returning the first error as `Error::Device`.
    pub(crate) fn finish(mut self) -> Result<(), Error> {
        use std::{
            future::Future,
            task::{Context, Poll, Waker},
        };
        let mut error = None;
        while let Some(scope) = self.0.pop() {
            let future = scope.pop();
            let mut future = std::pin::pin!(future);
            let result = match future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            {
                Poll::Ready(None) => None,
                Poll::Ready(Some(error)) => Some(error.to_string()),
                Poll::Pending => Some("native wgpu error scope unexpectedly pending".into()),
            };
            if error.is_none() {
                error = result;
            }
        }
        error.map_or(Ok(()), |error| Err(Error::Device(error)))
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl Drop for NativeErrors {
    fn drop(&mut self) {
        while let Some(scope) = self.0.pop() {
            drop(scope);
        }
    }
}

impl Config {
    /// A supported format, and nonzero dimensions and budget.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.width == 0
            || self.height == 0
            || self.max_frames == 0
            || !matches!(
                self.format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
            )
        {
            return Err(Error::InvalidConfig);
        }
        Ok(())
    }

    /// Dimensions within the device's 2D texture limit.
    pub(crate) fn check_limits(&self, device: &wgpu::Device) -> Result<(), Error> {
        let limit = device.limits().max_texture_dimension_2d;
        if self.width > limit || self.height > limit {
            return Err(Error::InvalidConfig);
        }
        Ok(())
    }

    /// The format as a `Frame` gives it.
    pub(crate) fn raw_format(&self) -> u32 {
        if self.format == wgpu::TextureFormat::Bgra8Unorm {
            raw::FORMAT_BGRA8
        } else {
            raw::FORMAT_RGBA8
        }
    }

    pub(crate) fn extent(&self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        }
    }
}

impl Bridge {
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    pub fn image(&self) -> &GpuImage {
        &self.image
    }
    /// Set the physical pixel dimensions for subsequent frames.
    ///
    /// Returns `Error::InvalidConfig` for zero dimensions or dimensions exceeding
    /// the device's 2D texture limit, leaving the previous size unchanged.
    /// The image handle and displayed frame are preserved until a new publication.
    /// Outstanding frames retire normally under the same budget, including
    /// quarantined allocations. This does not allocate, wait, or recreate the device.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), Error> {
        self.config
            .resize(width, height, self.device.limits().max_texture_dimension_2d)
    }
    pub fn clear(&self) {
        self.image.clear();
    }
    pub fn in_flight(&self) -> usize {
        self.counts.active.load(Ordering::Acquire)
    }
    pub fn quarantined(&self) -> usize {
        self.counts.quarantined.load(Ordering::Acquire)
    }

    /// Copy mip zero of a same-device, same-size/format, single-sampled 2D COPY_SRC
    /// texture into a fresh shared image, submit, then publish: a GPU copy, never a CPU
    /// one. wgpu validation errors return `Error::Device` without publishing.
    pub fn copy_from(&mut self, source: &wgpu::Texture) -> Result<(), Error> {
        if source.size() != self.extent()
            || source.dimension() != wgpu::TextureDimension::D2
            || source.format() != self.config.format
            || source.sample_count() != 1
            || !source.usage().contains(wgpu::TextureUsages::COPY_SRC)
        {
            return Err(Error::InvalidConfig);
        }
        let size = self.extent();
        // No destination resource is exposed to the caller.
        unsafe {
            self.render_raw(|_, encoder, target| {
                encoder.copy_texture_to_texture(
                    source.as_image_copy(),
                    target.as_image_copy(),
                    size,
                );
            })
        }
    }

    /// Publish a GPU-cleared image. RGB values are stored as supplied in the UNORM
    /// texture (sRGB-encoded); alpha is straight, not premultiplied.
    pub fn clear_color(&mut self, color: wgpu::Color) -> Result<(), Error> {
        unsafe {
            self.render_raw(|_, encoder, target| {
                let view = target.create_view(&Default::default());
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(color),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
            })
        }
    }

    pub(crate) fn extent(&self) -> wgpu::Extent3d {
        self.config.extent()
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        // Not a drain. Callback-owned senders keep the retirement worker alive
        // even when native views outlive this producer and its C image handle.
        self.image.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_is_held_until_permit_drops() {
        let counts = Arc::new(Counts::default());
        let one = Permit::new(&counts, 2).unwrap();
        let two = Permit::new(&counts, 2).unwrap();
        assert!(matches!(Permit::new(&counts, 2), Err(Error::Busy)));
        assert_eq!(counts.active.load(Ordering::Acquire), 2);
        drop(one);
        assert!(Permit::new(&counts, 2).is_ok());
        drop(two);
        assert_eq!(counts.active.load(Ordering::Acquire), 0);
    }

    #[test]
    fn concurrent_reservations_never_exceed_budget() {
        let counts = Arc::new(Counts::default());
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let counts = counts.clone();
                std::thread::spawn(move || {
                    for _ in 0..1000 {
                        if let Ok(permit) = Permit::new(&counts, 3) {
                            assert!(counts.active.load(Ordering::Acquire) <= 3);
                            std::thread::yield_now();
                            drop(permit);
                        }
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(counts.active.load(Ordering::Acquire), 0);
    }

    #[test]
    fn validate_rejects_zero_sizes_budgets_and_other_formats() {
        let valid = Config {
            width: 16,
            height: 16,
            origin: crate::GpuImageOrigin::TopLeft,
            format: wgpu::TextureFormat::Rgba8Unorm,
            max_frames: 2,
        };
        assert!(valid.validate().is_ok());
        let bgra = Config {
            format: wgpu::TextureFormat::Bgra8Unorm,
            ..valid
        };
        assert!(bgra.validate().is_ok());
        for invalid in [
            Config { width: 0, ..valid },
            Config { height: 0, ..valid },
            Config {
                max_frames: 0,
                ..valid
            },
            Config {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                ..valid
            },
        ] {
            assert!(matches!(invalid.validate(), Err(Error::InvalidConfig)));
        }
    }
}
