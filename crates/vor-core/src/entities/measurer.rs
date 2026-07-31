//! Measurement rule (slot `[46]`: `pack.measurers` JSON).
//!
//! A system of visual rules over the map (scale, distances).

/// A measurement rule.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Measurer {
    pub id: u32,
    /// Rule name.
    #[serde(default)]
    pub name: String,
    /// Control points `[x, y]` (polyline).
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
    /// Labeled length (km if `distanceUnit="km"`).
    #[serde(default)]
    pub length: Option<f32>,
}
