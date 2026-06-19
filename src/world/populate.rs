//! Energy / Crystal resource placement.
//!
//! Resources are only placed on cells reachable from the base (BFS). A Perlin
//! map can have pockets sealed by obstacles; placing a resource there would make
//! it impossible for collectors to reach, so we restrict placement to the
//! reachable region.

use std::collections::{HashSet, VecDeque};

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use crate::utils::Position;
use crate::world::map::Map;
use crate::world::resource::{Resource, ResourceKind};
use crate::world::tile::Tile;

/// Placement parameters.
#[derive(Debug, Clone, Copy)]
pub struct PopulateParams {
    pub energy_count: usize,
    pub crystal_count: usize,
}

impl Default for PopulateParams {
    fn default() -> Self {
        Self {
            energy_count: 12,
            crystal_count: 8,
        }
    }
}

impl Map {
    /// Populates the map with resources reachable from the base.
    /// Returns the number of resources actually placed.
    pub fn populate_resources(&mut self, seed: u64) -> usize {
        self.populate_resources_with(seed, PopulateParams::default())
    }

    /// Parameterised variant of [`Map::populate_resources`].
    pub fn populate_resources_with(&mut self, seed: u64, params: PopulateParams) -> usize {
        let mut rng = StdRng::seed_from_u64(seed);

        let mut candidates = self.reachable_empty_positions_from_base();

        let mut placed = 0;

        placed += self.place_kind_from_candidates(
            &mut rng,
            &mut candidates,
            ResourceKind::Energy,
            params.energy_count,
        );

        placed += self.place_kind_from_candidates(
            &mut rng,
            &mut candidates,
            ResourceKind::Crystal,
            params.crystal_count,
        );

        placed
    }

    /// Empty cells reachable from the base, found with a breadth-first search.
    fn reachable_empty_positions_from_base(&self) -> Vec<Position> {
        let mut visited = HashSet::<Position>::new();
        let mut queue = VecDeque::<Position>::new();
        let mut reachable_empty = Vec::<Position>::new();

        let base = self.base();

        visited.insert(base);
        queue.push_back(base);

        while let Some(current) = queue.pop_front() {
            if matches!(self.get(current), Some(Tile::Empty)) {
                reachable_empty.push(current);
            }

            for neighbor in current.neighbors4() {
                if !self.in_bounds(neighbor) {
                    continue;
                }

                if visited.contains(&neighbor) {
                    continue;
                }

                if !self.is_walkable(neighbor) {
                    continue;
                }

                visited.insert(neighbor);
                queue.push_back(neighbor);
            }
        }

        reachable_empty
    }

    /// Places `count` resources of one kind among the reachable candidates.
    fn place_kind_from_candidates<R: Rng>(
        &mut self,
        rng: &mut R,
        candidates: &mut Vec<Position>,
        kind: ResourceKind,
        count: usize,
    ) -> usize {
        let mut placed = 0;

        for _ in 0..count {
            if candidates.is_empty() {
                break;
            }

            let index = rng.gen_range(0..candidates.len());
            let position = candidates.swap_remove(index);

            // Only place on a still-empty cell.
            if !matches!(self.get(position), Some(Tile::Empty)) {
                continue;
            }

            let resource = Resource::random(rng, kind);
            self.set(position, Tile::Resource(resource));
            placed += 1;
        }

        placed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pathfinding::find_path;

    #[test]
    fn populate_places_resources() {
        let mut map = Map::generate(50, 25, 1);
        let n = map.populate_resources(99);

        assert!(n > 0);
        assert!(map.count_resources(ResourceKind::Energy) > 0);
        assert!(map.count_resources(ResourceKind::Crystal) > 0);
    }

    #[test]
    fn populate_is_deterministic() {
        let mut a = Map::generate(40, 20, 5);
        let mut b = Map::generate(40, 20, 5);

        a.populate_resources(77);
        b.populate_resources(77);

        assert_eq!(
            a.count_resources(ResourceKind::Energy),
            b.count_resources(ResourceKind::Energy)
        );
        assert_eq!(
            a.count_resources(ResourceKind::Crystal),
            b.count_resources(ResourceKind::Crystal)
        );
    }

    #[test]
    fn populated_resources_are_reachable_from_base() {
        let mut map = Map::generate(60, 25, 42);
        map.populate_resources(7);

        for (position, tile) in map.iter() {
            if matches!(tile, Tile::Resource(_)) {
                assert!(
                    find_path(&map, map.base(), position).is_some(),
                    "resource at {:?} should be reachable from base",
                    position
                );
            }
        }
    }
}
