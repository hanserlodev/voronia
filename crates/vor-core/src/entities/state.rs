//! Estado / país (slot `[14]`: `pack.states` JSON).
//!
//! Algunos sub-objetos (regimientos, guerras, diplomacia, milicia) son propios de
//! la Fase 7 (simulación). Para Fase 1 los dejamos como `serde_json::Value` opaco
//! para no perder el dato al importar, y se desempaquetan después.

use super::coat_of_arms::CoatOfArms;
use super::culture::CultureType;

/// Forma de gobierno. Variants a confirmar contra wiki "Military Forces" / formularios
// TODO Fase 1: confirmar variants exactas (Monarchy/Republic/Theocracy/Union/Anarchy/Federation/...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GovernmentForm {
    Monarchy,
    Republic,
    Theocracy,
    Union,
    /// `Anarchy` = sin estado formal (calza con el placeholder neutral `State::placeholder()`).
    #[default]
    Anarchy,
}

/// Un estado / país. El slot `[0]` de `pack.states` es el placeholder "neutral".
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct State {
    /// Id (su índice en `pack.states`; 0 = neutral).
    pub id: u16,
    /// Nombre ("Tal Empire", "Kingdom of X", ...).
    pub name: String,
    /// Forma de gobierno.
    #[serde(default)]
    pub form: GovernmentForm,
    /// Nombre formal completo ("The Holy Kingdom of Tal", ...).
    #[serde(default)]
    pub full_name: String,
    /// Color hex.
    #[serde(default)]
    pub color: String,
    /// Celda central.
    pub center_cell: u32,
    /// "Centro visual" del polígono (técnica de pole of inaccessibility de Mapbox).
    #[serde(default)]
    pub pole_of_inaccessibility: [f32; 2],
    /// Cultura del estado (relaciona con Culture::id).
    pub culture: u16,
    /// Tipo de cultura (mismo enum que `Culture`).
    #[serde(default)]
    pub kind: CultureType,
    /// Multiplicador de expansión política.
    #[serde(default)]
    pub expansionism: f32,
    /// Área total en pixels².
    #[serde(default)]
    pub area_px: u32,
    /// Cantidad de burgos.
    #[serde(default)]
    pub burg_count: u32,
    /// Cantidad de celdas.
    #[serde(default)]
    pub cell_count: u32,
    /// Población rural (puntos).
    #[serde(default)]
    pub rural_pop: f32,
    /// Población urbana.
    #[serde(default)]
    pub urban_pop: f32,
    /// Estados vecinos (ids).
    #[serde(default)]
    pub neighbors: Vec<u16>,
    /// Provincias del estado (ids).
    #[serde(default)]
    pub provinces: Vec<u16>,
    /// Blasón.
    #[serde(default)]
    pub coat_of_arms: CoatOfArms,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub removed: bool,
    /// Diplomacia, guerras/campañas y milicia (`Regiment`/`War`) — propio de Fase 7.
    /// Se preserva opaco al importar para no perder dato; se desempaquetará en Fase 7.
    #[serde(default)]
    pub diplomacy: serde_json::Value,
    #[serde(default)]
    pub campaigns: serde_json::Value,
    #[serde(default)]
    pub military: serde_json::Value,
}

impl State {
    /// Placeholder para el slot `[0]` ("neutral"/"Wildlands" en Azgaar).
    #[inline]
    pub fn placeholder() -> Self {
        Self {
            id: 0,
            name: "Wildlands".to_string(),
            form: GovernmentForm::Anarchy,
            full_name: String::new(),
            color: String::new(),
            center_cell: 0,
            pole_of_inaccessibility: [0.0, 0.0],
            culture: 0,
            kind: CultureType::Generic,
            expansionism: 0.0,
            area_px: 0,
            burg_count: 0,
            cell_count: 0,
            rural_pop: 0.0,
            urban_pop: 0.0,
            neighbors: Vec::new(),
            provinces: Vec::new(),
            coat_of_arms: CoatOfArms::default(),
            locked: false,
            removed: false,
            diplomacy: serde_json::Value::Null,
            campaigns: serde_json::Value::Null,
            military: serde_json::Value::Null,
        }
    }
}
