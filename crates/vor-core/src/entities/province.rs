//! Province (slot `[30]`: `pack.provinces` JSON). Subdivision of a state.

/// A province. Slot `[0]` is the placeholder.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Province {
    pub id: u16,
    pub name: String,
    /// State it belongs to.
    pub state: u16,
    /// Dominant culture.
    #[serde(default)]
    pub culture: u16,
    /// Province capital burg (`None` if it has none).
    #[serde(default)]
    pub capital: Option<u16>,
    /// Hex color.
    #[serde(default)]
    pub color: String,
    /// Central cell.
    #[serde(default)]
    pub center_cell: u32,
    /// "Visual center" of the polygon (pole of inaccessibility).
    #[serde(default)]
    pub pole_of_inaccessibility: [f32; 2],
    /// Burgs included in the province.
    #[serde(default)]
    pub burgs: Vec<u16>,
    /// Number of cells.
    #[serde(default)]
    pub cells: u32,
    /// Area in pixels².
    #[serde(default)]
    pub area_px: u32,
    /// Rural population (points).
    #[serde(default)]
    pub rural_pop: f32,
    /// Urban population.
    #[serde(default)]
    pub urban_pop: f32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub removed: bool,
}
