//! Messages exchanged between robots, the hub and the UI.
//!
//! Messages are split on purpose:
//! - RobotToHub: messages sent by a robot to the hub.
//! - HubToRobot: messages sent by the hub to a robot.
//! - SimulationCommand: commands sent by the UI to the simulation.

use crate::utils::Position;
use crate::world::ResourceKind;

/// Unique identifier of a robot (e.g. R0 a scout, R4 a collector).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RobotId(pub usize);

/// A resource known by the hub or by a robot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownResource {
    pub position: Position,
    pub kind: ResourceKind,
    pub quantity: u32,
}

/// Messages sent by robots to the hub.
///
/// The hub owns the single official global state: robots request or report,
/// the hub decides.
#[derive(Debug, Clone)]
pub enum RobotToHub {
    /// A robot discovered a resource.
    ResourceDiscovered {
        robot_id: RobotId,
        position: Position,
        kind: ResourceKind,
        quantity: u32,
    },

    /// A robot discovered an obstacle.
    ObstacleDiscovered {
        robot_id: RobotId,
        position: Position,
    },

    /// A robot requests a move.
    ///
    /// The robot never moves on its own: it proposes a move and the hub
    /// accepts or rejects it.
    MoveRequested {
        robot_id: RobotId,
        from: Position,
        to: Position,
    },

    /// A collector requests to collect one unit at a position.
    CollectRequested {
        robot_id: RobotId,
        position: Position,
    },

    /// A collector deposits its cargo at the base.
    Deposit {
        robot_id: RobotId,
        kind: ResourceKind,
        amount: u32,
    },

    /// Free-form message for the event log.
    Log { robot_id: RobotId, text: String },
}

/// Messages sent by the hub to robots.
#[derive(Debug, Clone)]
pub enum HubToRobot {
    /// The hub signals a new tick; robots act in reaction to it.
    Tick(u64),

    /// The hub shares the aggregated global knowledge. Robots keep a local
    /// knowledge but regularly receive the merged discoveries.
    Knowledge {
        resources: Vec<KnownResource>,
        obstacles: Vec<Position>,
    },

    /// The hub accepts the requested move.
    MoveGranted { to: Position },

    /// The hub rejects the requested move.
    MoveDenied { attempted: Position },

    /// The hub allows collecting one unit.
    CollectGranted {
        position: Position,
        kind: ResourceKind,
        remaining: u32,
    },

    /// The hub denies the collect.
    CollectDenied { position: Position },

    /// Request a clean shutdown of the robot.
    Shutdown,
}

/// Commands sent by the UI to the simulation.
#[derive(Debug, Clone)]
pub enum SimulationCommand {
    Shutdown,
}
