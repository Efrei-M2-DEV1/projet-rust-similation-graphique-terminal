//! Autonomous simulation robots, each running in its own thread with a local
//! knowledge and a pair of channels to talk to the hub.

mod collector;
mod common;
mod scout;

pub use collector::CollectorRobot;
pub use common::{CarriedResource, RobotKind, RobotSnapshot, RobotState};
pub use scout::ScoutRobot;
