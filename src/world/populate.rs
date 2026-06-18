//! Placement des ressources Energy / Crystal.
//!
//! Correction importante :
//! les ressources ne sont plus placées sur n'importe quelle case vide.
//! Elles sont placées uniquement sur des cases accessibles depuis la base.
//!
//! Pourquoi ?
//! Dans une carte générée avec du bruit de Perlin, certaines zones peuvent être
//! isolées par des obstacles. Si une ressource apparaît dans une zone isolée,
//! les scouts peuvent parfois la voir, mais les collectors ne peuvent pas
//! forcément l'atteindre.
//!
//! Pour une simulation fiable et démontrable, on garantit donc que les ressources
//! sont placées sur des cases atteignables depuis la base.

use std::collections::{HashSet, VecDeque};

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use crate::utils::Position;
use crate::world::map::Map;
use crate::world::resource::{Resource, ResourceKind};
use crate::world::tile::Tile;

/// Paramètres de peuplement.
#[derive(Debug, Clone, Copy)]
pub struct PopulateParams {
    pub energy_count: usize,
    pub crystal_count: usize,

    /// Conservé pour compatibilité avec l'ancienne version.
    ///
    /// Dans cette nouvelle version, on ne fait plus des tentatives aléatoires
    /// jusqu'à tomber sur une bonne case : on calcule d'abord les cases
    /// atteignables, puis on pioche dedans.
    pub max_attempts_per_resource: usize,
}

impl Default for PopulateParams {
    fn default() -> Self {
        Self {
            energy_count: 12,
            crystal_count: 8,
            max_attempts_per_resource: 200,
        }
    }
}

impl Map {
    /// Peuple la carte avec des ressources atteignables depuis la base.
    ///
    /// Renvoie le nombre total de ressources effectivement posées.
    pub fn populate_resources(&mut self, seed: u64) -> usize {
        self.populate_resources_with(seed, PopulateParams::default())
    }

    /// Variante paramétrable de [`Map::populate_resources`].
    pub fn populate_resources_with(&mut self, seed: u64, params: PopulateParams) -> usize {
        let mut rng = StdRng::seed_from_u64(seed);

        // On lit volontairement ce champ pour éviter un warning,
        // même si la nouvelle logique n'en a plus besoin.
        let _max_attempts = params.max_attempts_per_resource;

        // On calcule toutes les cases vides atteignables depuis la base.
        // C'est LA différence essentielle avec l'ancienne version.
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

    /// Calcule les cases vides accessibles depuis la base avec un parcours BFS.
    ///
    /// BFS = Breadth-First Search.
    /// On explore progressivement les cases franchissables autour de la base.
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

    /// Place un certain type de ressource dans la liste de positions atteignables.
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

            // Double sécurité : on ne place que sur une case encore vide.
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
