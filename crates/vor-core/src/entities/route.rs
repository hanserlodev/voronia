//! Ruta (slot `[37]`: `pack.routes` JSON).
//!
//! Ejemplo del Brample: `{"i":0,"group":"roads","feature":2,"points":[[758.56,351.83,325],...]}`.

/// Tipo de ruta. Variants confirmadas contra Azgaar (slots `[37]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RouteGroup {
    #[default]
    Roads,
    Trails,
    Searoutes,
}

/// Una ruta. El id `0` se reserva como "no ruta" en `PackCells::routes`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Route {
    pub id: u32,
    /// Grupo (roads/trails/searoutes).
    pub group: RouteGroup,
    /// Id de feature (isla/lago/océano) por la que pasa la ruta (algunos casos como searoutes).
    #[serde(default)]
    pub feature: u32,
    /// Puntos de control `[x, y, z]` (la `z` suele llevar cell id de Azgaar; preservarla opaca por ahora).
    pub points: Vec<[f32; 3]>,
    /// Longitud en units de canvas (calza con `d3.length`).
    #[serde(default)]
    pub length: f32,
}
