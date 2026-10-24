//! Raw bindings to Elegance's C++ API through the C shim in `shim/`. Use the `elegance` crate;
//! everything here is unsafe and mirrors `shim/*.cpp` one to one.
//!
//! Conventions (see `shim/shim.h`): pointer arguments named `take_*`, and arrays of view pointers,
//! are consumed; other pointers are borrowed; returned pointers are owned by the caller. A C++
//! exception becomes a pending error, read with [`elg_take_error`].

#![allow(non_camel_case_types, clippy::missing_safety_doc)]

use std::ffi::{c_char, c_void};
use std::marker::{PhantomData, PhantomPinned};

macro_rules! opaque {
    ($($(#[$m:meta])* $name:ident),* $(,)?) => {$(
        $(#[$m])*
        #[repr(C)]
        pub struct $name {
            _data: [u8; 0],
            _marker: PhantomData<(*mut u8, PhantomPinned)>,
        }
    )*};
}

pub mod external_gpu_image;

opaque!(
    /// `elg::View`
    View,
    /// `elg::Text`
    Text,
    /// `elg::Shape`
    Shape,
    /// `elg::Path`
    Path,
    /// `elg::Animation`
    Animation,
    /// `elg::Transition`
    Transition,
    /// `elg::EnvironmentValues`
    EnvironmentValues,
    /// `elg::WindowHandle`
    WindowHandle,
    /// `elg::App`
    App,
    /// `elg::Binding<bool>`
    BindingBool,
    /// `elg::Binding<float>`
    BindingF32,
    /// `elg::Binding<int>`
    BindingI32,
    /// `elg::Binding<std::string>`
    BindingString,
    /// `elg::ScrollPosition`
    ScrollPosition,
    /// `elg::GraphicsContext`
    GraphicsContext,
    /// `std::shared_ptr<const elg::ButtonStyle>`
    ButtonStyle,
    /// `std::shared_ptr<const elg::ToggleStyle>`
    ToggleStyle,
    /// `std::shared_ptr<const elg::TextFieldStyle>`
    TextFieldStyle,
    /// `elg::ButtonConfiguration`
    ButtonConfiguration,
    /// `elg::ToggleConfiguration`
    ToggleConfiguration,
    /// `elg::TextFieldConfiguration`
    TextFieldConfiguration,
    /// `elg::LayoutCache`
    LayoutCache,
    /// `elg::Effects`
    Effects,
    /// `elg::Sound`
    Sound,
    /// `elg::AudioPlayer`
    AudioPlayer,
    /// `elg::AudioStream`
    AudioStream,
    /// `elg::AudioRecorder`
    AudioRecorder,
    /// `elg::Camera`
    Camera,
    /// `elg::VideoPlayer`
    VideoPlayer,
    /// `elg::GpuImage`
    GpuImage,
    /// `elg::VideoFeed`
    VideoFeed,
    /// `elg::http::Task`
    HttpTask,
    /// `elg::http::Fetch`
    HttpFetch,
    /// `elg::http::WebSocket`
    WebSocket,
    /// `elg::TrayIcon`
    TrayIcon,
    /// `std::string` (a closure's string result)
    CppString,
    /// `std::vector<elg::TextSpan>`
    TextSpans,
    /// `std::vector<elg::detail::ForEachItem>`
    ForEachItems,
    /// `std::vector<std::string>`
    Strings,
    /// `elg::Font` (a closure's font result)
    Font,
    /// `elg::ShapeStyle` (a closure's style result)
    ShapeStyle,
);

/// `elg::LayoutChild`: one pointer, so a C++ `span<const LayoutChild>` is a slice of these.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LayoutChild {
    pub element: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgStr {
    pub ptr: *const c_char,
    pub len: usize,
}

impl ElgStr {
    pub fn new(s: &str) -> Self {
        Self { ptr: s.as_ptr().cast(), len: s.len() }
    }
    pub const EMPTY: Self = Self { ptr: std::ptr::null(), len: 0 };
    /// # Safety
    /// The pointer must be valid UTF-8 for `len` bytes, for `'a`.
    pub unsafe fn as_str<'a>(&self) -> &'a str {
        if self.ptr.is_null() {
            ""
        } else {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.ptr.cast(), self.len))
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgBytes {
    pub ptr: *const u8,
    pub len: usize,
}

impl ElgBytes {
    pub fn new(b: &[u8]) -> Self {
        Self { ptr: b.as_ptr(), len: b.len() }
    }
    pub const EMPTY: Self = Self { ptr: std::ptr::null(), len: 0 };
    /// # Safety
    /// The pointer must be valid for `len` bytes, for `'a`.
    pub unsafe fn as_slice<'a>(&self) -> &'a [u8] {
        if self.ptr.is_null() { &[] } else { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

pub type ElgCallFn = unsafe extern "C" fn(data: *mut c_void, args: *mut c_void, ret: *mut c_void) -> bool;
pub type ElgDropFn = unsafe extern "C" fn(data: *mut c_void);
pub type ElgStrSink = unsafe extern "C" fn(ctx: *mut c_void, ptr: *const c_char, len: usize);

#[repr(C)]
pub struct ElgClosure {
    pub data: *mut c_void,
    pub call: Option<ElgCallFn>,
    pub drop: Option<ElgDropFn>,
}

impl ElgClosure {
    pub const NONE: Self = Self { data: std::ptr::null_mut(), call: None, drop: None };
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgSize {
    pub width: f32,
    pub height: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgPoint {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgFont {
    pub family: ElgStr,
    pub size: f32,
    pub weight: i32,
    pub italic: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgGradientStop {
    pub color: ElgColor,
    pub location: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgShapeStyle {
    pub kind: i32,
    pub color: ElgColor,
    pub stops: *const ElgGradientStop,
    pub stop_count: usize,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub r0: f32,
    pub r1: f32,
    pub blur: f32,
    pub saturation: f32,
    pub noise: f32,
    pub has_dark_tint: bool,
    pub dark_tint: ElgColor,
}

#[repr(C)]
pub struct ElgProp {
    pub kind: i32,
    pub theme: i32,
    pub value: *const c_void,
    pub fn_: ElgClosure,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgProposedSize {
    pub has_width: bool,
    pub width: f32,
    pub has_height: bool,
    pub height: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgOptLen {
    pub has: bool,
    pub value: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgAlignment {
    pub horizontal: i32,
    pub vertical: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgKeyEvent {
    pub kind: i32,
    pub key: i32,
    pub modifiers: u32,
    pub repeat: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgPointerEvent {
    pub kind: i32,
    pub location: ElgPoint,
    pub button: i32,
    pub click_count: i32,
    pub modifiers: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgScrollEvent {
    pub location: ElgPoint,
    pub dx: f32,
    pub dy: f32,
    pub precise: bool,
    pub modifiers: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgTextInputEvent {
    pub kind: i32,
    pub text: ElgStr,
    pub cursor: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgEventResult {
    pub consumed: bool,
    pub redraw: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgHoverDelay {
    pub enter: f64,
    pub exit: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgOptHoverDelay {
    pub has: bool,
    pub delay: ElgHoverDelay,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgShadow {
    pub color: ElgColor,
    pub radius: f32,
    pub x: f32,
    pub y: f32,
    pub spread: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Borrowed same-device Dawn C handles, not Rust `wgpu` objects or cross-device handles.
/// Return all fields unchanged to end/cancel; never release/destroy the handles.
pub struct ElgGpuFrame {
    pub buffer: u32,
    pub width: u32,
    pub height: u32,
    pub device: *mut c_void,
    pub queue: *mut c_void,
    pub texture: *mut c_void,
    pub sequence: u64,
}

#[cfg(test)]
mod gpu_frame_tests {
    use super::ElgGpuFrame;
    use std::ffi::c_void;
    use std::mem::{align_of, offset_of, size_of};

    #[test]
    fn c_layout() {
        fn aligned(size: usize, alignment: usize) -> usize {
            size.next_multiple_of(alignment)
        }
        let pointer = size_of::<*mut c_void>();
        let device = aligned(3 * size_of::<u32>(), align_of::<*mut c_void>());
        let sequence = aligned(device + 3 * pointer, align_of::<u64>());
        assert_eq!(offset_of!(ElgGpuFrame, buffer), 0);
        assert_eq!(offset_of!(ElgGpuFrame, width), 4);
        assert_eq!(offset_of!(ElgGpuFrame, height), 8);
        assert_eq!(offset_of!(ElgGpuFrame, device), device);
        assert_eq!(offset_of!(ElgGpuFrame, queue), device + pointer);
        assert_eq!(offset_of!(ElgGpuFrame, texture), device + 2 * pointer);
        assert_eq!(offset_of!(ElgGpuFrame, sequence), sequence);
        let alignment = align_of::<u32>().max(align_of::<*mut c_void>()).max(align_of::<u64>());
        assert_eq!(align_of::<ElgGpuFrame>(), alignment);
        assert_eq!(size_of::<ElgGpuFrame>(), aligned(sequence + size_of::<u64>(), alignment));
    }

    #[test]
    fn default_has_no_handles_or_token() {
        let frame = ElgGpuFrame::default();
        assert_eq!((frame.buffer, frame.width, frame.height, frame.sequence), (0, 0, 0, 0));
        assert!(frame.device.is_null());
        assert!(frame.queue.is_null());
        assert!(frame.texture.is_null());
    }
}

impl Default for ElgGpuFrame {
    fn default() -> Self {
        Self {
            buffer: 0,
            width: 0,
            height: 0,
            device: std::ptr::null_mut(),
            queue: std::ptr::null_mut(),
            texture: std::ptr::null_mut(),
            sequence: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgStrokeStyle {
    pub width: f32,
    pub cap: i32,
    pub join: i32,
    pub miter_limit: f32,
    pub dash: *const f32,
    pub dash_count: usize,
    pub dash_phase: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgRunStyle {
    pub has_weight: bool,
    pub weight: i32,
    pub italic: bool,
    pub family: ElgStr,
    pub has_color: bool,
    pub color: ElgColor,
    pub has_background: bool,
    pub background: ElgColor,
    pub underline: bool,
    pub strikethrough: bool,
    pub keep_together: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgImageSource {
    pub system_name: ElgStr,
    pub file: ElgStr,
    pub data: ElgBytes,
    pub scale: f32,
}

#[repr(C)]
pub struct ElgWindowDesc {
    pub title: ElgStr,
    pub width: i32,
    pub height: i32,
    pub resizable: bool,
    pub has_min_size: bool,
    pub min_size: ElgSize,
    pub has_background: bool,
    pub background: ElgColor,
    pub title_bar: i32,
    pub transparent: bool,
    pub opacity: f32,
    pub fullscreen: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgPose {
    pub opacity: f32,
    pub scale: f32,
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
pub struct ElgTextFieldOptions {
    pub on_submit: ElgClosure,
    pub max_lines: i32,
    pub styler: ElgClosure,
    pub on_key: ElgClosure,
}

#[repr(C)]
pub struct ElgForEachItem {
    pub key_is_string: bool,
    pub int_key: i64,
    pub string_key: ElgStr,
    pub value: ElgClosure,
    pub equals: unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool,
    pub view: ElgClosure,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgColumnWidth {
    pub min: f32,
    pub ideal: f32,
    pub max: f32,
}

#[repr(C)]
pub struct ElgTextRun {
    pub text: ElgStr,
    pub style: ElgRunStyle,
    pub on_tap: ElgClosure,
    pub take_attachment: *mut View,
    pub attachment_size: f32,
}

#[repr(C)]
pub struct ElgPolyline {
    pub points: *const ElgPoint,
    pub count: usize,
    pub closed: bool,
}

#[repr(C)]
pub struct ElgAsyncImagePhase {
    pub kind: i32,
    pub image: *const View,
    pub error: ElgStr,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgIconFont {
    pub file: ElgStr,
    pub data: ElgBytes,
}

#[repr(C)]
pub struct ElgIconWeight {
    pub weight: i32,
    pub font: ElgIconFont,
}

#[repr(C)]
pub struct ElgIconVariant {
    pub name: ElgStr,
    pub font: ElgIconFont,
    pub names: ElgStr,
    pub names_data: ElgStr,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgFrameOptions {
    pub width: ElgOptLen,
    pub height: ElgOptLen,
    pub min_width: ElgOptLen,
    pub ideal_width: ElgOptLen,
    pub max_width: ElgOptLen,
    pub min_height: ElgOptLen,
    pub ideal_height: ElgOptLen,
    pub max_height: ElgOptLen,
    pub alignment: ElgAlignment,
}

#[repr(C)]
pub struct ElgDropFiles {
    pub paths: *const ElgStr,
    pub count: usize,
    pub location: ElgPoint,
}

#[repr(C)]
pub struct ElgDropText {
    pub text: ElgStr,
    pub location: ElgPoint,
}

#[repr(C)]
pub struct ElgLayoutCall {
    pub op: i32,
    pub proposal: ElgProposedSize,
    pub bounds: ElgRect,
    pub children: *const LayoutChild,
    pub count: usize,
    pub cache: *mut LayoutCache,
}

#[repr(C)]
pub struct ElgLayoutModifierCall {
    pub op: i32,
    pub proposal: ElgProposedSize,
    pub bounds: ElgRect,
    pub content: *const LayoutChild,
}

#[repr(C)]
pub struct ElgEventCall {
    pub op: i32,
    pub pointer: ElgPointerEvent,
    pub frame: ElgRect,
    pub flag: bool,
    pub scroll: ElgScrollEvent,
    pub key: ElgKeyEvent,
    pub text: ElgTextInputEvent,
}

#[repr(C)]
pub struct ElgDevice {
    pub id: ElgStr,
    pub name: ElgStr,
    pub is_default: bool,
}

#[repr(C)]
pub struct ElgRenderBuffer {
    pub samples: *mut f32,
    pub count: usize,
}

#[repr(C)]
pub struct ElgSamples {
    pub samples: *const f32,
    pub count: usize,
}

#[repr(C)]
pub struct ElgHeader {
    pub name: ElgStr,
    pub value: ElgStr,
}

#[repr(C)]
pub struct ElgRequest {
    pub url: ElgStr,
    pub method: ElgStr,
    pub headers: *const ElgHeader,
    pub header_count: usize,
    pub body: ElgBytes,
    pub timeout_ms: i64,
}

#[repr(C)]
pub struct ElgResponse {
    pub ok: bool,
    pub status: i32,
    pub headers: *const ElgHeader,
    pub header_count: usize,
    pub body: ElgBytes,
    pub error: ElgStr,
}

#[repr(C)]
pub struct ElgWebSocketEvents {
    pub on_open: ElgClosure,
    pub on_message: ElgClosure,
    pub on_binary: ElgClosure,
    pub on_close: ElgClosure,
}

#[repr(C)]
pub struct ElgWebSocketClose {
    pub code: i32,
    pub reason: ElgStr,
}

#[repr(C)]
pub struct ElgFileFilter {
    pub name: ElgStr,
    pub extensions: *const ElgStr,
    pub extension_count: usize,
}

#[repr(C)]
pub struct ElgPaths {
    pub paths: *const ElgStr,
    pub count: usize,
}

#[repr(C)]
pub struct ElgTrayMenuItem {
    pub title: ElgStr,
    pub action: ElgClosure,
    pub checked: i32,
    pub toggled: ElgClosure,
    pub enabled: bool,
    pub submenu: *const ElgTrayMenuItem,
    pub submenu_count: usize,
}

#[repr(C)]
pub struct ElgPreviewDesc {
    pub name: ElgStr,
    pub file: ElgStr,
    pub line: i32,
}

/// Builds view `index` of a views module into `ret` (-1: the root view); false if it panicked.
pub type ElgModuleBuild = unsafe extern "C" fn(index: i32, ret: *mut View) -> bool;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgGridItem {
    pub kind: i32,
    pub minimum: f32,
    pub maximum: f32,
    pub spacing: ElgOptLen,
    pub has_alignment: bool,
    pub alignment: ElgAlignment,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgGridTrack {
    pub size: f32,
    pub spacing: f32,
    pub has_alignment: bool,
    pub alignment: ElgAlignment,
}

#[repr(C)]
pub struct ElgGridDesc {
    pub kind: i32,
    pub items: *const ElgGridItem,
    pub item_count: usize,
    pub alignment: ElgAlignment,
    pub spacing_a: ElgOptLen,
    pub spacing_b: ElgOptLen,
}

/// `elg::Theme`: light then dark tokens, 12 colors each.
pub type ElgTheme = [ElgColor; 24];

extern "C" {
    // --- core.cpp ---
    pub fn elg_take_error(sink: ElgStrSink, ctx: *mut c_void) -> i32;
    pub fn elg_error_pending() -> bool;
    pub fn elg_string_assign(target: *mut CppString, value: ElgStr);
    pub fn elg_string_view(s: *const CppString) -> ElgStr;

    pub fn elg_view_empty() -> *mut View;
    pub fn elg_view_clone(view: *const View) -> *mut View;
    pub fn elg_view_free(view: *mut View);
    pub fn elg_view_move_into(target: *mut View, take_view: *mut View);
    pub fn elg_view_component(body: ElgClosure) -> *mut View;

    pub fn elg_signal_new(owner: ElgClosure) -> *mut c_void;
    pub fn elg_computed_new(compute: ElgClosure) -> *mut c_void;
    pub fn elg_node_track(node: *mut c_void);
    pub fn elg_node_notify(node: *mut c_void);
    pub fn elg_check_ui_thread();
    pub fn elg_effect(function: ElgClosure);
    pub fn elg_untrack(function: ElgClosure);
    pub fn elg_batch(function: ElgClosure);
    pub fn elg_with_animation(animation: *const Animation, changes: ElgClosure);

    pub fn elg_binding_bool(get: ElgClosure, set: ElgClosure) -> *mut BindingBool;
    pub fn elg_binding_f32(get: ElgClosure, set: ElgClosure) -> *mut BindingF32;
    pub fn elg_binding_i32(get: ElgClosure, set: ElgClosure) -> *mut BindingI32;
    pub fn elg_binding_string(get: ElgClosure, set: ElgClosure) -> *mut BindingString;
    pub fn elg_binding_f32_get(binding: *const BindingF32) -> f32;
    pub fn elg_binding_f32_set(binding: *const BindingF32, value: f32);
    pub fn elg_binding_f32_free(binding: *mut BindingF32);

    pub fn elg_env_current() -> *mut EnvironmentValues;
    pub fn elg_env_clone(env: *const EnvironmentValues) -> *mut EnvironmentValues;
    pub fn elg_env_free(env: *mut EnvironmentValues);
    pub fn elg_env_color(env: *const EnvironmentValues, token: i32) -> ElgColor;
    pub fn elg_env_font(
        env: *const EnvironmentValues,
        family: ElgStrSink,
        ctx: *mut c_void,
        size: *mut f32,
        weight: *mut i32,
        italic: *mut bool,
    );
    pub fn elg_env_color_scheme(env: *const EnvironmentValues) -> i32;
    pub fn elg_env_theme(env: *const EnvironmentValues, result: *mut ElgTheme);
    pub fn elg_env_foreground(env: *const EnvironmentValues) -> ElgColor;
    pub fn elg_env_tint(env: *const EnvironmentValues) -> ElgColor;
    pub fn elg_env_disabled(env: *const EnvironmentValues) -> bool;
    pub fn elg_env_hover_delay(env: *const EnvironmentValues) -> ElgHoverDelay;
    pub fn elg_env_window(env: *const EnvironmentValues) -> *mut WindowHandle;
    pub fn elg_env_button_style(env: *const EnvironmentValues) -> *mut ButtonStyle;
    pub fn elg_env_toggle_style(env: *const EnvironmentValues) -> *mut ToggleStyle;
    pub fn elg_env_text_field_style(env: *const EnvironmentValues) -> *mut TextFieldStyle;
    pub fn elg_env_rust_values(env: *const EnvironmentValues) -> *mut c_void;
    pub fn elg_env_set_font(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_color_scheme(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_theme(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_foreground(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_tint(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_disabled(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_hover_delay(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_window(env: *mut EnvironmentValues, window: *const WindowHandle);
    pub fn elg_env_set_button_style(env: *mut EnvironmentValues, style: *const ButtonStyle);
    pub fn elg_env_set_toggle_style(env: *mut EnvironmentValues, style: *const ToggleStyle);
    pub fn elg_env_set_text_field_style(env: *mut EnvironmentValues, style: *const TextFieldStyle);
    pub fn elg_env_set_rust_values(env: *mut EnvironmentValues, values: ElgClosure);

    pub fn elg_window_clone(window: *const WindowHandle) -> *mut WindowHandle;
    pub fn elg_window_free(window: *mut WindowHandle);
    pub fn elg_window_eq(a: *const WindowHandle, b: *const WindowHandle) -> bool;
    pub fn elg_window_close(window: *const WindowHandle);
    pub fn elg_window_minimize(window: *const WindowHandle);
    pub fn elg_window_toggle_maximized(window: *const WindowHandle);
    pub fn elg_window_is_maximized(window: *const WindowHandle) -> bool;
    pub fn elg_window_is_active(window: *const WindowHandle) -> bool;
    pub fn elg_window_controls(style: i32, minimize: bool, maximize: bool) -> *mut View;

    pub fn elg_app_new() -> *mut App;
    pub fn elg_app_free(app: *mut App);
    pub fn elg_app_window(app: *mut App, desc: *const ElgWindowDesc, content: ElgClosure);
    pub fn elg_app_run(app: *mut App) -> i32;
    pub fn elg_app_quit(app: *mut App);
    pub fn elg_post_to_ui_thread(task: ElgClosure);
    pub fn elg_version() -> *const c_char;
    pub fn elg_set_cursor(cursor: i32);

    pub fn elg_animation_linear(duration: f64) -> *mut Animation;
    pub fn elg_animation_ease_in(duration: f64) -> *mut Animation;
    pub fn elg_animation_ease_out(duration: f64) -> *mut Animation;
    pub fn elg_animation_ease_in_out(duration: f64) -> *mut Animation;
    pub fn elg_animation_spring(response: f64, damping_fraction: f64) -> *mut Animation;
    pub fn elg_animation_smooth(duration: f64) -> *mut Animation;
    pub fn elg_animation_custom(curve: ElgClosure, duration: f64) -> *mut Animation;
    pub fn elg_animation_delay(animation: *const Animation, seconds: f64) -> *mut Animation;
    pub fn elg_animation_progress(animation: *const Animation, seconds: f64) -> f64;
    pub fn elg_animation_finished(animation: *const Animation, seconds: f64) -> bool;
    pub fn elg_animation_clone(animation: *const Animation) -> *mut Animation;
    pub fn elg_animation_free(animation: *mut Animation);

    pub fn elg_transition_opacity() -> *mut Transition;
    pub fn elg_transition_identity() -> *mut Transition;
    pub fn elg_transition_scale(from: f32) -> *mut Transition;
    pub fn elg_transition_offset(x: f32, y: f32) -> *mut Transition;
    pub fn elg_transition_move(edge: u32) -> *mut Transition;
    pub fn elg_transition_slide() -> *mut Transition;
    pub fn elg_transition_asymmetric(insertion: *const Transition, removal: *const Transition) -> *mut Transition;
    pub fn elg_transition_combined(a: *const Transition, b: *const Transition) -> *mut Transition;
    pub fn elg_transition_pose(transition: *const Transition, progress: f32, removal: bool, size: ElgSize) -> ElgPose;
    pub fn elg_transition_clone(transition: *const Transition) -> *mut Transition;
    pub fn elg_transition_free(transition: *mut Transition);

    // --- views.cpp ---
    pub fn elg_vstack(alignment: i32, spacing: ElgOptLen, children: *mut *mut View, count: usize) -> *mut View;
    pub fn elg_hstack(alignment: i32, spacing: ElgOptLen, children: *mut *mut View, count: usize) -> *mut View;
    pub fn elg_zstack(alignment: ElgAlignment, children: *mut *mut View, count: usize) -> *mut View;
    pub fn elg_spacer(min_length: ElgOptLen) -> *mut View;
    pub fn elg_empty_view() -> *mut View;
    pub fn elg_aligned_position(container: ElgRect, content: ElgSize, alignment: ElgAlignment) -> ElgPoint;
    pub fn elg_stack_layout_size(
        kind: i32,
        alignment: ElgAlignment,
        spacing: ElgOptLen,
        proposal: ElgProposedSize,
        children: *const LayoutChild,
        count: usize,
    ) -> ElgSize;
    pub fn elg_stack_layout_place(
        kind: i32,
        alignment: ElgAlignment,
        spacing: ElgOptLen,
        bounds: ElgRect,
        proposal: ElgProposedSize,
        children: *const LayoutChild,
        count: usize,
    );

    pub fn elg_button(take_label: *mut View, on_click: ElgClosure) -> *mut View;
    pub fn elg_button_title(title: ElgStr, on_click: ElgClosure) -> *mut View;
    pub fn elg_text_spans_push(
        spans: *mut TextSpans,
        begin: usize,
        end: usize,
        style: *const ElgRunStyle,
        label: *const ElgStr,
    );
    pub fn elg_text_field(
        placeholder: ElgStr,
        take_text: *mut BindingString,
        options: *const ElgTextFieldOptions,
        secure: bool,
    ) -> *mut View;
    pub fn elg_slider(
        take_value: *mut BindingF32,
        min: f32,
        max: f32,
        step: f32,
        on_editing_changed: ElgClosure,
    ) -> *mut View;
    pub fn elg_stepper(take_label: *mut View, take_value: *mut BindingI32, min: i32, max: i32, step: i32) -> *mut View;
    pub fn elg_stepper_title(title: ElgStr, take_value: *mut BindingI32, min: i32, max: i32, step: i32) -> *mut View;
    pub fn elg_progress(fraction: ElgProp) -> *mut View;
    pub fn elg_progress_indeterminate() -> *mut View;
    pub fn elg_picker(
        labels: *const ElgStr,
        count: usize,
        selected: ElgClosure,
        select: ElgClosure,
        style: i32,
    ) -> *mut View;
    pub fn elg_toggle(take_label: *mut View, take_is_on: *mut BindingBool) -> *mut View;
    pub fn elg_toggle_title(title: ElgStr, take_is_on: *mut BindingBool) -> *mut View;

    pub fn elg_if(condition: ElgProp, take_then: *mut View, take_else: *mut View) -> *mut View;
    pub fn elg_for_each(items: ElgClosure) -> *mut View;
    pub fn elg_for_each_items_push(target: *mut ForEachItems, items: *const ElgForEachItem, count: usize);

    pub fn elg_scroll_position_new() -> *mut ScrollPosition;
    pub fn elg_scroll_position_clone(position: *const ScrollPosition) -> *mut ScrollPosition;
    pub fn elg_scroll_position_free(position: *mut ScrollPosition);
    pub fn elg_scroll_position_to_top(position: *const ScrollPosition, animated: bool);
    pub fn elg_scroll_position_to_end(position: *const ScrollPosition, animated: bool);
    pub fn elg_scroll_position_to_int(position: *const ScrollPosition, key: i64);
    pub fn elg_scroll_position_to_string(position: *const ScrollPosition, key: ElgStr);
    pub fn elg_scroll_view(
        take_content: *mut View,
        axes: i32,
        shows_indicators: bool,
        position: *const ScrollPosition,
        sticks_to_end: bool,
    ) -> *mut View;
    pub fn elg_lazy_stack(vertical: bool, alignment: i32, spacing: ElgOptLen, take_for_each: *mut View) -> *mut View;
    pub fn elg_list(take_for_each: *mut View, position: *const ScrollPosition) -> *mut View;

    pub fn elg_divider() -> *mut View;
    pub fn elg_menu_item(
        title: ElgStr,
        on_choose: ElgClosure,
        shortcut: ElgStr,
        destructive: bool,
        checked: *const ElgProp,
        image: *const ElgImageSource,
    ) -> *mut View;
    pub fn elg_submenu(title: ElgStr, menu: ElgClosure) -> *mut View;
    pub fn elg_menu(items: *mut *mut View, count: usize) -> *mut View;
    pub fn elg_menu_button(take_label: *mut View, menu: ElgClosure, opens: i32, shows_indicator: bool) -> *mut View;

    pub fn elg_split_view(axis: i32, panes: *mut *mut View, count: usize) -> *mut View;
    pub fn elg_navigation_split_view(
        take_sidebar: *mut View,
        take_content: *mut View,
        take_detail: *mut View,
        take_sidebar_visible: *mut BindingBool,
        sidebar_width: ElgColumnWidth,
        content_width: ElgColumnWidth,
    ) -> *mut View;
    pub fn elg_navigation_stack(
        take_root: *mut View,
        pages: ElgClosure,
        titles: ElgClosure,
        pop: ElgClosure,
        shows_bar: bool,
    ) -> *mut View;
    pub fn elg_strings_push(target: *mut Strings, value: ElgStr);

    pub fn elg_text(content: ElgProp) -> *mut Text;
    pub fn elg_text_clone(text: *const Text) -> *mut Text;
    pub fn elg_text_free(text: *mut Text);
    pub fn elg_text_to_view(text: *const Text) -> *mut View;
    pub fn elg_text_font(text: *const Text, font: *const ElgFont) -> *mut Text;
    pub fn elg_text_font_weight(text: *const Text, weight: i32) -> *mut Text;
    pub fn elg_text_bold(text: *const Text) -> *mut Text;
    pub fn elg_text_foreground_style(text: *const Text, color: ElgProp) -> *mut Text;
    pub fn elg_text_line_limit(text: *const Text, lines: i32) -> *mut Text;
    pub fn elg_text_multiline_alignment(text: *const Text, alignment: i32) -> *mut Text;
    pub fn elg_styled_text(runs: *const ElgTextRun, count: usize) -> *mut View;

    pub fn elg_shape_clone(shape: *const Shape) -> *mut Shape;
    pub fn elg_shape_free(shape: *mut Shape);
    pub fn elg_shape_to_view(shape: *const Shape) -> *mut View;
    pub fn elg_rectangle() -> *mut Shape;
    pub fn elg_rounded_rectangle(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> *mut Shape;
    pub fn elg_capsule() -> *mut Shape;
    pub fn elg_circle() -> *mut Shape;
    pub fn elg_path_shape(path: *const Path, view_box: *const ElgRect, fill_rule: i32) -> *mut Shape;
    pub fn elg_custom_shape(path: ElgClosure, fill_rule: i32) -> *mut Shape;
    pub fn elg_shape_fill(shape: *const Shape, style: ElgProp) -> *mut Shape;
    pub fn elg_shape_stroke(shape: *const Shape, color: ElgProp, style: *const ElgStrokeStyle) -> *mut Shape;
    pub fn elg_shape_stroke_border(shape: *const Shape, color: ElgProp, line_width: f32) -> *mut Shape;

    pub fn elg_path_new() -> *mut Path;
    pub fn elg_path_clone(path: *const Path) -> *mut Path;
    pub fn elg_path_free(path: *mut Path);
    pub fn elg_path_assign(target: *mut Path, source: *const Path);
    pub fn elg_path_eq(a: *const Path, b: *const Path) -> bool;
    pub fn elg_path_move_to(path: *mut Path, p: ElgPoint);
    pub fn elg_path_line_to(path: *mut Path, p: ElgPoint);
    pub fn elg_path_quad_to(path: *mut Path, control: ElgPoint, end: ElgPoint);
    pub fn elg_path_cubic_to(path: *mut Path, control1: ElgPoint, control2: ElgPoint, end: ElgPoint);
    pub fn elg_path_arc_to(
        path: *mut Path,
        rx: f32,
        ry: f32,
        rotation: f32,
        large_arc: bool,
        sweep: bool,
        end: ElgPoint,
    );
    pub fn elg_path_close(path: *mut Path);
    pub fn elg_path_add_rect(path: *mut Path, rect: ElgRect);
    pub fn elg_path_add_rounded_rect(path: *mut Path, rect: ElgRect, radius: f32);
    pub fn elg_path_add_ellipse(path: *mut Path, rect: ElgRect);
    pub fn elg_path_add_circle(path: *mut Path, center: ElgPoint, radius: f32);
    pub fn elg_path_add_path(path: *mut Path, other: *const Path);
    pub fn elg_path_svg(data: ElgStr) -> *mut Path;
    pub fn elg_path_empty(path: *const Path) -> bool;
    pub fn elg_path_bounds(path: *const Path) -> ElgRect;
    pub fn elg_path_contains(path: *const Path, p: ElgPoint, rule: i32) -> bool;
    pub fn elg_path_transformed(path: *const Path, sx: f32, sy: f32, dx: f32, dy: f32) -> *mut Path;
    pub fn elg_path_flatten(path: *const Path, tolerance: f32, polyline: ElgClosure);
    pub fn elg_path_stroked(path: *const Path, style: *const ElgStrokeStyle, tolerance: f32) -> *mut Path;

    pub fn elg_canvas(draw: ElgClosure) -> *mut View;
    pub fn elg_gc_size(gc: *const GraphicsContext) -> ElgSize;
    pub fn elg_gc_scale(gc: *const GraphicsContext) -> f32;
    pub fn elg_gc_time(gc: *const GraphicsContext) -> f64;
    pub fn elg_gc_request_frame(gc: *mut GraphicsContext);
    pub fn elg_gc_fill_rect(gc: *mut GraphicsContext, rect: ElgRect, color: ElgColor, radii: *const f32);
    pub fn elg_gc_stroke_rect(gc: *mut GraphicsContext, rect: ElgRect, color: ElgColor, width: f32, radii: *const f32);
    pub fn elg_gc_draw_rect(
        gc: *mut GraphicsContext,
        rect: ElgRect,
        radii: *const f32,
        fill: ElgColor,
        border: ElgColor,
        border_width: f32,
    );
    pub fn elg_gc_fill_path(gc: *mut GraphicsContext, path: *const Path, style: *const ElgShapeStyle, rule: i32);
    pub fn elg_gc_stroke_path(
        gc: *mut GraphicsContext,
        path: *const Path,
        color: ElgColor,
        style: *const ElgStrokeStyle,
    );
    pub fn elg_gc_push_clip(gc: *mut GraphicsContext, rect: ElgRect, radii: *const f32);
    pub fn elg_gc_pop_clip(gc: *mut GraphicsContext);

    pub fn elg_image(source: *const ElgImageSource) -> *mut View;
    pub fn elg_image_named(system_name: ElgStr) -> *mut View;
    pub fn elg_async_image(url: ElgStr, scale: f32, content_mode: i32) -> *mut View;
    pub fn elg_async_image_content(url: ElgStr, content: ElgClosure, scale: f32) -> *mut View;
    pub fn elg_label(title: ElgStr, icon: *const ElgImageSource) -> *mut View;
    pub fn elg_register_icon_set(
        name: ElgStr,
        font: ElgIconFont,
        weights: *const ElgIconWeight,
        weight_count: usize,
        variants: *const ElgIconVariant,
        variant_count: usize,
        names: ElgStr,
        names_data: ElgStr,
        replace: bool,
    );
    pub fn elg_set_default_icon_set(name: ElgStr);
    pub fn elg_register_font_file(family: ElgStr, file: ElgStr, weight: i32);
    pub fn elg_register_font_data(family: ElgStr, data: ElgBytes, weight: i32);
    pub fn elg_register_emoji_font(file: ElgStr);
    pub fn elg_font_assign(target: *mut Font, value: *const ElgFont);
    pub fn elg_shape_style_assign(target: *mut ShapeStyle, value: *const ElgShapeStyle);

    pub fn elg_mod_padding(v: *const View, top: f32, leading: f32, bottom: f32, trailing: f32) -> *mut View;
    pub fn elg_mod_frame(v: *const View, options: *const ElgFrameOptions) -> *mut View;
    pub fn elg_mod_frame_size(v: *const View, size: ElgProp, alignment: ElgAlignment) -> *mut View;
    pub fn elg_mod_fixed_size(v: *const View, horizontal: bool, vertical: bool) -> *mut View;
    pub fn elg_mod_aspect_ratio(v: *const View, has_ratio: bool, ratio: f32, mode: i32) -> *mut View;
    pub fn elg_mod_resizable(v: *const View) -> *mut View;
    pub fn elg_mod_layout_priority(v: *const View, priority: f32) -> *mut View;
    pub fn elg_mod_z_index(v: *const View, z: ElgProp) -> *mut View;
    pub fn elg_mod_offset(v: *const View, x: ElgProp, y: ElgProp) -> *mut View;
    pub fn elg_mod_background_view(v: *const View, take_background: *mut View, alignment: ElgAlignment) -> *mut View;
    pub fn elg_mod_background(v: *const View, style: ElgProp) -> *mut View;
    pub fn elg_mod_background_shape(v: *const View, style: ElgProp, shape: *const Shape) -> *mut View;
    pub fn elg_mod_overlay(v: *const View, take_overlay: *mut View, alignment: ElgAlignment) -> *mut View;
    pub fn elg_mod_border(v: *const View, color: ElgColor, width: f32) -> *mut View;
    pub fn elg_mod_clipped(v: *const View) -> *mut View;
    pub fn elg_mod_clip_shape(v: *const View, shape: *const Shape) -> *mut View;
    pub fn elg_mod_corner_radius(v: *const View, radius: f32) -> *mut View;
    pub fn elg_mod_opacity(v: *const View, opacity: ElgProp) -> *mut View;
    pub fn elg_mod_shadow(v: *const View, shadow: ElgShadow) -> *mut View;
    pub fn elg_mod_inner_shadow(v: *const View, shadow: ElgShadow) -> *mut View;
    pub fn elg_mod_blur(v: *const View, radius: ElgProp, opaque: bool) -> *mut View;
    pub fn elg_mod_color_filter(v: *const View, kind: i32, amount: ElgProp, color: ElgProp) -> *mut View;
    pub fn elg_mod_blend_mode(v: *const View, mode: i32) -> *mut View;
    pub fn elg_mod_compositing_group(v: *const View) -> *mut View;
    pub fn elg_mod_rotation_effect(v: *const View, degrees: ElgProp, anchor: ElgPoint) -> *mut View;
    pub fn elg_mod_scale_effect(v: *const View, x: ElgProp, y: *const ElgProp, anchor: ElgPoint) -> *mut View;
    pub fn elg_mod_mask(v: *const View, take_mask: *mut View, alignment: ElgAlignment) -> *mut View;
    pub fn elg_mod_mask_style(v: *const View, style: ElgProp) -> *mut View;
    pub fn elg_mod_content_shape(v: *const View, shape: *const Shape) -> *mut View;
    pub fn elg_mod_allows_hit_testing(v: *const View, enabled: bool) -> *mut View;
    pub fn elg_mod_on_tap_gesture(v: *const View, on_tap: ElgClosure, count: i32) -> *mut View;
    pub fn elg_mod_on_hover(v: *const View, on_hover: ElgClosure, delay: *const ElgHoverDelay) -> *mut View;
    pub fn elg_mod_on_drop(v: *const View, files: ElgClosure, text: ElgClosure, targeted: ElgClosure) -> *mut View;
    pub fn elg_mod_accessibility_label(v: *const View, label: ElgProp) -> *mut View;
    pub fn elg_mod_accessibility_value(v: *const View, value: ElgProp) -> *mut View;
    pub fn elg_mod_accessibility_hint(v: *const View, hint: ElgProp) -> *mut View;
    pub fn elg_mod_accessibility_hidden(v: *const View, hidden: bool) -> *mut View;
    pub fn elg_mod_accessibility_role(v: *const View, role: i32) -> *mut View;
    pub fn elg_mod_accessibility_element(v: *const View) -> *mut View;
    pub fn elg_mod_accessibility_action(v: *const View, on_action: ElgClosure) -> *mut View;
    pub fn elg_mod_hover_delay(v: *const View, delay: ElgHoverDelay) -> *mut View;
    pub fn elg_mod_window_drag_area(v: *const View, enabled: bool) -> *mut View;
    pub fn elg_mod_context_menu(v: *const View, menu: ElgClosure) -> *mut View;
    pub fn elg_mod_help(v: *const View, text: ElgStr) -> *mut View;
    pub fn elg_mod_popover(
        v: *const View,
        take_is_presented: *mut BindingBool,
        content: ElgClosure,
        opens: i32,
        padded: bool,
    ) -> *mut View;
    pub fn elg_mod_focusable(v: *const View, focusable: bool) -> *mut View;
    pub fn elg_mod_focused(v: *const View, take_is_focused: *mut BindingBool) -> *mut View;
    pub fn elg_mod_on_key_press(v: *const View, key: i32, on_press: ElgClosure) -> *mut View;
    pub fn elg_mod_on_key_event(v: *const View, on_key: ElgClosure) -> *mut View;
    pub fn elg_mod_font_weight(v: *const View, weight: i32) -> *mut View;
    pub fn elg_mod_font(v: *const View, font: ElgProp) -> *mut View;
    pub fn elg_mod_foreground_style(v: *const View, color: ElgProp) -> *mut View;
    pub fn elg_mod_tint(v: *const View, color: ElgProp) -> *mut View;
    pub fn elg_mod_color_scheme(v: *const View, scheme: ElgProp) -> *mut View;
    pub fn elg_mod_theme(v: *const View, theme: *const ElgTheme) -> *mut View;
    pub fn elg_mod_disabled(v: *const View, disabled: ElgProp) -> *mut View;
    pub fn elg_mod_button_style(v: *const View, style: *const ButtonStyle) -> *mut View;
    pub fn elg_mod_toggle_style(v: *const View, style: *const ToggleStyle) -> *mut View;
    pub fn elg_mod_text_field_style(v: *const View, style: *const TextFieldStyle) -> *mut View;
    pub fn elg_mod_animation(v: *const View, animation: *const Animation, version: ElgClosure) -> *mut View;
    pub fn elg_mod_transition(v: *const View, transition: *const Transition) -> *mut View;
    pub fn elg_mod_animates_layout(v: *const View, enabled: bool) -> *mut View;

    pub fn elg_button_style_builtin(kind: i32) -> *mut ButtonStyle;
    pub fn elg_toggle_style_builtin(kind: i32) -> *mut ToggleStyle;
    pub fn elg_text_field_style_builtin(kind: i32) -> *mut TextFieldStyle;
    pub fn elg_button_style_clone(s: *const ButtonStyle) -> *mut ButtonStyle;
    pub fn elg_button_style_free(s: *mut ButtonStyle);
    pub fn elg_toggle_style_clone(s: *const ToggleStyle) -> *mut ToggleStyle;
    pub fn elg_toggle_style_free(s: *mut ToggleStyle);
    pub fn elg_text_field_style_clone(s: *const TextFieldStyle) -> *mut TextFieldStyle;
    pub fn elg_text_field_style_free(s: *mut TextFieldStyle);
    pub fn elg_button_config_free(c: *mut ButtonConfiguration);
    pub fn elg_button_config_clone(c: *const ButtonConfiguration) -> *mut ButtonConfiguration;
    pub fn elg_button_config_label(c: *const ButtonConfiguration) -> *mut View;
    pub fn elg_button_config_state(c: *const ButtonConfiguration, which: i32) -> bool;
    pub fn elg_button_style_make_body(style: *const ButtonStyle, c: *const ButtonConfiguration) -> *mut View;
    pub fn elg_toggle_config_free(c: *mut ToggleConfiguration);
    pub fn elg_toggle_config_clone(c: *const ToggleConfiguration) -> *mut ToggleConfiguration;
    pub fn elg_toggle_config_label(c: *const ToggleConfiguration) -> *mut View;
    pub fn elg_toggle_config_state(c: *const ToggleConfiguration, which: i32) -> bool;
    pub fn elg_toggle_config_toggle(c: *const ToggleConfiguration);
    pub fn elg_toggle_style_make_body(style: *const ToggleStyle, c: *const ToggleConfiguration) -> *mut View;
    pub fn elg_text_field_config_free(c: *mut TextFieldConfiguration);
    pub fn elg_text_field_config_clone(c: *const TextFieldConfiguration) -> *mut TextFieldConfiguration;
    pub fn elg_text_field_config_field(c: *const TextFieldConfiguration) -> *mut View;
    pub fn elg_text_field_config_state(c: *const TextFieldConfiguration, which: i32) -> bool;
    pub fn elg_text_field_style_make_body(style: *const TextFieldStyle, c: *const TextFieldConfiguration) -> *mut View;

    /// The `elg::ModuleEntry` of a Rust views module (one per loaded module; built once).
    pub fn elg_rust_module_entry(
        has_root: bool,
        previews: *const ElgPreviewDesc,
        count: usize,
        build: ElgModuleBuild,
    ) -> *const c_void;

    pub fn elg_grid(desc: *const ElgGridDesc, children: *mut *mut View, count: usize) -> *mut View;
    pub fn elg_grid_layout_size(
        desc: *const ElgGridDesc,
        proposal: ElgProposedSize,
        children: *const LayoutChild,
        count: usize,
    ) -> ElgSize;
    pub fn elg_grid_layout_place(
        desc: *const ElgGridDesc,
        bounds: ElgRect,
        proposal: ElgProposedSize,
        children: *const LayoutChild,
        count: usize,
    );
    pub fn elg_grid_row(alignment: i32, cells: *mut *mut View, count: usize) -> *mut View;
    pub fn elg_resolve_grid_tracks(
        items: *const ElgGridItem,
        count: usize,
        has_length: bool,
        length: f32,
        spacing: ElgOptLen,
        each: ElgClosure,
    );
    pub fn elg_mod_grid_cell_columns(v: *const View, count: i32) -> *mut View;
    pub fn elg_mod_grid_cell_alignment(v: *const View, alignment: ElgAlignment) -> *mut View;
    pub fn elg_mod_grid_column_alignment(v: *const View, alignment: i32) -> *mut View;

    pub fn elg_hot_reload(
        module: ElgStr,
        build_command: ElgStr,
        watch: *const ElgStr,
        watch_count: usize,
        show: i32,
    ) -> *mut View;

    // --- ext.cpp ---
    pub fn elg_button_style_rust(make_body: ElgClosure) -> *mut ButtonStyle;
    pub fn elg_toggle_style_rust(make_body: ElgClosure) -> *mut ToggleStyle;
    pub fn elg_text_field_style_rust(make_body: ElgClosure) -> *mut TextFieldStyle;
    pub fn elg_rust_layout_view(layout: ElgClosure, children: *mut *mut View, count: usize) -> *mut View;
    pub fn elg_layout_cache_get(cache: *const LayoutCache) -> *mut c_void;
    pub fn elg_layout_cache_set(cache: *mut LayoutCache, owner: ElgClosure);
    pub fn elg_layout_cache_stale(cache: *const LayoutCache) -> bool;
    pub fn elg_layout_cache_set_stale(cache: *mut LayoutCache, stale: bool);
    pub fn elg_layout_child_size_that_fits(child: *const LayoutChild, proposal: ElgProposedSize) -> ElgSize;
    pub fn elg_layout_child_place(child: *const LayoutChild, position: ElgPoint, proposal: ElgProposedSize);
    pub fn elg_layout_child_priority(child: *const LayoutChild) -> f32;
    pub fn elg_layout_child_is_spacer(child: *const LayoutChild) -> bool;
    pub fn elg_mod_rust_layout_value(v: *const View, owner: ElgClosure) -> *mut View;
    pub fn elg_layout_child_rust_value(child: *const LayoutChild) -> *const c_void;
    pub fn elg_mod_rust_view_modifier(v: *const View, body: ElgClosure) -> *mut View;
    pub fn elg_mod_rust_layout_modifier(v: *const View, layout: ElgClosure) -> *mut View;
    pub fn elg_mod_rust_decoration_modifier(
        v: *const View,
        decoration: ElgClosure,
        placement: i32,
        alignment: ElgAlignment,
    ) -> *mut View;
    pub fn elg_mod_rust_environment_modifier(v: *const View, apply: ElgClosure) -> *mut View;
    pub fn elg_mod_rust_effect_modifier(v: *const View, apply: ElgClosure) -> *mut View;
    pub fn elg_mod_rust_event_modifier(v: *const View, make_handler: ElgClosure) -> *mut View;
    pub fn elg_effects_clip_to_bounds(e: *const Effects) -> bool;
    pub fn elg_effects_set_clip_to_bounds(e: *mut Effects, clip: bool);
    pub fn elg_effects_set_clip_shape(e: *mut Effects, shape: *const Shape);
    pub fn elg_effects_set_opacity(e: *mut Effects, opacity: ElgProp);
    pub fn elg_effects_set_shadow(e: *mut Effects, shadow: *const ElgShadow);
    pub fn elg_effects_set_offset(e: *mut Effects, x: ElgProp, y: ElgProp);
    pub fn elg_effects_set_z_index(e: *mut Effects, z: *const ElgProp);
    pub fn elg_effects_set_content_shape(e: *mut Effects, shape: *const Shape);
    pub fn elg_effects_set_inner_shadow(e: *mut Effects, shadow: *const ElgShadow);
    pub fn elg_effects_set_blur(e: *mut Effects, radius: *const ElgProp, opaque: bool);
    pub fn elg_effects_set_color_filter(e: *mut Effects, kind: i32, amount: ElgProp, color: ElgProp);
    pub fn elg_effects_blend_mode(e: *const Effects) -> i32;
    pub fn elg_effects_set_blend_mode(e: *mut Effects, mode: i32);
    pub fn elg_effects_compositing_group(e: *const Effects) -> bool;
    pub fn elg_effects_set_compositing_group(e: *mut Effects, group: bool);
    pub fn elg_effects_set_rotation(e: *mut Effects, degrees: *const ElgProp);
    pub fn elg_effects_set_scale(e: *mut Effects, x: *const ElgProp, y: *const ElgProp);
    pub fn elg_effects_anchor(e: *const Effects) -> ElgPoint;
    pub fn elg_effects_set_anchor(e: *mut Effects, anchor: ElgPoint);
    pub fn elg_effects_allows_hit_testing(e: *const Effects) -> bool;
    pub fn elg_effects_set_allows_hit_testing(e: *mut Effects, enabled: bool);

    // --- platform.cpp ---
    pub fn elg_audio_output_devices(each: ElgClosure);
    pub fn elg_sound_load_file(file: ElgStr) -> *mut Sound;
    pub fn elg_sound_load_data(data: ElgBytes) -> *mut Sound;
    pub fn elg_sound_clone(sound: *const Sound) -> *mut Sound;
    pub fn elg_sound_free(sound: *mut Sound);
    pub fn elg_sound_play(sound: *const Sound, volume: f32, pitch: f32, pan: f32);
    pub fn elg_sound_duration(sound: *const Sound) -> f64;
    pub fn elg_audio_player_new(file: ElgStr) -> *mut AudioPlayer;
    pub fn elg_audio_player_clone(player: *const AudioPlayer) -> *mut AudioPlayer;
    pub fn elg_audio_player_free(player: *mut AudioPlayer);
    pub fn elg_audio_player_control(player: *const AudioPlayer, op: i32);
    pub fn elg_audio_player_seek(player: *const AudioPlayer, seconds: f64);
    pub fn elg_audio_player_set_volume(player: *const AudioPlayer, volume: f32);
    pub fn elg_audio_player_set_looping(player: *const AudioPlayer, looping: bool);
    pub fn elg_audio_player_flag(player: *const AudioPlayer, which: i32) -> bool;
    pub fn elg_audio_player_value(player: *const AudioPlayer, which: i32) -> f64;
    pub fn elg_audio_player_binding(player: *const AudioPlayer, which: i32) -> *mut BindingF32;
    pub fn elg_audio_stream_new(
        device_id: ElgStr,
        sample_rate: u32,
        channels: u32,
        on_render: ElgClosure,
    ) -> *mut AudioStream;
    pub fn elg_audio_stream_clone(stream: *const AudioStream) -> *mut AudioStream;
    pub fn elg_audio_stream_free(stream: *mut AudioStream);
    pub fn elg_audio_stream_write(stream: *const AudioStream, samples: *const f32, count: usize);
    pub fn elg_audio_stream_control(stream: *const AudioStream, op: i32);
    pub fn elg_audio_stream_queued_frames(stream: *const AudioStream) -> usize;
    pub fn elg_audio_stream_set_volume(stream: *const AudioStream, volume: f32);
    pub fn elg_audio_stream_error(stream: *const AudioStream, sink: ElgStrSink, ctx: *mut c_void);
    pub fn elg_audio_stream_sample_rate(stream: *const AudioStream) -> u32;
    pub fn elg_audio_stream_channels(stream: *const AudioStream) -> u32;

    pub fn elg_audio_input_devices(each: ElgClosure);
    pub fn elg_audio_recorder_new(
        device_id: ElgStr,
        sample_rate: u32,
        channels: u32,
        on_samples: ElgClosure,
        keeps_samples: bool,
    ) -> *mut AudioRecorder;
    pub fn elg_audio_recorder_clone(recorder: *const AudioRecorder) -> *mut AudioRecorder;
    pub fn elg_audio_recorder_free(recorder: *mut AudioRecorder);
    pub fn elg_audio_recorder_control(recorder: *const AudioRecorder, op: i32);
    pub fn elg_audio_recorder_is_recording(recorder: *const AudioRecorder) -> bool;
    pub fn elg_audio_recorder_duration(recorder: *const AudioRecorder) -> f64;
    pub fn elg_audio_recorder_level(recorder: *const AudioRecorder) -> f32;
    pub fn elg_audio_recorder_error(recorder: *const AudioRecorder, sink: ElgStrSink, ctx: *mut c_void);
    pub fn elg_audio_recorder_sample_rate(recorder: *const AudioRecorder) -> u32;
    pub fn elg_audio_recorder_channels(recorder: *const AudioRecorder) -> u32;
    pub fn elg_audio_recorder_samples(recorder: *const AudioRecorder, each: ElgClosure);
    pub fn elg_audio_recorder_save_wav(recorder: *const AudioRecorder, file: ElgStr);
    pub fn elg_audio_recorder_wav(recorder: *const AudioRecorder, each: ElgClosure);
    pub fn elg_audio_recorder_sound(recorder: *const AudioRecorder) -> *mut Sound;
    pub fn elg_camera_devices(each: ElgClosure);
    pub fn elg_camera_new(device_id: ElgStr, width: u32, height: u32, mirrored: bool) -> *mut Camera;
    pub fn elg_camera_clone(camera: *const Camera) -> *mut Camera;
    pub fn elg_camera_free(camera: *mut Camera);
    pub fn elg_camera_start(camera: *const Camera, start: bool);
    pub fn elg_camera_is_running(camera: *const Camera) -> bool;
    pub fn elg_camera_size(camera: *const Camera) -> ElgSize;
    pub fn elg_camera_error(camera: *const Camera, sink: ElgStrSink, ctx: *mut c_void);
    pub fn elg_camera_mirrored(camera: *const Camera) -> bool;
    pub fn elg_camera_snapshot(camera: *const Camera, each: ElgClosure) -> bool;
    pub fn elg_camera_view(camera: *const Camera, mode: i32) -> *mut View;

    pub fn elg_video_player_new(source: ElgStr) -> *mut VideoPlayer;
    pub fn elg_video_player_new_file(file: ElgStr) -> *mut VideoPlayer;
    pub fn elg_video_player_clone(player: *const VideoPlayer) -> *mut VideoPlayer;
    pub fn elg_video_player_free(player: *mut VideoPlayer);
    pub fn elg_video_player_control(player: *const VideoPlayer, op: i32);
    pub fn elg_video_player_seek(player: *const VideoPlayer, seconds: f64);
    pub fn elg_video_player_set_volume(player: *const VideoPlayer, volume: f32);
    pub fn elg_video_player_set_muted(player: *const VideoPlayer, muted: bool);
    pub fn elg_video_player_set_looping(player: *const VideoPlayer, looping: bool);
    pub fn elg_video_player_flag(player: *const VideoPlayer, which: i32) -> bool;
    pub fn elg_video_player_value(player: *const VideoPlayer, which: i32) -> f64;
    pub fn elg_video_player_natural_size(player: *const VideoPlayer) -> ElgSize;
    pub fn elg_video_player_error(player: *const VideoPlayer, sink: ElgStrSink, ctx: *mut c_void);
    pub fn elg_video_player_binding(player: *const VideoPlayer, which: i32) -> *mut BindingF32;
    pub fn elg_video_view(player: *const VideoPlayer, mode: i32) -> *mut View;
    pub fn elg_video_controls(player: *const VideoPlayer) -> *mut View;
    pub fn elg_gpu_image_new(width: u32, height: u32, origin: i32) -> *mut GpuImage;
    pub fn elg_gpu_image_clone(image: *const GpuImage) -> *mut GpuImage;
    pub fn elg_gpu_image_free(image: *mut GpuImage);
    pub fn elg_gpu_image_same(a: *const GpuImage, b: *const GpuImage) -> bool;
    pub fn elg_gpu_image_begin_frame(image: *const GpuImage, frame: *mut ElgGpuFrame) -> bool;
    pub fn elg_gpu_image_end_frame(image: *const GpuImage, frame: *const ElgGpuFrame);
    pub fn elg_gpu_image_cancel_frame(image: *const GpuImage, frame: *const ElgGpuFrame);
    pub fn elg_gpu_image_size(image: *const GpuImage) -> ElgSize;
    pub fn elg_gpu_image_origin(image: *const GpuImage) -> i32;
    pub fn elg_gpu_image_view(image: *const GpuImage, mode: i32) -> *mut View;
    pub fn elg_video_feed_new() -> *mut VideoFeed;
    pub fn elg_video_feed_clone(feed: *const VideoFeed) -> *mut VideoFeed;
    pub fn elg_video_feed_free(feed: *mut VideoFeed);
    pub fn elg_video_feed_eq(a: *const VideoFeed, b: *const VideoFeed) -> bool;
    pub fn elg_video_feed_push(
        feed: *const VideoFeed,
        format: i32,
        width: u32,
        height: u32,
        pixels: *const u8,
        count: usize,
        full_range: bool,
    );
    pub fn elg_video_feed_clear(feed: *const VideoFeed);
    pub fn elg_video_feed_size(feed: *const VideoFeed) -> ElgSize;
    pub fn elg_video_feed_has_frame(feed: *const VideoFeed) -> bool;
    pub fn elg_video_feed_view(feed: *const VideoFeed, mode: i32, mirrored: bool) -> *mut View;

    pub fn elg_http_fetch(request: *const ElgRequest, done: ElgClosure) -> *mut HttpTask;
    pub fn elg_http_task_clone(task: *const HttpTask) -> *mut HttpTask;
    pub fn elg_http_task_free(task: *mut HttpTask);
    pub fn elg_http_task_cancel(task: *const HttpTask);
    pub fn elg_http_fetch_state(request: *const ElgRequest) -> *mut HttpFetch;
    pub fn elg_http_fetch_state_clone(fetch: *const HttpFetch) -> *mut HttpFetch;
    pub fn elg_http_fetch_state_free(fetch: *mut HttpFetch);
    pub fn elg_http_fetch_state_flag(fetch: *const HttpFetch, which: i32) -> bool;
    pub fn elg_http_fetch_state_result(fetch: *const HttpFetch, use_: ElgClosure);
    pub fn elg_http_fetch_state_reload(fetch: *const HttpFetch);

    pub fn elg_websocket_connect(
        url: ElgStr,
        headers: *const ElgHeader,
        header_count: usize,
        timeout_ms: i64,
        events: ElgWebSocketEvents,
    ) -> *mut WebSocket;
    pub fn elg_websocket_clone(socket: *const WebSocket) -> *mut WebSocket;
    pub fn elg_websocket_free(socket: *mut WebSocket);
    pub fn elg_websocket_send(socket: *const WebSocket, text: ElgStr, binary: bool);
    pub fn elg_websocket_close(socket: *const WebSocket, code: i32, reason: ElgStr);
    pub fn elg_websocket_is_open(socket: *const WebSocket) -> bool;

    pub fn elg_choose_files(
        filters: *const ElgFileFilter,
        filter_count: usize,
        directory: ElgStr,
        multiple: bool,
        done: ElgClosure,
    );
    pub fn elg_choose_save_file(
        filters: *const ElgFileFilter,
        filter_count: usize,
        directory: ElgStr,
        name: ElgStr,
        done: ElgClosure,
    );
    pub fn elg_choose_folders(directory: ElgStr, multiple: bool, done: ElgClosure);
    pub fn elg_notify(title: ElgStr, body: ElgStr, on_click: ElgClosure);
    pub fn elg_tray_icon_new(
        icon: *const ElgImageSource,
        tooltip: ElgStr,
        menu: *const ElgTrayMenuItem,
        count: usize,
    ) -> *mut TrayIcon;
    pub fn elg_tray_icon_clone(icon: *const TrayIcon) -> *mut TrayIcon;
    pub fn elg_tray_icon_free(icon: *mut TrayIcon);
    pub fn elg_tray_icon_set_tooltip(icon: *const TrayIcon, tooltip: ElgStr);
    pub fn elg_tray_icon_set_menu(icon: *const TrayIcon, menu: *const ElgTrayMenuItem, count: usize);
}
