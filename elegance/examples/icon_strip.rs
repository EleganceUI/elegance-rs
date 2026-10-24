// A fixed strip of icons down the side that selects the page: an HStack, no split view. Each
// icon shows its name to its right while hovered (HoverLabel).

use std::process::ExitCode;

use elegance::prelude::*;

#[derive(Clone, PartialEq)]
struct Page {
    id: i64,
    name: &'static str,
    icon: &'static str,
}

fn pages() -> Vec<Page> {
    vec![
        Page { id: 1, name: "Home", icon: "house" },
        Page { id: 2, name: "Mods", icon: "puzzle-piece" },
        Page { id: 3, name: "Downloads", icon: "download-simple" },
        Page { id: 4, name: "Settings", icon: "gear" },
    ]
}

#[component]
fn IconStrip(selected: State<i64>) -> impl IntoView {
    elg! {
        VStack(spacing: 4) {
            ForEach(pages(), id: |p| p.id) { page in
                StripButton(page: page.clone(), selected: selected)
            }
            Spacer()
        }
        .padding(8)
        .frame(width: 56, max_height: .infinity)
        .background(.surface)
    }
}

#[component]
fn StripButton(page: Page, selected: State<i64>) -> impl IntoView {
    let id = page.id;
    elg! {
        Button(Image(page.icon), move || selected.set(id))
            .button_style(PlainButtonStyle)
            .font(Font::system(20.0, .regular))
            .frame(40, 40)
            .foreground_style(move || if selected.get() == id { ThemeColor::Accent } else { ThemeColor::Secondary })
            .background_in(move || if selected.get() == id { ThemeColor::SurfaceRaised } else { ThemeColor::Surface }, RoundedRectangle(8))
            .modifier(HoverLabel(page.name))
            .accessibility_label(page.name)
    }
}

/// A label to the right of the view while the pointer is over it.
struct HoverLabel(&'static str);

impl ViewModifier for HoverLabel {
    fn body(&self, content: View) -> View {
        let hovered = State::new(false);
        let label = elg! {
            HStack(spacing: 0) {
                // The little triangle pointing back at the view.
                CustomShape(pointer, .non_zero).fill(.surface_raised).frame(5, 10)
                Text(self.0)
                    .font(Font::system(12.0, .regular))
                    .padding(horizontal: 8, vertical: 5)
                    .background_in(.surface_raised, RoundedRectangle(5))
            }
                .fixed_size()
                .opacity(move || if hovered.get() { 1.0 } else { 0.0 })
                // Hidden, it waits 12 points to the left; shown, it slides into place.
                .offset(move || if hovered.get() { 0.0 } else { -12.0 }, 0)
                .animation(Animation::smooth(0.25), hovered)
                .allows_hit_testing(false)
        };
        elg! {
            {content}
                .on_hover(move |inside| hovered.set(inside))
                .overlay(Beside { gap: 3.0 }.view(vec![label.into_view()]), alignment: .trailing)
        }
        .into_view()
    }
}

/// A triangle pointing left, filling `bounds`.
fn pointer(bounds: Rect) -> Path {
    let mut path = Path::new();
    path.move_to(Point::new(bounds.max_x(), bounds.min_y()))
        .line_to(Point::new(bounds.min_x(), bounds.center().y))
        .line_to(Point::new(bounds.max_x(), bounds.max_y()))
        .close();
    path
}

/// Takes no width, and puts its children just past its leading edge, vertically centered: in an
/// overlay aligned to the trailing edge, that's beside the view whatever its size.
struct Beside {
    gap: f32,
}

impl Layout for Beside {
    fn size_that_fits(&self, _: ProposedSize, children: &[LayoutChild]) -> Size {
        let height = children.iter().map(|c| c.size_that_fits(ProposedSize::unspecified()).height).fold(0.0, f32::max);
        Size::new(0.0, height)
    }

    fn place_children(&self, bounds: Rect, _: ProposedSize, children: &[LayoutChild]) {
        for child in children {
            let size = child.size_that_fits(ProposedSize::unspecified());
            child.place(Point::new(bounds.x + self.gap, bounds.center().y - size.height / 2.0), ProposedSize::from(size));
        }
    }
}

#[component]
fn Detail(selected: State<i64>) -> impl IntoView {
    elg! {
        // When `selected` changes, the old page is removed (with its state) and the new one built.
        ZStack(alignment: .top_leading) {
            match selected.get() {
                1 => HomePage(),
                2 => ModsPage(),
                3 => DownloadsPage(),
                4 => SettingsPage(),
                _ => EmptyView(),
            }
        }
        .frame(max_width: .infinity, max_height: .infinity, alignment: .top_leading)
    }
}

#[component]
fn HomePage() -> impl IntoView {
    elg! {
        VStack(alignment: .leading, spacing: 8) {
            Text("Home").font(.title)
            Text("Recently played, news, ...").foreground_style(.secondary)
        }
        .padding(24)
    }
}

#[component]
fn ModsPage() -> impl IntoView {
    let enabled = State::new(true);
    elg! {
        VStack(alignment: .leading, spacing: 8) {
            Text("Mods").font(.title)
            Toggle("Example mod", $enabled)
        }
        .padding(24)
    }
}

#[component]
fn DownloadsPage() -> impl IntoView {
    elg! {
        VStack(alignment: .leading, spacing: 8) {
            Text("Downloads").font(.title)
            ProgressView(0.4).frame(width: 240)
        }
        .padding(24)
    }
}

#[component]
fn SettingsPage() -> impl IntoView {
    elg! { Text("Settings").font(.title).padding(24) }
}

#[component]
fn Shell() -> impl IntoView {
    let selected = State::new(1_i64);

    elg! {
        HStack(spacing: 0) {
            IconStrip(selected: selected).z_index(1) // its hover labels draw over the page
            Rectangle().fill(.outline).frame(width: 1, max_height: .infinity)
            Detail(selected: selected)
        }
    }
}

fn main() -> Result<ExitCode, Error> {
    App::new().window(WindowDesc::new("Icon strip").size(720, 420), Shell).run()
}
