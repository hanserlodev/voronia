//! Test bit-exacto: `Voronoi` class de Azgaar (`voronoi.ts`) reproducida en Rust
//! (`vor_import::geometry::voronoi::calculate_voronoi`) sobre el mismo input de
//! `placePoints(2000, 2000, 10000, "861039636") + boundary` + `delaunator::from_pairs`.
//!
//! El fixture (`voronoi_grid_2000x2000_c10k_seed_861039636_selfref.json`) fue generado por
//! `generate_voronoi_fixture.js` desde este dir, replicando la `Voronoi` class de Azgaar
//! en vanilla JS (sin TS deps) sobre el mismo `Delaunator.from(allPoints)` del
//! `delaunator@5.1.0`. El test Rust reproduce todo y compara `cells.v/c/b` + `vertices.p/v/c`
//! bit-a-bit contra el fixture.
//!
//! Crítico: `circumcenter` (`voronoi.ts:142-154`) trunca a enteros con `Math.floor`. Si el
//! porte Rust no hace exactamente `f64::floor()`, los `vertices.p[t]` divergen — bug silencioso
//! en la geometría que rompería el mapeo slot→cell (fase-0 §6.3, §13.4).

use serde_json::Value;
use std::fs;
use vor_import::geometry::delaunay::{from_pairs, EMPTY};
use vor_import::geometry::place_points;
use vor_import::geometry::voronoi::calculate_voronoi;

fn load_fixture() -> Value {
    let path = "tests/reference/voronoi_grid_2000x2000_c10k_seed_861039636_selfref.json";
    let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("no se pudo leer {path}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("JSON inválido: {e}"))
}

/// Strings decimalizados de i64 (bits de f64, BigInt64Array) → `Vec<f64>`.
fn parse_f64_bits_strings(arr: &Value, field: &str) -> Vec<f64> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` debería ser array"))
        .iter()
        .map(|x| {
            let signed: i64 = x
                .as_str()
                .unwrap_or_else(|| panic!("`{field}` entry debería ser string decimal"))
                .parse()
                .unwrap_or_else(|e| panic!("`{field}` bits inválidos: {e}"));
            f64::from_bits(signed as u64)
        })
        .collect()
}

/// Esperado: `field` es un array de arrays de u32-strings (cells.v, cells.c).
/// Algunas entries pueden ser `null` (celda no visitada). Devuelve `Vec<Option<Vec<u32>>>`.
fn parse_nested_u32_opt(arr: &Value, field: &str) -> Vec<Option<Vec<u32>>> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` debería ser array"))
        .iter()
        .map(|x| {
            if x.is_null() {
                None
            } else {
                Some(
                    x.as_array()
                        .unwrap_or_else(|| panic!("`{field}` entry debería ser array o null"))
                        .iter()
                        .map(|s| {
                            s.as_str()
                                .unwrap_or_else(|| {
                                    panic!("`{field}` entry entry debería ser string")
                                })
                                .parse::<u32>()
                                .unwrap_or_else(|e| panic!("`{field}` u32 inválido: {e}"))
                        })
                        .collect(),
                )
            }
        })
        .collect()
}

/// `field` es un array de strings decimalizados de u8 (cells.b — "0" o "1"). Algunos pueden
/// ser null. Devuelve `Vec<Option<u8>>`.
fn parse_u8_opt_strings(arr: &Value, field: &str) -> Vec<Option<u8>> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` debería ser array"))
        .iter()
        .map(|x| {
            if x.is_null() {
                None
            } else {
                Some(
                    x.as_str()
                        .unwrap_or_else(|| panic!("`{field}` entry debería ser string o null"))
                        .parse::<u8>()
                        .unwrap_or_else(|e| panic!("`{field}` u8 inválido: {e}")),
                )
            }
        })
        .collect()
}

/// `field` es array de arrays de 3 i32-strings (vertices.v — -1 = EMPTY).
fn parse_nested_i32(arr: &Value, field: &str) -> Vec<[i64; 3]> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` debería ser array"))
        .iter()
        .map(|x| {
            let arr = x
                .as_array()
                .unwrap_or_else(|| panic!("`{field}` entry debería ser array"));
            assert_eq!(arr.len(), 3, "`{field}` entry debería tener 3 elementos");
            let mut out = [0i64; 3];
            for (i, s) in arr.iter().enumerate() {
                out[i] = s
                    .as_str()
                    .unwrap_or_else(|| panic!("`{field}` entry entry debería ser string"))
                    .parse::<i64>()
                    .unwrap_or_else(|e| panic!("`{field}` i32 inválido: {e}"));
            }
            out
        })
        .collect()
}

/// `field` es array de arrays de 3 u32-strings (vertices.c).
fn parse_nested_u32(arr: &Value, field: &str) -> Vec<[u32; 3]> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` debería ser array"))
        .iter()
        .map(|x| {
            let arr = x
                .as_array()
                .unwrap_or_else(|| panic!("`{field}` entry debería ser array"));
            assert_eq!(arr.len(), 3, "`{field}` entry debería tener 3 elementos");
            let mut out = [0u32; 3];
            for (i, s) in arr.iter().enumerate() {
                out[i] = s
                    .as_str()
                    .unwrap_or_else(|| panic!("`{field}` entry entry debería ser string"))
                    .parse::<u32>()
                    .unwrap_or_else(|e| panic!("`{field}` u32 inválido: {e}"));
            }
            out
        })
        .collect()
}

#[test]
fn voronoi_rust_matches_js_self_reference() {
    let v = load_fixture();

    // Sanity handshake: spacing, counts, nPoints, nBoundary.
    let spacing = v["spacing"].as_f64().expect("spacing");
    assert!(
        (spacing - 20.0).abs() < 1e-9,
        "spacing calza Brample: {spacing}"
    );
    let n_points = v["nPoints"].as_u64().expect("nPoints") as usize;
    let n_boundary = v["nBoundary"].as_u64().expect("nBoundary") as usize;
    let n_all = v["nAllPoints"].as_u64().expect("nAllPoints") as usize;
    let n_triangles = v["nTriangles"].as_u64().expect("nTriangles") as usize;
    assert_eq!(n_points, 10000, "10000 puntos jitterizados");
    assert_eq!(n_boundary, 200, "200 boundary points");
    assert_eq!(n_all, 10200, "allPoints = points + boundary");
    assert_eq!(n_triangles, 20198, "20198 triángulos Delaunay");

    // Reproducir `placePoints` desde el Rust (validado bit-exacto en grid_bit_exact.rs).
    let placed = place_points(2000.0, 2000.0, 10000, "861039636");
    assert_eq!(placed.points.len(), n_points);
    assert_eq!(placed.boundary.len(), n_boundary);

    // Concatenar `allPoints = points + boundary` (igual que Azgaar en `calculateVoronoi`).
    let mut all_points: Vec<[f64; 2]> = placed.points.clone();
    all_points.extend(placed.boundary.iter().cloned());
    assert_eq!(all_points.len(), n_all);

    // Triangulación (validada bit-exacta en delaunay_bit_exact.rs).
    let delaunay = from_pairs(&all_points);
    assert_eq!(delaunay.triangles.len() / 3, n_triangles);
    assert_eq!(delaunay.triangles.len(), 60594);
    assert_eq!(delaunay.halfedges.len(), 60594);
    assert_eq!(delaunay.hull.len(), 200);

    // Calcular Voronoi (porte Rust).
    let voronoi = calculate_voronoi(&delaunay, &all_points, n_points as u32);

    // === Validar cells.v ===
    // En el porte Rust, las celdas no-visitadas se codifican como `Vec::new()` (vacías),
    // mientras que el JS las deja como `undefined` (serializado como `null`). El fixture
    // `cells_v` tiene `null` para esas. En el porte, todos los puntos interiores (id < 10000)
    // DEBEN ser visitados (boundary points tienen id >= 10000 y no entran en `cells_c`).
    // Por eso el fixture debería tener 10000 entradas no-null en cells.v.
    // Verificamos: parseamos las entradas del fixture y comparamos con el Rust.
    let cells_v_json = &v["cells_v"];
    let cells_v_expected = parse_nested_u32_opt(cells_v_json, "cells_v");
    assert_eq!(
        cells_v_expected.len(),
        n_points,
        "cells_v tiene 10000 entradas (una por punto interior)"
    );

    // Recuento de celdas pobladas en el fixture (validación de invariantes).
    let populated_in_fixture = cells_v_expected.iter().filter(|x| x.is_some()).count();
    assert_eq!(
        populated_in_fixture, n_points,
        "todos los {n_points} puntos interiores deberían tener celda poblada (fixture)"
    );

    // Comparar bit-a-bit: el fixture tiene `Some(arr)`, el Rust tiene `Vec<u32>` poblado.
    let mut mismatches_v = 0usize;
    for (p, opt) in cells_v_expected.iter().enumerate() {
        let expected = opt
            .as_ref()
            .unwrap_or_else(|| panic!("celda {p} debería estar poblada"));
        let got = &voronoi.cells.v[p];
        assert!(
            !got.is_empty(),
            "celda {p} está vacía en Rust pero llena en fixture"
        );
        if got.len() != expected.len() {
            mismatches_v += 1;
            if mismatches_v <= 3 {
                eprintln!(
                    "cells.v[{p}]: lens divergen (Rust={}, JS={})",
                    got.len(),
                    expected.len()
                );
            }
            continue;
        }
        for (i, (g, e)) in got.iter().zip(expected.iter()).enumerate() {
            if g != e {
                mismatches_v += 1;
                if mismatches_v <= 3 {
                    eprintln!("cells.v[{p}][{i}]: Rust={g} JS={e}");
                }
                break;
            }
        }
    }
    assert_eq!(mismatches_v, 0, "{} mismatches en cells.v", mismatches_v);

    // === Validar cells.c ===
    let cells_c_expected = parse_nested_u32_opt(&v["cells_c"], "cells_c");
    assert_eq!(cells_c_expected.len(), n_points);

    let mut mismatches_c = 0usize;
    for (p, opt) in cells_c_expected.iter().enumerate() {
        let expected = opt.as_ref().expect("celda {p} debería estar poblada");
        let got = &voronoi.cells.c[p];
        assert!(
            !got.is_empty(),
            "cells.c[{p}] está vacía en Rust pero llena en fixture"
        );
        if got.len() != expected.len() {
            mismatches_c += 1;
            if mismatches_c <= 3 {
                eprintln!(
                    "cells.c[{p}]: lens divergen (Rust={}, JS={})",
                    got.len(),
                    expected.len()
                );
            }
            continue;
        }
        for (i, (g, e)) in got.iter().zip(expected.iter()).enumerate() {
            if g != e {
                mismatches_c += 1;
                if mismatches_c <= 3 {
                    eprintln!("cells.c[{p}][{i}]: Rust={g} JS={e}");
                }
                break;
            }
        }
    }
    assert_eq!(mismatches_c, 0, "{} mismatches en cells.c", mismatches_c);

    // === Validar cells.b ===
    let cells_b_json = &v["cells_b"];
    let cells_b_expected = parse_u8_opt_strings(cells_b_json, "cells_b");
    assert_eq!(cells_b_expected.len(), n_points);

    let mut mismatches_b = 0usize;
    for (p, opt) in cells_b_expected.iter().enumerate() {
        let expected = opt.unwrap_or_else(|| panic!("cells.b[{p}] debería estar seteado"));
        let got = voronoi.cells.b[p];
        if got != expected {
            mismatches_b += 1;
            if mismatches_b <= 5 {
                eprintln!("cells.b[{p}]: Rust={got} JS={expected}");
            }
        }
    }
    assert_eq!(mismatches_b, 0, "{} mismatches en cells.b", mismatches_b);

    // === Validar vertices.p (coords bit-exactas con Math.floor) ===
    let vertices_p_bits = &v["vertices_p_bits"];
    let vertices_p_flat = parse_f64_bits_strings(vertices_p_bits, "vertices_p_bits");
    assert_eq!(
        vertices_p_flat.len(),
        n_triangles * 2,
        "vertices.p_bits deberían ser nTriangles*2 entradas f64"
    );

    let mut mismatches_p = 0usize;
    for t in 0..n_triangles {
        let expected_x = vertices_p_flat[2 * t];
        let expected_y = vertices_p_flat[2 * t + 1];
        let got = &voronoi.vertices.p[t];
        if (got[0] - expected_x).abs() > 0.0 || (got[1] - expected_y).abs() > 0.0 {
            mismatches_p += 1;
            if mismatches_p <= 3 {
                eprintln!(
                    "vertices.p[{t}]: Rust={:?} JS=[{expected_x}, {expected_y}]",
                    got
                );
            }
        }
    }
    assert_eq!(mismatches_p, 0, "{} mismatches en vertices.p", mismatches_p);

    // === Validar vertices.v (3 triangle ids por vértice, -1 = EMPTY) ===
    let vertices_v_expected = parse_nested_i32(&v["vertices_v"], "vertices_v");
    assert_eq!(vertices_v_expected.len(), n_triangles);

    let mut mismatches_vv = 0usize;
    for (t, expected) in vertices_v_expected.iter().enumerate() {
        let got = &voronoi.vertices.v[t];
        for (i, (g, e)) in got.iter().zip(expected.iter()).enumerate() {
            // JS: -1 = EMPTY (Math.floor(-1/3) = -1 — JS tiene truncation hacia cero, no floor; pero
            // floor(-1/3) = -1 también).
            // Rust: nosotros guardamos `EMPTY = usize::MAX` (= 4294967295 u32) que serializamos
            // como `usize`. Para comparar con el JS, mapeamos EMPTY → -1.
            let g_i64 = if *g == EMPTY { -1i64 } else { *g as i64 };
            if g_i64 != *e {
                mismatches_vv += 1;
                if mismatches_vv <= 3 {
                    eprintln!("vertices.v[{t}][{i}]: Rust={g_i64} JS={e} (raw Rust usize={g})");
                }
                break;
            }
        }
    }
    assert_eq!(
        mismatches_vv, 0,
        "{} mismatches en vertices.v",
        mismatches_vv
    );

    // === Validar vertices.c (3 u32 cell ids por vértice) ===
    let vertices_c_expected = parse_nested_u32(&v["vertices_c"], "vertices_c");
    assert_eq!(vertices_c_expected.len(), n_triangles);

    let mut mismatches_vc = 0usize;
    for (t, expected) in vertices_c_expected.iter().enumerate() {
        let got = &voronoi.vertices.c[t];
        if got != expected {
            mismatches_vc += 1;
            if mismatches_vc <= 3 {
                eprintln!("vertices.c[{t}]: Rust={got:?} JS={expected:?}");
            }
        }
    }
    assert_eq!(
        mismatches_vc, 0,
        "{} mismatches en vertices.c",
        mismatches_vc
    );

    // Test pasa. Resumen del éxito en stderr paraHumano-readable.
    eprintln!(
        "[voronoi bit-exact] OK: {} cells, {} triangles — 0 mismatches en cells.v/c/b y vertices.p/v/c",
        n_points, n_triangles
    );
}
