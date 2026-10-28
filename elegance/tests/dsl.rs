// Compile checks for the elg! DSL and the API it reaches. The views are built only to type-check;
// nothing here opens a window.

#![allow(dead_code)]

use elegance::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum Size3 {
    S,
    M,
    L,
}

#[derive(Clone, PartialEq)]
enum Route {
    Detail(i64),
}

/// A component with required and optional props.
#[component]
fn Badge(title: String, #[prop(default = 1)] count: i32, note: Option<String>) -> impl IntoView {
    elg! {
        HStack(spacing: 4) {
            Text(title.clone()).bold()
            Text(format!("{count}")).foreground_style(.secondary)
            { note.clone().map(Text) }
        }
    }
}

#[derive(Clone)]
struct Card {
    title: String,
}

impl Component for Card {
    fn body(&self) -> impl IntoView {
        elg! { Text(self.title.clone()).padding(16).background_in(.surface, RoundedRectangle(12)) }
    }
}

struct Pill;
impl ButtonStyle for Pill {
    fn make_body(&self, button: ButtonConfiguration) -> View {
        let state = button.clone();
        elg! {
            {button.label()}
                .padding_with(|o| o.horizontal(14).vertical(6))
                .background_in(move || if state.is_pressed() { ThemeColor::Accent } else { ThemeColor::SurfaceRaised }, Capsule())
        }
        .into_view()
    }
}

struct Flow {
    spacing: f32,
}
impl Layout for Flow {
    fn size_that_fits(&self, proposal: ProposedSize, children: &[LayoutChild]) -> elegance::Size {
        let width = proposal.width.unwrap_or(f32::INFINITY);
        let (mut x, mut y, mut row) = (0.0f32, 0.0f32, 0.0f32);
        for child in children {
            let s = child.size_that_fits(ProposedSize::unspecified());
            if x > 0.0 && x + s.width > width {
                x = 0.0;
                y += row + self.spacing;
                row = 0.0;
            }
            x += s.width + self.spacing;
            row = row.max(s.height);
        }
        elegance::Size::new(width.min(x), y + row)
    }
    fn place_children(&self, bounds: Rect, _: ProposedSize, children: &[LayoutChild]) {
        let mut x = bounds.x;
        for child in children {
            let s = child.size_that_fits(ProposedSize::unspecified());
            child.place(Point::new(x, bounds.y), ProposedSize::from(s));
            x += s.width + self.spacing + child.value::<Rank>() as f32;
        }
    }
}

struct Rank;
impl LayoutValueKey for Rank {
    type Value = i32;
    fn default_value() -> i32 {
        0
    }
}

struct Compact;
impl EnvironmentKey for Compact {
    type Value = bool;
    fn default_value(_: &EnvironmentValues) -> bool {
        false
    }
}

struct Framed;
impl ViewModifier for Framed {
    fn body(&self, content: View) -> View {
        content.padding(8).border(Color::hex(0x888888), 1)
    }
}

struct Dim;
impl EffectModifier for Dim {
    fn apply(&self, effects: &mut Effects) {
        effects.set_opacity(0.5);
    }
}

struct Clicks;
impl EventHandler for Clicks {
    fn pointer(&mut self, event: &PointerEvent, _: Rect) -> EventResult {
        if event.kind == PointerEventKind::Down {
            EventResult::CONSUMED.cancellable()
        } else {
            EventResult::IGNORED
        }
    }
    fn pointer_style(&self, location: Point) -> PointerStyle {
        if location.x < 4.0 { PointerStyle::ResizeHorizontal } else { PointerStyle::Automatic }
    }
}
struct ClickModifier;
impl EventModifier for ClickModifier {
    fn make_handler(&self) -> Box<dyn EventHandler> {
        Box::new(Clicks)
    }
}

fn gallery() -> impl IntoView {
    let mut volume = State::new(0.5_f32);
    let guests = State::new(2);
    let selected = State::new(None::<i32>);
    let picked = State::new(std::collections::BTreeSet::<i32>::new());
    let size = State::new(Size3::M);
    let name = State::new(String::new());
    let on = State::new(false);
    let showing = State::new(false);
    let path: State<Vec<Route>> = State::new(Vec::new());
    let position = ScrollPosition::new();
    let compact = Environment::<Compact>::new();
    let header = elg! { Text("Header").font(.headline) };

    elg! {
        NavigationStack(path,destination: |route: &Route| match route {
            Route::Detail(id) => Text(format!("Detail {id}")),
        }, root_title: "Home") {
            ScrollView(position: position.clone(), sticks_to_end: false) {
                header
                Badge(title: "New".to_string())
                Badge(title: "Mail".to_string(), count: 3, note: "unread".to_string())
                { Card { title: "Card".into() } }
                Text(move || if compact.get() { "S" } else { "Small" })
                Slider(volume, min: 0, max: move || volume.get() + 1.0, step: 0.1)
                NumberInput(guests, min: 1, max: 10)
                Picker(size, [(Size3::S, "S"), (Size3::M, "M"), (Size3::L, "L")]).picker_style(MenuPickerStyle)
                TextField("Password", name, secure: move || !on.get(), line_limit: 1)
                TextArea("Notes", name, on_submit: || {})
                Toggle(Label("Wi-Fi", "wifi-high"), on).toggle_style(SwitchToggleStyle::new().fills_width(true))
                Toggle(on).accessibility_label("Wi-Fi")
                Checkbox("All", on, mixed: move || volume.get() > 0.5)
                    .checkbox_style(BoxCheckboxStyle::new().size(16).fill(.success))
                Progress(move || Some(volume.get())).progress_style(CircularProgressStyle::new().size(14))
                Progress()
                Separator(.vertical, inset: 4)
                if on.get() {
                    Text("on")
                } else if volume.get() > 0.5 {
                    Text("loud")
                } else {
                    EmptyView()
                }
                match size.get() {
                    Size3::S => Text("small"),
                    Size3::M if on.get() => { Text("medium, on").bold() }
                    other => Text(format!("other: {}", other == Size3::L)),
                }
                match path.get().last().cloned() {
                    Some(Route::Detail(id)) => Text(format!("detail {id}")),
                    None => EmptyView(),
                }
                VGrid([GridItem::adaptive(120).spacing(8), GridItem::fixed(40).alignment(.trailing)], alignment: .top, spacing: 12) {
                    ForEach(0..6) { i in Text(format!("{i}")) }
                }
                HGrid([GridItem::flexible().minimum(20).maximum(80)], spacing: 4) { Text("a") Text("b") }
                Grid(alignment: .leading, horizontal_spacing: 16, vertical_spacing: 4) {
                    GridRow(alignment: .top) {
                        Text("Name").grid_column_alignment(.leading)
                        Text("Size").grid_cell_alignment(.bottom_trailing)
                    }
                    Separator()
                    ForEach(0..3) { i in GridRow { Text(format!("{i}")) Text("x") } }
                    GridRow { Text("note").grid_cell_columns(2) }
                }
                (Flow { spacing: 4.0 }) {
                    Text("a").layout_value::<Rank>(2)
                    Text("b")
                }
                (Cached(CachedGrid)) { Text("c") }
                ZStack(alignment: .top_trailing) {
                    Circle().fill(LinearGradient::colors([Color::hex(0xFF0000), Color::hex(0x0000FF)]))
                    Capsule().stroke(.accent, 2)
                    Canvas(|gc| gc.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::WHITE, 2))
                }
                .frame(40, 40)
                LazyVStack(spacing: 2, separators: StackSeparators::default().inset(16)) {
                    ForEach(0..100) { i in Text(format!("Row {i}")) }
                }
                .selection(selected)
                .row_style(HighlightRowStyle::new().corner_radius(0))
                VStack { ForEach(0..3) { i in Text(format!("{i}")) } }
                    .selection_with(picked, |key: &i32| { let _ = key; })
                AsyncImage("https://example.com/a.png", scale: 2.0, disk_cache: false) { phase in
                    { match phase { AsyncImagePhase::Success(image) => image.clone().resizable(), _ => Progress::new().into_view() } }
                }
                MenuButton("Actions", || elg! {
                    Menu {
                        MenuItem("Rename", || {}, shortcut: "F2")
                        Separator()
                        Submenu("More", || elg! { Menu { MenuItem("Delete", || {}, destructive: true) } })
                    }
                }, edge: .top)
                .menu_style(PlainMenuStyle)
                Button(action: move || volume += 0.1) { Label("Louder", "speaker-high") }
                    .button_style(Pill)
                    .popover(showing,|| elg! { Text("Popover") }, edge: .top, padded: false)
                    .tooltip("Turns it up")
                    .on_key_press(.space, move || volume -= 0.1)
                    .pointer_style(.pointer)
                Text("Zoom")
                    .on_magnify(|e| { let _: f32 = e.scale; })
                    .on_rotate(|e| { let _: GesturePhase = e.phase; })
                    .on_two_finger_double_tap(|at| { let _: Point = at; })
                    .recognize(DragGesture::with(|o| o.minimum_distance(8)).on_changed(|e| { let _: Point = e.translation; }))
                    .recognize_with(LongPressGesture::new(|| {}), |o| o.priority(GesturePriority::High))
                    .on_click(|e| { let _: Point = e.location; }, count: 2)
                    .on_activate(|| {})
                    .on_hover_change(|_| {})
                    .on_pointer_enter(|| {})
                    .on_pointer(|e| e.window_location.x > 0.0)
                    .on_key(|e| e.kind == KeyEventKind::Up)
                    .floating(showing, || Text("Card"), |o| o.boundary(FloatingBoundary::Window).outside_click(false))
                    .backdrop_blur(30, saturation: 1.8)
                    .border_in(.outline, 1, Capsule())
                    .font_weight(move || if on.get() { Weight::Bold } else { Weight::Regular })
                    .pointer_style(move || if on.get() { PointerStyle::Wait } else { PointerStyle::Automatic })
            }
        }
        .environment::<Compact>(true)
        .modifier(Framed)
        .modifier(AsEffect(Dim))
        .modifier(AsEvent(ClickModifier))
        .shadow(radius: 4, y: 2)
        .on_double_click(|_| {})
        .navigation_stack_style(BarNavigationStackStyle::new().height(38))
        .overlay(Text("!"), alignment: .top_leading)
        .animation(.smooth, move || on.get())
        .transition(Transition::slide().combined(Transition::opacity()))
        .padding(top: 8, horizontal: 12)
        .frame(min_width: 200, max_width: .infinity)
    }
}

struct CachedGrid;
impl CachedLayout for CachedGrid {
    type Cache = Vec<elegance::Size>;
    fn make_cache(&self, children: &[LayoutChild]) -> Self::Cache {
        children
            .iter()
            .map(|c| c.size_that_fits(ProposedSize::unspecified()))
            .collect()
    }
    fn size_that_fits(
        &self,
        _: ProposedSize,
        _: &[LayoutChild],
        cache: &mut Self::Cache,
    ) -> elegance::Size {
        cache.iter().fold(elegance::Size::default(), |a, s| {
            elegance::Size::new(a.width + s.width, a.height.max(s.height))
        })
    }
    fn place_children(
        &self,
        bounds: Rect,
        _: ProposedSize,
        children: &[LayoutChild],
        cache: &mut Self::Cache,
    ) {
        let mut x = bounds.x;
        for (child, size) in children.iter().zip(cache.iter()) {
            child.place(Point::new(x, bounds.y), ProposedSize::from(*size));
            x += size.width;
        }
    }
}

#[test]
fn grid_tracks() {
    let tracks = resolve_grid_tracks(
        &[GridItem::fixed(100), GridItem::adaptive(50)],
        Some(400.0),
        10,
    );
    // 100 fixed + 10 gap leaves 290: five 50s with gaps fit exactly (5 * 50 + 4 * 10), six don't.
    assert_eq!(tracks.len(), 6);
    assert_eq!(tracks[0].size, 100.0);
    assert!((grid_tracks_length(&tracks) - 400.0).abs() < 0.01);
}

#[test]
fn dsl_compiles() {
    let _ = gallery as fn() -> _;
}

struct Track;
impl SliderStyle for Track {
    fn make_track(&self, slider: SliderConfiguration) -> View {
        let state = slider.clone();
        Capsule().fill(move || if state.is_hovered() { ThemeColor::Accent } else { ThemeColor::Outline }).into_view()
    }
    fn make_thumb(&self, _: SliderConfiguration) -> View {
        Circle().frame(12, 12)
    }
}

struct Rows;
impl MenuStyle for Rows {
    fn make_body(&self, menu: MenuConfiguration) -> View {
        menu.items().padding(4)
    }
}

/// The new controls, styles and modifiers, built for real (nothing reads the environment).
fn controls() -> View {
    let value = State::new(0.5_f32);
    let count = State::new(3);
    let on = State::new(true);
    let text = State::new(String::new());
    let one = State::new(Some("b".to_string()));
    let many = State::new(std::collections::BTreeSet::from([1_i64]));
    elg! {
        VStack {
            NumberInput(count, min: 0, max: 10, format: |v| format!("{v} x")).number_input_style(BoxNumberInputStyle::new().corner_radius(2))
            NumberInput(value, step: 0.05, decimals: 2).number_input_style(PlainNumberInputStyle)
            Slider(value).slider_style(Track)
            Slider(value).slider_style(KnobSliderStyle::new().fill(.success).knob_size(14))
            TextField("Password", text, secure: true).text_field_style(PlainTextFieldStyle)
            TextArea("Notes", text)
            Toggle(on).toggle_style(SwitchToggleStyle::new().on(Color::hex(0x00AA00)).animation(None))
            Checkbox("Mixed", on, mixed: true)
            Checkbox(on)
            Progress(0.4).progress_style(LinearProgressStyle::new().thickness(3))
            Picker(count, [(1, "One"), (2, "Two")]).picker_style(SegmentedPickerStyle)
            Separator()
            LazyVStack(separators: StackSeparators::default()) { ForEach(vec!["a".to_string(), "b".to_string()]) { s in Text(s.clone()) } }
                .selection(one)
                .row_style(PlainRowStyle)
            VStack { ForEach(vec![1_i64, 2]) { i in Text(format!("{i}")) } }
                .selection_with(many, |_: &i64| {})
                .row_style(HighlightRowStyle::new())
            MenuButton("Menu", || elg! { Menu { MenuItem("A", || {}) } }).menu_style(Rows)
            Text("Popups")
                .popover(on, || Text("Popover"))
                .tooltip_view(|| Text("Tip"), |o| o.delay(0.1).motion(PopupMotion::new(Transition::opacity(), Animation::ease_out(0.15))))
                .floating(on, || Text("Float"), |o| o.boundary(FloatingBoundary::Container))
                .floating_container()
                .popover_style(PlainPopoverStyle)
                .tooltip_style(DefaultTooltipStyle)
            Text("Effects")
                .backdrop_blur_in(20, RoundedRectangle(8), |o| o.noise(0.02))
                .border(.outline, 1)
                .padding(move || if on.get() { 4.0 } else { 8.0 })
                .recognize(DragGesture::new().on_began(|_| {}).on_ended(|_| {}).on_cancelled(|| {}))
                .recognize(MagnifyGesture::new().on_changed(|_| {}))
                .on_scroll(|_| false)
                .on_text_input(|_| false)
                .on_pointer_leave(|| {})
        }
        .navigation_stack_style(PlainNavigationStackStyle)
        .transition(Transition::scale_anchored(0.9, Alignment::TOP))
    }
    .into_view()
}

#[test]
fn controls_build() {
    // Each crosses into Elegance; a C++ exception would panic here.
    assert!(!controls().is_empty());
    assert!(EmptyView().is_empty());
}

#[test]
fn integrated_title_bar() {
    let desc = WindowDesc::new("App").title_bar(TitleBar::Integrated).system_controls_position(16.0, 18.0);
    assert_eq!(desc.title_bar, TitleBar::Integrated);
    assert_eq!(desc.system_controls_position, Some(Point::new(16.0, 18.0)));
}

fn nested_shorthand_keeps_its_own_context() -> impl IntoView {
    elg! {
        VStack().background(LinearGradient::new(vec![], .top.into(), .bottom.into()))
            .on_activate(|| { let _: Alignment = .center.into(); })
    }
}

struct CompletionCall {
    calls: usize,
}

#[elg_builder]
impl CompletionCall {
    pub fn borrowed(&self, _: impl Into<Alignment>) -> usize {
        self.calls
    }
    pub fn mutable(&mut self, _: impl Into<Alignment>) {
        self.calls += 1;
    }
    pub fn text(&self, text: &str, _: impl Into<Alignment>) -> usize {
        text.len()
    }
    pub fn callback(&self, f: impl FnOnce(&Self) -> usize, _: impl Into<Alignment>) -> usize {
        f(self)
    }
    pub fn guard(&self, _: (), _: impl Into<Alignment>, dropped: &std::cell::Cell<bool>) {
        assert!(
            !dropped.get(),
            "argument temporary dropped before method call"
        );
    }
}

#[test]
fn shorthand_preserves_method_call_semantics() {
    let mut builder = CompletionCall { calls: 1 };
    assert_eq!(elg! { builder.borrowed(.center) }, 1);
    elg! { builder.mutable(.center) };
    assert_eq!(builder.calls, 2);
    assert_eq!(
        elg! { builder.text(String::from("abc").as_str(), .center) },
        3
    );
    assert_eq!(elg! { builder.callback(|b| b.calls, .center) }, 2);
    assert_eq!(elg! { {CompletionCall { calls: 4 }}.borrowed(.center) }, 4);

    struct Guard<'a>(&'a std::cell::Cell<bool>);
    impl Guard<'_> {
        fn value(&self) {}
    }
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
    let dropped = std::cell::Cell::new(false);
    elg! { builder.guard(Guard(&dropped).value(), .center, &dropped) };
    assert!(dropped.get());
}

#[test]
fn completion_carriers_preserve_positional_evaluation_and_moves() {
    struct Identity;
    impl<T> elegance::dsl::ElgNew<(T,)> for Identity {
        type Out = T;
        fn elg_new((value,): (T,)) -> T {
            value
        }
    }
    let value = String::from("owned");
    let value = elg! { Identity(value) };
    assert_eq!(value, "owned");
    let borrowed = &value;
    assert_eq!(elg! { Identity(borrowed) }, "owned");
    let value = elg! { Identity(value) };
    assert_eq!(value, "owned");
}

#[derive(PartialEq)]
struct Viewport {
    size: (u32, u32),
    name: String,
}

#[test]
fn text_placeholders_take_fields_and_expressions() {
    struct Identity;
    impl<T> elegance::dsl::ElgNew<(T,)> for Identity {
        type Out = T;
        fn elg_new((value,): (T,)) -> T {
            value
        }
    }
    let size = (640, 480);
    let viewport = Viewport { size: (3, 4), name: "main".into() };
    let width = 6;
    let text = elg! { Identity("{size.0}x{size.1} {viewport.name:>width$} {viewport.size.1:03} {size.0 / 2} {{x}} {width}") };
    assert_eq!((text.0)(), "640x480   main 004 320 {x} 6");
}

fn reactive_text_placeholders() -> impl IntoView {
    let render_size = State::new((0, 0));
    let viewport = computed(|| Viewport { size: (1, 2), name: String::new() });
    let binding = render_size.binding();
    elg! {
        VStack {
            Text("Rendering at {render_size.0}x{render_size.1}")
            Text("{viewport.size.0:?} {viewport.name}")
            Text("{binding.1} {render_size.get().0 + 1}")
        }
    }
}
