//! Tuile élémentaire de la carte.

use super::resource::Resource;

/// État d'une case de la grille.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    /// Case libre, traversable.
    Empty,
    /// Obstacle infranchissable — affiché `O` (cyan clair).
    Obstacle,
    /// Base centrale — affichée `#` (vert clair).
    Base,
    /// Ressource collectable posée sur la case.
    Resource(Resource),
}

impl Tile {
    /// Une case est-elle franchissable par un robot ?
    pub fn is_walkable(&self) -> bool {
        !matches!(self, Tile::Obstacle)
    }
    // #[allow(dead_code)]
    /// Caractère ASCII utilisé pour le rendu.
    pub fn glyph(&self) -> char {
        match self {
            Tile::Empty => '.',
            Tile::Obstacle => 'O',
            Tile::Base => '#',
            Tile::Resource(r) => r.kind.glyph(),
        }
    }
}

impl Default for Tile {
    fn default() -> Self {
        Tile::Empty
    }
}
