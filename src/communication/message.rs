use crate::utils::Position;
use crate::world::ResourceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RobotId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub sender: RobotId,
    pub payload: Message,
}

impl Envelope {
    pub fn new(sender: RobotId, payload: Message) -> Self {
        Self { sender, payload }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    ResourceFound {
        position: Position,
        kind: ResourceKind,
        quantity: u32,
    },
    ObstacleFound { position: Position },
    ResourceCollected {
        position: Position,
        kind: ResourceKind,
        amount: u32,
    },
    ResourceDepleted {
        position: Position,
        kind: ResourceKind,
    },
    Deposit { kind: ResourceKind, amount: u32 },
}

impl Message {
    pub fn resource_found(position: Position, kind: ResourceKind, quantity: u32) -> Self {
        Self::ResourceFound {
            position,
            kind,
            quantity,
        }
    }

    pub fn obstacle_found(position: Position) -> Self {
        Self::ObstacleFound { position }
    }
}
