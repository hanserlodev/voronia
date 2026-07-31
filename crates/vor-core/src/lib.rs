//! `vor-core` — Voronia's World Data Model.
//!
//! Pure world data types: meshes (`Grid`/`Pack`), per-cell attributes in
//! Structure-of-Arrays layout (`GridCells`/`PackCells`), entities (`Culture`/`State`/`Burg`
//! /`Religion`/`Province`/`River`/`Marker`/`Route`/`Zone`/`Ice`), catalogs
//! (`Biome`/`NameBase`/`Note`) and the `World` root that brings them together.
//!
//! **No** logic lives here — not procedural generation (that goes to `vor-sim`), not
//! `.map` parsing (that goes to `vor-import`), not `.vorn` serialization (that goes to `vor-format`),
//! not rendering (that goes to `vor-render`). The boundaries between crates are in
//! `references/architecture.md` §"Workspace y límites".
//!
//! ## Data model
//!
//! The design follows Azgaar's real data model (confirmed against its `Data-model`
//! wiki page and a real test `.map` — see `docs/fase-0-investigacion.md`
//! §10.1, §12.3). What changes relative to Azgaar: strong typing with enums instead
//! of magic strings, `Option<...>` for optional ids instead of magic sentinel `0`
//! values (where applicable), and opaque preservation (`serde_json::Value`) of the
//! subsystems that are not modeled in depth yet (economy, military — Phase 7).
//!
//! ## Bit-exactness
//!
//! The `Grid` and `Pack` meshes are **not read from file** — they are regenerated
//! bit-exact from the seed + parameters in `vor-import` (critical finding, phase-0 §3 + §13.4).
//! Only the attributes (`GridCells`, `PackCells`, entities) are serializable.
//! That is why the struct here has neither iterable cell layout nor `circumcenter`
//! nor algorithms — that is logic, it goes in `vor-import`.

pub mod cells;
pub mod coordinates;
pub mod entities;
pub mod error;
pub mod feature;
pub mod grid;
pub mod pack;
pub mod serde_json_string;
pub mod settings;
pub mod voronoi;
pub mod world;

// Public re-exports at the root for idiomatic use: `vor_core::World`, `vor_core::Grid`, ...
pub use cells::{GridCells, PackCells, RoutesFromCell};
pub use coordinates::MapCoordinates;
pub use entities::{
    biome::Biome,
    burg::Burg,
    coat_of_arms::CoatOfArms,
    culture::{Culture, CultureType},
    ice::{Ice, IceKind},
    marker::Marker,
    measurer::Measurer,
    namebase::NameBase,
    note::Note,
    province::Province,
    religion::{Religion, ReligionExpansion, ReligionType},
    river::River,
    route::{Route, RouteGroup},
    state::{GovernmentForm, State},
    zone::Zone,
};
pub use error::CoreError;
pub use feature::{Feature, FeatureType, LakeGroup, LandGroup};
pub use grid::Grid;
pub use pack::Pack;
pub use settings::{MapHeader, Settings};
pub use voronoi::VoronoiVertices;
pub use world::World;
