use std::collections::{HashMap, HashSet};

use crate::communication::{KnownResource, Message, RobotId};
use crate::utils::Position;
use crate::world::{Map, Tile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotKind {
    Scout,
    Collector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarriedResource {
    pub kind: crate::world::ResourceKind,
    pub amount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RobotSnapshot {
    pub id: RobotId,
    pub kind: RobotKind,
    pub position: Position,
    pub cargo: Option<CarriedResource>,
}

pub trait Robot {
    fn id(&self) -> RobotId;
    fn kind(&self) -> RobotKind;
    fn position(&self) -> Position;
    fn cargo(&self) -> Option<CarriedResource> {
        None
    }

    fn tick(&mut self, ctx: &mut RobotTickContext<'_>);

    fn snapshot(&self) -> RobotSnapshot {
        RobotSnapshot {
            id: self.id(),
            kind: self.kind(),
            position: self.position(),
            cargo: self.cargo(),
        }
    }
}

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

    pub fn is_known_obstacle(&self, position: Position) -> bool {
        self.obstacles.contains(&position)
    }

    pub fn remove_resource(&mut self, position: Position) {
        self.resources.remove(&position);
    }

    pub fn set_resource(&mut self, position: Position, resource: KnownResource) {
        self.resources.insert(position, resource);
    }

    pub fn apply_message(&mut self, msg: &Message) {
        match msg {
            Message::ResourceFound {
                position,
                kind,
                quantity,
            } => {
                self.resources.insert(
                    *position,
                    KnownResource {
                        kind: *kind,
                        quantity: *quantity,
                    },
                );
            }
            Message::ObstacleFound { position } => {
                self.obstacles.insert(*position);
            }
            Message::ResourceCollected {
                position, amount, ..
            } => {
                if let Some(resource) = self.resources.get_mut(position) {
                    resource.quantity = resource.quantity.saturating_sub(*amount);
                    if resource.quantity == 0 {
                        self.resources.remove(position);
                    }
                }
            }
            Message::ResourceDepleted { position, .. } => {
                self.resources.remove(position);
            }
            Message::Deposit { .. } => {}
        }
    }

    /// Memorise ce que le robot voit et renvoie un message a partager si
    /// l'information est nouvelle ou a change.
    pub fn observe_tile(&mut self, position: Position, tile: Tile) -> Option<Message> {
        match tile {
            Tile::Obstacle => {
                if self.obstacles.insert(position) {
                    Some(Message::obstacle_found(position))
                } else {
                    None
                }
            }
            Tile::Resource(resource) => {
                let known = KnownResource {
                    kind: resource.kind,
                    quantity: resource.quantity,
                };
                let should_share = self.resources.get(&position) != Some(&known);
                self.resources.insert(position, known);

                if should_share {
                    Some(Message::resource_found(
                        position,
                        resource.kind,
                        resource.quantity,
                    ))
                } else {
                    None
                }
            }
            Tile::Empty | Tile::Base => {
                self.resources.remove(&position);
                None
            }
        }
    }
}

pub struct RobotTickContext<'a> {
    pub map: &'a mut Map,
    occupied: &'a mut HashSet<Position>,
}

impl<'a> RobotTickContext<'a> {
    pub fn new(map: &'a mut Map, occupied: &'a mut HashSet<Position>) -> Self {
        Self { map, occupied }
    }

    pub fn can_enter(&self, position: Position, current: Position) -> bool {
        self.map.is_walkable(position)
            && (position == current
                || position == self.map.base()
                || !self.occupied.contains(&position))
    }

    pub fn try_move(&mut self, from: Position, to: Position) -> bool {
        if from == to {
            return true;
        }

        if !self.can_enter(to, from) {
            return false;
        }

        if from != self.map.base() {
            self.occupied.remove(&from);
        }
        if to != self.map.base() {
            self.occupied.insert(to);
        }

        true
    }

    pub fn blocked_for_path(&self, current: Position, goal: Position) -> HashSet<Position> {
        self.occupied
            .iter()
            .copied()
            .filter(|position| {
                *position != current && *position != goal && *position != self.map.base()
            })
            .collect()
    }
}

pub fn visible_positions(center: Position, radius: i32, map: &Map) -> Vec<Position> {
    let mut positions = Vec::new();

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let position = Position::new(center.x + dx, center.y + dy);
            if map.in_bounds(position) {
                positions.push(position);
            }
        }
    }

    positions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::communication::Message;
    use crate::utils::Position;
    use crate::world::{Resource, ResourceKind, Tile};

    fn pos(x: i32, y: i32) -> Position {
        Position::new(x, y)
    }

    #[test]
    fn connaissance_obstacle_via_message() {
        // Un message ObstacleFound doit être mémorisé.
        let mut knowledge = LocalKnowledge::default();
        knowledge.apply_message(&Message::obstacle_found(pos(3, 3)));
        assert!(knowledge.is_known_obstacle(pos(3, 3)));
    }

    #[test]
    fn connaissance_ressource_via_message() {
        // Un message ResourceFound doit alimenter la map de ressources connues.
        let mut knowledge = LocalKnowledge::default();
        knowledge.apply_message(&Message::resource_found(pos(1, 1), ResourceKind::Energy, 50));
        assert!(knowledge.resources().contains_key(&pos(1, 1)));
    }

    #[test]
    fn ressource_epuisee_est_supprimee() {
        // Un message ResourceDepleted doit retirer la ressource de la connaissance.
        let mut knowledge = LocalKnowledge::default();
        knowledge.apply_message(&Message::resource_found(pos(2, 2), ResourceKind::Crystal, 10));
        knowledge.apply_message(&Message::ResourceDepleted {
            position: pos(2, 2),
            kind: ResourceKind::Crystal,
        });
        assert!(!knowledge.resources().contains_key(&pos(2, 2)));
    }

    #[test]
    fn observe_tile_obstacle_genere_message() {
        // Observer un obstacle pour la première fois doit renvoyer un message.
        let mut knowledge = LocalKnowledge::default();
        let msg = knowledge.observe_tile(pos(5, 5), Tile::Obstacle);
        assert!(msg.is_some());

        // La deuxième observation du même obstacle ne génère rien (déjà connu).
        let msg2 = knowledge.observe_tile(pos(5, 5), Tile::Obstacle);
        assert!(msg2.is_none());
    }

    #[test]
    fn observe_tile_ressource_genere_message() {
        let mut knowledge = LocalKnowledge::default();
        let r = Resource::new(ResourceKind::Energy, 100);
        let msg = knowledge.observe_tile(pos(4, 4), Tile::Resource(r));
        assert!(msg.is_some());
    }
}
