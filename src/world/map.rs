//! `Map`: 2D grid of [`Tile`] with bounds-checked access.

use crate::utils::Position;
use crate::world::resource::ResourceKind;
use crate::world::tile::Tile;

/// World map, a `width x height` grid stored row-major (`tiles[y * width + x]`).
#[derive(Debug, Clone)]
pub struct Map {
    width: usize,
    height: usize,
    tiles: Vec<Tile>,
    base: Position,
}

impl Map {
    /// Empty map (all walkable) with the base at the center.
    pub fn empty(width: usize, height: usize) -> Self {
        assert!(width > 0 && height > 0, "dimensions de carte invalides");
        let mut tiles = vec![Tile::Empty; width * height];
        let base = Position::new((width / 2) as i32, (height / 2) as i32);
        let idx = base.y as usize * width + base.x as usize;
        tiles[idx] = Tile::Base;
        Self {
            width,
            height,
            tiles,
            base,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn base(&self) -> Position {
        self.base
    }

    /// True if `p` is inside the grid.
    pub fn in_bounds(&self, p: Position) -> bool {
        p.x >= 0 && p.y >= 0 && (p.x as usize) < self.width && (p.y as usize) < self.height
    }

    fn index(&self, p: Position) -> usize {
        debug_assert!(self.in_bounds(p));
        p.y as usize * self.width + p.x as usize
    }

    /// Read access (None if out of bounds).
    pub fn get(&self, p: Position) -> Option<&Tile> {
        if self.in_bounds(p) {
            Some(&self.tiles[self.index(p)])
        } else {
            None
        }
    }

    /// Writes a tile (panics if out of bounds — controlled internal use).
    pub(crate) fn set(&mut self, p: Position, tile: Tile) {
        let i = self.index(p);
        self.tiles[i] = tile;
    }

    /// Removes one unit from the resource at `position`.
    ///
    /// Returns `(kind, remaining)` after the take, or `None` if there is no
    /// resource there. The tile becomes `Empty` once the deposit is exhausted.
    /// This mutation is why the map is shared behind a `Mutex` in the engine.
    pub fn take_resource_unit(&mut self, position: Position) -> Option<(ResourceKind, u32)> {
        match self.get(position).copied()? {
            Tile::Resource(mut resource) if resource.quantity > 0 => {
                resource.quantity -= 1;
                let result = (resource.kind, resource.quantity);
                let tile = if resource.quantity == 0 {
                    Tile::Empty
                } else {
                    Tile::Resource(resource)
                };
                self.set(position, tile);
                Some(result)
            }
            _ => None,
        }
    }

    /// True if `p` is walkable.
    pub fn is_walkable(&self, p: Position) -> bool {
        self.get(p).map(Tile::is_walkable).unwrap_or(false)
    }

    /// Iterates over every tile with its position.
    pub fn iter(&self) -> impl Iterator<Item = (Position, &Tile)> {
        let w = self.width;
        self.tiles
            .iter()
            .enumerate()
            .map(move |(i, t)| (Position::new((i % w) as i32, (i / w) as i32), t))
    }

    /// Counts obstacles on the map.
    #[allow(dead_code)]
    pub fn count_obstacles(&self) -> usize {
        self.tiles
            .iter()
            .filter(|t| matches!(t, Tile::Obstacle))
            .count()
    }

    /// Counts resource deposits of a given kind.
    #[allow(dead_code)]
    pub fn count_resources(&self, kind: ResourceKind) -> usize {
        self.tiles
            .iter()
            .filter(|t| matches!(t, Tile::Resource(r) if r.kind == kind))
            .count()
    }

    /// Sums the remaining units across all deposits of a given kind.
    #[allow(dead_code)]
    pub fn sum_resource_quantity(&self, kind: ResourceKind) -> u32 {
        self.tiles
            .iter()
            .filter_map(|tile| match tile {
                Tile::Resource(resource) if resource.kind == kind => Some(resource.quantity),
                _ => None,
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::world::resource::Resource;

    #[test]
    fn empty_map_has_base_at_center() {
        let map = Map::empty(10, 6);
        assert_eq!(map.base(), Position::new(5, 3));
        assert!(matches!(map.get(map.base()), Some(Tile::Base)));
    }

    #[test]
    fn out_of_bounds_returns_none() {
        let map = Map::empty(4, 4);
        assert!(map.get(Position::new(-1, 0)).is_none());
        assert!(map.get(Position::new(4, 0)).is_none());
    }

    #[test]
    fn sum_resource_quantity_adds_remaining_units() {
        let mut map = Map::empty(6, 6);

        map.set(
            Position::new(1, 1),
            Tile::Resource(Resource::new(ResourceKind::Energy, 50)),
        );

        map.set(
            Position::new(2, 1),
            Tile::Resource(Resource::new(ResourceKind::Energy, 120)),
        );

        map.set(
            Position::new(3, 1),
            Tile::Resource(Resource::new(ResourceKind::Crystal, 80)),
        );

        assert_eq!(map.sum_resource_quantity(ResourceKind::Energy), 170);
        assert_eq!(map.sum_resource_quantity(ResourceKind::Crystal), 80);
    }
}
