//! Génération procédurale d'une [`Map`] à partir d'un bruit de Perlin.
//!
//! Approche :
//! 1. On évalue Perlin sur chaque case (x, y) avec une fréquence donnée.
//! 2. Au-dessus d'un seuil → obstacle, sinon case libre.
//! 3. La base centrale et ses voisines sont toujours libres (spawn safe).

use noise::{NoiseFn, Perlin};

use crate::utils::Position;
use crate::world::map::Map;
use crate::world::tile::Tile;

/// Paramètres de génération (valeurs par défaut raisonnables pour
/// obtenir ~25 % d'obstacles avec des amas naturels).
#[derive(Debug, Clone, Copy)]
pub struct GenParams {
    /// Fréquence d'échantillonnage du bruit (plus haut = obstacles plus
    /// fragmentés ; plus bas = grandes nappes).
    pub frequency: f64,
    /// Seuil au-dessus duquel le bruit devient un obstacle (-1.0..1.0).
    pub obstacle_threshold: f64,
    /// Rayon (en cases) autour de la base à garder dégagé.
    pub safe_radius: i32,
}

impl Default for GenParams {
    fn default() -> Self {
        Self {
            frequency: 0.12,
            obstacle_threshold: 0.20,
            safe_radius: 2,
        }
    }
}

impl Map {
    /// Génère une carte `width x height` avec obstacles Perlin pour la
    /// graine `seed`. Les ressources seront placées dans un second temps
    /// par [`Map::populate_resources`] (commit suivant).
    pub fn generate(width: usize, height: usize, seed: u32) -> Self {
        Self::generate_with(width, height, seed, GenParams::default())
    }

    /// Variante paramétrable de [`Map::generate`].
    pub fn generate_with(width: usize, height: usize, seed: u32, params: GenParams) -> Self {
        let mut map = Map::empty(width, height);
        let perlin = Perlin::new(seed);
        let base = map.base();

        // Double boucle : on parcourt chaque case (x, y) de la grille,
        // on évalue le bruit de Perlin à cette position et on transforme
        // la case en obstacle si la valeur dépasse le seuil.
        // La zone autour de la base reste toujours libre (spawn safe).
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let p = Position::new(x, y);

                if is_in_safe_zone(p, base, params.safe_radius) {
                    continue;
                }

                let noise_value =
                    perlin.get([x as f64 * params.frequency, y as f64 * params.frequency]);

                if noise_value > params.obstacle_threshold {
                    map.set(p, Tile::Obstacle);
                }
            }
        }

        map
    }
}

/// Vrai si `p` est dans le carré de rayon `radius` autour de `base`.
fn is_in_safe_zone(p: Position, base: Position, radius: i32) -> bool {
    (p.x - base.x).abs() <= radius && (p.y - base.y).abs() <= radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_and_safe_zone_are_walkable() {
        let map = Map::generate(40, 20, 7);
        let b = map.base();

        // Double boucle : on vérifie chaque case du carré 5x5 centré
        // sur la base (rayon 2 par défaut) — toutes doivent être libres.
        for dy in -2..=2 {
            for dx in -2..=2 {
                let p = Position::new(b.x + dx, b.y + dy);
                assert!(map.is_walkable(p), "case {:?} doit être libre", p);
            }
        }
    }

    #[test]
    fn deterministic_with_same_seed() {
        let a = Map::generate(30, 15, 123);
        let b = Map::generate(30, 15, 123);
        assert_eq!(a.count_obstacles(), b.count_obstacles());
    }

    #[test]
    fn produces_some_obstacles() {
        let map = Map::generate(60, 30, 42);
        assert!(map.count_obstacles() > 0);
    }
}
