//! Biosphere parity guards over the Sorvik reference map: biome isolines
//! (FMG `getIsolines` engine), goods burg plates (top-3), and the tessellated
//! route meshes with FMG subgroup styles.

use vor_import::mapfile::{raw, Loader};
use vor_render::{
    biome_colors_from_catalog, build_biome_isolines_meshes, build_goods_burg_plates,
    build_route_group_meshes,
};

const SORVIK_MAP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vor-import/tests/reference/Sorvik-2026-07-24-23-39.map"
);

#[test]
fn biosphere_layers_on_sorvik() {
    let bytes = std::fs::read(SORVIK_MAP_PATH).expect("Sorvik.map");
    let raw = raw::parse(&bytes).expect("raw parse");
    let loaded = Loader::load(&raw).expect("loader");
    let world = &loaded.world;

    // --- Biomes: isoline fill + coastal gap ---
    let colors = biome_colors_from_catalog(&world.biomes);
    let meshes = build_biome_isolines_meshes(&world.pack, &colors);
    assert!(
        !meshes.fill.vertices.is_empty(),
        "biome isoline fill should cover land"
    );
    assert!(
        !meshes.gap.vertices.is_empty(),
        "coastal gap stroke should exist on a map with coasts"
    );
    // Fill is opaque (FMG #biomes has no opacity).
    assert_eq!(meshes.fill.vertices[0].color[3], 1.0);

    // --- Goods burg plates ---
    let burgs_with_production = world
        .burgs
        .iter()
        .filter(|b| !b.production.is_empty())
        .count();
    assert!(
        burgs_with_production > 0,
        "Sorvik burgs should carry production records"
    );
    let (_plates, quads, labels) = build_goods_burg_plates(&world.burgs, &world.goods);
    assert!(!labels.is_empty(), "top-3 plate labels");
    // Some goods may use icons outside the baked atlas (custom icons) —
    // their quad is skipped, so quads <= labels.
    assert!(quads.len() <= labels.len());
    assert!(!quads.is_empty(), "most plate entries have baked symbols");
    // Values are sorted desc per plate; global sanity: all parse as f32 > 0.
    for label in &labels {
        let v: f32 = label.text.parse().expect("numeric value");
        assert!(v > 0.0);
    }

    // --- Routes: three tessellated group meshes ---
    let routes = build_route_group_meshes(&world.routes);
    let total: usize = routes.roads.vertices.len()
        + routes.trails.vertices.len()
        + routes.searoutes.vertices.len();
    assert!(total > 0, "route meshes should have geometry");
    // Roads use #d06324 (linear red-dominant).
    if let Some(v) = routes.roads.vertices.first() {
        assert!(v.color[0] > v.color[2], "roads should be orange-dominant");
    }
    // Searoutes are white.
    if let Some(v) = routes.searoutes.vertices.first() {
        assert!((v.color[0] - v.color[2]).abs() < 1e-6 && v.color[0] > 0.9);
    }
}
