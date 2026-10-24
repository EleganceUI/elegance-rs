// Grids: a VGrid of cards that fits as many columns as the window allows, and a Grid table with
// columns sized to their content, a spanning cell, and full-width dividers.
// (A port of EleganceMain/examples/grids.)

use std::process::ExitCode;

use elegance::prelude::*;

#[derive(Clone, PartialEq)]
struct Game {
    id: i64,
    name: &'static str,
    mods: i32,
    size_gb: f32,
}

fn games() -> Vec<Game> {
    vec![
        Game { id: 1, name: "Skyrim", mods: 118, size_gb: 12.4 },
        Game { id: 2, name: "Minecraft", mods: 42, size_gb: 1.1 },
        Game { id: 3, name: "Stardew Valley", mods: 9, size_gb: 0.6 },
        Game { id: 4, name: "Fallout 4", mods: 77, size_gb: 31.0 },
        Game { id: 5, name: "Terraria", mods: 15, size_gb: 0.4 },
        Game { id: 6, name: "Cyberpunk 2077", mods: 23, size_gb: 70.2 },
    ]
}

#[component]
fn Card(game: Game) -> impl IntoView {
    elg! {
        VStack(alignment: .leading, spacing: 4) {
            Text(game.name).font(.headline)
            Text(format!("{} mods", game.mods)).foreground_style(.secondary)
        }
        .padding(14)
        .frame(max_width: .infinity, alignment: .leading)
        .background_in(.surface, RoundedRectangle(10))
    }
}

#[component]
fn Grids() -> impl IntoView {
    elg! {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                Text("Library").font(.large_title)
                VGrid([GridItem::adaptive(150)], spacing: 12) {
                    ForEach(games(), id: |g| g.id) { game in
                        Card(game: game.clone())
                    }
                }
                Text("Details").font(.title2)
                Grid(alignment: .leading, horizontal_spacing: 24, vertical_spacing: 8) {
                    GridRow {
                        Text("Game").font(.headline)
                        Text("Mods").font(.headline)
                        Text("Size").font(.headline).grid_column_alignment(.trailing)
                    }
                    Divider()
                    ForEach(games(), id: |g| g.id) { game in
                        GridRow {
                            Text(game.name)
                            Text(format!("{}", game.mods))
                            Text(format!("{:.1} GB", game.size_gb))
                        }
                    }
                    Divider()
                    GridRow {
                        Text("Sizes include downloaded mods.").foreground_style(.secondary).grid_cell_columns(3)
                    }
                }
            }
            .padding(24)
        }
    }
}

fn main() -> Result<ExitCode, Error> {
    App::new().window(WindowDesc::new("Grids").size(640, 640), Grids).run()
}
