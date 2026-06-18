//! Placement aléatoire des ressources (Energy / Crystal) sur une `Map`
//! déjà générée. Le tirage utilise une graine pour la reproductibilité.

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
    /// Nombre maximal d'essais de placement avant abandon (évite les
    /// boucles infinies si la carte est presque pleine).
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
    /// Peuple la carte avec des ressources aléatoires (cases libres
    /// uniquement, hors base). Renvoie le nombre total de ressources
    /// effectivement posées.
    pub fn populate_resources(&mut self, seed: u64) -> usize {
        self.populate_resources_with(seed, PopulateParams::default())
    }

    /// Variante paramétrable de [`Map::populate_resources`].
    pub fn populate_resources_with(&mut self, seed: u64, params: PopulateParams) -> usize {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut placed = 0;

        placed += self.place_kind(
            &mut rng,
            ResourceKind::Energy,
            params.energy_count,
            params.max_attempts_per_resource,
        );
        placed += self.place_kind(
            &mut rng,
            ResourceKind::Crystal,
            params.crystal_count,
            params.max_attempts_per_resource,
        );

        placed
    }

    fn place_kind<R: Rng>(
        &mut self,
        rng: &mut R,
        kind: ResourceKind,
        count: usize,
        max_attempts: usize,
    ) -> usize {
        let mut placed = 0;

        // Boucle externe : on essaie de poser `count` ressources de ce type.
        // Boucle interne : pour chaque ressource on tire des positions
        // aléatoires jusqu'à en trouver une vide (max `max_attempts`
        // essais pour éviter de boucler à l'infini si la carte est pleine).
        for _ in 0..count {
            for _ in 0..max_attempts {
                let x = rng.gen_range(0..self.width() as i32);
                let y = rng.gen_range(0..self.height() as i32);
                let p = Position::new(x, y);

                if matches!(self.get(p), Some(Tile::Empty)) {
                    let res = Resource::random(rng, kind);
                    self.set(p, Tile::Resource(res));
                    placed += 1;
                    break;
                }
            }
        }

        placed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
