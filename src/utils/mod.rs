//! Shared helpers: grid position, cardinal directions, small geometry.

use std::ops::Add;

/// 2D coordinate in the map grid.
///
/// Stored as `i32` so deltas can be negative without casts; bounds are checked
/// by the [`crate::world::Map`] on access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Manhattan distance — the A* heuristic on a 4-connected grid.
    pub fn manhattan(self, other: Self) -> u32 {
        (self.x - other.x).unsigned_abs() + (self.y - other.y).unsigned_abs()
    }

    /// The 4 cardinal neighbours (no bounds filtering).
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

/// Cardinal directions used by robots to move one cell at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    /// Associated movement vector.
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::East => (1, 0),
            Direction::South => (0, 1),
            Direction::West => (-1, 0),
        }
    }
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
