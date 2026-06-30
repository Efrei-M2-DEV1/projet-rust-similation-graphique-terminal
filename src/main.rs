//! Application entry point.
//!
//! This file is intentionally small. Its role:
//! 1. Generate the map.
//! 2. Place the resources.
//! 3. Start the concurrent simulation.
//! 4. Launch the Ratatui UI.

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
    // 1. Generate a procedural map.
    // Obstacles come from Perlin noise in world/generator.rs.
    let mut map = Map::generate(80, 28, 42);

    // 2. Place the initial resources.
    // Each resource gets a random quantity between 50 and 200.
    map.populate_resources(7);

    // 3. Create the simulation engine with 4 scouts and 4 collectors.
    let simulation = SimulationEngine::new(map, 4, 4).start();

    // 4. Ratatui stays on the main thread; the simulation runs in its own
    // threads and sends snapshots to the UI.
    app::run(simulation)
}
