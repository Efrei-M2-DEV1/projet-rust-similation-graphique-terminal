//! Tests du moteur de simulation concurrent.
//!
//! Ces tests ne vérifient pas seulement des fonctions isolées.
//! Ils vérifient que la simulation démarre réellement, produit des snapshots,
//! fait progresser les ticks et permet aux collectors de collecter/déposer.
//!
//! C'est important pour le projet, car le sujet attend une architecture
//! concurrente avec robots indépendants, communication asynchrone et interface
//! temps réel.

use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;

use super::{SimulationEngine, SimulationHandle, SimulationSnapshot};
use crate::robots::RobotKind;
use crate::utils::Position;
use crate::world::resource::Resource;
use crate::world::{Map, ResourceKind, Tile};

/// Attend le premier snapshot disponible.
///
/// Si aucun snapshot n'arrive dans le temps imparti, le test échoue.
/// Cela évite qu'un test reste bloqué indéfiniment.
fn wait_for_snapshot(handle: &SimulationHandle, timeout: Duration) -> SimulationSnapshot {
    let deadline = Instant::now() + timeout;

    loop {
        match handle.snapshot_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(snapshot) => return snapshot,

            Err(RecvTimeoutError::Timeout) if Instant::now() < deadline => {
                continue;
            }

            Err(error) => {
                panic!("aucun snapshot recu avant le timeout: {error:?}");
            }
        }
    }
}

/// Attend jusqu'à ce qu'un snapshot respecte une condition donnée.
///
/// Exemple :
/// - attendre que le tick augmente ;
/// - attendre qu'au moins une énergie soit collectée ;
/// - attendre que les deux types de ressources soient collectés.
fn wait_until<F>(
    handle: &SimulationHandle,
    timeout: Duration,
    mut predicate: F,
) -> SimulationSnapshot
where
    F: FnMut(&SimulationSnapshot) -> bool,
{
    let deadline = Instant::now() + timeout;
    let mut last_snapshot = None;

    while Instant::now() < deadline {
        match handle.snapshot_rx.recv_timeout(Duration::from_millis(80)) {
            Ok(snapshot) => {
                if predicate(&snapshot) {
                    return snapshot;
                }

                last_snapshot = Some(snapshot);
            }

            Err(RecvTimeoutError::Timeout) => {
                continue;
            }

            Err(RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    panic!(
        "condition non atteinte avant timeout. Dernier snapshot: {:?}",
        last_snapshot
    );
}

#[test]
fn simulation_emits_snapshot_with_configured_robots() {
    let mut map = Map::generate(40, 18, 42);
    map.populate_resources(7);

    let simulation = SimulationEngine::new(map, 2, 2).start();

    let snapshot = wait_for_snapshot(&simulation, Duration::from_secs(1));

    simulation.shutdown();

    assert_eq!(snapshot.width, 40);
    assert_eq!(snapshot.height, 18);

    assert_eq!(
        snapshot.robots.len(),
        4,
        "la simulation doit contenir 2 scouts + 2 collectors"
    );

    let scout_count = snapshot
        .robots
        .iter()
        .filter(|robot| robot.kind == RobotKind::Scout)
        .count();

    let collector_count = snapshot
        .robots
        .iter()
        .filter(|robot| robot.kind == RobotKind::Collector)
        .count();

    assert_eq!(scout_count, 2);
    assert_eq!(collector_count, 2);

    assert!(
        snapshot
            .resources
            .iter()
            .any(|resource| resource.kind == ResourceKind::Energy),
        "le snapshot doit contenir au moins une ressource d'energie"
    );

    assert!(
        snapshot
            .resources
            .iter()
            .any(|resource| resource.kind == ResourceKind::Crystal),
        "le snapshot doit contenir au moins un cristal"
    );
}

#[test]
fn simulation_tick_progresses_over_time() {
    let mut map = Map::generate(40, 18, 42);
    map.populate_resources(7);

    let simulation = SimulationEngine::new(map, 2, 2).start();

    let first = wait_for_snapshot(&simulation, Duration::from_secs(1));

    let later = wait_until(&simulation, Duration::from_secs(2), |snapshot| {
        snapshot.tick > first.tick + 2
    });

    simulation.shutdown();

    assert!(
        later.tick > first.tick,
        "le tick doit progresser pendant que la simulation tourne"
    );
}

#[test]
fn collectors_can_collect_and_deposit_energy_and_crystal() {
    // Carte contrôlée pour éviter un test fragile.
    // On place volontairement une énergie et un cristal proches de la base.
    //
    // Comme les ressources sont proches :
    // - le scout les découvre rapidement ;
    // - les collectors les reçoivent via le hub ;
    // - ils peuvent collecter puis déposer à la base.
    let mut map = Map::empty(15, 15);
    let base = map.base();

    let energy_position = Position::new(base.x + 1, base.y);
    let crystal_position = Position::new(base.x - 1, base.y);

    map.set(
        energy_position,
        Tile::Resource(Resource::new(ResourceKind::Energy, 5)),
    );

    map.set(
        crystal_position,
        Tile::Resource(Resource::new(ResourceKind::Crystal, 5)),
    );

    let simulation = SimulationEngine::new(map, 1, 2).start();

    let snapshot = wait_until(&simulation, Duration::from_secs(4), |snapshot| {
        snapshot.collected_energy > 0 && snapshot.collected_crystals > 0
    });

    simulation.shutdown();

    assert!(
        snapshot.collected_energy > 0,
        "au moins une unite d'energie doit etre collectee et deposee"
    );

    assert!(
        snapshot.collected_crystals > 0,
        "au moins un cristal doit etre collecte et depose"
    );
}
