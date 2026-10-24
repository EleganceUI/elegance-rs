use crate::{wgpu, Bridge, Error};
use wgpu::util::DeviceExt;

/// Attachments for a pass rendering into the bridge's private output texture.
/// The output is always stored, or resolved from `multisample` when supplied.
#[derive(Default)]
pub struct RenderPassOptions<'a> {
    pub label: Option<&'a str>,
    /// RGB is written to the UNORM target as supplied (sRGB-encoded, straight alpha).
    pub clear: wgpu::Color,
    /// Optional multisampled color attachment; it resolves directly into the output.
    pub multisample: Option<&'a wgpu::TextureView>,
    pub depth_stencil: Option<wgpu::RenderPassDepthStencilAttachment<'a>>,
}

/// Lazily cached attachments for [`RenderFrame::render_pass_with`].
///
/// Dimensions and color format follow the frame's private output. Resources are
/// reused until dimensions, color format, or device change; no output handles
/// are retained or exposed. Construction does not allocate GPU resources.
///
/// Use the same sample count and depth/stencil format in your render pipeline.
/// Unsupported sample counts and formats are reported through wgpu validation
/// when first used. Depth is cleared to 1, stencil to 0, and both are discarded
/// after each pass. Use [`RenderFrame::render_pass`] for custom attachment ops.
pub struct RenderAttachments {
    sample_count: u32,
    depth_format: Option<wgpu::TextureFormat>,
    cached: Option<CachedAttachments>,
}

struct CachedAttachments {
    device: wgpu::Device,
    size: wgpu::Extent3d,
    color_format: wgpu::TextureFormat,
    multisample: Option<wgpu::TextureView>,
    depth_stencil: Option<wgpu::TextureView>,
}

impl RenderAttachments {
    pub fn new(sample_count: u32, depth_format: Option<wgpu::TextureFormat>) -> Self {
        Self {
            sample_count,
            depth_format,
            cached: None,
        }
    }

    fn prepare(&mut self, device: &wgpu::Device, target: &wgpu::Texture) {
        let size = target.size();
        let color_format = target.format();
        if self.cached.as_ref().is_some_and(|cached| {
            cached.device == *device && cached.size == size && cached.color_format == color_format
        }) {
            return;
        }
        let attachment = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size,
                    mip_level_count: 1,
                    sample_count: self.sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        self.cached = Some(CachedAttachments {
            device: device.clone(),
            size,
            color_format,
            // Also let wgpu validate a sample count of zero rather than silently
            // treating it as single-sampled.
            multisample: (self.sample_count != 1)
                .then(|| attachment("shared frame MSAA", color_format)),
            depth_stencil: self
                .depth_format
                .map(|format| attachment("shared frame depth/stencil", format)),
        });
    }

    fn depth_stencil_ops(&self) -> (Option<wgpu::Operations<f32>>, Option<wgpu::Operations<u32>>) {
        let format = self.depth_format;
        (
            format
                .filter(wgpu::TextureFormat::has_depth_aspect)
                .map(|_| wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
            format
                .filter(wgpu::TextureFormat::has_stencil_aspect)
                .map(|_| wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Discard,
                }),
        )
    }
}

/// Restricted recording access to one shared GPU frame.
///
/// The destination texture, views, command encoder and command buffer are private:
/// they cannot be retained and used after publication. Render passes are scoped
/// to callbacks, and the bridge alone finishes and submits the encoder.
///
/// Destination handles cannot escape through the safe API:
/// ```compile_fail
/// use elegance_wgpu::{wgpu, RenderFrame};
/// fn steal_target(frame: &mut RenderFrame<'_>) -> wgpu::Texture {
///     frame.target.clone()
/// }
/// ```
/// Nor can the encoder be taken out and submitted later:
/// ```compile_fail
/// use elegance_wgpu::RenderFrame;
/// fn steal_encoder(frame: &mut RenderFrame<'_>) {
///     frame.encoder.finish();
/// }
/// ```
pub struct RenderFrame<'a> {
    device: &'a wgpu::Device,
    encoder: &'a mut wgpu::CommandEncoder,
    target: &'a wgpu::Texture,
}

impl RenderFrame<'_> {
    pub fn size(&self) -> wgpu::Extent3d {
        self.target.size()
    }

    /// Stage a CPU-to-GPU buffer update before subsequent passes in this frame.
    /// The buffer must belong to the bridge's device and allow COPY_DST. wgpu
    /// validates alignment, bounds and usage; this never maps image pixels.
    pub fn write_buffer(&mut self, buffer: &wgpu::Buffer, offset: u64, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let staging = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shared frame buffer update"),
                contents: data,
                usage: wgpu::BufferUsages::COPY_SRC,
            });
        self.encoder
            .copy_buffer_to_buffer(&staging, 0, buffer, offset, data.len() as u64);
    }

    /// Record a pass using lazily sized, reusable MSAA and depth/stencil attachments.
    /// The shared output is always stored (resolved when multisampling).
    /// Pipeline compatibility and invalid configurations use wgpu validation.
    pub fn render_pass_with(
        &mut self,
        attachments: &mut RenderAttachments,
        clear: wgpu::Color,
        encode: impl FnOnce(&mut wgpu::RenderPass<'_>),
    ) {
        attachments.prepare(self.device, self.target);
        let cached = attachments.cached.as_ref().unwrap();
        let (depth_ops, stencil_ops) = attachments.depth_stencil_ops();
        self.render_pass(
            RenderPassOptions {
                clear,
                multisample: cached.multisample.as_ref(),
                depth_stencil: cached.depth_stencil.as_ref().map(|view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops,
                        stencil_ops,
                    }
                }),
                ..Default::default()
            },
            encode,
        );
    }

    /// Record a color pass without exposing the shared output or its encoder.
    /// A multisample/depth attachment, if supplied, must have matching dimensions
    /// and sample count. Invalid combinations are reported through wgpu validation.
    pub fn render_pass(
        &mut self,
        options: RenderPassOptions<'_>,
        encode: impl FnOnce(&mut wgpu::RenderPass<'_>),
    ) {
        let view = self.target.create_view(&Default::default());
        let mut pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: options.label,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: options.multisample.unwrap_or(&view),
                depth_slice: None,
                resolve_target: options.multisample.map(|_| &view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(options.clear),
                    store: if options.multisample.is_some() {
                        wgpu::StoreOp::Discard
                    } else {
                        wgpu::StoreOp::Store
                    },
                },
            })],
            depth_stencil_attachment: options.depth_stencil,
            ..Default::default()
        });
        encode(&mut pass);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachments_are_lazy_and_clear_only_present_aspects() {
        for (format, depth, stencil) in [
            (None, false, false),
            (Some(wgpu::TextureFormat::Depth32Float), true, false),
            (Some(wgpu::TextureFormat::Stencil8), false, true),
            (Some(wgpu::TextureFormat::Depth24PlusStencil8), true, true),
        ] {
            let attachments = RenderAttachments::new(4, format);
            assert!(attachments.cached.is_none());
            let (depth_ops, stencil_ops) = attachments.depth_stencil_ops();
            assert_eq!(depth_ops.is_some(), depth);
            assert_eq!(stencil_ops.is_some(), stencil);
            if let Some(ops) = depth_ops {
                assert_eq!(ops.load, wgpu::LoadOp::Clear(1.0));
                assert_eq!(ops.store, wgpu::StoreOp::Discard);
            }
            if let Some(ops) = stencil_ops {
                assert_eq!(ops.load, wgpu::LoadOp::Clear(0));
                assert_eq!(ops.store, wgpu::StoreOp::Discard);
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires a Metal GPU and the native Elegance library"]
    fn metal_attachments_reuse_and_follow_size_format_and_device() {
        use std::{future::Future, sync::Arc, task};

        fn block_on<T>(future: impl Future<Output = T>) -> T {
            struct Wake;
            impl task::Wake for Wake {
                fn wake(self: Arc<Self>) {}
            }
            let waker = task::Waker::from(Arc::new(Wake));
            let mut cx = task::Context::from_waker(&waker);
            let mut future = std::pin::pin!(future);
            loop {
                if let task::Poll::Ready(value) = future.as_mut().poll(&mut cx) {
                    return value;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }

        fn render(
            device: &wgpu::Device,
            queue: &wgpu::Queue,
            attachments: &mut RenderAttachments,
            width: u32,
            height: u32,
            format: wgpu::TextureFormat,
        ) {
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("attachment test output"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            let mut frame = RenderFrame {
                device,
                encoder: &mut encoder,
                target: &target,
            };
            frame.render_pass_with(attachments, wgpu::Color::BLACK, |_| {});
            queue.submit([encoder.finish()]);
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        }

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = block_on(adapter.request_device(&Default::default())).unwrap();
        let (other_device, other_queue) =
            block_on(adapter.request_device(&Default::default())).unwrap();
        for sample_count in [1, 4] {
            for depth_format in [
                None,
                Some(wgpu::TextureFormat::Depth32Float),
                Some(wgpu::TextureFormat::Stencil8),
                Some(wgpu::TextureFormat::Depth24PlusStencil8),
            ] {
                let mut attachments = RenderAttachments::new(sample_count, depth_format);
                let mut previous: Option<(Option<wgpu::TextureView>, Option<wgpu::TextureView>)> =
                    None;
                for (width, height, format) in [
                    (16, 16, wgpu::TextureFormat::Rgba8Unorm),
                    (32, 16, wgpu::TextureFormat::Rgba8Unorm),
                    (32, 48, wgpu::TextureFormat::Rgba8Unorm),
                    (32, 48, wgpu::TextureFormat::Bgra8Unorm),
                ] {
                    render(&device, &queue, &mut attachments, width, height, format);
                    let cached = attachments.cached.as_ref().unwrap();
                    assert_eq!(cached.size.width, width);
                    assert_eq!(cached.size.height, height);
                    assert_eq!(cached.color_format, format);
                    assert_eq!(cached.multisample.is_some(), sample_count > 1);
                    assert_eq!(cached.depth_stencil.is_some(), depth_format.is_some());
                    let views = (cached.multisample.clone(), cached.depth_stencil.clone());
                    if let Some(previous) = previous {
                        if views.0.is_some() {
                            assert_ne!(views.0, previous.0);
                        }
                        if views.1.is_some() {
                            assert_ne!(views.1, previous.1);
                        }
                    }
                    render(&device, &queue, &mut attachments, width, height, format);
                    let cached = attachments.cached.as_ref().unwrap();
                    assert_eq!(
                        views,
                        (cached.multisample.clone(), cached.depth_stencil.clone())
                    );
                    previous = Some(views);
                }
                render(
                    &other_device,
                    &other_queue,
                    &mut attachments,
                    32,
                    48,
                    wgpu::TextureFormat::Bgra8Unorm,
                );
                let cached = attachments.cached.as_ref().unwrap();
                assert_eq!(cached.device, other_device);
                let previous = previous.unwrap();
                if cached.multisample.is_some() {
                    assert_ne!(cached.multisample, previous.0);
                }
                if cached.depth_stencil.is_some() {
                    assert_ne!(cached.depth_stencil, previous.1);
                }
            }
        }
    }
}

impl Bridge {
    /// Record and publish a GPU frame without exposing reusable destination handles.
    /// Resource creation uses `device()`. All frame writes/submission remain owned
    /// by the bridge, and existing GPU sharing/fence synchronization is unchanged.
    /// Use unsafe `render_raw` only when access beyond the restricted frame API is needed.
    pub fn render(&mut self, encode: impl FnOnce(&mut RenderFrame<'_>)) -> Result<(), Error> {
        // SAFETY: RenderFrame exposes neither the destination nor its encoder.
        // Supplied attachments and buffers are caller-owned; passes cannot finish
        // or submit the private encoder. Backend allocation initializes the target.
        unsafe {
            self.render_raw(|device, encoder, target| {
                encode(&mut RenderFrame {
                    device,
                    encoder,
                    target,
                });
            })
        }
    }
}
