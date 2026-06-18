//! Module `world`.
//!
//! Il regroupe tout ce qui décrit la carte de simulation :
//! - la génération procédurale ;
//! - le placement des ressources ;
//! - les tuiles ;
//! - les ressources ;
//! - la structure Map.
//!
//! On ne réexporte ici que les types réellement utilisés par le reste du projet,
//! afin d'éviter les imports inutilisés et de garder une API claire.

pub mod generator;
pub mod map;
pub mod populate;
pub mod resource;
pub mod tile;

pub use map::Map;
pub use resource::ResourceKind;
pub use tile::Tile;
