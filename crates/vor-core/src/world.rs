//! `World` — root of the World Data Model, what `vor-import` produces by populating everything else.

use crate::coordinates::MapCoordinates;
use crate::entities::{
    biome::Biome, burg::Burg, culture::Culture, deal::Deal, good::Good, ice::Ice, marker::Marker,
    market::Market, measurer::Measurer, namebase::NameBase, note::Note, province::Province,
    religion::Religion, river::River, route::Route, state::State, zone::Zone,
};
use crate::grid::Grid;
use crate::pack::Pack;
use crate::settings::{MapHeader, Settings};

/// The complete World Data Model: a map that has been loaded or generated.
///
/// `vor-import` populates this struct by regenerating geometry (Grid/Pack with cell counts and
/// Voronoi meshes) and applying attributes parsed from the `.map`. Every layout here
/// **is SoA** for per-cell attributes (plan §7.2; conventions.md §"Layout de datos").
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct World {
    /// Header of the `.map` (slot `[0]`).
    pub header: MapHeader,
    /// Settings of the `.map` (slot `[1]`).
    pub settings: Settings,
    /// Geographic coordinates of the map (slot `[2]`).
    pub coordinates: MapCoordinates,

    /// Grid mesh (regenerated from seed; attributes slots `[7]`–`[11]`).
    pub grid: Grid,
    /// Pack mesh (regenerated via `reGraph`; attributes slots `[16]`–`[44]`).
    pub pack: Pack,

    // --- Catalogs ---
    /// Biome catalog (slot `[3]`, pipe-delimited).
    pub biomes: Vec<Biome>,
    /// Notes/legends (slot `[4]` JSON).
    pub notes: Vec<Note>,
    /// Cultures (`pack.cultures`, slot `[13]`).
    pub cultures: Vec<Culture>,
    /// States (`pack.states`, slot `[14]`).
    pub states: Vec<State>,
    /// Burgs (`pack.burgs`, slot `[15]`).
    pub burgs: Vec<Burg>,
    /// Religions (`pack.religions`, slot `[29]`).
    pub religions: Vec<Religion>,
    /// Provinces (`pack.provinces`, slot `[30]`).
    pub provinces: Vec<Province>,
    /// Rivers (`pack.rivers`, slot `[32]`).
    pub rivers: Vec<River>,
    /// Markers (`pack.markers`, slot `[35]`).
    pub markers: Vec<Marker>,
    /// Routes (`pack.routes`, slot `[37]`).
    pub routes: Vec<Route>,
    /// Custom zones (`pack.zones`, slot `[38]`).
    pub zones: Vec<Zone>,
    /// Ice (`pack.ice`, slot `[39]`).
    pub ice: Vec<Ice>,
    /// NameBases (`namesData`, slot `[31]`).
    pub namebases: Vec<NameBase>,
    /// Measurement rules (`pack.measurers`, slot `[46]`).
    pub measurers: Vec<Measurer>,

    // --- Economy and military (Phase 7) ---
    //
    // Slots `[40]`–`[44]` (goods/markets/deals) and the `military` subtree within
    // `State::military` belong to Azgaar's economy and military subsystem, which
    // Voronia implements in Phase 7. For Phase 1 we keep them opaque (lossless
    // re-export) inside `State` or as `serde_json::Value`.
    /// Fonts (slot `[34]`) — Azgaar stores font configuration; keep opaque.
    /// No effect on `vor-render` (Voronia uses its own font stacks).
    #[serde(default, with = "crate::serde_json_string")]
    pub fonts: serde_json::Value,
    /// Custom good icons (slot `[45]`) — HTML outer string; keep opaque.
    #[serde(default, with = "crate::serde_json_string")]
    pub custom_good_icons: serde_json::Value,
    /// Economy: `pack.goods` (slot `[41]`) — typed catalog.
    #[serde(default)]
    pub goods: Vec<Good>,
    /// Economy: `pack.markets` (slot `[42]`) — typed catalog.
    #[serde(default)]
    pub markets: Vec<Market>,
    /// Economy: `pack.deals` (slot `[43]`) — typed catalog.
    #[serde(default)]
    pub deals: Vec<Deal>,
}
