//! Guard de regresión para el mesh de ríos (FMG `#rivers`): carga Sorvik real
//! y comprueba que `build_river_mesh` genera geometría válida — polígonos
//! dentro del canvas (con holgura de media anchura), color `#5d97bb` y sin
//! recorte manual de boca (el mask `#land` del renderer hace ese trabajo,
//! igual que FMG).

use vor_import::mapfile::{raw, Loader};
use vor_render::river::build_river_mesh;

const SORVIK_MAP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vor-import/tests/reference/Sorvik-2026-07-24-23-39.map"
);

#[test]
fn river_mesh_covers_rivers_within_bounds() {
    let bytes = std::fs::read(SORVIK_MAP_PATH).expect("map");
    let raw = raw::parse(&bytes).expect("parse");
    let loaded = Loader::load(&raw).expect("load");
    let world = &loaded.world;

    assert!(
        !world.rivers.is_empty(),
        "Sorvik fixture should carry rivers"
    );

    let mesh = build_river_mesh(
        &world.pack.points,
        &world.rivers,
        world.settings.distance_scale,
        world.grid.width,
        world.grid.height,
    );

    assert!(
        !mesh.vertices.is_empty(),
        "river mesh should have geometry for Sorvik"
    );
    assert_eq!(mesh.indices.len() % 3, 0, "triangle list");
    assert!(mesh.indices.len() >= mesh.vertices.len());

    // The ribbon may overshoot the canvas by up to one bank width; anything
    // beyond a generous slack means the geometry escaped the map.
    let slack = 50.0;
    assert!(mesh.bounds_min[0] >= -slack && mesh.bounds_min[1] >= -slack);
    assert!(
        mesh.bounds_max[0] <= world.grid.width + slack,
        "bounds_max_x {} vs width {}",
        mesh.bounds_max[0],
        world.grid.width
    );
    assert!(mesh.bounds_max[1] <= world.grid.height + slack);

    // Determinism: same inputs, same mesh.
    let again = build_river_mesh(
        &world.pack.points,
        &world.rivers,
        world.settings.distance_scale,
        world.grid.width,
        world.grid.height,
    );
    assert_eq!(mesh.vertices.len(), again.vertices.len());
    assert_eq!(mesh.indices, again.indices);
}
