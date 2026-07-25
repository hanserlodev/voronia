//! Burgos / asentamientos (slot `[15]`: `pack.burgs` JSON `[{}, {"cell":1133,"x":1468.66,...}]`).
//!
//! El slot `[0]` de `pack.burgs` es siempre `{}` (placeholder). En Voronia lo
//! inicializamos como `Burg::placeholder()` para mantener ids 1-based.

use super::coat_of_arms::CoatOfArms;
use super::culture::CultureType;

/// Un burgo / asentamiento.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Burg {
    /// Id (su índice en `pack.burgs`; 0 = placeholder).
    pub id: u16,
    /// Nombre del burgo (p.ej. "Tal").
    pub name: String,
    /// Celda del pack donde está ubicado.
    pub cell: u32,
    /// Coordenadas `[x, y]` en units de canvas (no cell center, pueden ser el punto exacto).
    pub position: [f32; 2],
    /// Id de cultura.
    pub culture: u16,
    /// Id de estado.
    pub state: u16,
    /// Id de feature (isla/lago/océano) en la que cae.
    pub feature: u32,
    /// Población en "puntos" (`f32`; 1 pt = 1000 hab por defecto).
    #[serde(default)]
    pub population: f32,
    /// Tipo de cultura del burgo (mismo enum que `Culture`.
    #[serde(default)]
    pub kind: CultureType,
    /// Escudo (compatible con Armoria).
    #[serde(default)]
    pub coat_of_arms: CoatOfArms,
    /// `true` si es capital del estado.
    #[serde(default)]
    pub is_capital: bool,
    /// Id de feature de tipo agua con puerto (`Some` si es puerto; celda de puerto via `haven_cell`).
    #[serde(default)]
    pub port_feature: Option<u32>,
    /// Flags de MFCG (Watabou — Medieval Fantasy City Generator). Import guardarlos
    /// para compatibilidad de re-export, aunque Voronia v1 no integre MFCG.
    #[serde(default)]
    pub has_citadel: bool,
    #[serde(default)]
    pub has_plaza: bool,
    #[serde(default)]
    pub has_shanty: bool,
    #[serde(default)]
    pub has_temple: bool,
    #[serde(default)]
    pub has_walls: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub removed: bool,
}

impl Burg {
    /// Placeholder reservado — refleja el slot `[0]` (vacío) que Azgaar deja en `pack.burgs`.
    #[inline]
    pub fn placeholder() -> Self {
        Self {
            id: 0,
            name: String::new(),
            cell: 0,
            position: [0.0, 0.0],
            culture: 0,
            state: 0,
            feature: 0,
            population: 0.0,
            kind: CultureType::Generic,
            coat_of_arms: CoatOfArms::default(),
            is_capital: false,
            port_feature: None,
            has_citadel: false,
            has_plaza: false,
            has_shanty: false,
            has_temple: false,
            has_walls: false,
            locked: false,
            removed: false,
        }
    }
}
