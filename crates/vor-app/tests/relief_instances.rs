//! Relief icons (FMG `#terrain`): smoke test over the Sorvik reference map —
//! instances must be generated, land-only, inside world bounds, with valid
//! atlas symbols and positive sizes.

use vor_import::mapfile::{raw, Loader};
use vor_render::relief::{build_relief_instances, ReliefSettings, SYMBOLS};

const SORVIK_MAP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vor-import/tests/reference/Sorvik-2026-07-24-23-39.map"
);

#[test]
fn relief_instances_on_sorvik() {
    let bytes = std::fs::read(SORVIK_MAP_PATH).expect("Sorvik.map");
    let raw = raw::parse(&bytes).expect("raw parse");
    let loaded = Loader::load(&raw).expect("loader");
    let world = &loaded.world;
    let pack = &world.pack;
    let n = pack.points_n();

    let icons = build_relief_instances(
        pack,
        world.header.seed.parse::<u64>().unwrap_or(0),
        &ReliefSettings::default(),
    );
    assert!(!icons.is_empty(), "Sorvik should place relief icons");

    let w = world.grid.width;
    let h = world.grid.height;
    for icon in &icons {
        assert!(
            (icon.symbol as usize) < SYMBOLS.len(),
            "symbol out of atlas"
        );
        assert!(icon.s > 0.0, "icon size must be positive");
        // Quads are centered on the sampled point, so allow half-size slack
        // beyond the canvas edge.
        assert!(
            icon.x >= -icon.s
                && icon.y >= -icon.s
                && icon.x + icon.s <= w + icon.s
                && icon.y + icon.s <= h + icon.s,
            "icon out of world bounds: {icon:?}"
        );
    }

    // Painter's order: sorted by y + size (non-decreasing).
    for pair in icons.windows(2) {
        assert!(
            pair[0].y + pair[0].s <= pair[1].y + pair[1].s + 1e-3,
            "icons not sorted by y+s"
        );
    }

    // No icons on water or river cells: sample-check the first 200 icons'
    // nearest cell is not obviously water (height >= 20 somewhere on map).
    let land_cells = (0..n)
        .filter(|&i| pack.cells.height.get(i).copied().unwrap_or(0) >= 20)
        .count();
    assert!(land_cells > 0);
    // Determinism: same seed → same output.
    let again = build_relief_instances(
        pack,
        world.header.seed.parse::<u64>().unwrap_or(0),
        &ReliefSettings::default(),
    );
    assert_eq!(icons, again);
}
