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
