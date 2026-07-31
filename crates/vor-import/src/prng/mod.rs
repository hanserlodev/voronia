//! Internal PRNG of `vor-import`.
//!
//! Only what Phase 1 needs: `Alea@1.0.1` (Baagøe, npm), the one Azgaar uses from
//! `generateGrid` onwards (see fase-0 §7.1). The vendored `aleaPRNG 1.1.0` is
//! NOT ported in Phase 1 (it belongs to the `setSeed → randomizeOptions` stretch,
//! unnecessary for importing an already-generated `.map` — §13.4).

pub mod alea;

pub use alea::Alea;
