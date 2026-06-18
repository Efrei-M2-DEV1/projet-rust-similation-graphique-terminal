pub mod base;
pub mod hub;
pub mod message;
pub mod scheduler;

pub use base::{Base, KnownResource};
pub use hub::{CommHub, RobotComm};
pub use message::{Envelope, Message, RobotId};
pub use scheduler::{RobotHandle, TickClock, DEFAULT_TICK_HZ};

pub type Comm = RobotComm;

use crate::utils::Position;

pub fn setup(base_position: Position) -> (CommHub, TickClock) {
    (CommHub::new(base_position), TickClock::default_hz())
}

pub fn register_robot(hub: &mut CommHub, clock: &mut TickClock) -> RobotHandle {
    let comm = hub.register_robot();
    let tick_rx = clock.subscribe();
    RobotHandle { comm, tick_rx }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Map, ResourceKind};

    #[test]
    fn scout_shares_discovery_then_collector_deposits() {
        let (mut hub, mut clock) = setup(Position::new(10, 5));
        let scout = register_robot(&mut hub, &mut clock);
        let collector = register_robot(&mut hub, &mut clock);

        let pos = Position::new(20, 12);
        scout
            .comm
            .send(Message::resource_found(pos, ResourceKind::Energy, 150));
        hub.poll();

        let base_ref = hub.base();
        let base = base_ref.lock().unwrap();
        assert_eq!(base.known_resource_count(), 1);
        drop(base);
        assert_eq!(collector.comm.drain_inbox().len(), 1);

        collector.comm.send(Message::Deposit {
            kind: ResourceKind::Energy,
            amount: 30,
        });
        hub.poll();

        let base_ref = hub.base();
        let base = base_ref.lock().unwrap();
        assert_eq!(base.stored_energy(), 30);
    }

    #[test]
    fn collect_one_unit_then_deposit() {
        let map = Map::generate(40, 20, 99);
        let (mut hub, mut clock) = setup(map.base());
        let scout = register_robot(&mut hub, &mut clock);
        let collector = register_robot(&mut hub, &mut clock);

        let pos = Position::new(25, 10);
        scout
            .comm
            .send(Message::resource_found(pos, ResourceKind::Crystal, 80));
        hub.poll();
        assert_eq!(collector.comm.drain_inbox().len(), 1);

        collector.comm.send(Message::ResourceCollected {
            position: pos,
            kind: ResourceKind::Crystal,
            amount: 1,
        });
        hub.poll();

        let base_ref = hub.base();
        let base = base_ref.lock().unwrap();
        assert_eq!(base.known_resources()[&pos].quantity, 79);
        drop(base);

        collector.comm.send(Message::Deposit {
            kind: ResourceKind::Crystal,
            amount: 1,
        });
        hub.poll();

        let base_ref = hub.base();
        let base = base_ref.lock().unwrap();
        assert_eq!(base.stored_crystals(), 1);
    }

    #[test]
    fn base_sits_on_map_center() {
        let map = Map::generate(60, 20, 42);
        let (hub, _) = setup(map.base());
        let base_ref = hub.base();
        let base = base_ref.lock().unwrap();
        assert_eq!(base.position(), map.base());
    }

    #[test]
    fn default_tick_interval_is_100ms() {
        let clock = TickClock::default_hz();
        assert_eq!(DEFAULT_TICK_HZ, 10);
        assert_eq!(clock.interval(), std::time::Duration::from_millis(100));
    }

    #[test]
    fn tick_reaches_all_robots() {
        let (mut hub, mut clock) = setup(Position::new(0, 0));
        let r1 = register_robot(&mut hub, &mut clock);
        let r2 = register_robot(&mut hub, &mut clock);

        clock.force_tick();
        assert_eq!(r1.poll_tick(), Some(1));
        assert_eq!(r2.poll_tick(), Some(1));
    }
}
