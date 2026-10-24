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
