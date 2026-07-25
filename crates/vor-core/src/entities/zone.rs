//! Zona (slot `[38]`: `pack.zones` JSON). Overlay de color custom sobre un set de celdas.

/// Una zona custom (p.ej. "región en guerra", "tierras de caza privativas", etc.).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Zone {
    pub id: u32,
    pub name: String,
    /// Color hex del overlay.
    #[serde(default)]
    pub color: String,
    /// Cells que conforman la zona (ids de pack).
    #[serde(default)]
    pub cells: Vec<u32>,
    /// `"random" | "solid"` u otros estilos de patrones de hatching en Azgaar (preservar opaco).
    #[serde(default)]
    pub style: Option<String>,
    /// Descripción/leyenda libre para la UI.
    #[serde(default)]
    pub description: Option<String>,
}
