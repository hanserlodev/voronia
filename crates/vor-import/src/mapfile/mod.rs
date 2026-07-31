//! Parser for the Azgaar Fantasy Map Generator `.map` binary-text format.
//!
//! Format: array of strings joined by `\r\n`. Each slot is a string.
//! Slots indexed `[0]`..`[45]` approx. (see `docs/fase-0-investigacion.md` §12
//! for the "Brample" dissection).
//!
//! gzip compression **optional**: if the file does not start with a `[0]` slot
//! containing `|` (Azgaar's delimited header), we retry after gunzipping.
//! This replicates `parseLoadedResult` (`azgaar-fmg/src/services/io/load.ts:167-197`).
//!
//! SVG rescue: the `<svg id="map"...</svg>` block (typically slot `[5]`) can have
//! internal CRLFs, which would break the split. Before splitting, the block is
//! located and its internal `\r\n` replaced with `\n` (`load.ts:177-184`).
//!
//! The parser is layered:
//! - `RawMap`: raw string slots (post-split, post-SVG-rescue, post-gunzip).
//! - `Header`: parsing of the `|`-delimited slot `[0]`.
//! - `Settings`: parsing of the `|`-delimited slot `[1]`.
//! - `Loader::load(raw) -> World`: populates a `vor_core::World` with slots mapped
//!   to strong types + geometry regenerated from the header seed.

pub mod catalogs;
pub mod cells;
pub mod header;
pub mod loader;
pub mod raw;

pub use loader::{LoadError, LoadResult, Loader};
pub use raw::RawMap;
