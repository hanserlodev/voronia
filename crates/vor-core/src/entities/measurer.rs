//! Regla de medición (slot `[46]`: `pack.measurers` JSON).
//!
//! Es un sistema de reglas visuales sobre el mapa (escala, distancias).

/// Una regla de medición.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Measurer {
    pub id: u32,
    /// Nombre de la regla.
    #[serde(default)]
    pub name: String,
    /// Puntos de control `[x, y]` (línea poligonal).
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
    /// Longitud etiquetada (km si `distanceUnit="km"`).
    #[serde(default)]
    pub length: Option<f32>,
}
