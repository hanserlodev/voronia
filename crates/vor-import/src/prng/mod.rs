//! PRNG interno de `vor-import`.
//!
//! Solo el que se necesita para Fase 1: `Alea@1.0.1` (Baagøe, npm), que es el que
//! Azgaar usa desde `generateGrid` en adelante (ver fase-0 §7.1). El `aleaPRNG 1.1.0`
//! vendored NO se porta en Fase 1 (es del tramo `setSeed → randomizeOptions`,
//! innecesario para import `.map` ya generado — §13.4).

pub mod alea;

pub use alea::Alea;
