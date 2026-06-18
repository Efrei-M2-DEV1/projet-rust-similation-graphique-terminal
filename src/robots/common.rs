//! Types communs aux robots.
//!
//! On regroupe ici ce qui est partagé entre scouts et collectors :
//! - type de robot ;
//! - snapshot affichable ;
//! - connaissance locale ;
//! - fonctions de visibilité.

use std::collections::{HashMap, HashSet};

use crate::communication::{KnownResource, RobotId};
use crate::utils::Position;
use crate::world::Tile;

/// Type métier d'un robot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotKind {
    Scout,
    Collector,
}

/// Ressource portée par un collecteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarriedResource {
    pub kind: crate::world::ResourceKind,
    pub amount: u32,
}

/// Version affichable d'un robot.
///
/// L'UI ne manipule pas les vrais robots.
/// Elle reçoit seulement des snapshots simples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobotSnapshot {
    pub id: RobotId,
    pub kind: RobotKind,
    pub position: Position,
    pub cargo: Option<CarriedResource>,
    pub state: &'static str,
}

/// Connaissance locale d'un robot.
///
/// Chaque robot ne connaît pas toute la carte au départ.
/// Il apprend progressivement grâce à :
/// - ses observations ;
/// - les messages diffusés par le hub.
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

    pub fn set_resource(&mut self, resource: KnownResource) {
        self.resources.insert(resource.position, resource);
    }

    /// Remplace la connaissance locale par la connaissance agrégée du hub.
    ///
    /// C'est volontairement simple :
    /// le hub est considéré comme la source de vérité pour les ressources connues.
    pub fn replace_with(&mut self, resources: &[KnownResource], obstacles: &[Position]) {
        self.resources = resources
            .iter()
            .map(|resource| (resource.position, *resource))
            .collect();

        self.obstacles = obstacles.iter().copied().collect();
    }

    /// Le robot observe une case.
    ///
    /// Cette méthode renvoie :
    /// - Some(KnownResource) si une ressource nouvelle ou mise à jour est vue ;
    /// - None sinon.
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

    /// Observe un obstacle et renvoie true s'il s'agit d'une nouvelle information.
    pub fn observe_obstacle(&mut self, position: Position, tile: Tile) -> bool {
        matches!(tile, Tile::Obstacle) && self.obstacles.insert(position)
    }
}

/// Renvoie toutes les positions visibles autour d'un robot.
///
/// Rayon 1 = carré 3x3 autour du robot.
/// Cela représente sa perception locale immédiate.
pub fn visible_positions(center: Position, radius: i32, map: &crate::world::Map) -> Vec<Position> {
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
