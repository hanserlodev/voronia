//! Provincia (slot `[30]`: `pack.provinces` JSON). Subdivisión de un estado.

/// Una provincia. El slot `[0]` es placeholder.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Province {
    pub id: u16,
    pub name: String,
    /// Estado al que pertenece.
    pub state: u16,
    /// Cultura dominante.
    #[serde(default)]
    pub culture: u16,
    /// Burgo capital de la provincia (`None` si no tiene).
    #[serde(default)]
    pub capital: Option<u16>,
    /// Color hex.
    #[serde(default)]
    pub color: String,
    /// Celda central.
    #[serde(default)]
    pub center_cell: u32,
    /// "Centro visual" del polígono (pole of inaccessibility).
    #[serde(default)]
    pub pole_of_inaccessibility: [f32; 2],
    /// Burgos incluidos en la provincia.
    #[serde(default)]
    pub burgs: Vec<u16>,
    /// Cantidad de celdas.
    #[serde(default)]
    pub cells: u32,
    /// Área en pixels².
    #[serde(default)]
    pub area_px: u32,
    /// Población rural (puntos).
    #[serde(default)]
    pub rural_pop: f32,
    /// Población urbana.
    #[serde(default)]
    pub urban_pop: f32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub removed: bool,
}
