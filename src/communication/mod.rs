//! Module de communication.
//!
//! Dans cette refonte, la communication repose sur des messages typés.
//! Les robots ne modifient pas directement l'état global :
//! ils envoient des messages au hub de simulation.
//!
//! Cette séparation permet de défendre clairement une architecture concurrente :
//! - les robots tournent dans leurs propres threads ;
//! - le hub centralise l'état global ;
//! - l'UI reçoit des snapshots prêts à afficher.

pub mod message;

pub use message::{HubToRobot, KnownResource, RobotId, RobotToHub, SimulationCommand};
