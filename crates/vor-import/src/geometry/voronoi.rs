//! Bit-exact port of Azgaar's `Voronoi` class (`src/generators/voronoi.ts`).
//!
//! Builds the dual Voronoi mesh from a Delaunay triangulation
//! (`crate::geometry::delaunay::Triangulation`) — 1-to-1 reproduction of the TS logic.
//!
//! ## `circumcenter` bit-exactness — critical
//!
//! Azgaar deliberately truncates the circumcenter to integers with `Math.floor`
//! (`voronoi.ts:151-152`). In Rust we reproduce it with `f64::floor()` — **not** with `as i32`
//! (which would truncate towards zero on negatives). Although cell coordinates are not
//! negative in Azgaar, the circumcenter can fall slightly outside the cell
//! range when the triangle is obtuse or has a hull point, and there `as i32`
//! would diverge from `Math.floor`. Finding phase-0 §6.3 requires literal reproduction.
//!
//! ## Output layout
//!
//! Output mapped to the `vor-core` types:
//!   - `cells.v[p] : Vec<u32>` → the 3+ triangle IDs (= Voronoi vertices) that form
//!     the cell of point `p`. Order: counter-clockwise via `edgesAroundPoint`.
//!   - `cells.c[p] : Vec<u32>` → adjacent cell IDs (interiors only —
//!     boundary points with id >= `pointsN` are filtered out).
//!   - `cells.b[p] : u8` → 1 if the cell touches the border (filtered neighbors != total
//!     neighbors), 0 otherwise.
//!   - `cells.i` is not materialized here (it is `[0,1,...,pointsN-1]`, implicit).
//!   - `vertices.p[t] : [f32;2]` → coords of the Voronoi vertex of triangle `t`.
//!     `f32` (fixed cap) — the `floor` already imposed the precision limit.
//!   - `vertices.v[t] : [i32;3]` → 3 neighboring triangles (one per opposite half-edge).
//!     `-1` = border (no neighbor).
//!   - `vertices.c[t] : [u32;3]` → 3 cells (points) that make up triangle `t`.

use crate::geometry::delaunay::{
    next_halfedge, points_of_triangle, triangle_of_edge, triangles_adjacent_to_triangle,
    Triangulation, EMPTY,
};

/// Cap of `edgesAroundPoint` (`voronoi.ts:87`). In the JS it is a safety cap
/// against infinite loops in meshes with half-edge bugs. In Rust we keep it for bit-exactness:
/// if a legitimate mesh exceeds 20 edges, Azgaar truncates silently and we do too.
const EDGES_AROUND_POINT_CAP: usize = 20;

/// Coordinates of a point (pair `[x, y]`).
pub type Point = [f64; 2];

/// Output of `calculate_voronoi` — equivalent to Azgaar's `Voronoi` class.
///
/// Cells (`cells.*`) are indexed by point-id `[0, pointsN)`.
/// Vertices (`vertices.*`) are indexed by triangle-id `[0, triangles.len()/3)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Voronoi {
    /// See `VoronoiCells`. `v`: vertex neighbors; `c`: cell neighbors; `b`: border flag.
    pub cells: VoronoiCells,
    /// See `VoronoiVertices`. `p`: coords; `v`: neighbors (triangles); `c`: adjacent cells.
    pub vertices: VoronoiVertices,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VoronoiCells {
    /// `cells.v[p]` — IDs of triangles (Voronoi vertices) that make up cell `p`,
    /// in CCW order. `Vec::new()` if the cell was not visited (case where point `p`
    /// is on the boundary and never became `triangles[nextHalfedge(e)]`).
    pub v: Vec<Vec<u32>>,
    /// `cells.c[p]` — adjacent cell IDs (interiors; boundary points filtered out).
    pub c: Vec<Vec<u32>>,
    /// `cells.b[p]` — 1 if the cell touches the border (some neighbors were filtered out), 0 otherwise.
    pub b: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VoronoiVertices {
    /// `vertices.p[t]` — coords `[x, y]` of the circumcenter of triangle `t`. **Integers**
    /// (`Math.floor` in JS); here `f32` for consistency with `vor-core::VoronoiVertices`.
    pub p: Vec<[f64; 2]>,
    /// `vertices.v[t]` — the 3 neighboring triangles (one per opposite half-edge). `EMPTY`
    /// marked as `None` to avoid confusion — the receiver decides whether to fill
    /// with `-1` or `u32::MAX`. In `vor-core::VoronoiVertices` it is stored as `i32` with
    /// `-1` for compat with Azgaar (`feature.rs` expects `[i32; 3]`).
    pub v: Vec<[usize; 3]>,
    /// `vertices.c[t]` — the 3 points (cells) that make up triangle `t`.
    pub c: Vec<[u32; 3]>,
}

/// Bit-exact replica of Azgaar's `new Voronoi(delaunay, points, pointsN)` constructor
/// (`voronoi.ts:25-50`).
///
/// `points` are all points (including boundary points), and `pointsN` is the number
/// of non-boundary points (boundary points have id `[pointsN, points.len())`).
pub fn calculate_voronoi(delaunay: &Triangulation, points: &[Point], points_n: u32) -> Voronoi {
    let n_triangles = delaunay.triangles.len() / 3;

    let mut cells_v: Vec<Vec<u32>> = vec![Vec::new(); points_n as usize];
    let mut cells_c: Vec<Vec<u32>> = vec![Vec::new(); points_n as usize];
    let mut cells_b: Vec<u8> = vec![0; points_n as usize];

    // `vertices.p[t]` is initialized as `None`/placeholder to distinguish "not set" from
    // "set to [0.0, 0.0]". We use an intermediate Vec<Option<[f64;2]>> — the JS relies
    // on `undefined` and `!this.vertices.p[t]`.
    let mut vertices_p: Vec<Option<[f64; 2]>> = vec![None; n_triangles];
    let mut vertices_v: Vec<[usize; 3]> = vec![[EMPTY; 3]; n_triangles];
    let mut vertices_c: Vec<[u32; 3]> = vec![[0; 3]; n_triangles];

    let triangles = delaunay.triangles.as_slice();
    let halfedges = delaunay.halfedges.as_slice();

    // The main loop replicates `voronoi.ts:34-49` line-by-line.
    for e in 0..delaunay.triangles.len() {
        let p = delaunay.triangles[next_halfedge(e)];
        // `if (p < pointsN && !cells.c[p])` — only interior and unvisited points.
        if p < points_n && cells_c.get(p as usize).is_none_or(|v| v.is_empty()) {
            // cells.v[p] = edges.map(e => triangleOfEdge(e))
            // cells.c[p] = edges.map(e => triangles[e]).filter(c => c < pointsN)
            // cells.b[p] = edges.length > cells.c[p].length ? 1 : 0
            let edges = edges_around_point(halfedges, e);
            let cell_v: Vec<u32> = edges.iter().map(|&e| triangle_of_edge(e) as u32).collect();
            let cell_c: Vec<u32> = edges
                .iter()
                .map(|&e| triangles[e])
                .filter(|&c| c < points_n)
                .collect();

            let is_border = if edges.len() > cell_c.len() { 1u8 } else { 0u8 };

            let pi = p as usize;
            cells_v[pi] = cell_v;
            cells_c[pi] = cell_c;
            cells_b[pi] = is_border;
        }

        let t = triangle_of_edge(e);
        // `if (!vertices.p[t])` — JS uses falsiness of `undefined`. Here we use `Option::is_none`.
        if vertices_p[t].is_none() {
            vertices_p[t] = Some(triangle_center(points, triangles, t));
            vertices_v[t] = triangles_adjacent_to_triangle(halfedges, t);
            vertices_c[t] = points_of_triangle(triangles, t);
        }
    }

    // The JS stores `vertices.p[t]` as `[number, number]`. In Azgaar, `vertices.p`
    // can be `undefined` if a triangle was not touched by the loop — but all
    // triangles appear via the loop over `triangles.length` (`for e in 0..triangles.len()`),
    // so every `t` is set. Even so, for safety we leave the unwrap as
    // `unwrap_or([0.0, 0.0])` to avoid panicking, and emit an assertion in debug builds.
    let vertices_p_final: Vec<[f64; 2]> = vertices_p
        .into_iter()
        .map(|opt| {
            debug_assert!(opt.is_some(), "vertex t was not populated");
            opt.unwrap_or([0.0, 0.0])
        })
        .collect();

    Voronoi {
        cells: VoronoiCells {
            v: cells_v,
            c: cells_c,
            b: cells_b,
        },
        vertices: VoronoiVertices {
            p: vertices_p_final,
            v: vertices_v,
            c: vertices_c,
        },
    }
}

/// `edgesAroundPoint(start)` of `voronoi.ts:80-89`.
///
/// Walks the half-edges touching the target point `start` (that is, all the
/// incoming/outgoing of point `triangles[start]`), CCW, with a cap of 20.
fn edges_around_point(halfedges: &[usize], start: usize) -> Vec<usize> {
    let mut result: Vec<usize> = Vec::new();
    let mut incoming = start;
    loop {
        result.push(incoming);
        let outgoing = next_halfedge(incoming);
        incoming = halfedges[outgoing];
        if incoming == EMPTY || incoming == start || result.len() >= EDGES_AROUND_POINT_CAP {
            break;
        }
    }
    result
}

/// `triangleCenter(t)` of `voronoi.ts:96-99` — the circumcenter of triangle `t`.
/// The coords are computed in f64 and truncated to integers with `f64::floor()`.
fn triangle_center(points: &[Point], triangles: &[u32], t: usize) -> [f64; 2] {
    let pts = points_of_triangle(triangles, t);
    let a = points[pts[0] as usize];
    let b = points[pts[1] as usize];
    let c = points[pts[2] as usize];
    circumcenter(a, b, c)
}

/// `circumcenter(a, b, c)` of `voronoi.ts:142-154` — Wikipedia's formula, with
/// `Math.floor` truncating the result to integers (phase-0 §6.3).
///
/// Literal reproduction of the JS:
/// ```js
/// const ad = ax*ax + ay*ay;
/// const bd = bx*bx + by*by;
/// const cd = cx*cx + cy*cy;
/// const D = 2 * (ax*(by-cy) + bx*(cy-ay) + cx*(ay-by));
/// return [ Math.floor((1/D) * (ad*(by-cy) + bd*(cy-ay) + cd*(ay-by))),
///          Math.floor((1/D) * (ad*(cx-bx) + bd*(ax-cx) + cd*(bx-ax))) ];
/// ```
///
/// Important for bit-exactness: the JS computes `(1/D) * numerator` — in f64 this is
/// arithmetically `numerator / D`, but **not** bit-identical. `(1/D)` computes the
/// f64 reciprocal (with its own rounding), then multiplies by `numerator`
/// (with another rounding) — 2 rounding profiles total. `numerator / D` applies a
/// single rounding. We reproduce the JS pattern: `recip * numerator`, not `numerator/D`.
fn circumcenter(a: Point, b: Point, c: Point) -> [f64; 2] {
    let [ax, ay] = a;
    let [bx, by] = b;
    let [cx, cy] = c;
    let ad = ax * ax + ay * ay;
    let bd = bx * bx + by * by;
    let cd = cx * cx + cy * cy;
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    let recip = 1.0 / d;
    let x = recip * (ad * (by - cy) + bd * (cy - ay) + cd * (ay - by));
    let y = recip * (ad * (cx - bx) + bd * (ax - cx) + cd * (bx - ax));
    [x.floor(), y.floor()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::delaunay::from_pairs;

    /// Basic sanity: 1×1 square with 4 points → 2 triangles, 4 cells (no border, no boundary).
    #[test]
    fn voronoi_square_no_boundary() {
        let points: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let delaunay = from_pairs(&points);
        let v = calculate_voronoi(&delaunay, &points, points.len() as u32);

        // 4 points = 4 cells (all populated), 2 triangles = 2 vertices.
        assert_eq!(v.cells.b.len(), 4, "cells.b has points_n entries");
        // Without boundary, all cells have neighbor type = number of edges.
        // (The cells are contiguous — everything is interior — b should be 0 everywhere.)
        // Actually, in a boundary-less square the 4 points are on the hull (Delaunator
        // puts them on `hull` automatically), so `vertices.v` will have EMPTYs.
        // We only validate the structural count.
        assert_eq!(v.vertices.p.len(), delaunay.triangles.len() / 3);
        assert_eq!(v.vertices.c.len(), delaunay.triangles.len() / 3);
    }

    /// Determinism: same input → same output.
    #[test]
    fn voronoi_is_deterministic() {
        let points: Vec<[f64; 2]> = vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [2.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [2.0, 1.0],
            [0.0, 2.0],
            [1.0, 2.0],
            [2.0, 2.0],
        ];
        let delaunay = from_pairs(&points);
        let a = calculate_voronoi(&delaunay, &points, points.len() as u32);
        let b = calculate_voronoi(&delaunay, &points, points.len() as u32);
        assert_eq!(a, b, "bit-exact determinism");
    }

    /// `circumcenter` of a unit isosceles right triangle:
    /// A=(0,0), B=(1,0), C=(0,1). Circumcenter = (0.5, 0.5). `floor(0.5)=0`. → [0, 0].
    #[test]
    fn circumcenter_unit_right_triangle() {
        let cc = circumcenter([0.0, 0.0], [1.0, 0.0], [0.0, 1.0]);
        // 1/D = 1/(2 * (0*(0-1) + 1*(1-0) + 0*(0-0))) = 1/2
        // x = 0.5*(0*(0-1) + 1*(1-0) + 1*(0-0)) = 0.5 * 1 = 0.5 → floor = 0
        // y = 0.5*(0*(0-1) + 1*(0-0) + 1*(1-0)) = 0.5 * 1 = 0.5 → floor = 0
        assert_eq!(cc, [0.0, 0.0], "isosceles right triangle circumcenter");
    }

    /// Equilateral triangle of side 2: A=(0,0), B=(2,0), C=(1,√3).
    /// Circumcenter = (1, √3/3) ≈ (1, 0.5773...) → floor = [1, 0].
    #[test]
    fn circumcenter_equilateral() {
        let sqrt3 = 3f64.sqrt();
        let cc = circumcenter([0.0, 0.0], [2.0, 0.0], [1.0, sqrt3]);
        // D = 2*(0*(0-√3) + 2*(√3-0) + 1*(0-0)) = 2*2*√3 = 4√3
        // x = (1/(4√3)) * (0 + 4*(√3-0) + 4*(0-0)) = (1/(4√3)) * 4√3 = 1.0 → floor = 1
        // y = (1/(4√3)) * (0*(1-2) + 4*(0-1) + 4*(2-0)) = (1/(4√3)) * (-4 + 8)
        //   = (1/(4√3)) * 4 = 1/√3 ≈ 0.5773502691... → floor = 0
        assert_eq!(cc, [1.0, 0.0], "equilateral circumcenter");
    }

    /// Bit-exactness of the circumcenter against JS `Math.floor` — negative case:
    /// triangle with the circumcenter in negative territory.
    /// A=(-2,-2), B=(0,-2), C=(-1,-1). Circumcenter ≈ (-1, -3).
    #[test]
    fn circumcenter_negative_floor() {
        // D = 2*((-2)*(-2-(-1)) + 0*(-1-(-2)) + (-1)*((-2)-(-2)))
        //   = 2*((-2)*(-1) + 0*1 + (-1)*0)
        //   = 2 * 2 = 4
        // x = (1/4) * (ad*(by-cy) + bd*(cy-ay) + cd*(ay-by))
        //   ad = 4+4=8; bd = 0+4=4; cd = 1+1=2.
        //   = (1/4) * (8*(-2-(-1)) + 4*(-1-(-2)) + 2*((-2)-(-2)))
        //   = (1/4) * (8*(-1) + 4*(1) + 2*0)
        //   = (1/4) * (-8+4+0) = (1/4) * -4 = -1.0 → floor(-1.0) = -1
        // y = (1/4) * (ad*(cx-bx) + bd*(ax-cx) + cd*(bx-ax))
        //   = (1/4) * (8*((-1)-0) + 4*((-2)-(-1)) + 2*(0-(-2)))
        //   = (1/4) * (8*(-1) + 4*(-1) + 2*2)
        //   = (1/4) * (-8 -4 + 4) = (1/4) * -8 = -2.0 → floor(-2.0) = -2
        let cc = circumcenter([-2.0, -2.0], [0.0, -2.0], [-1.0, -1.0]);
        assert_eq!(cc, [-1.0, -2.0], "negative circumcenter with floor");
    }
}
