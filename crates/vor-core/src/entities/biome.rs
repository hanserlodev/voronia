//! Biome: definition (slot `[3]` of the `.map` — per-cell ids go in `PackCells::biome`).

/// A biome from Azgaar's catalog. Slot `[3]` of the `.map` carries them pipe-delimited
/// as `color|habitability|name`, with the id implicit by order (0 = ocean, 1 = lowland, etc.).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Biome {
    /// Biome id (its position in Azgaar's catalog — 0-indexed).
    pub id: u8,
    /// Hex color (`#rrggbb`); Azgaar stores it as `rgb(R,G,B)`, which `vor-import` normalizes.
    pub color: String,
    /// Habitability (how suitable for settlements; used in burg scoring).
    pub habitability: f32,
    /// Movement cost per cell of this biome (used in culture/state expansion).
    pub move_cost: f32,
    /// Human-readable name. In Azgaar biome `0` is named `"Marine"` (ocean/coast), etc.
    pub name: String,
}
