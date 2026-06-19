//! Robots autonomes de la simulation.
//!
//! Chaque robot tourne dans son propre thread.
//! Il possède :
//! - une position ;
//! - une connaissance locale ;
//! - un canal pour recevoir les messages du hub ;
//! - un canal pour envoyer des messages au hub.

mod collector;
mod common;
mod scout;

pub use collector::CollectorRobot;
pub use common::{CarriedResource, RobotKind, RobotSnapshot};
pub use scout::ScoutRobot;
