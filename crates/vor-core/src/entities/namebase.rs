//! Phonetic name generator per culture (`namesData`, slot `[31]`).
//!
//! Azgaar's format in Brample: `"German|5|12|lt|0|/English|6|11|..."` — each
//! namebase separated by `/`, fields by `|`: `name|min|max|d|m|b`.
//! `d/m/b` are arrays serialized as strings (e.g. `"lt"` = letters that get doubled,
//! `m` = middle array, `b` = begin array). Voronia preserves them as opaque `String`
//! in Phase 1; the exact deserialization goes to Phase 7 when the native generator is implemented.

/// A namebase (training catalog + parameters of a phonetic generator).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NameBase {
    pub id: u16,
    /// Namebase name ("English", "German", ...).
    pub name: String,
    /// Minimum length of a generated name.
    pub min_length: u32,
    /// Maximum length.
    pub max_length: u32,
    /// `d` command (from Azgaar) — serialized array. Keep opaque in Phase 1.
    pub d: String,
    /// `m` command — opaque serialized array.
    pub m: String,
    /// `b` command — opaque serialized array.
    pub b: String,
    /// Probability (0–1) of generating a multi-word name (per the plan §7.8 spec).
    #[serde(default)]
    pub multiword_probability: Option<f32>,
}
