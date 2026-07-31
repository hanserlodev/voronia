//! Jittered grid geometry — bit-exact port of Azgaar's `graphUtils.ts:17-98`.
//!
//! Azgaar's pipeline (confirmed against Brample in `docs/phases/phase-0-research.md`
//! §12.4, §13.3):
//! 1. `generateGrid(seed, w, h)` reseeds the PRNG with `Math.random = Alea(seed)` (npm).
//! 2. `placePoints(w, h)` → `spacing`, `cellsDesired`, `boundary` (no RNG), `points` (jitter, consumes RNG), `cellsX`, `cellsY`.
//! 3. `cells = calculateVoronoi(points, boundary)` in vor-import (`geometry::voronoi`).
//!
//! Important: cell id `k` corresponds to `points[k]`, and `cells.i[k] = k` in
//! the reconstructed Voronoi mesh. The Alea order consumes jitter row-major
//! (`y` outer, `x` inner), 2 floats per cell. If the port iterates in another order or
//! consumes the RNG in another sequence, the `points` will not match the `[6]` slot
//! of the `.map` and the attributes will end up in the wrong cells — a silent bug
//! (phase-0 §13.4).

use crate::numbers::rn;
use crate::prng::Alea;

pub mod delaunay;
pub mod voronoi;

/// Product of `place_points` — the grid fields before Voronoi.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedGrid {
    /// `spacing` rounded to 2 decimals.
    pub spacing: f64,
    /// Desired cell count (user input in Azgaar, or comes from the `[6]` slot).
    pub cells_desired: u32,
    /// Virtual border points to avoid infinite Voronoi cells. Do not consume RNG.
    pub boundary: Vec<[f64; 2]>,
    /// Jittered points in row-major order (`y` outer, `x` inner). Order determines `cells.i`.
    pub points: Vec<[f64; 2]>,
    /// Number of columns (`cellsX` in Azgaar).
    pub cells_x: u32,
    /// Number of rows (`cellsY` in Azgaar).
    pub cells_y: u32,
}

/// Azgaar's `placePoints(graphWidth, graphHeight)` (`graphUtils.ts:69-98`), bit-exact.
///
/// `cells_desired` is an input that in Azgaar comes from the UI (`pointsInput.dataset.cells`).
/// When Voronia imports a `.map` it takes it from the `[6]` slot (`cellsDesired: 10000` in Brample).
/// **We do not infer it from the UI** — it is an explicit input from the caller.
///
/// The `seed` is used to build the internal `Alea(seed)` — this replicates the effect of
/// monkey-patching `Math.random = Alea(seed)` in `generateGrid` (`graphUtils.ts:137`).
/// The first `Math.random` consumed by `get_jittered_grid` is the first stream of this Alea.
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
    // Reseeds the PRNG with Alea(seed) — equivalent to `Math.random = Alea(seed)` in
    // `generateGrid` before the first getJitteredGrid. The stream starts fresh.
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

/// Azgaar's `getBoundaryPoints(width, height, spacing)` (`graphUtils.ts:17-37`).
/// **Does not consume RNG** (phase-0 §6.6 confirmed).
fn get_boundary_points(width: f64, height: f64, spacing: f64) -> Vec<[f64; 2]> {
    // `offset = rn(-1 * spacing)` — default d=0, equivalent to Math.round(-spacing).
    let offset = {
        // Clippy would suggest `-spacing` but we keep `-1.0 * spacing` to
        // textually mirror Azgaar's source `rn(-1 * spacing)` and minimize
        // the risk of arithmetic reinterpretation in sub-versions.
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

    // `for (let i = 0.5; i < numberX; i++)` — iterations `i = 0.5, 1.5, ..., numberX-0.5`.
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

/// Azgaar's `getJitteredGrid(width, height, spacing)` (`graphUtils.ts:46-61`).
/// Consumes the `rng` PRNG: 2 floats per cell (xj first, yj after), row-major.
///
/// `Math.random` is patched by the caller: we pass the already-reseeded external `Alea`
/// (in Voronia that is `Alea::new(seed)` built in `place_points`).
fn get_jittered_grid(width: f64, height: f64, spacing: f64, rng: &mut Alea) -> Vec<[f64; 2]> {
    let radius = spacing / 2.0;
    let jittering = radius * 0.9;
    let double_jittering = jittering * 2.0;
    // `jitter = () => Math.random() * doubleJittering - jittering`.
    let mut jitter = || rng.next_f64() * double_jittering - jittering;

    let mut points: Vec<[f64; 2]> = Vec::new();
    // Row-major: y outer, x inner. Critical: the RNG consumption order.
    let mut y = radius;
    while y < height {
        let mut x = radius;
        while x < width {
            // `xj = Math.min(rn(x + jitter(), 2), width)`.
            // Note: jitter() is consumed once for xj, and _again_ for yj.
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

    /// Determinism test: two calls with same seed + same params => same points.
    #[test]
    fn place_points_is_deterministic() {
        let a = place_points(2000.0, 2000.0, 10000, "861039636");
        let b = place_points(2000.0, 2000.0, 10000, "861039636");
        assert_eq!(a.points, b.points);
        assert_eq!(a.boundary, b.boundary);
        assert_eq!(a.spacing, b.spacing);
        assert_eq!(a.cells_x, b.cells_y);
    }

    /// Brample-handshake: the first Brample values (`docs/phases/phase-0-research.md`
    /// §12.4) — `spacing=20`, `cellsX=cellsY=100`, boundary starts with `[1,-20]` and `[1,2020]`.
    /// This asserts that the structural fields match before looking at the points.
    #[test]
    fn place_points_brample_sizing_matches() {
        let g = place_points(2000.0, 2000.0, 10000, "861039636");
        assert_eq!(g.spacing, 20.0, "spacing (Brample §12.4)");
        assert_eq!(g.cells_desired, 10000);
        assert_eq!(g.cells_x, 100, "cellsX (Brample §12.4)");
        assert_eq!(g.cells_y, 100, "cellsY (Brample §12.4)");
        // Validate the first boundary point: [1, -20] and [1, 2020] (Brample slot [6] confirmed).
        assert!(!g.boundary.is_empty(), "boundary not empty");
        // The algorithm produces [1, offset], [1, h+offset] as the first two entries.
        assert_eq!(
            g.boundary[0],
            [1.0, -20.0],
            "first boundary (Brample §12.4)"
        );
        assert_eq!(
            g.boundary[1],
            [1.0, 2020.0],
            "second boundary (Brample §12.4)"
        );
        // Number of jittered points (row-major).
        assert_eq!(g.points.len(), 100 * 100, "10000 points in a 100×100 grid");
    }
}
