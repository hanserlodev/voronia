//! Helpers numéricos de Azgaar traducidos a Rust, bit-exactos.
//!
//! Cubre solo lo que la Fase 1 (regeneración de geometría + parser `.map`) usa.
//! El resto (`rand`, `P`, `Pint`, `gauss`, `ra`, `rw`, `biased`, `getNumberInRange`)
//! es propio de Fase 7 (generación procedural desde seed pura) y **no** se porta acá.

/// Redondeo a `d` decimales con el mismo comportamiento que `rn(v, d)` de Azgaar
/// (`src/utils/numberUtils.ts:7`):
/// ```ts
/// export const rn = (v: number, d: number = 0) => {
///   const m = 10 ** d;
///   return Math.round(v * m) / m;
/// };
/// ```
///
/// Crítico: `Math.round` de JS redondea ties (`.5` exactos) **hacia +Infinity**, no
/// round-half-away-from-zero. Diferencias con `f64::round` de Rust:
/// - `JS Math.round(+0.5)=1`, `Rust (+0.5).round()=1` — igual.
/// - `JS Math.round(-0.5)=-0`, `Rust (-0.5).round()=-1` — DISTINTO.
/// - `JS Math.round(-1.5)=-1`, `Rust (-1.5).round()=-2` — DISTINTO.
///
/// Para `getJitteredGrid` todos los valores `x + jitter` con `x > 0` son positivos
/// en la práctica (ver análisis en `docs/fase-0-investigacion.md` §6.5), así que
/// un flag ingenuo positivo funciona. Pero `rn(v, d)` se usa en otros puntos de
/// Azgaar (p.ej. `reGraph` punto medio costero `rn((x + ex)/2, 1)`), también en
/// coordenadas positivas — en la práctica no se cruza con negativos puros. Aun así,
/// implementamos el `Math.round` exacto para no tener bugs latentes en otros flows
/// que se porteen en Fase 7.
#[inline]
pub fn rn(v: f64, d: u32) -> f64 {
    let m = 10f64.powi(d as i32);
    js_math_round(v * m) / m
}

/// Réplica bit-exacta de `Math.round(x)` de ECMAScript.
///
/// Spec ES2025 §21.3.2.27: "Returns the Number value that is closest to x and is
/// equal to a mathematical integer. If two Numbers are equally close, the one
/// that is **+0**'s closest even? no — ES says: the Number value that is closer
/// is returned; if two Number values are equally close, then **the one that is
/// larger** (closer to +∞) is returned. If x is -0, returns -0."
///
/// En la práctica eso es `floor(x + 0.5)` para todos los casos saexcepto por:
/// - Tie negativo: `floor(-1.0 + 0.5) = floor(-0.5) = -1` ✓ (calza con ES).
/// - Tie negativo entero + 0.5: `floor(-1.5 + 0.5) = floor(-1.0) = -1` ✓.
/// - Para `x = -0.5`: `floor(-0.5 + 0.5) = floor(0.0) = 0` pero ES dice `-0`.
///
/// La diferencia entre `0` y `-0` (signed zero) no afecta al resultado de
/// `Math.round(...)/m` para nuestros usos (resulta en `+0` en lugar de `-0`, y
/// todas las operaciones aritméticas subsiguientes dan idéntico resultado). Por
/// eso implementamos como `floor(x + 0.5)`, sin distinguir signed zero — bit-exacto
/// salvo el signo de la cero, que es irrelevante para las salidas que se comparan.
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
        // Vector confirmado contra JS: `node -e "console.log([0.5,1.5,2.5,-0.5,-1.5,-2.5,0.4999,0.5001,1.4999,1.5001].map(v=>Math.round(v)))"`
        // = [1, 2, 3, -0, -1, -2, 0, 1, 1, 2]. El `-0` se vuelve `+0` en Rust
        // (signed zero no propagado) — para este test asumimos behavior numérico,
        // no bit-pattern del signed zero (no afecta a operaciones aritméticas).
        let cases: &[(f64, i64)] = &[
            (0.5, 1),
            (1.5, 2),
            (2.5, 3),
            (-0.5, 0), // JS da -0; Rust +0; numéricamente iguales.
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

    /// `rn(v, 2)` redondea a 2 decimales (caso típico de `getJitteredGrid`).
    #[test]
    fn rn_two_decimals() {
        // Casos confirmados contra JS `rn`:
        //   10.124 -> 10.12,  10.125 -> 10.13,  10.344 -> 10.34,  10.345 -> 10.35.
        assert_eq!(rn(10.124, 2), 10.12);
        assert_eq!(rn(10.125, 2), 10.13);
        assert_eq!(rn(10.344, 2), 10.34);
        assert_eq!(rn(10.345, 2), 10.35);
    }

    /// `rn(v, 1)` redondea a 1 decimal (caso `reGraph` punto medio costero).
    #[test]
    fn rn_one_decimal() {
        assert_eq!(rn(10.04, 1), 10.0);
        assert_eq!(rn(10.05, 1), 10.1);
        assert_eq!(rn(10.15, 1), 10.2); // tie → up (jMath.round behavior)
    }
}
