//! Linux OPAQUE_FD transport. `new` owns private queues; unsafe `from_device`
//! attaches an externally serialized producer queue and a reserved retirement queue.
//! Each publication owns a fresh dedicated allocation; unknown completion is
//! quarantined (including its budget permit), never guessed from device loss.
//! Before HAL import, allocation performs a native GPU zero-clear and waits up
//! to five seconds for its fence on the calling thread. No CPU pixels are copied;
//! an uncertain submission/wait quarantines the allocation rather than freeing it.
use crate::{Config, Error, GpuImage};
use ash::vk;
use elegance_sys::gpu_image::{self as raw, Fence, Frame, Release};
use std::{
    ffi::{c_void, CStr},
    mem::ManuallyDrop,
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
use wgpu::hal::{self, api::Vulkan};

const HANDLE: vk::ExternalMemoryHandleTypeFlags = vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;
const SEM_HANDLE: vk::ExternalSemaphoreHandleTypeFlags =
    vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD;
// RESOURCE maps to precisely this layout in this fork's Vulkan HAL. Keeping
// old == new is intentional: ownership changes, not the tracked image layout.
const HANDOFF: vk::ImageLayout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
const INITIAL_LAYOUT: vk::ImageLayout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
const INITIAL_STATE: wgpu::TextureUses = wgpu::TextureUses::COPY_DST;
const INITIALIZATION_TIMEOUT_NS: u64 = 5_000_000_000;
// How long dropping a Bridge waits for the retirement worker to finish outstanding frames.
const RETIRE_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
const IMAGE_USAGE: vk::ImageUsageFlags = vk::ImageUsageFlags::from_raw(
    vk::ImageUsageFlags::TRANSFER_DST.as_raw()
        | vk::ImageUsageFlags::COLOR_ATTACHMENT.as_raw()
        | vk::ImageUsageFlags::SAMPLED.as_raw(),
);

fn device_error(e: impl std::fmt::Display) -> Error {
    Error::Device(e.to_string())
}

#[derive(Default)]
struct Counts {
    active: AtomicUsize,
    quarantined: AtomicUsize,
}
struct Permit(Arc<Counts>);
impl Permit {
    fn new(counts: &Arc<Counts>, limit: usize) -> Result<Self, Error> {
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

struct Context {
    raw: ash::Device,
    memory_fd: ash::khr::external_memory_fd::Device,
    semaphore_fd: ash::khr::external_semaphore_fd::Device,
    memory: vk::PhysicalDeviceMemoryProperties,
    producer_queue: vk::Queue,
    producer_family: u32,
    retire_queue: vk::Queue,
    retire_family: u32,
    // ash::Device is only a dispatch table; this owns the actual logical device.
    device: wgpu::Device,
    // The last wgpu Queue drop waits for idle. Keep it on the worker (or in
    // quarantine), not on the thread dropping Bridge while work is unresolved.
    _queue: wgpu::Queue,
}

struct Allocation {
    context: Arc<Context>,
    image: vk::Image,
    memory: vk::DeviceMemory,
    rendered: vk::Semaphore,
    producer: vk::Semaphore,
    producer_done: vk::Fence,
    retire_done: vk::Fence,
    initialization_done: vk::Fence,
    producer_pool: vk::CommandPool,
    retire_pool: vk::CommandPool,
    memory_fd: Option<OwnedFd>,
    producer_fd: Option<OwnedFd>,
    memory_type: u32,
    size: u64,
    permit: Permit,
}
impl Drop for Allocation {
    fn drop(&mut self) {
        // Only unsubmitted allocations or GPU-complete retirements reach here.
        let d = &self.context.raw;
        unsafe {
            d.destroy_command_pool(self.producer_pool, None);
            d.destroy_command_pool(self.retire_pool, None);
            d.destroy_fence(self.producer_done, None);
            d.destroy_fence(self.retire_done, None);
            d.destroy_fence(self.initialization_done, None);
            d.destroy_semaphore(self.rendered, None);
            d.destroy_semaphore(self.producer, None);
            d.destroy_image(self.image, None);
            d.free_memory(self.memory, None);
        }
    }
}
struct Backing {
    texture: wgpu::Texture,
    allocation: Arc<Allocation>,
}
// Incomplete GPU work must survive *any* drop path, including worker unwinding
// and messages discarded by a dead receiver. Only proof of completion arms Drop.
struct GpuLifetime<T> {
    value: ManuallyDrop<T>,
    counts: Arc<Counts>,
    complete: bool,
}
impl<T> Drop for GpuLifetime<T> {
    fn drop(&mut self) {
        if self.complete {
            unsafe { ManuallyDrop::drop(&mut self.value) };
        } else {
            self.counts.quarantined.fetch_add(1, Ordering::Relaxed);
        }
    }
}
struct RetirementResources {
    backing: Backing,
    fds: Vec<OwnedFd>,
    semaphores: Vec<vk::Semaphore>,
}
impl Drop for RetirementResources {
    fn drop(&mut self) {
        for semaphore in self.semaphores.drain(..) {
            unsafe {
                self.backing
                    .allocation
                    .context
                    .raw
                    .destroy_semaphore(semaphore, None)
            };
        }
    }
}
struct Retirement {
    resources: GpuLifetime<RetirementResources>,
    layouts: Option<(vk::ImageLayout, vk::ImageLayout)>,
    failed: bool,
    submitted: bool,
}
impl Retirement {
    fn new(backing: Backing) -> Self {
        let counts = backing.allocation.permit.0.clone();
        Self {
            resources: GpuLifetime {
                value: ManuallyDrop::new(RetirementResources {
                    backing,
                    fds: Vec::new(),
                    semaphores: Vec::new(),
                }),
                counts,
                complete: false,
            },
            layouts: None,
            failed: false,
            submitted: false,
        }
    }
}
fn quarantine(retired: Retirement) {
    debug_assert!(!retired.resources.complete);
    drop(retired);
}
fn send(tx: &mpsc::Sender<Retirement>, retired: Retirement) {
    if let Err(e) = tx.send(retired) {
        quarantine(e.0);
    }
}
struct EncodingGuard(Option<Backing>);
impl Drop for EncodingGuard {
    fn drop(&mut self) {
        if let Some(backing) = self.0.take() {
            quarantine(Retirement::new(backing));
        }
    }
}
struct Callback {
    backing: Backing,
    tx: mpsc::Sender<Retirement>,
}

struct StagedSignal<'a> {
    queue: &'a wgpu::Queue,
    semaphore: vk::Semaphore,
}
impl StagedSignal<'_> {
    fn remove(&self) -> bool {
        if let Some(queue) = unsafe { self.queue.as_hal::<Vulkan>() } {
            queue.remove_signal_semaphore(self.semaphore)
        } else {
            false
        }
    }
}
impl Drop for StagedSignal<'_> {
    fn drop(&mut self) {
        // Validation can return without consuming the staged signal, and user
        // error callbacks can unwind. Do not contaminate the next app submit.
        self.remove();
    }
}

unsafe extern "C" fn released(userdata: *mut c_void, release: *const Release) {
    let state = unsafe { Box::from_raw(userdata.cast::<Callback>()) };
    let mut retired = Retirement::new(state.backing);
    retired.failed = release.is_null();
    if let Some(release) = unsafe { release.as_ref() } {
        retired.failed |= release.status != raw::STATUS_OK
            || (release.fence_count != 0 && release.fences.is_null());
        retired.layouts = Some((
            vk::ImageLayout::from_raw(release.vk_old_layout),
            vk::ImageLayout::from_raw(release.vk_new_layout),
        ));
        if !release.fences.is_null() {
            for i in 0..release.fence_count {
                let fence = unsafe { &*release.fences.add(i) };
                match duplicate_fence(fence) {
                    Ok(fd) => retired.resources.value.fds.push(fd),
                    Err(_) => retired.failed = true,
                }
            }
        }
    }
    send(&state.tx, retired);
}

// Never consume a callback's borrowed FD. Continue duplicating subsequent
// fences even after a failure, and do no Vulkan work or GPU waits here.
fn duplicate_fence(fence: &Fence) -> Result<OwnedFd, Error> {
    if fence.r#type != raw::FENCE_VK_OPAQUE_FD || fence.fd < 0 || fence.value != 1 {
        return Err(device_error(
            "invalid consumer Vulkan binary semaphore fence",
        ));
    }
    let fd = unsafe { libc::fcntl(fence.fd, libc::F_DUPFD_CLOEXEC, 0) };
    if fd < 0 {
        return Err(device_error(std::io::Error::last_os_error()));
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn range() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(1)
}

/// Record an ownership barrier in its own pool. Pools are never shared between
/// producer and worker threads, even when both queues belong to the same family.
unsafe fn ownership_commands(
    allocation: &Allocation,
    acquire: Option<(vk::ImageLayout, vk::ImageLayout)>,
) -> Result<vk::CommandBuffer, Error> {
    let c = &allocation.context;
    let (pool, src, dst, old, new) = match acquire {
        Some((old, new)) => (
            allocation.retire_pool,
            vk::QUEUE_FAMILY_EXTERNAL,
            c.retire_family,
            old,
            new,
        ),
        None => (
            allocation.producer_pool,
            c.producer_family,
            vk::QUEUE_FAMILY_EXTERNAL,
            HANDOFF,
            HANDOFF,
        ),
    };
    let d = &c.raw;
    let commands = unsafe {
        d.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )
    }
    .map_err(device_error)?[0];
    unsafe {
        d.begin_command_buffer(
            commands,
            &vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )
    }
    .map_err(device_error)?;
    let barrier = vk::ImageMemoryBarrier::default()
        .image(allocation.image)
        .subresource_range(range())
        .old_layout(old)
        .new_layout(new)
        .src_queue_family_index(src)
        .dst_queue_family_index(dst)
        .src_access_mask(if acquire.is_some() {
            vk::AccessFlags::empty()
        } else {
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE
        })
        .dst_access_mask(if acquire.is_some() {
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE
        } else {
            vk::AccessFlags::empty()
        });
    unsafe {
        d.cmd_pipeline_barrier(
            commands,
            if acquire.is_some() {
                vk::PipelineStageFlags::TOP_OF_PIPE
            } else {
                vk::PipelineStageFlags::ALL_COMMANDS
            },
            if acquire.is_some() {
                vk::PipelineStageFlags::ALL_COMMANDS
            } else {
                vk::PipelineStageFlags::BOTTOM_OF_PIPE
            },
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
        d.end_command_buffer(commands)
    }
    .map_err(device_error)?;
    Ok(commands)
}

fn initialization_barriers(image: vk::Image) -> [vk::ImageMemoryBarrier<'static>; 2] {
    let base = vk::ImageMemoryBarrier::default()
        .image(image)
        .subresource_range(range())
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED);
    [
        base.old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(INITIAL_LAYOUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE),
        // The first wgpu use might also be COPY_DST, in which case it need not
        // emit another barrier. Publish the clear's writes to subsequent GPU
        // operations explicitly, not merely through the host fence wait.
        base.old_layout(INITIAL_LAYOUT)
            .new_layout(INITIAL_LAYOUT)
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE),
    ]
}

fn initialize(allocation: &Arc<Allocation>) -> Result<(), Error> {
    let c = &allocation.context;
    let d = &c.raw;
    let commands = unsafe {
        d.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(allocation.producer_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )
    }
    .map_err(device_error)?[0];
    unsafe {
        d.begin_command_buffer(
            commands,
            &vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )
    }
    .map_err(device_error)?;
    let barriers = initialization_barriers(allocation.image);
    unsafe {
        d.cmd_pipeline_barrier(
            commands,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &barriers[..1],
        );
        d.cmd_clear_color_image(
            commands,
            allocation.image,
            INITIAL_LAYOUT,
            &vk::ClearColorValue { float32: [0.0; 4] },
            &[range()],
        );
        d.cmd_pipeline_barrier(
            commands,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &barriers[1..],
        );
        d.end_command_buffer(commands)
    }
    .map_err(device_error)?;

    // Before vkQueueSubmit, errors can safely drop the unsubmitted allocation.
    // From the submit attempt until a successful fence wait, ALL exits (even
    // unwinding) must retain memory, image, command pool, fence, device and queue.
    let mut guard = GpuLifetime {
        value: ManuallyDrop::new(allocation.clone()),
        counts: allocation.permit.0.clone(),
        complete: false,
    };
    let commands = [commands];
    let submit = vk::SubmitInfo::default().command_buffers(&commands);
    unsafe { d.queue_submit(c.producer_queue, &[submit], allocation.initialization_done) }
        .map_err(|e| {
            device_error(format!(
                "Vulkan image initialization submit failed: {e:?}; allocation quarantined"
            ))
        })?;
    unsafe {
        d.wait_for_fences(
            &[allocation.initialization_done],
            true,
            INITIALIZATION_TIMEOUT_NS,
        )
    }
    .map_err(|e| {
        device_error(format!(
            "Vulkan image initialization fence wait failed: {e:?}; allocation quarantined"
        ))
    })?;
    guard.complete = true;
    // The command buffer stays in producer_pool for the allocation's lifetime;
    // no pending commands or pool can be freed on an uncertain wait result.
    Ok(())
}

fn valid_return_layout(layout: vk::ImageLayout) -> bool {
    matches!(
        layout,
        vk::ImageLayout::GENERAL
            | vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            | vk::ImageLayout::TRANSFER_DST_OPTIMAL
            | vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
    )
}

// Called only by the isolated queue's single worker. First poll the producer's
// fence, then wait every imported binary semaphore in an acquire submission.
fn progress(retired: &mut Retirement) -> Result<bool, Error> {
    let resources = &mut *retired.resources.value;
    let a = &resources.backing.allocation;
    let c = &a.context;
    if !unsafe { c.raw.get_fence_status(a.producer_done) }.map_err(device_error)? {
        return Ok(false);
    }
    let Some(layouts) = retired.layouts else {
        return Ok(true);
    }; // rejected publish: no consumer
    if !valid_return_layout(layouts.0) || !valid_return_layout(layouts.1) {
        return Err(device_error("consumer returned unsupported Vulkan layouts"));
    }
    if !retired.submitted {
        for fd in resources.fds.drain(..) {
            let semaphore = unsafe {
                c.raw
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }
            .map_err(device_error)?;
            resources.semaphores.push(semaphore);
            let import = vk::ImportSemaphoreFdInfoKHR::default()
                .semaphore(semaphore)
                .handle_type(SEM_HANDLE)
                .fd(fd.as_raw_fd());
            unsafe { c.semaphore_fd.import_semaphore_fd(&import) }.map_err(device_error)?;
            // OPAQUE_FD import consumes the duplicate only on success.
            let _ = fd.into_raw_fd();
        }
        let commands = [unsafe { ownership_commands(a, Some(layouts)) }?];
        let stages = vec![vk::PipelineStageFlags::ALL_COMMANDS; resources.semaphores.len()];
        let info = vk::SubmitInfo::default()
            .wait_semaphores(&resources.semaphores)
            .wait_dst_stage_mask(&stages)
            .command_buffers(&commands);
        unsafe { c.raw.queue_submit(c.retire_queue, &[info], a.retire_done) }
            .map_err(device_error)?;
        retired.submitted = true;
    }
    unsafe { c.raw.get_fence_status(a.retire_done) }.map_err(device_error)
}

fn retire_worker(rx: mpsc::Receiver<Retirement>, context: Arc<Context>) {
    let mut pending = Vec::<Retirement>::new();
    let mut connected = true;
    while connected || !pending.is_empty() {
        let next = if pending.is_empty() {
            rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        } else {
            rx.recv_timeout(Duration::from_millis(2))
        };
        match next {
            Ok(r) => {
                if r.failed {
                    quarantine(r);
                } else {
                    pending.push(r);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => connected = false,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let mut i = 0;
        while i < pending.len() {
            match progress(&mut pending[i]) {
                Ok(false) => i += 1,
                Ok(true) => {
                    pending[i].resources.complete = true;
                    drop(pending.swap_remove(i));
                }
                Err(_) => quarantine(pending.swap_remove(i)),
            }
        }
        // Drive HAL deferred destruction as well as wgpu submission bookkeeping.
        let _ = context.device.poll(wgpu::PollType::Poll);
        if !connected && !pending.is_empty() {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

/// A same-adapter Vulkan bridge with an exclusive retirement queue.
/// `new` creates its own producer; unsafe `from_device` attaches an existing pair.
/// Native import still requires Elegance's renderer to select that physical GPU.
/// There is no cross-adapter, DMA-BUF, CPU-copy, or device-loss fallback.
pub struct Bridge {
    device: wgpu::Device,
    queue: wgpu::Queue,
    image: GpuImage,
    context: Arc<Context>,
    config: Config,
    counts: Arc<Counts>,
    tx: mpsc::Sender<Retirement>,
    worker_done: mpsc::Receiver<()>,
}
impl Bridge {
    pub async fn new(adapter: &wgpu::Adapter, config: Config) -> Result<Self, Error> {
        if adapter.get_info().backend != wgpu::Backend::Vulkan {
            return Err(Error::Unsupported);
        }
        if config.width == 0
            || config.height == 0
            || config.max_frames == 0
            || !matches!(
                config.format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
            )
        {
            return Err(Error::InvalidConfig);
        }
        let hal_adapter = unsafe { adapter.as_hal::<Vulkan>() }.ok_or(Error::Unsupported)?;
        let instance = hal_adapter.shared_instance().raw_instance();
        let physical = hal_adapter.raw_physical_device();
        let extensions = unsafe { instance.enumerate_device_extension_properties(physical) }
            .map_err(device_error)?;
        for required in [
            ash::khr::external_memory_fd::NAME,
            ash::khr::external_semaphore_fd::NAME,
        ] {
            if !extensions
                .iter()
                .any(|e| unsafe { CStr::from_ptr(e.extension_name.as_ptr()) } == required)
            {
                return Err(device_error(format!(
                    "required Vulkan extension unavailable: {}",
                    required.to_string_lossy()
                )));
            }
        }
        let format = hal_adapter.texture_format_as_raw(config.format);
        check_capabilities(instance, physical, format, config)?;
        let families = unsafe { instance.get_physical_device_queue_family_properties(physical) };
        // The local HAL's open_with_callback selects family 0, queue 0. Do not
        // silently change it: device_from_raw below still records that family.
        let producer = families
            .first()
            .ok_or_else(|| device_error("Vulkan queue family 0 missing"))?;
        if !producer.queue_flags.contains(vk::QueueFlags::GRAPHICS) || producer.queue_count == 0 {
            return Err(device_error("this HAL requires graphics queue family 0"));
        }
        let (retire_family, retire_index) = if producer.queue_count >= 2 {
            (0, 1)
        } else {
            let family = families
                .iter()
                .enumerate()
                .skip(1)
                .find(|(_, p)| {
                    p.queue_count > 0
                        && p.queue_flags.intersects(
                            vk::QueueFlags::GRAPHICS
                                | vk::QueueFlags::COMPUTE
                                | vk::QueueFlags::TRANSFER,
                        )
                })
                .ok_or_else(|| {
                    device_error("a second, isolated Vulkan queue is required for retirement")
                })?;
            (family.0 as u32, 0)
        };
        let priorities = [1.0, 1.0];
        let desc = wgpu::DeviceDescriptor::default();
        let opened = unsafe {
            hal_adapter.open_with_callback(
                desc.required_features,
                &desc.required_limits,
                &desc.memory_hints,
                Some(Box::new(|args| {
                    for extension in [
                        ash::khr::external_memory_fd::NAME,
                        ash::khr::external_semaphore_fd::NAME,
                    ] {
                        if !args.extensions.contains(&extension) {
                            args.extensions.push(extension);
                        }
                    }
                    if retire_family == 0 {
                        args.queue_create_infos[0] = vk::DeviceQueueCreateInfo::default()
                            .queue_family_index(0)
                            .queue_priorities(&priorities);
                    } else {
                        args.queue_create_infos.push(
                            vk::DeviceQueueCreateInfo::default()
                                .queue_family_index(retire_family)
                                .queue_priorities(&priorities[..1]),
                        );
                    }
                })),
            )
        }
        .map_err(device_error)?;
        let (device, queue) = unsafe { adapter.create_device_from_hal::<Vulkan>(opened, &desc) }
            .map_err(device_error)?;
        drop(hal_adapter);
        // Both queues were created above and have never been exposed to others.
        unsafe { Self::attach(device, queue, config, retire_family, retire_index) }
    }

    /// Attach an existing Vulkan logical device/queue. Existing textures from
    /// this logical device can be passed to `copy_from` without another device.
    ///
    /// Creation requirements: enable VK_KHR_external_memory_fd AND
    /// VK_KHR_external_semaphore_fd (not merely physical-device support). Create
    /// two unprotected queues in the producer's graphics family: when the HAL
    /// producer uses index 0, retirement uses index 1; otherwise it uses index 0.
    /// This same-family requirement is specific to attachment: `new` can instead
    /// create a retirement queue in a different family when necessary.
    /// Ordinary `Adapter::request_device` does not promise these requirements;
    /// use HAL `open_with_callback`/`create_device_from_hal` or use `new` instead.
    ///
    /// # Safety
    /// - `device` and `queue` must be the SAME logical pair, not just the same GPU.
    /// - The secondary queue selected above MUST have been requested in
    ///   VkDeviceCreateInfo, with flags zero. Vulkan has no created-queue query;
    ///   this constructor can check physical capacity, NOT actual creation.
    /// - Reserve that secondary queue exclusively for this bridge for the entire
    ///   logical-device lifetime. No other bridge/native user may submit, present,
    ///   wait-idle, or otherwise operate on it, including AFTER Bridge is dropped:
    ///   native callbacks and the detached retirement worker can outlive Bridge.
    /// - Serialize the ENTIRE copy_from/clear_color/render call (allocation and
    ///   publication included) against all submissions, writes, and native/HAL
    ///   producer-queue operations through every alias. No reentrant queue ops
    ///   from callbacks. Ordinary application queue use BETWEEN calls is allowed.
    ///   Enter with no staged HAL waits/signals; drain them first. The retirement
    ///   worker polls this shared device and can deliver application callbacks
    ///   on its thread; those callbacks must obey the same serialization rules.
    /// - Never explicitly destroy the logical device while any publication,
    ///   callback, or worker work remains outstanding, even after Bridge drops.
    ///   Dropping ordinary wgpu handles is allowed: retirement retains owners.
    /// - Source writes must be submitted on this queue before copy_from, or be
    ///   explicitly synchronized from another queue. Never write published
    ///   destinations afterwards. Elegance must use the same physical GPU.
    ///
    /// Missing enabled extensions or physical secondary-queue capacity returns
    /// `Error::Device`. Moving cloneable wgpu handles does not prove exclusivity.
    pub unsafe fn from_device(
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: Config,
    ) -> Result<Self, Error> {
        let hal_device = unsafe { device.as_hal::<Vulkan>() }.ok_or(Error::Unsupported)?;
        let family = hal_device.queue_family_index();
        let instance = hal_device.shared_instance().raw_instance();
        let families = unsafe {
            instance.get_physical_device_queue_family_properties(hal_device.raw_physical_device())
        };
        let properties = families
            .get(family as usize)
            .ok_or_else(|| device_error("producer queue family is missing"))?;
        let index = secondary_queue_index(hal_device.queue_index(), properties.queue_count)?;
        drop(hal_device);
        // Actual secondary-queue creation and exclusive ownership are the
        // caller's preconditions, not inferred from physical queue_count.
        unsafe { Self::attach(device, queue, config, family, index) }
    }

    unsafe fn attach(
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: Config,
        retire_family: u32,
        retire_index: u32,
    ) -> Result<Self, Error> {
        if config.width == 0
            || config.height == 0
            || config.max_frames == 0
            || !matches!(
                config.format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
            )
        {
            return Err(Error::InvalidConfig);
        }
        let hal_device = unsafe { device.as_hal::<Vulkan>() }.ok_or(Error::Unsupported)?;
        let hal_queue = unsafe { queue.as_hal::<Vulkan>() }.ok_or(Error::Unsupported)?;
        if hal_device.raw_device().handle() != hal_queue.raw_device().handle()
            || hal_device.raw_queue() != hal_queue.as_raw()
        {
            return Err(Error::InvalidConfig);
        }
        for required in [
            ash::khr::external_memory_fd::NAME,
            ash::khr::external_semaphore_fd::NAME,
        ] {
            if !hal_device.enabled_device_extensions().contains(&required) {
                return Err(device_error(format!(
                    "required Vulkan extension is not enabled on this logical device: {}",
                    required.to_string_lossy()
                )));
            }
        }
        let instance = hal_device.shared_instance().raw_instance();
        let physical = hal_device.raw_physical_device();
        let producer_family = hal_device.queue_family_index();
        let producer_queue = hal_device.raw_queue();
        let families = unsafe { instance.get_physical_device_queue_family_properties(physical) };
        if !families
            .get(producer_family as usize)
            .is_some_and(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        {
            return Err(device_error("producer must use a graphics queue family"));
        }
        if !families.get(retire_family as usize).is_some_and(|p| {
            retire_index < p.queue_count
                && p.queue_flags.intersects(
                    vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
                )
        }) {
            return Err(device_error("isolated retirement queue is unavailable"));
        }
        check_capabilities(instance, physical, image_info(config).format, config)?;
        let memory = unsafe { instance.get_physical_device_memory_properties(physical) };
        let raw_device = hal_device.raw_device().clone();
        let memory_fd = ash::khr::external_memory_fd::Device::new(instance, &raw_device);
        let semaphore_fd = ash::khr::external_semaphore_fd::Device::new(instance, &raw_device);
        // attach's caller guarantees this queue was created with flags zero.
        let retire_queue = unsafe { raw_device.get_device_queue(retire_family, retire_index) };
        if retire_queue == vk::Queue::null() || retire_queue == producer_queue {
            return Err(device_error("a distinct retirement queue is required"));
        }
        drop(hal_queue);
        drop(hal_device);
        if config.width > device.limits().max_texture_dimension_2d
            || config.height > device.limits().max_texture_dimension_2d
        {
            return Err(Error::InvalidConfig);
        }
        let image = GpuImage::new(config.width, config.height, config.origin)
            .map_err(device_error)?;
        let context = Arc::new(Context {
            raw: raw_device,
            memory_fd,
            semaphore_fd,
            memory,
            producer_queue,
            producer_family,
            retire_queue,
            retire_family,
            device: device.clone(),
            _queue: queue.clone(),
        });
        let (tx, rx) = mpsc::channel();
        // Disconnects once the worker has dropped its context, for Bridge's drop to wait on.
        let (worker_exit, worker_done) = mpsc::channel::<()>();
        let worker_context = context.clone();
        std::thread::Builder::new()
            .name("elegance-vulkan-retire".into())
            .spawn(move || {
                retire_worker(rx, worker_context);
                drop(worker_exit);
            })
            .map_err(device_error)?;
        Ok(Self {
            device,
            queue,
            image,
            context,
            config,
            counts: Arc::default(),
            tx,
            worker_done,
        })
    }
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
    fn extent(&self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.config.width,
            height: self.config.height,
            depth_or_array_layers: 1,
        }
    }
    /// Source must belong to device(), be single-sampled, same size/format, and COPY_SRC.
    /// Device mismatch follows wgpu's validation/error-scope policy.
    pub fn copy_from(&mut self, source: &wgpu::Texture) -> Result<(), Error> {
        if source.size() != self.extent()
            || source.format() != self.config.format
            || source.dimension() != wgpu::TextureDimension::D2
            || source.sample_count() != 1
            || !source.usage().contains(wgpu::TextureUsages::COPY_SRC)
        {
            return Err(Error::InvalidConfig);
        }
        let size = self.extent();
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
    /// RGB values are stored unchanged in UNORM storage (supply sRGB-encoded RGB).
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
    /// Encode, submit, release external ownership, and publish a fresh image.
    /// Allocation first zero-clears the native image on the GPU and waits up to
    /// five seconds for its initialization fence before HAL import. This initial
    /// implementation therefore blocks the caller during allocation, but never
    /// uploads or reads back CPU pixels.
    ///
    /// # Safety
    /// Fully initialize the image with straight alpha and sRGB-encoded RGB. Do not
    /// retain/clone the target, views, bind groups, native handles, or commands
    /// referencing it beyond the closure. Do not destroy the target/device or
    /// submit through another queue. No later writes or escaped handles are allowed.
    pub unsafe fn render_raw(
        &mut self,
        encode: impl FnOnce(&wgpu::Device, &mut wgpu::CommandEncoder, &wgpu::Texture),
    ) -> Result<(), Error> {
        let backing = self.allocate()?;
        let mut guard = EncodingGuard(Some(backing));
        let backing = guard.0.as_ref().unwrap();
        let a = &backing.allocation;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encode(&self.device, &mut encoder, &backing.texture);
        // Establish a known *normal tracked state*, not UNDEFINED/PRESENT or an
        // untracked native transition. Release preserves this exact layout.
        encoder.transition_resources(
            std::iter::empty(),
            std::iter::once(wgpu::TextureTransition {
                texture: &backing.texture,
                selector: None,
                state: wgpu::TextureUses::RESOURCE,
            }),
        );
        let commands = encoder.finish();
        let release = [unsafe { ownership_commands(a, None) }?];
        let staged = StagedSignal {
            queue: &self.queue,
            semaphore: a.rendered,
        };
        let hal_queue = unsafe { self.queue.as_hal::<Vulkan>() }.ok_or(Error::Unsupported)?;
        hal_queue.add_signal_semaphore(a.rendered, None);
        drop(hal_queue);
        self.queue.submit([commands]);
        if staged.remove() {
            // No native submission consumed this signal. Waiting on it would
            // poison the producer queue indefinitely after a validation error.
            return Err(device_error(
                "wgpu rejected the producer submission before consuming its Vulkan signal",
            ));
        }
        drop(staged);
        let waits = [a.rendered];
        let stages = [vk::PipelineStageFlags::ALL_COMMANDS];
        let signals = [a.producer];
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(&waits)
            .wait_dst_stage_mask(&stages)
            .command_buffers(&release)
            .signal_semaphores(&signals);
        unsafe {
            self.context
                .raw
                .queue_submit(self.context.producer_queue, &[submit], a.producer_done)
        }
        .map_err(device_error)?;
        let backing = guard.0.take().unwrap();
        // These exact structures (including pNext) remain live throughout synchronous import.
        let mut external = vk::ExternalMemoryImageCreateInfo::default().handle_types(HANDLE);
        let info = image_info(self.config).push_next(&mut external);
        let frame = Frame {
            transport: raw::TRANSPORT_VULKAN_OPAQUE_FD,
            format: if self.config.format == wgpu::TextureFormat::Bgra8Unorm {
                raw::FORMAT_BGRA8
            } else {
                raw::FORMAT_RGBA8
            },
            width: self.config.width,
            height: self.config.height,
            handle: std::ptr::null_mut(),
            memory_fd: backing.allocation.memory_fd.as_ref().unwrap().as_raw_fd(),
            vk_image_create_info: (&info as *const vk::ImageCreateInfo<'_>).cast(),
            vk_memory_type_index: backing.allocation.memory_type,
            vk_allocation_size: backing.allocation.size,
            vk_dedicated_allocation: 1,
            vk_old_layout: HANDOFF.as_raw(),
            vk_new_layout: HANDOFF.as_raw(),
            producer_fence: Fence {
                r#type: raw::FENCE_VK_OPAQUE_FD,
                handle: std::ptr::null_mut(),
                fd: backing.allocation.producer_fd.as_ref().unwrap().as_raw_fd(),
                value: 1,
            },
            token: 0,
            userdata: Box::into_raw(Box::new(Callback {
                backing,
                tx: self.tx.clone(),
            }))
            .cast(),
            release: Some(released),
        };
        let status = unsafe { raw::elg_gpu_image_publish(self.image.as_raw(), &frame) };
        if status != raw::STATUS_OK {
            // No callback/no consumer on rejection, but still wait producer GPU completion.
            let state = unsafe { Box::from_raw(frame.userdata.cast::<Callback>()) };
            send(&state.tx, Retirement::new(state.backing));
            return Err(Error::Publish(status));
        }
        Ok(())
    }
    fn allocate(&self) -> Result<Backing, Error> {
        let _ = self.device.poll(wgpu::PollType::Poll);
        let permit = Permit::new(&self.counts, self.config.max_frames)?;
        let c = &self.context;
        let d = &c.raw;
        let mut a = Allocation {
            context: c.clone(),
            image: vk::Image::null(),
            memory: vk::DeviceMemory::null(),
            rendered: vk::Semaphore::null(),
            producer: vk::Semaphore::null(),
            producer_done: vk::Fence::null(),
            retire_done: vk::Fence::null(),
            initialization_done: vk::Fence::null(),
            producer_pool: vk::CommandPool::null(),
            retire_pool: vk::CommandPool::null(),
            memory_fd: None,
            producer_fd: None,
            memory_type: 0,
            size: 0,
            permit,
        };
        let mut external = vk::ExternalMemoryImageCreateInfo::default().handle_types(HANDLE);
        let info = image_info(self.config).push_next(&mut external);
        a.image = unsafe { d.create_image(&info, None) }.map_err(device_error)?;
        let requirements = unsafe { d.get_image_memory_requirements(a.image) };
        a.size = requirements.size;
        a.memory_type = c
            .memory
            .memory_types_as_slice()
            .iter()
            .enumerate()
            .filter(|(i, _)| requirements.memory_type_bits & (1u32 << i) != 0)
            .max_by_key(|(_, m)| {
                m.property_flags
                    .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
            })
            .map(|(i, _)| i as u32)
            .ok_or(Error::Allocation)?;
        let mut export = vk::ExportMemoryAllocateInfo::default().handle_types(HANDLE);
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(a.image);
        let allocate = vk::MemoryAllocateInfo::default()
            .allocation_size(a.size)
            .memory_type_index(a.memory_type)
            .push_next(&mut export)
            .push_next(&mut dedicated);
        a.memory = unsafe { d.allocate_memory(&allocate, None) }.map_err(device_error)?;
        unsafe { d.bind_image_memory(a.image, a.memory, 0) }.map_err(device_error)?;
        let fd = unsafe {
            c.memory_fd.get_memory_fd(
                &vk::MemoryGetFdInfoKHR::default()
                    .memory(a.memory)
                    .handle_type(HANDLE),
            )
        }
        .map_err(device_error)?;
        a.memory_fd = Some(unsafe { OwnedFd::from_raw_fd(fd) });
        a.rendered = unsafe { d.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }
            .map_err(device_error)?;
        let mut export_semaphore =
            vk::ExportSemaphoreCreateInfo::default().handle_types(SEM_HANDLE);
        a.producer = unsafe {
            d.create_semaphore(
                &vk::SemaphoreCreateInfo::default().push_next(&mut export_semaphore),
                None,
            )
        }
        .map_err(device_error)?;
        let fd = unsafe {
            c.semaphore_fd.get_semaphore_fd(
                &vk::SemaphoreGetFdInfoKHR::default()
                    .semaphore(a.producer)
                    .handle_type(SEM_HANDLE),
            )
        }
        .map_err(device_error)?;
        a.producer_fd = Some(unsafe { OwnedFd::from_raw_fd(fd) });
        a.producer_done = unsafe { d.create_fence(&vk::FenceCreateInfo::default(), None) }
            .map_err(device_error)?;
        a.retire_done = unsafe { d.create_fence(&vk::FenceCreateInfo::default(), None) }
            .map_err(device_error)?;
        a.initialization_done = unsafe { d.create_fence(&vk::FenceCreateInfo::default(), None) }
            .map_err(device_error)?;
        a.producer_pool = unsafe {
            d.create_command_pool(
                &vk::CommandPoolCreateInfo::default().queue_family_index(c.producer_family),
                None,
            )
        }
        .map_err(device_error)?;
        a.retire_pool = unsafe {
            d.create_command_pool(
                &vk::CommandPoolCreateInfo::default().queue_family_index(c.retire_family),
                None,
            )
        }
        .map_err(device_error)?;
        let allocation = Arc::new(a);
        // HAL wrapping bypasses wgpu's zero-initialization path. Clear every
        // texel and prove GPU completion before exposing any wgpu handle.
        initialize(&allocation)?;
        let hal_desc = hal::TextureDescriptor {
            label: Some("Elegance Vulkan external image"),
            size: self.extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUses::COPY_DST
                | wgpu::TextureUses::COLOR_TARGET
                | wgpu::TextureUses::RESOURCE,
            memory_flags: hal::MemoryFlags::empty(),
            view_formats: vec![self.config.format.add_srgb_suffix()],
        };
        let hal_device = unsafe { c.device.as_hal::<Vulkan>() }.ok_or(Error::Unsupported)?;
        let keep_alive = allocation.clone();
        let texture = unsafe {
            hal_device.texture_from_raw(
                allocation.image,
                &hal_desc,
                Some(Box::new(move || drop(keep_alive))),
                hal::vulkan::TextureMemory::External,
            )
        };
        drop(hal_device);
        let texture = unsafe {
            self.device.create_texture_from_hal::<Vulkan>(
                texture,
                &wgpu::TextureDescriptor {
                    label: hal_desc.label,
                    size: self.extent(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.config.format,
                    usage: wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &hal_desc.view_formats,
                },
                INITIAL_STATE,
            )
        };
        Ok(Backing {
            texture,
            allocation,
        })
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.image.clear();
        // Let the worker drain and exit before returning: a worker still polling fences when
        // the process exits races the driver's own teardown. Frames Elegance hasn't released
        // keep their senders, and a fence may never signal, so the wait is bounded.
        drop(std::mem::replace(&mut self.tx, mpsc::channel().0));
        let _ = self.worker_done.recv_timeout(RETIRE_DRAIN_TIMEOUT);
    }
}

fn secondary_queue_index(producer: u32, physical_count: u32) -> Result<u32, Error> {
    if physical_count < 2 || producer >= physical_count {
        return Err(device_error("two queues in the producer graphics family are required; create a dedicated Bridge::new device instead"));
    }
    Ok(if producer == 0 { 1 } else { 0 })
}

fn image_info(config: Config) -> vk::ImageCreateInfo<'static> {
    vk::ImageCreateInfo::default()
        .flags(vk::ImageCreateFlags::MUTABLE_FORMAT)
        .image_type(vk::ImageType::TYPE_2D)
        .format(if config.format == wgpu::TextureFormat::Bgra8Unorm {
            vk::Format::B8G8R8A8_UNORM
        } else {
            vk::Format::R8G8B8A8_UNORM
        })
        .extent(vk::Extent3D {
            width: config.width,
            height: config.height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(IMAGE_USAGE)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
}

fn check_capabilities(
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
    format: vk::Format,
    config: Config,
) -> Result<(), Error> {
    let mut external = vk::PhysicalDeviceExternalImageFormatInfo::default().handle_type(HANDLE);
    let info = vk::PhysicalDeviceImageFormatInfo2::default()
        .format(format)
        .ty(vk::ImageType::TYPE_2D)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(IMAGE_USAGE)
        .flags(vk::ImageCreateFlags::MUTABLE_FORMAT)
        .push_next(&mut external);
    let mut external_properties = vk::ExternalImageFormatProperties::default();
    let mut properties = vk::ImageFormatProperties2::default().push_next(&mut external_properties);
    unsafe {
        instance.get_physical_device_image_format_properties2(physical, &info, &mut properties)
    }
    .map_err(device_error)?;
    let limits = properties.image_format_properties;
    let memory = external_properties.external_memory_properties;
    if !memory.external_memory_features.contains(
        vk::ExternalMemoryFeatureFlags::EXPORTABLE | vk::ExternalMemoryFeatureFlags::IMPORTABLE,
    ) || !memory.compatible_handle_types.contains(HANDLE)
        || !memory.export_from_imported_handle_types.contains(HANDLE)
    {
        return Err(device_error(
            "exact mutable-format image does not support importable/exportable OPAQUE_FD memory",
        ));
    }
    if config.width > limits.max_extent.width
        || config.height > limits.max_extent.height
        || limits.max_mip_levels < 1
        || limits.max_array_layers < 1
        || !limits.sample_counts.contains(vk::SampleCountFlags::TYPE_1)
    {
        return Err(Error::InvalidConfig);
    }
    let srgb = if format == vk::Format::B8G8R8A8_UNORM {
        vk::Format::B8G8R8A8_SRGB
    } else {
        vk::Format::R8G8B8A8_SRGB
    };
    let srgb_properties = unsafe { instance.get_physical_device_format_properties(physical, srgb) };
    if !srgb_properties.optimal_tiling_features.contains(
        vk::FormatFeatureFlags::SAMPLED_IMAGE | vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR,
    ) {
        return Err(device_error("sRGB sampling view is unsupported"));
    }
    let semaphore_info = vk::PhysicalDeviceExternalSemaphoreInfo::default().handle_type(SEM_HANDLE);
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
    ) || !semaphore.compatible_handle_types.contains(SEM_HANDLE)
        || !semaphore
            .export_from_imported_handle_types
            .contains(SEM_HANDLE)
    {
        return Err(device_error(
            "binary OPAQUE_FD semaphore import/export is unsupported",
        ));
    }
    Ok(())
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
        drop(one);
        assert!(Permit::new(&counts, 2).is_ok());
        drop(two);
        assert_eq!(counts.active.load(Ordering::Acquire), 0);
    }
    #[test]
    fn concurrent_budget_reservations_are_bounded() {
        let counts = Arc::new(Counts::default());
        let barrier = Arc::new(std::sync::Barrier::new(16));
        let workers: Vec<_> = (0..16)
            .map(|_| {
                let counts = counts.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let permit = Permit::new(&counts, 3);
                    barrier.wait();
                    assert!(counts.active.load(Ordering::Acquire) <= 3);
                    barrier.wait();
                    drop(permit);
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(counts.active.load(Ordering::Acquire), 0);
    }
    #[test]
    fn borrowed_fence_fd_is_duplicated_and_cloexec() {
        let (source, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
        let fence = Fence {
            r#type: raw::FENCE_VK_OPAQUE_FD,
            handle: std::ptr::null_mut(),
            fd: source.as_raw_fd(),
            value: 1,
        };
        let duplicate = duplicate_fence(&fence).unwrap();
        assert_ne!(duplicate.as_raw_fd(), source.as_raw_fd());
        drop(source);
        let flags = unsafe { libc::fcntl(duplicate.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0);
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
        assert!(duplicate_fence(&Fence { value: 2, ..fence }).is_err());
        assert!(duplicate_fence(&Fence { fd: -1, ..fence }).is_err());
        assert!(duplicate_fence(&Fence {
            r#type: raw::FENCE_NONE,
            ..fence
        })
        .is_err());
    }
    struct Tracked {
        drops: Arc<AtomicUsize>,
        _permit: Permit,
    }
    impl Drop for Tracked {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn tracked(counts: &Arc<Counts>, drops: &Arc<AtomicUsize>) -> GpuLifetime<Tracked> {
        GpuLifetime {
            value: ManuallyDrop::new(Tracked {
                drops: drops.clone(),
                _permit: Permit::new(counts, 1).unwrap(),
            }),
            counts: counts.clone(),
            complete: false,
        }
    }
    #[test]
    fn worker_unwind_quarantines_resources_and_budget() {
        let counts = Arc::new(Counts::default());
        let drops = Arc::new(AtomicUsize::new(0));
        let result = std::panic::catch_unwind({
            let guard = tracked(&counts, &drops);
            move || {
                let _pending = vec![guard];
                panic!("simulated poll callback panic");
            }
        });
        assert!(result.is_err());
        assert_eq!(counts.active.load(Ordering::Acquire), 1);
        assert_eq!(counts.quarantined.load(Ordering::Acquire), 1);
        assert_eq!(drops.load(Ordering::Acquire), 0);
    }
    #[test]
    fn dead_worker_channel_quarantines_queued_resources() {
        let counts = Arc::new(Counts::default());
        let drops = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();
        tx.send(tracked(&counts, &drops)).ok().unwrap();
        drop(rx);
        assert_eq!(counts.active.load(Ordering::Acquire), 1);
        assert_eq!(counts.quarantined.load(Ordering::Acquire), 1);
        assert_eq!(drops.load(Ordering::Acquire), 0);
    }
    #[test]
    fn completed_retirement_releases_resources_and_budget() {
        let counts = Arc::new(Counts::default());
        let drops = Arc::new(AtomicUsize::new(0));
        let mut guard = tracked(&counts, &drops);
        guard.complete = true;
        drop(guard);
        assert_eq!(counts.active.load(Ordering::Acquire), 0);
        assert_eq!(counts.quarantined.load(Ordering::Acquire), 0);
        assert_eq!(drops.load(Ordering::Acquire), 1);
    }
    #[test]
    fn initialization_error_guard_retains_allocation_owner() {
        for error in [
            vk::Result::TIMEOUT,
            vk::Result::ERROR_DEVICE_LOST,
            vk::Result::ERROR_OUT_OF_DEVICE_MEMORY,
        ] {
            let counts = Arc::new(Counts::default());
            let drops = Arc::new(AtomicUsize::new(0));
            let owner = Arc::new(Tracked {
                drops: drops.clone(),
                _permit: Permit::new(&counts, 1).unwrap(),
            });
            let result = (|| -> Result<(), vk::Result> {
                let mut guard = GpuLifetime {
                    value: ManuallyDrop::new(owner.clone()),
                    counts: counts.clone(),
                    complete: false,
                };
                // Simulate an uncertain submit or fence-wait error after the
                // guard is armed, using the same early-return path as initialize.
                Err(error)?;
                guard.complete = true;
                Ok(())
            })();
            assert_eq!(result, Err(error));
            assert_eq!(Arc::strong_count(&owner), 2);
            drop(owner);
            assert_eq!(counts.active.load(Ordering::Acquire), 1);
            assert_eq!(counts.quarantined.load(Ordering::Acquire), 1);
            assert_eq!(drops.load(Ordering::Acquire), 0);
        }
    }
    #[test]
    fn native_initialization_matches_import_state_and_publishes_clear_writes() {
        use ash::vk::Handle;
        let image = vk::Image::from_raw(7);
        let barriers = initialization_barriers(image);
        assert_eq!(INITIAL_STATE, wgpu::TextureUses::COPY_DST);
        assert_eq!(INITIAL_LAYOUT, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        assert_eq!(barriers[0].old_layout, vk::ImageLayout::UNDEFINED);
        assert_eq!(barriers[0].new_layout, INITIAL_LAYOUT);
        assert_eq!(barriers[0].src_access_mask, vk::AccessFlags::empty());
        assert_eq!(barriers[0].dst_access_mask, vk::AccessFlags::TRANSFER_WRITE);
        assert_eq!(barriers[1].old_layout, INITIAL_LAYOUT);
        assert_eq!(barriers[1].new_layout, INITIAL_LAYOUT);
        assert_eq!(barriers[1].src_access_mask, vk::AccessFlags::TRANSFER_WRITE);
        assert!(barriers[1]
            .dst_access_mask
            .contains(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE));
        for barrier in barriers {
            assert_eq!(barrier.image, image);
            assert_eq!(barrier.src_queue_family_index, vk::QUEUE_FAMILY_IGNORED);
            assert_eq!(barrier.dst_queue_family_index, vk::QUEUE_FAMILY_IGNORED);
            assert_eq!(
                barrier.subresource_range.aspect_mask,
                vk::ImageAspectFlags::COLOR
            );
            assert_eq!(barrier.subresource_range.base_mip_level, 0);
            assert_eq!(barrier.subresource_range.level_count, 1);
            assert_eq!(barrier.subresource_range.base_array_layer, 0);
            assert_eq!(barrier.subresource_range.layer_count, 1);
        }
    }
    #[test]
    fn attached_secondary_queue_is_distinct_and_physically_present() {
        assert_eq!(secondary_queue_index(0, 2).unwrap(), 1);
        assert_eq!(secondary_queue_index(1, 2).unwrap(), 0);
        assert_eq!(secondary_queue_index(2, 3).unwrap(), 0);
        assert!(secondary_queue_index(0, 1).is_err());
        assert!(secondary_queue_index(2, 2).is_err());
    }
    #[test]
    fn handoff_is_a_real_tracked_layout() {
        assert_eq!(HANDOFF, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        assert!(!valid_return_layout(vk::ImageLayout::UNDEFINED));
        assert!(!valid_return_layout(vk::ImageLayout::PREINITIALIZED));
        // The exported image does not have TRANSFER_SRC usage.
        assert!(!valid_return_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL));
        assert!(valid_return_layout(HANDOFF));
    }
}
