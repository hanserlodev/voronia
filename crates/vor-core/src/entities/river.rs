//! Río (slot `[32]`: `pack.rivers` JSON).
//!
//! A diferencia de Azgaar (que guarda `discharge`, `length`, `width` sin unidades
//! físicas), Voronia usa unidades físicas reales (plan §7.7): `m³/s` para caudal,
//! `km` para longitud/ancho.

/// Un río. El id `0` se reserva como "no río" en `PackCells::river`; los ríos
/// reales arrancan en id `1`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct River {
    pub id: u16,
    pub name: String,
    /// Celda fuente (nacimiento).
    pub source_cell: u32,
    /// Celda desembocadura (mouth — donde mete al mar o se une a otro río).
    pub mouth_cell: u32,
    /// Río padre (`Some` si este río es afluente de otro; `None` si desemboca directo al mar/lago).
    #[serde(default)]
    pub parent_river: Option<u16>,
    /// Id de la cuenca hidrográfica a la que pertenece.
    #[serde(default)]
    pub basin_id: u16,
    /// Caudal en m³/s (físico, no arbitrario).
    #[serde(default)]
    pub discharge_m3s: f32,
    /// Longitud en km.
    #[serde(default)]
    pub length_km: f32,
    /// Ancho medio en km.
    #[serde(default)]
    pub width_km: f32,
    /// Camino de celdas pack que recorre el río (desde `source_cell` hasta `mouth_cell`).
    /// Poblado por `vor-import` durante la carga. No se persiste (derivable de `river` ids
    /// + adyacencia + flujo).
    #[serde(skip)]
    pub cell_path: Vec<u32>,
}
