//! Geometría del grid jitterizado — porte bit-exacto de `graphUtils.ts:17-98` de Azgaar.
//!
//! Pipeline de Azgaar (confirmado contra el Brample en `docs/fase-0-investigacion.md`
//! §12.4, §13.3):
//! 1. `generateGrid(seed, w, h)` resembra el PRNG con `Math.random = Alea(seed)` (npm).
//! 2. `placePoints(w, h)` → `spacing`, `cellsDesired`, `boundary` (no RNG), `points` (jitter, consume RNG), `cellsX`, `cellsY`.
//! 3. `cells = calculateVoronoi(points, boundary)` en vor-import (`geometry::voronoi`).
//!
//! Importante: el `id` de celda `k` corresponde a `points[k]`, y `cells.i[k] = k` en
//! la malla Voronoi reconstruida. El orden del Alea consume jitter en fila-mayor
//! (`y` externo, `x` interno), 2 floats por celda. Si el porte itera en otro orden o
//! consume RNG en otra secuencia, los `points` no calzan con el slot `[6]` del `.map`
//! y los atributos acabarán en celdas equivocadas — bug silencioso (fase-0 §13.4).

use crate::numbers::rn;
use crate::prng::Alea;

/// Producto de `place_points` — los campos del grid antes de Voronoi.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedGrid {
    /// `spacing` redondeado a 2 decimales.
    pub spacing: f64,
    /// Cantidad deseada de celdas (input del usuario en Azgaar, o viene del slot `[6]`).
    pub cells_desired: u32,
    /// Puntos virtuales de borde para evitar celdas Voronoi infinitas. No consumen RNG.
    pub boundary: Vec<[f64; 2]>,
    /// Puntos jitterizados en fila-mayor (`y` externo, `x` interno). Orden determina `cells.i`.
    pub points: Vec<[f64; 2]>,
    /// Cantidad de columnas (`cellsX` en Azgaar).
    pub cells_x: u32,
    /// Cantidad de filas (`cellsY` en Azgaar).
    pub cells_y: u32,
}

/// `placePoints(graphWidth, graphHeight)` de Azgaar (`graphUtils.ts:69-98`),_BIT-exacto.
///
/// `cells_desired` es una entrada que en Azgaar viene de la UI (`pointsInput.dataset.cells`).
/// Voronia al importar un `.map` lo toma del slot `[6]` (`cellsDesired: 10000` en Brample).
/// **No lo inferimos de UI** — es input explícito del caller.
///
/// El `seed` se usa para construir el `Alea(seed)` interno — esto replica el efecto del
/// monkey-patcheo de `Math.random = Alea(seed)` en `generateGrid` (`graphUtils.ts:137`).
/// El primer `Math.random` que consume `get_jittered_grid` es el primer stream de este Alea.
pub fn place_points(
    graph_width: f64,
    graph_height: f64,
    cells_desired: u32,
    seed: &str,
) -> PlacedGrid {
    let spacing = rn(
        (graph_width * graph_height / cells_desired as f64).sqrt(),
        2,
    );
    let boundary = get_boundary_points(graph_width, graph_height, spacing);
    // Re-seedea PRNG con Alea(seed) — equivalente a `Math.random = Alea(seed)` en
    // `generateGrid` antes del primer getJitteredGrid. El stream empieza fresh.
    let mut rng = Alea::new(seed);
    let points = get_jittered_grid(graph_width, graph_height, spacing, &mut rng);
    let cells_x = ((graph_width + 0.5 * spacing - 1e-10_f64) / spacing).floor() as u32;
    let cells_y = ((graph_height + 0.5 * spacing - 1e-10_f64) / spacing).floor() as u32;
    PlacedGrid {
        spacing,
        cells_desired,
        boundary,
        points,
        cells_x,
        cells_y,
    }
}

/// `getBoundaryPoints(width, height, spacing)` de Azgaar (`graphUtils.ts:17-37`).
/// **No consume RNG** (fase-0 §6.6 confirmado).
fn get_boundary_points(width: f64, height: f64, spacing: f64) -> Vec<[f64; 2]> {
    // `offset = rn(-1 * spacing)` — default d=0, equivale a Math.round(-spacing).
    let offset = {
        // Clippy sugeriría `-spacing` pero mantenemos `-1.0 * spacing` para
        // reflejar textualmente el fuente de Azgaar `rn(-1 * spacing)` y minimizar
        // riesgo de reinterpretación aritmética en sub-versiones.
        #[allow(clippy::neg_multiply)]
        let v = -1.0 * spacing;
        rn(v, 0)
    };
    let b_spacing = spacing * 2.0;
    let w = width - offset * 2.0;
    let h = height - offset * 2.0;
    let number_x = (w / b_spacing).ceil() - 1.0;
    let number_y = (h / b_spacing).ceil() - 1.0;

    let mut points: Vec<[f64; 2]> = Vec::new();

    // `for (let i = 0.5; i < numberX; i++)` — iteraciones `i = 0.5, 1.5, ..., numberX-0.5`.
    let mut i = 0.5_f64;
    while i < number_x {
        let x = ((w * i) / number_x + offset).ceil();
        points.push([x, offset]);
        points.push([x, h + offset]);
        i += 1.0;
    }

    let mut i = 0.5_f64;
    while i < number_y {
        let y = ((h * i) / number_y + offset).ceil();
        points.push([offset, y]);
        points.push([w + offset, y]);
        i += 1.0;
    }

    points
}

/// `getJitteredGrid(width, height, spacing)` de Azgaar (`graphUtils.ts:46-61`).
/// Consume el PRNG `rng`: 2 floats por celda (xj primero, yj después), fila-mayor.
///
/// `Math.random` patcheado en el caller: pasamos el `Alea` externo ya re-seedeado
/// (en Voronia eso es `Alea::new(seed)` construido en `place_points`).
fn get_jittered_grid(width: f64, height: f64, spacing: f64, rng: &mut Alea) -> Vec<[f64; 2]> {
    let radius = spacing / 2.0;
    let jittering = radius * 0.9;
    let double_jittering = jittering * 2.0;
    // `jitter = () => Math.random() * doubleJittering - jittering`.
    let mut jitter = || rng.next_f64() * double_jittering - jittering;

    let mut points: Vec<[f64; 2]> = Vec::new();
    // Fila-mayor: y externo, x interno. Critíco: el órden de consumo del RNG.
    let mut y = radius;
    while y < height {
        let mut x = radius;
        while x < width {
            // `xj = Math.min(rn(x + jitter(), 2), width)`.
            // Ojo: el jitter() se consume una vez para xj, y _otra_ vez para yj.
            let xj = (rn(x + jitter(), 2)).min(width);
            let yj = (rn(y + jitter(), 2)).min(height);
            points.push([xj, yj]);
            x += spacing;
        }
        y += spacing;
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test determinismo: dos llamadas con misma seed + sames params => mismos puntos.
    #[test]
    fn place_points_is_deterministic() {
        let a = place_points(2000.0, 2000.0, 10000, "861039636");
        let b = place_points(2000.0, 2000.0, 10000, "861039636");
        assert_eq!(a.points, b.points);
        assert_eq!(a.boundary, b.boundary);
        assert_eq!(a.spacing, b.spacing);
        assert_eq!(a.cells_x, b.cells_y);
    }

    /// Brample-handshake: los primeros valores del Brample (`docs/fase-0-investigacion.md`
    /// §12.4) — `spacing=20`, `cellsX=cellsY=100`, boundary arranca con `[1,-20]` y `[1,2020]`.
    /// Esto afirma que los campos estructurales calzan antes de mirar los puntos.
    #[test]
    fn place_points_brample_sizing_matches() {
        let g = place_points(2000.0, 2000.0, 10000, "861039636");
        assert_eq!(g.spacing, 20.0, "spacing (Brample §12.4)");
        assert_eq!(g.cells_desired, 10000);
        assert_eq!(g.cells_x, 100, "cellsX (Brample §12.4)");
        assert_eq!(g.cells_y, 100, "cellsY (Brample §12.4)");
        // Validar primer boundary point: [1, -20] y [1, 2020] (Brample slot [6] confirmado).
        assert!(!g.boundary.is_empty(), "boundary no vacío");
        // El algoritmo produce [1, offset], [1, h+offset] como primeras dos entradas.
        assert_eq!(
            g.boundary[0],
            [1.0, -20.0],
            "primer boundary (Brample §12.4)"
        );
        assert_eq!(
            g.boundary[1],
            [1.0, 2020.0],
            "segundo boundary (Brample §12.4)"
        );
        // Cantidad de puntos jitterizados (fila-mayor).
        assert_eq!(g.points.len(), 100 * 100, "10000 puntos en grid 100×100");
    }
}
