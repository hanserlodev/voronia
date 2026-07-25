//! Test bit-exacto: `place_points` Rust vs versión JS de Azgaar (`graphUtils.ts:17-98`).
//!
//! Carga un fixture generado en `node` con la misma versión del fuente de Azgaar
//! que vive en `/home/hans/Proyectos/azgaar-fmg` (master a la fecha de esta test);
//! los bits están serializados como strings decimalizados de `u64` (vía
//! `BigUint64Array` de JS) para evitar pérdida al pasar por JSON (ver rationale en
//! `alea_bit_exact.rs`).
//!
//! ## Aclaración importante: fixtures self-reference vs Brample real
//!
//! El fixture NO se compara contra el `.map` "Brample" de fase-0 §12 (ese mapa fue
//! generado con una versión `1.138.0` de Azgaar que divergió del repo clonado
//! — ver `docs/fase-0-investigacion.md` §12.1 para header `1.138.0`, y commit
//! `51d8e3e chore: bump version to 1.138.0` en azgaar-fmg cuyo `package.json`
//! quedó en `1.135.2`). Esto produjo una divergencia en el stream del PRNG entre
//! la versión que generó Brample y la versión actual del repo (por ejemplo, los
//! primeros 2 puntos en Brample son `[10.12, 10.34]` mientras que el algoritmo
//! actual produce `[15.35, 16.11]`).
//!
//! Por eso el fixture acá es **self-reference**: representa "el algoritmo de
//! Azgaar master tal como está hoy en azgaar-fmg", lo cual es bit-exactamente lo
//! que este porta (Voronia). Cuando se regenere un nuevo `.map` de referencia con
//! la misma versión del repo y se compare, deberá calzar bit-a-bit.
//!
//! El test que valida contra el Brample real (item `Validación empírica contra
//! .map Brample` del todo) está marcado como **bloqueado por diferencia de
//! versión**; véase el item en `references/status.md`.

use serde_json::Value;
use std::fs;
use vor_import::geometry::place_points;

#[test]
fn place_points_rust_matches_js_self_reference() {
    let ref_path = "tests/reference/grid_2000x2000_c10k_seed_861039636_selfref.json";
    let raw =
        fs::read_to_string(ref_path).unwrap_or_else(|e| panic!("no se pudo leer {ref_path}: {e}"));
    let v: Value = serde_json::from_str(&raw).unwrap_or_else(|e| panic!("JSON inválido: {e}"));

    let want_spacing: u64 = v["spacing_bits"]
        .as_str()
        .expect("spacing_bits like string")
        .parse()
        .expect("spacing_bits decimal u64");
    let want_boundary: Vec<u64> = v["boundary_bits"]
        .as_array()
        .expect("boundary_bits array")
        .iter()
        .map(|x| x.as_str().expect("str").parse::<u64>().expect("u64"))
        .collect();
    let want_points: Vec<u64> = v["points_first200_bits"]
        .as_array()
        .expect("points_first200_bits array")
        .iter()
        .map(|x| x.as_str().expect("str").parse::<u64>().expect("u64"))
        .collect();

    let g = place_points(2000.0, 2000.0, 10000, "861039636");

    // Spacing.
    assert_eq!(
        g.spacing.to_bits(),
        want_spacing,
        "spacing diverge: rust {:#x} want {:#x}",
        g.spacing.to_bits(),
        want_spacing
    );

    // Boundary (todo).
    assert_eq!(
        g.boundary.len() * 2,
        want_boundary.len(),
        "cantidad de boundary bits ({}) no calza con entries ({})",
        want_boundary.len(),
        g.boundary.len()
    );
    for (i, p) in g.boundary.iter().enumerate() {
        let want_x = f64::from_bits(want_boundary[2 * i]);
        let want_y = f64::from_bits(want_boundary[2 * i + 1]);
        assert_eq!(
            *p,
            [want_x, want_y],
            "boundary[{}] diverge: rust {:?} want {:?}",
            i,
            p,
            [want_x, want_y]
        );
    }

    // Primeros 100 puntos (200 bits) — verifica el jitter + orden de consumo del RNG.
    assert!(
        want_points.len() <= g.points.len() * 2,
        "fixture pide {} bits pero solo hay {} puntos ({} bits)",
        want_points.len(),
        g.points.len(),
        g.points.len() * 2
    );
    let n = want_points.len() / 2;
    for i in 0..n {
        let want_x = f64::from_bits(want_points[2 * i]);
        let want_y = f64::from_bits(want_points[2 * i + 1]);
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
            want_points[2 * i],
            want_points[2 * i + 1]
        );
    }
}
