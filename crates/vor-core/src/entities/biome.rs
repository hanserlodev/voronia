//! Bioma: definición (slot `[3]` del `.map` — los ids por celda van en `PackCells::biome`).

/// Un bioma del catálogo de Azgaar. El slot `[3]` del `.map` los lleva pipe-delimited
/// como `color|habitability|name`, con el id implícito por orden (0 = océano, 1 = tierra baja, etc.).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Biome {
    /// Id del bioma (su posición en el catálogo de Azgaar — 0-indexed).
    pub id: u8,
    /// Color hex (`#rrggbb`); Azgaar lo guarda en formato `rgb(R,G,B)` que `vor-import` normaliza.
    pub color: String,
    /// Habitabilidad (qué tan apto para asentamientos; usado en scoring de burgos).
    pub habitability: f32,
    /// Costo de movimiento por celda de este bioma (usado en expansión de culturas/estados).
    pub move_cost: f32,
    /// Nombre legible. En Azgaar el bioma `0` lleva el nombre `"Marine"` (océano/costa), etc.
    pub name: String,
}
