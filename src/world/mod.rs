//! Module `world` — carte, tuiles et ressources.
//!
//! Ce module est la responsabilité de Dev 1. Il expose :
//! - [`Tile`] : nature d'une case (vide, obstacle, base, ressource)
//! - [`Resource`] / [`ResourceKind`] : ressources collectables

pub mod resource;
pub mod tile;
pub mod map;
pub mod generator;

pub use generator::GenParams;
pub use map::Map;
pub use resource::{Resource, ResourceKind};
pub use tile::Tile;
