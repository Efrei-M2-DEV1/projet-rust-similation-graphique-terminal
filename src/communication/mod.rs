//! Communication module.
//!
//! Communication relies on typed messages. Robots never mutate the global
//! state directly: they send messages to the simulation hub.
//!
//! This separation makes the concurrent architecture easy to reason about:
//! - robots run in their own threads;
//! - the hub centralizes the global state;
//! - the UI receives ready-to-render snapshots.

pub mod message;

pub use message::{HubToRobot, KnownResource, RobotId, RobotToHub, SimulationCommand};
