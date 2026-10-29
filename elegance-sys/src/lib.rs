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

pub mod gpu_image;

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
    /// `elg::Binding<double>`
    BindingF64,
    /// `elg::ScrollPosition`
    ScrollPosition,
    /// `elg::GraphicsContext`
    GraphicsContext,
    /// `std::shared_ptr<const elg::ButtonStyle>`
    ButtonStyle,
    /// `std::shared_ptr<const elg::ToggleStyle>`
    ToggleStyle,
    /// `std::shared_ptr<const elg::CheckboxStyle>`
    CheckboxStyle,
    /// `std::shared_ptr<const elg::NumberInputStyle>`
    NumberInputStyle,
    /// `std::shared_ptr<const elg::ProgressStyle>`
    ProgressStyle,
    /// `std::shared_ptr<const elg::TextFieldStyle>`
    TextFieldStyle,
    /// `std::shared_ptr<const elg::PickerStyle>`
    PickerStyle,
    /// `std::shared_ptr<const elg::SliderStyle>`
    SliderStyle,
    /// `std::shared_ptr<const elg::NavigationStackStyle>`
    NavigationStackStyle,
    /// `std::shared_ptr<const elg::PopoverStyle>`
    PopoverStyle,
    /// `std::shared_ptr<const elg::TooltipStyle>`
    TooltipStyle,
    /// `std::shared_ptr<const elg::MenuStyle>`
    MenuStyle,
    /// `std::shared_ptr<const elg::RowStyle>`
    RowStyle,
    /// `elg::ButtonConfiguration`
    ButtonConfiguration,
    /// `elg::ToggleConfiguration`
    ToggleConfiguration,
    /// `elg::CheckboxConfiguration`
    CheckboxConfiguration,
    /// `elg::NumberInputConfiguration`
    NumberInputConfiguration,
    /// `elg::ProgressConfiguration`
    ProgressConfiguration,
    /// `elg::TextFieldConfiguration`
    TextFieldConfiguration,
    /// `elg::PickerConfiguration`
    PickerConfiguration,
    /// `elg::SliderConfiguration`
    SliderConfiguration,
    /// `elg::NavigationStackConfiguration`
    NavigationStackConfiguration,
    /// `elg::PopoverConfiguration`
    PopoverConfiguration,
    /// `elg::TooltipConfiguration`
    TooltipConfiguration,
    /// `elg::MenuConfiguration`
    MenuConfiguration,
    /// `elg::MenuItemConfiguration`
    MenuItemConfiguration,
    /// `elg::RowConfiguration`
    RowConfiguration,
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
    /// `elg::VideoFeed`
    VideoFeed,
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
    /// `std::optional<float>` (a closure's result)
    OptFloat,
    /// `std::vector<elg::ItemKey>`
    ItemKeys,
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
        Self {
            ptr: s.as_ptr().cast(),
            len: s.len(),
        }
    }
    pub const EMPTY: Self = Self {
        ptr: std::ptr::null(),
        len: 0,
    };
    /// # Safety
    /// The pointer must be valid UTF-8 for `len` bytes, for `'a`.
    pub unsafe fn as_str<'a>(&self) -> &'a str { unsafe {
        if self.ptr.is_null() {
            ""
        } else {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.ptr.cast(), self.len))
        }
    }}
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgBytes {
    pub ptr: *const u8,
    pub len: usize,
}

impl ElgBytes {
    pub fn new(b: &[u8]) -> Self {
        Self {
            ptr: b.as_ptr(),
            len: b.len(),
        }
    }
    pub const EMPTY: Self = Self {
        ptr: std::ptr::null(),
        len: 0,
    };
    /// # Safety
    /// The pointer must be valid for `len` bytes, for `'a`.
    pub unsafe fn as_slice<'a>(&self) -> &'a [u8] { unsafe {
        if self.ptr.is_null() {
            &[]
        } else {
            std::slice::from_raw_parts(self.ptr, self.len)
        }
    }}
}

pub type ElgCallFn =
    unsafe extern "C" fn(data: *mut c_void, args: *mut c_void, ret: *mut c_void) -> bool;
pub type ElgDropFn = unsafe extern "C" fn(data: *mut c_void);
pub type ElgStrSink = unsafe extern "C" fn(ctx: *mut c_void, ptr: *const c_char, len: usize);

#[repr(C)]
pub struct ElgClosure {
    pub data: *mut c_void,
    pub call: Option<ElgCallFn>,
    pub drop: Option<ElgDropFn>,
}

impl ElgClosure {
    pub const NONE: Self = Self {
        data: std::ptr::null_mut(),
        call: None,
        drop: None,
    };
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
pub struct ElgPixelSize {
    pub width: u32,
    pub height: u32,
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
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgEdgeInsets {
    pub top: f32,
    pub leading: f32,
    pub bottom: f32,
    pub trailing: f32,
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
    pub window_location: ElgPoint,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgScrollEvent {
    pub location: ElgPoint,
    pub window_location: ElgPoint,
    pub dx: f32,
    pub dy: f32,
    pub precise: bool,
    pub modifiers: u32,
    pub phase: i32,
    pub momentum: bool,
    pub time: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgClickEvent {
    pub location: ElgPoint,
    pub window_location: ElgPoint,
    pub button: i32,
    pub modifiers: u32,
    pub click_count: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgDragEvent {
    pub location: ElgPoint,
    pub start_location: ElgPoint,
    pub translation: ElgPoint,
    pub velocity: ElgPoint,
    pub button: i32,
    pub modifiers: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgMagnifyEvent {
    pub location: ElgPoint,
    pub scale: f32,
    pub delta: f32,
    pub phase: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgRotateEvent {
    pub location: ElgPoint,
    pub degrees: f32,
    pub delta: f32,
    pub phase: i32,
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
    pub cancellable: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgFloatingPlacement {
    pub edge: i32,
    pub alignment: i32,
    pub spacing: f32,
    pub offset: ElgPoint,
    pub has_position: bool,
    pub position: ElgPoint,
}

/// `has` false: the style's motion; null transition or animation: none. Borrowed.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgPopupMotion {
    pub has: bool,
    pub transition: *const Transition,
    pub animation: *const Animation,
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
    pub has_system_controls_position: bool,
    pub system_controls_position: ElgPoint,
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
    pub secure: ElgProp,
    pub line_limit: ElgProp,
    pub styler: ElgClosure,
    pub on_key: ElgClosure,
}

#[repr(C)]
pub struct ElgNumberInputOptions {
    pub min: ElgProp,
    pub max: ElgProp,
    pub step: ElgProp,
    pub decimals: ElgProp,
    pub format: ElgClosure,
    pub parse: ElgClosure,
    pub live: ElgProp,
    pub shows_buttons: ElgProp,
    pub scrubs: ElgProp,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgOptNumber {
    pub has: bool,
    pub value: f64,
}

#[repr(C)]
pub struct ElgStackSeparators {
    pub color: ElgProp,
    pub inset: f32,
    pub thickness: f32,
}

#[repr(C)]
pub struct ElgSplitViewOptions {
    pub divider: ElgProp,
    pub active_divider: *const ElgProp,
    pub thickness: f32,
    pub active_thickness: f32,
}

#[repr(C)]
pub struct ElgNavigationSplitViewOptions {
    pub take_sidebar_visible: *mut BindingBool,
    pub sidebar_transition: *const Transition,
    pub sidebar_background: ElgProp,
    pub sidebar_width: ElgColumnWidth,
    pub content_width: ElgColumnWidth,
    pub detail_width: ElgColumnWidth,
    pub dividers: ElgSplitViewOptions,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgGestureOptions {
    pub priority: i32,
    pub simultaneous: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgDragOptions {
    pub minimum_distance: f32,
    pub button: i32,
    pub hold: f64,
    pub hold_tolerance: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ElgLongPressOptions {
    pub duration: f64,
    pub maximum_distance: f32,
    pub button: i32,
}

#[repr(C)]
pub struct ElgFloatingOptions {
    pub placement: ElgFloatingPlacement,
    pub boundary: i32,
    pub outside_click: bool,
    pub escape: bool,
    pub motion: ElgPopupMotion,
}

#[repr(C)]
pub struct ElgTooltipOptions {
    pub placement: ElgFloatingPlacement,
    pub boundary: i32,
    pub delay: f64,
    pub motion: ElgPopupMotion,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgItemKey {
    pub is_string: bool,
    pub int_key: i64,
    pub string_key: ElgStr,
}

#[repr(C)]
pub struct ElgItemKeys {
    pub keys: *const ElgItemKey,
    pub count: usize,
}

#[repr(C)]
pub struct ElgSwitchToggleStyleOptions {
    pub on: *const ElgProp,
    pub off: ElgProp,
    pub knob: ElgProp,
    pub width: f32,
    pub height: f32,
    pub knob_inset: f32,
    pub spacing: f32,
    pub fills_width: bool,
    pub animation: *const Animation,
}

#[repr(C)]
pub struct ElgBoxCheckboxStyleOptions {
    pub size: f32,
    pub corner_radius: f32,
    pub fill: *const ElgProp,
    pub empty: ElgProp,
    pub border: ElgProp,
    pub border_width: f32,
    pub mark: ElgProp,
    pub take_check: *mut View,
    pub take_dash: *mut View,
    pub spacing: f32,
    pub animation: *const Animation,
}

#[repr(C)]
pub struct ElgBoxNumberInputStyleOptions {
    pub background: ElgProp,
    pub border: ElgProp,
    pub focused_border: *const ElgProp,
    pub border_width: f32,
    pub corner_radius: f32,
    pub padding: ElgEdgeInsets,
    pub arrow_size: f32,
}

#[repr(C)]
pub struct ElgProgressStyleOptions {
    pub track: ElgProp,
    pub fill: *const ElgProp,
    pub a: f32,
    pub b: f32,
}

#[repr(C)]
pub struct ElgKnobSliderStyleOptions {
    pub track: ElgProp,
    pub fill: *const ElgProp,
    pub track_thickness: f32,
    pub knob_size: f32,
}

#[repr(C)]
pub struct ElgBarNavigationStackStyleOptions {
    pub height: f32,
    pub background: ElgProp,
    pub title_font: ElgFont,
    pub back_color: *const ElgProp,
    pub shows_divider: bool,
}

#[repr(C)]
pub struct ElgHighlightRowStyleOptions {
    pub selected: *const ElgProp,
    pub on_selected: ElgProp,
    pub hovered: ElgProp,
    pub corner_radius: f32,
    pub padding: ElgEdgeInsets,
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
    pub location: ElgPoint,
    pub magnify: ElgMagnifyEvent,
    pub rotate: ElgRotateEvent,
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

/// `elg::Theme`: light then dark tokens, 17 colors each, then the blur radii.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ElgTheme {
    pub colors: [ElgColor; 34],
    pub blur: [f32; 7],
}

unsafe extern "C" {
    // --- core.cpp ---
    pub fn elg_take_error(sink: ElgStrSink, ctx: *mut c_void) -> i32;
    pub fn elg_error_pending() -> bool;
    pub fn elg_string_assign(target: *mut CppString, value: ElgStr);
    pub fn elg_string_view(s: *const CppString) -> ElgStr;

    pub fn elg_view_empty() -> *mut View;
    pub fn elg_view_clone(view: *const View) -> *mut View;
    pub fn elg_view_free(view: *mut View);
    pub fn elg_view_move_into(target: *mut View, take_view: *mut View);
    pub fn elg_view_is_empty(view: *const View) -> bool;
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
    pub fn elg_binding_f64(get: ElgClosure, set: ElgClosure) -> *mut BindingF64;
    pub fn elg_opt_float_assign(target: *mut OptFloat, value: ElgOptLen);
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
    pub fn elg_env_row_selected(env: *const EnvironmentValues) -> bool;
    pub fn elg_env_rust_values(env: *const EnvironmentValues) -> *mut c_void;
    pub fn elg_env_set_font(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_color_scheme(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_theme(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_foreground(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_tint(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_disabled(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_hover_delay(env: *mut EnvironmentValues, value: ElgProp);
    pub fn elg_env_set_row_selected(env: *mut EnvironmentValues, value: ElgProp);
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
    pub fn elg_window_title_bar(window: *const WindowHandle) -> i32;
    pub fn elg_window_system_controls_frame(window: *const WindowHandle) -> ElgRect;
    pub fn elg_window_controls(style: i32, minimize: bool, maximize: bool) -> *mut View;

    pub fn elg_app_new() -> *mut App;
    pub fn elg_app_free(app: *mut App);
    pub fn elg_app_window(app: *mut App, desc: *const ElgWindowDesc, content: ElgClosure);
    pub fn elg_app_run(app: *mut App) -> i32;
    pub fn elg_app_quit(app: *mut App);
    pub fn elg_post_to_ui_thread(task: ElgClosure);
    pub fn elg_version() -> *const c_char;

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
    pub fn elg_transition_scale(from: f32, anchor: ElgAlignment) -> *mut Transition;
    pub fn elg_transition_offset(x: f32, y: f32) -> *mut Transition;
    pub fn elg_transition_move(edge: u32) -> *mut Transition;
    pub fn elg_transition_slide() -> *mut Transition;
    pub fn elg_transition_asymmetric(
        insertion: *const Transition,
        removal: *const Transition,
    ) -> *mut Transition;
    pub fn elg_transition_combined(a: *const Transition, b: *const Transition) -> *mut Transition;
    pub fn elg_transition_pose(
        transition: *const Transition,
        progress: f32,
        removal: bool,
        size: ElgSize,
    ) -> ElgPose;
    pub fn elg_transition_clone(transition: *const Transition) -> *mut Transition;
    pub fn elg_transition_free(transition: *mut Transition);

    pub fn elg_layouts_core(sink: ElgStrSink, ctx: *mut c_void);

    // --- views.cpp ---
    pub fn elg_vstack(
        alignment: i32,
        spacing: ElgOptLen,
        children: *mut *mut View,
        count: usize,
    ) -> *mut View;
    pub fn elg_hstack(
        alignment: i32,
        spacing: ElgOptLen,
        children: *mut *mut View,
        count: usize,
    ) -> *mut View;
    pub fn elg_zstack(alignment: ElgAlignment, children: *mut *mut View, count: usize)
    -> *mut View;
    pub fn elg_spacer(min_length: ElgOptLen) -> *mut View;
    pub fn elg_empty_view() -> *mut View;
    pub fn elg_aligned_position(
        container: ElgRect,
        content: ElgSize,
        alignment: ElgAlignment,
    ) -> ElgPoint;
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
    pub fn elg_button_title(title: ElgProp, on_click: ElgClosure) -> *mut View;
    pub fn elg_text_spans_push(
        spans: *mut TextSpans,
        begin: usize,
        end: usize,
        style: *const ElgRunStyle,
        label: *const ElgStr,
    );
    pub fn elg_text_field(
        placeholder: ElgProp,
        take_text: *mut BindingString,
        options: *const ElgTextFieldOptions,
    ) -> *mut View;
    pub fn elg_text_area(
        placeholder: ElgProp,
        take_text: *mut BindingString,
        options: *const ElgTextFieldOptions,
    ) -> *mut View;
    pub fn elg_slider(
        take_value: *mut BindingF32,
        min: ElgProp,
        max: ElgProp,
        step: ElgProp,
        on_editing_changed: ElgClosure,
    ) -> *mut View;
    pub fn elg_number_input(
        take_value: *mut BindingF64,
        integer: bool,
        options: *const ElgNumberInputOptions,
    ) -> *mut View;
    pub fn elg_progress(fraction: ElgProp) -> *mut View;
    pub fn elg_picker(
        labels: *const ElgStr,
        count: usize,
        selected: ElgClosure,
        select: ElgClosure,
    ) -> *mut View;
    pub fn elg_toggle(take_label: *mut View, take_is_on: *mut BindingBool) -> *mut View;
    pub fn elg_toggle_title(title: ElgProp, take_is_on: *mut BindingBool) -> *mut View;
    pub fn elg_checkbox(
        take_label: *mut View,
        take_is_checked: *mut BindingBool,
        mixed: ElgProp,
    ) -> *mut View;
    pub fn elg_checkbox_title(
        title: ElgProp,
        take_is_checked: *mut BindingBool,
        mixed: ElgProp,
    ) -> *mut View;
    pub fn elg_separator(axis: i32, color: ElgProp, thickness: f32, inset: f32) -> *mut View;

    pub fn elg_if(condition: ElgProp, take_then: *mut View, take_else: *mut View) -> *mut View;
    pub fn elg_for_each(items: ElgClosure) -> *mut View;
    pub fn elg_for_each_items_push(
        target: *mut ForEachItems,
        items: *const ElgForEachItem,
        count: usize,
    );

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
        shows_indicators: ElgProp,
        position: *const ScrollPosition,
        sticks_to_end: ElgProp,
    ) -> *mut View;
    pub fn elg_lazy_stack(
        vertical: bool,
        alignment: i32,
        spacing: ElgOptLen,
        separators: *const ElgStackSeparators,
        take_for_each: *mut View,
    ) -> *mut View;

    pub fn elg_menu_item(
        title: ElgProp,
        on_choose: ElgClosure,
        shortcut: ElgProp,
        destructive: ElgProp,
        checked: *const ElgProp,
        image: *const ElgImageSource,
    ) -> *mut View;
    pub fn elg_submenu(title: ElgProp, menu: ElgClosure) -> *mut View;
    pub fn elg_menu(items: *mut *mut View, count: usize) -> *mut View;
    pub fn elg_menu_button(
        take_label: *mut View,
        title: *const ElgProp,
        menu: ElgClosure,
        placement: *const ElgFloatingPlacement,
        shows_indicator: ElgProp,
        motion: ElgPopupMotion,
    ) -> *mut View;

    pub fn elg_split_view(
        axis: i32,
        panes: *mut *mut View,
        count: usize,
        options: *const ElgSplitViewOptions,
    ) -> *mut View;
    pub fn elg_navigation_split_view(
        take_sidebar: *mut View,
        take_content: *mut View,
        take_detail: *mut View,
        options: *const ElgNavigationSplitViewOptions,
    ) -> *mut View;
    pub fn elg_navigation_stack(
        take_root: *mut View,
        pages: ElgClosure,
        titles: ElgClosure,
        pop: ElgClosure,
        transition: i32,
        animation: *const Animation,
        back_gestures: bool,
    ) -> *mut View;
    pub fn elg_strings_push(target: *mut Strings, value: ElgStr);

    pub fn elg_text(content: ElgProp) -> *mut Text;
    pub fn elg_text_clone(text: *const Text) -> *mut Text;
    pub fn elg_text_free(text: *mut Text);
    pub fn elg_text_to_view(text: *const Text) -> *mut View;
    pub fn elg_text_font(text: *const Text, font: ElgProp) -> *mut Text;
    pub fn elg_text_font_weight(text: *const Text, weight: ElgProp) -> *mut Text;
    pub fn elg_text_bold(text: *const Text) -> *mut Text;
    pub fn elg_text_foreground_style(text: *const Text, color: ElgProp) -> *mut Text;
    pub fn elg_text_line_limit(text: *const Text, lines: i32) -> *mut Text;
    pub fn elg_text_multiline_alignment(text: *const Text, alignment: i32) -> *mut Text;
    pub fn elg_styled_text(runs: *const ElgTextRun, count: usize) -> *mut View;

    pub fn elg_shape_clone(shape: *const Shape) -> *mut Shape;
    pub fn elg_shape_free(shape: *mut Shape);
    pub fn elg_shape_to_view(shape: *const Shape) -> *mut View;
    pub fn elg_rectangle() -> *mut Shape;
    pub fn elg_rounded_rectangle(
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) -> *mut Shape;
    pub fn elg_capsule() -> *mut Shape;
    pub fn elg_circle() -> *mut Shape;
    pub fn elg_path_shape(
        path: *const Path,
        view_box: *const ElgRect,
        fill_rule: i32,
    ) -> *mut Shape;
    pub fn elg_custom_shape(path: ElgClosure, fill_rule: i32) -> *mut Shape;
    pub fn elg_shape_fill(shape: *const Shape, style: ElgProp) -> *mut Shape;
    pub fn elg_shape_stroke(
        shape: *const Shape,
        color: ElgProp,
        style: *const ElgStrokeStyle,
    ) -> *mut Shape;
    pub fn elg_shape_stroke_border(
        shape: *const Shape,
        color: ElgProp,
        line_width: f32,
    ) -> *mut Shape;

    pub fn elg_path_new() -> *mut Path;
    pub fn elg_path_clone(path: *const Path) -> *mut Path;
    pub fn elg_path_free(path: *mut Path);
    pub fn elg_path_assign(target: *mut Path, source: *const Path);
    pub fn elg_path_eq(a: *const Path, b: *const Path) -> bool;
    pub fn elg_path_move_to(path: *mut Path, p: ElgPoint);
    pub fn elg_path_line_to(path: *mut Path, p: ElgPoint);
    pub fn elg_path_quad_to(path: *mut Path, control: ElgPoint, end: ElgPoint);
    pub fn elg_path_cubic_to(
        path: *mut Path,
        control1: ElgPoint,
        control2: ElgPoint,
        end: ElgPoint,
    );
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
    pub fn elg_path_transformed(path: *const Path, sx: f32, sy: f32, dx: f32, dy: f32)
    -> *mut Path;
    pub fn elg_path_flatten(path: *const Path, tolerance: f32, polyline: ElgClosure);
    pub fn elg_path_stroked(
        path: *const Path,
        style: *const ElgStrokeStyle,
        tolerance: f32,
    ) -> *mut Path;

    pub fn elg_canvas(draw: ElgClosure) -> *mut View;
    pub fn elg_gc_size(gc: *const GraphicsContext) -> ElgSize;
    pub fn elg_gc_scale(gc: *const GraphicsContext) -> f32;
    pub fn elg_gc_time(gc: *const GraphicsContext) -> f64;
    pub fn elg_gc_request_frame(gc: *mut GraphicsContext);
    pub fn elg_gc_fill_rect(
        gc: *mut GraphicsContext,
        rect: ElgRect,
        color: ElgColor,
        radii: *const f32,
    );
    pub fn elg_gc_stroke_rect(
        gc: *mut GraphicsContext,
        rect: ElgRect,
        color: ElgColor,
        width: f32,
        radii: *const f32,
    );
    pub fn elg_gc_draw_rect(
        gc: *mut GraphicsContext,
        rect: ElgRect,
        radii: *const f32,
        fill: ElgColor,
        border: ElgColor,
        border_width: f32,
    );
    pub fn elg_gc_fill_path(
        gc: *mut GraphicsContext,
        path: *const Path,
        style: *const ElgShapeStyle,
        rule: i32,
    );
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
    pub fn elg_async_image(url: ElgStr, scale: f32, content_mode: i32, disk_cache: bool) -> *mut View;
    pub fn elg_async_image_content(
        url: ElgStr,
        content: ElgClosure,
        scale: f32,
        disk_cache: bool,
    ) -> *mut View;
    pub fn elg_set_image_cache(disk: bool, disk_limit: u64, directory: ElgStr);
    pub fn elg_clear_image_cache();
    pub fn elg_label(title: ElgProp, icon: *const ElgImageSource) -> *mut View;
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

    pub fn elg_mod_padding(v: *const View, insets: ElgProp) -> *mut View;
    pub fn elg_mod_padding_edges(v: *const View, edges: u32, length: ElgProp) -> *mut View;
    pub fn elg_mod_frame(v: *const View, options: *const ElgFrameOptions) -> *mut View;
    pub fn elg_mod_frame_size(v: *const View, size: ElgProp, alignment: ElgAlignment) -> *mut View;
    pub fn elg_mod_fixed_size(v: *const View, horizontal: bool, vertical: bool) -> *mut View;
    pub fn elg_mod_aspect_ratio(
        v: *const View,
        has_ratio: bool,
        ratio: f32,
        mode: i32,
    ) -> *mut View;
    pub fn elg_mod_resizable(v: *const View) -> *mut View;
    pub fn elg_mod_layout_priority(v: *const View, priority: f32) -> *mut View;
    pub fn elg_mod_z_index(v: *const View, z: ElgProp) -> *mut View;
    pub fn elg_mod_offset(v: *const View, x: ElgProp, y: ElgProp) -> *mut View;
    pub fn elg_mod_background_view(
        v: *const View,
        take_background: *mut View,
        alignment: ElgAlignment,
    ) -> *mut View;
    pub fn elg_mod_background(v: *const View, style: ElgProp) -> *mut View;
    pub fn elg_mod_background_shape(
        v: *const View,
        style: ElgProp,
        shape: *const Shape,
    ) -> *mut View;
    pub fn elg_mod_overlay(
        v: *const View,
        take_overlay: *mut View,
        alignment: ElgAlignment,
    ) -> *mut View;
    pub fn elg_mod_border(
        v: *const View,
        color: ElgProp,
        width: f32,
        shape: *const Shape,
    ) -> *mut View;
    pub fn elg_mod_clipped(v: *const View) -> *mut View;
    pub fn elg_mod_clip_shape(v: *const View, shape: *const Shape) -> *mut View;
    pub fn elg_mod_corner_radius(v: *const View, radius: f32) -> *mut View;
    pub fn elg_mod_opacity(v: *const View, opacity: ElgProp) -> *mut View;
    pub fn elg_mod_shadow(v: *const View, shadow: ElgShadow) -> *mut View;
    pub fn elg_mod_inner_shadow(v: *const View, shadow: ElgShadow) -> *mut View;
    pub fn elg_mod_blur(v: *const View, radius: ElgProp, opaque: bool) -> *mut View;
    pub fn elg_mod_backdrop_blur(
        v: *const View,
        radius: ElgProp,
        shape: *const Shape,
        saturation: ElgProp,
        brightness: ElgProp,
        noise: ElgProp,
    ) -> *mut View;
    pub fn elg_mod_color_filter(
        v: *const View,
        kind: i32,
        amount: ElgProp,
        color: ElgProp,
    ) -> *mut View;
    pub fn elg_mod_blend_mode(v: *const View, mode: i32) -> *mut View;
    pub fn elg_mod_compositing_group(v: *const View) -> *mut View;
    pub fn elg_mod_rotation_effect(v: *const View, degrees: ElgProp, anchor: ElgPoint)
    -> *mut View;
    pub fn elg_mod_scale_effect(
        v: *const View,
        x: ElgProp,
        y: *const ElgProp,
        anchor: ElgPoint,
    ) -> *mut View;
    pub fn elg_mod_mask(v: *const View, take_mask: *mut View, alignment: ElgAlignment)
    -> *mut View;
    pub fn elg_mod_mask_style(v: *const View, style: ElgProp) -> *mut View;
    pub fn elg_mod_content_shape(v: *const View, shape: *const Shape) -> *mut View;
    pub fn elg_mod_allows_hit_testing(v: *const View, enabled: bool) -> *mut View;
    pub fn elg_mod_on_click(
        v: *const View,
        on_click: ElgClosure,
        button: i32,
        count: i32,
    ) -> *mut View;
    pub fn elg_mod_on_activate(v: *const View, on_activate: ElgClosure) -> *mut View;
    pub fn elg_mod_drag_gesture(
        v: *const View,
        options: ElgDragOptions,
        began: ElgClosure,
        changed: ElgClosure,
        ended: ElgClosure,
        cancelled: ElgClosure,
        gesture: ElgGestureOptions,
    ) -> *mut View;
    pub fn elg_mod_long_press_gesture(
        v: *const View,
        on_press: ElgClosure,
        options: ElgLongPressOptions,
        pressing: ElgClosure,
        gesture: ElgGestureOptions,
    ) -> *mut View;
    pub fn elg_mod_magnify_gesture(
        v: *const View,
        began: ElgClosure,
        changed: ElgClosure,
        ended: ElgClosure,
        cancelled: ElgClosure,
    ) -> *mut View;
    pub fn elg_mod_rotate_gesture(
        v: *const View,
        began: ElgClosure,
        changed: ElgClosure,
        ended: ElgClosure,
        cancelled: ElgClosure,
    ) -> *mut View;
    pub fn elg_mod_on_two_finger_double_tap(v: *const View, on_tap: ElgClosure) -> *mut View;
    pub fn elg_mod_pointer_style(v: *const View, style: ElgProp) -> *mut View;
    pub fn elg_mod_on_pointer_boundary(v: *const View, leave: bool, on_cross: ElgClosure) -> *mut View;
    pub fn elg_mod_on_hover_change(v: *const View, on_hover: ElgClosure) -> *mut View;
    pub fn elg_mod_on_pointer(v: *const View, on_pointer: ElgClosure) -> *mut View;
    pub fn elg_mod_on_scroll(v: *const View, on_scroll: ElgClosure) -> *mut View;
    pub fn elg_mod_on_text_input(v: *const View, on_text_input: ElgClosure) -> *mut View;
    /// `on_size_change` receives `*const ElgPixelSize` as its args; ownership transfers to the view.
    pub fn elg_mod_on_size_change(v: *const View, on_size_change: ElgClosure) -> *mut View;
    pub fn elg_mod_on_drop(
        v: *const View,
        files: ElgClosure,
        text: ElgClosure,
        targeted: ElgClosure,
    ) -> *mut View;
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
    pub fn elg_mod_floating(
        v: *const View,
        take_is_presented: *mut BindingBool,
        content: ElgClosure,
        options: *const ElgFloatingOptions,
    ) -> *mut View;
    pub fn elg_mod_floating_container(v: *const View) -> *mut View;
    pub fn elg_mod_tooltip(
        v: *const View,
        content: ElgClosure,
        text: *const ElgProp,
        options: *const ElgTooltipOptions,
    ) -> *mut View;
    pub fn elg_mod_popover(
        v: *const View,
        take_is_presented: *mut BindingBool,
        content: ElgClosure,
        placement: *const ElgFloatingPlacement,
        boundary: i32,
        padded: ElgProp,
        motion: ElgPopupMotion,
    ) -> *mut View;
    pub fn elg_mod_focusable(v: *const View, focusable: bool) -> *mut View;
    pub fn elg_mod_focused(v: *const View, take_is_focused: *mut BindingBool) -> *mut View;
    pub fn elg_mod_on_key_press(v: *const View, key: i32, on_press: ElgClosure) -> *mut View;
    pub fn elg_mod_on_key(v: *const View, on_key: ElgClosure) -> *mut View;
    pub fn elg_item_keys_push(target: *mut ItemKeys, key: ElgItemKey);
    pub fn elg_mod_selection(
        v: *const View,
        multiple: bool,
        string_keys: bool,
        get: ElgClosure,
        set: ElgClosure,
        activate: ElgClosure,
    ) -> *mut View;
    pub fn elg_mod_font_weight(v: *const View, weight: ElgProp) -> *mut View;
    pub fn elg_mod_font(v: *const View, font: ElgProp) -> *mut View;
    pub fn elg_mod_foreground_style(v: *const View, color: ElgProp) -> *mut View;
    pub fn elg_mod_tint(v: *const View, color: ElgProp) -> *mut View;
    pub fn elg_mod_color_scheme(v: *const View, scheme: ElgProp) -> *mut View;
    pub fn elg_mod_theme(v: *const View, theme: *const ElgTheme) -> *mut View;
    pub fn elg_mod_disabled(v: *const View, disabled: ElgProp) -> *mut View;
    pub fn elg_mod_animation(
        v: *const View,
        animation: *const Animation,
        version: ElgClosure,
    ) -> *mut View;
    pub fn elg_mod_transition(v: *const View, transition: *const Transition) -> *mut View;
    pub fn elg_mod_animates_layout(v: *const View, enabled: bool) -> *mut View;

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

    pub fn elg_layouts_views(sink: ElgStrSink, ctx: *mut c_void);

    // --- ext.cpp ---
    pub fn elg_rust_layout_view(
        layout: ElgClosure,
        children: *mut *mut View,
        count: usize,
    ) -> *mut View;
    pub fn elg_layout_cache_get(cache: *const LayoutCache) -> *mut c_void;
    pub fn elg_layout_cache_set(cache: *mut LayoutCache, owner: ElgClosure);
    pub fn elg_layout_cache_stale(cache: *const LayoutCache) -> bool;
    pub fn elg_layout_cache_set_stale(cache: *mut LayoutCache, stale: bool);
    pub fn elg_layout_child_size_that_fits(
        child: *const LayoutChild,
        proposal: ElgProposedSize,
    ) -> ElgSize;
    pub fn elg_layout_child_place(
        child: *const LayoutChild,
        position: ElgPoint,
        proposal: ElgProposedSize,
    );
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
    pub fn elg_effects_set_clip_shape(e: *mut Effects, shape: *const Shape);
    pub fn elg_effects_set_opacity(e: *mut Effects, opacity: ElgProp);
    pub fn elg_effects_set_shadow(e: *mut Effects, shadow: *const ElgShadow);
    pub fn elg_effects_set_offset(e: *mut Effects, x: ElgProp, y: ElgProp);
    pub fn elg_effects_set_z_index(e: *mut Effects, z: *const ElgProp);
    pub fn elg_effects_set_content_shape(e: *mut Effects, shape: *const Shape);
    pub fn elg_effects_set_inner_shadow(e: *mut Effects, shadow: *const ElgShadow);
    pub fn elg_effects_set_blur(e: *mut Effects, radius: *const ElgProp, opaque: bool);
    pub fn elg_effects_set_color_filter(
        e: *mut Effects,
        kind: i32,
        amount: ElgProp,
        color: ElgProp,
    );
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

    pub fn elg_layouts_ext(sink: ElgStrSink, ctx: *mut c_void);

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
    pub fn elg_audio_recorder_error(
        recorder: *const AudioRecorder,
        sink: ElgStrSink,
        ctx: *mut c_void,
    );
    pub fn elg_audio_recorder_sample_rate(recorder: *const AudioRecorder) -> u32;
    pub fn elg_audio_recorder_channels(recorder: *const AudioRecorder) -> u32;
    pub fn elg_audio_recorder_samples(recorder: *const AudioRecorder, each: ElgClosure);
    pub fn elg_audio_recorder_save_wav(recorder: *const AudioRecorder, file: ElgStr);
    pub fn elg_audio_recorder_wav(recorder: *const AudioRecorder, each: ElgClosure);
    pub fn elg_audio_recorder_sound(recorder: *const AudioRecorder) -> *mut Sound;
    pub fn elg_camera_devices(each: ElgClosure);
    pub fn elg_camera_new(
        device_id: ElgStr,
        width: u32,
        height: u32,
        mirrored: bool,
    ) -> *mut Camera;
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
    pub fn elg_tray_icon_set_menu(
        icon: *const TrayIcon,
        menu: *const ElgTrayMenuItem,
        count: usize,
    );

    pub fn elg_layouts_platform(sink: ElgStrSink, ctx: *mut c_void);
}

// --- styles.cpp ---
//
// Every family has `elg_<family>_style_clone/free`, `elg_mod_<family>_style` and
// `elg_<family>_config_clone/free`; most have `elg_<family>_style_make_body`.
unsafe extern "C" {
    pub fn elg_button_style_clone(s: *const ButtonStyle) -> *mut ButtonStyle;
    pub fn elg_button_style_free(s: *mut ButtonStyle);
    pub fn elg_mod_button_style(v: *const View, style: *const ButtonStyle) -> *mut View;
    pub fn elg_button_config_clone(c: *const ButtonConfiguration) -> *mut ButtonConfiguration;
    pub fn elg_button_config_free(c: *mut ButtonConfiguration);
    pub fn elg_button_style_make_body(s: *const ButtonStyle, c: *const ButtonConfiguration) -> *mut View;
    pub fn elg_button_style_rust(make_body: ElgClosure) -> *mut ButtonStyle;
    pub fn elg_button_style_builtin(kind: i32) -> *mut ButtonStyle;
    pub fn elg_button_config_label(c: *const ButtonConfiguration) -> *mut View;
    pub fn elg_button_config_state(c: *const ButtonConfiguration, which: i32) -> bool;

    pub fn elg_toggle_style_clone(s: *const ToggleStyle) -> *mut ToggleStyle;
    pub fn elg_toggle_style_free(s: *mut ToggleStyle);
    pub fn elg_mod_toggle_style(v: *const View, style: *const ToggleStyle) -> *mut View;
    pub fn elg_toggle_config_clone(c: *const ToggleConfiguration) -> *mut ToggleConfiguration;
    pub fn elg_toggle_config_free(c: *mut ToggleConfiguration);
    pub fn elg_toggle_style_make_body(s: *const ToggleStyle, c: *const ToggleConfiguration) -> *mut View;
    pub fn elg_toggle_style_rust(make_body: ElgClosure, animation: *const Animation) -> *mut ToggleStyle;
    pub fn elg_switch_toggle_style(options: *const ElgSwitchToggleStyleOptions) -> *mut ToggleStyle;
    pub fn elg_toggle_config_label(c: *const ToggleConfiguration) -> *mut View;
    pub fn elg_toggle_config_state(c: *const ToggleConfiguration, which: i32) -> bool;
    pub fn elg_toggle_config_toggle(c: *const ToggleConfiguration);

    pub fn elg_checkbox_style_clone(s: *const CheckboxStyle) -> *mut CheckboxStyle;
    pub fn elg_checkbox_style_free(s: *mut CheckboxStyle);
    pub fn elg_mod_checkbox_style(v: *const View, style: *const CheckboxStyle) -> *mut View;
    pub fn elg_checkbox_config_clone(c: *const CheckboxConfiguration) -> *mut CheckboxConfiguration;
    pub fn elg_checkbox_config_free(c: *mut CheckboxConfiguration);
    pub fn elg_checkbox_style_make_body(s: *const CheckboxStyle, c: *const CheckboxConfiguration) -> *mut View;
    pub fn elg_checkbox_style_rust(make_body: ElgClosure, animation: *const Animation) -> *mut CheckboxStyle;
    pub fn elg_box_checkbox_style(options: *const ElgBoxCheckboxStyleOptions) -> *mut CheckboxStyle;
    pub fn elg_checkbox_config_label(c: *const CheckboxConfiguration) -> *mut View;
    pub fn elg_checkbox_config_state(c: *const CheckboxConfiguration, which: i32) -> bool;
    pub fn elg_checkbox_config_toggle(c: *const CheckboxConfiguration);

    pub fn elg_number_input_style_clone(s: *const NumberInputStyle) -> *mut NumberInputStyle;
    pub fn elg_number_input_style_free(s: *mut NumberInputStyle);
    pub fn elg_mod_number_input_style(v: *const View, style: *const NumberInputStyle) -> *mut View;
    pub fn elg_number_input_config_clone(c: *const NumberInputConfiguration) -> *mut NumberInputConfiguration;
    pub fn elg_number_input_config_free(c: *mut NumberInputConfiguration);
    pub fn elg_number_input_style_make_body(
        s: *const NumberInputStyle,
        c: *const NumberInputConfiguration,
    ) -> *mut View;
    pub fn elg_number_input_style_rust(make_body: ElgClosure) -> *mut NumberInputStyle;
    pub fn elg_number_input_style_builtin(options: *const ElgBoxNumberInputStyleOptions) -> *mut NumberInputStyle;
    pub fn elg_number_input_config_field(c: *const NumberInputConfiguration) -> *mut View;
    pub fn elg_number_input_config_state(c: *const NumberInputConfiguration, which: i32) -> bool;
    pub fn elg_number_input_config_action(c: *const NumberInputConfiguration, which: i32);

    pub fn elg_progress_style_clone(s: *const ProgressStyle) -> *mut ProgressStyle;
    pub fn elg_progress_style_free(s: *mut ProgressStyle);
    pub fn elg_mod_progress_style(v: *const View, style: *const ProgressStyle) -> *mut View;
    pub fn elg_progress_config_clone(c: *const ProgressConfiguration) -> *mut ProgressConfiguration;
    pub fn elg_progress_config_free(c: *mut ProgressConfiguration);
    pub fn elg_progress_style_make_body(s: *const ProgressStyle, c: *const ProgressConfiguration) -> *mut View;
    pub fn elg_progress_style_rust(make_body: ElgClosure) -> *mut ProgressStyle;
    pub fn elg_progress_style_builtin(kind: i32, options: *const ElgProgressStyleOptions) -> *mut ProgressStyle;
    pub fn elg_progress_config_fraction(c: *const ProgressConfiguration) -> ElgOptLen;

    pub fn elg_text_field_style_clone(s: *const TextFieldStyle) -> *mut TextFieldStyle;
    pub fn elg_text_field_style_free(s: *mut TextFieldStyle);
    pub fn elg_mod_text_field_style(v: *const View, style: *const TextFieldStyle) -> *mut View;
    pub fn elg_text_field_config_clone(c: *const TextFieldConfiguration) -> *mut TextFieldConfiguration;
    pub fn elg_text_field_config_free(c: *mut TextFieldConfiguration);
    pub fn elg_text_field_style_make_body(
        s: *const TextFieldStyle,
        c: *const TextFieldConfiguration,
    ) -> *mut View;
    pub fn elg_text_field_style_rust(make_body: ElgClosure) -> *mut TextFieldStyle;
    pub fn elg_text_field_style_builtin(kind: i32) -> *mut TextFieldStyle;
    pub fn elg_text_field_config_field(c: *const TextFieldConfiguration) -> *mut View;
    pub fn elg_text_field_config_state(c: *const TextFieldConfiguration, which: i32) -> bool;

    pub fn elg_picker_style_clone(s: *const PickerStyle) -> *mut PickerStyle;
    pub fn elg_picker_style_free(s: *mut PickerStyle);
    pub fn elg_mod_picker_style(v: *const View, style: *const PickerStyle) -> *mut View;
    pub fn elg_picker_config_clone(c: *const PickerConfiguration) -> *mut PickerConfiguration;
    pub fn elg_picker_config_free(c: *mut PickerConfiguration);
    pub fn elg_picker_style_make_body(s: *const PickerStyle, c: *const PickerConfiguration) -> *mut View;
    pub fn elg_picker_style_rust(make_body: ElgClosure) -> *mut PickerStyle;
    pub fn elg_picker_style_builtin(kind: i32) -> *mut PickerStyle;
    pub fn elg_picker_config_count(c: *const PickerConfiguration) -> usize;
    pub fn elg_picker_config_label(c: *const PickerConfiguration, index: usize) -> ElgStr;
    pub fn elg_picker_config_selected(c: *const PickerConfiguration) -> usize;
    pub fn elg_picker_config_select(c: *const PickerConfiguration, index: usize);

    pub fn elg_slider_style_clone(s: *const SliderStyle) -> *mut SliderStyle;
    pub fn elg_slider_style_free(s: *mut SliderStyle);
    pub fn elg_mod_slider_style(v: *const View, style: *const SliderStyle) -> *mut View;
    pub fn elg_slider_config_clone(c: *const SliderConfiguration) -> *mut SliderConfiguration;
    pub fn elg_slider_config_free(c: *mut SliderConfiguration);
    pub fn elg_slider_style_make_part(s: *const SliderStyle, c: *const SliderConfiguration, part: i32) -> *mut View;
    pub fn elg_slider_style_rust(track: ElgClosure, fill: ElgClosure, thumb: ElgClosure) -> *mut SliderStyle;
    pub fn elg_knob_slider_style(options: *const ElgKnobSliderStyleOptions) -> *mut SliderStyle;
    pub fn elg_slider_config_fraction(c: *const SliderConfiguration) -> f32;
    pub fn elg_slider_config_state(c: *const SliderConfiguration, which: i32) -> bool;

    pub fn elg_navigation_stack_style_clone(s: *const NavigationStackStyle) -> *mut NavigationStackStyle;
    pub fn elg_navigation_stack_style_free(s: *mut NavigationStackStyle);
    pub fn elg_mod_navigation_stack_style(v: *const View, style: *const NavigationStackStyle) -> *mut View;
    pub fn elg_navigation_stack_config_clone(
        c: *const NavigationStackConfiguration,
    ) -> *mut NavigationStackConfiguration;
    pub fn elg_navigation_stack_config_free(c: *mut NavigationStackConfiguration);
    pub fn elg_navigation_stack_style_make_body(
        s: *const NavigationStackStyle,
        c: *const NavigationStackConfiguration,
    ) -> *mut View;
    pub fn elg_navigation_stack_style_rust(make_body: ElgClosure) -> *mut NavigationStackStyle;
    pub fn elg_navigation_stack_style_builtin(
        options: *const ElgBarNavigationStackStyleOptions,
    ) -> *mut NavigationStackStyle;
    pub fn elg_navigation_stack_config_pages(c: *const NavigationStackConfiguration) -> *mut View;
    pub fn elg_navigation_stack_config_titles(
        c: *const NavigationStackConfiguration,
        each: ElgStrSink,
        ctx: *mut c_void,
    );
    pub fn elg_navigation_stack_config_can_go_back(c: *const NavigationStackConfiguration) -> bool;
    pub fn elg_navigation_stack_config_back(c: *const NavigationStackConfiguration);

    pub fn elg_popover_style_clone(s: *const PopoverStyle) -> *mut PopoverStyle;
    pub fn elg_popover_style_free(s: *mut PopoverStyle);
    pub fn elg_mod_popover_style(v: *const View, style: *const PopoverStyle) -> *mut View;
    pub fn elg_popover_config_clone(c: *const PopoverConfiguration) -> *mut PopoverConfiguration;
    pub fn elg_popover_config_free(c: *mut PopoverConfiguration);
    pub fn elg_popover_style_make_body(s: *const PopoverStyle, c: *const PopoverConfiguration) -> *mut View;
    pub fn elg_popover_style_rust(make_body: ElgClosure, motion: ElgPopupMotion) -> *mut PopoverStyle;
    pub fn elg_popover_style_builtin(kind: i32) -> *mut PopoverStyle;
    pub fn elg_popover_config_content(c: *const PopoverConfiguration) -> *mut View;
    pub fn elg_popover_config_is_padded(c: *const PopoverConfiguration) -> bool;

    pub fn elg_tooltip_style_clone(s: *const TooltipStyle) -> *mut TooltipStyle;
    pub fn elg_tooltip_style_free(s: *mut TooltipStyle);
    pub fn elg_mod_tooltip_style(v: *const View, style: *const TooltipStyle) -> *mut View;
    pub fn elg_tooltip_config_clone(c: *const TooltipConfiguration) -> *mut TooltipConfiguration;
    pub fn elg_tooltip_config_free(c: *mut TooltipConfiguration);
    pub fn elg_tooltip_style_make_body(s: *const TooltipStyle, c: *const TooltipConfiguration) -> *mut View;
    pub fn elg_tooltip_style_rust(make_body: ElgClosure, motion: ElgPopupMotion) -> *mut TooltipStyle;
    pub fn elg_tooltip_style_builtin(kind: i32) -> *mut TooltipStyle;
    pub fn elg_tooltip_config_content(c: *const TooltipConfiguration) -> *mut View;

    pub fn elg_menu_style_clone(s: *const MenuStyle) -> *mut MenuStyle;
    pub fn elg_menu_style_free(s: *mut MenuStyle);
    pub fn elg_mod_menu_style(v: *const View, style: *const MenuStyle) -> *mut View;
    pub fn elg_menu_config_clone(c: *const MenuConfiguration) -> *mut MenuConfiguration;
    pub fn elg_menu_config_free(c: *mut MenuConfiguration);
    pub fn elg_menu_item_config_clone(c: *const MenuItemConfiguration) -> *mut MenuItemConfiguration;
    pub fn elg_menu_item_config_free(c: *mut MenuItemConfiguration);
    pub fn elg_menu_style_make_body(s: *const MenuStyle, c: *const MenuConfiguration) -> *mut View;
    pub fn elg_menu_style_rust(make_body: ElgClosure, make_item: ElgClosure, motion: ElgPopupMotion) -> *mut MenuStyle;
    pub fn elg_menu_style_builtin(kind: i32) -> *mut MenuStyle;
    pub fn elg_menu_style_make_item(s: *const MenuStyle, c: *const MenuItemConfiguration) -> *mut View;
    pub fn elg_menu_config_items(c: *const MenuConfiguration) -> *mut View;
    pub fn elg_menu_item_config_view(c: *const MenuItemConfiguration, which: i32) -> *mut View;
    pub fn elg_menu_item_config_state(c: *const MenuItemConfiguration, which: i32) -> bool;

    pub fn elg_row_style_clone(s: *const RowStyle) -> *mut RowStyle;
    pub fn elg_row_style_free(s: *mut RowStyle);
    pub fn elg_mod_row_style(v: *const View, style: *const RowStyle) -> *mut View;
    pub fn elg_row_config_clone(c: *const RowConfiguration) -> *mut RowConfiguration;
    pub fn elg_row_config_free(c: *mut RowConfiguration);
    pub fn elg_row_style_make_body(s: *const RowStyle, c: *const RowConfiguration) -> *mut View;
    pub fn elg_row_style_rust(make_body: ElgClosure) -> *mut RowStyle;
    pub fn elg_row_style_builtin(options: *const ElgHighlightRowStyleOptions) -> *mut RowStyle;
    pub fn elg_row_config_content(c: *const RowConfiguration) -> *mut View;
    pub fn elg_row_config_axis(c: *const RowConfiguration) -> i32;
    pub fn elg_row_config_state(c: *const RowConfiguration, which: i32) -> bool;

    pub fn elg_layouts_styles(sink: ElgStrSink, ctx: *mut c_void);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::mem::{align_of, size_of};

    unsafe extern "C" fn append(ctx: *mut c_void, ptr: *const c_char, len: usize) {
        let text = unsafe { &mut *ctx.cast::<String>() };
        let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) };
        text.push_str(std::str::from_utf8(bytes).unwrap());
    }

    macro_rules! layouts {
        ($($name:literal => $ty:ty),* $(,)?) => {
            BTreeMap::from([$(($name, (size_of::<$ty>(), align_of::<$ty>()))),*])
        };
    }

    /// Every struct shared with the shim has the C++ side's size and alignment, and every struct
    /// the shim reports has a Rust declaration.
    #[test]
    fn struct_layouts_match_the_shim() {
        let mut text = String::new();
        for layouts in [
            elg_layouts_core,
            elg_layouts_views,
            elg_layouts_ext,
            elg_layouts_platform,
            elg_layouts_styles,
        ] {
            unsafe { layouts(append, (&raw mut text).cast()) };
        }
        let native: BTreeMap<&str, (usize, usize)> = text
            .lines()
            .map(|line| {
                let mut parts = line.split(' ');
                let name = parts.next().unwrap();
                let mut number = || parts.next().unwrap().parse::<usize>().unwrap();
                (name, (number(), number()))
            })
            .collect();

        use gpu_image::{Fence, Frame, Release};
        let rust = layouts! {
            "ElgStr" => ElgStr,
            "ElgBytes" => ElgBytes,
            "ElgClosure" => ElgClosure,
            "ElgColor" => ElgColor,
            "ElgSize" => ElgSize,
            "ElgPixelSize" => ElgPixelSize,
            "ElgPoint" => ElgPoint,
            "ElgRect" => ElgRect,
            "ElgFont" => ElgFont,
            "ElgGradientStop" => ElgGradientStop,
            "ElgShapeStyle" => ElgShapeStyle,
            "ElgEdgeInsets" => ElgEdgeInsets,
            "ElgProp" => ElgProp,
            "ElgProposedSize" => ElgProposedSize,
            "ElgOptLen" => ElgOptLen,
            "ElgAlignment" => ElgAlignment,
            "ElgKeyEvent" => ElgKeyEvent,
            "ElgPointerEvent" => ElgPointerEvent,
            "ElgScrollEvent" => ElgScrollEvent,
            "ElgClickEvent" => ElgClickEvent,
            "ElgDragEvent" => ElgDragEvent,
            "ElgMagnifyEvent" => ElgMagnifyEvent,
            "ElgRotateEvent" => ElgRotateEvent,
            "ElgTextInputEvent" => ElgTextInputEvent,
            "ElgEventResult" => ElgEventResult,
            "ElgFloatingPlacement" => ElgFloatingPlacement,
            "ElgPopupMotion" => ElgPopupMotion,
            "ElgHoverDelay" => ElgHoverDelay,
            "ElgShadow" => ElgShadow,
            "ElgStrokeStyle" => ElgStrokeStyle,
            "ElgRunStyle" => ElgRunStyle,
            "ElgImageSource" => ElgImageSource,
            "ElgWindowDesc" => ElgWindowDesc,
            "ElgPose" => ElgPose,
            "ElgTextFieldOptions" => ElgTextFieldOptions,
            "ElgNumberInputOptions" => ElgNumberInputOptions,
            "ElgOptNumber" => ElgOptNumber,
            "ElgForEachItem" => ElgForEachItem,
            "ElgStackSeparators" => ElgStackSeparators,
            "ElgSplitViewOptions" => ElgSplitViewOptions,
            "ElgColumnWidth" => ElgColumnWidth,
            "ElgNavigationSplitViewOptions" => ElgNavigationSplitViewOptions,
            "ElgGestureOptions" => ElgGestureOptions,
            "ElgDragOptions" => ElgDragOptions,
            "ElgLongPressOptions" => ElgLongPressOptions,
            "ElgFloatingOptions" => ElgFloatingOptions,
            "ElgTooltipOptions" => ElgTooltipOptions,
            "ElgItemKey" => ElgItemKey,
            "ElgItemKeys" => ElgItemKeys,
            "ElgSwitchToggleStyleOptions" => ElgSwitchToggleStyleOptions,
            "ElgBoxCheckboxStyleOptions" => ElgBoxCheckboxStyleOptions,
            "ElgBoxNumberInputStyleOptions" => ElgBoxNumberInputStyleOptions,
            "ElgProgressStyleOptions" => ElgProgressStyleOptions,
            "ElgKnobSliderStyleOptions" => ElgKnobSliderStyleOptions,
            "ElgBarNavigationStackStyleOptions" => ElgBarNavigationStackStyleOptions,
            "ElgHighlightRowStyleOptions" => ElgHighlightRowStyleOptions,
            "ElgTextRun" => ElgTextRun,
            "ElgPolyline" => ElgPolyline,
            "ElgAsyncImagePhase" => ElgAsyncImagePhase,
            "ElgIconFont" => ElgIconFont,
            "ElgIconWeight" => ElgIconWeight,
            "ElgIconVariant" => ElgIconVariant,
            "ElgFrameOptions" => ElgFrameOptions,
            "ElgDropFiles" => ElgDropFiles,
            "ElgDropText" => ElgDropText,
            "ElgPreviewDesc" => ElgPreviewDesc,
            "ElgGridItem" => ElgGridItem,
            "ElgGridTrack" => ElgGridTrack,
            "ElgGridDesc" => ElgGridDesc,
            "ElgLayoutCall" => ElgLayoutCall,
            "ElgLayoutModifierCall" => ElgLayoutModifierCall,
            "ElgEventCall" => ElgEventCall,
            "ElgOptHoverDelay" => ElgOptHoverDelay,
            "ElgDevice" => ElgDevice,
            "ElgRenderBuffer" => ElgRenderBuffer,
            "ElgSamples" => ElgSamples,
            "ElgFileFilter" => ElgFileFilter,
            "ElgPaths" => ElgPaths,
            "ElgTrayMenuItem" => ElgTrayMenuItem,
            "ElgGpuImageFence" => Fence,
            "ElgGpuImageRelease" => Release,
            "ElgGpuImageFrame" => Frame,
        };

        let mismatches: Vec<String> = native
            .keys()
            .chain(rust.keys())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter_map(|name| match (native.get(name), rust.get(name)) {
                (Some(c), Some(r)) if c == r => None,
                (c, r) => Some(format!("{name}: C++ (size, align) {c:?}, Rust {r:?}")),
            })
            .collect();
        assert!(mismatches.is_empty(), "\n{}", mismatches.join("\n"));
    }
}
