// NavigationSplitView: a sidebar list that selects what the detail column shows.

use std::process::ExitCode;

use elegance::prelude::*;

#[derive(Clone, PartialEq)]
struct Game {
    id: i64,
    name: &'static str,
    mods: i32,
}

fn games() -> Vec<Game> {
    vec![
        Game { id: 1, name: "Minecraft", mods: 42 },
        Game { id: 2, name: "Skyrim", mods: 118 },
        Game { id: 3, name: "Stardew Valley", mods: 9 },
    ]
}

#[component]
fn Sidebar(selected: State<i64>) -> impl IntoView {
    elg! {
        List {
            ForEach(games(), id: |g| g.id) { game in
                SidebarRow(game: game.clone(), selected: selected)
            }
        }
    }
}

#[component]
fn SidebarRow(game: Game, selected: State<i64>) -> impl IntoView {
    let id = game.id;
    elg! {
        Text(game.name)
            .frame(max_width: .infinity, alignment: .leading)
            .content_shape(Rectangle())
            .on_tap_gesture(move || selected.set(id))
            .foreground_style(move || if selected.get() == id { ThemeColor::Accent } else { ThemeColor::Primary })
    }
}

#[component]
fn Detail(selected: State<i64>, sidebar_visible: State<bool>) -> impl IntoView {
    let game = move || games().into_iter().find(|g| g.id == selected.get());
    elg! {
        VStack(alignment: .leading, spacing: 8) {
            Text(move || game().map(|g| g.name).unwrap_or("Pick a game")).font(.title)
            Text(move || game().map(|g| format!("{} mods installed", g.mods)).unwrap_or_default())
                .foreground_style(.secondary)
            Button("Toggle sidebar", move || sidebar_visible.update(|v| *v = !*v))
        }
        .padding(24)
        .frame(max_width: .infinity, max_height: .infinity, alignment: .top_leading)
    }
}

#[component]
fn Library() -> impl IntoView {
    let selected = State::new(1_i64);
    let sidebar_visible = State::new(true);

    elg! {
        // Two views in the block: sidebar, detail. (Three: sidebar, content, detail.)
        NavigationSplitView(sidebar_visible: $sidebar_visible, sidebar_width: ColumnWidth::new(160.0, 200.0, 300.0)) {
            Sidebar(selected: selected)
            Detail(selected: selected, sidebar_visible: sidebar_visible)
        }
    }
}

fn main() -> Result<ExitCode, Error> {
    App::new().window(WindowDesc::new("Library").size(720, 420), Library).run()
}
