//! Errors of `vor-edit`.
//!
//! Controlled World Data Model mutations — they fail with `thiserror` instead
//! of panicking.

use thiserror::Error;

/// Voronia editing error.
#[derive(Debug, Error)]
pub enum EditError {
    /// The entity id does not exist in the catalog (e.g. `state_id = 99` when
    /// `world.states` only has 14 states).
    #[error("entity {what} with id {id} does not exist (collection length {len})")]
    EntityNotFound {
        what: &'static str,
        id: u16,
        len: usize,
    },

    /// The provided hex color is invalid (does not match `#rrggbb`).
    #[error("invalid hex color: {0:?} — expected format #rrggbb")]
    InvalidHexColor(String),

    /// The provided name is empty.
    #[error("empty name not allowed for {what} id={id}")]
    EmptyName { what: &'static str, id: u16 },
}
