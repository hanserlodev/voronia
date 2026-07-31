//! World entities: cultures, states, burgs, religions, provinces, rivers,
//! markers, routes, zones, ice. Consolidated listing to avoid cluttering `lib.rs`.
//!
//! Each entity here follows the data model confirmed against Azgaar (refs in
//! `docs/plans/master-plan.md` §7.4–§7.7 and `docs/phases/phase-0-research.md` §10.1).
//! The Azgaar model enums whose variants need confirmation against the wiki are
//! marked `// TODO Phase 0/1: confirmar variants contra wiki` — even so we keep
//! them closed to preserve strong typing.

pub mod biome;
pub mod burg;
pub mod coat_of_arms;
pub mod culture;
pub mod ice;
pub mod marker;
pub mod measurer;
pub mod namebase;
pub mod note;
pub mod province;
pub mod religion;
pub mod river;
pub mod route;
pub mod state;
pub mod zone;
