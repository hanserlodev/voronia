//! Religión (slot `[29]`: `pack.religions` JSON).

/// Tipo de religión. Variants confirmadas en el modelo de Azgaar (plan §7.7).
// TODO Fase 1: confirmar nombre de `Organized` (Azgaar usa "Organized"也许是 "Organized Religion" —lock exacto).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReligionType {
    #[default]
    Folk,
    Organized,
    Heresy,
    Cult,
}

/// Modo/dominio de expansión de la religión (§7.7 del plan).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReligionExpansion {
    /// Solo dentro de su cultura de origen.
    #[default]
    Culture,
    /// Global — cualquier cultura.
    Global,
}

/// Una religión (entrada de `pack.religions`). El slot `[0]` es placeholder.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Religion {
    pub id: u16,
    pub name: String,
    /// Culturas/religiones de origen (árbol evolutivo).
    #[serde(default)]
    pub origins: Vec<u16>,
    /// Color hex.
    #[serde(default)]
    pub color: String,
    /// Tipo de religión.
    pub kind: ReligionType,
    /// Modo de expansión.
    #[serde(default)]
    pub expansion: ReligionExpansion,
    /// Celda central.
    #[serde(default)]
    pub center_cell: u32,
    /// Cantidad de celdas.
    #[serde(default)]
    pub cells: u32,
    /// Área total en pixels².
    #[serde(default)]
    pub area_px: u32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub removed: bool,
}
