//! What the Vulkan transport takes from Linux: OPAQUE_FD memory and BINARY semaphores, shared
//! with the consumer through VK_KHR_external_memory_fd and VK_KHR_external_semaphore_fd.
use super::device_error;
use crate::Error;
use ash::vk;
use elegance_sys::gpu_image::{self as raw, Fence};
use std::{
    ffi::CStr,
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
};

pub(super) const NAMES: [&CStr; 2] = [
    ash::khr::external_memory_fd::NAME,
    ash::khr::external_semaphore_fd::NAME,
];
pub(super) const MEMORY: vk::ExternalMemoryHandleTypeFlags =
    vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;
/// The handle types that the image's creation lists: the one that the consumer imports.
pub(super) const IMAGE_HANDLES: vk::ExternalMemoryHandleTypeFlags = MEMORY;
/// Whether the capability checks ask for exporting again what was imported.
pub(super) const REEXPORT_REQUIRED: bool = true;
pub(super) const SEMAPHORE: vk::ExternalSemaphoreHandleTypeFlags =
    vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD;
pub(super) const TRANSPORT: u32 = raw::TRANSPORT_VULKAN_OPAQUE_FD;
pub(super) const FENCE: u32 = raw::FENCE_VK_OPAQUE_FD;
/// What the producer's writes signal: a binary semaphore has one value.
pub(super) const PRODUCER_VALUE: u64 = 1;

pub(super) struct Handles {
    memory: ash::khr::external_memory_fd::Device,
    semaphore: ash::khr::external_semaphore_fd::Device,
}

impl Handles {
    pub(super) fn new(instance: &ash::Instance, device: &ash::Device) -> Result<Self, Error> {
        Ok(Self {
            memory: ash::khr::external_memory_fd::Device::new(instance, device),
            semaphore: ash::khr::external_semaphore_fd::Device::new(instance, device),
        })
    }

    /// A descriptor for `memory`, which the caller owns.
    pub(super) unsafe fn export_memory(&self, memory: vk::DeviceMemory) -> Result<OwnedFd, Error> {
        let fd = unsafe {
            self.memory.get_memory_fd(
                &vk::MemoryGetFdInfoKHR::default()
                    .memory(memory)
                    .handle_type(MEMORY),
            )
        }
        .map_err(device_error)?;
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    /// A descriptor for `semaphore`, which the caller owns.
    pub(super) unsafe fn export_semaphore(
        &self,
        semaphore: vk::Semaphore,
    ) -> Result<OwnedFd, Error> {
        let fd = unsafe {
            self.semaphore.get_semaphore_fd(
                &vk::SemaphoreGetFdInfoKHR::default()
                    .semaphore(semaphore)
                    .handle_type(SEMAPHORE),
            )
        }
        .map_err(device_error)?;
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    /// Makes `consumer`'s semaphore the payload of `semaphore`; the descriptor is consumed only
    /// when this succeeds.
    pub(super) unsafe fn import_semaphore(
        &self,
        semaphore: vk::Semaphore,
        consumer: Consumer,
    ) -> Result<(), Error> {
        let import = vk::ImportSemaphoreFdInfoKHR::default()
            .semaphore(semaphore)
            .handle_type(SEMAPHORE)
            .fd(consumer.fd.as_raw_fd());
        unsafe { self.semaphore.import_semaphore_fd(&import) }.map_err(device_error)?;
        let _ = consumer.fd.into_raw_fd();
        Ok(())
    }
}

/// The semaphore that carries the producer's completion to the consumer, which the producer
/// exports: binary, with an export info.
pub(super) fn create_producer_semaphore(device: &ash::Device) -> Result<vk::Semaphore, Error> {
    let mut export = vk::ExportSemaphoreCreateInfo::default().handle_types(SEMAPHORE);
    unsafe {
        device.create_semaphore(
            &vk::SemaphoreCreateInfo::default().push_next(&mut export),
            None,
        )
    }
    .map_err(device_error)
}

/// Memory is allocated from any memory type.
pub(super) fn memory_type_allowed(_: vk::MemoryPropertyFlags) -> bool {
    true
}

/// A semaphore for a consumer's, which `import_semaphore` fills.
pub(super) fn create_consumer_semaphore(device: &ash::Device) -> Result<vk::Semaphore, Error> {
    unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }
        .map_err(device_error)
}

/// A consumer's fence from a release, as a descriptor of the producer's own.
pub(super) struct Consumer {
    pub(super) fd: OwnedFd,
}

/// Never consume a callback's borrowed FD. Continue duplicating subsequent
/// fences even after a failure, and do no Vulkan work or GPU waits here.
pub(super) fn duplicate_fence(fence: &Fence) -> Result<Consumer, Error> {
    if fence.type_ != FENCE || fence.fd < 0 || fence.value != 1 {
        return Err(device_error(
            "invalid consumer Vulkan binary semaphore fence",
        ));
    }
    let fd = unsafe { libc::fcntl(fence.fd, libc::F_DUPFD_CLOEXEC, 0) };
    if fd < 0 {
        return Err(device_error(std::io::Error::last_os_error()));
    }
    Ok(Consumer {
        fd: unsafe { OwnedFd::from_raw_fd(fd) },
    })
}

pub(super) fn check_semaphore(
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
) -> Result<(), Error> {
    let semaphore_info = vk::PhysicalDeviceExternalSemaphoreInfo::default().handle_type(SEMAPHORE);
    let mut semaphore = vk::ExternalSemaphoreProperties::default();
    unsafe {
        instance.get_physical_device_external_semaphore_properties(
            physical,
            &semaphore_info,
            &mut semaphore,
        )
    };
    // No SemaphoreTypeCreateInfo in the query/create chain: binary, not timeline.
    if !semaphore.external_semaphore_features.contains(
        vk::ExternalSemaphoreFeatureFlags::IMPORTABLE
            | vk::ExternalSemaphoreFeatureFlags::EXPORTABLE,
    ) || !semaphore.compatible_handle_types.contains(SEMAPHORE)
        || !semaphore
            .export_from_imported_handle_types
            .contains(SEMAPHORE)
    {
        return Err(device_error(
            "binary OPAQUE_FD semaphore import/export is unsupported",
        ));
    }
    Ok(())
}
