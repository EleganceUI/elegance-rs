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
