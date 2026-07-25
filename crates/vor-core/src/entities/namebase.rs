//! Generador fonético de nombres por cultura (`namesData`, slot `[31]`).
//!
//! Formato de Azgaar en el Brample: `"German|5|12|lt|0|/English|6|11|..."` — cada
//! namebase separado por `/`, campos por `|`: `name|min|max|d|m|b`.
//! `d/m/b` son arrays serializados como strings (p.ej. `"lt"` = letras que se duplican,
//! `m` = array middle, `b` = array begin). Voronia los preserva como `String` opaco
//! en Fase 1; el deserializado exacto va a Fase 7 cuando se implemente el generador nativo.

/// Una namebase (catálogo de entrenamiento + parámetros de un generador fonético).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NameBase {
    pub id: u16,
    /// Nombre de la namebase ("English", "German", ...).
    pub name: String,
    /// Longitud mínima de un nombre generado.
    pub min_length: u32,
    /// Longitud máxima.
    pub max_length: u32,
    /// Comando `d` (de Azgaar) — array serializado. Preservar opaco en Fase 1.
    pub d: String,
    /// Comando `m` — array serializado opaco.
    pub m: String,
    /// Comando `b` — array serializado opaco.
    pub b: String,
    /// Probabilidad (0–1) de generar un nombre multi-palabra (pl. spec del plan §7.8).
    #[serde(default)]
    pub multiword_probability: Option<f32>,
}
