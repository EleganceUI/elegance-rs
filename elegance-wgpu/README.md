# elegance-wgpu

A bounded, zero-CPU-pixel-copy bridge from native Rust **wgpu 30.0.1** to Elegance's new `GpuImage` input.

## Implementation and validation status

| Platform | Producer → Elegance | Implementation | Validation |
| --- | --- | --- | --- |
| macOS | wgpu Metal → Dawn Metal, IOSurface + shared events | Implemented | Real Apple M4 Pro GPU tests and one/two-window publication passed |
| Windows | wgpu DX12 → Dawn D3D12, shared DXGI textures + fences | Implemented | Rust MSVC cross-target check; native linking/GPU execution not tested |
| Linux | wgpu Vulkan → Dawn Vulkan, dedicated opaque-FD memory + binary semaphores | Implemented | Rust Linux cross-target check and isolated host unit tests; native linking/GPU execution not tested |

This is **not support for every Dawn backend**. D3D11, OpenGL/GLES, cross-adapter sharing, DMA-BUF, YUV/multi-plane images, and device-loss recovery are not implemented. Unsupported paths fail explicitly, with no CPU-copy fallback. Choose the same physical GPU on both sides; there is no automatic adapter negotiation.

## Local wgpu checkout

`wgpu/` is a vendored clone of <https://github.com/gfx-rs/wgpu>, pinned to tag `v30.0.1`, commit `40f4a34ebaf56f9a046231f54125ad046239d3f3`. Its sources, licenses and upstream Cargo workspace are committed directly in this repository, not as a submodule, so fresh checkouts need no additional Git repository. No remote fork was created or pushed. The original local clone metadata was preserved in the parent repository's `.git/elegance-wgpu-upstream.git` (local only, not needed to build).

Inspection found that this release already provides the raw HAL texture import and external queue synchronization hooks needed by these three transports. **No GPU implementation patches to wgpu were necessary.** The local changes to its package manifests explicitly select their nested workspace, allowing the checkout to remain inside this crate. The parent workspace uses glob membership and excludes `elegance-wgpu/wgpu`; parent Cargo builds now work.

Use `elegance_wgpu::wgpu` in the application and renderer, or point their `wgpu` dependencies to the **same local package**. Registry `wgpu` and this path package are different Cargo identities even when their versions match. A top-level application can patch registry dependencies:

```toml
[patch.crates-io]
wgpu = { path = "/path/to/elegance-rs/elegance-wgpu/wgpu/wgpu" }
```

Cargo patches must be set at the consuming workspace root. Rust **1.93+** is required by the pinned wgpu tree. The vendored source is a separate Cargo workspace, but is versioned together with this integration. Preserve its upstream license files and explicit workspace pointers when updating the pinned snapshot.

## Basic usage

```rust,no_run
use elegance_wgpu::{wgpu, Bridge, Config, GpuImageOrigin};

async fn make_bridge() -> Result<Bridge, Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(
        wgpu::InstanceDescriptor::new_without_display_handle(),
    );
    let adapter = instance.request_adapter(&Default::default()).await?;
    Ok(Bridge::new(&adapter, Config {
        width: 640,
        height: 480,
        origin: GpuImageOrigin::TopLeft,
        format: wgpu::TextureFormat::Bgra8Unorm,
        max_frames: 3,
    }).await?)
}

fn produce(bridge: &mut Bridge) -> Result<(), elegance_wgpu::Error> {
    // After Elegance's render device is initialized:
    bridge.clear_color(wgpu::Color::RED)?;
    // In the UI: GpuImageView::new(bridge.image())
    Ok(())
}
```

`Bridge::new` creates its own device/private queue on the chosen adapter. `device()` exposes that device for resource creation, but the queue is not exposed. `image()` is the cloneable UI source, not a writable producer texture.

## Attach an existing renderer

All three implementations provide:

```rust,no_run
# use elegance_wgpu::{wgpu, Bridge, Config, Error};
# fn attach(device: wgpu::Device, queue: wgpu::Queue, config: Config) -> Result<Bridge, Error> {
let bridge = unsafe { Bridge::from_device(device, queue, config)? };
// On the renderer's thread, after submitting writes to its output texture:
// bridge.copy_from(&renderer_output)?;
# Ok(bridge) }
```

**`from_device` is unsafe.** The handles must belong to the same logical device/queue pair. Serialize each **entire** bridge production call against all operations through all aliases of that queue: submissions, writes and native/HAL operations, including reentrant callbacks. A single dedicated renderer thread can satisfy this; otherwise use one shared queue lock. Enter with no outstanding staged external waits/signals. Do not destroy the device while bridge operations or retirement are outstanding. Read the constructor's full safety contract before use.

`copy_from` requires a same-device, same-size/format, single-sample `COPY_SRC` texture. Submit source writes first. The bridge GPU-copies into its own shareable allocation; Elegance never reads the renderer's original texture. Subsequent renderer submissions can reuse that source normally.

### Linux device creation requirements

An ordinary wgpu `request_device` is **not sufficient** for Vulkan attachment. The device must enable `VK_KHR_external_memory_fd` and `VK_KHR_external_semaphore_fd`, and actually create/reserve the secondary queue required by `from_device`. Physical queue capacity does not prove that the queue was created. The constructor's unsafe contract requires exclusive reservation of that retirement queue even after `Bridge` drops while callbacks remain outstanding.

`Bridge::new` handles custom device/queue creation internally using HAL. It can select a separate retirement family; attachment currently requires a second queue in the producer's graphics family. Missing extensions, suitable queues, formats, or memory/semaphore capabilities return an error. Existing Linux renderers therefore need device-creation integration, not just a post-hoc texture pointer.

## Frame operations and ownership

- `clear_color` renders into a shareable frame.
- `copy_from` performs a GPU-to-GPU copy, never pixel readback.
- `unsafe render` records arbitrary wgpu commands directly into the shareable frame, avoiding that copy.
- `clear` retires the current image asynchronously.
- `in_flight` includes producer, displayed, snapshot-held, retiring, and quarantined allocations.
- `quarantined` counts allocations retained because GPU completion could not be established.

A full budget returns `Error::Busy`; retry later while retaining the last displayed frame. Retained UI snapshots may occupy every slot. This implementation allocates fresh frames and frees them on completion; it does **not yet recycle a persistent texture pool**.

`render` is intentionally unsafe: ordinary wgpu texture/view/bind-group handles are cloneable. Do not let destination aliases, native handles, or recorded commands escape its closure. Encode all writes in the supplied encoder, finish passes inside the closure, never destroy the target/device, and never write the target after publication. See its API safety documentation.

Frames are single-mip/layer/sample RGBA8Unorm or BGRA8Unorm with **sRGB-encoded RGB and straight alpha**. Elegance samples an sRGB view. No format conversion or MSAA resolve is supplied automatically.

## Synchronization and performance

The producer signals a GPU fence after its writes. Dawn waits before sampling, retains the imported allocation across all snapshots/windows/submissions, then exports every consumer dependency. Rust retains/duplicates borrowed fence handles during the callback and holds backing until producer and consumer GPU completion. Vulkan additionally performs external queue-family ownership and layout transitions; its retirement queue waits imported consumer semaphores.

Release callbacks mean **no future consumer submissions**, not GPU completion. Failed imports have no callback, but producer work must still complete before backing can be freed. Unknown completion, device loss, failed exports or worker unwinding quarantine allocations and their budget permits rather than risking reuse. Quarantine intentionally retains resources until process exit.

**No CPU pixel copies occur.** There is still CPU coordination, allocation, and asynchronous completion tracking. Metal and Vulkan currently GPU-clear each fresh allocation and wait on the CPU for initialization before HAL import; Vulkan bounds that wait at five seconds and quarantines uncertain completion. DX12 initializes on its producer queue before later wgpu work. Persistent pooling is a follow-up optimization, not an implemented performance claim.

Window presentation remains Elegance's FIFO/VSync path. The producer is independent; publication is not itself display-paced. Keep playback/media timing separate from preview display timing and export rendering.

## Shutdown

Stop/join producers, clear images and remove UI views/snapshots before renderer shutdown. `Drop` is not a GPU drain. Callback state and retirement workers retain resources after the bridge drops as necessary. Device-loss recovery is not implemented.
