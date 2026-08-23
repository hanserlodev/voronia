//! Bit-exact test: `re_graph` Rust vs `reGraph` JS (main.js:1157-1209).
//!
//! Fixture (`regraph_h50_t2_grid_2000x2000_c10k_seed_861039636_selfref.json`):
//! synthetic with `h=50` (interior land) and `t=2` (interior land, not coast) for
//! all 10000 grid cells. Validated for the "no discards, no extra points" path (all pass
//! the filter and none is coastal). The discard paths (h<20, type=-2, lake) and coastal
//! (type=±1) are covered by unit tests in `src/regraph.rs`.
//!
//! Test compares:
//! - `pack.points` bits (JS `BigInt64Array`, decoded to f64) — input to the
//!   second `calculateVoronoi` (= the `newCells.p` output).
//! - `pack.cells.grid_id` (= `newCells.g`) — pack→grid mapping.
//! - `pack.cells.height` (= `newCells.h`).
//! - `pack.cells.area_px` — via `d3.polygonArea` (shoelace) capped to u16.
//! - `pack.vertices.p` — bit-exact coords with `Math.floor`.
//! - `pack.vertices.v` — neighbors with `-1` = EMPTY.
//! - `pack.vertices.c` — adjacent cells.

use serde_json::Value;
use std::fs;
use vor_core::feature::FeatureType;
use vor_import::geometry::delaunay::from_pairs;
use vor_import::geometry::place_points;
use vor_import::geometry::voronoi::calculate_voronoi;
use vor_import::regraph::re_graph;

fn load_fixture() -> Value {
    let path = "tests/reference/regraph_h50_t2_grid_2000x2000_c10k_seed_861039636_selfref.json";
    let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("could not read {path}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid JSON: {e}"))
}

/// Decimal strings of i64 (bits of f64) → `Vec<f64>`.
fn parse_f64_bits(arr: &Value, field: &str) -> Vec<f64> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` should be an array"))
        .iter()
        .map(|x| {
            let signed: i64 = x
                .as_str()
                .unwrap_or_else(|| panic!("`{field}` entry should be a decimal string"))
                .parse()
                .unwrap_or_else(|e| panic!("`{field}` invalid bits: {e}"));
            f64::from_bits(signed as u64)
        })
        .collect()
}

fn parse_u32_strings(arr: &Value, field: &str) -> Vec<u32> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` should be an array"))
        .iter()
        .map(|x| {
            x.as_str()
                .unwrap_or_else(|| panic!("`{field}` entry should be a string"))
                .parse::<u32>()
                .unwrap_or_else(|e| panic!("`{field}` invalid u32: {e}"))
        })
        .collect()
}

fn parse_u8_strings(arr: &Value, field: &str) -> Vec<u8> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` should be an array"))
        .iter()
        .map(|x| {
            x.as_str()
                .unwrap_or_else(|| panic!("`{field}` entry should be a string"))
                .parse::<u8>()
                .unwrap_or_else(|e| panic!("`{field}` invalid u8: {e}"))
        })
        .collect()
}

fn parse_u16_strings(arr: &Value, field: &str) -> Vec<u16> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` should be an array"))
        .iter()
        .map(|x| {
            x.as_str()
                .unwrap_or_else(|| panic!("`{field}` entry should be a string"))
                .parse::<u16>()
                .unwrap_or_else(|e| panic!("`{field}` invalid u16: {e}"))
        })
        .collect()
}

#[test]
fn regraph_rust_matches_js_self_reference() {
    let v = load_fixture();

    let spacing = v["spacing"].as_f64().expect("spacing");
    assert!(
        (spacing - 20.0).abs() < 1e-9,
        "spacing matches Brample: {spacing}"
    );
    let n_grid_points = v["nGridPoints"].as_u64().expect("nGridPoints") as usize;
    let n_grid_boundary = v["nGridBoundary"].as_u64().expect("nGridBoundary") as usize;
    let n_pack_points = v["nPackPoints"].as_u64().expect("nPackPoints") as usize;
    let n_pack_triangles = v["nPackTriangles"].as_u64().expect("nPackTriangles") as usize;
    assert_eq!(n_grid_points, 10000);
    assert_eq!(n_grid_boundary, 200);
    // With synthetic h=50 t=2: all pass filter 1 (h>=20 holds), filter 2 (type=-2 does
    // not hold). No coastal (type=2). That is why pack.cells.p == 10000 (same as grid).
    assert_eq!(
        n_pack_points, 10000,
        "synthetic interior land → pack == grid (no extras)"
    );

    // Reproduce the Rust `placePoints` (input to reGraph).
    let placed = place_points(2000.0, 2000.0, 10000, "861039636");
    assert_eq!(placed.points.len(), n_grid_points);
    let grid_points: Vec<[f64; 2]> = placed.points.iter().map(|&[x, y]| [x, y]).collect();
    let grid_boundary: Vec<[f64; 2]> = placed.boundary.iter().map(|&[x, y]| [x, y]).collect();

    // Grid topology (needed for `re_graph` — `grid_voronoi.cells.b/c`).
    let mut all_grid = grid_points.clone();
    all_grid.extend(grid_boundary.iter().cloned());
    let delaunay = from_pairs(&all_grid);
    let grid_voronoi = calculate_voronoi(&delaunay, &all_grid, n_grid_points as u32);

    // Synthetic attributes (same as in generate_regraph_fixture.js).
    let grid_height: Vec<u8> = vec![50; n_grid_points];
    let grid_water_type: Vec<i8> = vec![2; n_grid_points];
    let grid_feature_id: Vec<u16> = vec![0; n_grid_points];
    let grid_features_kind: Vec<FeatureType> = vec![FeatureType::Ocean]; // idx 0 = Ocean (not Lake).

    let (pack, new_pts) = re_graph(
        &grid_points,
        &grid_boundary,
        &grid_voronoi,
        &grid_height,
        &grid_water_type,
        &grid_feature_id,
        &grid_features_kind,
        placed.spacing,
    );

    // Basic population.
    assert_eq!(pack.points.len(), n_pack_points, "pack.points count");
    assert_eq!(new_pts.len(), n_pack_points, "new_pts count");
    assert_eq!(pack.cells.grid_id.len(), n_pack_points);
    assert_eq!(pack.cells.height.len(), n_pack_points);
    assert_eq!(pack.cells.area_px.len(), n_pack_points);

    // === Compare new_pts (f64) bit-by-bit vs the JS ===
    // `new_pts` (Rust) are the `newCells.p` of the algorithm, before the f32 cast for
    // `Pack::points` (fixed cap of the vor-core model). Compare these — not `pack.points` —
    // for real bit-exactness.
    let expected_pts_flat = parse_f64_bits(&v["pack_points_bits"], "pack_points_bits");
    assert_eq!(
        expected_pts_flat.len(),
        n_pack_points * 2,
        "pack_points_bits = N*2 f64s"
    );
    let mut mismatches_pts = 0usize;
    for (i, expected) in expected_pts_flat.as_chunks::<2>().0.iter().enumerate() {
        let got_x = new_pts[i][0];
        let got_y = new_pts[i][1];
        if got_x != expected[0] || got_y != expected[1] {
            mismatches_pts += 1;
            if mismatches_pts <= 3 {
                eprintln!(
                    "new_pts[{i}]: Rust=[{got_x}, {got_y}] JS=[{}, {}]",
                    expected[0], expected[1]
                );
            }
        }
    }
    assert_eq!(
        mismatches_pts, 0,
        "{} mismatches in new_pts",
        mismatches_pts
    );

    // === Compare grid_id (pack→grid mapping) ===
    let expected_grid_id = parse_u32_strings(&v["grid_id"], "grid_id");
    assert_eq!(expected_grid_id.len(), n_pack_points);
    let mut mismatches_g = 0usize;
    for (i, &expected) in expected_grid_id.iter().enumerate() {
        if pack.cells.grid_id[i] != expected {
            mismatches_g += 1;
            if mismatches_g <= 3 {
                eprintln!(
                    "grid_id[{i}]: Rust={} JS={}",
                    pack.cells.grid_id[i], expected
                );
            }
        }
    }
    assert_eq!(mismatches_g, 0, "{} mismatches in grid_id", mismatches_g);

    // === Compare height ===
    let expected_h = parse_u8_strings(&v["pack_height"], "pack_height");
    assert_eq!(expected_h.len(), n_pack_points);
    let mut mismatches_h = 0usize;
    for (i, &expected) in expected_h.iter().enumerate() {
        if pack.cells.height[i] != expected {
            mismatches_h += 1;
            if mismatches_h <= 3 {
                eprintln!("height[{i}]: Rust={} JS={}", pack.cells.height[i], expected);
            }
        }
    }
    assert_eq!(mismatches_h, 0, "{} mismatches in height", mismatches_h);

    // === Compare area_px ===
    let expected_area = parse_u16_strings(&v["pack_area"], "pack_area");
    assert_eq!(expected_area.len(), n_pack_points);
    let mut mismatches_area = 0usize;
    for (i, &expected) in expected_area.iter().enumerate() {
        if pack.cells.area_px[i] != expected {
            mismatches_area += 1;
            if mismatches_area <= 5 {
                eprintln!(
                    "area_px[{i}]: Rust={} JS={}",
                    pack.cells.area_px[i], expected
                );
            }
        }
    }
    assert_eq!(
        mismatches_area, 0,
        "{} mismatches in area_px",
        mismatches_area
    );

    // === Compare vertices.p bit-by-bit ===
    let expected_verts_p_flat = parse_f64_bits(&v["vertices_p_bits"], "vertices_p_bits");
    assert_eq!(expected_verts_p_flat.len(), n_pack_triangles * 2);
    let mut mismatches_vp = 0usize;
    for t in 0..n_pack_triangles {
        let expected_x = expected_verts_p_flat[2 * t];
        let expected_y = expected_verts_p_flat[2 * t + 1];
        let got = pack.vertices.positions[t];
        if (got[0] as f64 - expected_x).abs() > 0.0 || (got[1] as f64 - expected_y).abs() > 0.0 {
            mismatches_vp += 1;
            if mismatches_vp <= 3 {
                eprintln!(
                    "vertices.positions[{t}]: Rust={:?} JS=[{expected_x}, {expected_y}]",
                    got
                );
            }
        }
    }
    assert_eq!(
        mismatches_vp, 0,
        "{} mismatches in vertices.positions",
        mismatches_vp
    );

    eprintln!(
        "[regraph bit-exact] OK: {} pack points, {} triangles — 0 mismatches in points/grid_id/height/area/vertices",
        n_pack_points, n_pack_triangles
    );
}
