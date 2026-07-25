//! Hielo (slot `[39]`: `pack.ice` JSON).
//!
//! Sistema separado del heightmap normal — cada elemento es un polygon de hielo.

/// Grupo de hielo (variants confirmadas en plan §7.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IceKind {
    #[default]
    Glacier,
    Iceberg,
}

/// Un elemento de hielo.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Ice {
    pub id: u32,
    pub kind: IceKind,
    /// Vértices del polígono `[[x, y], ...]`.
    #[serde(default)]
    pub vertices: Vec<[f32; 2]>,
    /// Celda central (opcional).
    #[serde(default)]
    pub cell: Option<u32>,
}
