//! Ajustes del mapa (slots `[0]` header + `[1]` settings del `.map` de Azgaar).
//!
//! Ref parse exacto en `docs/fase-0-investigacion.md` §12.1, §12.2. El header `[0]`
//! es pipe-delimited `version|license|date|seed|graphWidth|graphHeight|mapId`. El
//! settings `[1]` tiene ~27 campos pipe-delimited y un sub-JSON `options` embebido
//! en la posición `[19]` (resultado del `randomizeOptions()` de Azgaar — ver §7.2).

/// Header del `.map` (slot `[0]`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct MapHeader {
    /// Versión de Azgaar que produjo el archivo (`"1.138.0"` en Brample).
    pub version: String,
    /// Texto license/notice (`"File can be loaded in azgaar.github.io/Fantasy-Map-Generator"`).
    pub license: String,
    /// Fecha (formato `año-mes-día` sin zero-pad: `"2026-7-22"`).
    pub date: String,
    /// Semilla procedural de Azgaar como string (puede tener entre 1 y 10 dígitos numéricos).
    /// Importante: Azgaar la usa como string para `Alea(seed)`, no como entero.
    pub seed: String,
    /// Ancho del canvas (`graphWidth`) en unidades de Azgaar.
    pub graph_width: u32,
    /// Alto del canvas (`graphHeight`).
    pub graph_height: u32,
    /// Timestamp `Date.now()` al momento de creación — Id único del mapa.
    pub map_id: u64,
}

/// Settings de distancia/altura/unidades del mapa (slot `[1]`, primer tramo pipe-delimited).
///
/// Los campos vacíos en Azgaar (slots `[6]`–`[11]`, `[14]`–`[18]`) son compat con
/// migraciones antiguas — se preservan como `None` si el archivo los trae vacíos.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Settings {
    /// Unidad de distancia (`"km"`, `"mi"`, ...).
    pub distance_unit: String,
    /// Escala de la unidad de distancia (p.ej. `1` = 1 km por pixel; multiplicador).
    #[serde(default)]
    pub distance_scale: f32,
    /// Unidad de área (`"square"`, ...).
    pub area_unit: String,
    /// Unidad de altura (`"m"`, `"ft"`, ...).
    pub height_unit: String,
    /// Exponente de altura (Azgaar usa `2`).
    #[serde(default)]
    pub height_exponent: u32,
    /// Unidad de temperatura (`"°C"`, `"°F"`, `"K"`).
    pub temperature_unit: String,
    /// Tasa de población (puntos → habitantes, p. ej. `1000`).
    #[serde(default)]
    pub population_rate: f32,
    /// Tasa de urbanización (`1` por defecto en Brample).
    #[serde(default)]
    pub urbanization: f32,
    /// `options` completo — el sub-JSON que Azgaar serializa en la posición `[19]` de `[1]`.
    /// Es el resultado del `randomizeOptions()` (primer consumo generativo del PRNG
    /// `aleaPRNG` — NO `Alea@npm`). Si Voronia solo importa mapas ya generados,
    /// este payload se come como opaco y NO se re-genera (ver §13.4 de fase-0).
    #[serde(default, with = "crate::serde_json_string")]
    pub options: serde_json::Value,
    /// Nombre del mapa (slot `[1]` pos `[20]`).
    #[serde(default)]
    pub map_name: String,
    /// Ocultar labels (`[21]`).
    #[serde(default)]
    pub hide_labels: bool,
    /// Preset de estilo (`[22]`).
    #[serde(default)]
    pub style_preset: Option<String>,
    /// Rescalar labels (`[23]` — distinto del slot deprecated `[23]` del top-level .map;
    /// confirmar con parseo real; mantener opaco por ahora).
    #[serde(default, with = "crate::serde_json_string")]
    pub rescale_labels: serde_json::Value,
    /// Densidad urbana (`[24]`).
    #[serde(default, with = "crate::serde_json_string")]
    pub urban_density: serde_json::Value,
    /// Tasa de crecimiento (`[26]`).
    #[serde(default, with = "crate::serde_json_string")]
    pub growth_rate: serde_json::Value,
}
