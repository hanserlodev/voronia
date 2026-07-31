//! Ice (slot `[39]`: `pack.ice` JSON).
//!
//! Separate system from the normal heightmap — each element is an ice polygon.

/// Ice group (variants confirmed in plan §7.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IceKind {
    #[default]
    Glacier,
    Iceberg,
}

/// An ice element.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Ice {
    pub id: u32,
    pub kind: IceKind,
    /// Polygon vertices `[[x, y], ...]`.
    #[serde(default)]
    pub vertices: Vec<[f32; 2]>,
    /// Central cell (optional).
    #[serde(default)]
    pub cell: Option<u32>,
}
