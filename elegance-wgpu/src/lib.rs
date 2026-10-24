//! Bounded external GPU images for Elegance. See the crate README for ownership
//! and shutdown requirements. Native transports are Metal/IOSurface on macOS,
//! DX12/DXGI on Windows, and Vulkan/opaque FD on Linux. Dawn must use a compatible
//! backend on the same physical GPU; unsupported combinations fail explicitly.
//!
//! `Bridge::new` creates a dedicated producer device/queue. Unsafe
//! `Bridge::from_device` attaches an existing pair with externally serialized
//! queue access; Vulkan additionally requires enabled sharing extensions and an
//! explicitly created/reserved retirement queue. See the README and
//! `src/bin/wgpu-image.rs` for a runnable platform-neutral Rust UI smoke example.

pub use elegance::{GpuImage, GpuImageOrigin, GpuImageView};
use std::fmt;
pub use wgpu;

mod render;
pub use render::{RenderAttachments, RenderFrame, RenderPassOptions};

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub origin: GpuImageOrigin,
    /// BGRA8Unorm or RGBA8Unorm. RGB bytes must contain sRGB-encoded values.
    pub format: wgpu::TextureFormat,
    /// Includes displayed, producer-in-flight, retiring, and quarantined frames.
    pub max_frames: usize,
}

impl Config {
    fn resize(&mut self, width: u32, height: u32, max_dimension: u32) -> Result<(), Error> {
        if width == 0 || height == 0 || width > max_dimension || height > max_dimension {
            return Err(Error::InvalidConfig);
        }
        self.width = width;
        self.height = height;
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    Unsupported,
    InvalidConfig,
    Busy,
    Device(String),
    Allocation,
    Publish(u32),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

#[cfg(target_os = "macos")]
mod metal;
#[cfg(target_os = "macos")]
pub use metal::Bridge;

#[cfg(target_os = "windows")]
mod dx12;
#[cfg(target_os = "windows")]
pub use dx12::Bridge;

#[cfg(target_os = "linux")]
mod vulkan;
#[cfg(target_os = "linux")]
pub use vulkan::Bridge;

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            width: 16,
            height: 32,
            origin: GpuImageOrigin::BottomLeft,
            format: wgpu::TextureFormat::Rgba8Unorm,
            max_frames: 3,
        }
    }

    #[test]
    fn resize_changes_only_dimensions_and_accepts_limit_and_same_size() {
        let mut config = config();
        for (width, height) in [(1, 64), (64, 1), (64, 1), (8, 16)] {
            config.resize(width, height, 64).unwrap();
            assert_eq!((config.width, config.height), (width, height));
            assert!(matches!(config.origin, GpuImageOrigin::BottomLeft));
            assert_eq!(config.format, wgpu::TextureFormat::Rgba8Unorm);
            assert_eq!(config.max_frames, 3);
        }
    }

    #[test]
    fn invalid_resize_leaves_config_unchanged() {
        let mut config = config();
        for (width, height) in [
            (0, 32),
            (16, 0),
            (0, 0),
            (65, 32),
            (16, 65),
            (u32::MAX, u32::MAX),
        ] {
            assert!(matches!(
                config.resize(width, height, 64),
                Err(Error::InvalidConfig)
            ));
            assert_eq!((config.width, config.height), (16, 32));
            assert!(matches!(config.origin, GpuImageOrigin::BottomLeft));
            assert_eq!(config.format, wgpu::TextureFormat::Rgba8Unorm);
            assert_eq!(config.max_frames, 3);
        }
    }
}

/// Unsupported-platform placeholder with the same construction and submission API.
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub struct Bridge;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
impl Bridge {
    pub async fn new(_: &wgpu::Adapter, _: Config) -> Result<Self, Error> {
        Err(Error::Unsupported)
    }
    /// # Safety
    /// The matching device/queue pair must satisfy the supported backend's full
    /// external synchronization and queue reservation contract.
    pub unsafe fn from_device(_: wgpu::Device, _: wgpu::Queue, _: Config) -> Result<Self, Error> {
        Err(Error::Unsupported)
    }
    pub fn device(&self) -> &wgpu::Device {
        unreachable!()
    }
    pub fn image(&self) -> &GpuImage {
        unreachable!()
    }
    pub fn resize(&mut self, _: u32, _: u32) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
    pub fn clear(&self) {}
    pub fn in_flight(&self) -> usize {
        0
    }
    pub fn quarantined(&self) -> usize {
        0
    }
    pub fn copy_from(&mut self, _: &wgpu::Texture) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
    pub fn clear_color(&mut self, _: wgpu::Color) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
    /// # Safety
    /// No destination handles or unsubmitted commands may escape the closure.
    pub unsafe fn render_raw(
        &mut self,
        _: impl FnOnce(&wgpu::Device, &mut wgpu::CommandEncoder, &wgpu::Texture),
    ) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
}
