//! Procedural [`Map`] generation from Perlin noise.
//!
//! For each cell we sample Perlin: above a threshold it becomes an obstacle.
//! The base and its immediate neighbours always stay clear (safe spawn).

use noise::{NoiseFn, Perlin};

use crate::utils::Position;
use crate::world::map::Map;
use crate::world::tile::Tile;

/// Generation parameters (defaults give ~25% obstacles in natural clusters).
#[derive(Debug, Clone, Copy)]
pub struct GenParams {
    /// Noise sampling frequency (higher = more fragmented obstacles).
    pub frequency: f64,
    /// Threshold above which a cell becomes an obstacle (-1.0..1.0).
    pub obstacle_threshold: f64,
    /// Radius (in cells) kept clear around the base.
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
    /// Generates a `width x height` map with Perlin obstacles for `seed`.
    /// Resources are placed afterwards by [`Map::populate_resources`].
    pub fn generate(width: usize, height: usize, seed: u32) -> Self {
        Self::generate_with(width, height, seed, GenParams::default())
    }

    /// Parameterised variant of [`Map::generate`].
    pub fn generate_with(width: usize, height: usize, seed: u32, params: GenParams) -> Self {
        let mut map = Map::empty(width, height);
        let perlin = Perlin::new(seed);
        let base = map.base();

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

/// True if `p` is inside the square of side `2*radius` around `base`.
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

        // Every cell of the 5x5 square around the base (radius 2) must be free.
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
