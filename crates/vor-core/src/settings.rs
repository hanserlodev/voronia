//! Map settings (slots `[0]` header + `[1]` settings of Azgaar's `.map`).
//!
//! Exact parse reference in `docs/fase-0-investigacion.md` §12.1, §12.2. Header `[0]`
//! is pipe-delimited `version|license|date|seed|graphWidth|graphHeight|mapId`. Settings
//! `[1]` has ~27 pipe-delimited fields and an embedded sub-JSON `options` at
//! position `[19]` (result of Azgaar's `randomizeOptions()` — see §7.2).

/// Header of the `.map` (slot `[0]`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct MapHeader {
    /// Azgaar version that produced the file (`"1.138.0"` in Brample).
    pub version: String,
    /// License/notice text (`"File can be loaded in azgaar.github.io/Fantasy-Map-Generator"`).
    pub license: String,
    /// Date (`year-month-day` format without zero-padding: `"2026-7-22"`).
    pub date: String,
    /// Azgaar's procedural seed as a string (may have between 1 and 10 numeric digits).
    /// Important: Azgaar uses it as a string for `Alea(seed)`, not as an integer.
    pub seed: String,
    /// Canvas width (`graphWidth`) in Azgaar units.
    pub graph_width: u32,
    /// Canvas height (`graphHeight`).
    pub graph_height: u32,
    /// `Date.now()` timestamp at creation time — unique map id.
    pub map_id: u64,
}

/// Map distance/height/unit settings (slot `[1]`, first pipe-delimited stretch).
///
/// Empty fields in Azgaar (slots `[6]`–`[11]`, `[14]`–`[18]`) are for backward
/// compatibility with old migrations — they are preserved as `None` when the file
/// carries them empty.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Settings {
    /// Distance unit (`"km"`, `"mi"`, ...).
    pub distance_unit: String,
    /// Distance unit scale (e.g. `1` = 1 km per pixel; multiplier).
    #[serde(default)]
    pub distance_scale: f32,
    /// Area unit (`"square"`, ...).
    pub area_unit: String,
    /// Height unit (`"m"`, `"ft"`, ...).
    pub height_unit: String,
    /// Height exponent (Azgaar uses `2`).
    #[serde(default)]
    pub height_exponent: u32,
    /// Temperature unit (`"°C"`, `"°F"`, `"K"`).
    pub temperature_unit: String,
    /// Population rate (points → inhabitants, e.g. `1000`).
    #[serde(default)]
    pub population_rate: f32,
    /// Urbanization rate (`1` by default in Brample).
    #[serde(default)]
    pub urbanization: f32,
    /// Full `options` — the sub-JSON Azgaar serializes at position `[19]` of `[1]`.
    /// It is the result of `randomizeOptions()` (first generative consumption of the
    /// `aleaPRNG` PRNG — NOT `Alea@npm`). If Voronia only imports already-generated
    /// maps, this payload is consumed as opaque and is NOT re-generated (see phase-0 §13.4).
    #[serde(default, with = "crate::serde_json_string")]
    pub options: serde_json::Value,
    /// Map name (slot `[1]` pos `[20]`).
    #[serde(default)]
    pub map_name: String,
    /// Hide labels (`[21]`).
    #[serde(default)]
    pub hide_labels: bool,
    /// Style preset (`[22]`).
    #[serde(default)]
    pub style_preset: Option<String>,
    /// Rescale labels (`[23]` — distinct from the deprecated `[23]` slot of the top-level .map;
    /// confirm with real parsing; keep opaque for now).
    #[serde(default, with = "crate::serde_json_string")]
    pub rescale_labels: serde_json::Value,
    /// Urban density (`[24]`).
    #[serde(default, with = "crate::serde_json_string")]
    pub urban_density: serde_json::Value,
    /// Growth rate (`[26]`).
    #[serde(default, with = "crate::serde_json_string")]
    pub growth_rate: serde_json::Value,
}
