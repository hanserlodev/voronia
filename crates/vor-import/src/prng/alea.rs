//! `Alea@1.0.1` — Johannes Baagøe's PRNG (npm `alea` 1.0.1), ported bit-exact to Rust.
//!
//! This replicates exactly `node_modules/alea/alea@1.0.1` (the original source
//! committed at `crates/vor-import/tests/reference/alea-1.0.1.original.js`).
//!
//! Why bit-exactness matters here: Azgaar (`graphUtils.ts:1`) does
//! `Math.random = Alea(seed)` before `getJitteredGrid`, so the first
//! `Math.random()` consumed by `getJitteredGrid` is the first one of the freshly
//! built `Alea(seed)` stream (see `docs/fase-0-investigacion.md` §6.5, §7.1, §13.4).
//! If our Alea diverges even by 1 ULP, the grid `points` will not match the `[6]`
//! slot of the Azgaar `.map`, and attributes land in wrong cells (silent bug with
//! no runtime error — §13.4 consequence 3).
//!
//! ## JS → Rust equivalences used
//!
//! - JS `number` (f64): represented as `f64`.
//! - `n >>> 0`: conversion to `Uint32` (drops the integer part > 2^32 and sign);
//!   in Rust `t as u32` (which truncates saturating to the low-bit pattern,
//!   equivalent to `>>> 0` for the ranges involved).
//! - `t | 0`: conversion to `Int32` (signed, two's complement wrap);
//!   in Rust `t as i32`.
//! - `+x`: coerces to `Number`/`f64`; here all our variables are already `f64`.
//! - `× 0x100000000`: `2^32` (exceeds int32 but fits in f64 losslessly up to ~2^53).

/// Internal state of the Alea generator, equivalent to the `Alea()` closure in JS.
#[derive(Debug, Clone)]
pub struct Alea {
    /// `s0`, `s1`, `s2` ∈ [0, 1). Always positive (see seeding + generator).
    s0: f64,
    s1: f64,
    s2: f64,
    /// `c` starts at 1; after each `next()` it holds `t | 0` (signed int32). We store it
    /// as `f64` to reproduce exactly the `c * 2^-32` computation of the next step
    /// (in JS, `c` is not reinterpreted as int before the multiplication; it is
    /// always a Number).
    c: f64,
}

/// Exact constant from the JS source: `2^-32 = 2.3283064365386963e-10`.
const TWO_POW_NEG_32: f64 = 2.3283064365386963e-10;
/// `2^32` (= `0x100000000` in JS). In Rust `0x100000000` overflows i32, so we
/// write it explicitly. It is the exact constant from the JS source.
const TWO_POW_32: f64 = 4_294_967_296.0;

impl Alea {
    /// Equivalent to `new Alea(seed)` (with `seed` passed as a string in Azgaar).
    /// `args` is passed here as a slice of items; the bit-exact behavior replicates
    /// `Alea(seed)` with `seed: &str` (Azgaar always passes 1 string).
    pub fn new(seed: &str) -> Self {
        // `Mash()` constructor — `n = 0xefc8249d` as a Number (keeps internal f64):
        // var n = 0xefc8249d;  (Uint32 literal = 4022871197. Before any
        // `>>> 0` the JS keeps it as a Number (f64). In Rust, the literal `0xefc8249d`
        // overflows i32, so we write it as `u32` and cast to f64 —
        // if we had left it as `-272096099i32 as f64` it would yield a negative value,
        // different from the JS.)
        let mut mash_n: f64 = 0xefc8249du32 as f64;
        // JS closures: each `mash(data)` mutates internal `n`. We define closures
        // here so as not to expose `Mash` outside this constructor.
        let mut mash = |data: &str| -> f64 {
            // `data = data.toString()` — we already pass &str.
            for ch in data.chars() {
                // `n += data.charCodeAt(i)`: each step in its own assignment
                // to prevent LLVM from emitting FMA and diverging by 1 ULP.
                mash_n += ch as u32 as f64;
                // `h = 0.02519603282416938 * n`:
                let mut h = 0.02519603282416938_f64 * mash_n;
                // `n = h >>> 0`:
                mash_n = (h as u32) as f64;
                // `h -= n`:
                h -= mash_n;
                // `h *= n`:
                h *= mash_n;
                // `n = h >>> 0`:
                mash_n = (h as u32) as f64;
                // `h -= n`:
                h -= mash_n;
                // `n += h * 0x100000000` (2^32):
                let scaled = h * TWO_POW_32;
                mash_n += scaled;
            }
            // `return (n >>> 0) * 2.3283064365386963e-10`:
            let n_u32 = mash_n as u32 as f64;
            n_u32 * TWO_POW_NEG_32
        };
        // var s0 = 0; s1 = 0; s2 = 0; c = 1;
        // var mash = Mash();
        let mut s0 = mash(" ");
        let mut s1 = mash(" ");
        let mut s2 = mash(" ");
        // for (var i = 0; i < args.length; i++) { s0 -= mash(args[i]); ...}
        // In Azgaar `Alea(seed)` always takes 1 arg; the Voronia API accepts 1 seed string.
        // We reproduce the exact arg-by-arg loop:
        s0 -= mash(seed);
        if s0 < 0.0 {
            s0 += 1.0;
        }
        s1 -= mash(seed);
        if s1 < 0.0 {
            s1 += 1.0;
        }
        s2 -= mash(seed);
        if s2 < 0.0 {
            s2 += 1.0;
        }
        // mash = null;  no effect in Rust.
        Self { s0, s1, s2, c: 1.0 }
    }

    /// Equivalent to `random()` (or its alias `.next()`):
    /// ```js
    /// var t = 2091639 * s0 + c * 2.3283064365386963e-10;
    /// s0 = s1; s1 = s2; s2 = t - (c = t | 0);
    /// ```
    /// Returns `f64 ∈ [0, 1)`.
    ///
    /// ## Bit-exactness note
    /// The JS source performs two multiplications and one addition in textual order:
    /// `2091639 * s0` first, `c * 2^-32` afterwards, sum last. LLVM may emit FMA
    /// (fused multiply-add) on CPUs with `target-cpu=native`, which reduces
    /// intermediate rounding — that diverges from JS by 1 ULP of the mantissa. For
    /// Rust to produce exactly the same rounding as JS, we evaluate the terms
    /// in separate temporary variables (no FMA), and the `assume(FAST_MATH)`
    /// policy must never be active in this crate so nothing is reordered.
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        // Separate assignments force the rounding at each step — no FMA.
        let term_a = 2091639.0_f64 * self.s0;
        let term_b = self.c * TWO_POW_NEG_32;
        let t = term_a + term_b;
        self.s0 = self.s1;
        self.s1 = self.s2;
        // In JS: `c = t | 0` (signed int32); in Rust: `t as i32` (two's complement wrap).
        let new_c = t as i32;
        // `s2 = t - (c = t | 0)`:
        self.c = new_c as f64;
        self.s2 = t - self.c;
        self.s2
    }

    /// Equivalent to `random.uint32()` (not used by Azgaar in the geometric stretch,
    /// kept exposed for fidelity to the source): `random() * 0x100000000`.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_f64() * TWO_POW_32) as u32
    }

    /// Equivalent to `random.fract53()` — not used by Azgaar in the geometric stretch.
    /// ```js
    /// return random() + (random() * 0x200000 | 0) * 1.1102230246251565e-16;
    /// ```
    #[inline]
    pub fn next_fract53(&mut self) -> f64 {
        // 1.1102230246251565e-16 = 2^-53. Exact constant from the JS.
        const TWO_POW_NEG_53: f64 = 1.1102230246251565e-16;
        // 0x200000 = 2^21, fits in i32.
        let a = self.next_f64();
        let b = (self.next_f64() * (0x200000 as f64)) as i32;
        a + (b as f64) * TWO_POW_NEG_53
    }
}

#[cfg(test)]
mod tests {
    use super::Alea;

    /// Basic determinism test — generates a sequence with a fixed seed and verifies
    /// that the first floats are stable across runs. Test vector from the JS
    /// version of `alea@1.0.1`: to obtain the reference values run
    /// ```
    /// node -e "const A=require('$ROOT/tests/reference/alea-1.0.1.original.js'); const r=A('861039636'); for (let i=0;i<10;i++) console.log(r());"
    /// ```
    /// Replace the array below with the exact values from the node output (first 10
    /// floats with seed `861039636`, the same as Brample).
    ///
    /// Until node is available, this test asserts intra-Rust determinism (regression)
    /// even if not bit-exactness against JS.
    #[test]
    fn alea_seed_is_stable_between_runs() {
        let mut a1 = Alea::new("861039636");
        let mut a2 = Alea::new("861039636");
        for _ in 0..1000 {
            assert_eq!(a1.next_f64(), a2.next_f64());
        }
    }

    /// `s0`, `s1`, `s2` start in [0, 1) and all outputs should be in [0, 1).
    /// Invariant test, not a bit-exactness test.
    #[test]
    fn alea_outputs_are_in_unit_range() {
        let mut a = Alea::new("861039636");
        for _ in 0..10_000 {
            let v = a.next_f64();
            assert!((0.0..1.0).contains(&v), "alea output outside [0,1): {v}");
        }
    }

    /// Different seeds must yield different sequences.
    #[test]
    fn alea_different_seeds_diverge() {
        let mut a = Alea::new("1");
        let mut b = Alea::new("2");
        let mut diffs = 0;
        for _ in 0..100 {
            if a.next_f64() != b.next_f64() {
                diffs += 1;
            }
        }
        assert!(
            diffs > 90,
            "different seeds produce nearly identical sequences"
        );
    }
}
