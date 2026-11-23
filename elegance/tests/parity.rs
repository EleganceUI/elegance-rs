// API that mirrors Elegance's C++ API (include/elg/*.h): built for real where that needs no App,
// type-checked otherwise.

#![allow(dead_code)]

use elegance::prelude::*;

#[test]
fn weights_between_the_named_ones_are_kept() {
    // font.h: any weight in 1...1000, e.g. Weight{550}.
    assert_eq!(Weight::new(550).value(), 550);
    assert_eq!(Weight::from(550u16), Weight::new(550));
    assert_eq!(Weight::new(0).value(), 1);
    assert_eq!(Weight::new(5000).value(), 1000);
    assert_eq!(Weight::default(), Weight::Regular);
    assert!(Weight::Semibold < Weight::new(650) && Weight::new(650) < Weight::Bold);
    assert_eq!(Font::system(12.0, Weight::new(550)).font_weight(), Weight::new(550));
}

#[test]
fn the_platform_is_the_one_built_for() {
    let expected = if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOS
    } else if cfg!(target_os = "mikros") {
        Platform::Mikros
    } else {
        Platform::Linux
    };
    assert_eq!(current_platform(), expected);
}

#[test]
fn an_empty_window_handle() {
    let handle = WindowHandle::default();
    assert!(handle.is_none());
    assert_eq!(handle, WindowHandle::none());
    assert_eq!(format!("{handle:?}"), "WindowHandle(none)");
}

#[test]
fn props_without_an_environment() {
    // prop.h: get() works for props that don't need an environment.
    assert_eq!(Prop::Value(2.0f32).try_get(), Some(2.0));
    assert_eq!(Prop::from_fn(|| 3.0f32).try_get(), Some(3.0));
    assert_eq!(Prop::<Color>::Theme(ThemeColor::Accent).try_get(), None);
    assert_eq!(Prop::from_env(|_| 1.0f32).try_get(), None);
}

#[test]
fn image_sources_default_to_scale_1_and_elegances_limits() {
    let source = ImageSource::default();
    assert_eq!((source.scale, source.max_pixels, source.max_side), (1.0, 0, 0));
    let limited = ImageSource::file("big.png").max_pixels(1 << 20).max_side(2048);
    assert_eq!((limited.max_pixels, limited.max_side), (1 << 20, 2048));
}

#[test]
fn icon_fonts_from_paths_and_data() {
    let file = IconFont::from(String::from("icons.ttf"));
    assert_eq!(file.file(), Some(std::path::Path::new("icons.ttf")));
    assert_eq!(IconFont::from(std::path::Path::new("a.ttf")).data(), None);
    assert_eq!(IconFont::from(&b"font"[..]).data(), Some(&b"font"[..]));
}

#[test]
fn layouts_are_layouts() {
    // layout.h, grid.h: the built-in layouts are Layouts.
    fn takes_layout(_: &dyn Layout) {}
    takes_layout(&VStackLayout::default());
    takes_layout(&HStackLayout::default());
    takes_layout(&ZStackLayout::default());
    takes_layout(&VGridLayout::default());
    takes_layout(&HGridLayout::default());
    takes_layout(&GridLayout::default());
    assert_eq!(VStackLayout::default().spacer_axis(), Some(Axis::Vertical));
    assert_eq!(Layout::spacer_axis(&HStackLayout::default()), Some(Axis::Horizontal));
    assert_eq!(ZStackLayout::default().spacer_axis(), None);
    assert_eq!(VGridLayout::default().columns, vec![GridItem::flexible()]);
    assert_eq!(VGridLayout::default().alignment, Alignment::CENTER);
}

#[test]
fn views_outside_the_dsl() {
    // Each crosses into Elegance; a C++ exception would panic here.
    let views = [
        If(true, Text("then"), ()),
        If(false, Text("then"), Text("else")),
        Submenu("More", || Menu::new().item(MenuItem::new("A", || {}))),
        MenuItem::without_action("Close only").into_view(),
        VStackLayout::default().view(vec![Text("a").into_view()]),
        Layout::view(VGridLayout::default(), vec![Text("a").into_view(), Text("b").into_view()]),
        Text("masked").modifier(MaskModifier { view: Rectangle().into_view(), alignment: Alignment::CENTER }),
        Text("blurred").modifier(BlurModifier { radius: Prop::Value(4.0), opaque: false }),
        Text("turned").modifier(RotationModifier { degrees: Prop::Value(10.0), anchor: UnitPoint::CENTER }),
        Text("scaled").modifier(ScaleModifier { x: Prop::Value(2.0), y: Prop::Value(1.0), anchor: UnitPoint::CENTER }),
        Text("grouped").modifier(CompositingGroupModifier),
        Text("dragged").modifier(DragGestureModifier { gesture: DragGesture::new(), options: GestureOptions::default() }),
        Text("pointer").modifier(PointerStyleModifier(Prop::Value(PointerStyle::Pointer))),
    ];
    assert!(views.iter().all(|view| !view.is_empty()));
}

/// Type checks for API that needs a running App or a view's body.
fn type_checks() -> impl IntoView {
    let title = State::new(String::from("Save"));
    let todos = State::new(vec![(1, String::from("milk"))]);
    let count = State::new(0);
    struct NotComparable;
    let _computed: Computed<NotComparable> = Computed::always_notify(|| NotComparable);
    let _motion = PopupMotion::none().animates();
    VStack::new()
        .child(Button::new(title, || {}))
        .child(Button::new(Prop::from_fn(move || format!("Save {}", count.get())), || {}))
        .child(ForEach::new(todos).id(|t| t.0).content_signal(|t| Text(move || t.get().1)))
        .child(Text("sized").on_size_change(|_| {}))
        .child(AsyncImage::new("https://example.com/a.png").max_pixels(1 << 20).max_side(1024))
}

#[test]
fn type_checks_compile() {
    let _ = type_checks as fn() -> _;
}

// --- Milestone 13: the C API's coverage of the C++ API ---

#[test]
fn backdrop_fills_and_window_materials() {
    let fill = BackdropFill::new(20.0).saturation(1.8).noise(0.02);
    assert_eq!((fill.blur, fill.saturation, fill.brightness, fill.noise), (20.0, 1.8, 0.0, 0.02));
    assert_eq!(BackdropFill::default().saturation, 1.0);
    assert_eq!(ShapeStyle::from(fill), ShapeStyle::Backdrop(fill));
    assert_eq!(WindowDesc::default().material, WindowMaterial::None);
    assert_eq!(WindowDesc::new("Glass").material(WindowMaterial::Mica).material, WindowMaterial::Mica);
    let pose = Pose::default();
    assert_eq!((pose.anchor_x, pose.anchor_y), (0.5, 0.5));
}

#[test]
fn menu_bar_items() {
    let save = MenuBarItem::new("Save", || {}).shortcut(KeyboardShortcut::new(Key::S));
    assert!(save.enabled);
    assert_eq!(save.shortcut.unwrap().modifiers, KeyModifiers::primary());
    assert_eq!(MenuBarItem::role(MenuRole::Quit).role, MenuRole::Quit);
    assert!(MenuBarItem::default().enabled);
}

#[test]
fn event_handlers_bubble_by_default() {
    struct Plain;
    impl EventHandler for Plain {}
    assert_eq!(Plain.phase(), EventPhase::Bubble);
    assert_eq!(EventPhase::default(), EventPhase::Bubble);
    assert_eq!(KeyEventModifier::new(|_| false).phase, EventPhase::Bubble);
}

#[test]
fn a_default_timer_is_never_active() {
    let timer = Timer::default();
    assert!(!timer.active());
    timer.cancel();
    assert!(!timer.clone().active());
}

#[test]
fn an_empty_environment_reads_defaults() {
    let environment = EnvironmentValues::new();
    assert_eq!(environment.color_scheme(), ColorScheme::Light);
    let _ = environment.get::<CheckboxStyleKey>();
    let _ = environment.get::<RowStyleKey>().clone();
}

struct Compact;
impl EnvironmentKey for Compact {
    type Value = bool;
    const NAME: Option<&'static str> = Some("elegance-rs.tests.compact");
    fn default_value(_: &EnvironmentValues) -> bool {
        false
    }
}

struct Label;
impl EnvironmentKey for Label {
    type Value = String;
    const NAME: Option<&'static str> = Some("elegance-rs.tests.label");
    fn default_value(_: &EnvironmentValues) -> String {
        String::from("none")
    }
}

/// A named key of a type Elegance doesn't keep stays Rust's own.
struct Sizes;
impl EnvironmentKey for Sizes {
    type Value = Vec<i32>;
    const NAME: Option<&'static str> = Some("elegance-rs.tests.sizes");
    fn default_value(_: &EnvironmentValues) -> Vec<i32> {
        Vec::new()
    }
}

#[test]
fn named_environment_keys() {
    let mut environment = EnvironmentValues::new();
    assert!(!environment.get::<Compact>());
    assert_eq!(environment.get::<Label>(), "none");
    environment.set::<Compact>(true);
    environment.set::<Label>("set");
    environment.set::<Sizes>(vec![1, 2]);
    assert!(environment.get::<Compact>());
    assert_eq!(environment.get::<Label>(), "set");
    assert_eq!(environment.get::<Sizes>(), vec![1, 2]);
    // Kept by Elegance under the name, so copies made later see it.
    assert!(environment.clone().get::<Compact>());
}

/// Type checks for the new API that needs a running App or a view's body.
fn milestone_13_type_checks(app: &App) -> impl IntoView {
    app.menu_bar(vec![MenuBarItem::menu("File", vec![MenuBarItem::role(MenuRole::Quit)])]);
    set_badge(Some("3"));
    set_progress(Some(0.5), ProgressState::Normal);
    request_attention(false);
    let _ = system_accent_color();
    let _ = after(1.0, || {});
    let _ = every(1.0, || {});
    let _ = Animation::timing_curve(0.55, 0.1, 0.45, 0.9, 1.4).repeat_count(3, true).repeat_forever(false);
    let _ = DefaultPopoverStyle.into_popover_style().motion().animates();
    let _ = SwitchToggleStyle::default().into_toggle_style().animation();
    struct Fade;
    impl EffectModifier for Fade {
        fn apply(&self, effects: &mut Effects) {
            let opacity = effects.opacity();
            effects.set_opacity(Prop::from_env(move |env| opacity.get(env) * 0.5));
            let _ = (effects.blur(), effects.shadow(), effects.scale(), effects.color_filter(), effects.clip_shape());
            effects.add_on_size_change(|_| {});
        }
    }
    VStack::new()
        .child(Text("selectable").selectable(true))
        .child(Text("appears").on_appear(|| {}).on_disappear(|| {}))
        .child(Text("captures").on_pointer_capture(|_| false).on_key_capture(|_| false).on_key_press_capture(Key::Escape, || {}))
        .child(Rectangle().fill(BackdropFill::new(12.0)))
        .child(Text("styled").environment::<CheckboxStyleKey>(Prop::from_fn(|| BoxCheckboxStyle::default().into_checkbox_style())))
        .child(Text("faded").modifier(AsEffect(Fade)))
}

#[test]
fn milestone_13_type_checks_compile() {
    let _ = milestone_13_type_checks;
}

// --- Milestone 13, part 3: values Rust had hard-coded now come from Elegance ---

#[test]
fn values_from_elegance_match_the_ones_rust_documents() {
    assert_eq!(default_stack_spacing(), DEFAULT_STACK_SPACING);
    assert_eq!(KeyModifiers::primary(), if cfg!(target_os = "macos") { KeyModifiers::SUPER } else { KeyModifiers::CONTROL });
    // font.h's text styles: the system font, at these sizes and weights.
    let styles = [
        (Font::large_title(), 30.0, Weight::Bold),
        (Font::title(), 24.0, Weight::Semibold),
        (Font::title2(), 20.0, Weight::Semibold),
        (Font::title3(), 17.0, Weight::Semibold),
        (Font::headline(), 15.0, Weight::Semibold),
        (Font::body(), 15.0, Weight::Regular),
        (Font::callout(), 14.0, Weight::Regular),
        (Font::subheadline(), 13.0, Weight::Regular),
        (Font::footnote(), 12.0, Weight::Regular),
        (Font::caption(), 12.0, Weight::Regular),
        (Font::caption2(), 11.0, Weight::Regular),
    ];
    for (font, size, weight) in styles {
        assert_eq!((font.family(), font.size(), font.font_weight(), font.is_italic()), ("", size, weight, false));
    }
}

#[test]
fn grid_track_lengths_include_the_spacing_between_tracks() {
    let tracks = [
        GridTrack { size: 10.0, spacing: 4.0, alignment: None },
        GridTrack { size: 20.0, spacing: 6.0, alignment: Some(Alignment::CENTER) },
    ];
    assert_eq!(grid_tracks_length(&tracks), 34.0);
    assert_eq!(grid_tracks_length(&[]), 0.0);
}

#[test]
fn gpu_images_know_their_origin() {
    assert!(!GpuImage::new(4, 4, GpuImageOrigin::TopLeft).unwrap().flipped());
    assert!(GpuImage::new(4, 4, GpuImageOrigin::BottomLeft).unwrap().flipped());
}
