//! Resource Collection Simulation — point d'entrée.
//!
//! Squelette initial : les modules (`world`, `robots`, `communication`,
//! `ui`, `pathfinding`) seront ajoutés progressivement.


mod app;
mod ui;
mod utils;
mod world;

use anyhow::Result;
use world::Map;

fn main() -> Result<()> {
    println!("Resource Collection Simulation — (génération de carte)");

    let mut map = Map::generate(60, 20, 42);
    map.populate_resources(7);


let app= app::App::new(map);
    app::run(app)
}
