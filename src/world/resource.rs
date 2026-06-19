//! Collectable resource types (energy / crystals).

use rand::Rng;

/// Kind of a resource on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    /// Energy source, drawn `E` (green).
    Energy,
    /// Crystal deposit, drawn `C` (magenta).
    Crystal,
}

/// Initial quantity bounds (inclusive), per the spec: 50 to 200 units.
pub const RESOURCE_QTY_MIN: u32 = 50;
pub const RESOURCE_QTY_MAX: u32 = 200;

/// A resource on the map with its remaining stock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resource {
    pub kind: ResourceKind,
    pub quantity: u32,
}

impl Resource {
    pub fn new(kind: ResourceKind, quantity: u32) -> Self {
        Self { kind, quantity }
    }

    /// Resource with a random quantity in `[RESOURCE_QTY_MIN; RESOURCE_QTY_MAX]`.
    pub fn random<R: Rng>(rng: &mut R, kind: ResourceKind) -> Self {
        let q = rng.gen_range(RESOURCE_QTY_MIN..=RESOURCE_QTY_MAX);
        Self::new(kind, q)
    }
}
