use crate::{Config, Error, ExternalGpuImage};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained};
use objc2_io_surface::{
    kIOSurfaceBytesPerElement, kIOSurfaceHeight, kIOSurfacePixelFormat, kIOSurfaceWidth,
    IOSurfaceRef,
};
use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandBufferStatus, MTLCommandEncoder, MTLCommandQueue,
    MTLDevice, MTLLoadAction, MTLPixelFormat, MTLRenderPassDescriptor, MTLSharedEvent,
    MTLStorageMode, MTLStoreAction, MTLTextureDescriptor, MTLTextureType, MTLTextureUsage,
};
use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
use wgpu::hal::{self, api::Metal};

use elegance_sys::external_gpu_image::{self as raw, Fence, Frame, Release};

// wgpu-core on this pinned native backend returns ready error-scope futures.
// Do not block or silently treat Pending as success if that implementation changes.
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
                Poll::Ready(Some(e)) => Some(e.to_string()),
                Poll::Pending => Some("native wgpu error scope unexpectedly pending".into()),
            };
            if error.is_none() {
                error = result;
            }
        }
        error.map_or(Ok(()), |e| Err(Error::Device(e)))
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
    event: Event,
}
impl Drop for StagedSignal<'_> {
    fn drop(&mut self) {
        if let Some(queue) = unsafe { self.queue.as_hal::<Metal>() } {
            queue.remove_signal_event(&self.event);
        }
    }
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

type Event = Retained<ProtocolObject<dyn MTLSharedEvent>>;
#[derive(Default)]
struct Counts {
    active: AtomicUsize,
    quarantined: AtomicUsize,
}
struct Permit(Arc<Counts>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Backing {
    _texture: wgpu::Texture,
    surface: CFRetained<IOSurfaceRef>,
    producer: Event,
    _permit: Permit,
}
// IOSurface is thread-safe; Metal shared events and wgpu resources are Send/Sync.
unsafe impl Send for Backing {}
struct Retirement {
    backing: Option<Backing>,
    consumers: Vec<(Event, u64)>,
    failed: bool,
}
impl Retirement {
    fn pending(backing: Backing) -> Self {
        Self {
            backing: Some(backing),
            consumers: Vec::new(),
            failed: false,
        }
    }
    fn ready(&self) -> bool {
        let backing = self.backing.as_ref().unwrap();
        backing.producer.signaledValue() >= 1
            && self
                .consumers
                .iter()
                .all(|(event, value)| event.signaledValue() >= *value)
    }
    fn complete(&mut self) {
        drop(self.backing.take());
    }
}
impl Drop for Retirement {
    fn drop(&mut self) {
        if let Some(backing) = self.backing.take() {
            backing._permit.0.quarantined.fetch_add(1, Ordering::AcqRel);
            // Also covers worker unwinding and records drained by Receiver::drop.
            // Only complete() is allowed to return the budget permit.
            std::mem::forget(backing);
            std::mem::forget(std::mem::take(&mut self.consumers));
        }
    }
}
struct EncodingGuard(Option<Backing>);
impl Drop for EncodingGuard {
    fn drop(&mut self) {
        if let Some(backing) = self.0.take() {
            quarantine(Retirement::pending(backing));
        }
    }
}
struct Callback {
    backing: Backing,
    tx: mpsc::Sender<Retirement>,
}

fn quarantine(retired: Retirement) {
    drop(retired);
}
fn send(tx: &mpsc::Sender<Retirement>, retired: Retirement) {
    if let Err(e) = tx.send(retired) {
        quarantine(e.0);
    }
}
unsafe extern "C" fn released(userdata: *mut c_void, release: *const Release) {
    if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        released_inner(userdata, release)
    })) {
        // Never unwind through the C ABI, even if a panic payload has a panicking
        // destructor. Retirement's Drop quarantines any partially built record.
        std::mem::forget(payload);
    }
}
unsafe fn released_inner(userdata: *mut c_void, release: *const Release) {
    let state = unsafe { Box::from_raw(userdata.cast::<Callback>()) };
    let mut retired = Retirement::pending(state.backing);
    retired.failed = release.is_null();
    if let Some(release) = unsafe { release.as_ref() } {
        retired.failed |= release.status != raw::STATUS_OK
            || (release.fence_count != 0 && release.fences.is_null());
        // Retain valid returned events even on failed retirement: quarantined
        // GPU work may still reference them. Do not skip the rest of the list
        // merely because one export was malformed.
        if release.fence_count == 0 || !release.fences.is_null() {
            for i in 0..release.fence_count {
                let fence = unsafe { &*release.fences.add(i) };
                if fence.r#type != raw::FENCE_MTL_SHARED_EVENT || fence.handle.is_null() {
                    retired.failed = true;
                    continue;
                }
                // The native event is borrowed only for this callback.
                let event = unsafe {
                    Retained::retain(fence.handle.cast::<ProtocolObject<dyn MTLSharedEvent>>())
                };
                if let Some(event) = event {
                    retired.consumers.push((event, fence.value));
                } else {
                    retired.failed = true;
                }
            }
        }
    }
    send(&state.tx, retired);
}

/// A producer device/queue and a fixed-size publication budget.
///
/// `new` creates a private queue; unsafe `from_device` attaches the application's
/// existing pair under an external queue-serialization contract. The queue is
/// never exposed here: HAL event staging and submission must remain atomic with
/// respect to all other users of that queue.
pub struct Bridge {
    device: wgpu::Device,
    queue: wgpu::Queue,
    image: ExternalGpuImage,
    config: Config,
    counts: Arc<Counts>,
    tx: mpsc::Sender<Retirement>,
}
impl Bridge {
    /// Request a dedicated Metal device on the adapter selected by the app.
    /// Elegance must use the same physical GPU; publication validates import.
    pub async fn new(adapter: &wgpu::Adapter, config: Config) -> Result<Self, Error> {
        if adapter.get_info().backend != wgpu::Backend::Metal {
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
            .map_err(|e| Error::Device(e.to_string()))?;
        // This freshly requested pair has no other queue users.
        unsafe { Self::from_device(device, queue, config) }
    }

    /// Attach an existing wgpu Metal device/queue, without creating a second
    /// logical device. Existing textures from this device work with copy_from.
    ///
    /// # Safety
    /// - `device` and `queue` must be the pair returned by the SAME request_device
    ///   call (not merely two devices on the same physical GPU).
    /// - Serialize EVERY bridge production call with all queue submissions,
    ///   write_buffer/write_texture operations, and native/HAL queue operations
    ///   through every alias. Hold the application's queue lock for the ENTIRE
    ///   copy_from/clear_color/render call, including allocation and publication.
    ///   Between bridge calls, normal application submissions are allowed. Do
    ///   not submit reentrantly from device/queue callbacks during these calls.
    /// - Do not enter a production call with staged HAL waits/signals outstanding;
    ///   drain them first. Do not destroy the device while the bridge is live.
    /// - Source writes must have been submitted before copy_from, on this queue,
    ///   or explicitly synchronized from other queues. Elegance must use the same
    ///   physical GPU. Never write the bridge's published destination afterwards.
    ///
    /// Moving the handles does not prove exclusivity: wgpu handles are cloneable.
    /// The constructor checks Metal compatibility, not logical device identity or
    /// compliance with the serialization contract. Other backend implementations
    /// have additional platform-specific creation requirements.
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
            let metal_device = unsafe { device.as_hal::<Metal>() }.ok_or(Error::Unsupported)?;
            let metal_queue = unsafe { queue.as_hal::<Metal>() }.ok_or(Error::Unsupported)?;
            if !std::ptr::eq(
                &**metal_device.raw_device(),
                &*metal_queue.as_raw().device(),
            ) {
                return Err(Error::InvalidConfig);
            }
        }
        if config.width > device.limits().max_texture_dimension_2d
            || config.height > device.limits().max_texture_dimension_2d
        {
            return Err(Error::InvalidConfig);
        }
        let image = ExternalGpuImage::new(config.width, config.height, config.origin)
            .map_err(|e| Error::Device(e.to_string()))?;
        let (tx, rx) = mpsc::channel::<Retirement>();
        std::thread::Builder::new()
            .name("elegance-wgpu-retire".into())
            .spawn(move || {
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
                    pending.retain_mut(|r| {
                        if r.ready() {
                            r.complete();
                            false
                        } else {
                            true
                        }
                    });
                    if !connected && !pending.is_empty() {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                }
            })
            .map_err(|e| Error::Device(e.to_string()))?;
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
    pub fn image(&self) -> &ExternalGpuImage {
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

    /// Copy mip zero of a same-device, same-format, same-size, single-sampled
    /// COPY_SRC texture into a fresh IOSurface, submit, then publish. No CPU copy.
    /// wgpu validation errors are returned as Error::Device before publication.
    /// Pre-submit validation failures return their budget permit immediately.
    pub fn copy_from(&mut self, source: &wgpu::Texture) -> Result<(), Error> {
        if source.size() != self.extent()
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
                )
            })
        }
    }

    /// Initialize and publish a solid color using a GPU render pass.
    /// RGB values are stored as supplied in the UNORM texture (sRGB-encoded).
    pub fn clear_color(&mut self, color: wgpu::Color) -> Result<(), Error> {
        unsafe {
            self.render_raw(|_, encoder, texture| {
                let view = texture.create_view(&Default::default());
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

    /// Encode all writes into the provided encoder, submit once, then publish.
    ///
    /// # Safety
    /// Do not retain/clone the destination texture, any of its views, native
    /// handles, bind groups, or commands referencing it beyond the closure.
    /// Do not submit writes through another queue. Do not destroy the device or
    /// destination. Fully initialize the image; use straight alpha and sRGB RGB
    /// values. Any escaped handle or later write can race native consumer reads.
    pub unsafe fn render_raw(
        &mut self,
        encode: impl FnOnce(&wgpu::Device, &mut wgpu::CommandEncoder, &wgpu::Texture),
    ) -> Result<(), Error> {
        let errors = NativeErrors::new(&self.device);
        let backing = self.allocate()?;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // A panic during encoding leaves completion unknown. Conservatively pin
        // the backing rather than releasing it through unwinding.
        let mut guard = EncodingGuard(Some(backing));
        let backing = guard.0.as_ref().unwrap();
        encode(&self.device, &mut encoder, &backing._texture);
        let commands = encoder.finish();
        if let Err(error) = errors.finish() {
            // No GPU commands referencing the destination were submitted. In
            // particular, a wrong-device source must not strand capacity or
            // publish a frame with a producer event that will never signal.
            drop(guard.0.take());
            return Err(error);
        }
        let errors = NativeErrors::new(&self.device);
        let staged = StagedSignal {
            queue: &self.queue,
            event: backing.producer.clone(),
        };
        let queue = unsafe { self.queue.as_hal::<Metal>() }.ok_or(Error::Unsupported)?;
        queue.add_signal_event(backing.producer.clone(), 1);
        drop(queue);
        self.queue.submit([commands]);
        drop(staged);
        // Submit errors may leave completion unknown. EncodingGuard quarantines
        // the allocation; StagedSignal has removed any unconsumed signal.
        errors.finish()?;
        let backing = guard.0.take().unwrap();
        let frame = Frame {
            transport: raw::TRANSPORT_IOSURFACE,
            format: if self.config.format == wgpu::TextureFormat::Bgra8Unorm {
                raw::FORMAT_BGRA8
            } else {
                raw::FORMAT_RGBA8
            },
            width: self.config.width,
            height: self.config.height,
            handle: (&*backing.surface as *const IOSurfaceRef).cast_mut().cast(),
            memory_fd: -1,
            vk_image_create_info: std::ptr::null(),
            vk_memory_type_index: 0,
            vk_allocation_size: 0,
            vk_dedicated_allocation: 0,
            vk_old_layout: 0,
            vk_new_layout: 0,
            producer_fence: Fence {
                r#type: raw::FENCE_MTL_SHARED_EVENT,
                handle: Retained::as_ptr(&backing.producer).cast_mut().cast(),
                fd: -1,
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
        let status = unsafe { raw::elg_external_gpu_image_publish(self.image.as_raw(), &frame) };
        if status != raw::STATUS_OK {
            // Rejection promises no callback/no consumer reads, but producer
            // work is still in flight and must keep its backing alive.
            let state = unsafe { Box::from_raw(frame.userdata.cast::<Callback>()) };
            send(&state.tx, Retirement::pending(state.backing));
            return Err(Error::Publish(status));
        }
        Ok(())
    }
    fn extent(&self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.config.width,
            height: self.config.height,
            depth_or_array_layers: 1,
        }
    }
    fn allocate(&self) -> Result<Backing, Error> {
        reserve(&self.counts, self.config.max_frames)?;
        let permit = Permit(self.counts.clone());
        let bgra = self.config.format == wgpu::TextureFormat::Bgra8Unorm;
        let values = [
            CFNumber::new_i64(self.config.width.into()),
            CFNumber::new_i64(self.config.height.into()),
            CFNumber::new_i64(4),
            CFNumber::new_i64(
                if bgra {
                    u32::from_be_bytes(*b"BGRA")
                } else {
                    u32::from_be_bytes(*b"RGBA")
                }
                .into(),
            ),
        ];
        let keys = unsafe {
            [
                kIOSurfaceWidth,
                kIOSurfaceHeight,
                kIOSurfaceBytesPerElement,
                kIOSurfacePixelFormat,
            ]
        };
        let dict = CFDictionary::from_slices(&keys, &values.each_ref().map(|v| &**v));
        let surface = unsafe { IOSurfaceRef::new(dict.as_ref()) }.ok_or(Error::Allocation)?;
        let device = unsafe { self.device.as_hal::<Metal>() }.ok_or(Error::Unsupported)?;
        let raw = device.raw_device();
        let desc = MTLTextureDescriptor::new();
        desc.setTextureType(MTLTextureType::Type2D);
        desc.setPixelFormat(if bgra {
            MTLPixelFormat::BGRA8Unorm
        } else {
            MTLPixelFormat::RGBA8Unorm
        });
        // Dimensions are nonzero and checked against the device's 2D limit.
        unsafe {
            desc.setWidth(self.config.width as usize);
            desc.setHeight(self.config.height as usize);
        }
        desc.setStorageMode(MTLStorageMode::Shared);
        desc.setUsage(
            MTLTextureUsage::RenderTarget
                | MTLTextureUsage::ShaderRead
                | MTLTextureUsage::PixelFormatView,
        );
        let texture = raw
            .newTextureWithDescriptor_iosurface_plane(&desc, &surface, 0)
            .ok_or(Error::Allocation)?;
        let producer = raw.newSharedEvent().ok_or(Error::Allocation)?;
        // Imported HAL textures have no wgpu initialization clear path. Establish
        // initialized storage before wrapping, rather than relying on IOSurface
        // allocator contents or misinterpreting TextureUses::UNINITIALIZED.
        let queue = unsafe { self.queue.as_hal::<Metal>() }.ok_or(Error::Unsupported)?;
        let commands = queue.as_raw().commandBuffer().ok_or(Error::Allocation)?;
        let pass = MTLRenderPassDescriptor::new();
        let attachment = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
        attachment.setTexture(Some(&texture));
        attachment.setLoadAction(MTLLoadAction::Clear);
        attachment.setStoreAction(MTLStoreAction::Store);
        attachment.setClearColor(MTLClearColor {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha: 0.0,
        });
        let clear = commands
            .renderCommandEncoderWithDescriptor(&pass)
            .ok_or(Error::Allocation)?;
        clear.endEncoding();
        commands.commit();
        commands.waitUntilCompleted();
        if commands.status() != MTLCommandBufferStatus::Completed {
            return Err(Error::Allocation);
        }
        drop(queue);
        let hal_texture = unsafe {
            hal::metal::Device::texture_from_raw(
                texture,
                self.config.format,
                MTLTextureType::Type2D,
                1,
                1,
                hal::CopyExtent {
                    width: self.config.width,
                    height: self.config.height,
                    depth: 1,
                },
                None,
            )
        };
        drop(device);
        let texture = unsafe {
            self.device.create_texture_from_hal::<Metal>(
                hal_texture,
                &wgpu::TextureDescriptor {
                    label: Some("Elegance IOSurface"),
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
                wgpu::TextureUses::COLOR_TARGET,
            )
        };
        Ok(Backing {
            _texture: texture,
            surface,
            producer,
            _permit: permit,
        })
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.image.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_is_bounded_and_returned_on_completion() {
        let counts = Arc::new(Counts::default());
        reserve(&counts, 1).unwrap();
        let permit = Permit(counts.clone());
        assert!(matches!(reserve(&counts, 1), Err(Error::Busy)));
        assert_eq!(counts.active.load(Ordering::Acquire), 1);
        drop(permit);
        assert_eq!(counts.active.load(Ordering::Acquire), 0);
        reserve(&counts, 1).unwrap();
        drop(Permit(counts));
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
                            let _permit = Permit(counts.clone());
                            assert!(counts.active.load(Ordering::Acquire) <= 3);
                            std::thread::yield_now();
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

    fn test_config() -> Config {
        Config {
            width: 16,
            height: 16,
            origin: crate::GpuImageOrigin::TopLeft,
            format: wgpu::TextureFormat::Bgra8Unorm,
            max_frames: 4,
        }
    }
    fn wait_for(mut condition: impl FnMut() -> bool) {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !condition() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(condition(), "timed out waiting for GPU retirement");
    }

    #[test]
    #[ignore = "requires a Metal GPU and the native Elegance library"]
    fn resize_preserves_image_backing_and_shared_budget() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        let mut config = test_config();
        config.max_frames = 1;
        let mut bridge = block_on(Bridge::new(&adapter, config)).unwrap();
        let image = bridge.image().clone();
        let counts = bridge.counts.clone();
        let old = bridge.allocate().unwrap();
        let old_size = old._texture.size();

        bridge.resize(32, 48).unwrap();
        bridge.resize(32, 48).unwrap();
        assert_eq!(bridge.image().as_raw(), image.as_raw());
        assert!(Arc::ptr_eq(&bridge.counts, &counts));
        assert_eq!(old._texture.size(), old_size);
        assert_eq!((bridge.extent().width, bridge.extent().height), (32, 48));
        assert_eq!(bridge.in_flight(), 1);
        assert_eq!(bridge.quarantined(), 0);
        assert!(matches!(bridge.allocate(), Err(Error::Busy)));

        let limit = bridge.device().limits().max_texture_dimension_2d;
        for (width, height) in [(0, 48), (32, 0), (limit + 1, 48), (32, limit + 1)] {
            assert!(matches!(
                bridge.resize(width, height),
                Err(Error::InvalidConfig)
            ));
            assert_eq!((bridge.extent().width, bridge.extent().height), (32, 48));
            assert_eq!(bridge.in_flight(), 1);
        }

        // No commands were submitted for these allocations, so dropping is safe.
        drop(old);
        let resized = bridge.allocate().unwrap();
        assert_eq!(
            (resized._texture.width(), resized._texture.height()),
            (32, 48)
        );
        assert_eq!(bridge.in_flight(), 1);
        drop(resized);
        assert_eq!(bridge.in_flight(), 0);

        // Synthetic quarantine: its permit must continue occupying the same budget.
        reserve(&counts, 1).unwrap();
        let permit = Permit(counts.clone());
        counts.quarantined.store(1, Ordering::Release);
        bridge.resize(8, 8).unwrap();
        assert_eq!(bridge.quarantined(), 1);
        assert_eq!(bridge.in_flight(), 1);
        assert!(matches!(bridge.allocate(), Err(Error::Busy)));
        counts.quarantined.store(0, Ordering::Release);
        drop(permit);
    }

    #[test]
    #[ignore = "requires a Metal GPU and no Elegance renderer"]
    fn metal_attached_queue_validation_panic_and_worker_drop() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
        // All aliases are used serially on this thread, with no staged HAL work.
        let mut bridge =
            unsafe { Bridge::from_device(device.clone(), queue.clone(), test_config()) }.unwrap();
        let desc = wgpu::TextureDescriptor {
            label: None,
            size: bridge.extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: test_config().format,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        let (other_device, _) = block_on(adapter.request_device(&Default::default())).unwrap();
        let wrong_device = other_device.create_texture(&desc);
        for _ in 0..8 {
            assert!(matches!(
                bridge.copy_from(&wrong_device),
                Err(Error::Device(_))
            ));
            assert_eq!(
                bridge.in_flight(),
                0,
                "pre-submit validation must return capacity"
            );
            assert_eq!(bridge.quarantined(), 0);
        }
        let source = device.create_texture(&desc);
        let view = source.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        queue.submit([encoder.finish()]);
        assert!(matches!(
            bridge.copy_from(&source),
            Err(Error::Publish(raw::STATUS_DEVICE_UNAVAILABLE))
        ));
        wait_for(|| bridge.in_flight() == 0);
        assert_eq!(
            bridge.quarantined(),
            0,
            "invalid copies must not leave staged signals behind"
        );

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            bridge.render(|_| panic!("deliberate producer panic"))
        }));
        assert!(panic.is_err());
        assert_eq!(bridge.in_flight(), 1);
        assert_eq!(bridge.quarantined(), 1);
        assert!(matches!(
            bridge.clear_color(wgpu::Color::BLUE),
            Err(Error::Publish(raw::STATUS_DEVICE_UNAVAILABLE))
        ));
        wait_for(|| bridge.in_flight() == 1);
        assert_eq!(
            bridge.quarantined(),
            1,
            "unwinding must restore error scopes and signal staging"
        );

        let (tx, rx) = mpsc::channel();
        tx.send(Retirement::pending(bridge.allocate().unwrap()))
            .unwrap_or_else(|_| panic!("send failed"));
        drop(rx); // Models a worker dying with records still in its channel.
        assert_eq!(bridge.in_flight(), 2);
        assert_eq!(bridge.quarantined(), 2);
        let pending = Retirement::pending(bridge.allocate().unwrap());
        assert!(std::thread::spawn(move || {
            let _pending = pending;
            panic!("deliberate retirement-worker panic");
        })
        .join()
        .is_err());
        assert_eq!(bridge.in_flight(), 3);
        assert_eq!(bridge.quarantined(), 3);
        // The final permit remains usable; delayed callbacks and their worker
        // must also survive destruction of the public bridge and C image handle.
        let backing = bridge.allocate().unwrap();
        backing.producer.setSignaledValue(1); // No synthetic producer GPU accesses.
        let counts = bridge.counts.clone();
        let state = Box::into_raw(Box::new(Callback {
            backing,
            tx: bridge.tx.clone(),
        }));
        drop(bridge);
        assert_eq!(counts.active.load(Ordering::Acquire), 4);
        let release = Release {
            status: raw::STATUS_OK,
            token: 0,
            fences: std::ptr::null(),
            fence_count: 0,
            vk_old_layout: 0,
            vk_new_layout: 0,
        };
        unsafe {
            released(state.cast(), &release);
        }
        wait_for(|| counts.active.load(Ordering::Acquire) == 3);
        assert_eq!(counts.quarantined.load(Ordering::Acquire), 3);
    }

    // A tiny executor avoids a runtime dependency for these hardware tests.
    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        struct Wake;
        impl std::task::Wake for Wake {
            fn wake(self: Arc<Self>) {}
        }
        let waker = std::task::Waker::from(Arc::new(Wake));
        let mut cx = std::task::Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut cx) {
                return value;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    #[ignore = "requires access to a Metal GPU; uses synthetic native retirement records"]
    fn metal_callback_retains_and_waits_every_consumer_fence() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        let bridge = block_on(Bridge::new(
            &adapter,
            Config {
                width: 16,
                height: 16,
                origin: crate::GpuImageOrigin::TopLeft,
                format: wgpu::TextureFormat::Bgra8Unorm,
                max_frames: 2,
            },
        ))
        .unwrap();
        let backing = bridge.allocate().unwrap();
        let device = unsafe { bridge.device.as_hal::<Metal>() }.unwrap();
        let first = device.raw_device().newSharedEvent().unwrap();
        let second = device.raw_device().newSharedEvent().unwrap();
        drop(device);
        // This synthetic retirement has no GPU accesses; CPU signaling is used
        // only to test callback retention and the all-dependencies predicate.
        backing.producer.setSignaledValue(1);
        let fences = [
            Fence {
                r#type: raw::FENCE_MTL_SHARED_EVENT,
                handle: Retained::as_ptr(&first).cast_mut().cast(),
                fd: -1,
                value: 3,
            },
            Fence {
                r#type: raw::FENCE_MTL_SHARED_EVENT,
                handle: Retained::as_ptr(&second).cast_mut().cast(),
                fd: -1,
                value: 5,
            },
        ];
        let state = Box::into_raw(Box::new(Callback {
            backing,
            tx: bridge.tx.clone(),
        }));
        let release = Release {
            status: raw::STATUS_OK,
            token: 0,
            fences: fences.as_ptr(),
            fence_count: fences.len(),
            vk_old_layout: 0,
            vk_new_layout: 0,
        };
        unsafe {
            released(state.cast(), &release);
        }
        first.setSignaledValue(3);
        drop(first);
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(
            bridge.in_flight(),
            1,
            "must wait for the second fence as well"
        );
        second.setSignaledValue(5);
        drop(second);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while bridge.in_flight() != 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(bridge.in_flight(), 0);
        assert_eq!(bridge.quarantined(), 0);

        let backing = bridge.allocate().unwrap();
        let state = Box::into_raw(Box::new(Callback {
            backing,
            tx: bridge.tx.clone(),
        }));
        let release = Release {
            status: raw::STATUS_SYNC_FAILED,
            token: 0,
            fences: std::ptr::null(),
            fence_count: 0,
            vk_old_layout: 0,
            vk_new_layout: 0,
        };
        unsafe {
            released(state.cast(), &release);
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while bridge.quarantined() == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(bridge.quarantined(), 1);
        assert_eq!(
            bridge.in_flight(),
            1,
            "failed empty fence list must not release capacity"
        );
    }

    #[test]
    #[ignore = "requires access to a Metal GPU; no Elegance renderer should be running"]
    fn metal_rejected_publish_preserves_producer_until_complete() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        let mut bridge = block_on(Bridge::new(
            &adapter,
            Config {
                width: 16,
                height: 16,
                origin: crate::GpuImageOrigin::TopLeft,
                format: wgpu::TextureFormat::Bgra8Unorm,
                max_frames: 2,
            },
        ))
        .unwrap();
        let buffer = bridge.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("safe frame buffer update"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        assert!(matches!(
            bridge.render(|frame| {
                assert_eq!(frame.size().width, 16);
                frame.write_buffer(&buffer, 0, &[1, 2, 3, 4]);
                frame.render_pass(crate::RenderPassOptions {
                    clear: wgpu::Color::RED,
                    ..Default::default()
                }, |_| {});
            }),
            Err(Error::Publish(raw::STATUS_DEVICE_UNAVAILABLE))
        ));
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while bridge.in_flight() != 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(bridge.in_flight(), 0, "producer event never completed");
        assert_eq!(bridge.quarantined(), 0);
    }
}
