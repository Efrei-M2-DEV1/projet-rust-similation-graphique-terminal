//! Point d'entrée de l'application.
//!
//! Ce fichier reste volontairement petit.
//! Son rôle :
//! 1. Générer la carte.
//! 2. Placer les ressources.
//! 3. Démarrer la simulation concurrente.
//! 4. Lancer l'interface Ratatui.

mod app;
mod communication;
mod pathfinding;
mod robots;
mod simulation;
mod ui;
mod utils;
mod world;

use anyhow::Result;
use simulation::SimulationEngine;
use world::Map;

fn main() -> Result<()> {
    // 1. On génère une carte procédurale.
    // Les obstacles sont produits par le bruit de Perlin dans world/generator.rs.
    let mut map = Map::generate(80, 28, 42);

    // 2. On place les ressources initiales.
    // Chaque ressource possède une quantité aléatoire entre 50 et 200.
    map.populate_resources(7);

    // 3. On crée le moteur de simulation.
    // Ici, on démarre 3 scouts et 3 collectors.
    let simulation = SimulationEngine::new(map, 4, 4).start();

    // 4. Ratatui reste dans le thread principal.
    // La simulation tourne dans ses propres threads et envoie des snapshots à l'UI.
    app::run(simulation)
}
