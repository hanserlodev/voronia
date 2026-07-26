//! Loader orquestador — combina `raw` + `header` + `cells` + `catalogs` + la
//! regeneración de geometría (`place_points` + `calculate_voronoi` + `re_graph`)
//! para producir un `vor_core::World` completo.
//!
//! Bit-exactitud garantías:
//! 1. Geometría grid: `place_points(seed, graphWidth, graphHeight, cellsDesired)` debe
//!    reproducir bit-exacto el slot `[6].points` del `.map` (handshake test en
//!    `tests/sorvik_handshake.rs`).
//! 2. Voronoi grid: `calculate_voronoi(allPoints, pointsN)` debe reproducir
//!    bit-exacto el `cells.v/c/b` + `vertices.p/v/c` que Azgaar computa.
//! 3. Pack: `re_graph(...)` debe reproducir bit-exacto el `pack.points` count
//!    (Sorvik espera 7268 pack cells tras `re_graph`).
//! 4. Atributos: los slots `[7]`-`[11]` (grid) y `[16]`-`[27]` (pack) Yankees del .map
//!    se aplican 1-a-1 sobre la malla regenerada, con el mismo id mapping
//!    (`pack.cells.g[packId] → gridId`) que Azgaar generó originalmente. Si la malla
//!    del caller diverge de la del header seed, estos atributos caen en celdas
//!    equivocadas — bug silencioso. Acá lo garantimos por handshakes en tests.

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
    #[error("geometría inconsistente: {0}")]
    Geometry(String),
}

/// Resultado de cargar un `.map`: el `World` poblado + los `new_points` del pack en
/// f64 (antes del cast f32 al storage — ver `regraph::re_graph` docstring).
pub struct LoadResult {
    pub world: World,
    pub pack_new_points_f64: Vec<[f64; 2]>,
}

/// Loader API principal.
///
/// Uso:
/// ```ignore
/// let bytes = std::fs::read("Sorvik.map")?;
/// let raw = vor_import::mapfile::raw::parse(&bytes)?;
/// let loaded = vor_import::mapfile::Loader::load(&raw)?;
/// let world = loaded.world;
/// // `loaded.pack_new_points_f64` si se quiere validar bit-exactitud del pack.
/// ```
pub struct Loader;

impl Loader {
    /// Carga un `RawMap` (post `raw::parse`) en un `World` completo.
    pub fn load(raw: &RawMap) -> Result<LoadResult, LoadError> {
        // --- Header ---
        let slot0 = raw.must(0);
        let header = crate::mapfile::header::parse_header(slot0)?;
        let settings = crate::mapfile::header::parse_settings(raw.must(1))?;
        let coordinates = crate::mapfile::header::parse_coordinates(raw.get(2))?;

        // --- Grid general (slot [6] JSON) ---
        let grid_general = crate::mapfile::cells::parse_grid_general(raw.get(6))?;

        // --- Geometría regenerada: place_points ---
        let placed = place_points(
            header.graph_width as f64,
            header.graph_height as f64,
            grid_general.cells_desired(),
            &header.seed,
        );

        // Sanity: el `place_points` debe yield el mismo número de celdas que el slot [6].points.
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

        // --- Slot [7]-[11] → grid.cells (atributos) ---
        let grid_cells: GridCells =
            parse_grid_cells(raw.get(7), raw.get(8), raw.get(9), raw.get(10), raw.get(11));
        // Sanity: GridCells debe tener el mismo largo que `points_n`.
        let n_cells = grid_cells.height.len();
        if n_cells != points_n {
            return Err(LoadError::Geometry(format!(
                "grid.cells h length={} != points count {} — archive mismatch",
                n_cells, points_n
            )));
        }

        // --- reGraph → Pack ---
        // Las features del grid (slot [6].features, no slot [12]) son las que Azgaar usa
        // durante `reGraph` para distinguir lagos de océanos internos (`features[f].type === "lake"`).
        // El slot [12] trae `pack.features` (post-markup), distintas del grid.
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

        // --- PackCells atributos (slots [16]-[27], [36], [40], [44]) ---
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
        // El largo esperado es pack.points.len() (según `re_graph`).
        let expected = pack.points.len();
        // tracks mismatches; emitimos un solo error agregado si los lengths difieren.
        let mut mismatches = Vec::new();
        macro_rules! chk {
            ($field:ident) => {
                if pack_cells.$field.len() != 0 && pack_cells.$field.len() != expected {
                    mismatches.push(format!(
                        "{}: {} (esperado {})",
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

        // Actualizo el `pack.cells` con los atributos parseados + el `grid_id` y
        // `height` y `area_px` que `re_graph` ya pobló en su PackCells interno.
        // NOTA: `pack.cells` fue inicializado por `re_graph` con grid_id/height/area_px
        // ya poblados. Reemplazamos solo los atributos parseados desde .map.
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

        // --- Grid model — la topología Voronoi no se persiste en vor-core::Grid
        // (derivableavana), solo atributos serializados. Mantenemos points/boundary/cells/vertices.
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

        // --- Catálogos (slot [3]/[4]/[12]-[15]/[29]-[46]) ---
        let biomes = crate::mapfile::catalogs::parse_biomes(raw.must(3))?;
        let notes = crate::mapfile::catalogs::parse_notes(raw.get(4))?;
        let features = crate::mapfile::catalogs::parse_features(raw.get(12))?;
        let cultures = crate::mapfile::catalogs::parse_cultures(raw.get(13))?;
        let states = crate::mapfile::catalogs::parse_states(raw.get(14))?;
        let burgs = crate::mapfile::catalogs::parse_burgs(raw.get(15))?;
        let religions = crate::mapfile::catalogs::parse_religions(raw.get(29))?;
        let provinces = crate::mapfile::catalogs::parse_provinces(raw.get(30))?;
        let rivers = crate::mapfile::catalogs::parse_rivers(raw.get(32))?;
        let markers = crate::mapfile::catalogs::parse_markers(raw.get(35))?;
        let routes = crate::mapfile::catalogs::parse_routes(raw.get(37))?;
        let zones = crate::mapfile::catalogs::parse_zones(raw.get(38))?;
        let ice = crate::mapfile::catalogs::parse_ice(raw.get(39))?;
        let measurers = crate::mapfile::catalogs::parse_measurers(raw.get(46))?;
        let namebases = crate::mapfile::catalogs::parse_namebases(raw.get(31))?;

        // Opacos: fonts [34], goods [41]/[42]/[43], custom_good_icons [45]
        let fonts = parse_json_opaque(raw.get(34));
        let goods = parse_json_opaque(raw.get(41));
        let markets = parse_json_opaque(raw.get(42));
        let deals = parse_json_opaque(raw.get(43));
        let custom_good_icons = parse_string_opaque(raw.get(45));

        // --- features van al pack ---
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

/// Convierte `vor_import::geometry::voronoi::Voronoi` → `vor_core::VoronoiVertices`
/// (vuelca `vertices.p/v/c` + `cells.v` como `cell_rings` para que el renderer no
/// recalcule mallas).
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
