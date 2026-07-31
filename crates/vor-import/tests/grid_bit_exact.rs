//! Bit-exact test: `place_points` Rust vs the Azgaar JS version (`graphUtils.ts:17-98`).
//!
//! Loads a fixture generated in `node` with the same version of the Azgaar source
//! that lives in `/home/hans/Proyectos/azgaar-fmg` (master as of this test);
//! the bits are serialized as decimal strings of `u64` (via the JS `BigUint64Array`)
//! to avoid loss when going through JSON (see rationale in `alea_bit_exact.rs`).
//!
//! ## Important note: self-reference fixtures vs the real Brample
//!
//! The fixture is NOT compared against the "Brample" `.map` of fase-0 §12 (that map was
//! generated with a `1.138.0` version of Azgaar that diverged from the cloned repo
//! — see `docs/fase-0-investigacion.md` §12.1 for the `1.138.0` header, and commit
//! `51d8e3e chore: bump version to 1.138.0` in azgaar-fmg whose `package.json`
//! stayed at `1.135.2`). This produced a divergence in the PRNG stream between the
//! version that generated Brample and the current repo version (for example, the
//! first 2 points in Brample are `[10.12, 10.34]` while the current algorithm
//! produces `[15.35, 16.11]`).
//!
//! That is why the fixture here is **self-reference**: it represents "the Azgaar
//! master algorithm as it is today in azgaar-fmg", which is bit-exactly what
//! this port reproduces (Voronia). When a new reference `.map` is regenerated with
//! the same repo version and compared, it should match bit-by-bit.
//!
//! The test that validates against the real Brample (item "Empirical validation
//! against the .map Brample" of the todo) is marked as **blocked by version difference**;
//! see the item in `references/status.md`.

use serde_json::Value;
use std::fs;
use vor_import::geometry::place_points;

#[test]
fn place_points_rust_matches_js_self_reference() {
    let ref_path = "tests/reference/grid_2000x2000_c10k_seed_861039636_selfref.json";
    let raw =
        fs::read_to_string(ref_path).unwrap_or_else(|e| panic!("could not read {ref_path}: {e}"));
    let v: Value = serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid JSON: {e}"));

    let want_spacing: u64 = v["spacing_bits"]
        .as_str()
        .expect("spacing_bits as string")
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
        "spacing diverges: rust {:#x} want {:#x}",
        g.spacing.to_bits(),
        want_spacing
    );

    // Boundary (all).
    assert_eq!(
        g.boundary.len() * 2,
        want_boundary.len(),
        "number of boundary bits ({}) does not match entries ({})",
        want_boundary.len(),
        g.boundary.len()
    );
    for (i, p) in g.boundary.iter().enumerate() {
        let want_x = f64::from_bits(want_boundary[2 * i]);
        let want_y = f64::from_bits(want_boundary[2 * i + 1]);
        assert_eq!(
            *p,
            [want_x, want_y],
            "boundary[{}] diverges: rust {:?} want {:?}",
            i,
            p,
            [want_x, want_y]
        );
    }

    // First 100 points (200 bits) — verifies the jitter + RNG consumption order.
    assert!(
        want_points.len() <= g.points.len() * 2,
        "fixture asks for {} bits but there are only {} points ({} bits)",
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
            "points[{}] diverges: rust {:?} want {:?} (bits {:#x},{:#x} vs {:#x},{:#x})",
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
