//! Cultura (slot `[13]`: `pack.cultures` JSON `[{"name":"Wildlands","i":0,...}, ...]`).

use super::coat_of_arms::CoatOfArms;

/// Tipo de cultura. Variants confirmadas contra wiki "Culture types" de Azgaar (Fase 0 §4.3);
/// agregá las que falten según confirmación final sin romper migraciones viejas.
// TODO Fase 1: confirmar variants exactas y el nombre de código que Azgaar usa en
// el JSON (p.ej. "Generic"/"River"/"Lake"/"Naval"/"Nomadic"/"Hunting"/"Highland" — ver fase-0 §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CultureType {
    #[default]
    Generic,
    River,
    Lake,
    Naval,
    Nomadic,
    Hunting,
    Highland,
}

/// Una cultura (entrada de `pack.cultures`).
///
/// El item `[0]` de Azgaar es el placeholder "Wildlands" (cultura sin asignar).
/// Voronia lo mantiene en `pack.cultures[0]` para mantener el mapeo 1:1 con ids.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Culture {
    /// Id (su índice en `pack.cultures`; 0 = Wildlands).
    pub id: u16,
    /// Nombre de la cultura ("Wildlands" en el slot 0; luego "English", "German", etc.).
    pub name: String,
    /// Id del namebase asociado (índice en `namesData`, slot `[31]`).
    pub namebase_id: u16,
    /// Culturas de origen (ids), para el árbol evolutivo.
    #[serde(default)]
    pub origins: Vec<u16>,
    /// Escudo (compatible con Armoria de Watabou).
    #[serde(default)]
    pub shield: CoatOfArms,
    /// Celda central de la cultura.
    pub center_cell: u32,
    /// Abreviación/código (p.ej. "ENG" para English).
    #[serde(default)]
    pub code: String,
    /// Color hex.
    #[serde(default)]
    pub color: String,
    /// Multiplicador de expansión.
    #[serde(default)]
    pub expansionism: f32,
    /// Tipo de cultura.
    pub kind: CultureType,
    /// Área total en pixels² (poblada durante la simulación).
    #[serde(default)]
    pub area_px: u32,
    /// Cantidad de celdas bajo la cultura.
    #[serde(default)]
    pub cells: u32,
    /// Población rural (en "puntos de población", `f32`).
    #[serde(default)]
    pub rural_pop: f32,
    /// Población urbana.
    #[serde(default)]
    pub urban_pop: f32,
    /// `true` si el usuario la marcó como bloqueada (no reautogenerable).
    #[serde(default)]
    pub locked: bool,
    /// `true` si la cultura fue removida manualmente (soft delete — Azgaar mantiene el id libre).
    #[serde(default)]
    pub removed: bool,
}
