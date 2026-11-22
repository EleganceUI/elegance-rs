# C++ vs Rust API gaps, and docs/README vs code inconsistencies

Method: read-only. C++ headers in BOTH /workspaces/mikros/elegance (older checkout) and the milestone-15 worktree
(/workspaces/mikros/.worktrees/elegance/milestone-15-mikros-target, 59 commits ahead of main; this is what elegance-sys's
`bindings.rs` (ABI 4) matches). Compared identifier-by-identifier and enum-by-enum against elegance-rs/elegance/src, plus the
C API header (worktree `c/include/elg-c/elegance.h`, 707 functions). Result: the Rust crate covers essentially the whole
public C++ view/modifier/state/window/menu/tray/media/capture/gesture/style/theme/layout API. The gaps below are small.

## 1. Headline finding: Mikros
- Worktree C++ adds `Platform::mikros` (include/elg/platform.h); C API `elg_current_platform()` returns 0 windows, 1 macos,
  2 linux, 3 mikros. Rust `Platform` has `Mikros` too, and returns other values as `Unknown(i32)`
  (elegance/src/platform.rs).
- On Mikros the C++ headers say these throw `PlatformError`: audio output (`Sound`, `AudioPlayer`, `AudioStream`), audio
  recording (`AudioRecorder`), `Camera`, video playback (`VideoPlayer` when it would play), `notify`, `TrayIcon`.
  Rust wrappers returning `Result` surface it; ones returning `()` (`notify`, `AudioStream::start`, `Sound::play`, ...) go
  through `ffi!` and panic. docs/building.md describes the Mikros build.
- Mikros build: c/CMakeLists.txt builds `eleganceui.<major>.dlib` exporting only `elg_*` (c/exports.map). elegance-sys links
  it dynamically for the Mikros target (build_mikros.rs) and refuses every other way of getting Elegance there.
- No Rust API for the Mikros-specific bits at all; nothing in the C API is Mikros-only beyond the platform value.

## 2. C++ features with no Rust counterpart
| C++ name (header) | What | In C API? | Docs say C++-only? |
|---|---|---|---|
| `Platform::mikros` (platform.h) | 4th platform | yes (value 3) | covered: `Platform::Mikros` |
| `EnvironmentValues::get_named<T>/set_named<T>(name)`, `LayoutChild::value_named<T>` (environment.h, layout.h; worktree only) | read/set a key by name without a key type; T in bool,int,i32/i64,f32,f64,String,Color | yes (`elg_env_get_named_*`, `elg_env_set_named_*`, `elg_layout_child_value_*`) | yes: environment.mdx cpp-only block. Rust uses them internally (named.rs): a Rust `EnvironmentKey` whose Value is one of those types interoperates by name, but `get_named`/`set_named` are not public |
| `ELG_MODULE`, `ELG_MODULE_WITH_APP_HASH`, `ELG_PREVIEW`, `HotReloadOptions`, `layout_hash<T...>()`, CMake `elegance_views_module`, `elegance_hot_reload`, `elg_add_application` (hot_reload.h, CMake) | views-module authoring, build glue | C API has `elg_hot_reload(...)` only | yes (hot-reload.mdx, desktop.mdx). Rust equivalents: `#[preview]`, `#[preview(root, app_hash = ...)]`, `layout_hash!`, `cargo elegance`, `HotReload` (feature `dynamic`) |
| `ELG_TRACY` profiling, `ELG_ACCESSIBILITY` option, ELG_SANITIZER, `ELG_BUILD_SHARED` (CMake) | build options | n/a | yes (debugging.mdx, installation.mdx cpp-only). Prebuilt Rust package has accessibility on, no Tracy |
| `UsageError/ResourceError/GpuError/PlatformError` as exception types (error.h) | typed exceptions | via `ElgError` kinds | Rust has `ErrorKind::{Usage,Resource,Gpu,Platform,Elegance,Other}`; only `App::new/run` and constructors return `Result`, everything else panics |
| `MoveOnlyFunction`, `Prop` internals, `detail::*` (`release_on_ui_thread`, `begin_batch`, `Scope`, `DeferEffects`, `ReactiveNode`) | internals | partly (`elg_check_ui_thread`, `elg_node_notify`) | internal; ignore |
| `elg_gpu_image_publish` C ABI + `gpu_image_source()/gpu_image_flipped()` (gpu_image.h) | GPU image producer ABI | yes | Rust has `GpuImage`, `GpuImageView`, `GpuImageOrigin` and crate `elegance-wgpu`; no guide page in docs (only README + elegance-wgpu/README) |
| C++ default arguments (see section 3) | ergonomics | n/a | not a gap, but Rust must pass them |
| `View::apply` free-function modifiers | chaining own modifiers | n/a | Rust: `Modifier` traits / extension trait via `#[elg_builder]` |

C API functions declared but not used by elegance (could be bound; none is a feature gap): `elg_binding_i32` and the
`elg_binding_{bool,f64,i32,string}_free` family, `elg_image_named` (icon by name; Rust builds `ImageSource` instead),
`elg_prop_color_clone`, `elg_prop_f32_clone`. (Some may be reached through macros; I grepped for the literal names.)

Checked and NOT gaps (all present in Rust): clipboard (no public clipboard API exists in C++ either; only built-in text
editing), drag sources (C++ has only receiving: `on_drop` = Rust `on_drop`/`DropOptions`; `DragGesture` is pointer dragging),
capture (`AudioRecorder`, `Camera`, `CameraView`, `audio_input_devices`, `camera_devices`), all gestures
(`DragGesture`, `LongPressGesture`, `MagnifyGesture`, `RotateGesture`, `TwoFingerDoubleTapGesture`), menus (`MenuBarItem`,
`MenuRole` all variants, `Menu`, `MenuButton`, `Submenu`, `context_menu`), tray/notification/badge/progress/attention,
file dialogs, accessibility (all 7 modifiers and every `AccessibilityRole` variant), theme tokens (all 17 `ThemeColor`,
`ThemeBlur`, `ColorTokens`, `BlurTokens`, `system_accent_color`), `Key` (every variant), `KeyModifiers`, `WindowDesc` (every
field), `WindowHandle` (every method), `WindowMaterial`, `TitleBar`, text editing options (`secure`, `line_limit`, `styler`,
`on_key`, `on_submit`), grid/hgrid/vgrid/GridRow, custom `Layout` + `Cached`, layout values, `Canvas`/`GraphicsContext`,
`Path` ops, `ColorFilter` kinds, `VideoFeed/VideoFormat`, `AudioStream`, `HotReloadShow`, `Timer after/every`.

## 3. Same feature, different Rust shape (affects code you write)
| C++ | Rust (verified in source) |
|---|---|
| `Animation::spring()`, `smooth()`, `ease_out()`... default args | all constructors require args: `Animation::spring(response, damping_fraction)`, `smooth(duration)`, `linear/ease_in/ease_out/ease_in_out(duration)`. Only `Animation::default()` = `ease_in_out(0.35)` is argument-free |
| `repeat_count(n, autoreverses = true)`, `repeat_forever(autoreverses = true)` | both require the bool |
| `Transition::move(Edges)`, `scale(from = 0, anchor = center)` | `Transition::move_edge(edge)`, `scale(from)` + `scale_anchored(from, anchor)` |
| `disabled(true)`, `focusable(true)`, `window_drag_area(true)`, `accessibility_hidden(true)`, `shadow({})`, `scroll_to_top(animated = true)`, `request_attention(critical = false)`, `If(c, a, b = View())`, `aspect_ratio(r, mode = fit)`, `border(color, w = 1)` | argument required in Rust: `disabled(bool/prop)`, `focusable(bool)`, `shadow(Shadow)`, `scroll_to_top(animated)`, `request_attention(critical)`, `If(cond, then, else)` (use `elg!` `if` for else-less), `aspect_ratio(ratio, mode)`, `border(color, width)` |
| overloads (`padding(Edges, len)`, `frame({...})`, `background(View)`, `mask(View)`, `blur(r, opaque)`, `tooltip(fn)`, `selection(b, opts)`, `on_click(fn, opts)`, `scale_effect(x, y, anchor)`) | split into suffixed methods: `padding_edges/_insets/_with`, `frame_with/frame_size`, `background_in/_view/_view_with`, `mask_view(_with)`, `blur_with`, `tooltip_view/tooltip_with`, `selection_with`, `on_click_with`, `scale_effect_xy(_with)`, `rotation_effect_with`, `color_invert_amount`, `fixed_size_axes` |
| `GraphicsContext::fill_rect(rect, color, radii = {})`, `fill_path(path, style, rule = non_zero)`, `push_clip(rect, radii = {})` | `radii`/`rule` required |
| `State<T>` works for any T (equality-comparable T skips equal writes) | `State::new` needs `T: PartialEq`; otherwise `State::always_notify(v)`; `computed(f)` needs `PartialEq` (`Computed::always_notify` exists as a constructor only); `State::get` needs `Clone`; `read()/peek()/write()` guards, `toggle()`, `map()`, `is_alive()` (Rust-only) |
| `EnvironmentValues::current_environment()` / `Environment<Key>()` | `EnvironmentValues::current()`, `Environment::<K>::new()` |
| exceptions (`throws UsageError`) | panics, or `Result<_, Error>` from constructors/`run`; `notify` panics without an app bundle |
| `App::window(desc, ContentFunction)` | `App::window(&self, desc, impl IntoView)`; `App::run() -> Result<ExitCode, Error>`; Rust-only `run_code()`, `handle() -> AppHandle`, `post_to_ui_thread` needs `Send` |
| `Platform::windows/macos/linux/mikros` | `Platform::Windows/MacOS/Linux/Mikros`, plus `Unknown(i32)` |
| `ELG_MODULE(Root)` / `ELG_MODULE(nullptr)` | `#[preview(root)]` / no root; `HotReload::new(path).build_command(..).watch([..]).show(..).app_hash(..)` exists only with cargo feature `dynamic` (`cargo elegance preview` sets `preview` = `dynamic`) |
| Rust-only: `asset!`, `layout_hash!`, `DotValues`, `#[component]`, `#[elg_builder]`, `elg!`, `State::always_notify`, `Binding::read_only`, `Sound::load_data`, `VideoPlayer::file`, `WindowDesc::new(..).size(..)` builders |

## 4. Docs vs code inconsistencies (Rust side)
Compile-checked Rust fences: I extracted every identifier in all ```rust blocks of guides/start and found no name missing
from elegance-rs except user-defined ones, so snippets are consistent. Inline Rust-only prose names also all exist
(`Mica`, `AppCode::Body`, `Spacer().min_length`, `is_alive()`, `VGridLayout`, ...). Real inconsistencies:
1. docs start/installation.mdx ("Shipping an app", ~l.89): "The native library has to travel with your executable: keep it
   in the same folder" and step 4 "downloads Elegance's native library (`elegance_c`)... copies it next to your executable".
   Code/README/docs/building.md: the prebuilt `elegance_c` is a STATIC archive linked into the exe; only AccessKit's shared
   lib (and Dawn's dxcompiler/dxil on Windows if present) is copied. Prose is stale.
2. docs start/installation.mdx l.106 links `elegance-rs#using-it` for "environment variables that control where Elegance
   comes from": README has no `using-it` heading and no env-var list; they are in docs/building.md
   (`ELEGANCE_PREBUILT`, `_NATIVE_DIR`, `_PREBUILT_URL`, `_ROOT`, `_BUILD_DIR`, `_SHARED_BUILD_DIR`, `_GIT_REF`, `_AUTO_BUILD`,
   `_VCPKG_DIR`).
3. Minimum Rust: README badge/Requirements "Rust 1.88+"; elegance-wgpu/README "Rust 1.93+"; docs "a recent stable toolchain
   (edition 2024)"; no Cargo.toml (workspace, elegance, sys, macros, wgpu, cargo-elegance) declares `rust-version`.
   Code uses let-chains (`if let .. && let ..`, edition 2024) in elegance/src/error.rs and macros, which is what needs 1.88.
   CI uses `dtolnay/rust-toolchain@stable`.
4. README prebuilt table: Windows aarch64 and Linux aarch64 listed; "Ships beside the executable" row says only
   "Linux x86_64 | libaccesskit.so". On Linux ARM64 AccessKit is built from source by CI (c-api.yml l.117) so the aarch64 Linux
   package's shipped libs are unstated. CI of elegance-rs tests only Linux x86_64 (ubuntu-22.04), macos-15, windows-2025.
5. Docs (windows.mdx, outside this repo) may list platforms Windows/macOS/Linux only; the worktree C API/C++ also target Mikros (section 1); `Platform`
   in docs (windows.mdx l.153) has 3 variants.
6. docs guides/grids.mdx l.143: `resolve_grid_tracks` is mentioned for C++ only; Rust also exports `resolve_grid_tracks` and
   `grid_tracks_length`.
7. docs guides/animation.mdx l.124-127: `Animation::timing_curve(x1,y1,x2,y2,duration)` prose is inside a cpp-only block,
   yet Rust has it (animation.rs l.67) and a Rust snippet uses it (l.219).
8. docs guides/debugging.mdx l.211: GPU images are noted for C++ only, but Rust has `GpuImage`/`elegance-wgpu` (README
   documents it; no guide page for either language; README Vulkan/D3D12 status "not yet tested on hardware").
9. docs guides/hot-reload.mdx Rust section covers only `#[preview]` + `cargo elegance preview`. Not documented anywhere in
   docs: `HotReload` (needs feature `dynamic`), `#[preview(root, app_hash = ..)]`, `layout_hash!`, `cargo elegance
   build-module`, `HotReloadShow`. README also omits them.
10. cargo-elegance: `USAGE` string and doc header list `-p/--package --bin --manifest-path --release --features --watch`;
    parser also accepts `-F` and `--name=value`, and commands `build-module` and `help`/`--help`/`-h`. README/docs only
    show `preview` and `--root`. `--watch` default folders: `src` and `assets` (only if they exist), matches docs.
    The host error text tells users to `cmake --build build/dev-release --target elegance-preview` and mentions
    `ELEGANCE_SHARED_BUILD_DIR`, matching docs/building.md; docs hot-reload says previews build Elegance from source
    (true; prebuilt static package is never used with `dynamic`).
11. README: "`State`, `Binding`, and `Computed` are `Copy` handles" is correct (Copy impls exist); `Signal` is `Clone` only
    (Rc), not mentioned. README says `ForEach(xs, id: ...)`; code OK.
12. README/Cargo: workspace version 0.2.2, `elegance-rev` pins db0b10e5be25... ; if the worktree's C API (ABI 4, adds
    typed named keys, Mikros platform) is newer than the pin used for the prebuilt download, `App::new()` fails with an
    ABI mismatch message (app.rs l.382). Not verified which commit the pin is.

## 5. Verification notes / unverified
- I did not run builds. "No Rust counterpart" was decided by name, enum-variant and signature greps across elegance/src.
- `/workspaces/mikros/elegance` (the checkout named in the brief) is OLDER than the worktree: it lacks `Platform::mikros`,
  `get_named/set_named/value_named`, `elg-c/elegance.h`. The Rust sys bindings match the worktree. Use the worktree as truth.
- Unverified: whether Rust `ffi!` converts every `PlatformError` to a panic for `()`-returning calls on Mikros; whether
  `elg_binding_*_free` leaks (maybe freed by closure drops).
