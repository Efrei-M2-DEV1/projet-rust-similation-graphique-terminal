use std::collections::{HashMap, HashSet};

use crate::communication::message::Message;
use crate::utils::Position;
use crate::world::ResourceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownResource {
    pub kind: ResourceKind,
    pub quantity: u32,
}

/// Stock central : compteurs + ce que les robots ont découvert.
#[derive(Debug, Clone)]
pub struct Base {
    position: Position,
    stored_energy: u32,
    stored_crystals: u32,
    known_resources: HashMap<Position, KnownResource>,
    known_obstacles: HashSet<Position>,
}

impl Base {
    pub fn new(position: Position) -> Self {
        Self {
            position,
            stored_energy: 0,
            stored_crystals: 0,
            known_resources: HashMap::new(),
            known_obstacles: HashSet::new(),
        }
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn stored_energy(&self) -> u32 {
        self.stored_energy
    }

    pub fn stored_crystals(&self) -> u32 {
        self.stored_crystals
    }

    pub fn known_resources(&self) -> &HashMap<Position, KnownResource> {
        &self.known_resources
    }

    pub fn known_obstacles(&self) -> &HashSet<Position> {
        &self.known_obstacles
    }

    pub fn known_resource_count(&self) -> usize {
        self.known_resources.len()
    }

    pub fn apply(&mut self, msg: &Message) {
        match msg {
            Message::ResourceFound {
                position,
                kind,
                quantity,
            } => {
                self.known_resources
                    .entry(*position)
                    .and_modify(|r| {
                        if *quantity > r.quantity {
                            r.quantity = *quantity;
                        }
                    })
                    .or_insert(KnownResource {
                        kind: *kind,
                        quantity: *quantity,
                    });
            }
            Message::ObstacleFound { position } => {
                self.known_obstacles.insert(*position);
            }
            Message::ResourceCollected {
                position,
                kind,
                amount,
            } => {
                if let Some(r) = self.known_resources.get_mut(position) {
                    r.quantity = r.quantity.saturating_sub(*amount);
                    if r.quantity == 0 {
                        self.known_resources.remove(position);
                    }
                } else {
                    self.known_resources.insert(
                        *position,
                        KnownResource {
                            kind: *kind,
                            quantity: 0,
                        },
                    );
                }
            }
            Message::ResourceDepleted { position, .. } => {
                self.known_resources.remove(position);
            }
            Message::Deposit { kind, amount } => {
                match kind {
                    ResourceKind::Energy => self.stored_energy += amount,
                    ResourceKind::Crystal => self.stored_crystals += amount,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_updates_known_resources() {
        let mut base = Base::new(Position::new(5, 5));
        let pos = Position::new(10, 3);
        base.apply(&Message::ResourceFound {
            position: pos,
            kind: ResourceKind::Energy,
            quantity: 120,
        });
        assert_eq!(base.known_resource_count(), 1);
        assert_eq!(base.known_resources()[&pos].quantity, 120);
    }

    #[test]
    fn deposit_increments_counters() {
        let mut base = Base::new(Position::new(0, 0));
        base.apply(&Message::Deposit {
            kind: ResourceKind::Energy,
            amount: 15,
        });
        base.apply(&Message::Deposit {
            kind: ResourceKind::Crystal,
            amount: 7,
        });
        assert_eq!(base.stored_energy(), 15);
        assert_eq!(base.stored_crystals(), 7);
    }

    #[test]
    fn depleted_resource_is_removed() {
        let mut base = Base::new(Position::new(0, 0));
        let pos = Position::new(2, 2);
        base.apply(&Message::ResourceFound {
            position: pos,
            kind: ResourceKind::Crystal,
            quantity: 50,
        });
        base.apply(&Message::ResourceDepleted {
            position: pos,
            kind: ResourceKind::Crystal,
        });
        assert_eq!(base.known_resource_count(), 0);
    }

    #[test]
    fn collect_reduces_known_quantity() {
        let mut base = Base::new(Position::new(0, 0));
        let pos = Position::new(8, 4);
        base.apply(&Message::ResourceFound {
            position: pos,
            kind: ResourceKind::Energy,
            quantity: 100,
        });
        base.apply(&Message::ResourceCollected {
            position: pos,
            kind: ResourceKind::Energy,
            amount: 1,
        });
        assert_eq!(base.known_resources()[&pos].quantity, 99);
    }
}
