//! Resource Collection Simulation — point d'entrée.

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
    let mut map = Map::generate(80, 40, 42);
    map.populate_resources(7);

    let application = app::App::new(map);
    app::run(application)
}
