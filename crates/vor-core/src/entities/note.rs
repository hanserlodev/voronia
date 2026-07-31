//! Note (slot `[4]`: notes JSON). Free-form user text associated with any entity
//! (burg, state, marker, etc.). Relevant for the Atenea integration (plan §22).

/// A legend/free-form note.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Note {
    pub id: u32,
    /// Legend text (may be multi-line).
    #[serde(default)]
    pub content: String,
    /// Id of the linked entity (burg/state/marker...). The type is inferred from context
    /// in Azgaar; we keep it as an opaque `u32` for now.
    #[serde(default)]
    pub linked_id: Option<u32>,
}
