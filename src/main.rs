//! Resource Collection Simulation — point d'entrée.
//!
//! Squelette initial : les modules (`world`, `robots`, `communication`,
//! `ui`, `pathfinding`) seront ajoutés progressivement.

mod communication;
mod utils;
mod world;

use anyhow::Result;
use world::{Map, ResourceKind};

fn main() -> Result<()> {
    println!("Resource Collection Simulation — (génération de carte)");

    let mut map = Map::generate(60, 20, 42);
    let placed = map.populate_resources(7);

    println!(
        "Carte {}x{} | obstacles: {} | énergie: {} | cristaux: {} | total ressources placées: {}",
        map.width(),
        map.height(),
        map.count_obstacles(),
        map.count_resources(ResourceKind::Energy),
        map.count_resources(ResourceKind::Crystal),
        placed,
    );

    // Aperçu ASCII (pas encore Ratatui — c'est le job de Dev 4).
    // Double boucle : la boucle externe parcourt les lignes (y), la
    // boucle interne construit chaque ligne caractère par caractère
    // en demandant à la map le glyphe de chaque case (x, y).
    for y in 0..map.height() as i32 {
        let mut line = String::with_capacity(map.width());
        for x in 0..map.width() as i32 {
            let p = utils::Position::new(x, y);
            let glyph = map.get(p).map(|t| t.glyph()).unwrap_or(' ');
            line.push(glyph);
        }
        println!("{line}");
    }

    Ok(())
}
