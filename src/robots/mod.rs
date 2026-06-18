//! Robots autonomes de la simulation.
//!
//! Dev 2 couvre les comportements des eclaireurs, des collecteurs et la
//! navigation. Les robots utilisent le hub de communication existant pour
//! partager leurs decouvertes sans bloquer la boucle UI.

mod collector;
mod common;
mod scout;

pub use collector::{CollectorRobot, CollectorState, DEFAULT_COLLECTOR_CAPACITY};
pub use common::{
    visible_positions, CarriedResource, LocalKnowledge, Robot, RobotKind, RobotSnapshot,
    RobotTickContext,
};
pub use scout::ScoutRobot;
