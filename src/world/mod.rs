//! `world` module.
//!
//! Everything that describes the simulation map: procedural generation,
//! resource placement, tiles, resources and the `Map` struct.

pub mod generator;
pub mod map;
pub mod populate;
pub mod resource;
pub mod tile;

pub use map::Map;
pub use resource::ResourceKind;
pub use tile::Tile;
