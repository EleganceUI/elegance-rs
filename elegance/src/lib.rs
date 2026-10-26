//! Rust bindings to Elegance, a declarative GPU UI library, with a SwiftUI-style DSL.
//!
//! ```ignore
//! use elegance::prelude::*;
//!
//! #[component]
//! fn Counter() -> impl IntoView {
//!     let mut count = State::new(0);
//!     elg! {
//!         VStack(spacing: 16) {
//!             Text("{count}").font(Font::system(56.0, .bold))
//!             HStack {
//!                 Button("−", move || count -= 1)
//!                 Button("+", move || count += 1)
//!             }
//!         }
//!     }
//! }
//!
//! fn main() -> Result<std::process::ExitCode, elegance::Error> {
//!     App::new().window(WindowDesc::new("Counter").size(360, 280), Counter).run()
//! }
//! ```
//!
//! # The `elg!` DSL
//!
//! | DSL | Means |
//! |---|---|
//! | `Name(a, b)` | `Name::elg_new((a, b))`: an element; overloads are tuple impls of [`dsl::ElgNew`] |
//! | `Name(a, key: v)` | the same, then the builder call `.key(v)` |
//! | `Name { ... }` | its children (a stack's views, a button's label, ...) |
//! | `ForEach(xs, id: \|x\| x.id) { x in ... }` | a content closure for each item |
//! | `if cond { ... } else { ... }` | a reactive `If` |
//! | `match value { pat => View(), ... }` | the view for the current value, rebuilt when it changes |
//! | `.leading`, `.accent`, `.bold` | [`dot`] shorthand, converted to the parameter's type |
//! | `"{count} left"` | a reactive string: `move \|\| format!("{count} left")` |
//! | `{ expr }` | any Rust value that is [`IntoView`] |
//! | `(layout) { ... }` | a custom [`Layout`] over the children |
//! | `.m(a, key: v)` | the modifier `.m_with(a, \|o\| o.key(v))` |

#![allow(clippy::type_complexity)]

extern crate self as elegance;

mod ffi;

/// An image compiled into the program from the crate's `assets/` folder:
/// `Image(asset!("images/logo.png"))`. It works wherever the views run (the app, a preview,
/// an installed copy), unlike a relative file path, which Elegance resolves against the current
/// directory. The path is relative to `<crate>/assets/`.
#[macro_export]
macro_rules! asset {
    ($path:literal) => {
        $crate::ImageSource::data(
            &::std::include_bytes!(::std::concat!(
                ::std::env!("CARGO_MANIFEST_DIR"),
                "/assets/",
                $path
            ))[..],
        )
    };
}

pub mod animation;
pub mod app;
pub mod color;
pub mod desktop;
pub mod dot;
pub mod dsl;
pub mod environment;
pub mod error;
pub mod events;
pub mod font;
pub mod geometry;
pub mod gpu_image;
pub mod graphics;
pub mod grid;
pub mod hot_reload;
pub mod http;
pub mod image;
pub mod layout;
pub mod media;
pub mod modifier;
pub mod navigation;
pub mod path;
pub mod preview;
pub mod prop;
pub mod reactive;
pub mod shapes;
pub mod styles;
pub mod text;
pub mod view;
pub mod views;
pub mod window;

pub use items::*;

// The crate root's types, functions and macros, in a module of their own so the prelude can take
// them all without also taking the modules (`image`, `path`, `text`, ...).
mod items {
    pub use crate::asset;
    pub use elegance_macros::{component, elg, elg_builder, preview, DotValues};

    pub use crate::animation::{with_animation, Animation, AnimationValue, Pose, Transition};
    pub use crate::app::{post_to_ui_thread, version, App, AppHandle, TitleBar, WindowDesc};
    pub use crate::color::{
        AngularGradient, Color, ColorScheme, ColorTokens, EllipticalGradient, GradientStop,
        LinearGradient, Material, RadialGradient, ShapeStyle, Theme, ThemeColor,
    };
    pub use crate::environment::{Environment, EnvironmentKey, EnvironmentValues};
    pub use crate::error::{Error, ErrorKind};
    pub use crate::events::{
        DragEvent, EventResult, GesturePhase, HoverDelay, Key, KeyEvent, KeyEventKind,
        KeyModifiers, MagnifyEvent, MouseButton, PixelSize, PointerEvent, PointerEventKind,
        PointerStyle, RotateEvent, ScrollEvent, TextInputEvent, TextInputKind,
    };
    pub use crate::font::{register_emoji_font, register_font, register_font_data, Font, Weight};
    pub use crate::geometry::{
        aligned_position, Alignment, Axis, ContentMode, CornerRadii, EdgeInsets, Edges,
        HorizontalAlignment, Len, Number, Point, ProposedSize, Rect, Size, UnitPoint,
        VerticalAlignment, DEFAULT_STACK_SPACING, INFINITY,
    };
    pub use crate::gpu_image::{GpuImage, GpuImageOrigin, GpuImageView};
    pub use crate::graphics::{Canvas, GraphicsContext};
    pub use crate::grid::{
        grid_tracks_length, resolve_grid_tracks, Grid, GridItem, GridItemKind, GridLayout,
        GridRow, GridTrack, HGrid, HGridLayout, VGrid, VGridLayout,
    };
    #[cfg(feature = "dynamic")]
    pub use crate::hot_reload::HotReload;
    pub use crate::hot_reload::HotReloadShow;
    pub use crate::image::{
        register_icon_set, set_default_icon_set, AsyncImage, AsyncImagePhase, IconFont,
        IconSetOptions, IconVariant, Image, ImageSource, Label,
    };
    pub use crate::layout::{
        Cached, CachedLayout, HStackLayout, Layout, LayoutChild, LayoutValueKey, VStackLayout,
        ZStackLayout,
    };
    pub use crate::media::{
        audio_input_devices, audio_output_devices, camera_devices, AudioDevice, AudioPlayer,
        AudioRecorder, AudioRecorderOptions, AudioStream, AudioStreamOptions, Camera,
        CameraDevice, CameraOptions, CameraView, PlayOptions, Sound, VideoControls, VideoFeed,
        VideoFeedView, VideoFormat, VideoPlayer, VideoView,
    };
    pub use crate::modifier::{
        AsDecoration, AsEffect, AsEnvironment, AsEvent, AsLayout, DecorationModifier,
        EffectModifier, Effects, EnvironmentModifier, EventHandler, EventModifier, LayoutModifier,
        Modifier, Placement, ViewModifier,
    };
    pub use crate::navigation::{
        ColumnWidth, HSplitView, NavigationSplitView, NavigationStack, VSplitView,
    };
    pub use crate::path::{FillRule, LineCap, LineJoin, Path, Polyline, StrokeStyle};
    pub use crate::prop::{IntoProp, Prop, ResolveColor, ResolveStyle};
    pub use crate::reactive::{
        batch, computed, effect, untrack, Binding, Computed, IntoBinding, ReadRef, State, WriteRef,
    };
    pub use crate::shapes::{
        Capsule, Circle, CustomShape, PathShape, PathShapeOptions, Rectangle, RoundedRectangle,
        Shape,
    };
    pub use crate::styles::{
        AnyButtonStyle, AnyTextFieldStyle, AnyToggleStyle, BorderedButtonStyle,
        ButtonConfiguration, ButtonStyle, CheckboxToggleStyle, IntoButtonStyle, IntoTextFieldStyle,
        IntoToggleStyle, PlainButtonStyle, PlainTextFieldStyle, ProminentButtonStyle,
        RoundedBorderTextFieldStyle, SwitchToggleStyle, TextFieldConfiguration, TextFieldStyle,
        ToggleConfiguration, ToggleStyle,
    };
    pub use crate::text::{RunStyle, StyledText, Text, TextAlignment, TextRun, TextSpan};
    pub use crate::view::{
        AccessibilityRole, AnchorOptions, AspectRatioOptions, BlendMode, BlurOptions, ColorFilter,
        Component, DecorationOptions, DropOptions, FrameOptions, IntoView, PopoverEdge,
        PopoverOptions, Shadow, TapOptions, View, ViewExt,
    };
    pub use crate::views::{
        Button, Divider, EmptyView, ForEach, HStack, If, IntoItemKey, IntoLabel, ItemKey,
        ItemsSource, LazyHStack, LazyVStack, List, Menu, MenuButton, MenuEdge, MenuItem, Picker,
        PickerStyle, ProgressView, ScrollAxes, ScrollPosition, ScrollView, SecureField, Slider,
        Spacer, Stepper, Submenu, TextField, Toggle, VStack, ZStack,
    };
    pub use crate::window::{WindowControls, WindowControlsStyle, WindowHandle};
}

/// Everything for writing views: the crate root's types, functions and macros (not its modules),
/// and the environment keys.
pub mod prelude {
    pub use crate::environment::{
        ButtonStyleKey, ColorSchemeKey, DisabledKey, FontKey, ForegroundKey, HoverDelayKey,
        TextFieldStyleKey, ThemeKey, TintKey, ToggleStyleKey, WindowKey,
    };
    pub use crate::items::*;
}

/// Used by the macros' expansions.
#[doc(hidden)]
pub mod __private {
    pub use crate::dsl::{
        if_view, match_view, ElgChildren, ElgNew, Interpolated, Missing, ReadPlain, ReadReactive,
        Reader,
    };
    pub use crate::preview::PreviewItem;
    pub use crate::view::{Component, IntoView, View, ViewExt};
    pub use inventory;

    /// A `.name` shorthand's value, a clone of its namespace field. A function rather than
    /// `Clone::clone` in the expansion, which clippy flags as `clone_on_copy` in user code.
    #[inline]
    pub fn dot_value<T: Clone>(value: &T) -> T {
        value.clone()
    }
}
