//! `Alea@1.0.1` — PRNG de Johannes Baagøe (npm `alea` 1.0.1).
//!
//! Port bit-exacto a Rust, verificado contra el fuente JS original en
//! `vor-import/tests/reference/alea-1.0.1.original.js`.

#[derive(Debug, Clone)]
pub struct Alea {
    s0: f64,
    s1: f64,
    s2: f64,
    c: f64,
}

const TWO_POW_NEG_32: f64 = 2.3283064365386963e-10;
const TWO_POW_32: f64 = 4_294_967_296.0;

impl Alea {
    pub fn new(seed: &str) -> Self {
        let mut mash_n: f64 = 0xefc8249du32 as f64;
        let mut mash = |data: &str| -> f64 {
            for ch in data.chars() {
                mash_n += ch as u32 as f64;
                let mut h = 0.02519603282416938_f64 * mash_n;
                mash_n = (h as u32) as f64;
                h -= mash_n;
                h *= mash_n;
                mash_n = (h as u32) as f64;
                h -= mash_n;
                mash_n += h * TWO_POW_32;
            }
            let n_u32 = mash_n as u32 as f64;
            n_u32 * TWO_POW_NEG_32
        };
        let mut s0 = mash(" ");
        let mut s1 = mash(" ");
        let mut s2 = mash(" ");
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
        Self { s0, s1, s2, c: 1.0 }
    }

    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        let term_a = 2091639.0_f64 * self.s0;
        let term_b = self.c * TWO_POW_NEG_32;
        let t = term_a + term_b;
        self.s0 = self.s1;
        self.s1 = self.s2;
        let new_c = t as i32;
        self.c = new_c as f64;
        self.s2 = t - self.c;
        self.s2
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_f64() * TWO_POW_32) as u32
    }

    pub fn next_fract53(&mut self) -> f64 {
        const TWO_POW_NEG_53: f64 = 1.1102230246251565e-16;
        let a = self.next_f64();
        let b = (self.next_f64() * (0x200000 as f64)) as i32;
        a + (b as f64) * TWO_POW_NEG_53
    }
}
