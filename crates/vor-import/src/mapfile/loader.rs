//! Orchestrating loader — combines `raw` + `header` + `cells` + `catalogs` + the
//! geometry regeneration (`place_points` + `calculate_voronoi` + `re_graph`)
//! to produce a complete `vor_core::World`.
//!
//! Bit-exactness guarantees:
//! 1. Grid geometry: `place_points(seed, graphWidth, graphHeight, cellsDesired)` must
//!    reproduce bit-exactly the `[6].points` slot of the `.map` (handshake test in
//!    `tests/sorvik_handshake.rs`).
//! 2. Grid Voronoi: `calculate_voronoi(allPoints, pointsN)` must reproduce
//!    bit-exactly the `cells.v/c/b` + `vertices.p/v/c` that Azgaar computes.
//! 3. Pack: `re_graph(...)` must reproduce bit-exactly the `pack.points` count
//!    (Sorvik expects 7268 pack cells after `re_graph`).
//! 4. Attributes: the `.map` slots `[7]`-`[11]` (grid) and `[16]`-`[27]` (pack)
//!    are applied 1-to-1 over the regenerated mesh, with the same id mapping
//!    (`pack.cells.g[packId] → gridId`) that Azgaar originally generated. If the
//!    caller's mesh diverges from the header seed's, these attributes land in the
//!    wrong cells — a silent bug. Here we guarantee it via handshakes in tests.

use thiserror::Error;
use vor_core::cells::GridCells;
use vor_core::world::World;
use vor_core::Grid;

use crate::geometry::delaunay::from_pairs;
use crate::geometry::{
    place_points,
    voronoi::{calculate_voronoi, Voronoi},
};
use crate::mapfile::cells::parse_grid_cells;
use crate::mapfile::raw::RawMap;
use crate::regraph::re_graph;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error(transparent)]
    Raw(#[from] crate::mapfile::raw::RawError),
    #[error(transparent)]
    Header(#[from] crate::mapfile::header::HeaderError),
    #[error(transparent)]
    Cells(#[from] crate::mapfile::cells::CellError),
    #[error(transparent)]
    Catalog(#[from] crate::mapfile::catalogs::CatalogError),
    #[error("inconsistent geometry: {0}")]
    Geometry(String),
}

/// Result of loading a `.map`: the populated `World` + the pack `new_points` in
/// f64 (before the f32 cast to storage — see `regraph::re_graph` docstring).
pub struct LoadResult {
    pub world: World,
    pub pack_new_points_f64: Vec<[f64; 2]>,
}

/// Main loader API.
///
/// Usage:
/// ```ignore
/// let bytes = std::fs::read("Sorvik.map")?;
/// let raw = vor_import::mapfile::raw::parse(&bytes)?;
/// let loaded = vor_import::mapfile::Loader::load(&raw)?;
/// let world = loaded.world;
/// // `loaded.pack_new_points_f64` if you want to validate pack bit-exactness.
/// ```
pub struct Loader;

impl Loader {
    /// Loads a `RawMap` (post `raw::parse`) into a complete `World`.
    pub fn load(raw: &RawMap) -> Result<LoadResult, LoadError> {
        // --- Header ---
        let slot0 = raw.must(0);
        let header = crate::mapfile::header::parse_header(slot0)?;
        let settings = crate::mapfile::header::parse_settings(raw.must(1))?;
        let coordinates = crate::mapfile::header::parse_coordinates(raw.get(2))?;

        // --- Grid general (slot [6] JSON) ---
        let grid_general = crate::mapfile::cells::parse_grid_general(raw.get(6))?;

        // --- Regenerated geometry: place_points ---
        let placed = place_points(
            header.graph_width as f64,
            header.graph_height as f64,
            grid_general.cells_desired(),
            &header.seed,
        );

        // Sanity: `place_points` must yield the same number of cells as slot [6].points.
        if placed.points.len() != grid_general.points.len() {
            return Err(LoadError::Geometry(format!(
                "place_points count {} != slot[6].points count {} — seed/width/height/cellsDesired mismatch",
                placed.points.len(),
                grid_general.points.len()
            )));
        }

        // --- Voronoi grid ---
        let points_n = placed.points.len();
        let all_points: Vec<[f64; 2]> = placed
            .points
            .iter()
            .cloned()
            .chain(placed.boundary.iter().cloned())
            .collect();
        let delaunay = from_pairs(&all_points);
        let voronoi = calculate_voronoi(&delaunay, &all_points, points_n as u32);

        // --- Slots [7]-[11] → grid.cells (attributes) ---
        let grid_cells: GridCells =
            parse_grid_cells(raw.get(7), raw.get(8), raw.get(9), raw.get(10), raw.get(11));
        // Sanity: GridCells must have the same length as `points_n`.
        let n_cells = grid_cells.height.len();
        if n_cells != points_n {
            return Err(LoadError::Geometry(format!(
                "grid.cells h length={} != points count {} — archive mismatch",
                n_cells, points_n
            )));
        }

        // --- reGraph → Pack ---
        // The grid features (slot [6].features, not slot [12]) are the ones Azgaar uses
        // during `reGraph` to distinguish lakes from internal oceans (`features[f].type === "lake"`).
        // Slot [12] carries `pack.features` (post-markup), different from the grid ones.
        let features_kind: Vec<vor_core::feature::FeatureType> =
            crate::mapfile::cells::parse_grid_features_kind(raw.get(6))?;

        let (mut pack, new_points_f64) = re_graph(
            &placed.points,
            &placed.boundary,
            &voronoi,
            &grid_cells.height,
            &grid_cells.water_type,
            &grid_cells.feature_id,
            &features_kind,
            placed.spacing,
        );

        // --- PackCells attributes (slots [16]-[27], [36], [40], [44]) ---
        let mut pack_cells = crate::mapfile::cells::parse_pack_cells(
            raw.get(16),
            raw.get(17),
            raw.get(18),
            raw.get(19),
            raw.get(20),
            raw.get(21),
            raw.get(22),
            raw.get(24),
            raw.get(25),
            raw.get(26),
            raw.get(27),
            raw.get(36),
            raw.get(40),
            raw.get(44),
        );
        // The expected length is pack.points.len() (per `re_graph`).
        let expected = pack.points.len();
        // Tracks mismatches; we emit a single aggregated error if lengths differ.
        let mut mismatches = Vec::new();
        macro_rules! chk {
            ($field:ident) => {
                if pack_cells.$field.len() != 0 && pack_cells.$field.len() != expected {
                    mismatches.push(format!(
                        "{}: {} (expected {})",
                        stringify!($field),
                        pack_cells.$field.len(),
                        expected
                    ));
                }
            };
        }
        chk!(biome);
        chk!(burg);
        chk!(confluence);
        chk!(culture);
        chk!(flux);
        chk!(population);
        chk!(river);
        chk!(score);
        chk!(state);
        chk!(religion);
        chk!(province);
        chk!(good);
        chk!(market);
        if !mismatches.is_empty() {
            return Err(LoadError::Geometry(format!(
                "pack.cells attributes length mismatch vs re_graph pack points count ({}): {}",
                expected,
                mismatches.join("; ")
            )));
        }

        // Update `pack.cells` with the parsed attributes plus the `grid_id` and
        // `height` and `area_px` that `re_graph` already populated in its internal PackCells.
        // NOTE: `pack.cells` was initialized by `re_graph` with grid_id/height/area_px
        // already populated. We replace only the attributes parsed from the .map.
        pack.cells.biome = std::mem::take(&mut pack_cells.biome);
        pack.cells.burg = std::mem::take(&mut pack_cells.burg);
        pack.cells.confluence = std::mem::take(&mut pack_cells.confluence);
        pack.cells.culture = std::mem::take(&mut pack_cells.culture);
        pack.cells.flux = std::mem::take(&mut pack_cells.flux);
        pack.cells.population = std::mem::take(&mut pack_cells.population);
        pack.cells.river = std::mem::take(&mut pack_cells.river);
        pack.cells.score = std::mem::take(&mut pack_cells.score);
        pack.cells.state = std::mem::take(&mut pack_cells.state);
        pack.cells.religion = std::mem::take(&mut pack_cells.religion);
        pack.cells.province = std::mem::take(&mut pack_cells.province);
        pack.cells.good = std::mem::take(&mut pack_cells.good);
        pack.cells.market = std::mem::take(&mut pack_cells.market);
        pack.cells.routes = std::mem::take(&mut pack_cells.routes);

        // Populate feature_id for pack cells from grid cells via grid_id mapping
        pack.cells.feature_id = pack
            .cells
            .grid_id
            .iter()
            .map(|&gid| {
                let idx = gid as usize;
                grid_cells.feature_id.get(idx).copied().unwrap_or(0)
            })
            .collect();

        // --- Grid model — the Voronoi topology is not persisted in vor-core::Grid
        // (derivable), only serialized attributes. We keep points/boundary/cells/vertices.
        let grid = Grid {
            cells_desired: grid_general.cells_desired(),
            spacing: placed.spacing as f32,
            cells_x: placed.cells_x,
            cells_y: placed.cells_y,
            width: header.graph_width as f32,
            height: header.graph_height as f32,
            points: placed
                .points
                .iter()
                .map(|p| [p[0] as f32, p[1] as f32])
                .collect(),
            boundary: placed
                .boundary
                .iter()
                .map(|p| [p[0] as f32, p[1] as f32])
                .collect(),
            cells: grid_cells,
            vertices: voronoi_to_vor_core(&voronoi),
            features: crate::mapfile::cells::parse_grid_features(raw.get(6))?,
        };

        // --- Catalogs (slots [3]/[4]/[12]-[15]/[29]-[46]) ---
        let biomes = crate::mapfile::catalogs::parse_biomes(raw.must(3))?;
        let notes = crate::mapfile::catalogs::parse_notes(raw.get(4))?;
        let features = crate::mapfile::catalogs::parse_features(raw.get(12))?;
        let cultures = crate::mapfile::catalogs::parse_cultures(raw.get(13))?;
        let states = crate::mapfile::catalogs::parse_states(raw.get(14))?;
        let burgs = crate::mapfile::catalogs::parse_burgs(raw.get(15))?;
        let religions = crate::mapfile::catalogs::parse_religions(raw.get(29))?;
        let provinces = crate::mapfile::catalogs::parse_provinces(raw.get(30))?;
        let mut rivers = crate::mapfile::catalogs::parse_rivers(raw.get(32))?;
        let markers = crate::mapfile::catalogs::parse_markers(raw.get(35))?;
        let routes = crate::mapfile::catalogs::parse_routes(raw.get(37))?;
        let zones = crate::mapfile::catalogs::parse_zones(raw.get(38))?;
        let ice = crate::mapfile::catalogs::parse_ice(raw.get(39))?;
        let measurers = crate::mapfile::catalogs::parse_measurers(raw.get(46))?;
        let namebases = crate::mapfile::catalogs::parse_namebases(raw.get(31))?;

        // Opaques: fonts [34], goods [41]/[42]/[43], custom_good_icons [45]
        let fonts = parse_json_opaque(raw.get(34));
        let goods = parse_json_opaque(raw.get(41));
        let markets = parse_json_opaque(raw.get(42));
        let deals = parse_json_opaque(raw.get(43));
        let custom_good_icons = parse_string_opaque(raw.get(45));

        // --- Trace each river's path (cell_path from source to mouth) ---
        // Only rivers with an empty `cell_path` are reconstructed. Azgaa serializes its
        // `cells` in slot [32] verbatim, INCLUDING the final water cell where the river
        // pours out (its `river_id` is 0, so it cannot be rediscovered by tracing);
        // tracing would drop that cell and the river would stop short of the sea.
        trace_river_paths(
            &mut rivers,
            &pack.cells.river,
            &pack.cells.adjacency,
            &pack.cells.height,
        );

        // --- features go into the pack ---
        pack.features = features;

        let world = World {
            header,
            settings,
            coordinates,
            grid,
            pack,
            biomes,
            notes,
            cultures,
            states,
            burgs,
            religions,
            provinces,
            rivers,
            markers,
            routes,
            zones,
            ice,
            namebases,
            measurers,
            fonts,
            custom_good_icons,
            goods,
            markets,
            deals,
        };

        Ok(LoadResult {
            world,
            pack_new_points_f64: new_points_f64,
        })
    }
}

/// Traces each river's `cell_path` following the downhill flow.
///
/// For each river with id > 0, it starts at `source_cell` and walks adjacent
/// cells with the same `river_id` and decreasing height until reaching `mouth_cell`.
fn trace_river_paths(
    rivers: &mut [vor_core::entities::river::River],
    pack_river: &[u16],
    adjacency: &[Vec<u32>],
    height: &[u8],
) {
    for river in rivers.iter_mut() {
        if river.id == 0 {
            continue;
        }
        // A `.map` serializes each river's full `cells` path in slot [32], including the
        // water cell at the mouth. Trust that ground truth when present.
        if !river.cell_path.is_empty() {
            continue;
        }
        let rid = river.id;
        let source = river.source_cell as usize;
        let mouth = river.mouth_cell as usize;
        let mut path = Vec::new();
        let mut current = source;
        // Safety valve: max iterations = all pack cells.
        let max_steps = pack_river.len();
        for _ in 0..max_steps {
            path.push(current as u32);
            if current == mouth {
                break;
            }
            let current_h = height.get(current).copied().unwrap_or(0);
            // Finds the neighbor with the lowest height that also has this river_id
            let next = adjacency.get(current).and_then(|neighbors| {
                neighbors
                    .iter()
                    .filter(|&&n| {
                        let n = n as usize;
                        n < pack_river.len()
                            && pack_river[n] == rid
                            && height.get(n).copied().unwrap_or(0) < current_h
                    })
                    .min_by_key(|&&n| height.get(n as usize).copied().unwrap_or(0))
                    .copied()
            });
            match next {
                Some(n) => current = n as usize,
                None => break, // dead end, the path is not completed
            }
        }
        river.cell_path = path;
    }
}

fn parse_json_opaque(slot: Option<&str>) -> serde_json::Value {
    match slot {
        Some(s) if !s.is_empty() => serde_json::from_str(s).unwrap_or(serde_json::Value::Null),
        _ => serde_json::Value::Null,
    }
}

fn parse_string_opaque(slot: Option<&str>) -> serde_json::Value {
    match slot {
        Some(s) if !s.is_empty() => serde_json::Value::String(s.to_string()),
        _ => serde_json::Value::Null,
    }
}

/// Converts `vor_import::geometry::voronoi::Voronoi` → `vor_core::VoronoiVertices`
/// (dumps `vertices.p/v/c` + `cells.v` as `cell_rings` so the renderer does not
/// recompute meshes).
fn voronoi_to_vor_core(v: &Voronoi) -> vor_core::voronoi::VoronoiVertices {
    vor_core::voronoi::VoronoiVertices {
        positions: v
            .vertices
            .p
            .iter()
            .map(|p| [p[0] as f32, p[1] as f32])
            .collect(),
        adjacent_cells: v
            .vertices
            .c
            .iter()
            .map(|c| [c[0] as i32, c[1] as i32, c[2] as i32])
            .collect(),
        adjacent_vertices: v
            .vertices
            .v
            .iter()
            .map(|v| [v[0] as i32, v[1] as i32, v[2] as i32])
            .collect(),
        cell_rings: v.cells.v.clone(),
    }
}

impl crate::mapfile::cells::GridGeneral {
    fn cells_desired(&self) -> u32 {
        if self.cellsDesired == 0 {
            self.cellsX * self.cellsY
        } else {
            self.cellsDesired
        }
    }
}
