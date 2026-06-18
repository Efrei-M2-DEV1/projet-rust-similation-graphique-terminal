//! Robot éclaireur.
//!
//! Responsabilités :
//! - explorer aléatoirement ;
//! - observer son environnement proche ;
//! - signaler les ressources et obstacles au hub ;
//! - ne jamais collecter.

use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::communication::{HubToRobot, RobotId, RobotToHub};
use crate::robots::common::{visible_positions, LocalKnowledge};
use crate::utils::Position;
use crate::world::Map;

pub const SCOUT_SCAN_RADIUS: i32 = 1;

pub struct ScoutRobot {
    id: RobotId,
    position: Position,
    knowledge: LocalKnowledge,
    map: Arc<Map>,
    to_hub: Sender<RobotToHub>,
    from_hub: Receiver<HubToRobot>,
    rng: StdRng,
    awaiting_move: bool,
}

impl ScoutRobot {
    pub fn new(
        id: RobotId,
        start: Position,
        map: Arc<Map>,
        to_hub: Sender<RobotToHub>,
        from_hub: Receiver<HubToRobot>,
        seed: u64,
    ) -> Self {
        Self {
            id,
            position: start,
            knowledge: LocalKnowledge::default(),
            map,
            to_hub,
            from_hub,
            rng: StdRng::seed_from_u64(seed),
            awaiting_move: false,
        }
    }

    /// Boucle principale du robot.
    ///
    /// Cette fonction tourne dans un thread dédié.
    /// Le scout ne fait rien tant que le hub ne lui envoie pas un Tick.
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
                }

                HubToRobot::Shutdown => break,

                HubToRobot::CollectGranted { .. } | HubToRobot::CollectDenied { .. } => {
                    // Un scout ne collecte jamais.
                }
            }
        }
    }

    fn on_tick(&mut self) {
        self.scan_surroundings();

        if !self.awaiting_move {
            self.request_random_move();
        }
    }

    fn scan_surroundings(&mut self) {
        for position in visible_positions(self.position, SCOUT_SCAN_RADIUS, &self.map) {
            let Some(tile) = self.map.get(position).copied() else {
                continue;
            };

            if self.knowledge.observe_obstacle(position, tile) {
                let _ = self.to_hub.send(RobotToHub::ObstacleDiscovered {
                    robot_id: self.id,
                    position,
                });
            }

            if let Some(resource) = self.knowledge.observe_resource(position, tile) {
                let _ = self.to_hub.send(RobotToHub::ResourceDiscovered {
                    robot_id: self.id,
                    position: resource.position,
                    kind: resource.kind,
                    quantity: resource.quantity,
                });
            }
        }
    }

    fn request_random_move(&mut self) {
        let mut candidates = self
            .position
            .neighbors4()
            .into_iter()
            .filter(|position| self.map.is_walkable(*position))
            .filter(|position| !self.knowledge.is_known_obstacle(*position))
            .collect::<Vec<_>>();

        candidates.shuffle(&mut self.rng);

        if let Some(next) = candidates.first().copied() {
            self.awaiting_move = true;

            let _ = self.to_hub.send(RobotToHub::MoveRequested {
                robot_id: self.id,
                from: self.position,
                to: next,
            });
        }
    }
}
