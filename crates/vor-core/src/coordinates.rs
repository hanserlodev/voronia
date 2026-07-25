//! Coordenadas del mapa en proyección lat/lon (slot `[2]`).
//!
//! Ejemplo del Brample: `{"latT":180,"latN":90,"latS":-90,"lonL":-180,"lonR":180,...}`.
//! Define la proyección geográfica del canvas. Preservaropaco los sub-campos que
//! Azgaar trae pero que Voronia v1 no renderiza (latBands, etc.).

/// Coordenadas geográficas del mapa (slot `[2]`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct MapCoordinates {
    /// Latitud total (rango latitudinal del canvas).
    #[serde(default)]
    pub lat_t: f32,
    /// Latitud del borde norte.
    #[serde(default)]
    pub lat_n: f32,
    /// Latitud del borde sur.
    #[serde(default)]
    pub lat_s: f32,
    /// Longitud del borde izquierdo.
    #[serde(default)]
    pub lon_l: f32,
    /// Longitud del borde derecho.
    #[serde(default)]
    pub lon_r: f32,
    /// Sub-json opaco de opciones avanzadas (`latBands` etc.) que Azgaar usa para
    /// ajustar la proyección por banda. Voronia v1 no las interpreta; las preserva.
    #[serde(default)]
    pub extras: serde_json::Value,
}
