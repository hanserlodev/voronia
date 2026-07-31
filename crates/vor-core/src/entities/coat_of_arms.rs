//! Coat of arms / blazon. Format compatible with Watabou's "Armoria" (MFCG/MFCG-Origen),
//! so Voronia can import/export the same format Azgaar uses today.

/// Blazon of an entity (burg, state). The exact field format is kept as
/// `serde_json::Value` until validated against Armoria's wiki in Phase 0/1
/// (a sub-format not deeply documented in the master plan).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CoatOfArms {
    /// Opaque blazon payload. Preserves all of Azgaar's JSON (= full interoperability
    /// with Armoria/MFCG without losing data when importing).
    #[serde(default, with = "crate::serde_json_string")]
    pub payload: serde_json::Value,
}
