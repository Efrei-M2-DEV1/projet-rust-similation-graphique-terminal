//! Navigation A* sur la grille.
//!
//! Le module expose une petite API métier autour de la crate
//! `pathfinding`: les robots demandent un chemin de case en case, sans
//! connaître les détails de l'algorithme.

use std::collections::HashSet;

use pathfinding_crate::prelude::astar;

use crate::utils::Position;
use crate::world::Map;

/// Calcule un chemin A* entre `start` et `goal`.
///
/// Le chemin renvoyé contient toujours `start` en première position et
/// `goal` en dernière position. Les obstacles de la carte sont exclus.
pub fn find_path(map: &Map, start: Position, goal: Position) -> Option<Vec<Position>> {
    find_path_avoiding(map, start, goal, &HashSet::new())
}

/// Variante de [`find_path`] qui évite aussi des positions dynamiques,
/// typiquement les cases déjà occupées par d'autres robots.
pub fn find_path_avoiding(
    map: &Map,
    start: Position,
    goal: Position,
    blocked: &HashSet<Position>,
) -> Option<Vec<Position>> {
    if !map.in_bounds(start)
        || !map.in_bounds(goal)
        || !map.is_walkable(start)
        || !map.is_walkable(goal)
    {
        return None;
    }

    if start == goal {
        return Some(vec![start]);
    }

    astar(
        &start,
        |position| -> Vec<(Position, u32)> {
            position
                .neighbors4()
                .into_iter()
                .filter(|next| map.is_walkable(*next))
                .filter(|next| *next == goal || !blocked.contains(next))
                .map(|next| (next, 1_u32))
                .collect()
        },
        |position| position.manhattan(goal),
        |position| *position == goal,
    )
    .map(|(path, _cost)| path)
}

/// Renvoie uniquement la prochaine case à emprunter vers `goal`.
pub fn next_step_avoiding(
    map: &Map,
    start: Position,
    goal: Position,
    blocked: &HashSet<Position>,
) -> Option<Position> {
    let path = find_path_avoiding(map, start, goal, blocked)?;
    path.get(1).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Tile;

    #[test]
    fn path_starts_and_ends_at_requested_positions() {
        let map = Map::empty(8, 8);
        let start = Position::new(1, 1);
        let goal = Position::new(6, 5);

        let path = find_path(&map, start, goal).expect("path should exist");

        assert_eq!(path.first().copied(), Some(start));
        assert_eq!(path.last().copied(), Some(goal));
    }

    #[test]
    fn path_avoids_obstacles() {
        let mut map = Map::empty(7, 5);
        map.set(Position::new(2, 1), Tile::Obstacle);
        map.set(Position::new(2, 2), Tile::Obstacle);
        map.set(Position::new(2, 3), Tile::Obstacle);

        let path = find_path(&map, Position::new(1, 2), Position::new(5, 2))
            .expect("path should go around wall");

        assert!(!path.contains(&Position::new(2, 2)));
    }

    #[test]
    fn dynamic_blocked_positions_are_avoided() {
        let map = Map::empty(5, 5);
        let mut blocked = HashSet::new();
        blocked.insert(Position::new(2, 1));

        let next = next_step_avoiding(
            &map,
            Position::new(1, 1),
            Position::new(3, 1),
            &blocked,
        )
        .expect("alternate path should exist");

        assert_ne!(next, Position::new(2, 1));
    }
}
