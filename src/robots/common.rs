//! Types shared between scouts and collectors.

use std::collections::{HashMap, HashSet};

use crate::communication::{KnownResource, RobotId};
use crate::utils::Position;
use crate::world::Tile;

/// Robot business type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotKind {
    Scout,
    Collector,
}

/// Displayable robot activity. Used by the UI instead of raw `&str` literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotState {
    Exploring,
    Waiting,
    Moving,
    ReturningToBase,
    Carrying,
}

impl RobotState {
    /// Short label shown in the UI.
    pub const fn label(self) -> &'static str {
        match self {
            RobotState::Exploring => "exploring",
            RobotState::Waiting => "waiting",
            RobotState::Moving => "moving",
            RobotState::ReturningToBase => "to base",
            RobotState::Carrying => "carrying",
        }
    }
}

/// Resource carried by a collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarriedResource {
    pub kind: crate::world::ResourceKind,
    pub amount: u32,
}

/// Displayable view of a robot. The UI only ever sees snapshots, never the live
/// robots running in their own threads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobotSnapshot {
    pub id: RobotId,
    pub kind: RobotKind,
    pub position: Position,
    pub cargo: Option<CarriedResource>,
    pub state: RobotState,
}

/// Local knowledge of a robot, learned from observations and hub broadcasts.
#[derive(Debug, Default, Clone)]
pub struct LocalKnowledge {
    resources: HashMap<Position, KnownResource>,
    obstacles: HashSet<Position>,
}

impl LocalKnowledge {
    pub fn resources(&self) -> &HashMap<Position, KnownResource> {
        &self.resources
    }

    pub fn obstacles(&self) -> &HashSet<Position> {
        &self.obstacles
    }

    pub fn remove_resource(&mut self, position: Position) {
        self.resources.remove(&position);
    }

    pub fn set_resource(&mut self, resource: KnownResource) {
        self.resources.insert(resource.position, resource);
    }

    /// Replaces local knowledge with the hub's aggregated knowledge, which is
    /// the source of truth for known resources.
    pub fn replace_with(&mut self, resources: &[KnownResource], obstacles: &[Position]) {
        self.resources = resources
            .iter()
            .map(|resource| (resource.position, *resource))
            .collect();

        self.obstacles = obstacles.iter().copied().collect();
    }

    /// Observes a cell. Returns the resource if it is new or updated, else None.
    pub fn observe_resource(&mut self, position: Position, tile: Tile) -> Option<KnownResource> {
        match tile {
            Tile::Resource(resource) => {
                let known = KnownResource {
                    position,
                    kind: resource.kind,
                    quantity: resource.quantity,
                };

                let changed = self.resources.get(&position) != Some(&known);
                self.resources.insert(position, known);

                if changed {
                    Some(known)
                } else {
                    None
                }
            }
            Tile::Empty | Tile::Base => {
                self.resources.remove(&position);
                None
            }
            Tile::Obstacle => None,
        }
    }

    /// Observes an obstacle; returns true if it is new information.
    pub fn observe_obstacle(&mut self, position: Position, tile: Tile) -> bool {
        matches!(tile, Tile::Obstacle) && self.obstacles.insert(position)
    }
}

/// Positions visible around `center` within a square of the given `radius`,
/// clamped to the map bounds.
pub fn visible_positions(center: Position, radius: i32, map: &crate::world::Map) -> Vec<Position> {
    (-radius..=radius)
        .flat_map(|dy| (-radius..=radius).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| Position::new(center.x + dx, center.y + dy))
        .filter(|position| map.in_bounds(*position))
        .collect()
}
