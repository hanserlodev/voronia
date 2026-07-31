//! State / country (slot `[14]`: `pack.states` JSON).
//!
//! Some sub-objects (regiments, wars, diplomacy, military) belong to
//! Phase 7 (simulation). For Phase 1 we keep them as opaque `serde_json::Value`
//! to not lose the data when importing; they are unpacked later.

use super::coat_of_arms::CoatOfArms;
use super::culture::CultureType;

/// Form of government. Variants to be confirmed against the "Military Forces" wiki / forms.
// TODO Phase 1: confirm the exact variants (Monarchy/Republic/Theocracy/Union/Anarchy/Federation/...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GovernmentForm {
    Monarchy,
    Republic,
    Theocracy,
    Union,
    /// `Anarchy` = no formal state (matches the neutral placeholder `State::placeholder()`).
    #[default]
    Anarchy,
}

/// A state / country. Slot `[0]` of `pack.states` is the "neutral" placeholder.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct State {
    /// Id (its index in `pack.states`; 0 = neutral).
    pub id: u16,
    /// Name ("Tal Empire", "Kingdom of X", ...).
    pub name: String,
    /// Form of government.
    #[serde(default)]
    pub form: GovernmentForm,
    /// Full formal name ("The Holy Kingdom of Tal", ...).
    #[serde(default)]
    pub full_name: String,
    /// Hex color.
    #[serde(default)]
    pub color: String,
    /// Central cell.
    pub center_cell: u32,
    /// "Visual center" of the polygon (Mapbox's pole-of-inaccessibility technique).
    #[serde(default)]
    pub pole_of_inaccessibility: [f32; 2],
    /// State culture (links to Culture::id).
    pub culture: u16,
    /// Culture type (same enum as `Culture`).
    #[serde(default)]
    pub kind: CultureType,
    /// Political expansion multiplier.
    #[serde(default)]
    pub expansionism: f32,
    /// Total area in pixels².
    #[serde(default)]
    pub area_px: u32,
    /// Number of burgs.
    #[serde(default)]
    pub burg_count: u32,
    /// Number of cells.
    #[serde(default)]
    pub cell_count: u32,
    /// Rural population (points).
    #[serde(default)]
    pub rural_pop: f32,
    /// Urban population.
    #[serde(default)]
    pub urban_pop: f32,
    /// Neighboring states (ids).
    #[serde(default)]
    pub neighbors: Vec<u16>,
    /// State provinces (ids).
    #[serde(default)]
    pub provinces: Vec<u16>,
    /// Coat of arms.
    #[serde(default)]
    pub coat_of_arms: CoatOfArms,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub removed: bool,
    /// Diplomacy, wars/campaigns and military (`Regiment`/`War`) — belongs to Phase 7.
    /// Kept opaque when importing to avoid losing data; it will be unpacked in Phase 7.
    #[serde(default, with = "crate::serde_json_string")]
    pub diplomacy: serde_json::Value,
    #[serde(default, with = "crate::serde_json_string")]
    pub campaigns: serde_json::Value,
    #[serde(default, with = "crate::serde_json_string")]
    pub military: serde_json::Value,
}

impl State {
    /// Placeholder for slot `[0]` ("neutral"/"Wildlands" in Azgaar).
    #[inline]
    pub fn placeholder() -> Self {
        Self {
            id: 0,
            name: "Wildlands".to_string(),
            form: GovernmentForm::Anarchy,
            full_name: String::new(),
            color: String::new(),
            center_cell: 0,
            pole_of_inaccessibility: [0.0, 0.0],
            culture: 0,
            kind: CultureType::Generic,
            expansionism: 0.0,
            area_px: 0,
            burg_count: 0,
            cell_count: 0,
            rural_pop: 0.0,
            urban_pop: 0.0,
            neighbors: Vec::new(),
            provinces: Vec::new(),
            coat_of_arms: CoatOfArms::default(),
            locked: false,
            removed: false,
            diplomacy: serde_json::Value::Null,
            campaigns: serde_json::Value::Null,
            military: serde_json::Value::Null,
        }
    }
}
