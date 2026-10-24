# elegance-rs

Rust bindings to [Elegance](https://github.com/EleganceUI/elegance), with `elg!`, a SwiftUI-style
DSL. Examples are in [elegance-examples](https://github.com/EleganceUI/elegance-examples)
(the `*-rs` folders).

```rust
use elegance::prelude::*;

#[component]
fn Counter() -> impl IntoView {
    let mut count = State::new(0);
    elg! {
        VStack(spacing: 16) {
            Text("{count}").font(Font::system(56.0, .bold))
            HStack {
                Button("−", move || count -= 1)
                Button("+", move || count += 1)
            }
        }
    }
}

fn main() -> Result<std::process::ExitCode, Error> {
    App::new().window(WindowDesc::new("Counter").size(360, 280), Counter).run()
}
```

## Crates

- `elegance-sys`: a C++ shim (`shim/*.cpp`) giving Elegance's C++23 API a C ABI, and the raw FFI.
- `elegance`: the safe API: views, modifiers, reactive state, environment, styles, layouts, custom
  modifiers, effects, animation, media, GPU images, HTTP/WebSocket, desktop, previews.
- `elegance-macros`: `elg!`, `#[component]`, and `#[preview]`.
- `cargo-elegance`: `cargo elegance preview`, the preview host runner.
- [`elegance-wgpu`](elegance-wgpu/README.md): optional native Rust wgpu → Dawn GPU-image
  sharing, with a local pinned wgpu checkout. Separate from the existing `GpuImage` API;
  Metal is hardware-tested here, while DX12/Vulkan still require platform validation.

## Using it

```toml
[dependencies]
elegance = { git = "https://github.com/EleganceUI/elegance-rs" }
```

The first build compiles Elegance itself (C++, with CMake and vcpkg), which takes a while: about
half an hour when vcpkg has to build Dawn. `cargo build -vv` shows its progress. Later builds reuse
it. You need Visual Studio's C++ workload (or Apple Clang on macOS) and git; the rest is found or
downloaded.

Elegance is linked statically by default: an app is one executable plus the DLLs Elegance's
dependencies come in, which are copied next to it. The `dynamic` feature links `elegance.dll`
instead, which previews need.

### Where Elegance comes from

`elegance-sys`'s build script needs Elegance's source:

1. `ELEGANCE_ROOT`, if set;
2. a checkout beside this repository (`../elegance`), for working on both;
3. otherwise a clone of `EleganceUI/elegance` in the user cache folder
   (`%LOCALAPPDATA%\elegance\src\main`, `~/.cache/elegance/src/main`), made once and shared by
   every project. `ELEGANCE_GIT_REF` picks another branch or tag; delete the folder to update it.

Then a build of it to link, in this order:

1. `ELEGANCE_BUILD_DIR` (static) / `ELEGANCE_SHARED_BUILD_DIR` (`dynamic`), if set;
2. `build/dev-release-static` / `build/dev-release` in the source tree, if built;
3. otherwise it builds one itself, into `build/cargo-release-static` (or `cargo-release`), and
   rebuilds it when Elegance's sources change. Set `ELEGANCE_AUTO_BUILD=0` to get an error instead.

For that build it uses the first vcpkg that has the baseline Elegance's `vcpkg.json` pins:
`VCPKG_ROOT`, the `VCPKG_ROOT` Elegance's `CMakeUserPresets.json` sets, `vcpkg` on PATH, or a clone
it bootstraps in `ELEGANCE_VCPKG_DIR` (default: the cache folder's `elegance/vcpkg`). CMake 4.3+
and Ninja come from `CMAKE`/`NINJA`, PATH, Visual Studio's bundled copies, or `vcpkg fetch`; MSVC's
environment is found without a developer prompt. The choice of build is made when the build script
runs: after building a tree of your own, `cargo clean -p elegance-sys` to switch to it.

A build of your own must use the release C runtime (`/MD`) like Rust, not a Debug build's `/MDd`
(the shim passes `std::string` and `std::function` to Elegance), and a static one needs
`-DELG_ACCESSKIT_SHARED=ON` (AccessKit's static library is a Rust staticlib, whose own Rust standard
library clashes with the app's).

### Working on the bindings

Check out [elegance](https://github.com/EleganceUI/elegance) beside this repository, and
[elegance-examples](https://github.com/EleganceUI/elegance-examples) to try changes; to build the
examples against this checkout instead of GitHub, add to `elegance-examples/.cargo/config.toml`:

```toml
[patch."https://github.com/EleganceUI/elegance-rs"]
elegance = { path = "../elegance-rs/elegance" }
```

## The `elg!` DSL

| DSL | Means |
|---|---|
| `Name(a, b)` | an element; overloads are tuple impls of `ElgNew` |
| `Name(a, key: v)` | the same, then the builder call `.key(v)` |
| `Name { ... }` | its children, on the element's line: a stack's views, a button's label, ... |
| `ForEach(xs, id: \|x\| x.id) { x in ... }` | a content closure per item (also `AsyncImage(url) { phase in ... }`) |
| `if cond { ... } else { ... }` | a reactive `If` |
| `match value { pat => View(), ... }` | the view for the current value, rebuilt when it changes (`Clone + PartialEq` values) |
| `.leading`, `.accent`, `.bold`, `.infinity` | shorthand for the parameter's type (`elegance::dot`) |
| `$name` | `name.binding()` |
| `"{count} left"` | text that updates when `count` changes |
| `{ expr }`, or a variable | any Rust value that is `IntoView` |
| `(layout) { ... }` | a custom `Layout` over the children |
| `.m(a, key: v)` | the modifier `.m_with(a, \|o\| o.key(v))`, e.g. `.frame(max_width: .infinity)` |

## Grids

`VGrid`/`HGrid` flow children into fixed, flexible, or adaptive tracks; `Grid` is a table of
`GridRow`s (see `grids-rs` in elegance-examples, and Elegance's Grids guide):

```rust
VGrid([GridItem::adaptive(150)], spacing: 12) {
    ForEach(games, id: |g| g.id) { game in Card(game: game.clone()) }
}
```

## Images

Put the app's own images in `<crate>/assets/` and compile them in with `asset!`, which works
wherever the views run (the app, a preview, an installed copy):

```rust
Image(asset!("images/logo.png")).resizable().frame(96, 96)
```

For images that only exist at runtime (files the user picks, downloads), use
`ImageSource::file(path)` with an absolute path: Elegance resolves relative paths against the
current directory.

## Previews and hot reload

Mark functions with `#[preview]` anywhere in an app; no library/binary split is needed:

```rust
#[preview("Counter")]
fn counter() -> impl IntoView {
    elg! { Counter() }
}

#[preview(root)] // the whole app, for `--root`
fn app() -> impl IntoView {
    elg! { Root() }
}
```

Then run the preview host, which rebuilds and reloads the views as you edit:

```sh
cargo install --git https://github.com/EleganceUI/elegance-rs cargo-elegance
cargo elegance preview            # the previews
cargo elegance preview --root     # the root view
cargo elegance preview -p my-app --bin my-app --release
```

`cargo elegance` builds the crate (its binary, or its library if it has no binary) a second time
as a views module: a cdylib, with `elegance/preview` enabled, in `target/elegance-preview`. That
build links the shared Elegance (`dynamic`), which the module and the host share. Without the
`preview` feature, `#[preview]` functions register nothing, so the app itself carries no preview
code. A reload rebuilds the views, so state in them starts over. `preview-rs` in elegance-examples
is set up this way.

## Effects

Materials, blur, color adjustments, blend modes, inner shadows and spread, elliptical and angular
gradients, rotation/scale effects, and masks (see `effects-rs` in elegance-examples):

```rust
Text("Hi").padding(12).background_in(Material::THIN, RoundedRectangle(12))
tile().grayscale(1).blend_mode(.multiply).rotation_effect(15.0)
backdrop().mask(LinearGradient::colors([Color::BLACK, Color::CLEAR]))
```

### GPU images (Dawn/WebGPU)

`GpuImage` (`elegance::media`) provides triple-buffered textures from **Elegance's own Dawn
WebGPU device**, sampled without native handle imports or CPU readback on all supported Dawn
backends. `GpuImageView(&image)` displays the image.

```rust,ignore
let image = GpuImage::new(512, 512, GpuImageOrigin::TopLeft)?;
// On a producer thread, keeping the image alive:
if let Some(frame) = image.begin_frame() {
    // Use Dawn's C API to encode writes to frame.texture with frame.device.
    // Submit ALL commands and queue writes to frame.queue before publishing.
    image.end_frame(&frame);
    // On failure instead: image.cancel_frame(&frame);
}
```

`GpuFrame` contains `buffer`, `width`, `height`, `device`, `queue`, `texture`, and `sequence`.
Return **every field unchanged** to end/cancel. `device`, `queue`, and `texture` are opaque
`*mut c_void` values representing borrowed Dawn C `WGPUDevice`, `WGPUQueue`, and `WGPUTexture`
handles. Use Elegance's matching Dawn runtime/ABI and `webgpu/webgpu.h`; never install another
Dawn proc table. They are **not Rust `wgpu` crate objects**, wgpu-native handles, cross-device
sharing handles, or importable OpenGL/Vulkan/Direct3D resources. Never release/destroy the
handles or device.

`begin_frame` is nonblocking: it returns `None` before a device exists, after shutdown, for
invalid dimensions, while another reservation is outstanding, or when no buffer is free.
Calls on image clones may be concurrent, but only one write reservation is granted at a time;
copying a frame does not create another. Safe Rust methods only forward tokens, not raw GPU
operations. The caller's unsafe Dawn interop must synchronize all writers before end/cancel;
never end/cancel another thread's in-progress writes. Keep the image alive while using handles,
and stop and join producers before renderer/application shutdown. Frames do not keep the
renderer running; device-loss recovery is unsupported.

Submission order on `frame.queue` synchronizes rendering: no fences or GPU completion waits
are needed. Never submit commands referencing the reservation after end/cancel, including when
cancelling after partial submission. Foreign, stale, and already-ended tokens are ignored;
newer published frames may replace older ones before display. Migration: `GpuImageSync` and
the `end_frame` sync argument have been removed; use `end_frame(&frame)` or `cancel_frame(&frame)`.

Textures are 2D RGBA8Unorm with one mip/layer/sample and RenderAttachment | TextureBinding |
CopySrc | CopyDst | StorageBinding usage. RGB must be sRGB-encoded and alpha straight, not
premultiplied. RGBA8UnormSrgb views are allowed: an sRGB attachment view automatically encodes
linear shader RGB; set its usage to RenderAttachment (sRGB cannot inherit StorageBinding).
Origin only flips display.

## State

State (`State`, `Binding`, `Computed`) is made of `Copy` handles, so `move` closures capture them
without cloning. State lives as long as the views that made it: when an `If` branch switches or a
`ForEach` item goes, the state created inside it is freed, and using a freed handle panics.
