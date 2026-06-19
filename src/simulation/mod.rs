//! Concurrent simulation engine.
//!
//! - the Ratatui UI runs on the main thread;
//! - the hub runs on a dedicated thread and owns the global state;
//! - each robot runs on its own thread;
//! - everyone communicates through channels.
//!
//! The map is shared as `Arc<Mutex<Map>>`: robots lock it briefly to read, and
//! the hub locks it to remove a resource unit on collect.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::communication::{HubToRobot, KnownResource, RobotId, RobotToHub, SimulationCommand};
use crate::robots::{
    CarriedResource, CollectorRobot, RobotKind, RobotSnapshot, RobotState, ScoutRobot,
};
use crate::utils::Position;
use crate::world::{Map, ResourceKind, Tile};

const TICK_RATE: Duration = Duration::from_millis(120);
const EVENT_LOG_LIMIT: usize = 10;

/// Resource displayable in the UI.
#[derive(Debug, Clone)]
pub struct ResourceView {
    pub position: Position,
    pub kind: ResourceKind,
    pub quantity: u32,
}

/// Displayable snapshot sent to Ratatui. The UI never touches the live
/// simulation; it only renders this ready-made structure.
#[derive(Debug, Clone)]
pub struct SimulationSnapshot {
    pub tick: u64,
    pub width: usize,
    pub height: usize,
    pub base: Position,
    pub obstacles: Vec<Position>,
    pub resources: Vec<ResourceView>,
    pub robots: Vec<RobotSnapshot>,
    pub collected_energy: u32,
    pub collected_crystals: u32,
    pub known_resources: usize,
    pub known_obstacles: usize,
    pub events: Vec<String>,
}

/// Simulation engine, configured before launch.
pub struct SimulationEngine {
    map: Map,
    scout_count: usize,
    collector_count: usize,
}

impl SimulationEngine {
    pub fn new(map: Map, scout_count: usize, collector_count: usize) -> Self {
        Self {
            map,
            scout_count,
            collector_count,
        }
    }

    pub fn start(self) -> SimulationHandle {
        let map = Arc::new(Mutex::new(self.map));

        // Channel robots -> hub.
        let (robot_tx, robot_rx) = unbounded::<RobotToHub>();

        // Channel hub -> UI.
        let (snapshot_tx, snapshot_rx) = unbounded::<SimulationSnapshot>();

        // Channel UI -> hub.
        let (command_tx, command_rx) = unbounded::<SimulationCommand>();

        let mut robot_senders = HashMap::<RobotId, Sender<HubToRobot>>::new();
        let mut robot_handles = Vec::<JoinHandle<()>>::new();
        let mut initial_robots = Vec::<RobotSnapshot>::new();

        let base = map.lock().expect("map mutex poisoned").base();
        let mut next_id = 0usize;

        // Scouts.
        for index in 0..self.scout_count {
            let id = RobotId(next_id);
            next_id += 1;

            let (to_robot_tx, to_robot_rx) = unbounded::<HubToRobot>();
            robot_senders.insert(id, to_robot_tx);

            initial_robots.push(RobotSnapshot {
                id,
                kind: RobotKind::Scout,
                position: base,
                cargo: None,
                state: RobotState::Exploring,
            });

            let scout = ScoutRobot::new(
                id,
                base,
                Arc::clone(&map),
                robot_tx.clone(),
                to_robot_rx,
                0x5C0A_0000 + index as u64,
            );

            robot_handles.push(thread::spawn(move || scout.run()));
        }

        // Collectors.
        for _ in 0..self.collector_count {
            let id = RobotId(next_id);
            next_id += 1;

            let (to_robot_tx, to_robot_rx) = unbounded::<HubToRobot>();
            robot_senders.insert(id, to_robot_tx);

            initial_robots.push(RobotSnapshot {
                id,
                kind: RobotKind::Collector,
                position: base,
                cargo: None,
                state: RobotState::Waiting,
            });

            let collector =
                CollectorRobot::new(id, base, Arc::clone(&map), robot_tx.clone(), to_robot_rx);

            robot_handles.push(thread::spawn(move || collector.run()));
        }

        let hub_map = Arc::clone(&map);

        let hub_handle = thread::spawn(move || {
            run_hub(
                hub_map,
                robot_rx,
                robot_senders,
                snapshot_tx,
                command_rx,
                initial_robots,
            );
        });

        SimulationHandle {
            snapshot_rx,
            command_tx,
            hub_handle: Some(hub_handle),
            robot_handles,
        }
    }
}

/// Handle kept by the UI to receive snapshots and stop the simulation.
pub struct SimulationHandle {
    pub snapshot_rx: Receiver<SimulationSnapshot>,
    command_tx: Sender<SimulationCommand>,
    hub_handle: Option<JoinHandle<()>>,
    robot_handles: Vec<JoinHandle<()>>,
}

impl SimulationHandle {
    pub fn shutdown(&self) {
        let _ = self.command_tx.send(SimulationCommand::Shutdown);
    }
}

impl Drop for SimulationHandle {
    fn drop(&mut self) {
        let _ = self.command_tx.send(SimulationCommand::Shutdown);

        if let Some(handle) = self.hub_handle.take() {
            let _ = handle.join();
        }

        for handle in self.robot_handles.drain(..) {
            let _ = handle.join();
        }
    }
}

fn run_hub(
    map: Arc<Mutex<Map>>,
    robot_rx: Receiver<RobotToHub>,
    robot_senders: HashMap<RobotId, Sender<HubToRobot>>,
    snapshot_tx: Sender<SimulationSnapshot>,
    command_rx: Receiver<SimulationCommand>,
    initial_robots: Vec<RobotSnapshot>,
) {
    let mut tick = 0_u64;

    // The terrain never changes, so read these fixed facts once.
    let (base, width, height, obstacle_positions) = {
        let guard = map.lock().expect("map mutex poisoned");
        let obstacles = guard
            .iter()
            .filter_map(|(position, tile)| matches!(tile, Tile::Obstacle).then_some(position))
            .collect::<Vec<_>>();
        (guard.base(), guard.width(), guard.height(), obstacles)
    };

    // Global knowledge discovered by the robots (empty at first). Resource
    // quantities themselves live in the map and are decremented on collect.
    let mut known_resources = HashMap::<Position, KnownResource>::new();
    let mut known_obstacles = HashSet::<Position>::new();

    let mut robot_states = initial_robots
        .into_iter()
        .map(|snapshot| (snapshot.id, snapshot))
        .collect::<HashMap<_, _>>();

    let mut occupied = HashMap::<Position, RobotId>::new();

    let mut collected_energy = 0_u32;
    let mut collected_crystals = 0_u32;

    let mut events = vec!["Mission initialisee : robots deployes depuis la base".to_string()];

    let mut last_tick = Instant::now();

    loop {
        if let Ok(SimulationCommand::Shutdown) = command_rx.try_recv() {
            for tx in robot_senders.values() {
                let _ = tx.send(HubToRobot::Shutdown);
            }
            break;
        }

        // Drain all pending robot messages without blocking.
        while let Ok(message) = robot_rx.try_recv() {
            let _changed_knowledge = handle_robot_message(
                message,
                &map,
                base,
                &robot_senders,
                &mut robot_states,
                &mut occupied,
                &mut known_resources,
                &mut known_obstacles,
                &mut collected_energy,
                &mut collected_crystals,
                &mut events,
            );
        }

        // New simulation tick every 120 ms.
        if last_tick.elapsed() >= TICK_RATE {
            tick = tick.wrapping_add(1);

            // Re-broadcast the current global knowledge at tick rate so
            // collectors always have the latest discoveries.
            broadcast_knowledge(&robot_senders, &known_resources, &known_obstacles);

            for tx in robot_senders.values() {
                let _ = tx.try_send(HubToRobot::Tick(tick));
            }

            let snapshot = build_snapshot(
                &map,
                tick,
                &obstacle_positions,
                base,
                width,
                height,
                &robot_states,
                collected_energy,
                collected_crystals,
                known_resources.len(),
                known_obstacles.len(),
                &events,
            );

            let _ = snapshot_tx.try_send(snapshot);

            last_tick = Instant::now();
        }

        // Small pause to avoid a 100% CPU busy loop.
        thread::sleep(Duration::from_millis(5));
    }
}

/// Handles a message sent by a robot.
/// Returns true if the global knowledge changed (so the hub re-broadcasts it).
#[allow(clippy::too_many_arguments)]
fn handle_robot_message(
    message: RobotToHub,
    map: &Mutex<Map>,
    base: Position,
    robot_senders: &HashMap<RobotId, Sender<HubToRobot>>,
    robot_states: &mut HashMap<RobotId, RobotSnapshot>,
    occupied: &mut HashMap<Position, RobotId>,
    known_resources: &mut HashMap<Position, KnownResource>,
    known_obstacles: &mut HashSet<Position>,
    collected_energy: &mut u32,
    collected_crystals: &mut u32,
    events: &mut Vec<String>,
) -> bool {
    match message {
        RobotToHub::ResourceDiscovered {
            robot_id,
            position,
            kind,
            quantity,
        } => {
            let actual = match map
                .lock()
                .expect("map mutex poisoned")
                .get(position)
                .copied()
            {
                Some(Tile::Resource(resource)) => KnownResource {
                    position,
                    kind: resource.kind,
                    quantity: resource.quantity,
                },
                _ => return false,
            };

            // Skip if already known with the same data (avoids log spam).
            let changed = known_resources.get(&position) != Some(&actual);
            known_resources.insert(position, actual);

            if changed {
                let reported_matches_actual = actual.kind == kind && actual.quantity == quantity;

                let displayed_kind = if reported_matches_actual {
                    kind
                } else {
                    actual.kind
                };

                let displayed_quantity = if reported_matches_actual {
                    quantity
                } else {
                    actual.quantity
                };

                push_event(
                    events,
                    format!(
                        "R{} decouvre {} {} en ({}, {})",
                        robot_id.0,
                        displayed_quantity,
                        resource_label(displayed_kind),
                        position.x,
                        position.y
                    ),
                );
            }

            changed
        }

        RobotToHub::ObstacleDiscovered { robot_id, position } => {
            let is_obstacle = matches!(
                map.lock().expect("map mutex poisoned").get(position),
                Some(Tile::Obstacle)
            );
            let changed = is_obstacle && known_obstacles.insert(position);

            if changed {
                push_event(
                    events,
                    format!(
                        "R{} signale un obstacle en ({}, {})",
                        robot_id.0, position.x, position.y
                    ),
                );
            }

            changed
        }

        RobotToHub::MoveRequested { robot_id, from, to } => {
            let current = robot_states
                .get(&robot_id)
                .map(|robot| robot.position)
                .unwrap_or(from);

            let one_step = current == to || current.manhattan(to) == 1;

            // Map obstacles are hard constraints; other robots are not, so the
            // simulation never deadlocks in narrow corridors.
            let walkable = map.lock().expect("map mutex poisoned").is_walkable(to);
            let valid = one_step && walkable;

            if valid {
                if current != base {
                    occupied.remove(&current);
                }

                if to != base {
                    occupied.insert(to, robot_id);
                }

                if let Some(robot) = robot_states.get_mut(&robot_id) {
                    robot.position = to;

                    robot.state = if robot.kind == RobotKind::Scout {
                        RobotState::Exploring
                    } else if robot.cargo.is_some() {
                        RobotState::ReturningToBase
                    } else {
                        RobotState::Moving
                    };
                }

                send_to_robot(robot_senders, robot_id, HubToRobot::MoveGranted { to });
            } else {
                send_to_robot(
                    robot_senders,
                    robot_id,
                    HubToRobot::MoveDenied { attempted: to },
                );
            }

            false
        }

        RobotToHub::CollectRequested { robot_id, position } => {
            let robot_position = robot_states.get(&robot_id).map(|robot| robot.position);

            let in_collection_range = robot_position
                .map(|current| current == position || current.manhattan(position) <= 3)
                .unwrap_or(false);

            if !in_collection_range {
                send_to_robot(
                    robot_senders,
                    robot_id,
                    HubToRobot::CollectDenied { position },
                );
                return false;
            }

            // The hub locks the map and removes one unit. This is the single
            // writer; robots only ever read the map.
            let taken = map
                .lock()
                .expect("map mutex poisoned")
                .take_resource_unit(position);

            let Some((kind, remaining)) = taken else {
                known_resources.remove(&position);
                send_to_robot(
                    robot_senders,
                    robot_id,
                    HubToRobot::CollectDenied { position },
                );
                return true;
            };

            if remaining == 0 {
                known_resources.remove(&position);
            } else {
                known_resources.insert(
                    position,
                    KnownResource {
                        position,
                        kind,
                        quantity: remaining,
                    },
                );
            }

            add_cargo_to_snapshot(robot_states, robot_id, kind);

            send_to_robot(
                robot_senders,
                robot_id,
                HubToRobot::CollectGranted {
                    position,
                    kind,
                    remaining,
                },
            );

            push_event(
                events,
                format!(
                    "R{} collecte 1 {} en ({}, {})",
                    robot_id.0,
                    resource_label(kind),
                    position.x,
                    position.y
                ),
            );

            if remaining == 0 {
                push_event(
                    events,
                    format!(
                        "Depot de {} epuise en ({}, {})",
                        resource_label(kind),
                        position.x,
                        position.y
                    ),
                );
            }

            true
        }

        RobotToHub::Deposit {
            robot_id,
            kind,
            amount,
        } => {
            match kind {
                ResourceKind::Energy => *collected_energy += amount,
                ResourceKind::Crystal => *collected_crystals += amount,
            }

            if let Some(robot) = robot_states.get_mut(&robot_id) {
                robot.cargo = None;
                robot.state = RobotState::Waiting;
            }

            push_event(
                events,
                format!(
                    "R{} depose {} {} a la base",
                    robot_id.0,
                    amount,
                    resource_label(kind)
                ),
            );

            false
        }

        RobotToHub::Log { robot_id, text } => {
            push_event(events, format!("R{} {}", robot_id.0, text));
            false
        }
    }
}

fn add_cargo_to_snapshot(
    robot_states: &mut HashMap<RobotId, RobotSnapshot>,
    robot_id: RobotId,
    kind: ResourceKind,
) {
    let Some(robot) = robot_states.get_mut(&robot_id) else {
        return;
    };

    match &mut robot.cargo {
        Some(cargo) if cargo.kind == kind => {
            cargo.amount += 1;
        }
        Some(cargo) => {
            cargo.kind = kind;
            cargo.amount = 1;
        }
        None => {
            robot.cargo = Some(CarriedResource { kind, amount: 1 });
        }
    }

    robot.state = RobotState::Carrying;
}

fn send_to_robot(
    robot_senders: &HashMap<RobotId, Sender<HubToRobot>>,
    robot_id: RobotId,
    message: HubToRobot,
) {
    if let Some(tx) = robot_senders.get(&robot_id) {
        let _ = tx.try_send(message);
    }
}

fn broadcast_knowledge(
    robot_senders: &HashMap<RobotId, Sender<HubToRobot>>,
    known_resources: &HashMap<Position, KnownResource>,
    known_obstacles: &HashSet<Position>,
) {
    let resources = known_resources.values().copied().collect::<Vec<_>>();
    let obstacles = known_obstacles.iter().copied().collect::<Vec<_>>();

    for tx in robot_senders.values() {
        let _ = tx.try_send(HubToRobot::Knowledge {
            resources: resources.clone(),
            obstacles: obstacles.clone(),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn build_snapshot(
    map: &Mutex<Map>,
    tick: u64,
    obstacles: &[Position],
    base: Position,
    width: usize,
    height: usize,
    robots: &HashMap<RobotId, RobotSnapshot>,
    collected_energy: u32,
    collected_crystals: u32,
    known_resources: usize,
    known_obstacles: usize,
    events: &[String],
) -> SimulationSnapshot {
    // Read the remaining resources straight from the map (the source of truth).
    let resources = {
        let guard = map.lock().expect("map mutex poisoned");
        guard
            .iter()
            .filter_map(|(position, tile)| match tile {
                Tile::Resource(resource) => Some(ResourceView {
                    position,
                    kind: resource.kind,
                    quantity: resource.quantity,
                }),
                _ => None,
            })
            .collect::<Vec<_>>()
    };

    let mut robot_list = robots.values().cloned().collect::<Vec<_>>();
    robot_list.sort_by_key(|robot| robot.id);

    SimulationSnapshot {
        tick,
        width,
        height,
        base,
        obstacles: obstacles.to_vec(),
        resources,
        robots: robot_list,
        collected_energy,
        collected_crystals,
        known_resources,
        known_obstacles,
        events: events.to_vec(),
    }
}

fn push_event(events: &mut Vec<String>, text: String) {
    events.push(text);

    if events.len() > EVENT_LOG_LIMIT {
        events.remove(0);
    }
}

fn resource_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Energy => "energie",
        ResourceKind::Crystal => "cristal",
    }
}

#[cfg(test)]
mod tests;
