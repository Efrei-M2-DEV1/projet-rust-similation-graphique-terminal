//! Moteur de simulation concurrent.
//!
//! C'est le cœur de la nouvelle architecture.
//!
//! Principe :
//! - l'UI Ratatui tourne dans le thread principal ;
//! - le hub tourne dans un thread dédié ;
//! - chaque robot tourne dans son propre thread ;
//! - tout le monde communique par channels.
//!
//! Le hub possède l'état global officiel :
//! - ressources restantes ;
//! - compteurs collectés ;
//! - positions des robots ;
//! - connaissances agrégées ;
//! - journal d'événements.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::communication::{HubToRobot, KnownResource, RobotId, RobotToHub, SimulationCommand};
use crate::robots::{CollectorRobot, RobotKind, RobotSnapshot, ScoutRobot};
use crate::utils::Position;
use crate::world::{Map, ResourceKind, Tile};

const TICK_RATE: Duration = Duration::from_millis(120);
const EVENT_LOG_LIMIT: usize = 10;

/// Ressource affichable dans l'UI.
#[derive(Debug, Clone)]
pub struct ResourceView {
    pub position: Position,
    pub kind: ResourceKind,
    pub quantity: u32,
}

/// Snapshot complet envoyé à Ratatui.
///
/// L'UI ne manipule pas la simulation vivante.
/// Elle reçoit seulement cette structure prête à afficher.
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

/// Moteur de simulation à configurer avant lancement.
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
        let map = Arc::new(self.map);

        let (robot_tx, robot_rx) = unbounded::<RobotToHub>();
        let (snapshot_tx, snapshot_rx) = unbounded::<SimulationSnapshot>();
        let (command_tx, command_rx) = unbounded::<SimulationCommand>();

        let mut robot_senders = HashMap::<RobotId, Sender<HubToRobot>>::new();
        let mut robot_handles = Vec::<JoinHandle<()>>::new();
        let mut initial_robots = Vec::<RobotSnapshot>::new();

        let base = map.base();
        let mut next_id = 0usize;

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
                state: "exploring",
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
                state: "waiting",
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

/// Objet gardé par l'UI pour recevoir les snapshots et arrêter la simulation.
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
    map: Arc<Map>,
    robot_rx: Receiver<RobotToHub>,
    robot_senders: HashMap<RobotId, Sender<HubToRobot>>,
    snapshot_tx: Sender<SimulationSnapshot>,
    command_rx: Receiver<SimulationCommand>,
    initial_robots: Vec<RobotSnapshot>,
) {
    let mut tick = 0_u64;

    let obstacle_positions = map
        .iter()
        .filter_map(|(position, tile)| {
            if matches!(tile, Tile::Obstacle) {
                Some(position)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut resources = collect_initial_resources(&map);
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

        while let Ok(message) = robot_rx.try_recv() {
            handle_robot_message(
                message,
                &map,
                &robot_senders,
                &mut robot_states,
                &mut occupied,
                &mut resources,
                &mut known_resources,
                &mut known_obstacles,
                &mut collected_energy,
                &mut collected_crystals,
                &mut events,
            );
        }

        broadcast_knowledge(&robot_senders, &known_resources, &known_obstacles);

        let snapshot = build_snapshot(
            &map,
            tick,
            &obstacle_positions,
            &resources,
            &robot_states,
            collected_energy,
            collected_crystals,
            known_resources.len(),
            known_obstacles.len(),
            &events,
        );

        let _ = snapshot_tx.try_send(snapshot);

        if last_tick.elapsed() >= TICK_RATE {
            tick = tick.wrapping_add(1);

            for tx in robot_senders.values() {
                let _ = tx.try_send(HubToRobot::Tick(tick));
            }

            last_tick = Instant::now();
        }

        thread::sleep(Duration::from_millis(5));
    }
}

fn collect_initial_resources(map: &Map) -> HashMap<Position, KnownResource> {
    map.iter()
        .filter_map(|(position, tile)| match tile {
            Tile::Resource(resource) => Some((
                position,
                KnownResource {
                    position,
                    kind: resource.kind,
                    quantity: resource.quantity,
                },
            )),
            _ => None,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn handle_robot_message(
    message: RobotToHub,
    map: &Map,
    robot_senders: &HashMap<RobotId, Sender<HubToRobot>>,
    robot_states: &mut HashMap<RobotId, RobotSnapshot>,
    occupied: &mut HashMap<Position, RobotId>,
    resources: &mut HashMap<Position, KnownResource>,
    known_resources: &mut HashMap<Position, KnownResource>,
    known_obstacles: &mut HashSet<Position>,
    collected_energy: &mut u32,
    collected_crystals: &mut u32,
    events: &mut Vec<String>,
) {
    match message {
        RobotToHub::ResourceDiscovered {
            robot_id, position, ..
        } => {
            if let Some(actual) = resources.get(&position).copied() {
                known_resources.insert(position, actual);
                push_event(
                    events,
                    format!(
                        "R{} decouvre {} {} en ({}, {})",
                        robot_id.0,
                        actual.quantity,
                        resource_label(actual.kind),
                        position.x,
                        position.y
                    ),
                );
            }
        }

        RobotToHub::ObstacleDiscovered { robot_id, position } => {
            if matches!(map.get(position), Some(Tile::Obstacle)) && known_obstacles.insert(position)
            {
                push_event(
                    events,
                    format!(
                        "R{} signale un obstacle en ({}, {})",
                        robot_id.0, position.x, position.y
                    ),
                );
            }
        }

        RobotToHub::MoveRequested { robot_id, from, to } => {
            let current = robot_states
                .get(&robot_id)
                .map(|robot| robot.position)
                .unwrap_or(from);

            let one_step = current == to || current.manhattan(to) == 1;
            let free = to == map.base() || !occupied.contains_key(&to);
            let valid = one_step && map.is_walkable(to) && free;

            if valid {
                if current != map.base() {
                    occupied.remove(&current);
                }

                if to != map.base() {
                    occupied.insert(to, robot_id);
                }

                if let Some(robot) = robot_states.get_mut(&robot_id) {
                    robot.position = to;
                }

                send_to_robot(robot_senders, robot_id, HubToRobot::MoveGranted { to });
            } else {
                send_to_robot(
                    robot_senders,
                    robot_id,
                    HubToRobot::MoveDenied { attempted: to },
                );
            }
        }

        RobotToHub::CollectRequested { robot_id, position } => {
            let robot_position = robot_states.get(&robot_id).map(|robot| robot.position);

            if robot_position != Some(position) {
                send_to_robot(
                    robot_senders,
                    robot_id,
                    HubToRobot::CollectDenied { position },
                );
                return;
            }

            let Some(resource_before) = resources.get(&position).copied() else {
                send_to_robot(
                    robot_senders,
                    robot_id,
                    HubToRobot::CollectDenied { position },
                );
                return;
            };

            if resource_before.quantity == 0 {
                resources.remove(&position);
                known_resources.remove(&position);

                send_to_robot(
                    robot_senders,
                    robot_id,
                    HubToRobot::CollectDenied { position },
                );
                return;
            }

            let remaining = resource_before.quantity - 1;

            if remaining == 0 {
                resources.remove(&position);
                known_resources.remove(&position);
            } else {
                resources.insert(
                    position,
                    KnownResource {
                        position,
                        kind: resource_before.kind,
                        quantity: remaining,
                    },
                );
                known_resources.insert(
                    position,
                    KnownResource {
                        position,
                        kind: resource_before.kind,
                        quantity: remaining,
                    },
                );
            }

            send_to_robot(
                robot_senders,
                robot_id,
                HubToRobot::CollectGranted {
                    position,
                    kind: resource_before.kind,
                    remaining,
                },
            );

            push_event(
                events,
                format!(
                    "R{} collecte 1 {} en ({}, {})",
                    robot_id.0,
                    resource_label(resource_before.kind),
                    position.x,
                    position.y
                ),
            );

            if remaining == 0 {
                push_event(
                    events,
                    format!(
                        "Depot de {} epuise en ({}, {})",
                        resource_label(resource_before.kind),
                        position.x,
                        position.y
                    ),
                );
            }
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
        }

        RobotToHub::Log { robot_id, text } => {
            push_event(events, format!("R{} {}", robot_id.0, text));
        }
    }
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
    map: &Map,
    tick: u64,
    obstacles: &[Position],
    resources: &HashMap<Position, KnownResource>,
    robots: &HashMap<RobotId, RobotSnapshot>,
    collected_energy: u32,
    collected_crystals: u32,
    known_resources: usize,
    known_obstacles: usize,
    events: &[String],
) -> SimulationSnapshot {
    let mut robot_list = robots.values().cloned().collect::<Vec<_>>();
    robot_list.sort_by_key(|robot| robot.id);

    SimulationSnapshot {
        tick,
        width: map.width(),
        height: map.height(),
        base: map.base(),
        obstacles: obstacles.to_vec(),
        resources: resources
            .values()
            .map(|resource| ResourceView {
                position: resource.position,
                kind: resource.kind,
                quantity: resource.quantity,
            })
            .collect(),
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
