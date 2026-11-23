//! What the Vulkan transport takes from Mikros: memory objects and kernel timelines, shared with
//! the consumer through VK_MIKROS_external_memory_handle and VK_MIKROS_external_semaphore_handle.
//! The Vulkan side speaks in handles (`mikros_handle_t`); the C API of Elegance takes C library
//! descriptors, which `mikros_fd_from_handle` and `mikros_handle_from_fd` make and read.
//! The structure types and bits below are those of the Mikros sysroot's vulkan_core.h and
//! vulkan_mikros.h.
use super::device_error;
use crate::Error;
use ash::vk;
use elegance_sys::gpu_image::{self as raw, Fence};
use std::{
    ffi::{c_int, c_void, CStr},
    mem::MaybeUninit,
    os::fd::{FromRawFd, OwnedFd},
};

type Handle = u32;

unsafe extern "C" {
    fn mikros_fd_from_handle(handle: Handle, kind: u32) -> c_int;
    fn mikros_handle_from_fd(fd: c_int, handle: *mut Handle) -> c_int;
    fn mikros_handle_close(handle: Handle) -> i64;
}

/// The descriptor kinds of `mikros_fd_from_handle`: a memory object, and a plain handle.
const KIND_MEMORY: u32 = 4;
const KIND_HANDLE: u32 = 10;

pub(super) const NAMES: [&CStr; 2] = [
    c"VK_MIKROS_external_memory_handle",
    c"VK_MIKROS_external_semaphore_handle",
];
/// VK_EXTERNAL_MEMORY_HANDLE_TYPE_MEMORY_OBJECT_BIT_MIKROS
pub(super) const MEMORY: vk::ExternalMemoryHandleTypeFlags =
    vk::ExternalMemoryHandleTypeFlags::from_raw(0x2000_0000);
/// Dawn imports the image's memory as OPAQUE_FD and refuses an image whose creation info doesn't list
/// that type, so the image lists it next to the memory object's.
pub(super) const IMAGE_HANDLES: vk::ExternalMemoryHandleTypeFlags =
    vk::ExternalMemoryHandleTypeFlags::from_raw(
        MEMORY.as_raw() | vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD.as_raw(),
    );
/// Nothing here exports what was imported again.
pub(super) const REEXPORT_REQUIRED: bool = false;
/// VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_TIMELINE_BIT_MIKROS
pub(super) const SEMAPHORE: vk::ExternalSemaphoreHandleTypeFlags =
    vk::ExternalSemaphoreHandleTypeFlags::from_raw(0x1000_0000);
pub(super) const TRANSPORT: u32 = raw::TRANSPORT_VULKAN_OPAQUE_FD;
pub(super) const FENCE: u32 = raw::FENCE_VK_OPAQUE_FD;
/// What the producer's writes signal on the timeline: any value of at least 1.
pub(super) const PRODUCER_VALUE: u64 = 1;

const STRUCTURE_MEMORY_GET_HANDLE_INFO: i32 = 1_009_000_002;
const STRUCTURE_IMPORT_SEMAPHORE_HANDLE_INFO: i32 = 1_009_001_000;
const STRUCTURE_SEMAPHORE_GET_HANDLE_INFO: i32 = 1_009_001_001;

#[repr(C)]
struct MemoryGetHandleInfo {
    s_type: vk::StructureType,
    p_next: *const c_void,
    memory: vk::DeviceMemory,
    handle_type: vk::ExternalMemoryHandleTypeFlags,
}

#[repr(C)]
struct SemaphoreGetHandleInfo {
    s_type: vk::StructureType,
    p_next: *const c_void,
    semaphore: vk::Semaphore,
    handle_type: vk::ExternalSemaphoreHandleTypeFlags,
}

#[repr(C)]
struct ImportSemaphoreHandleInfo {
    s_type: vk::StructureType,
    p_next: *const c_void,
    semaphore: vk::Semaphore,
    flags: vk::SemaphoreImportFlags,
    handle_type: vk::ExternalSemaphoreHandleTypeFlags,
    handle: Handle,
}

type GetMemoryHandle =
    unsafe extern "system" fn(vk::Device, *const MemoryGetHandleInfo, *mut Handle) -> vk::Result;
type GetSemaphoreHandle =
    unsafe extern "system" fn(vk::Device, *const SemaphoreGetHandleInfo, *mut Handle) -> vk::Result;
type ImportSemaphoreHandle =
    unsafe extern "system" fn(vk::Device, *const ImportSemaphoreHandleInfo) -> vk::Result;

pub(super) struct Handles {
    device: vk::Device,
    get_memory: GetMemoryHandle,
    get_semaphore: GetSemaphoreHandle,
    import_semaphore: ImportSemaphoreHandle,
}

impl Handles {
    /// Loads the commands of the two extensions, which the device must have enabled.
    pub(super) fn new(instance: &ash::Instance, device: &ash::Device) -> Result<Self, Error> {
        fn command(
            instance: &ash::Instance,
            device: &ash::Device,
            name: &CStr,
        ) -> Result<unsafe extern "system" fn(), Error> {
            unsafe { instance.get_device_proc_addr(device.handle(), name.as_ptr()) }
                .ok_or_else(|| device_error(format!("{} is missing", name.to_string_lossy())))
        }
        // The commands are known by name, and each type is the one its name has in vulkan_mikros.h.
        unsafe {
            Ok(Self {
                device: device.handle(),
                get_memory: std::mem::transmute::<_, GetMemoryHandle>(command(
                    instance,
                    device,
                    c"vkGetMemoryHandleMIKROS",
                )?),
                get_semaphore: std::mem::transmute::<_, GetSemaphoreHandle>(command(
                    instance,
                    device,
                    c"vkGetSemaphoreHandleMIKROS",
                )?),
                import_semaphore: std::mem::transmute::<_, ImportSemaphoreHandle>(command(
                    instance,
                    device,
                    c"vkImportSemaphoreHandleMIKROS",
                )?),
            })
        }
    }

    /// A descriptor for the memory object of `memory`, which the caller owns.
    pub(super) unsafe fn export_memory(&self, memory: vk::DeviceMemory) -> Result<OwnedFd, Error> {
        let info = MemoryGetHandleInfo {
            s_type: vk::StructureType::from_raw(STRUCTURE_MEMORY_GET_HANDLE_INFO),
            p_next: std::ptr::null(),
            memory,
            handle_type: MEMORY,
        };
        let mut handle = MaybeUninit::uninit();
        unsafe { (self.get_memory)(self.device, &info, handle.as_mut_ptr()) }
            .result()
            .map_err(device_error)?;
        descriptor(unsafe { handle.assume_init() }, KIND_MEMORY)
    }

    /// A descriptor for the kernel timeline of `semaphore`, which the caller owns.
    pub(super) unsafe fn export_semaphore(
        &self,
        semaphore: vk::Semaphore,
    ) -> Result<OwnedFd, Error> {
        let info = SemaphoreGetHandleInfo {
            s_type: vk::StructureType::from_raw(STRUCTURE_SEMAPHORE_GET_HANDLE_INFO),
            p_next: std::ptr::null(),
            semaphore,
            handle_type: SEMAPHORE,
        };
        let mut handle = MaybeUninit::uninit();
        unsafe { (self.get_semaphore)(self.device, &info, handle.as_mut_ptr()) }
            .result()
            .map_err(device_error)?;
        descriptor(unsafe { handle.assume_init() }, KIND_HANDLE)
    }

    /// Makes the timeline of `consumer` the payload of `semaphore`, for good. The handle is
    /// consumed only when this succeeds.
    pub(super) unsafe fn import_semaphore(
        &self,
        semaphore: vk::Semaphore,
        consumer: Consumer,
    ) -> Result<(), Error> {
        let info = ImportSemaphoreHandleInfo {
            s_type: vk::StructureType::from_raw(STRUCTURE_IMPORT_SEMAPHORE_HANDLE_INFO),
            p_next: std::ptr::null(),
            semaphore,
            flags: vk::SemaphoreImportFlags::empty(),
            handle_type: SEMAPHORE,
            handle: consumer.handle,
        };
        unsafe { (self.import_semaphore)(self.device, &info) }
            .result()
            .map_err(device_error)?;
        std::mem::forget(consumer);
        Ok(())
    }
}

/// Wraps a handle that the caller owns in a descriptor of `kind`, which then owns it; a handle
/// that can't be wrapped is closed.
fn descriptor(handle: Handle, kind: u32) -> Result<OwnedFd, Error> {
    let fd = unsafe { mikros_fd_from_handle(handle, kind) };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        unsafe { mikros_handle_close(handle) };
        return Err(device_error(error));
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn timeline_info(value: u64) -> vk::SemaphoreTypeCreateInfo<'static> {
    vk::SemaphoreTypeCreateInfo::default()
        .semaphore_type(vk::SemaphoreType::TIMELINE)
        .initial_value(value)
}

/// The semaphore that carries the producer's completion to the consumer, which the producer
/// exports: a timeline of value 0 that its writes signal to `PRODUCER_VALUE`.
pub(super) fn create_producer_semaphore(device: &ash::Device) -> Result<vk::Semaphore, Error> {
    let mut timeline = timeline_info(0);
    let mut export = vk::ExportSemaphoreCreateInfo::default().handle_types(SEMAPHORE);
    unsafe {
        device.create_semaphore(
            &vk::SemaphoreCreateInfo::default()
                .push_next(&mut timeline)
                .push_next(&mut export),
            None,
        )
    }
    .map_err(device_error)
}

/// A semaphore for a consumer's timeline, which `import_semaphore` fills.
pub(super) fn create_consumer_semaphore(device: &ash::Device) -> Result<vk::Semaphore, Error> {
    let mut timeline = timeline_info(0);
    unsafe {
        device.create_semaphore(
            &vk::SemaphoreCreateInfo::default().push_next(&mut timeline),
            None,
        )
    }
    .map_err(device_error)
}

/// A consumer's timeline from a release, as a handle of the producer's own, and the value to wait
/// for. It is closed unless an import consumes it.
pub(super) struct Consumer {
    handle: Handle,
    pub(super) value: u64,
}

impl Drop for Consumer {
    fn drop(&mut self) {
        unsafe { mikros_handle_close(self.handle) };
    }
}

/// Never consume a callback's borrowed descriptor: take a duplicate of its handle, and do no Vulkan
/// work or GPU waits here. Continue with subsequent fences even after a failure.
pub(super) fn duplicate_fence(fence: &Fence) -> Result<Consumer, Error> {
    if fence.type_ != FENCE || fence.fd < 0 || fence.value < 1 {
        return Err(device_error(
            "invalid consumer Vulkan timeline semaphore fence",
        ));
    }
    let mut handle = 0;
    if unsafe { mikros_handle_from_fd(fence.fd, &mut handle) } < 0 {
        return Err(device_error(std::io::Error::last_os_error()));
    }
    Ok(Consumer {
        handle,
        value: fence.value,
    })
}

/// Memory is allocated from CPU-visible memory types only.
pub(super) fn memory_type_allowed(flags: vk::MemoryPropertyFlags) -> bool {
    flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE)
}

/// Both views of the image: the consumer samples it through the sRGB one.
pub(super) fn view_formats(format: vk::Format) -> [vk::Format; 2] {
    if format == vk::Format::B8G8R8A8_UNORM {
        [vk::Format::B8G8R8A8_UNORM, vk::Format::B8G8R8A8_SRGB]
    } else {
        [vk::Format::R8G8B8A8_UNORM, vk::Format::R8G8B8A8_SRGB]
    }
}

pub(super) fn check_semaphore(
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
) -> Result<(), Error> {
    let mut timeline =
        vk::SemaphoreTypeCreateInfo::default().semaphore_type(vk::SemaphoreType::TIMELINE);
    let info = vk::PhysicalDeviceExternalSemaphoreInfo::default()
        .handle_type(SEMAPHORE)
        .push_next(&mut timeline);
    let mut semaphore = vk::ExternalSemaphoreProperties::default();
    unsafe {
        instance.get_physical_device_external_semaphore_properties(physical, &info, &mut semaphore)
    };
    if !semaphore.external_semaphore_features.contains(
        vk::ExternalSemaphoreFeatureFlags::IMPORTABLE
            | vk::ExternalSemaphoreFeatureFlags::EXPORTABLE,
    ) || !semaphore.compatible_handle_types.contains(SEMAPHORE)
    {
        return Err(device_error(
            "timeline semaphore import/export through Mikros handles is unsupported",
        ));
    }
    Ok(())
}
