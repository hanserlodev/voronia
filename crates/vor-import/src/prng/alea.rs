//! `Alea@1.0.1` — PRNG de Johannes Baagøe (npm `alea` 1.0.1), portado bit-exacto a Rust.
//!
//! Esto replica exactamente `node_modules/alea/alea@1.0.1` (el fuente original commiteado
//! en `crates/vor-import/tests/reference/alea-1.0.1.original.js`).
//!
//! Por qué bit-exactitud importa aquí: Azgaar (`graphUtils.ts:1`) hace
//! `Math.random = Alea(seed)` antes de `getJitteredGrid`, así que el primer
//! `Math.random()` que consume `getJitteredGrid` es el primero del stream del `Alea(seed)`
//! recién construido (ver `docs/fase-0-investigacion.md` §6.5, §7.1, §13.4). Si nuestro
//! Alea diverge aunque sea en 1 ULP, los `points` del grid no van a calzar con
//! el slot `[6]` del `.map` de Azgaar, y los atributos caen en celdas equivocadas
//! (bug silencioso sin error en runtime — §13.4 consequence 3).
//!
//! ## Equivalencias JS → Rust usadas
//!
//! - JS `number` (f64): representado como `f64`.
//! - `n >>> 0`: conversión a `Uint32` (descarta parte entera > 2^32 y signo);
//!   en Rust `t as u32` (que truncae saturando el patrón de bits bajos, equivale a
//!   `>>> 0` para los rangos que interviene).
//! - `t | 0`: conversión a `Int32` (signed, wrap de dos complementos);
//!   en Rust `t as i32`.
//! - `+x`: coerce a `Number`/`f64`; aquí todas nuestras variables ya son `f64`.
//! - `× 0x100000000`: `2^32` (supera el int32 pero calza en f64 sin pérdida hasta ~2^53).

/// Estado interno del generador Alea, equivalente al closure de `Alea()` en JS.
#[derive(Debug, Clone)]
pub struct Alea {
    /// `s0`, `s1`, `s2` ∈ [0, 1). Siempre positivos (ver seeding + generador).
    s0: f64,
    s1: f64,
    s2: f64,
    /// `c` arranca en 1; tras cada `next()` vale `t | 0` (signed int32). Lo guardamos
    /// como `f64` para reproducir exacto el cómputo `c * 2^-32` del paso siguiente
    /// (en JS, `c` no es reinterpretado como int antes de la multiplicación; es
    /// siempre Number).
    c: f64,
}

/// Constante exacta del fuente JS: `2^-32 = 2.3283064365386963e-10`.
const TWO_POW_NEG_32: f64 = 2.3283064365386963e-10;
/// `2^32` (= `0x100000000` en JS). En Rust `0x100000000` rebasa i32, así que lo
/// escribimos explícito. Es la constante exacta del fuente JS.
const TWO_POW_32: f64 = 4_294_967_296.0;

impl Alea {
    /// Equivalente a `new Alea(seed)` (con `seed` pasado como string en Azgaar).
    /// `args` se pasa acá como slice de items; el comportamiento bit-exacto replica
    /// `Alea(seed)` con `seed: &str` (Azgaar pasa siempre 1 string).
    pub fn new(seed: &str) -> Self {
        // `Mash()` constructor — `n = 0xefc8249d` como Number (mantiene f64 interno):
        // var n = 0xefc8249d;  (Uint32 literal = 4022871197. Antes de cualquier
        // `>>> 0` el JS lo mantiene como Number (f64). En Rust, el literal `0xefc8249d`
        // rebasa i32, así que lo escribimos como `u32` y formoseamos a f64 —
        // si lo hubiéramos dejado como `-272096099i32 as f64` daría un valor negativo,
        // distinto del JS.)
        let mut mash_n: f64 = 0xefc8249du32 as f64;
        // Closures JS: cada `mash(data)` muta `n` interno. Definimos closures
        // aquí para no exhibir `Mash` fuera de este constructor.
        let mut mash = |data: &str| -> f64 {
            // `data = data.toString()` — ya pasamos &str.
            for ch in data.chars() {
                // `n += data.charCodeAt(i)`: cada paso en su propia asignación
                // para impedir que LLVM emita FMA y diverja en 1 ULP.
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
        // En Azgaar `Alea(seed)` always 1 arg; el API de Voronia acepta 1 seed string.
        // Reproducimos el lazo exacto arg-por-arg:
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
        // mash = null;  sin efecto en Rust.
        Self { s0, s1, s2, c: 1.0 }
    }

    /// Equivalente a `random()` (o su alias `.next()`):
    /// ```js
    /// var t = 2091639 * s0 + c * 2.3283064365386963e-10;
    /// s0 = s1; s1 = s2; s2 = t - (c = t | 0);
    /// ```
    /// Retorna `f64 ∈ [0, 1)`.
    ///
    /// ## Nota de bit-exactitud
    /// El fuente JS hace dos multiplicaciones y una suma en orden textual:
    /// `2091639 * s0` primero, `c * 2^-32` después, suma al final. LLVM puede que
    /// tire FMA (fused multiply-add) en CPUs con `target-cpu=native` que reduce
    /// redondeos intermedios — eso diverge de JS en 1 ULP del mantissa. Para que
    /// Rust produzca exactamente los mismos redondeos que JS, evalúo los términos
    /// en variables temporales separadas (sin FMA) y laítica `assume(FAST_MATH)`
    /// nunca debe estar activa en este crate para no reordenar.
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        // Asignaciones separadas fuerzan el redondeo en cada paso — no FMA.
        let term_a = 2091639.0_f64 * self.s0;
        let term_b = self.c * TWO_POW_NEG_32;
        let t = term_a + term_b;
        self.s0 = self.s1;
        self.s1 = self.s2;
        // En JS: `c = t | 0` (signed int32); en Rust: `t as i32` (wrap de dos complementos).
        let new_c = t as i32;
        // `s2 = t - (c = t | 0)`:
        self.c = new_c as f64;
        self.s2 = t - self.c;
        self.s2
    }

    /// Equivalente a `random.uint32()` (no usado por Azgaar en el tramo geometric,
    /// queda expuesto por fidelidad al source): `random() * 0x100000000`.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_f64() * TWO_POW_32) as u32
    }

    /// Equivalente a `random.fract53()` — no usado por Azgaar en el tramo geometrico.
    /// ```js
    /// return random() + (random() * 0x200000 | 0) * 1.1102230246251565e-16;
    /// ```
    #[inline]
    pub fn next_fract53(&mut self) -> f64 {
        // 1.1102230246251565e-16 = 2^-53. constante exacta del JS.
        const TWO_POW_NEG_53: f64 = 1.1102230246251565e-16;
        // 0x200000 = 2^21, calza en i32.
        let a = self.next_f64();
        let b = (self.next_f64() * (0x200000 as f64)) as i32;
        a + (b as f64) * TWO_POW_NEG_53
    }
}

#[cfg(test)]
mod tests {
    use super::Alea;

    /// Test básico no determinismo — genera una secuencia con seed fija y verifica
    /// que los primeros floats son estables entre corridas. Vector de test备案 contra
    /// la versión JS de `alea@1.0.1`: para obtener los valores de referencia correr
    /// ```
    /// node -e "const A=require('$ROOT/tests/reference/alea-1.0.1.original.js'); const r=A('861039636'); for (let i=0;i<10;i++) console.log(r());"
    /// ```
    /// Reemplazar el array abajo con los valores exactos del node output (primeros 10
    /// floats con la seed `861039636`, la misma del Brample).
    ///
    /// Hasta tener node, este test afirma determinismo intra-Rust (regresión)
    /// aunque no bit-exactitud contra JS.
    #[test]
    fn alea_seed_is_stable_between_runs() {
        let mut a1 = Alea::new("861039636");
        let mut a2 = Alea::new("861039636");
        for _ in 0..1000 {
            assert_eq!(a1.next_f64(), a2.next_f64());
        }
    }

    /// `s0`, `s1`, `s2` arrancan en [0, 1) y todas las salidas deberían estar en [0, 1).
    /// Test de invariantes, no de bit-exactitud.
    #[test]
    fn alea_outputs_are_in_unit_range() {
        let mut a = Alea::new("861039636");
        for _ in 0..10_000 {
            let v = a.next_f64();
            assert!((0.0..1.0).contains(&v), "alea output fuera de [0,1): {v}");
        }
    }

    /// Seeds distintas deben dar secuencias distintas.
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
            "seeds distintas producen secuencias casi idénticas"
        );
    }
}
