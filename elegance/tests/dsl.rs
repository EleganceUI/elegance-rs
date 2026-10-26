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
            EventResult::CONSUMED
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
    let size = State::new(Size3::M);
    let name = State::new(String::new());
    let on = State::new(false);
    let showing = State::new(false);
    let path: State<Vec<Route>> = State::new(Vec::new());
    let position = ScrollPosition::new();
    let compact = Environment::<Compact>::new();
    let header = elg! { Text("Header").font(.headline) };

    elg! {
        NavigationStack($path, destination: |route: &Route| match route {
            Route::Detail(id) => Text(format!("Detail {id}")),
        }, root_title: "Home") {
            ScrollView(position: position.clone(), sticks_to_end: false) {
                header
                Badge(title: "New".to_string())
                Badge(title: "Mail".to_string(), count: 3, note: "unread".to_string())
                { Card { title: "Card".into() } }
                Text(move || if compact.get() { "S" } else { "Small" })
                Slider($volume, min: 0, max: 1, step: 0.1)
                Stepper("Guests: {guests}", $guests, min: 1, max: 10)
                Picker($size, [(Size3::S, "S"), (Size3::M, "M"), (Size3::L, "L")], style: .menu)
                SecureField("Password", $name, max_lines: 1)
                Toggle(Label("Wi-Fi", "wifi-high"), $on).toggle_style(SwitchToggleStyle)
                ProgressView(volume)
                ProgressView()
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
                    Divider()
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
                LazyVStack(spacing: 2) {
                    ForEach(0..100) { i in Text(format!("Row {i}")) }
                }
                AsyncImage("https://example.com/a.png", scale: 2.0) { phase in
                    { match phase { AsyncImagePhase::Success(image) => image.clone().resizable(), _ => ProgressView::new().into_view() } }
                }
                MenuButton("Actions", || elg! {
                    Menu {
                        MenuItem("Rename", || {}, shortcut: "F2")
                        Divider()
                        Submenu("More", || elg! { Menu { MenuItem("Delete", || {}, destructive: true) } })
                    }
                })
                Button(action: move || volume += 0.1) { Label("Louder", "speaker-high") }
                    .button_style(Pill)
                    .popover($showing, || elg! { Text("Popover") }, opens: .above)
                    .help("Turns it up")
                    .on_key_press(.space, move || volume -= 0.1)
                    .pointer_style(.pointer)
                Text("Zoom")
                    .on_magnify(|e| { let _: f32 = e.scale; })
                    .on_rotate(|e| { let _: GesturePhase = e.phase; })
                    .on_smart_magnify(|at| { let _: Point = at; })
                    .pointer_style(move || if on.get() { PointerStyle::Wait } else { PointerStyle::Automatic })
            }
        }
        .environment::<Compact>(true)
        .modifier(Framed)
        .modifier(AsEffect(Dim))
        .modifier(AsEvent(ClickModifier))
        .shadow(radius: 4, y: 2)
        .on_tap_gesture(|| {}, count: 2)
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

#[test]
fn integrated_title_bar() {
    let desc = WindowDesc::new("App").title_bar(TitleBar::Integrated).traffic_lights_position(16.0, 18.0);
    assert_eq!(desc.title_bar, TitleBar::Integrated);
    assert_eq!(desc.traffic_lights_position, Some(Point::new(16.0, 18.0)));
}

fn nested_shorthand_keeps_its_own_context() -> impl IntoView {
    elg! {
        VStack().background(LinearGradient::new(vec![], .top.into(), .bottom.into()))
            .on_tap_gesture(|| { let _: Alignment = .center.into(); })
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
