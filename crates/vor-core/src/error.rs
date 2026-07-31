//! Errors of Voronia's World Data Model.
//!
//! `vor-core` is pure: it does not parse files or validate IDs against geometry —
//! that is `vor-import`'s responsibility. Only errors that can arise when
//! building or using the pure types live here (IDs out of range, SoA layout
//! invariants, unknown enums during deserialization).

use thiserror::Error;

/// `vor-core` error. Library types use `thiserror`.
#[derive(Debug, Error)]
pub enum CoreError {
    /// A cell/feature/entity index falls outside the valid range of its collection.
    #[error("index out of range: {what} {index} does not exist in a collection of length {len}")]
    OutOfRange {
        what: &'static str,
        index: usize,
        len: usize,
    },

    /// Broken Structure-of-Arrays layout invariant: two `Vec`s that must be indexed
    /// in parallel by cell id have different lengths.
    #[error("inconsistent SoA layout: field `{field}` has length {actual} but {expected} was expected (matching the other cells)")]
    SoaLengthMismatch {
        field: &'static str,
        expected: usize,
        actual: usize,
    },

    /// Unknown enum variant during deserialization (e.g. `FeatureType` with a string
    /// that matches no known variant). `raw` preserves the original value for
    /// diagnostics and to allow a lossless re-export.
    #[error("unknown enum variant for {enum_name}: {raw:?}")]
    UnknownEnumVariant {
        enum_name: &'static str,
        raw: String,
    },
}
