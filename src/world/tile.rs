//! Single map cell.

use super::resource::Resource;

/// State of a grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tile {
    /// Free, walkable cell.
    #[default]
    Empty,
    /// Impassable obstacle, drawn `O` (cyan).
    Obstacle,
    /// Central base, drawn `#` (green).
    Base,
    /// Collectable resource sitting on the cell.
    Resource(Resource),
}

impl Tile {
    /// Can a robot walk over this cell?
    pub fn is_walkable(&self) -> bool {
        !matches!(self, Tile::Obstacle)
    }
}
