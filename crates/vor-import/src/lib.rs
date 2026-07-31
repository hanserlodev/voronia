//! `vor-import` — Azgaar parsers + bit-exact geometry regeneration.
//!
//! For now (Phase 1): only the legacy `.map` parser + grid/pack geometry
//! regeneration. The Full JSON export (Full mode of `export-json.ts`) is
//! DEFERRED to a later phase.
//!
//! ## Bit-exactness: why this entire crate exists
//!
//! The Azgaar `.map` does not store geometry — only attributes indexed by cell
//! id (`pack.cells.biome[k]`, etc.) (phase-0 finding §3, §13). For attributes to
//! match the correct geography, the following must be reproduced bit-exactly:
//! 1. `Alea(seed)` (npm, 1.0.1 by Baagøe) — here in `prng::alea`.
//! 2. `getJitteredGrid` + `getBoundaryPoints` (`graphUtils.ts:17-98`).
//! 3. `Delaunator.from(allPoints)` (the triangulation mesh).
//! 4. `Voronoi` class with integer-truncated `circumcenter` (voronoi.ts:142).
//! 5. `reGraph` (main.js:1157-1209) for `pack.cells.*`.
//!
//! If any step diverges, attributes land in the wrong cells — with no visible
//! error, just incorrect data (see §13.4 consequence 3).

pub mod geometry;
pub mod mapfile;
pub mod numbers;
pub mod prng;
pub mod regraph;
