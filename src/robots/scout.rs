use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::communication::{RobotHandle, RobotId};
use crate::robots::common::{
    visible_positions, LocalKnowledge, Robot, RobotKind, RobotTickContext,
};
use crate::utils::Position;

pub const SCOUT_SCAN_RADIUS: i32 = 1;

pub struct ScoutRobot {
    handle: RobotHandle,
    position: Position,
    knowledge: LocalKnowledge,
    rng: StdRng,
    scan_radius: i32,
}

impl ScoutRobot {
    pub fn new(handle: RobotHandle, position: Position, seed: u64) -> Self {
        Self {
            handle,
            position,
            knowledge: LocalKnowledge::default(),
            rng: StdRng::seed_from_u64(seed),
            scan_radius: SCOUT_SCAN_RADIUS,
        }
    }

    pub fn knowledge(&self) -> &LocalKnowledge {
        &self.knowledge
    }

    fn drain_inbox(&mut self) {
        while let Some(envelope) = self.handle.comm.try_recv() {
            self.knowledge.apply_message(&envelope.payload);
        }
    }

    fn scan_surroundings(&mut self, ctx: &RobotTickContext<'_>) {
        for position in visible_positions(self.position, self.scan_radius, &*ctx.map) {
            if let Some(tile) = (&*ctx.map).get(position).copied() {
                if let Some(message) = self.knowledge.observe_tile(position, tile) {
                    let _ = self.handle.comm.send(message);
                }
            }
        }
    }

    fn move_randomly(&mut self, ctx: &mut RobotTickContext<'_>) {
        let mut candidates = self
            .position
            .neighbors4()
            .into_iter()
            .filter(|position| {
                ctx.can_enter(*position, self.position)
                    && !self.knowledge.is_known_obstacle(*position)
            })
            .collect::<Vec<_>>();

        candidates.shuffle(&mut self.rng);

        for next in candidates {
            if ctx.try_move(self.position, next) {
                self.position = next;
                break;
            }
        }
    }
}

impl Robot for ScoutRobot {
    fn id(&self) -> RobotId {
        self.handle.comm.id()
    }

    fn kind(&self) -> RobotKind {
        RobotKind::Scout
    }

    fn position(&self) -> Position {
        self.position
    }

    fn tick(&mut self, ctx: &mut RobotTickContext<'_>) {
        if self.handle.poll_tick().is_none() {
            return;
        }

        self.drain_inbox();
        self.scan_surroundings(ctx);
        self.move_randomly(ctx);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::communication::{CommHub, TickClock};
    use crate::robots::common::RobotTickContext;
    use crate::world::Map;

    /// Construit un ScoutRobot branché sur un CommHub + TickClock de test.
    fn make_scout(map: &Map) -> (ScoutRobot, TickClock, CommHub) {
        let mut hub = CommHub::new(map.base());
        let comm = hub.register_robot();
        let mut clock = TickClock::new(1000);
        let tick_rx = clock.subscribe();
        let handle = RobotHandle { comm, tick_rx };
        let scout = ScoutRobot::new(handle, map.base(), 42);
        (scout, clock, hub)
    }

    #[test]
    fn scout_ne_bouge_pas_sans_tick() {
        // Sans signal de tick, le scout ne doit pas changer de position.
        let map = Map::generate(20, 10, 1);
        let (mut scout, _clock, _hub) = make_scout(&map);
        let pos_before = scout.position();

        let mut map2 = map.clone();
        let mut occupied = HashSet::new();
        let mut ctx = RobotTickContext::new(&mut map2, &mut occupied);
        scout.tick(&mut ctx);

        assert_eq!(scout.position(), pos_before, "pas de mouvement sans tick");
    }

    #[test]
    fn scout_bouge_apres_tick() {
        // Après un tick forcé le scout doit tenter de se déplacer sur
        // une carte sans obstacle autour de la base.
        let map = Map::generate(20, 10, 0);
        let (mut scout, mut clock, _hub) = make_scout(&map);

        clock.force_tick();

        let mut map2 = map.clone();
        let mut occupied = HashSet::new();
        let mut ctx = RobotTickContext::new(&mut map2, &mut occupied);
        scout.tick(&mut ctx);

        // On ne peut pas garantir la destination (aléatoire) mais le
        // robot doit avoir appelé son tick sans paniquer.
        assert!(map2.in_bounds(scout.position()), "position hors carte");
    }

    #[test]
    fn scout_evite_les_obstacles_connus() {
        // On place un obstacle dans la connaissance locale du scout et on
        // vérifie qu'il n'essaie pas d'y aller.
        let map = Map::generate(20, 10, 5);
        let (mut scout, mut clock, _hub) = make_scout(&map);

        // Marquer tous les voisins comme obstacles sauf la base.
        let base = map.base();
        for p in base.neighbors4() {
            scout.knowledge.apply_message(&crate::communication::Message::obstacle_found(p));
        }

        clock.force_tick();

        let mut map2 = map.clone();
        let mut occupied = HashSet::new();
        let mut ctx = RobotTickContext::new(&mut map2, &mut occupied);
        scout.tick(&mut ctx);

        // Le scout doit rester sur place car tous les voisins sont
        // marqués obstacles (ou hors bornes selon la map).
        for p in base.neighbors4() {
            assert_ne!(
                scout.position(), p,
                "le scout ne doit pas entrer dans un obstacle connu"
            );
        }
    }

    #[test]
    fn scout_genre_scout() {
        let map = Map::generate(10, 10, 0);
        let (scout, _clock, _hub) = make_scout(&map);
        assert_eq!(scout.kind(), RobotKind::Scout);
    }
}
