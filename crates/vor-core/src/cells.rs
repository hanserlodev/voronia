//! Atributos de celdas en layout Structure-of-Arrays.
//!
//! En Azgaar, `grid.cells.*` y `pack.cells.*` son TypedArrays paralelos indexados
//! por id de celda (id de grid y id de pack respectivamente — distintos namespaces,
//! `pack.cells.g[packId]` mapea al id de grid original). En Voronia mantenemos
//! exactamente el mismo layout SoA: jamás usamos `Vec<Cell>` con un struct gordo por elemento,
//! porque en mapas de 10k–100k celdas la localidad de cache sí importa (regla
//! `references/conventions.md` §"Layout de datos").
//!
//! Importante: **ni `Grid`, ni `Pack`, ni `GridCells`, ni `PackCells` se leen de archivo**
//! — la geometría (IDs de celdas, vértices, vecinos) se regenera bit-exacta desde
//! semilla + parámetros (ver `docs/fase-0-investigacion.md` §13.4 por qué esto es
//! crítico para no aplicar atributos a celdas equivocadas). Los atributos SÍ se
//! persisten (aquí), la geometría no.

/// Atributos de celdas del grid (slot `[7]`–`[11]` del `.map`).
///
/// La geometría asociada (IDs, vecinos, vértices) la repone `vor-import` regenerándola
/// desde la semilla. Acá solo viven los atributos serializados.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GridCells {
    /// Altura: 0–100, donde 20 es el nivel mínimo de tierra (slot `[7]`, Uint8).
    pub height: Vec<u8>,
    /// Precipitación (slot `[8]`).
    pub precipitation: Vec<u16>,
    /// Id de la feature (isla/lago/océano) a la que pertenece la celda (slot `[9]`, Uint16).
    pub feature_id: Vec<u16>,
    /// Tipo de celda respecto al agua/costa (slot `[10]`, Int8). Codificación de Azgaar:
    /// - `-2` = lago (no-costero si `i % 4 != 0`),
    /// - `-1` = agua costera (cercana a tierra),
    /// - `1`  = tierra costera (cercana a agua),
    /// - otro = tierra interior / océano profundo.
    pub water_type: Vec<i8>,
    /// Temperatura (slot `[11]`, Int8 en °C — puede ser negativa).
    pub temperature: Vec<i8>,
}

impl GridCells {
    /// Cantidad de celdas del grid. Debe calzar con `Grid::points.len()`.
    #[inline]
    pub fn len(&self) -> usize {
        self.height.len()
    }

    /// `true` si no hay celdas.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.height.is_empty()
    }
}

/// Atributos de celdas del pack (slots `[16]`–`[44]` del `.map`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PackCells {
    /// Id de grid original — mapeo pack→grid. Se llena durante el repacking en `vor-import`,
    /// no viene directo del archivo Azgaar (está implícito en el orden de `reGraph`).
    pub grid_id: Vec<u32>,
    /// Altura replicada desde el grid de origen (Uint8). Azgaar rellenaa `pack.cells.h` durante `reGraph`.
    pub height: Vec<u8>,
    /// Área de la celda en pixels², capped a `UINT16_MAX` (Uint16).
    pub area_px: Vec<u16>,
    /// Bioma (slot `[16]`, Uint8; el slot `[3]` lleva la tabla de biomas, no el id por celda).
    pub biome: Vec<u8>,
    /// Id de burgo (slot `[17]`, Uint16; `0` = sin burgo → `Option` se arma en `vor-import` con sentinel `0`).
    pub burg: Vec<u16>,
    /// Confluencia fluvial (slot `[18]`).
    pub confluence: Vec<u16>,
    /// Id de cultura (slot `[19]`, Uint16; `0` = Wildlands, no es `Option`).
    pub culture: Vec<u16>,
    /// Flujo de agua (slot `[20]`, Uint16).
    pub flux: Vec<u16>,
    /// Población en "puntos de población" (slot `[21]`, Float32 redondeado a 4 decimales; 1 pt = 1000 hab por defecto).
    pub population: Vec<f32>,
    /// Id de río que pasa por la celda (slot `[22]`, Uint16; `0` = sin río).
    pub river: Vec<u16>,
    /// Score de la celda para fundación de burgos (slot `[24]`, Uint16).
    pub score: Vec<u16>,
    /// Id de estado (slot `[25]`, Uint16; `0` = neutral/Wildlands).
    pub state: Vec<u16>,
    /// Id de religión (slot `[26]`, Uint16; `0` = sin religión).
    pub religion: Vec<u16>,
    /// Id de provincia (slot `[27]`, Uint16; `0` = sin provincia).
    pub province: Vec<u16>,
    /// Id de bien producido (slot `[40]`, Uint16; `0` = sin bien — sistema de economía, Fase 7).
    pub good: Vec<u16>,
    /// Id de mercado vinculado (slot `[44]`, Uint16; `0` = sin mercado).
    pub market: Vec<u16>,
    /// Rutas que parten de/atraviesan la celda (slot `[36]`, JSON adjacency map).
    /// Layout confirmado contra slot `[36]` de Brample: `{"6":{"7":359, "39":359}, "7":{...}}`
    /// (id de celda origen → {id de celda destino → id de ruta}).
    pub routes: Vec<RoutesFromCell>,
}

/// Rutas que parten de una celda. Sub-estructura de `PackCells::routes`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RoutesFromCell {
    /// Cada entrada = (celda destino, id de ruta).
    pub to: Vec<(u32, u32)>,
}

impl PackCells {
    /// Cantidad de celdas del pack. Debe calzar con `Pack::points.len()` (vor-import repoblará).
    #[inline]
    pub fn len(&self) -> usize {
        self.biome.len()
    }

    /// `true` si no hay celdas.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.biome.is_empty()
    }
}
