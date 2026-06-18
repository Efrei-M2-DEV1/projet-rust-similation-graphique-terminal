use crate::communication::{KnownResource, Message, RobotHandle, RobotId};
use crate::robots::common::{
    CarriedResource, LocalKnowledge, Robot, RobotKind, RobotTickContext,
};
use crate::utils::Position;
use crate::world::{ResourceKind, Tile};

pub const DEFAULT_COLLECTOR_CAPACITY: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorState {
    Waiting,
    MovingToResource,
    Collecting,
    ReturningToBase,
}

pub struct CollectorRobot {
    handle: RobotHandle,
    position: Position,
    knowledge: LocalKnowledge,
    target: Option<Position>,
    cargo: Option<CarriedResource>,
    capacity: u32,
    state: CollectorState,
}

impl CollectorRobot {
    pub fn new(handle: RobotHandle, position: Position) -> Self {
        Self {
            handle,
            position,
            knowledge: LocalKnowledge::default(),
            target: None,
            cargo: None,
            capacity: DEFAULT_COLLECTOR_CAPACITY,
            state: CollectorState::Waiting,
        }
    }

    pub fn state(&self) -> CollectorState {
        self.state
    }

    pub fn target(&self) -> Option<Position> {
        self.target
    }

    pub fn carried_amount(&self) -> u32 {
        self.cargo.map(|cargo| cargo.amount).unwrap_or(0)
    }

    fn drain_inbox(&mut self) {
        while let Some(envelope) = self.handle.comm.try_recv() {
            self.knowledge.apply_message(&envelope.payload);
        }
    }

    fn has_cargo(&self) -> bool {
        self.cargo.map(|cargo| cargo.amount > 0).unwrap_or(false)
    }

    fn choose_target(&self, ctx: &RobotTickContext<'_>) -> Option<Position> {
        self.knowledge
            .resources()
            .iter()
            .filter(|(_position, known)| known.quantity > 0)
            .filter(|(position, _known)| {
                matches!((&*ctx.map).get(**position), Some(Tile::Resource(resource)) if resource.quantity > 0)
            })
            .filter_map(|(position, _known)| {
                crate::pathfinding::find_path(&*ctx.map, self.position, *position).map(|path| {
                    (*position, path.len(), self.position.manhattan(*position))
                })
            })
            .min_by_key(|(_position, path_len, distance)| (*path_len, *distance))
            .map(|(position, _path_len, _distance)| position)
    }

    fn target_is_still_valid(&self, ctx: &RobotTickContext<'_>, target: Position) -> bool {
        matches!((&*ctx.map).get(target), Some(Tile::Resource(resource)) if resource.quantity > 0)
    }

    fn move_towards(&mut self, ctx: &mut RobotTickContext<'_>, target: Position) -> bool {
        let blocked = ctx.blocked_for_path(self.position, target);
        let Some(next) =
            crate::pathfinding::next_step_avoiding(&*ctx.map, self.position, target, &blocked)
        else {
            return false;
        };

        if ctx.try_move(self.position, next) {
            self.position = next;
            true
        } else {
            false
        }
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

    fn collect_one_unit(&mut self, ctx: &mut RobotTickContext<'_>) {
        let position = self.position;
        let outcome = match ctx.map.get_mut(position) {
            Some(Tile::Resource(resource)) => {
                if resource.take_one() {
                    Some((resource.kind, resource.quantity))
                } else {
                    None
                }
            }
            _ => None,
        };

        let Some((kind, remaining)) = outcome else {
            self.knowledge.remove_resource(position);
            self.target = None;
            self.state = if self.has_cargo() {
                CollectorState::ReturningToBase
            } else {
                CollectorState::Waiting
            };
            return;
        };

        self.add_cargo(kind);
        let _ = self.handle.comm.send(Message::ResourceCollected {
            position,
            kind,
            amount: 1,
        });

        if remaining == 0 {
            if let Some(tile) = ctx.map.get_mut(position) {
                *tile = Tile::Empty;
            }
            self.knowledge.remove_resource(position);
            self.target = None;
            self.state = CollectorState::ReturningToBase;
            let _ = self
                .handle
                .comm
                .send(Message::ResourceDepleted { position, kind });
            return;
        }

        self.knowledge
            .set_resource(position, KnownResource { kind, quantity: remaining });

        if self.carried_amount() >= self.capacity {
            self.target = None;
            self.state = CollectorState::ReturningToBase;
        } else {
            self.target = Some(position);
            self.state = CollectorState::Collecting;
        }
    }

    fn deposit_at_base(&mut self) {
        if let Some(cargo) = self.cargo.take() {
            let _ = self.handle.comm.send(Message::Deposit {
                kind: cargo.kind,
                amount: cargo.amount,
            });
        }
        self.state = CollectorState::Waiting;
    }
}

impl Robot for CollectorRobot {
    fn id(&self) -> RobotId {
        self.handle.comm.id()
    }

    fn kind(&self) -> RobotKind {
        RobotKind::Collector
    }

    fn position(&self) -> Position {
        self.position
    }

    fn cargo(&self) -> Option<CarriedResource> {
        self.cargo
    }

    fn tick(&mut self, ctx: &mut RobotTickContext<'_>) {
        if self.handle.poll_tick().is_none() {
            return;
        }

        self.drain_inbox();

        if self.position == ctx.map.base() && self.has_cargo() {
            self.deposit_at_base();
            return;
        }

        if self.has_cargo() {
            self.state = CollectorState::ReturningToBase;
            let base = ctx.map.base();
            let _ = self.move_towards(ctx, base);
            return;
        }

        if let Some(target) = self.target {
            if self.position == target {
                self.collect_one_unit(ctx);
            } else if self.target_is_still_valid(ctx, target) {
                self.state = CollectorState::MovingToResource;
                if !self.move_towards(ctx, target) {
                    self.target = None;
                    self.state = CollectorState::Waiting;
                }
            } else {
                self.knowledge.remove_resource(target);
                self.target = None;
                self.state = CollectorState::Waiting;
            }
            return;
        }

        if let Some(target) = self.choose_target(ctx) {
            self.target = Some(target);
            self.state = CollectorState::MovingToResource;
            if self.position == target {
                self.collect_one_unit(ctx);
            } else {
                let _ = self.move_towards(ctx, target);
            }
        } else {
            self.state = CollectorState::Waiting;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::communication::{CommHub, Message, TickClock};
    use crate::robots::common::RobotTickContext;
    use crate::utils::Position;
    use crate::world::{Map, Resource, ResourceKind, Tile};

    /// Construit un CollectorRobot branché sur un CommHub + TickClock.
    fn make_collector(map: &Map) -> (CollectorRobot, TickClock, CommHub) {
        let mut hub = CommHub::new(map.base());
        let comm = hub.register_robot();
        let mut clock = TickClock::new(1000);
        let tick_rx = clock.subscribe();
        let handle = RobotHandle { comm, tick_rx };
        let collector = CollectorRobot::new(handle, map.base());
        (collector, clock, hub)
    }

    #[test]
    fn collector_en_attente_sans_ressource_connue() {
        // Sans ressource dans sa connaissance locale, le collector doit
        // rester en état Waiting après un tick.
        let map = Map::generate(20, 10, 1);
        let (mut collector, mut clock, _hub) = make_collector(&map);
        clock.force_tick();

        let mut map2 = map.clone();
        let mut occupied = HashSet::new();
        let mut ctx = RobotTickContext::new(&mut map2, &mut occupied);
        collector.tick(&mut ctx);

        assert_eq!(collector.state(), CollectorState::Waiting);
    }

    #[test]
    fn collector_se_dirige_vers_ressource_connue() {
        // On signale une ressource voisine via un message ; après le tick
        // le collector doit passer en MovingToResource ou Collecting.
        let mut map = Map::generate(20, 10, 2);
        let target = Position::new(map.base().x + 2, map.base().y);

        // Placer la ressource sur la carte.
        map.set(target, Tile::Resource(Resource::new(ResourceKind::Energy, 10)));

        let (mut collector, mut clock, _hub) = make_collector(&map);

        // Informer le collector de la ressource via le canal de messages.
        collector.knowledge.apply_message(&Message::resource_found(
            target,
            ResourceKind::Energy,
            10,
        ));

        clock.force_tick();

        let mut map2 = map.clone();
        let mut occupied = HashSet::new();
        let mut ctx = RobotTickContext::new(&mut map2, &mut occupied);
        collector.tick(&mut ctx);

        assert!(
            matches!(
                collector.state(),
                CollectorState::MovingToResource | CollectorState::Collecting
            ),
            "état attendu: MovingToResource ou Collecting, obtenu: {:?}",
            collector.state()
        );
    }

    #[test]
    fn collector_ne_bouge_pas_sans_tick() {
        // Sans signal de tick, la position ne change pas.
        let map = Map::generate(20, 10, 3);
        let (mut collector, _clock, _hub) = make_collector(&map);
        let pos_before = collector.position();

        let mut map2 = map.clone();
        let mut occupied = HashSet::new();
        let mut ctx = RobotTickContext::new(&mut map2, &mut occupied);
        collector.tick(&mut ctx);

        assert_eq!(collector.position(), pos_before);
    }

    #[test]
    fn collector_genre_collector() {
        let map = Map::generate(10, 10, 0);
        let (collector, _clock, _hub) = make_collector(&map);
        assert_eq!(collector.kind(), RobotKind::Collector);
    }
}
