//! Cell attributes in Structure-of-Arrays layout.
//!
//! In Azgaar, `grid.cells.*` and `pack.cells.*` are parallel TypedArrays indexed
//! by cell id (grid id and pack id respectively — separate namespaces,
//! `pack.cells.g[packId]` maps to the original grid id). Voronia keeps exactly
//! the same SoA layout: we never use `Vec<Cell>` with a fat struct per element,
//! because on maps of 10k–100k cells cache locality really matters (rule
//! `references/conventions.md` §"Layout de datos").
//!
//! Important: **neither `Grid`, nor `Pack`, nor `GridCells`, nor `PackCells` are read from file**
//! — the geometry (cell IDs, vertices, neighbors) is regenerated bit-exact from
//! seed + parameters (see `docs/phases/phase-0-research.md` §13.4 for why this is
//! critical to avoid applying attributes to wrong cells). Attributes ARE
//! persisted (here); geometry is not.

/// Grid cell attributes (slots `[7]`–`[11]` of the `.map`).
///
/// The associated geometry (IDs, neighbors, vertices) is restored by `vor-import`
/// regenerating it from the seed. Only the serialized attributes live here.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GridCells {
    /// Height: 0–100, where 20 is the minimum land level (slot `[7]`, Uint8).
    pub height: Vec<u8>,
    /// Precipitation (slot `[8]`).
    pub precipitation: Vec<u16>,
    /// Id of the feature (island/lake/ocean) the cell belongs to (slot `[9]`, Uint16).
    pub feature_id: Vec<u16>,
    /// Cell type with respect to water/coast (slot `[10]`, Int8). Azgaar's encoding:
    /// - `-2` = lake (non-coastal if `i % 4 != 0`),
    /// - `-1` = coastal water (near land),
    /// - `1`  = coastal land (near water),
    /// - other = inland land / deep ocean.
    pub water_type: Vec<i8>,
    /// Temperature (slot `[11]`, Int8 in °C — may be negative).
    pub temperature: Vec<i8>,
}

impl GridCells {
    /// Number of grid cells. Must match `Grid::points.len()`.
    #[inline]
    pub fn len(&self) -> usize {
        self.height.len()
    }

    /// `true` if there are no cells.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.height.is_empty()
    }
}

/// Pack cell attributes (slots `[16]`–`[44]` of the `.map`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PackCells {
    /// Original grid id — pack→grid mapping. Filled during repacking in `vor-import`,
    /// not read directly from the Azgaar file (it is implicit in `reGraph`'s ordering).
    pub grid_id: Vec<u32>,
    /// Height replicated from the source grid (Uint8). Azgaar fills `pack.cells.h` during `reGraph`.
    pub height: Vec<u8>,
    /// Cell area in pixels², capped at `UINT16_MAX` (Uint16).
    pub area_px: Vec<u16>,
    /// Biome (slot `[16]`, Uint8; slot `[3]` holds the biome table, not the per-cell id).
    pub biome: Vec<u8>,
    /// Burg id (slot `[17]`, Uint16; `0` = no burg → `Option` is built in `vor-import` from the `0` sentinel).
    pub burg: Vec<u16>,
    /// River confluence (slot `[18]`).
    pub confluence: Vec<u16>,
    /// Culture id (slot `[19]`, Uint16; `0` = Wildlands, not `Option`).
    pub culture: Vec<u16>,
    /// Water flow (slot `[20]`, Uint16).
    pub flux: Vec<u16>,
    /// Population in "population points" (slot `[21]`, Float32 rounded to 4 decimal places; 1 pt = 1000 people by default).
    pub population: Vec<f32>,
    /// Id of the river passing through the cell (slot `[22]`, Uint16; `0` = no river).
    pub river: Vec<u16>,
    /// Cell score for burg foundation (slot `[24]`, Uint16).
    pub score: Vec<u16>,
    /// State id (slot `[25]`, Uint16; `0` = neutral/Wildlands).
    pub state: Vec<u16>,
    /// Religion id (slot `[26]`, Uint16; `0` = no religion).
    pub religion: Vec<u16>,
    /// Province id (slot `[27]`, Uint16; `0` = no province).
    pub province: Vec<u16>,
    /// Id of the produced good (slot `[40]`, Uint16; `0` = no good — economy system, Phase 7).
    pub good: Vec<u16>,
    /// Id of the linked market (slot `[44]`, Uint16; `0` = no market).
    pub market: Vec<u16>,
    /// Routes departing from/crossing the cell (slot `[36]`, JSON adjacency map).
    /// Layout confirmed against Brample's slot `[36]`: `{"6":{"7":359, "39":359}, "7":{...}}`
    /// (source cell id → {destination cell id → route id}).
    pub routes: Vec<RoutesFromCell>,
    /// Id of the feature (island/lake/ocean) the cell belongs to.
    /// Populated by `vor-import` from `re_graph` + grid feature mapping.
    pub feature_id: Vec<u16>,
    /// Adjacent pack cell IDs (inner neighbors, without boundary).
    /// Populated by `vor-import` during `re_graph` (from the second `calculate_voronoi`).
    /// Not persisted — derivable from the Delaunay.
    #[serde(skip)]
    pub adjacency: Vec<Vec<u32>>,
}

/// Routes departing from a cell. Sub-structure of `PackCells::routes`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RoutesFromCell {
    /// Each entry = (destination cell, route id).
    pub to: Vec<(u32, u32)>,
}

impl PackCells {
    /// Number of pack cells. Must match `Pack::points.len()` (vor-import will repopulate).
    #[inline]
    pub fn len(&self) -> usize {
        self.biome.len()
    }

    /// `true` if there are no cells.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.biome.is_empty()
    }
}
