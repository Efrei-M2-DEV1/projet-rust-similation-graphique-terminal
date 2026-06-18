//! Robot collecteur.
//!
//! Responsabilités :
//! - recevoir les ressources connues depuis le hub ;
//! - choisir une cible ;
//! - se déplacer avec A* ;
//! - demander au hub de collecter une unité ;
//! - revenir à la base pour déposer.

use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};

use crate::communication::{HubToRobot, KnownResource, RobotId, RobotToHub};
use crate::robots::common::{CarriedResource, LocalKnowledge};
use crate::utils::Position;
use crate::world::{Map, ResourceKind};

pub const DEFAULT_COLLECTOR_CAPACITY: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorState {
    Waiting,
    MovingToResource,
    Collecting,
    ReturningToBase,
}

impl CollectorState {
    pub const fn label(self) -> &'static str {
        match self {
            CollectorState::Waiting => "waiting",
            CollectorState::MovingToResource => "to resource",
            CollectorState::Collecting => "collecting",
            CollectorState::ReturningToBase => "to base",
        }
    }
}

pub struct CollectorRobot {
    id: RobotId,
    position: Position,
    knowledge: LocalKnowledge,
    map: Arc<Map>,
    to_hub: Sender<RobotToHub>,
    from_hub: Receiver<HubToRobot>,
    target: Option<Position>,
    cargo: Option<CarriedResource>,
    capacity: u32,
    state: CollectorState,
    awaiting_move: bool,
    awaiting_collect: bool,
}

impl CollectorRobot {
    pub fn new(
        id: RobotId,
        start: Position,
        map: Arc<Map>,
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
        }
    }

    /// Boucle principale du collecteur.
    ///
    /// Elle tourne dans un thread dédié.
    /// Le collecteur réagit aux messages du hub.
    pub fn run(mut self) {
        while let Ok(message) = self.from_hub.recv() {
            match message {
                HubToRobot::Tick(_tick) => self.on_tick(),

                HubToRobot::Knowledge {
                    resources,
                    obstacles,
                } => {
                    self.knowledge.replace_with(&resources, &obstacles);
                }

                HubToRobot::MoveGranted { to } => {
                    self.position = to;
                    self.awaiting_move = false;
                }

                HubToRobot::MoveDenied { .. } => {
                    self.awaiting_move = false;
                    self.target = None;
                    self.state = CollectorState::Waiting;
                }

                HubToRobot::CollectGranted {
                    position,
                    kind,
                    remaining,
                } => {
                    self.awaiting_collect = false;
                    self.add_cargo(kind);

                    if remaining == 0 {
                        self.knowledge.remove_resource(position);
                        self.target = None;
                    } else {
                        self.knowledge.set_resource(KnownResource {
                            position,
                            kind,
                            quantity: remaining,
                        });
                        self.target = Some(position);
                    }

                    if self.carried_amount() >= self.capacity {
                        self.state = CollectorState::ReturningToBase;
                        self.target = None;
                    } else {
                        self.state = CollectorState::Collecting;
                    }
                }

                HubToRobot::CollectDenied { position } => {
                    self.awaiting_collect = false;
                    self.knowledge.remove_resource(position);
                    self.target = None;
                    self.state = CollectorState::Waiting;
                }

                HubToRobot::Shutdown => break,
            }
        }
    }

    fn on_tick(&mut self) {
        if self.awaiting_move || self.awaiting_collect {
            return;
        }

        let base = self.map.base();

        if self.position == base && self.has_cargo() {
            self.deposit_at_base();
            return;
        }

        if self.carried_amount() >= self.capacity {
            self.state = CollectorState::ReturningToBase;
            self.request_move_towards(base);
            return;
        }

        if let Some(target) = self.target {
            if self.position == target {
                self.request_collect(target);
            } else if self.target_is_known(target) {
                self.state = CollectorState::MovingToResource;
                self.request_move_towards(target);
            } else {
                self.target = None;
                self.state = CollectorState::Waiting;
            }

            return;
        }

        if let Some(target) = self.choose_target() {
            self.target = Some(target);
            self.state = CollectorState::MovingToResource;
            self.request_move_towards(target);
            return;
        }

        if self.has_cargo() {
            self.state = CollectorState::ReturningToBase;
            self.request_move_towards(base);
        } else {
            self.state = CollectorState::Waiting;
        }
    }

    fn choose_target(&self) -> Option<Position> {
        self.knowledge
            .resources()
            .values()
            .filter(|resource| resource.quantity > 0)
            .filter_map(|resource| {
                crate::pathfinding::find_path(&self.map, self.position, resource.position)
                    .map(|path| (resource.position, path.len()))
            })
            .min_by_key(|(_position, path_len)| *path_len)
            .map(|(position, _)| position)
    }

    fn target_is_known(&self, target: Position) -> bool {
        self.knowledge
            .resources()
            .get(&target)
            .map(|resource| resource.quantity > 0)
            .unwrap_or(false)
    }

    fn request_move_towards(&mut self, target: Position) {
        let Some(next) = crate::pathfinding::next_step_avoiding(
            &self.map,
            self.position,
            target,
            &std::collections::HashSet::new(),
        ) else {
            self.target = None;
            self.state = CollectorState::Waiting;
            return;
        };

        self.awaiting_move = true;

        let _ = self.to_hub.send(RobotToHub::MoveRequested {
            robot_id: self.id,
            from: self.position,
            to: next,
        });
    }

    fn request_collect(&mut self, position: Position) {
        self.awaiting_collect = true;
        self.state = CollectorState::Collecting;

        let _ = self.to_hub.send(RobotToHub::CollectRequested {
            robot_id: self.id,
            position,
        });
    }

    fn deposit_at_base(&mut self) {
        if let Some(cargo) = self.cargo.take() {
            let _ = self.to_hub.send(RobotToHub::Deposit {
                robot_id: self.id,
                kind: cargo.kind,
                amount: cargo.amount,
            });
        }

        self.state = CollectorState::Waiting;
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
}
