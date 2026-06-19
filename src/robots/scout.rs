//! Scout robot.
//!
//! Explores at random, observes its surroundings, reports resources and
//! obstacles to the hub, and never collects.

use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::communication::{HubToRobot, RobotId, RobotToHub};
use crate::robots::common::{visible_positions, LocalKnowledge};
use crate::utils::Position;
use crate::world::{Map, Tile};

/// Local scan radius of the scout (kept small so knowledge stays local).
pub const SCOUT_SCAN_RADIUS: i32 = 2;

pub struct ScoutRobot {
    id: RobotId,
    position: Position,
    knowledge: LocalKnowledge,
    map: Arc<Mutex<Map>>,
    to_hub: Sender<RobotToHub>,
    from_hub: Receiver<HubToRobot>,
    rng: StdRng,
    awaiting_move: bool,
}

impl ScoutRobot {
    pub fn new(
        id: RobotId,
        start: Position,
        map: Arc<Mutex<Map>>,
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

    /// Main loop, runs in a dedicated thread. The scout stays idle until the
    /// hub sends a Tick.
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
                    // A scout never collects.
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
        // Lock the shared map briefly to read the visible tiles, then release
        // it before sending any message.
        let observations: Vec<(Position, Tile)> = {
            let map = self.map.lock().expect("map mutex poisoned");
            visible_positions(self.position, SCOUT_SCAN_RADIUS, &map)
                .into_iter()
                .filter_map(|position| map.get(position).copied().map(|tile| (position, tile)))
                .collect()
        };

        for (position, tile) in observations {
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
        let known_obstacles = self.knowledge.obstacles().clone();
        let mut candidates = {
            let map = self.map.lock().expect("map mutex poisoned");
            self.position
                .neighbors4()
                .into_iter()
                .filter(|position| map.is_walkable(*position))
                .filter(|position| !known_obstacles.contains(position))
                .collect::<Vec<_>>()
        };

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
