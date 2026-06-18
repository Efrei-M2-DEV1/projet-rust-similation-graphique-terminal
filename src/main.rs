//! Resource Collection Simulation — point d'entrée.
//!
//! Squelette initial : les modules (`world`, `robots`, `communication`,
//! `ui`, `pathfinding`) seront ajoutés progressivement.

mod communication;
mod utils;
mod world;

use anyhow::Result;
use communication::{register_robot, setup, Message};
use utils::Position;
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

    // petit smoke test communication avant l'intégration complète
    let (mut hub, mut clock) = setup(map.base());
    let scout = register_robot(&mut hub, &mut clock);
    let collector = register_robot(&mut hub, &mut clock);

    scout.comm.send(Message::resource_found(
        Position::new(15, 8),
        ResourceKind::Energy,
        120,
    ));
    scout.comm.send(Message::obstacle_found(Position::new(14, 8)));
    hub.poll();

    let n = collector.comm.drain_inbox().len();
    println!("Communication : {n} message(s) reçu(s) par le collecteur");

    collector.comm.send(Message::Deposit {
        kind: ResourceKind::Energy,
        amount: 25,
    });
    hub.poll();

    let base_ref = hub.base();
    let base = base_ref.lock().unwrap();
    println!(
        "Base : énergie={} cristaux={} | ressources connues={} | obstacles connus={}",
        base.stored_energy(),
        base.stored_crystals(),
        base.known_resource_count(),
        base.known_obstacles().len(),
    );
    drop(base);

    // Aperçu ASCII (pas encore Ratatui — c'est le job de Dev 4).
    for y in 0..map.height() as i32 {
        let mut line = String::with_capacity(map.width());
        for x in 0..map.width() as i32 {
            let p = Position::new(x, y);
            let glyph = map.get(p).map(|t| t.glyph()).unwrap_or(' ');
            line.push(glyph);
        }
        println!("{line}");
    }

    Ok(())
}
