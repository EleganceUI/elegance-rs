//! Native external GPU image sharing (`elg/gpu_image.h`), by the names the `elegance` crate gives
//! its declarations, which bindings.rs has.
//!
//! Tags are fixed-width integers, not Rust enums. Handles and FDs are borrowed.
//! Publication and retirement require the caller to uphold the native GPU synchronization
//! and lifetime contract; release does not itself mean GPU completion.

pub use crate::{elg_gpu_image_clear, elg_gpu_image_create, elg_gpu_image_destroy, elg_gpu_image_publish};

pub type GpuImage = crate::ElgGpuImage;
pub type Fence = crate::ElgGpuImageFence;
pub type Release = crate::ElgGpuImageRelease;
pub type Frame = crate::ElgGpuImageFrame;
pub type ReleaseCallback = crate::ElgGpuImageReleaseCallback;

pub const STATUS_OK: u32 = crate::ELG_GPU_IMAGE_OK;
pub const STATUS_INVALID_ARGUMENT: u32 = crate::ELG_GPU_IMAGE_INVALID_ARGUMENT;
pub const STATUS_DEVICE_UNAVAILABLE: u32 = crate::ELG_GPU_IMAGE_DEVICE_UNAVAILABLE;
pub const STATUS_UNSUPPORTED: u32 = crate::ELG_GPU_IMAGE_UNSUPPORTED;
pub const STATUS_IMPORT_FAILED: u32 = crate::ELG_GPU_IMAGE_IMPORT_FAILED;
pub const STATUS_SYNC_FAILED: u32 = crate::ELG_GPU_IMAGE_SYNC_FAILED;
pub const STATUS_INTERNAL_ERROR: u32 = crate::ELG_GPU_IMAGE_INTERNAL_ERROR;
pub const TRANSPORT_IOSURFACE: u32 = crate::ELG_GPU_IMAGE_IOSURFACE;
pub const TRANSPORT_DXGI_HANDLE: u32 = crate::ELG_GPU_IMAGE_DXGI_HANDLE;
pub const TRANSPORT_VULKAN_OPAQUE_FD: u32 = crate::ELG_GPU_IMAGE_VULKAN_OPAQUE_FD;
pub const FORMAT_RGBA8: u32 = crate::ELG_GPU_IMAGE_RGBA8;
pub const FORMAT_BGRA8: u32 = crate::ELG_GPU_IMAGE_BGRA8;
pub const ORIGIN_TOP_LEFT: u32 = crate::ELG_GPU_IMAGE_TOP_LEFT;
pub const ORIGIN_BOTTOM_LEFT: u32 = crate::ELG_GPU_IMAGE_BOTTOM_LEFT;
pub const FENCE_NONE: u32 = crate::ELG_GPU_IMAGE_FENCE_NONE;
pub const FENCE_MTL_SHARED_EVENT: u32 = crate::ELG_GPU_IMAGE_FENCE_MTL_SHARED_EVENT;
pub const FENCE_DXGI_HANDLE: u32 = crate::ELG_GPU_IMAGE_FENCE_DXGI_HANDLE;
pub const FENCE_VK_OPAQUE_FD: u32 = crate::ELG_GPU_IMAGE_FENCE_VK_OPAQUE_FD;
