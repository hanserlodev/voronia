//! Full handshake test with the real Sorvik `.map` (azgaar.github.io v1.138.0,
//! 24 Jul 2026). Loads the file end-to-end with `vor_import::mapfile::Loader::load`
//! and validates structural invariants to confirm that the parser + geometry
//! regeneration + mapping to strong types works against real (non-synthetic) data.
//!
//! Expected invariants (extracted via python over the same file):
//! - 47 raw slots.
//! - 10000 grid cells (slot [6].points count == 100×100).
//! - 7268 pack cells (slot [16] biome count == re_graph output).
//! - 19 features (slot [12]).
//! - 16 cultures (slot [13]).
//! - 14 states (slot [14]).
//! - 1010 burgs (slot [15]).
//! - 24 religions (slot [29]).
//! - 226 provinces (slot [30]).
//! - 141 rivers (slot [32]).
//! - 815 routes (slot [37]).
//! - 13 zones (slot [38]).
//! - 4 ice groups (slot [39]).
//! - 83 markers (slot [35]).
//! - 1 measurer (slot [46]).

use vor_import::mapfile::{raw, Loader};

const SORVIK_MAP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/reference/Sorvik-2026-07-24-23-39.map"
);

fn load_sorvik() -> vor_import::mapfile::LoadResult {
    let bytes = std::fs::read(SORVIK_MAP_PATH)
        .expect("Sorvik.map must exist in crates/vor-import/tests/reference/");
    let raw = raw::parse(&bytes).expect("raw parse");
    Loader::load(&raw).expect("loader succeeds on Sorvik")
}

#[test]
fn sorvik_raw_has_47_slots() {
    let bytes = std::fs::read(SORVIK_MAP_PATH).unwrap();
    let raw = raw::parse(&bytes).unwrap();
    assert_eq!(raw.slots.len(), 47, "Sorvik raw slot count");
}

#[test]
fn sorvik_header_fields_match() {
    let w = load_sorvik();
    assert_eq!(w.world.header.version, "1.138.0");
    assert_eq!(w.world.header.seed, "279321909");
    assert_eq!(w.world.header.graph_width, 937);
    assert_eq!(w.world.header.graph_height, 945);
    assert_eq!(w.world.header.map_id, 1784954343635);
}

#[test]
fn sorvik_settings_fields_match() {
    let w = load_sorvik();
    assert_eq!(w.world.settings.distance_unit, "km");
    assert_eq!(w.world.settings.distance_scale, 2.0);
    assert_eq!(w.world.settings.height_exponent, 2);
    assert_eq!(w.world.settings.population_rate, 1000.0);
    assert_eq!(w.world.settings.urbanization, 1.0);
    assert_eq!(w.world.settings.map_name, "Sorvik");
    assert!(
        w.world.settings.options.get("mapSize").is_some(),
        "options.mapSize must be present"
    );
}

#[test]
fn sorvik_coordinates_have_lat_lon() {
    let w = load_sorvik();
    assert!((w.world.coordinates.lat_t - 54.0).abs() < 1e-3, "latT");
    assert!((w.world.coordinates.lat_n - 44.6).abs() < 1e-3, "latN");
    assert!((w.world.coordinates.lat_s - (-9.4)).abs() < 1e-3, "latS");
    assert!((w.world.coordinates.lon_l - (-26.7)).abs() < 1e-3, "lonW");
    assert!((w.world.coordinates.lon_r - 26.8).abs() < 1e-3, "lonE");
}

#[test]
fn sorvik_grid_has_10000_cells() {
    let w = load_sorvik();
    assert_eq!(w.world.grid.points.len(), 10000, "grid points count");
    assert_eq!(w.world.grid.cells_x, 100, "cellsX");
    assert_eq!(w.world.grid.cells_y, 100, "cellsY");
    assert_eq!(w.world.grid.cells.height.len(), 10000, "h length");
    assert_eq!(w.world.grid.cells.precipitation.len(), 10000, "prec length");
    assert_eq!(w.world.grid.cells.feature_id.len(), 10000, "f length");
    assert_eq!(w.world.grid.cells.water_type.len(), 10000, "t length");
    assert_eq!(w.world.grid.cells.temperature.len(), 10000, "temp length");
}

#[test]
fn sorvik_grid_features_match_count() {
    // Slot [6] brings `grid.features` embedded: 25 entries (placeholder `0` + 24 real
    // features with ids 1..24). The loader must populate `world.grid.features` with 24 items.
    let w = load_sorvik();
    assert_eq!(
        w.world.grid.features.len(),
        24,
        "grid.features count (placeholder skipped)"
    );
    // Slot [12] brings `pack.features`: 19 entries (placeholder `0` + 18 real).
    assert_eq!(
        w.world.pack.features.len(),
        18,
        "pack.features count (placeholder skipped)"
    );
}

#[test]
fn sorvik_pack_has_7268_cells() {
    let w = load_sorvik();
    assert_eq!(
        w.world.pack.points.len(),
        7268,
        "pack.points count (re_graph)"
    );
    assert_eq!(w.world.pack.cells.grid_id.len(), 7268, "grid_id count");
    assert_eq!(w.world.pack.cells.height.len(), 7268, "height count");
    assert_eq!(w.world.pack.cells.area_px.len(), 7268, "area_px count");
    assert_eq!(w.world.pack.cells.biome.len(), 7268, "biome count");
    assert_eq!(w.world.pack.cells.state.len(), 7268, "state count");
}

#[test]
fn sorvik_catalogs_match_counts() {
    let w = load_sorvik();
    assert_eq!(w.world.biomes.len(), 13, "biome catalog");
    assert_eq!(
        w.world.pack.features.len(),
        18,
        "features (slot[0] placeholder skipped)"
    );
    assert_eq!(
        w.world.cultures.len(),
        15,
        "cultures (slot[0] placeholder skipped)"
    );
    assert_eq!(
        w.world.states.len(),
        13,
        "states (slot[0] placeholder skipped)"
    );
    assert_eq!(
        w.world.burgs.len(),
        1009,
        "burgs (slot[0] placeholder skipped)"
    );
    assert_eq!(w.world.religions.len(), 24, "religions");
    assert_eq!(
        w.world.provinces.len(),
        225,
        "provinces (slot[0] placeholder skipped)"
    );
    assert_eq!(w.world.rivers.len(), 141, "rivers");
    assert_eq!(w.world.routes.len(), 815, "routes");
    assert_eq!(w.world.zones.len(), 13, "zones");
    assert_eq!(w.world.ice.len(), 4, "ice");
    assert_eq!(w.world.markers.len(), 83, "markers");
    assert_eq!(w.world.measurers.len(), 1, "measurers");
}
