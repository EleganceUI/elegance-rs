//! Native external GPU image sharing (`elg/gpu_image.h`).
//!
//! Tags are fixed-width integers, not Rust enums. Handles and FDs are borrowed.
//! Publication and retirement require the caller to uphold the native GPU synchronization
//! and lifetime contract; release does not itself mean GPU completion.

use std::ffi::{c_int, c_void};
use std::marker::{PhantomData, PhantomPinned};

opaque!(
    /// Opaque native `ElgGpuImage` handle.
    GpuImage,
);

pub const STATUS_OK: u32 = 0;
pub const STATUS_INVALID_ARGUMENT: u32 = 1;
pub const STATUS_DEVICE_UNAVAILABLE: u32 = 2;
pub const STATUS_UNSUPPORTED: u32 = 3;
pub const STATUS_IMPORT_FAILED: u32 = 4;
pub const STATUS_SYNC_FAILED: u32 = 5;
pub const STATUS_INTERNAL_ERROR: u32 = 6;

pub const TRANSPORT_IOSURFACE: u32 = 1;
pub const TRANSPORT_DXGI_HANDLE: u32 = 2;
pub const TRANSPORT_VULKAN_OPAQUE_FD: u32 = 3;
pub const FORMAT_RGBA8: u32 = 1;
pub const FORMAT_BGRA8: u32 = 2;
pub const ORIGIN_TOP_LEFT: u32 = 0;
pub const ORIGIN_BOTTOM_LEFT: u32 = 1;

pub const FENCE_NONE: u32 = 0;
pub const FENCE_MTL_SHARED_EVENT: u32 = 1;
pub const FENCE_DXGI_HANDLE: u32 = 2;
pub const FENCE_VK_OPAQUE_FD: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fence {
    pub r#type: u32,
    pub handle: *mut c_void,
    pub fd: c_int,
    pub value: u64,
}

/// Borrowed only during the release callback, including every fence handle/FD.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Release {
    pub status: u32,
    pub token: u64,
    pub fences: *const Fence,
    pub fence_count: usize,
    pub vk_old_layout: i32,
    pub vk_new_layout: i32,
}

/// May run on any thread. Must not unwind, block on renderer work, or reenter this API.
pub type ReleaseCallback = unsafe extern "C" fn(userdata: *mut c_void, release: *const Release);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub transport: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub handle: *mut c_void,
    pub memory_fd: c_int,
    pub vk_image_create_info: *const c_void,
    pub vk_memory_type_index: u32,
    pub vk_allocation_size: u64,
    pub vk_dedicated_allocation: u32,
    pub vk_old_layout: i32,
    pub vk_new_layout: i32,
    pub producer_fence: Fence,
    pub token: u64,
    pub userdata: *mut c_void,
    pub release: Option<ReleaseCallback>,
}

unsafe extern "C" {
    pub fn elg_gpu_image_create(width: u32, height: u32, origin: u32) -> *mut GpuImage;
    /// Must not race any calls on the handle. Not a drain: views may still retain frames.
    pub fn elg_gpu_image_destroy(image: *mut GpuImage);
    /// Synchronous import; requires a live renderer. Failure retains caller ownership and
    /// invokes no callback. Success invokes exactly one callback, possibly before returning.
    ///
    /// # Safety
    /// Keep the image alive and publish only fresh, exclusive allocations with producer
    /// writes stopped and a producer fence (or NONE only after GPU writes have completed).
    /// Keep userdata, handles/FDs, and allocations alive until callback. Retain/duplicate
    /// callback fences there and wait on ALL of them before reuse or freeing. On non-OK
    /// release, quarantine until GPU completion is independently established. Vulkan callers
    /// must also honor external queue-family ownership and the returned layout transitions.
    /// See the native header for the complete transport-specific contract.
    pub fn elg_gpu_image_publish(image: *mut GpuImage, frame: *const Frame) -> u32;
    pub fn elg_gpu_image_clear(image: *mut GpuImage);
    /// C++ view shim; the returned owned view independently retains its source.
    pub fn elg_gpu_image_view(image: *const GpuImage, mode: i32) -> *mut crate::View;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, offset_of, size_of};

    #[test]
    fn native_c_layout() {
        macro_rules! layout {
            ($ty:ty, $($field:ident: $field_ty:ty),+ $(,)?) => {{
                let mut end = 0usize;
                $(
                    let offset = end.next_multiple_of(align_of::<$field_ty>());
                    assert_eq!(offset_of!($ty, $field), offset);
                    end = offset + size_of::<$field_ty>();
                )+
                assert_eq!(size_of::<$ty>(), end.next_multiple_of(align_of::<$ty>()));
            }};
        }
        layout!(Fence, r#type: u32, handle: *mut c_void, fd: c_int, value: u64);
        layout!(Release, status: u32, token: u64, fences: *const Fence,
            fence_count: usize, vk_old_layout: i32, vk_new_layout: i32);
        layout!(Frame, transport: u32, format: u32, width: u32, height: u32,
            handle: *mut c_void, memory_fd: c_int, vk_image_create_info: *const c_void,
            vk_memory_type_index: u32, vk_allocation_size: u64, vk_dedicated_allocation: u32,
            vk_old_layout: i32, vk_new_layout: i32, producer_fence: Fence,
            token: u64, userdata: *mut c_void, release: Option<ReleaseCallback>);
        assert_eq!(size_of::<Option<ReleaseCallback>>(), size_of::<*const c_void>());
    }
}
