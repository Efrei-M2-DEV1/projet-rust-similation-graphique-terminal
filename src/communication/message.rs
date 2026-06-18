//! Messages échangés entre les robots, le hub et l'interface.
//!
//! On sépare volontairement les messages :
//! - RobotToHub : messages envoyés par un robot au hub.
//! - HubToRobot : messages envoyés par le hub à un robot.
//! - SimulationCommand : commandes envoyées par l'UI à la simulation.
//!
//! Cette séparation rend le code plus clair à expliquer en soutenance.

use crate::utils::Position;
use crate::world::ResourceKind;

/// Identifiant unique d'un robot.
///
/// Exemple :
/// - R0 peut être un scout.
/// - R4 peut être un collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RobotId(pub usize);

/// Ressource connue par le hub ou par un robot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownResource {
    pub position: Position,
    pub kind: ResourceKind,
    pub quantity: u32,
}

/// Messages envoyés par les robots vers le hub.
///
/// Le hub est le seul à posséder l'état global officiel.
/// Les robots demandent ou signalent ; le hub décide.
#[derive(Debug, Clone)]
pub enum RobotToHub {
    /// Un robot a découvert une ressource.
    ResourceDiscovered {
        robot_id: RobotId,
        position: Position,
        kind: ResourceKind,
        quantity: u32,
    },

    /// Un robot a découvert un obstacle.
    ObstacleDiscovered {
        robot_id: RobotId,
        position: Position,
    },

    /// Un robot demande à se déplacer.
    ///
    /// Important : le robot ne se déplace pas directement.
    /// Il propose un mouvement, et le hub accepte ou refuse.
    MoveRequested {
        robot_id: RobotId,
        from: Position,
        to: Position,
    },

    /// Un collecteur demande à collecter une unité sur une position.
    CollectRequested {
        robot_id: RobotId,
        position: Position,
    },

    /// Un collecteur dépose sa cargaison à la base.
    Deposit {
        robot_id: RobotId,
        kind: ResourceKind,
        amount: u32,
    },

    /// Message libre utile pour le journal d'événements.
    Log { robot_id: RobotId, text: String },
}

/// Messages envoyés par le hub vers les robots.
#[derive(Debug, Clone)]
pub enum HubToRobot {
    /// Le hub donne le signal d'un nouveau tick.
    ///
    /// Les robots agissent en réaction à ce tick.
    Tick(u64),

    /// Le hub partage l'état de connaissance global.
    ///
    /// Les robots gardent une connaissance locale,
    /// mais ils reçoivent régulièrement les découvertes agrégées.
    Knowledge {
        resources: Vec<KnownResource>,
        obstacles: Vec<Position>,
    },

    /// Le hub accepte le déplacement demandé.
    MoveGranted { to: Position },

    /// Le hub refuse le déplacement demandé.
    MoveDenied { attempted: Position },

    /// Le hub autorise la collecte d'une unité.
    CollectGranted {
        position: Position,
        kind: ResourceKind,
        remaining: u32,
    },

    /// Le hub refuse la collecte.
    CollectDenied { position: Position },

    /// Demande d'arrêt propre du robot.
    Shutdown,
}

/// Commandes envoyées par l'UI vers la simulation.
#[derive(Debug, Clone)]
pub enum SimulationCommand {
    Shutdown,
}
