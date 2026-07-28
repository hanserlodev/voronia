//! Errores de `vor-edit`.
//!
//! Mutaciones del World Data Model controladas — fallan con `thiserror` en vez
//! de panickear (regla de `references/conventions.md` §"Manejo de errores").

use thiserror::Error;

/// Error de edición de Voronia.
#[derive(Debug, Error)]
pub enum EditError {
    /// El id de entidad no existe en el catálogo (p.ej. `state_id = 99` cuando
    /// `world.states` solo tiene 14 estados).
    #[error("entidad {what} con id {id} no existe (colección de largo {len})")]
    EntityNotFound {
        what: &'static str,
        id: u16,
        len: usize,
    },

    /// El color hex provisto es inválido (no calza `#rrggbb`).
    #[error("color hex inválido: {0:?} — se espera formato #rrggbb")]
    InvalidHexColor(String),

    /// El nombre provisto está vacío.
    #[error("nombre vacío no permitido para {what} id={id}")]
    EmptyName { what: &'static str, id: u16 },
}
