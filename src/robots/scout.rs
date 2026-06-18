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
