//! Structure `Map` : grille 2D de [`Tile`], accesseurs sûrs et requêtes
//! géométriques. La génération procédurale est ajoutée dans un commit
//! ultérieur (Perlin + placement des ressources).

use crate::utils::Position;
use crate::world::resource::ResourceKind;
use crate::world::tile::{self, Tile};

/// Carte du monde — grille rectangulaire `width x height` de [`Tile`].
///
/// Stockée en row-major : `tiles[y * width + x]`.
#[derive(Debug, Clone)]
pub struct Map {
    width: usize,
    height: usize,
    tiles: Vec<Tile>,
    base: Position,
}

impl Map {
    /// Crée une carte vide (toutes cases libres) de la taille demandée,
    /// avec une base placée au centre. Utilisée pour les tests et comme
    /// base pour la génération procédurale.
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

    /// Vérifie que la position est dans les bornes de la grille.
    pub fn in_bounds(&self, p: Position) -> bool {
        p.x >= 0 && p.y >= 0 && (p.x as usize) < self.width && (p.y as usize) < self.height
    }

    fn index(&self, p: Position) -> usize {
        debug_assert!(self.in_bounds(p));
        p.y as usize * self.width + p.x as usize
    }

    /// Accès en lecture (None si hors bornes).
    pub fn get(&self, p: Position) -> Option<&Tile> {
        if self.in_bounds(p) {
            Some(&self.tiles[self.index(p)])
        } else {
            None
        }
    }

    /// Accès en écriture (None si hors bornes).
    pub fn get_mut(&mut self, p: Position) -> Option<&mut Tile> {
        if self.in_bounds(p) {
            let i = self.index(p);
            Some(&mut self.tiles[i])
        } else {
            None
        }
    }

    /// Place une tuile (panique si hors bornes — usage interne contrôlé).
    pub(crate) fn set(&mut self, p: Position, tile: Tile) {
        let i = self.index(p);
        self.tiles[i] = tile;
    }

    /// Une position est-elle franchissable ?
    pub fn is_walkable(&self, p: Position) -> bool {
        self.get(p).map(Tile::is_walkable).unwrap_or(false)
    }

    /// Itère toutes les cases avec leur position.
    pub fn iter(&self) -> impl Iterator<Item = (Position, &Tile)> {
        let w = self.width;
        self.tiles
            .iter()
            .enumerate()
            .map(move |(i, t)| (Position::new((i % w) as i32, (i / w) as i32), t))
    }

    /// Compte les obstacles présents sur la carte.
    pub fn count_obstacles(&self) -> usize {
        self.tiles.iter().filter(|t| matches!(t, Tile::Obstacle)).count()
    }

    /// Compte les ressources d'un certain type.
    pub fn count_resources(&self, kind: ResourceKind) -> usize {
        self.tiles
            .iter()
            .filter(|t| matches!(t, Tile::Resource(r) if r.kind == kind))
            .count()
    }
   /// Additionne les quantités restantes pour un type de ressource.
///
/// Différence importante avec `count_resources` :
/// - `count_resources(ResourceKind::Energy)` compte le nombre de gisements E.
/// - `sum_resource_quantity(ResourceKind::Energy)` additionne les unités restantes.
///
/// Exemple :
/// Si la carte contient 3 sources d'énergie de 50, 100 et 120 unités,
/// cette fonction renvoie 270.
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

    use crate::world :: resource:: Resource;

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
