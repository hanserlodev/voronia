//! Religion (slot `[29]`: `pack.religions` JSON).

/// Religion type. Variants confirmed in Azgaar's model (plan §7.7).
// TODO Phase 1: confirm the exact `Organized` name (Azgaar uses "Organized" or perhaps
// "Organized Religion" — lock the exact one).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReligionType {
    #[default]
    Folk,
    Organized,
    Heresy,
    Cult,
}

/// Religion expansion mode/scope (plan §7.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReligionExpansion {
    /// Only within its culture of origin.
    #[default]
    Culture,
    /// Global — any culture.
    Global,
}

/// A religion (entry of `pack.religions`). Slot `[0]` is the placeholder.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Religion {
    pub id: u16,
    pub name: String,
    /// Origin cultures/religions (evolutionary tree).
    #[serde(default)]
    pub origins: Vec<u16>,
    /// Hex color.
    #[serde(default)]
    pub color: String,
    /// Religion type.
    pub kind: ReligionType,
    /// Expansion mode.
    #[serde(default)]
    pub expansion: ReligionExpansion,
    /// Central cell.
    #[serde(default)]
    pub center_cell: u32,
    /// Number of cells.
    #[serde(default)]
    pub cells: u32,
    /// Total area in pixels².
    #[serde(default)]
    pub area_px: u32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub removed: bool,
}
