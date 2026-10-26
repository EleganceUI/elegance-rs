//! DXGI shared textures with GPU-only production and asynchronous retirement.

use crate::{Config, Error, GpuImage};
use elegance_sys::gpu_image::{self as raw, Fence, Frame, Release};
use std::{
    ffi::c_void,
    mem::ManuallyDrop,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
use wgpu::hal::{self, api::Dx12};
use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        Foundation::{DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE},
        Graphics::{Direct3D12::*, Dxgi::Common::*},
        System::Threading::GetCurrentProcess,
    },
};

// The pinned native wgpu-core backend resolves scope futures synchronously.
// Unexpected Pending is an error, never permission to publish or block the UI.
struct NativeErrors(Vec<wgpu::ErrorScopeGuard>);
impl NativeErrors {
    fn new(device: &wgpu::Device) -> Self {
        Self(vec![
            device.push_error_scope(wgpu::ErrorFilter::OutOfMemory),
            device.push_error_scope(wgpu::ErrorFilter::Internal),
            device.push_error_scope(wgpu::ErrorFilter::Validation),
        ])
    }

    fn finish(mut self) -> Result<(), Error> {
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

impl Drop for NativeErrors {
    fn drop(&mut self) {
        while let Some(scope) = self.0.pop() {
            drop(scope);
        }
    }
}

struct StagedSignal<'a> {
    queue: &'a wgpu::Queue,
    fence: ID3D12Fence,
}

impl StagedSignal<'_> {
    fn finish(self) -> Result<(), Error> {
        let queue = unsafe { self.queue.as_hal::<Dx12>() }.ok_or(Error::Unsupported)?;
        if queue.remove_signal_fence(&self.fence) {
            Err(Error::Device(
                "wgpu submission did not consume its DX12 signal fence".into(),
            ))
        } else {
            Ok(())
        }
    }
}

impl Drop for StagedSignal<'_> {
    fn drop(&mut self) {
        if let Some(queue) = unsafe { self.queue.as_hal::<Dx12>() } {
            queue.remove_signal_fence(&self.fence);
        }
    }
}

#[derive(Default)]
struct Counts {
    active: AtomicUsize,
    quarantined: AtomicUsize,
}

fn reserve(counts: &Counts, limit: usize) -> Result<(), Error> {
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
            Ok(_) => return Ok(()),
            Err(actual) => active = actual,
        }
    }
}

struct Permit(Arc<Counts>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::AcqRel);
    }
}

struct Backing {
    texture: Option<wgpu::Texture>,
    // Keep an independent native reference even if wgpu validation/device loss
    // destroys its wrapper. Quarantine must retain the allocation itself.
    _resource: ID3D12Resource,
    texture_handle: OwnedHandle,
    producer: ID3D12Fence,
    producer_handle: OwnedHandle,
    // Native initialization commands precede the wgpu submit on the same queue.
    // Their allocator and descriptors cannot be released while they execute.
    _init_allocator: ID3D12CommandAllocator,
    _init_commands: ID3D12GraphicsCommandList,
    _init_descriptors: ID3D12DescriptorHeap,
    native_device: ID3D12Device,
    device: wgpu::Device,
    _queue: wgpu::Queue,
    permit: Permit,
}

struct Consumer {
    handle: OwnedHandle,
    value: u64,
    fence: Option<ID3D12Fence>,
}

struct Retirement {
    backing: Backing,
    consumers: Vec<Consumer>,
    failed: bool,
}

fn quarantine(retired: Retirement) {
    retired
        .backing
        .permit
        .0
        .quarantined
        .fetch_add(1, Ordering::Relaxed);
    // A release callback, empty fence list, or removed device is not proof of
    // GPU completion. Preserve the allocation AND its budget slot for life.
    std::mem::forget(retired);
}

// Protect pending GPU allocations if encoding, submission, or worker polling
// unwinds. In particular, never free the native initializer on an error path.
struct RetirementGuard(Option<Retirement>);
impl Drop for RetirementGuard {
    fn drop(&mut self) {
        if let Some(retired) = self.0.take() {
            quarantine(retired);
        }
    }
}

fn send(tx: &mpsc::Sender<RetirementGuard>, retired: Retirement) {
    // Channel entries must also be armed: if the worker panics, dropping its
    // receiver must quarantine queued messages rather than free live resources.
    let _ = tx.send(RetirementGuard(Some(retired)));
}

struct Callback {
    retired: Retirement,
    tx: mpsc::Sender<RetirementGuard>,
}

unsafe extern "C" fn released(userdata: *mut c_void, release: *const Release) {
    if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        released_inner(userdata, release)
    })) {
        // Never unwind through C, including a panic payload's destructor. The
        // armed retirement guard quarantines partially processed callbacks.
        std::mem::forget(payload);
    }
}

unsafe fn released_inner(userdata: *mut c_void, release: *const Release) {
    let state = unsafe { Box::from_raw(userdata.cast::<Callback>()) };
    let mut guard = RetirementGuard(Some(state.retired));
    let retired = guard.0.as_mut().unwrap();
    match unsafe { release.as_ref() } {
        None => retired.failed = true,
        Some(release) => {
            retired.failed |= release.status != raw::STATUS_OK
                || (release.fence_count != 0 && release.fences.is_null());
            // Preserve every valid fence even on failed/partially malformed
            // retirement. Native asynchronous work may still reference them.
            if !release.fences.is_null() {
                for i in 0..release.fence_count {
                    let fence = unsafe { &*release.fences.add(i) };
                    if fence.r#type != raw::FENCE_DXGI_HANDLE
                        || fence.handle.is_null()
                        || fence.value == u64::MAX
                    {
                        retired.failed = true;
                        continue;
                    }
                    // Exported handles are borrowed ONLY during this callback.
                    // Defer OpenSharedHandle and all completion checks to worker.
                    let process = unsafe { GetCurrentProcess() };
                    let mut duplicate = HANDLE::default();
                    if unsafe {
                        DuplicateHandle(
                            process,
                            HANDLE(fence.handle),
                            process,
                            &mut duplicate,
                            0,
                            false,
                            DUPLICATE_SAME_ACCESS,
                        )
                    }
                    .is_err()
                    {
                        retired.failed = true;
                        continue;
                    }
                    retired.consumers.push(Consumer {
                        handle: unsafe { OwnedHandle::from_raw_handle(duplicate.0) },
                        value: fence.value,
                        fence: None,
                    });
                }
            }
        }
    }
    let _ = state.tx.send(guard);
}

fn fence_complete(fence: &ID3D12Fence, value: u64) -> Result<bool, ()> {
    let completed = unsafe { fence.GetCompletedValue() };
    // D3D12 reports device removal as UINT64_MAX, not successful completion.
    if completed == u64::MAX {
        Err(())
    } else {
        Ok(completed >= value)
    }
}

fn complete(retired: &mut Retirement) -> Result<bool, ()> {
    if retired.failed || unsafe { retired.backing.native_device.GetDeviceRemovedReason() }.is_err()
    {
        return Err(());
    }
    let mut ready = fence_complete(&retired.backing.producer, 1)?;
    for consumer in &mut retired.consumers {
        if consumer.fence.is_none() {
            unsafe {
                retired
                    .backing
                    .native_device
                    .OpenSharedHandle(HANDLE(consumer.handle.as_raw_handle()), &mut consumer.fence)
            }
            .map_err(|_| ())?;
        }
        let fence = consumer.fence.as_ref().ok_or(())?;
        ready &= fence_complete(fence, consumer.value)?;
    }
    Ok(ready)
}

fn retire(rx: mpsc::Receiver<RetirementGuard>) {
    let mut pending = Vec::<RetirementGuard>::new();
    let mut connected = true;
    while connected || !pending.is_empty() {
        if connected {
            match rx.recv_timeout(Duration::from_millis(2)) {
                Ok(retired) => pending.push(retired),
                Err(mpsc::RecvTimeoutError::Disconnected) => connected = false,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        } else {
            std::thread::sleep(Duration::from_millis(2));
        }
        let mut i = 0;
        while i < pending.len() {
            let retired = pending[i].0.as_mut().unwrap();
            match complete(retired) {
                Ok(false) => i += 1,
                Err(()) => {
                    // Dropping an armed guard quarantines instead of freeing.
                    drop(pending.swap_remove(i));
                }
                Ok(true) => {
                    // Poll only off-thread, after native completion. Release the
                    // wgpu wrapper and collect deferred destruction before its
                    // capacity permit is returned. The native reference stays
                    // pinned if polling reports an error or panics.
                    drop(retired.backing.texture.take());
                    if retired.backing.device.poll(wgpu::PollType::Poll).is_err() {
                        drop(pending.swap_remove(i));
                    } else {
                        let mut guard = pending.swap_remove(i);
                        drop(guard.0.take());
                    }
                }
            }
        }
    }
}

/// A DX12 producer device/queue and a bounded budget of fresh shared allocations.
///
/// `new` creates a private pair; unsafe `from_device` attaches the application's
/// existing pair under an external queue-serialization contract. The queue is
/// never exposed here: raw initialization, HAL fence staging, and submission
/// must remain atomic with respect to all other users of that queue.
/// Elegance must use the same physical adapter; native publication validates import.
pub struct Bridge {
    device: wgpu::Device,
    queue: wgpu::Queue,
    image: GpuImage,
    config: Config,
    counts: Arc<Counts>,
    tx: mpsc::Sender<RetirementGuard>,
}

impl Bridge {
    /// Request a dedicated DX12 device on the adapter selected by the app.
    pub async fn new(adapter: &wgpu::Adapter, config: Config) -> Result<Self, Error> {
        if adapter.get_info().backend != wgpu::Backend::Dx12 {
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
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|error| Error::Device(error.to_string()))?;
        // This freshly requested pair has no other queue users.
        unsafe { Self::from_device(device, queue, config) }
    }

    /// Attach an existing wgpu DX12 device/queue without creating another logical
    /// device. Existing textures from this device work with `copy_from`.
    ///
    /// # Safety
    /// - `device` and `queue` must be the pair returned by the SAME request_device
    ///   call, not merely devices on the same physical GPU.
    /// - Serialize EVERY bridge production call with all queue submissions,
    ///   write_buffer/write_texture operations, and native/HAL queue operations
    ///   through every alias. Hold the application's queue lock for the ENTIRE
    ///   copy_from/clear_color/render call, including allocation and publication.
    ///   Between calls, normal application submissions are allowed. Do not submit
    ///   reentrantly from device/queue callbacks during these calls.
    /// - Do not enter a production call with staged HAL waits/signals outstanding;
    ///   drain them first. Do not destroy the device while the bridge is live.
    /// - Source writes must have been submitted before copy_from on this queue,
    ///   or explicitly synchronized from other queues. Elegance must use the same
    ///   physical GPU. Never write a bridge destination after publication.
    ///
    /// Moving cloneable handles does not prove exclusivity. The constructor
    /// checks DX12 compatibility and native device identity, not compliance with
    /// the queue-serialization contract. Retirement remains asynchronous and may
    /// outlive the bridge; its worker polls the device and native fences.
    pub unsafe fn from_device(
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: Config,
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
        {
            let native_device = unsafe { device.as_hal::<Dx12>() }.ok_or(Error::Unsupported)?;
            let native_queue = unsafe { queue.as_hal::<Dx12>() }.ok_or(Error::Unsupported)?;
            let mut queue_device = None::<ID3D12Device>;
            unsafe { native_queue.as_raw().GetDevice(&mut queue_device) }
                .map_err(|error| Error::Device(error.to_string()))?;
            let queue_device = queue_device.ok_or(Error::InvalidConfig)?;
            // Compare canonical COM identities, not physical adapters or an
            // interface pointer that could be a different COM tear-off.
            let expected = native_device
                .raw_device()
                .cast::<windows::core::IUnknown>()
                .map_err(|error| Error::Device(error.to_string()))?;
            let actual = queue_device
                .cast::<windows::core::IUnknown>()
                .map_err(|error| Error::Device(error.to_string()))?;
            if expected.as_raw() != actual.as_raw() {
                return Err(Error::InvalidConfig);
            }
        }
        if config.width > device.limits().max_texture_dimension_2d
            || config.height > device.limits().max_texture_dimension_2d
        {
            return Err(Error::InvalidConfig);
        }
        let image = GpuImage::new(config.width, config.height, config.origin)
            .map_err(|error| Error::Device(error.to_string()))?;
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("elegance-wgpu-dx12-retire".into())
            .spawn(move || retire(rx))
            .map_err(|error| Error::Device(error.to_string()))?;
        Ok(Self {
            device,
            queue,
            image,
            config,
            counts: Arc::default(),
            tx,
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

    /// Copy mip zero from a same-device, same-size/format, single-sampled
    /// COPY_SRC texture. Validation errors return `Error::Device` without
    /// publishing. Native initialization retires asynchronously on rejection.
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

    /// Publish a GPU-cleared UNORM image. RGB values are stored as supplied
    /// (sRGB-encoded); alpha is straight, not premultiplied.
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

    /// Encode writes into a fresh shared image, submit once, then publish.
    ///
    /// # Safety
    /// No destination texture, view, native handle, bind group, or command
    /// referencing it may escape the closure. Do not destroy the device or
    /// destination, submit through another queue, or alter native resource
    /// states outside the encoder's tracking. Initialize the entire image with
    /// straight alpha and sRGB-encoded RGB. No writes may follow publication.
    pub unsafe fn render_raw(
        &mut self,
        encode: impl FnOnce(&wgpu::Device, &mut wgpu::CommandEncoder, &wgpu::Texture),
    ) -> Result<(), Error> {
        let errors = NativeErrors::new(&self.device);
        let mut guard = self.allocate()?;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let retired = guard.0.as_ref().unwrap();
        let texture = retired.backing.texture.as_ref().unwrap();
        encode(&self.device, &mut encoder, texture);
        // PRESENT maps to COMMON on DX12. Unlike relying only on automatic
        // simultaneous-access decay, this also updates wgpu's state tracker.
        encoder.transition_resources(
            std::iter::empty(),
            std::iter::once(wgpu::TextureTransition {
                texture,
                selector: None,
                state: wgpu::TextureUses::PRESENT,
            }),
        );
        let commands = encoder.finish();
        if let Err(error) = errors.finish() {
            drop(commands);
            // Unlike Metal, allocate() has already submitted native clear
            // commands. Signal their completion on the actual queue and retire
            // off-thread; never free the backing just because encoding failed.
            self.retire_unpublished(guard);
            return Err(error);
        }
        let errors = NativeErrors::new(&self.device);
        // Arm before staging so unwinding cannot leave a signal for an unrelated
        // later submit. It is not enough that Queue::submit returned an index:
        // validation can reject the commands without ever entering HAL submit.
        let staged = StagedSignal {
            queue: &self.queue,
            fence: retired.backing.producer.clone(),
        };
        let queue = unsafe { self.queue.as_hal::<Dx12>() }.ok_or(Error::Unsupported)?;
        queue.add_signal_fence(retired.backing.producer.clone(), 1);
        drop(queue);
        self.queue.submit([commands]);
        let signal = staged.finish();
        let submitted = errors.finish();
        // Any submission error leaves completion uncertain. The armed backing
        // quarantines, and signal cleanup has already removed unconsumed state.
        submitted?;
        signal?;

        let retired = guard.0.take().unwrap();
        let frame = Frame {
            transport: raw::TRANSPORT_DXGI_HANDLE,
            format: if self.config.format == wgpu::TextureFormat::Bgra8Unorm {
                raw::FORMAT_BGRA8
            } else {
                raw::FORMAT_RGBA8
            },
            width: self.config.width,
            height: self.config.height,
            handle: retired.backing.texture_handle.as_raw_handle(),
            memory_fd: -1,
            vk_image_create_info: std::ptr::null(),
            vk_memory_type_index: 0,
            vk_allocation_size: 0,
            vk_dedicated_allocation: 0,
            vk_old_layout: 0,
            vk_new_layout: 0,
            producer_fence: Fence {
                r#type: raw::FENCE_DXGI_HANDLE,
                handle: retired.backing.producer_handle.as_raw_handle(),
                fd: -1,
                value: 1,
            },
            token: 0,
            userdata: Box::into_raw(Box::new(Callback {
                retired,
                tx: self.tx.clone(),
            }))
            .cast(),
            release: Some(released),
        };
        let status = unsafe { raw::elg_gpu_image_publish(self.image.as_raw(), &frame) };
        if status != raw::STATUS_OK {
            // Rejected import promises no callback/consumer reads. The producer
            // submission still needs its allocation until its fence completes.
            let state = unsafe { Box::from_raw(frame.userdata.cast::<Callback>()) };
            send(&state.tx, state.retired);
            return Err(Error::Publish(status));
        }
        Ok(())
    }

    fn retire_unpublished(&self, guard: RetirementGuard) {
        let retired = guard.0.as_ref().unwrap();
        let Some(queue) = (unsafe { self.queue.as_hal::<Dx12>() }) else {
            return; // guard quarantines the already submitted initializer
        };
        // No wgpu destination commands were submitted, nor any native consumer
        // given access. FIFO orders this GPU signal after raw initialization.
        // Failure keeps the guard armed; no CPU signal/wait can substitute.
        if unsafe { queue.as_raw().Signal(&retired.backing.producer, 1) }.is_err() {
            return;
        }
        drop(queue);
        let _ = self.tx.send(guard);
    }

    fn extent(&self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.config.width,
            height: self.config.height,
            depth_or_array_layers: 1,
        }
    }

    fn allocate(&self) -> Result<RetirementGuard, Error> {
        reserve(&self.counts, self.config.max_frames)?;
        let permit = Permit(self.counts.clone());
        let device = unsafe { self.device.as_hal::<Dx12>() }.ok_or(Error::Unsupported)?;
        let native = device.raw_device();
        // Dawn's DXGI importer derives the WebGPU format from GetDesc and
        // rejects TYPELESS. The native bridge supports D3D12, not D3D11.
        let format = if self.config.format == wgpu::TextureFormat::Bgra8Unorm {
            DXGI_FORMAT_B8G8R8A8_UNORM
        } else {
            DXGI_FORMAT_R8G8B8A8_UNORM
        };
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Width: self.config.width.into(),
            Height: self.config.height,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET
                | D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS,
            ..Default::default()
        };
        let mut resource = None;
        unsafe {
            native.CreateCommittedResource(
                &D3D12_HEAP_PROPERTIES {
                    Type: D3D12_HEAP_TYPE_DEFAULT,
                    CreationNodeMask: 1,
                    VisibleNodeMask: 1,
                    ..Default::default()
                },
                D3D12_HEAP_FLAG_SHARED,
                &desc,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
                None,
                &mut resource,
            )
        }
        .map_err(|_| Error::Allocation)?;
        let resource: ID3D12Resource = resource.ok_or(Error::Allocation)?;
        let producer: ID3D12Fence = unsafe { native.CreateFence(0, D3D12_FENCE_FLAG_SHARED) }
            .map_err(|_| Error::Allocation)?;
        let texture_handle = shared_handle(native, &resource)?;
        let producer_handle = shared_handle(native, &producer)?;
        let allocator: ID3D12CommandAllocator =
            unsafe { native.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }
                .map_err(|_| Error::Allocation)?;
        let commands: ID3D12GraphicsCommandList = unsafe {
            native.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None)
        }
        .map_err(|_| Error::Allocation)?;
        let descriptors: ID3D12DescriptorHeap = unsafe {
            native.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                NumDescriptors: 1,
                ..Default::default()
            })
        }
        .map_err(|_| Error::Allocation)?;
        let rtv = unsafe { descriptors.GetCPUDescriptorHandleForHeapStart() };
        unsafe {
            native.CreateRenderTargetView(
                &resource,
                Some(&D3D12_RENDER_TARGET_VIEW_DESC {
                    Format: if self.config.format == wgpu::TextureFormat::Bgra8Unorm {
                        DXGI_FORMAT_B8G8R8A8_UNORM
                    } else {
                        DXGI_FORMAT_R8G8B8A8_UNORM
                    },
                    ViewDimension: D3D12_RTV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D12_RENDER_TARGET_VIEW_DESC_0 {
                        Texture2D: D3D12_TEX2D_RTV::default(),
                    },
                }),
                rtv,
            );
            commands.ClearRenderTargetView(rtv, &[0.0; 4], None);
        }
        let mut barrier = D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
            Anonymous: D3D12_RESOURCE_BARRIER_0 {
                Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                    pResource: ManuallyDrop::new(Some(resource.clone())),
                    Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                    StateBefore: D3D12_RESOURCE_STATE_RENDER_TARGET,
                    StateAfter: D3D12_RESOURCE_STATE_COMMON,
                }),
            },
        };
        unsafe {
            commands.ResourceBarrier(std::slice::from_ref(&barrier));
            ManuallyDrop::drop(&mut (*barrier.Anonymous.Transition).pResource);
            commands.Close()
        }
        .map_err(|_| Error::Allocation)?;
        let list: ID3D12CommandList = commands.cast().map_err(|_| Error::Allocation)?;
        let mut guard = RetirementGuard(Some(Retirement {
            backing: Backing {
                texture: None,
                _resource: resource.clone(),
                texture_handle,
                producer,
                producer_handle,
                _init_allocator: allocator,
                _init_commands: commands,
                _init_descriptors: descriptors,
                native_device: native.clone(),
                device: self.device.clone(),
                _queue: self.queue.clone(),
                permit,
            },
            consumers: Vec::new(),
            failed: false,
        }));
        let queue = unsafe { self.queue.as_hal::<Dx12>() }.ok_or(Error::Unsupported)?;
        // This is a fresh allocation: there is no previous consumer to wait on.
        // FIFO on the same direct queue orders initialization and its COMMON
        // barrier before the later wgpu submit; no CPU wait is needed. For an
        // attached queue, from_device requires external serialization across
        // this entire production call. The final producer fence covers BOTH
        // native and wgpu lists; their backing stays guarded until completion.
        unsafe { queue.as_raw().ExecuteCommandLists(&[Some(list)]) };
        drop(queue);
        let texture = unsafe {
            hal::dx12::Device::texture_from_raw(
                resource,
                self.config.format,
                wgpu::TextureDimension::D2,
                self.extent(),
                1,
                1,
            )
        };
        drop(device);
        let texture = unsafe {
            self.device.create_texture_from_hal::<Dx12>(
                texture,
                &wgpu::TextureDescriptor {
                    label: Some("Elegance DXGI shared image"),
                    size: self.extent(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.config.format,
                    usage: wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[self.config.format.add_srgb_suffix()],
                },
                wgpu::TextureUses::PRESENT,
            )
        };
        guard.0.as_mut().unwrap().backing.texture = Some(texture);
        Ok(guard)
    }
}

fn shared_handle(device: &ID3D12Device, object: &ID3D12DeviceChild) -> Result<OwnedHandle, Error> {
    // GENERIC_ALL is required by CreateSharedHandle for D3D12 shared objects.
    let handle = unsafe { device.CreateSharedHandle(object, None, 0x1000_0000, PCWSTR::null()) }
        .map_err(|_| Error::Allocation)?;
    Ok(unsafe { OwnedHandle::from_raw_handle(handle.0) })
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
    use crate::block_on;

    #[test]
    fn budget_is_bounded_and_returned_on_completion() {
        let counts = Arc::new(Counts::default());
        reserve(&counts, 1).unwrap();
        let permit = Permit(counts.clone());
        assert!(matches!(reserve(&counts, 1), Err(Error::Busy)));
        drop(permit);
        reserve(&counts, 1).unwrap();
        drop(Permit(counts.clone()));
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
                        if reserve(&counts, 3).is_ok() {
                            let permit = Permit(counts.clone());
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

    fn test_bridge() -> Bridge {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        // This test owns the pair and has no other submitting aliases.
        unsafe {
            Bridge::from_device(
                device,
                queue,
                Config {
                    width: 16,
                    height: 16,
                    origin: crate::GpuImageOrigin::TopLeft,
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    max_frames: 1,
                },
            )
        }
        .unwrap()
    }

    fn wait_until(mut predicate: impl FnMut() -> bool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !predicate() {
            assert!(
                std::time::Instant::now() < deadline,
                "fence retirement timed out"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    #[ignore = "requires Windows DX12; synthetic native retirement records"]
    fn callback_duplicates_handles_and_waits_every_consumer() {
        let bridge = test_bridge();
        let mut guard = bridge.allocate().unwrap();
        let backing = &guard.0.as_ref().unwrap().backing;
        let first: ID3D12Fence = unsafe {
            backing
                .native_device
                .CreateFence(0, D3D12_FENCE_FLAG_SHARED)
        }
        .unwrap();
        let second: ID3D12Fence = unsafe {
            backing
                .native_device
                .CreateFence(0, D3D12_FENCE_FLAG_SHARED)
        }
        .unwrap();
        let first_handle = shared_handle(&backing.native_device, &first).unwrap();
        let second_handle = shared_handle(&backing.native_device, &second).unwrap();
        // This fence covers actual native initialization; never CPU-signal the
        // producer fence and pretend that its GPU commands have finished.
        let queue = unsafe { bridge.queue.as_hal::<Dx12>() }.unwrap();
        unsafe { queue.as_raw().Signal(&backing.producer, 1) }.unwrap();
        drop(queue);
        let fences = [
            Fence {
                r#type: raw::FENCE_DXGI_HANDLE,
                handle: first_handle.as_raw_handle(),
                fd: -1,
                value: 2,
            },
            Fence {
                r#type: raw::FENCE_DXGI_HANDLE,
                handle: second_handle.as_raw_handle(),
                fd: -1,
                value: 4,
            },
        ];
        let (tx, rx) = mpsc::channel();
        let callback = Box::into_raw(Box::new(Callback {
            retired: guard.0.take().unwrap(),
            tx,
        }));
        unsafe {
            released(
                callback.cast(),
                &Release {
                    status: raw::STATUS_OK,
                    token: 0,
                    fences: fences.as_ptr(),
                    fence_count: fences.len(),
                    vk_old_layout: 0,
                    vk_new_layout: 0,
                },
            );
        }
        // Only callback-created duplicates remain valid for OpenSharedHandle.
        drop((first_handle, second_handle));
        let mut guard = rx.recv().unwrap();
        let retired = guard.0.as_mut().unwrap();
        wait_until(|| fence_complete(&retired.backing.producer, 1).unwrap());
        assert!(!complete(retired).unwrap());
        unsafe { first.Signal(2) }.unwrap();
        assert!(!complete(retired).unwrap());
        assert_eq!(bridge.in_flight(), 1);
        assert!(matches!(bridge.allocate(), Err(Error::Busy)));
        unsafe { second.Signal(4) }.unwrap();
        assert!(complete(retired).unwrap());
        assert!(bridge.tx.send(guard).is_ok());
        wait_until(|| bridge.in_flight() == 0);
        assert_eq!(bridge.quarantined(), 0);
    }

    #[test]
    #[ignore = "requires Windows DX12 and the native Elegance library"]
    fn attached_device_copies_existing_renderer_texture() {
        let mut bridge = test_bridge();
        let source = bridge.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("existing renderer texture"),
            size: bridge.extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: bridge.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let mut encoder = bridge.device.create_command_encoder(&Default::default());
        let view = source.create_view(&Default::default());
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        // Model application submissions outside the serialized bridge call.
        let external_queue = bridge.queue.clone();
        external_queue.submit([encoder.finish()]);
        let result = bridge.copy_from(&source);
        // Standalone tests have no renderer. Rejected native publication must
        // still retire the copy only after the actual producer fence completes.
        assert!(result.is_ok() || matches!(result, Err(Error::Publish(_))));
        bridge.clear();
        external_queue.submit([]);
        wait_until(|| bridge.in_flight() == 0);
        assert_eq!(bridge.quarantined(), 0);
    }

    #[test]
    #[ignore = "requires Windows DX12 and the native Elegance library"]
    fn wrong_device_copy_never_publishes_or_leaves_staged_signals() {
        let mut bridge = test_bridge();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        let (foreign_device, _foreign_queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let source = foreign_device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wrong-device source with otherwise valid metadata"),
            size: bridge.extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: bridge.config.format,
            usage: wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        // An outer scope makes errors non-panicking even without our internal
        // scopes. Error::Device (not Publish or Ok) proves publication was never
        // attempted; repeatedly recovering the one slot covers raw init lifetime.
        let outer = NativeErrors::new(&bridge.device);
        for _ in 0..3 {
            assert!(matches!(bridge.copy_from(&source), Err(Error::Device(_))));
            wait_until(|| bridge.in_flight() == 0);
            assert_eq!(bridge.quarantined(), 0);
        }

        // Exercise rejection after staging as well: deliberately submit an
        // invalid command buffer whose finish error we have already captured.
        let errors = NativeErrors::new(&bridge.device);
        let target = bridge.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rejected submit target"),
            size: bridge.extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: bridge.config.format,
            usage: wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut encoder = bridge.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(
            source.as_image_copy(),
            target.as_image_copy(),
            bridge.extent(),
        );
        let invalid = encoder.finish();
        assert!(matches!(errors.finish(), Err(Error::Device(_))));
        let native = unsafe { bridge.device.as_hal::<Dx12>() }.unwrap();
        let stale: ID3D12Fence =
            unsafe { native.raw_device().CreateFence(0, D3D12_FENCE_FLAG_NONE) }.unwrap();
        let marker: ID3D12Fence =
            unsafe { native.raw_device().CreateFence(0, D3D12_FENCE_FLAG_NONE) }.unwrap();
        drop(native);
        let errors = NativeErrors::new(&bridge.device);
        let staged = StagedSignal {
            queue: &bridge.queue,
            fence: stale.clone(),
        };
        let hal_queue = unsafe { bridge.queue.as_hal::<Dx12>() }.unwrap();
        hal_queue.add_signal_fence(stale.clone(), 1);
        drop(hal_queue);
        bridge.queue.submit([invalid]);
        assert!(matches!(staged.finish(), Err(Error::Device(_))));
        assert!(matches!(errors.finish(), Err(Error::Device(_))));

        // The same cleanup must happen if submission/encoding unwinds.
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _staged = StagedSignal {
                queue: &bridge.queue,
                fence: stale.clone(),
            };
            let hal_queue = unsafe { bridge.queue.as_hal::<Dx12>() }.unwrap();
            hal_queue.add_signal_fence(stale.clone(), 1);
            drop(hal_queue);
            panic!("synthetic staged-submit panic");
        }));
        assert!(panic.is_err());
        let hal_queue = unsafe { bridge.queue.as_hal::<Dx12>() }.unwrap();
        assert!(!hal_queue.remove_signal_fence(&stale));
        drop(hal_queue);

        // A later valid producer submit must not signal the rejected fence or
        // be affected by the previous error scopes. Native import can explicitly
        // reject this standalone test when no Elegance renderer is running.
        let result = bridge.clear_color(wgpu::Color::RED);
        assert!(result.is_ok() || matches!(result, Err(Error::Publish(_))));
        let hal_queue = unsafe { bridge.queue.as_hal::<Dx12>() }.unwrap();
        unsafe { hal_queue.as_raw().Signal(&marker, 1) }.unwrap();
        drop(hal_queue);
        wait_until(|| fence_complete(&marker, 1).unwrap());
        assert_eq!(unsafe { stale.GetCompletedValue() }, 0);
        bridge.clear();
        wait_until(|| bridge.in_flight() == 0);
        assert_eq!(bridge.quarantined(), 0);
        outer.finish().unwrap();
    }

    #[test]
    #[ignore = "requires Windows DX12; intentionally quarantines one allocation"]
    fn failed_release_keeps_allocation_and_capacity() {
        let bridge = test_bridge();
        let mut guard = bridge.allocate().unwrap();
        let backing = &guard.0.as_ref().unwrap().backing;
        let handle = shared_handle(&backing.native_device, &backing.producer).unwrap();
        let valid = Fence {
            r#type: raw::FENCE_DXGI_HANDLE,
            handle: handle.as_raw_handle(),
            fd: -1,
            value: 1,
        };
        let fences = [
            valid,
            Fence {
                handle: std::ptr::null_mut(),
                ..valid
            },
            valid,
        ];
        let (tx, rx) = mpsc::channel();
        let callback = Box::into_raw(Box::new(Callback {
            retired: guard.0.take().unwrap(),
            tx,
        }));
        unsafe {
            released(
                callback.cast(),
                &Release {
                    status: raw::STATUS_SYNC_FAILED,
                    token: 0,
                    fences: fences.as_ptr(),
                    fence_count: fences.len(),
                    vk_old_layout: 0,
                    vk_new_layout: 0,
                },
            );
        }
        drop(handle);
        let guard = rx.recv().unwrap();
        let retired = guard.0.as_ref().unwrap();
        assert!(retired.failed);
        // Retain valid handles before AND after a malformed entry, even when
        // native retirement already reports failure.
        assert_eq!(retired.consumers.len(), 2);
        // Even unsignaled GPU work must survive the worker's rejection path.
        assert!(bridge.tx.send(guard).is_ok());
        wait_until(|| bridge.quarantined() == 1);
        assert_eq!(bridge.in_flight(), 1);
        assert!(matches!(bridge.allocate(), Err(Error::Busy)));
    }
}
