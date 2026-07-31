//! Zone (slot `[38]`: `pack.zones` JSON). Custom color overlay over a set of cells.

/// A custom zone (e.g. "region at war", "private hunting grounds", etc.).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Zone {
    pub id: u32,
    pub name: String,
    /// Overlay hex color.
    #[serde(default)]
    pub color: String,
    /// Cells that make up the zone (pack ids).
    #[serde(default)]
    pub cells: Vec<u32>,
    /// `"random" | "solid"` or other hatching pattern styles in Azgaar (keep opaque).
    #[serde(default)]
    pub style: Option<String>,
    /// Free-form description/lore for the UI.
    #[serde(default)]
    pub description: Option<String>,
}
