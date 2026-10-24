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

pub use elegance::{ExternalGpuImage, ExternalGpuImageView, GpuImageOrigin};
use std::fmt;
pub use wgpu;

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
    pub fn image(&self) -> &ExternalGpuImage {
        unreachable!()
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
    pub unsafe fn render(
        &mut self,
        _: impl FnOnce(&wgpu::Device, &mut wgpu::CommandEncoder, &wgpu::Texture),
    ) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
}
