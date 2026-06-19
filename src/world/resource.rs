//! Types de ressources collectables (énergie / cristaux).

use rand::Rng;

/// Type d'une ressource présente sur la carte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    /// Source d'énergie — affichée `E` (vert).
    Energy,
    /// Gisement de cristal — affiché `C` (magenta clair).
    Crystal,
}

impl ResourceKind {
    /// Caractère utilisé pour le rendu terminal.
    #[allow(dead_code)]
    pub const fn glyph(self) -> char {
        match self {
            ResourceKind::Energy => 'E',
            ResourceKind::Crystal => 'C',
        }
    }
}

/// Bornes (incluses) de la quantité initiale d'une ressource, selon le
/// cahier des charges (50 à 200 unités).
pub const RESOURCE_QTY_MIN: u32 = 50;
pub const RESOURCE_QTY_MAX: u32 = 200;

/// Une ressource posée sur la carte, avec son stock restant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resource {
    pub kind: ResourceKind,
    pub quantity: u32,
}

impl Resource {
    pub fn new(kind: ResourceKind, quantity: u32) -> Self {
        Self { kind, quantity }
    }

    /// Crée une ressource avec une quantité aléatoire dans les bornes
    /// `[RESOURCE_QTY_MIN; RESOURCE_QTY_MAX]`.
    pub fn random<R: Rng>(rng: &mut R, kind: ResourceKind) -> Self {
        let q = rng.gen_range(RESOURCE_QTY_MIN..=RESOURCE_QTY_MAX);
        Self::new(kind, q)
    }

    /// Prélève une unité ; renvoie `true` si une unité a été retirée.
    #[allow(dead_code)]
    pub fn take_one(&mut self) -> bool {
        if self.quantity == 0 {
            false
        } else {
            self.quantity -= 1;
            true
        }
    }

    #[allow(dead_code)]
    pub fn is_depleted(&self) -> bool {
        self.quantity == 0
    }
}
