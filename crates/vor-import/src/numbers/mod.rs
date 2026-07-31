//! Azgaar numeric helpers ported to Rust, bit-exact.
//!
//! Covers only what Phase 1 (geometry regeneration + `.map` parser) uses.
//! The rest (`rand`, `P`, `Pint`, `gauss`, `ra`, `rw`, `biased`, `getNumberInRange`)
//! belongs to Phase 7 (procedural generation from a pure seed) and is **not**
//! ported here.

/// Rounds to `d` decimals with the same behavior as Azgaar's `rn(v, d)`
/// (`src/utils/numberUtils.ts:7`):
/// ```ts
/// export const rn = (v: number, d: number = 0) => {
///   const m = 10 ** d;
///   return Math.round(v * m) / m;
/// };
/// ```
///
/// Critical: JS `Math.round` rounds ties (exact `.5`) **towards +Infinity**, not
/// round-half-away-from-zero. Differences from Rust's `f64::round`:
/// - `JS Math.round(+0.5)=1`, `Rust (+0.5).round()=1` — same.
/// - `JS Math.round(-0.5)=-0`, `Rust (-0.5).round()=-1` — DIFFERENT.
/// - `JS Math.round(-1.5)=-1`, `Rust (-1.5).round()=-2` — DIFFERENT.
///
/// For `getJitteredGrid`, all `x + jitter` values with `x > 0` are positive in
/// practice (see the analysis in `docs/phases/phase-0-research.md` §6.5), so a naive
/// positive flag would work. But `rn(v, d)` is also used elsewhere in Azgaar
/// (e.g. `reGraph` coastal midpoint `rn((x + ex)/2, 1)`), also on positive
/// coordinates — in practice it never crosses purely negative values. Even so,
/// we implement the exact `Math.round` to avoid latent bugs in other flows
/// ported in Phase 7.
#[inline]
pub fn rn(v: f64, d: u32) -> f64 {
    let m = 10f64.powi(d as i32);
    js_math_round(v * m) / m
}

/// Bit-exact replica of ECMAScript `Math.round(x)`.
///
/// Spec ES2025 §21.3.2.27: "Returns the Number value that is closest to x and is
/// equal to a mathematical integer. If two Numbers are equally close, the one
/// that is **+0**'s closest even? no — ES says: the Number value that is closer
/// is returned; if two Number values are equally close, then **the one that is
/// larger** (closer to +∞) is returned. If x is -0, returns -0."
///
/// In practice that is `floor(x + 0.5)` for all cases except:
/// - Negative tie: `floor(-1.0 + 0.5) = floor(-0.5) = -1` ✓ (matches ES).
/// - Negative integer + 0.5 tie: `floor(-1.5 + 0.5) = floor(-1.0) = -1` ✓.
/// - For `x = -0.5`: `floor(-0.5 + 0.5) = floor(0.0) = 0` but ES says `-0`.
///
/// The difference between `0` and `-0` (signed zero) does not affect the result
/// of `Math.round(...)/m` for our uses (it yields `+0` instead of `-0`, and all
/// subsequent arithmetic produces identical results). That is why we implement it
/// as `floor(x + 0.5)` without distinguishing signed zero — bit-exact except for
/// the sign of zero, which is irrelevant for the compared outputs.
#[inline]
fn js_math_round(x: f64) -> f64 {
    (x + 0.5_f64).floor()
}

#[cfg(test)]
mod tests {
    use super::rn;

    /// `rn(v, 0)` ≡ `Math.round(v)`.
    #[test]
    fn rn_zero_decimals_matches_js_math_round() {
        // Vector confirmed against JS: `node -e "console.log([0.5,1.5,2.5,-0.5,-1.5,-2.5,0.4999,0.5001,1.4999,1.5001].map(v=>Math.round(v)))"`
        // = [1, 2, 3, -0, -1, -2, 0, 1, 1, 2]. The `-0` becomes `+0` in Rust
        // (signed zero is not propagated) — for this test we assume numeric
        // behavior, not the signed-zero bit pattern (it does not affect arithmetic).
        let cases: &[(f64, i64)] = &[
            (0.5, 1),
            (1.5, 2),
            (2.5, 3),
            (-0.5, 0), // JS gives -0; Rust +0; numerically equal.
            (-1.5, -1),
            (-2.5, -2),
            (0.4999, 0),
            (0.5001, 1),
            (1.4999, 1),
            (1.5001, 2),
            (-1.4999, -1),
            (-1.5001, -2),
        ];
        for (v, want) in cases {
            assert_eq!(rn(*v, 0) as i64, *want, "rn({v}, 0) = Math.round({v})");
        }
    }

    /// `rn(v, 2)` rounds to 2 decimals (typical `getJitteredGrid` case).
    #[test]
    fn rn_two_decimals() {
        // Cases confirmed against JS `rn`:
        //   10.124 -> 10.12,  10.125 -> 10.13,  10.344 -> 10.34,  10.345 -> 10.35.
        assert_eq!(rn(10.124, 2), 10.12);
        assert_eq!(rn(10.125, 2), 10.13);
        assert_eq!(rn(10.344, 2), 10.34);
        assert_eq!(rn(10.345, 2), 10.35);
    }

    /// `rn(v, 1)` rounds to 1 decimal (`reGraph` coastal midpoint case).
    #[test]
    fn rn_one_decimal() {
        assert_eq!(rn(10.04, 1), 10.0);
        assert_eq!(rn(10.05, 1), 10.1);
        assert_eq!(rn(10.15, 1), 10.2); // tie → up (jMath.round behavior)
    }
}
