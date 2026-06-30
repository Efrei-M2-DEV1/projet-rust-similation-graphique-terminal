//! Collector robot.
//!
//! Receives known resources from the hub, picks a target, navigates with A*,
//! asks the hub to collect one unit, then returns to the exact base cell to
//! deposit it.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};

use crate::communication::{HubToRobot, KnownResource, RobotId, RobotToHub};
use crate::robots::common::{CarriedResource, LocalKnowledge};
use crate::utils::Position;
use crate::world::{Map, ResourceKind};

/// Capacity fixed to 1: collect one unit, then return to deposit it.
pub const DEFAULT_COLLECTOR_CAPACITY: u32 = 1;

/// Collection range around a resource: the collector does not need to stand
/// exactly on the deposit cell.
const COLLECTION_RANGE: u32 = 3;

/// Max number of cells temporarily avoided after move denials.
const TEMPORARY_BLOCK_LIMIT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorState {
    Waiting,
    MovingToResource,
    Collecting,
    ReturningToBase,
}

pub struct CollectorRobot {
    id: RobotId,
    position: Position,
    knowledge: LocalKnowledge,
    map: Arc<Mutex<Map>>,
    to_hub: Sender<RobotToHub>,
    from_hub: Receiver<HubToRobot>,
    target: Option<Position>,
    cargo: Option<CarriedResource>,
    capacity: u32,
    state: CollectorState,
    awaiting_move: bool,
    awaiting_collect: bool,
    temporary_blocked: HashSet<Position>,
}

impl CollectorRobot {
    pub fn new(
        id: RobotId,
        start: Position,
        map: Arc<Mutex<Map>>,
        to_hub: Sender<RobotToHub>,
        from_hub: Receiver<HubToRobot>,
    ) -> Self {
        Self {
            id,
            position: start,
            knowledge: LocalKnowledge::default(),
            map,
            to_hub,
            from_hub,
            target: None,
            cargo: None,
            capacity: DEFAULT_COLLECTOR_CAPACITY,
            state: CollectorState::Waiting,
            awaiting_move: false,
            awaiting_collect: false,
            temporary_blocked: HashSet::new(),
        }
    }

    /// Main loop, runs in its own thread.
    pub fn run(mut self) {
        while let Ok(message) = self.from_hub.recv() {
            match message {
                HubToRobot::Tick(_tick) => self.on_tick(),

                HubToRobot::Knowledge {
                    resources,
                    obstacles,
                } => {
                    let previous_count = self.knowledge.resources().len();
                    self.knowledge.replace_with(&resources, &obstacles);
                    let new_count = self.knowledge.resources().len();

                    if previous_count == 0 && new_count > 0 {
                        self.log(format!("connait maintenant {} ressource(s)", new_count));
                    }
                }

                HubToRobot::MoveGranted { to } => {
                    self.position = to;
                    self.awaiting_move = false;

                    // A successful move means the situation unblocked.
                    self.temporary_blocked.clear();
                }

                HubToRobot::MoveDenied { attempted } => {
                    self.awaiting_move = false;

                    self.log(format!(
                        "deplacement refuse vers ({}, {})",
                        attempted.x, attempted.y
                    ));

                    self.temporary_blocked.insert(attempted);

                    if self.temporary_blocked.len() > TEMPORARY_BLOCK_LIMIT {
                        self.temporary_blocked.clear();
                    }
                }

                HubToRobot::CollectGranted {
                    position,
                    kind,
                    remaining,
                } => {
                    self.awaiting_collect = false;
                    self.add_cargo(kind);

                    self.log(format!(
                        "collecte 1 {} en ({}, {})",
                        resource_label(kind),
                        position.x,
                        position.y
                    ));

                    if remaining == 0 {
                        self.knowledge.remove_resource(position);
                        self.target = None;
                    } else {
                        self.knowledge.set_resource(KnownResource {
                            position,
                            kind,
                            quantity: remaining,
                        });
                    }

                    // Capacity = 1: return to base immediately.
                    if self.carried_amount() >= self.capacity {
                        self.state = CollectorState::ReturningToBase;
                        self.target = None;
                        self.temporary_blocked.clear();

                        self.log("retourne a la base avec sa cargaison".to_string());
                    }
                }

                HubToRobot::CollectDenied { position } => {
                    self.awaiting_collect = false;
                    self.knowledge.remove_resource(position);
                    self.target = None;
                    self.state = CollectorState::Waiting;

                    self.log(format!(
                        "collecte refusee en ({}, {})",
                        position.x, position.y
                    ));
                }

                HubToRobot::Shutdown => break,
            }
        }
    }

    fn on_tick(&mut self) {
        if self.awaiting_move || self.awaiting_collect {
            return;
        }

        // Lock the shared map once for the whole tick and pass it down to the
        // read-only helpers. The hub holds the same mutex when it mutates a
        // resource, so reads and writes never race.
        let map_handle = Arc::clone(&self.map);
        let map = map_handle.lock().expect("map mutex poisoned");
        let base = map.base();

        // CASE 1: on the base with cargo -> deposit.
        if self.position == base && self.has_cargo() {
            self.deposit_at_base();
            return;
        }

        // CASE 2: carrying something -> return to the exact base cell.
        if self.has_cargo() {
            self.state = CollectorState::ReturningToBase;
            self.request_move_to_base(&map, base);
            return;
        }

        // CASE 3: already has a resource target.
        if let Some(target) = self.target {
            if self.is_in_collection_range(target) {
                self.request_collect(target);
            } else if self.target_is_known(target) {
                self.state = CollectorState::MovingToResource;
                self.request_move_towards_resource(&map, target);
            } else {
                self.target = None;
                self.state = CollectorState::Waiting;
            }

            return;
        }

        // CASE 4: pick a new known resource.
        if let Some(target) = self.choose_target(&map) {
            self.target = Some(target);
            self.state = CollectorState::MovingToResource;

            self.log(format!("vise une ressource en ({}, {})", target.x, target.y));

            if self.is_in_collection_range(target) {
                self.request_collect(target);
            } else {
                self.request_move_towards_resource(&map, target);
            }

            return;
        }

        self.state = CollectorState::Waiting;
    }

    /// Picks a resource: prefer this collector's kind, else any reachable one.
    fn choose_target(&self, map: &Map) -> Option<Position> {
        self.choose_target_for_kind(map, self.preferred_kind())
            .or_else(|| self.choose_any_target(map))
    }

    fn choose_target_for_kind(&self, map: &Map, kind: ResourceKind) -> Option<Position> {
        self.knowledge
            .resources()
            .values()
            .filter(|resource| resource.quantity > 0)
            .filter(|resource| resource.kind == kind)
            .filter_map(|resource| {
                self.path_len_to_collection_range(map, resource.position)
                    .map(|path_len| (resource.position, path_len))
            })
            .min_by_key(|(_position, path_len)| *path_len)
            .map(|(position, _)| position)
    }

    fn choose_any_target(&self, map: &Map) -> Option<Position> {
        self.knowledge
            .resources()
            .values()
            .filter(|resource| resource.quantity > 0)
            .filter_map(|resource| {
                self.path_len_to_collection_range(map, resource.position)
                    .map(|path_len| (resource.position, path_len))
            })
            .min_by_key(|(_position, path_len)| *path_len)
            .map(|(position, _)| position)
    }

    fn preferred_kind(&self) -> ResourceKind {
        if self.id.0.is_multiple_of(2) {
            ResourceKind::Energy
        } else {
            ResourceKind::Crystal
        }
    }

    /// Cells the collector currently avoids when planning: temporary denials
    /// plus every obstacle it already knows about. This is how the robot stays
    /// aware of where obstacles are.
    fn blocked_positions(&self) -> HashSet<Position> {
        let mut blocked = self.temporary_blocked.clone();
        blocked.extend(self.knowledge.obstacles().iter().copied());
        blocked
    }

    /// Length of the best path to a collection cell around a resource.
    fn path_len_to_collection_range(&self, map: &Map, target: Position) -> Option<usize> {
        let blocked = self.blocked_positions();
        self.collection_positions(map, target)
            .into_iter()
            .filter_map(|goal| {
                crate::pathfinding::find_path_avoiding(map, self.position, goal, &blocked)
                    .map(|path| path.len())
            })
            .min()
    }

    /// Cells from which the collector may collect a resource (not the base).
    fn collection_positions(&self, map: &Map, target: Position) -> Vec<Position> {
        let mut positions = Vec::new();
        let radius = COLLECTION_RANGE as i32;

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let position = Position::new(target.x + dx, target.y + dy);

                if !map.in_bounds(position) {
                    continue;
                }

                if position.manhattan(target) > COLLECTION_RANGE {
                    continue;
                }

                if map.is_walkable(position) {
                    positions.push(position);
                }
            }
        }

        positions
    }

    fn target_is_known(&self, target: Position) -> bool {
        self.knowledge
            .resources()
            .get(&target)
            .map(|resource| resource.quantity > 0)
            .unwrap_or(false)
    }

    fn is_in_collection_range(&self, target: Position) -> bool {
        self.position.manhattan(target) <= COLLECTION_RANGE
    }

    /// Moves towards a collection cell within range of the resource.
    fn request_move_towards_resource(&mut self, map: &Map, target: Position) {
        let Some(goal) = self.best_collection_goal(map, target) else {
            self.log(format!(
                "aucun point de collecte vers ({}, {})",
                target.x, target.y
            ));
            self.target = None;
            self.state = CollectorState::Waiting;
            self.temporary_blocked.clear();
            return;
        };

        if goal == self.position {
            self.request_collect(target);
            return;
        }

        let Some(next) = self.next_step_to(map, goal) else {
            self.log(format!(
                "aucun chemin vers ressource ({}, {})",
                target.x, target.y
            ));
            self.target = None;
            self.state = CollectorState::Waiting;
            self.temporary_blocked.clear();
            return;
        };

        self.request_move(next);
    }

    /// Moves towards the exact base cell (required to deposit).
    fn request_move_to_base(&mut self, map: &Map, base: Position) {
        if self.position == base {
            self.deposit_at_base();
            return;
        }

        let Some(next) = self.next_step_to(map, base) else {
            self.log(format!(
                "chemin exact vers base introuvable ({}, {})",
                base.x, base.y
            ));

            // The base must always stay reachable: forget temporary blocks.
            self.temporary_blocked.clear();
            return;
        };

        self.request_move(next);
    }

    fn request_move(&mut self, next: Position) {
        self.awaiting_move = true;

        let _ = self.to_hub.send(RobotToHub::MoveRequested {
            robot_id: self.id,
            from: self.position,
            to: next,
        });
    }

    fn best_collection_goal(&self, map: &Map, target: Position) -> Option<Position> {
        let blocked = self.blocked_positions();
        self.collection_positions(map, target)
            .into_iter()
            .filter_map(|goal| {
                crate::pathfinding::find_path_avoiding(map, self.position, goal, &blocked)
                    .map(|path| (goal, path.len()))
            })
            .min_by_key(|(_goal, path_len)| *path_len)
            .map(|(goal, _)| goal)
    }

    /// Next step towards a goal, avoiding known obstacles and temporary denials.
    fn next_step_to(&mut self, map: &Map, goal: Position) -> Option<Position> {
        if self.position == goal {
            return None;
        }

        let blocked = self.blocked_positions();
        if let Some(next) =
            crate::pathfinding::next_step_avoiding(map, self.position, goal, &blocked)
        {
            return Some(next);
        }

        // If temporary denials sealed the path, drop them and retry using only
        // the known obstacles.
        if !self.temporary_blocked.is_empty() {
            self.temporary_blocked.clear();
            let blocked = self.blocked_positions();
            return crate::pathfinding::next_step_avoiding(map, self.position, goal, &blocked);
        }

        None
    }

    fn request_collect(&mut self, position: Position) {
        self.awaiting_collect = true;
        self.state = CollectorState::Collecting;

        self.log(format!(
            "demande collecte en ({}, {})",
            position.x, position.y
        ));

        let _ = self.to_hub.send(RobotToHub::CollectRequested {
            robot_id: self.id,
            position,
        });
    }

    fn deposit_at_base(&mut self) {
        if let Some(cargo) = self.cargo.take() {
            self.log(format!(
                "depose {} {} a la base",
                cargo.amount,
                resource_label(cargo.kind)
            ));

            let _ = self.to_hub.send(RobotToHub::Deposit {
                robot_id: self.id,
                kind: cargo.kind,
                amount: cargo.amount,
            });
        }

        self.state = CollectorState::Waiting;
        self.temporary_blocked.clear();
    }

    fn add_cargo(&mut self, kind: ResourceKind) {
        match &mut self.cargo {
            Some(cargo) if cargo.kind == kind => {
                cargo.amount += 1;
            }
            Some(cargo) => {
                cargo.kind = kind;
                cargo.amount = 1;
            }
            None => {
                self.cargo = Some(CarriedResource { kind, amount: 1 });
            }
        }
    }

    fn has_cargo(&self) -> bool {
        self.carried_amount() > 0
    }

    fn carried_amount(&self) -> u32 {
        self.cargo.map(|cargo| cargo.amount).unwrap_or(0)
    }

    fn log(&self, text: String) {
        let _ = self.to_hub.send(RobotToHub::Log {
            robot_id: self.id,
            text,
        });
    }
}

fn resource_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Energy => "energie",
        ResourceKind::Crystal => "cristal",
    }
}
