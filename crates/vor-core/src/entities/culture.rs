//! Culture (slot `[13]`: `pack.cultures` JSON `[{"name":"Wildlands","i":0,...}, ...]`).

use super::coat_of_arms::CoatOfArms;

/// Culture type. Variants confirmed against Azgaar's "Culture types" wiki (Phase 0 §4.3);
/// add any missing ones per the final confirmation without breaking old migrations.
// TODO Phase 1: confirm exact variants and the code name Azgaar uses in the JSON
// (e.g. "Generic"/"River"/"Lake"/"Naval"/"Nomadic"/"Hunting"/"Highland" — see fase-0 §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CultureType {
    #[default]
    Generic,
    River,
    Lake,
    Naval,
    Nomadic,
    Hunting,
    Highland,
}

/// A culture (entry of `pack.cultures`).
///
/// Azgaar's item `[0]` is the "Wildlands" placeholder (unassigned culture).
/// Voronia keeps it at `pack.cultures[0]` to preserve the 1:1 mapping with ids.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Culture {
    /// Id (its index in `pack.cultures`; 0 = Wildlands).
    pub id: u16,
    /// Culture name ("Wildlands" in slot 0; then "English", "German", etc.).
    pub name: String,
    /// Id of the associated namebase (index in `namesData`, slot `[31]`).
    pub namebase_id: u16,
    /// Origin cultures (ids), for the evolutionary tree.
    #[serde(default)]
    pub origins: Vec<u16>,
    /// Shield (compatible with Watabou's Armoria).
    #[serde(default)]
    pub shield: CoatOfArms,
    /// Central cell of the culture.
    pub center_cell: u32,
    /// Abbreviation/code (e.g. "ENG" for English).
    #[serde(default)]
    pub code: String,
    /// Hex color.
    #[serde(default)]
    pub color: String,
    /// Expansion multiplier.
    #[serde(default)]
    pub expansionism: f32,
    /// Culture type.
    pub kind: CultureType,
    /// Total area in pixels² (populated during simulation).
    #[serde(default)]
    pub area_px: u32,
    /// Number of cells under the culture.
    #[serde(default)]
    pub cells: u32,
    /// Rural population (in "population points", `f32`).
    #[serde(default)]
    pub rural_pop: f32,
    /// Urban population.
    #[serde(default)]
    pub urban_pop: f32,
    /// `true` if the user marked it as locked (not re-generatable).
    #[serde(default)]
    pub locked: bool,
    /// `true` if the culture was removed manually (soft delete — Azgaar keeps the id free).
    #[serde(default)]
    pub removed: bool,
}
