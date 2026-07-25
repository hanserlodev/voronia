//! Test bit-exacto: `Alea@1.0.1` Rust vs `alea@1.0.1` JS (npm).
//!
//! Carga los primeros 1000 floats (como bits `u64` serializados como strings
//! decimalizados para evitar pérdida al serializar `f64` por JSON) generados por
//! la versión JS con seed `861039636` (la del Brample, el `.map` real de prueba)
//! y verifica que la versión Rust produce los mismos bits. Los valores de
//! referencia se generaron con `gen_alea_ref_bits.js` corriendo el fuente
//! original `alea-1.0.1.original.js` con `node`, y se commitean en
//! `crates/vor-import/tests/reference/alea_seed_*_first_*_bits.json`.
//!
//! Si este test falla, las `grid.points` que vor-import regenere no van a calzar
//! con el slot `[6]` del `.map` de Azgaar — ver `docs/fase-0-investigacion.md` §6.5,
//! §13.4 consequence 3 (bug silencioso: atributos en celdas equivocadas, sin error).
//!
//! Nota: serializamos como bits y no como strings-decimales-renderizados porque
//! `serde_json` parsea `0.20791621506214142` al f64 más cercano al string (que puede
//! NO ser el mismo f64 que el JS produjo — el string es solo el más cercano round-trip
//! printable, sin 1-1 con bits). Serializar bits evita esa ambigüedad.

use serde_json::Value;
use std::fs;
use vor_import::prng::Alea;

#[test]
fn alea_rust_matches_js_for_brample_seed() {
    let ref_path = "tests/reference/alea_seed_861039636_first_1000_bits.json";
    let raw =
        fs::read_to_string(ref_path).unwrap_or_else(|e| panic!("no se pudo leer {ref_path}: {e}"));
    let arr: Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("JSON inválido en {ref_path}: {e}"));
    let expected: Vec<u64> = arr
        .as_array()
        .expect("la referencia debe ser un array JSON")
        .iter()
        .map(|v| {
            v.as_str()
                .expect("cada elemento debe ser un string (bits como decimal string)")
                .parse::<u64>()
                .expect("cada string debe ser decimal u64 válido")
        })
        .collect();
    assert_eq!(
        expected.len(),
        1000,
        "esperaba exactamente 1000 valores de referencia"
    );

    let mut rng = Alea::new("861039636");
    for (i, want_bits) in expected.iter().enumerate() {
        let got = rng.next_f64();
        assert_eq!(
            got.to_bits(),
            *want_bits,
            "alea[{}]: got bits {:#x} ({}) want bits {:#x} ({})",
            i,
            got.to_bits(),
            got,
            want_bits,
            f64::from_bits(*want_bits),
        );
    }
}

/// Test adicional con otro seed (cubre paths distintos del Mash).
#[test]
fn alea_rust_matches_js_for_short_seed() {
    let ref_path = "tests/reference/alea_seed_42_first_100_bits.json";
    let raw = fs::read_to_string(ref_path).expect("fixture de seed '42' debe existir");
    let arr: Value = serde_json::from_str(&raw).expect("JSON inválido");
    let expected: Vec<u64> = arr
        .as_array()
        .expect("array esperado")
        .iter()
        .map(|v| v.as_str().expect("string").parse::<u64>().expect("u64"))
        .collect();
    let mut rng = Alea::new("42");
    for (i, want_bits) in expected.iter().enumerate() {
        let got = rng.next_f64();
        assert_eq!(
            got.to_bits(),
            *want_bits,
            "seed '42' [{}]: got {:#x} want {:#x} (got {}, want {})",
            i,
            got.to_bits(),
            want_bits,
            got,
            f64::from_bits(*want_bits)
        );
    }
}
