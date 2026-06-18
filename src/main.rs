//! Resource Collection Simulation — point d'entrée.
//!
//! Assemble la carte procedurale, les robots, la communication et l'UI
//! Ratatui.

mod app;
mod communication;
mod pathfinding;
mod robots;
mod ui;
mod utils;
mod world;

use anyhow::Result;
use world::Map;

fn main() -> Result<()> {
    let mut map = Map::generate(60, 20, 42);
    map.populate_resources(7);

    let app = app::App::new(map);
    app::run(app)
}
