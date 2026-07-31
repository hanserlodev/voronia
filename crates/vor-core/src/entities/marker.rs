//! Point-of-interest marker (slot `[35]`: `pack.markers` JSON).
//!
//! Brample example (fase-0 §12.3): `{"icon":"🌋","type":"volcanoes","dx":52,"px":13,"x":..,"y":..,"cell":..,"i":0}`.

/// A marker (POI pin) on the map.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Marker {
    /// Id (its index in `pack.markers`; Azgaar's slot `[0]` is usually `{}`).
    #[serde(default)]
    pub id: u32,
    /// Icon (emoji or unicode): "🌋", "⚔️", etc.
    pub icon: String,
    /// Marker type: "volcanoes", "battlefields", "ruins",... (Azgaar magic string;
    /// Phase 1 keeps it as an opaque `String`; strong enum if Voronia normalizes it later).
    pub kind: String,
    /// Label offset in `x` (Azgaar style).
    #[serde(default)]
    pub label_dx: i32,
    /// Label size in `px`.
    #[serde(default)]
    pub label_px: i32,
    /// Position on the canvas `[x, y]`.
    #[serde(default)]
    pub position: [f32; 2],
    /// Pack cell it falls on (for geographic linking).
    #[serde(default)]
    pub cell: u32,
    /// Associated legend/lore text (optional).
    #[serde(default)]
    pub legend: Option<String>,
    /// Free-form user note (id → `Note` in slot `[4]`).
    #[serde(default)]
    pub note_id: Option<u32>,
    /// `true` if the marker is "hidden" / removed manually in the UI.
    #[serde(default)]
    pub removed: bool,
}
