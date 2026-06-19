//! Concurrent simulation engine tests.
//!
//! These tests do more than check isolated functions: they verify that the
//! simulation actually starts, produces snapshots, advances ticks and lets
//! collectors collect and deposit.

use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;

use super::{SimulationEngine, SimulationHandle, SimulationSnapshot};
use crate::robots::RobotKind;
use crate::utils::Position;
use crate::world::resource::Resource;
use crate::world::{Map, ResourceKind, Tile};

/// Waits for the first available snapshot.
///
/// Fails the test if no snapshot arrives within the allotted time, so a test
/// never blocks forever.
fn wait_for_snapshot(handle: &SimulationHandle, timeout: Duration) -> SimulationSnapshot {
    let deadline = Instant::now() + timeout;

    loop {
        match handle.snapshot_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(snapshot) => return snapshot,

            Err(RecvTimeoutError::Timeout) if Instant::now() < deadline => {
                continue;
            }

            Err(error) => {
                panic!("no snapshot received before timeout: {error:?}");
            }
        }
    }
}

/// Waits until a snapshot satisfies a given condition, e.g.:
/// - wait for the tick to increase;
/// - wait for at least one energy to be collected;
/// - wait for both resource kinds to be collected.
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
        "condition not met before timeout. Last snapshot: {:?}",
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
        "the simulation must contain 2 scouts + 2 collectors"
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
        "the snapshot must contain at least one energy resource"
    );

    assert!(
        snapshot
            .resources
            .iter()
            .any(|resource| resource.kind == ResourceKind::Crystal),
        "the snapshot must contain at least one crystal"
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
        "the tick must progress while the simulation runs"
    );
}

#[test]
fn collectors_can_collect_and_deposit_energy_and_crystal() {
    // Controlled map to avoid a flaky test. An energy and a crystal are placed
    // close to the base so the scout discovers them quickly, the collectors
    // receive them via the hub, then collect and deposit at the base.
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
        "at least one energy unit must be collected and deposited"
    );

    assert!(
        snapshot.collected_crystals > 0,
        "at least one crystal must be collected and deposited"
    );
}
