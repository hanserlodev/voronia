//! Test bit-exacto: triangulación Delaunay sobre `allPoints = points.concat(boundary)`
//! usando el crate `delaunator = "1.1"` vs el JS `delaunator@5.1.0` (el que Azgaar usa
//! según `azgaar-fmg/package-lock.json` línea 1599: `resolved: delaunator-5.1.0.tgz`).
//!
//! El fixture (`delaunay_grid_2000x2000_c10k_seed_861039636_selfref.json`) fue generado por
//! `generate_delaunay_fixture.js` desde este dir — es **self-reference**: los `points` y
//! `boundary` se reproducen desde `placePoints` del `azgaar-fmg` actual, y `triangles`/
//! `halfedges` desde `delaunator@5.1.0` JS aplicado sobre `allPoints`. El test Rust corre
//! `vor_import::geometry::place_points` (ya validado bit-exacto en `grid_bit_exact.rs`) +
//! `delaunator::triangulate` (crate Rust) y compara ambos resultados contra el fixture.
//!
//! Si el crate Rust no calza bit-a-bit, hay divergencia en la triangulación y NO debemos
//! usar el crate — el siguiente paso sería portear manualmente desde `delaunator-5.1.0.js`.
//! El `azgaar-fmg` consume `delaunay.triangles` y `delaunay.halfedges` en `voronoi.ts`, así
//! que la bit-exactitud es crítica para que los atributos del `.map` calcen con las celdas
//! correctas (fase-0 §13.4).

use serde_json::Value;
use std::fs;
use vor_import::geometry::delaunay::{from_pairs, EMPTY};
use vor_import::geometry::place_points;

/// Carga el fixture JSON.
fn load_fixture() -> Value {
    let path = "tests/reference/delaunay_grid_2000x2000_c10k_seed_861039636_selfref.json";
    let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("no se pudo leer {path}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("JSON inválido: {e}"))
}

/// Parser de un array de strings decimalizados a `Vec<u32>` (valores de `triangles`).
fn parse_u32_strings(arr: &Value, field: &str) -> Vec<u32> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` debería ser array"))
        .iter()
        .map(|x| {
            x.as_str()
                .unwrap_or_else(|| panic!("`{field}` entry debería ser string decimal"))
                .parse::<u32>()
                .unwrap_or_else(|e| panic!("`{field}` entry inválida: {e}"))
        })
        .collect()
}

/// Parser de un array de strings decimalizados a `Vec<i32>` (valores de `halfedges` — -1 = EMPTY).
fn parse_i32_strings(arr: &Value, field: &str) -> Vec<i32> {
    arr.as_array()
        .unwrap_or_else(|| panic!("`{field}` debería ser array"))
        .iter()
        .map(|x| {
            x.as_str()
                .unwrap_or_else(|| panic!("`{field}` entry debería ser string decimal"))
                .parse::<i32>()
                .unwrap_or_else(|e| panic!("`{field}` entry inválida: {e}"))
        })
        .collect()
}

/// Parser de strings decimalizados de i64 (bits de f64, MSB interpretado como signo por
/// `BigInt64Array` del JS) a `Vec<f64>`. Cada string puede ser negativo si el MSB del f64
/// está prendido (por ejemplo, y = -20.0 → bits `0xC034000000000000` → BigInt64Array = negativo).
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
            // Reinterpretar como u64 sin pérdida (transmut de bits).
            f64::from_bits(signed as u64)
        })
        .collect()
}

#[test]
fn delaunator_rust_matches_js_self_reference() {
    let v = load_fixture();

    // Handshake de tamaño — si estos no calzan, el fixture o el Rust cambiaron.
    let n_points: usize = v["nPoints"].as_u64().expect("nPoints u64") as usize;
    let n_boundary: usize = v["nBoundary"].as_u64().expect("nBoundary u64") as usize;
    let n_all_points: usize = v["nAllPoints"].as_u64().expect("nAllPoints u64") as usize;
    let n_triangles: usize = v["nTriangles"].as_u64().expect("nTriangles u64") as usize;
    assert_eq!(
        n_all_points,
        n_points + n_boundary,
        "nAllPoints inconsistente"
    );

    // Los `triangles` y `halfedges` del fixture (from `delaunator@5.1.0` JS).
    let want_triangles: Vec<u32> = parse_u32_strings(&v["triangles"], "triangles");
    let want_halfedges: Vec<i32> = parse_i32_strings(&v["halfedges"], "halfedges");
    assert_eq!(
        want_triangles.len(),
        n_triangles * 3,
        "triangles.len() inconsistente con nTriangles"
    );
    assert_eq!(
        want_halfedges.len(),
        n_triangles * 3,
        "halfedges.len() inconsistente con nTriangles"
    );

    // (1) Handshake de `points`+`boundary`: re-producir desde `place_points` Rust y comparar
    //     bit-a-bit con el fixture. Si esto falla, el problema es en `place_points` (no en
    //     `delaunator`) — debería verse primero en `grid_bit_exact.rs`.
    let g = place_points(2000.0, 2000.0, 10000, "861039636");
    assert_eq!(g.points.len(), n_points, "place_points.points.len()");
    assert_eq!(g.boundary.len(), n_boundary, "place_points.boundary.len()");
    assert_eq!(
        g.spacing,
        v["spacing"].as_f64().expect("spacing f64"),
        "spacing diverge"
    );

    // Puntos: 10000 (x,y) → 20000 entries en `points_bits` (flat: x0,y0,x1,y1,...).
    let want_points_flat = parse_f64_bits_strings(&v["points_bits"], "points_bits");
    assert_eq!(
        want_points_flat.len(),
        n_points * 2,
        "points_bits.len() inconsistente"
    );
    for i in 0..n_points {
        let want_x = want_points_flat[2 * i];
        let want_y = want_points_flat[2 * i + 1];
        let got = g.points[i];
        assert_eq!(
            got,
            [want_x, want_y],
            "points[{}] diverge: rust {:?} want {:?} (bits {:#x},{:#x} vs {:#x},{:#x})",
            i,
            got,
            [want_x, want_y],
            got[0].to_bits(),
            got[1].to_bits(),
            want_points_flat[2 * i].to_bits(),
            want_points_flat[2 * i + 1].to_bits(),
        );
    }

    // Boundary: 200 (x,y) → 400 entries en `boundary_bits`.
    let want_boundary_flat = parse_f64_bits_strings(&v["boundary_bits"], "boundary_bits");
    assert_eq!(
        want_boundary_flat.len(),
        n_boundary * 2,
        "boundary_bits.len() inconsistente"
    );
    for i in 0..n_boundary {
        let want_x = want_boundary_flat[2 * i];
        let want_y = want_boundary_flat[2 * i + 1];
        assert_eq!(g.boundary[i], [want_x, want_y], "boundary[{}] diverge", i);
    }

    // (2) Triangulación Delaunay con mi porte bit-exacto de `delaunator@5.1.0.js`.
    let mut all_points: Vec<[f64; 2]> = g.points.clone();
    all_points.extend_from_slice(&g.boundary);
    assert_eq!(all_points.len(), n_all_points);

    let result = from_pairs(&all_points);

    // (3) Comparar `triangles` bit-a-bit.
    assert_eq!(
        result.triangles.len(),
        n_triangles * 3,
        "triangles.len() diverge: rust {} want {} (n_triangles={})",
        result.triangles.len(),
        n_triangles * 3,
        n_triangles,
    );
    let mut mismatches: Vec<(usize, u32, u32)> = Vec::new();
    for (i, &got) in result.triangles.iter().enumerate() {
        let want = want_triangles[i];
        if got != want {
            mismatches.push((i, got, want));
            if mismatches.len() >= 10 {
                break;
            }
        }
    }
    let mismatches_count = if mismatches.is_empty() {
        "0".to_string()
    } else if mismatches.len() == 10 {
        ">=10".to_string()
    } else {
        mismatches.len().to_string()
    };
    assert!(
        mismatches.is_empty(),
        "triangles diverge en {} entradas (primeros 10: {:?})",
        mismatches_count,
        mismatches,
    );

    // (4) Comparar `halfedges` bit-a-bit. Mi porte usa `EMPTY = usize::MAX` para el hull,
    //     equivalente al `-1` (Int32) de JS. Convertimos y comparamos.
    assert_eq!(
        result.halfedges.len(),
        n_triangles * 3,
        "halfedges.len() diverge"
    );
    let mut mismatches: Vec<(usize, i64, i32)> = Vec::new();
    for (i, &got) in result.halfedges.iter().enumerate() {
        let got_i32: i32 = if got == EMPTY { -1 } else { got as i32 };
        let want = want_halfedges[i];
        if got_i32 != want {
            mismatches.push((i, got as i64, want));
            if mismatches.len() >= 10 {
                break;
            }
        }
    }
    let mismatches_count = if mismatches.is_empty() {
        "0".to_string()
    } else if mismatches.len() == 10 {
        ">=10".to_string()
    } else {
        mismatches.len().to_string()
    };
    assert!(
        mismatches.is_empty(),
        "halfedges diverge en {} entradas (primeros 10: {:?})",
        mismatches_count,
        mismatches,
    );

    // (5) Bonus: hull count — Azgaar controla que el boundary (200 puntos) sea el convex hull.
    //     El hull del Rust debe calzar con 200 (puntos del boundary).
    assert_eq!(
        result.hull.len(),
        n_boundary,
        "hull.len() diverge: rust {} want {} (= n_boundary). El boundary debería ser el convex hull.",
        result.hull.len(),
        n_boundary,
    );
}
