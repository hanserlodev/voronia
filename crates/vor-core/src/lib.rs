//! `vor-core` — World Data Model de Voronia.
//!
//! Puros tipos de datos del mundo: mallas (`Grid`/`Pack`), atributos por celda en
//! Structure-of-Arrays (`GridCells`/`PackCells`), entidades (`Culture`/`State`/`Burg`
//! /`Religion`/`Province`/`River`/`Marker`/`Route`/`Zone`/`Ice`), catálogos
//! (`Biome`/`NameBase`/`Note`) y la raíz `World` que los reúne.
//!
//! **No** vive lógica acá — ni de generación procedural (va a `vor-sim`), ni de
//! parseo de `.map` (va a `vor-import`), ni de serialización `.vorn` (va a `vor-format`),
//! ni de render (va a `vor-render`). Los límites entre crates están en
//! `references/architecture.md` §"Workspace y límites".
//!
//! ## Modelo de datos
//!
//! El diseño sigue el modelo de datos real de Azgaar (confirmado contra su wiki
//! `Data-model` y contra un `.map` real de prueba — ver `docs/fase-0-investigacion.md`
//! §10.1, §12.3). Lo que cambia respecto a Azgaar: tipado fuerte con enums en vez
//! de strings mágicos, `Option<...>` para ids opcionales en vez de sentinelares `0`
//! mágicos (cuando aplica), y preservación opaca (`serde_json::Value`) de los
//! subsistemas que no se modelan a fondo todavía (economía, milicia — Fase 7).
//!
//! ## Bit-exactitud
//!
//! Las mallas `Grid` y `Pack` **no se leen de archivo** — se regeneran bit-exactas
//! desde la semilla + parámetros en `vor-import` (hallazgo crítico, fase-0 §3 + §13.4).
//! Solo los atributos (`GridCells`, `PackCells`, entidades) son serializables.
//! Por eso el struct acá no tiene ni layout de celdas iterables ni `circumcenter`
//! ni algoritmos — eso es lógica, va en `vor-import`.

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

// Re-exports públicos en la raíz para uso idiomático: `vor_core::World`, `vor_core::Grid`, ...
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
