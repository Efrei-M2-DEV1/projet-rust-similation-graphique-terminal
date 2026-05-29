//! Utilitaires partagés : position dans la grille, directions cardinales,
//! petits helpers géométriques.

use std::ops::Add;

/// Coordonnée 2D dans la grille de la carte.
///
/// Les coordonnées sont des `i32` pour autoriser les calculs de delta
/// (déplacements négatifs) sans cast ; les bornes sont vérifiées par
/// la `Map` au moment de l'accès.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Distance de Manhattan — heuristique standard pour A* sur grille
    /// 4-connexe (utilisée par le module `pathfinding`).
    pub fn manhattan(self, other: Self) -> u32 {
        (self.x - other.x).unsigned_abs() + (self.y - other.y).unsigned_abs()
    }

    /// Renvoie les 4 voisins cardinaux (sans filtrage de bornes).
    pub fn neighbors4(self) -> [Position; 4] {
        [
            self + Direction::North.delta(),
            self + Direction::East.delta(),
            self + Direction::South.delta(),
            self + Direction::West.delta(),
        ]
    }
}

impl Add<(i32, i32)> for Position {
    type Output = Position;
    fn add(self, (dx, dy): (i32, i32)) -> Position {
        Position::new(self.x + dx, self.y + dy)
    }
}

/// Directions cardinales utilisées par les robots pour se déplacer
/// d'une case à la fois.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    /// Vecteur de déplacement associé.
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::East => (1, 0),
            Direction::South => (0, 1),
            Direction::West => (-1, 0),
        }
    }

    pub const ALL: [Direction; 4] = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manhattan_distance() {
        let a = Position::new(0, 0);
        let b = Position::new(3, 4);
        assert_eq!(a.manhattan(b), 7);
    }

    #[test]
    fn neighbors_are_adjacent() {
        let p = Position::new(5, 5);
        for n in p.neighbors4() {
            assert_eq!(p.manhattan(n), 1);
        }
    }
}
