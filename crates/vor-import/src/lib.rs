//! `vor-import` — parsers de Azgaar + regeneración bit-exacta de geometría.
//!
//! Por ahora (Fase 1): solo el parser del `.map` legacy + regeneración de la
//! geometría del grid/pack. El JSON export Full (modo Full de `export-json.ts`)
//! está DIFERIDO a una fase siguiente.
//!
//! ## Bit-exactitud: por qué todo este crate existe
//!
//! El `.map` de Azgaar no guarda la geometría — solo atributos indexados por id
//! de celda (`pack.cells.biome[k]`, etc.) (hallazgo fase-0 §3, §13). Para que
//! los atributos matcheen la geografía correcta, hay que reproducir bit-exacto:
//! 1. `Alea(seed)` (npm, 1.0.1 de Baagøe) — aquí en `prng::alea`.
//! 2. `getJitteredGrid` + `getBoundaryPoints` (`graphUtils.ts:17-98`).
//! 3. `Delaunator.from(allPoints)` (la malla de triangulación).
//! 4. `Voronoi` class con `circumcenter` truncado a entero (voronoi.ts:142).
//! 5. `reGraph` (main.js:1157-1209) para `pack.cells.*`.
//!
//! Si cualquier paso diverge, los atributos caen en celdas equivocadas — sin
//! error visible, solo datos incorrectos (ver §13.4 consequence 3).

pub mod geometry;
pub mod numbers;
pub mod prng;
