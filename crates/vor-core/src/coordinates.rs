//! Map coordinates in lat/lon projection (slot `[2]`).
//!
//! Brample example: `{"latT":180,"latN":90,"latS":-90,"lonL":-180,"lonR":180,...}`.
//! Defines the geographic projection of the canvas. Preserve opaque the sub-fields
//! that Azgaar carries but Voronia v1 does not render (latBands, etc.).

/// Geographic coordinates of the map (slot `[2]`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct MapCoordinates {
    /// Total latitude (latitudinal range of the canvas).
    #[serde(default)]
    pub lat_t: f32,
    /// Latitude of the northern edge.
    #[serde(default)]
    pub lat_n: f32,
    /// Latitude of the southern edge.
    #[serde(default)]
    pub lat_s: f32,
    /// Longitude of the left edge.
    #[serde(default)]
    pub lon_l: f32,
    /// Longitude of the right edge.
    #[serde(default)]
    pub lon_r: f32,
    /// Opaque sub-json of advanced options (`latBands` etc.) that Azgaar uses to
    /// adjust the projection per band. Voronia v1 does not interpret them; it preserves them.
    #[serde(default, with = "crate::serde_json_string")]
    pub extras: serde_json::Value,
}
