//! Bit-exact port of `delaunator@5.1.0` (npm, Mapbox) — 2D Delaunay triangulation
//! algorithm. 1-to-1 replica of the JS source (`delaunator.js`, 880 lines), including the
//! inline robust predicates of Shewchuk (`orient2d`, `orient2dadapt`).
//!
//! ## Why this port (vs. using the `delaunator = "1.1"` crate from crates.io)
//!
//! The Rust `delaunator` crate, although a conceptual port of the same algorithm, is **not
//! bit-exact** against `delaunator@5.1.0` (the npm that Azgaar uses per
//! `azgaar-fmg/package-lock.json:1599`):
//!   1. Bug of `find_closest_point` (it filters `d > 0` indiscriminately).
//!   2. Differences in the `robust = "1.2"` (Rust) crate: sign of `orient2dadapt` not
//!      negated, `THETA` vs `ccwerrboundA` constants (second-order correction), diverging
//!      behavior on degenerate ties.
//!
//! Tested in `tests/delaunay_bit_exact.rs`: with the Rust crate, over
//! `place_points(2000,2000,10000,"861039636") + boundary`, 6280 `triangles` entries and
//! 12145 `halfedges` entries diverge from the JS. Accepting that divergence would break
//! the finding of fase-0 §13.4 (attributes land in the wrong cells).
//!
//! This port avoids all of that by sharing the code with the JS.
//!
//! ## Scratch buffers
//!
//! The JS uses module-level globals (`B`, `C1`, `C2`, `D`, `u`, `EDGE_STACK`) as reusable
//! scratch. In Rust we make them `thread_local!` with `RefCell` to keep single-thread
//! safety without having to pass the context everywhere.

//! ```js
//! // Public JS API:
//! //   Delaunator.from(points) → triangulates [x,y] pairs.
//! //   result.triangles, result.halfedges, result.hull
//! ```
#![allow(
    clippy::many_single_char_names,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::manual_swap,
    clippy::redundant_field_names,
    clippy::new_without_default,
    unused_assignments
)]
const EPSILON: f64 = 1.1102230246251565e-16;
const SPLITTER: f64 = 134_217_729.0;
const RESULTERRBOUND: f64 = (3.0 + 8.0 * EPSILON) * EPSILON;
const CCWERRBOUND_A: f64 = (3.0 + 16.0 * EPSILON) * EPSILON;
const CCWERRBOUND_B: f64 = (2.0 + 12.0 * EPSILON) * EPSILON;
const CCWERRBOUND_C: f64 = (9.0 + 64.0 * EPSILON) * EPSILON * EPSILON;
const EPS2_52: f64 = 2.220446049250313e-16; // = 2.0f64.powi(-52)
const EDGE_STACK_CAP: usize = 512;

/// `EMPTY` — index of a halfedge with no adjacent (on the outer hull). JS uses `-1` (Int32);
/// in Rust `usize::MAX` to always store `usize` in `halfedges`.
pub const EMPTY: usize = usize::MAX;

// Scratch buffers of the JS, (turned into thread_local RefCell for safe Rust):
//
//   const B = vec(4); const C1 = vec(8); const C2 = vec(12); const D = vec(16); const u = vec(4);
//   const EDGE_STACK = new Uint32Array(512);
//
// All lazily allocated with `thread_local!` and `RefCell::new(...)`.

use std::cell::RefCell;

thread_local! {
    static B_BUF: RefCell<[f64; 4]> = const { RefCell::new([0.0; 4]) };
    static U_BUF: RefCell<[f64; 4]> = const { RefCell::new([0.0; 4]) };
    static C1_BUF: RefCell<[f64; 8]> = const { RefCell::new([0.0; 8]) };
    static C2_BUF: RefCell<[f64; 12]> = const { RefCell::new([0.0; 12]) };
    static D_BUF: RefCell<[f64; 16]> = const { RefCell::new([0.0; 16]) };
    static EDGE_STACK: RefCell<[u32; EDGE_STACK_CAP]> = const { RefCell::new([0; EDGE_STACK_CAP]) };
}

/// Result of the triangulation — equivalent to the JS output:
/// `delaunay.triangles`, `delaunay.halfedges`, `delaunay.hull`.
#[derive(Debug, Clone)]
pub struct Triangulation {
    /// `Uint32Array` in JS. Each triple `(triangles[3t], triangles[3t+1], triangles[3t+2])`
    /// is a triangle, oriented counter-clockwise.
    pub triangles: Vec<u32>,
    /// `Int32Array` in JS. `halfedges[i]` is the index of the twin halfedge in the adjacent
    /// triangle, or `EMPTY` (= `-1` in JS) if it is an edge of the outer hull.
    pub halfedges: Vec<usize>,
    /// `Uint32Array` in JS. Indices of points on the convex hull, counter-clockwise.
    pub hull: Vec<u32>,
}

/// Bit-exact replica of `Delaunator.from(points)` (using `[x,y]` pairs).
///
/// Azgaar uses `Delaunator.from(allPoints)` with `allPoints = points.concat(boundary)` in
/// `voronoi.ts` (`calculateVoronoi`). For bit-exactness, pass the same points in the
/// same order.
pub fn from_pairs(points: &[[f64; 2]]) -> Triangulation {
    // Flattened coords: [x0,y0,x1,y1,...] — exact replica of the JS `Float64Array(n*2)`.
    let n = points.len();
    let mut coords = Vec::with_capacity(n * 2);
    for p in points {
        coords.push(p[0]);
        coords.push(p[1]);
    }
    triangulate(&coords)
}

/// Bit-exact replica of the `new Delaunator(coords)` + `update()` constructor of the JS.
///
/// `coords` is a flat array `[x0,y0,x1,y1,...]` (in JS, `Float64Array`).
pub fn triangulate(coords: &[f64]) -> Triangulation {
    let n = coords.len() >> 1;
    if n == 0 {
        return Triangulation {
            triangles: Vec::new(),
            halfedges: Vec::new(),
            hull: Vec::new(),
        };
    }

    let max_triangles = if n > 2 { 2 * n - 5 } else { 0 };

    // Arrays that the JS initializes in the constructor:
    let mut triangles: Vec<u32> = vec![0; max_triangles * 3];
    let mut halfedges: Vec<usize> = vec![EMPTY; max_triangles * 3];

    // Temporaries for hull tracking:
    let hash_size = (n as f64).sqrt().ceil() as usize;
    let mut hull_prev: Vec<u32> = vec![0; n];
    let mut hull_next: Vec<u32> = vec![0; n];
    let mut hull_tri: Vec<u32> = vec![0; n];
    let mut hull_hash: Vec<i32> = vec![-1; hash_size];

    // For sorting:
    let mut ids: Vec<u32> = (0..n as u32).collect();
    let mut dists: Vec<f64> = vec![0.0; n];

    // Mutables of the constructor:
    let mut triangles_len: usize = 0;
    let mut cx: f64 = 0.0;
    let mut cy: f64 = 0.0;
    let mut hull_start: u32 = 0;

    // === update() ===

    // populate _ids + bbox.
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for i in 0..n {
        let x = coords[2 * i];
        let y = coords[2 * i + 1];
        if x < min_x {
            min_x = x;
        }
        if y < min_y {
            min_y = y;
        }
        if x > max_x {
            max_x = x;
        }
        if y > max_y {
            max_y = y;
        }
        ids[i] = i as u32;
    }
    cx = (min_x + max_x) / 2.0;
    cy = (min_y + max_y) / 2.0;

    let mut i0: u32 = 0;
    let mut i1: u32 = 0;
    let mut i2: u32 = 0;

    // pick a seed point close to the center — JS: `if (d < minDist)`, no `d > 0` filter.
    {
        let mut min_dist = f64::INFINITY;
        for i in 0..n {
            let d = dist(cx, cy, coords[2 * i], coords[2 * i + 1]);
            if d < min_dist {
                i0 = i as u32;
                min_dist = d;
            }
        }
    }
    let i0x = coords[2 * i0 as usize];
    let i0y = coords[2 * i0 as usize + 1];

    // find the point closest to the seed — JS: `if (i === i0) continue; if (d < minDist && d > 0)`.
    {
        let mut min_dist = f64::INFINITY;
        for i in 0..n {
            if i as u32 == i0 {
                continue;
            }
            let d = dist(i0x, i0y, coords[2 * i], coords[2 * i + 1]);
            if d < min_dist && d > 0.0 {
                i1 = i as u32;
                min_dist = d;
            }
        }
    }
    let mut i1x = coords[2 * i1 as usize];
    let mut i1y = coords[2 * i1 as usize + 1];

    // find the third point forming smallest circumcircle.
    let mut min_radius = f64::INFINITY;
    for i in 0..n {
        if i as u32 == i0 || i as u32 == i1 {
            continue;
        }
        let r = circumradius(i0x, i0y, i1x, i1y, coords[2 * i], coords[2 * i + 1]);
        if r < min_radius {
            i2 = i as u32;
            min_radius = r;
        }
    }
    let mut i2x = coords[2 * i2 as usize];
    let mut i2y = coords[2 * i2 as usize + 1];

    if min_radius == f64::INFINITY {
        // order collinear points by dx (or dy if all x are identical) and return the list as a hull.
        for i in 0..n {
            let dx_or_dy = if coords[2 * i] - coords[0] != 0.0 {
                coords[2 * i] - coords[0]
            } else {
                coords[2 * i + 1] - coords[1]
            };
            dists[i] = dx_or_dy;
        }
        quicksort(&mut ids, &dists, 0, (n - 1) as i64);
        let mut hull: Vec<u32> = Vec::with_capacity(n);
        let mut d0 = f64::NEG_INFINITY;
        for i in 0..n {
            let id = ids[i] as usize;
            let d = dists[id];
            if d > d0 {
                hull.push(id as u32);
                d0 = d;
            }
        }
        return Triangulation {
            triangles: Vec::new(),
            halfedges: Vec::new(),
            hull,
        };
    }

    // swap the order of the seed points for counter-clockwise orientation.
    if orient2d(i0x, i0y, i1x, i1y, i2x, i2y) < 0.0 {
        let i = i1;
        let x = i1x;
        let y = i1y;
        i1 = i2;
        i1x = i2x;
        i1y = i2y;
        i2 = i;
        i2x = x;
        i2y = y;
    }

    let center = circumcenter(i0x, i0y, i1x, i1y, i2x, i2y);
    cx = center.0;
    cy = center.1;

    for i in 0..n {
        dists[i] = dist(coords[2 * i], coords[2 * i + 1], center.0, center.1);
    }

    // sort the points by distance from the seed triangle circumcenter.
    quicksort(&mut ids, &dists, 0, (n - 1) as i64);

    // set up the seed triangle as the starting hull.
    hull_start = i0;
    let mut hull_size: u32 = 3;
    hull_next[i0 as usize] = i1;
    hull_prev[i2 as usize] = i1;
    hull_next[i1 as usize] = i2;
    hull_prev[i0 as usize] = i2;
    hull_next[i2 as usize] = i0;
    hull_prev[i1 as usize] = i0;

    hull_tri[i0 as usize] = 0;
    hull_tri[i1 as usize] = 1;
    hull_tri[i2 as usize] = 2;

    for h in hull_hash.iter_mut() {
        *h = -1;
    }
    hull_hash[hash_key(cx, cy, i0x, i0y, hash_size) as usize] = i0 as i32;
    hull_hash[hash_key(cx, cy, i1x, i1y, hash_size) as usize] = i1 as i32;
    hull_hash[hash_key(cx, cy, i2x, i2y, hash_size) as usize] = i2 as i32;

    triangles_len = 0;
    add_triangle(
        &mut triangles,
        &mut halfedges,
        &mut triangles_len,
        i0 as usize,
        i1 as usize,
        i2 as usize,
        EMPTY,
        EMPTY,
        EMPTY,
    );

    // Main iteration.
    let mut xp: f64 = 0.0;
    let mut yp: f64 = 0.0;
    for k in 0..n {
        let i = ids[k] as usize;
        let x = coords[2 * i];
        let y = coords[2 * i + 1];

        // skip near-duplicate points: `if (k > 0 && Math.abs(x - xp) <= EPSILON && Math.abs(y - yp) <= EPSILON) continue;`.
        if k > 0 && (x - xp).abs() <= EPS2_52 && (y - yp).abs() <= EPS2_52 {
            continue;
        }
        xp = x;
        yp = y;

        // skip seed triangle points.
        if i as u32 == i0 || i as u32 == i1 || i as u32 == i2 {
            continue;
        }

        // find a visible edge on the convex hull using edge hash.
        // JS: `for (let j = 0, key = this._hashKey(x, y); j < this._hashSize; j++)`.
        // Note: in JS `start` stays at -1 if none is found (because `hullHash` is filled with -1).
        // In Rust, negative i32 cast to u32 → 0xFFFFFFFF; better to keep signed.
        let mut start_signed: i32 = -1;
        let key = hash_key(cx, cy, x, y, hash_size) as usize;
        for j in 0..hash_size {
            let s = hull_hash[(key + j) % hash_size];
            if s != -1 && s as u32 != hull_next[s as usize] {
                start_signed = s;
                break;
            }
        }
        if start_signed == -1 {
            continue; // shouldn't happen for non-degenerate inputs — matches JS `if (e === -1) continue;`
        }

        let mut start: u32 = start_signed as u32;
        start = hull_prev[start as usize]; // matches JS `start = hullPrev[start]`.
        let mut e: u32 = start;
        // walk forward while not visible:
        let mut no_visible = false;
        loop {
            let q = hull_next[e as usize];
            if orient2d(
                x,
                y,
                coords[2 * e as usize],
                coords[2 * e as usize + 1],
                coords[2 * q as usize],
                coords[2 * q as usize + 1],
            ) >= 0.0
            {
                e = q;
                if e == start {
                    no_visible = true;
                    break;
                }
            } else {
                break;
            }
        }
        if no_visible {
            continue; // likely a near-duplicate point; skip it
        }

        // add the first triangle from the point.
        let mut t = add_triangle(
            &mut triangles,
            &mut halfedges,
            &mut triangles_len,
            e as usize,
            i,
            hull_next[e as usize] as usize,
            EMPTY,
            EMPTY,
            hull_tri[e as usize] as usize,
        );
        hull_tri[i] = legalize(
            &mut triangles,
            &mut halfedges,
            coords,
            t + 2,
            &hull_prev,
            &hull_next,
            &mut hull_tri,
            &mut hull_start,
        );
        hull_tri[e as usize] = t as u32; // keep track of boundary triangles on the hull
        hull_size += 1;

        // walk forward through the hull, adding more triangles and flipping recursively.
        let mut n_node: u32 = hull_next[e as usize];
        loop {
            let q = hull_next[n_node as usize];
            if orient2d(
                x,
                y,
                coords[2 * n_node as usize],
                coords[2 * n_node as usize + 1],
                coords[2 * q as usize],
                coords[2 * q as usize + 1],
            ) < 0.0
            {
                t = add_triangle(
                    &mut triangles,
                    &mut halfedges,
                    &mut triangles_len,
                    n_node as usize,
                    i,
                    q as usize,
                    hull_tri[i] as usize,
                    EMPTY,
                    hull_tri[n_node as usize] as usize,
                );
                hull_tri[i] = legalize(
                    &mut triangles,
                    &mut halfedges,
                    coords,
                    t + 2,
                    &hull_prev,
                    &hull_next,
                    &mut hull_tri,
                    &mut hull_start,
                );
                hull_next[n_node as usize] = n_node; // mark as removed
                hull_size -= 1;
                n_node = q;
            } else {
                break;
            }
        }

        // walk backward from the other side, adding more triangles and flipping.
        if e == start {
            loop {
                let q = hull_prev[e as usize];
                if orient2d(
                    x,
                    y,
                    coords[2 * q as usize],
                    coords[2 * q as usize + 1],
                    coords[2 * e as usize],
                    coords[2 * e as usize + 1],
                ) < 0.0
                {
                    t = add_triangle(
                        &mut triangles,
                        &mut halfedges,
                        &mut triangles_len,
                        q as usize,
                        i,
                        e as usize,
                        EMPTY,
                        hull_tri[e as usize] as usize,
                        hull_tri[q as usize] as usize,
                    );
                    legalize(
                        &mut triangles,
                        &mut halfedges,
                        coords,
                        t + 2,
                        &hull_prev,
                        &hull_next,
                        &mut hull_tri,
                        &mut hull_start,
                    );
                    hull_tri[q as usize] = t as u32;
                    hull_next[e as usize] = e; // mark as removed
                    hull_size -= 1;
                    e = q;
                } else {
                    break;
                }
            }
        }

        // update the hull indices.
        hull_prev[i] = e;
        hull_start = e;
        hull_next[e as usize] = i as u32;
        hull_prev[n_node as usize] = i as u32;
        hull_next[i] = n_node;

        // save the two new edges in the hash table.
        hull_hash[hash_key(cx, cy, x, y, hash_size) as usize] = i as i32;
        hull_hash[hash_key(
            cx,
            cy,
            coords[2 * e as usize],
            coords[2 * e as usize + 1],
            hash_size,
        ) as usize] = e as i32;
    }

    // hull output.
    let mut hull: Vec<u32> = Vec::with_capacity(hull_size as usize);
    let mut e = hull_start;
    for _ in 0..hull_size {
        hull.push(e);
        e = hull_next[e as usize];
    }

    // trim arrays.
    triangles.truncate(triangles_len);
    halfedges.truncate(triangles_len);

    Triangulation {
        triangles,
        halfedges,
        hull,
    }
}

#[inline]
fn hash_key(cx: f64, cy: f64, x: f64, y: f64, hash_size: usize) -> i64 {
    let r = pseudo_angle(x - cx, y - cy) * (hash_size as f64);
    (r.floor() as i64) % (hash_size as i64)
}

#[inline]
fn pseudo_angle(dx: f64, dy: f64) -> f64 {
    let p = dx / (dx.abs() + dy.abs());
    (if dy > 0.0 { 3.0 - p } else { 1.0 + p }) / 4.0
}

#[inline]
fn dist(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

#[inline]
fn in_circle(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64, px: f64, py: f64) -> bool {
    let dx = ax - px;
    let dy = ay - py;
    let ex = bx - px;
    let ey = by - py;
    let fx = cx - px;
    let fy = cy - py;

    let ap = dx * dx + dy * dy;
    let bp = ex * ex + ey * ey;
    let cp = fx * fx + fy * fy;

    dx * (ey * cp - bp * fy) - dy * (ex * cp - bp * fx) + ap * (ex * fy - ey * fx) < 0.0
}

#[inline]
fn circumradius(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let ex = cx - ax;
    let ey = cy - ay;

    let bl = dx * dx + dy * dy;
    let cl = ex * ex + ey * ey;
    let d = 0.5 / (dx * ey - dy * ex);

    let x = (ey * bl - dy * cl) * d;
    let y = (dx * cl - ex * bl) * d;

    x * x + y * y
}

#[inline]
fn circumcenter(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> (f64, f64) {
    let dx = bx - ax;
    let dy = by - ay;
    let ex = cx - ax;
    let ey = cy - ay;

    let bl = dx * dx + dy * dy;
    let cl = ex * ex + ey * ey;
    let d = 0.5 / (dx * ey - dy * ex);

    let x = ax + (ey * bl - dy * cl) * d;
    let y = ay + (dx * cl - ex * bl) * d;
    (x, y)
}

/// `swap(arr, i, j)` JS — swaps `arr[i]` and `arr[j]`.
#[inline]
fn swap_u32(arr: &mut [u32], i: usize, j: usize) {
    let tmp = arr[i];
    arr[i] = arr[j];
    arr[j] = tmp;
}

/// `quicksort(ids, dists, left, right)` JS — identical to the JS (unstable, deterministic).
fn quicksort(ids: &mut [u32], dists: &[f64], left: i64, right: i64) {
    if right - left <= 20 {
        for i in left + 1..=right {
            let temp = ids[i as usize];
            let temp_dist = dists[temp as usize];
            let mut j = i - 1;
            while j >= left && dists[ids[j as usize] as usize] > temp_dist {
                ids[(j + 1) as usize] = ids[j as usize];
                j -= 1;
            }
            ids[(j + 1) as usize] = temp;
        }
    } else {
        let median = (left + right) >> 1;
        let mut i = left + 1;
        let mut j = right;
        swap_u32(ids, median as usize, i as usize);
        if dists[ids[left as usize] as usize] > dists[ids[right as usize] as usize] {
            swap_u32(ids, left as usize, right as usize);
        }
        if dists[ids[i as usize] as usize] > dists[ids[right as usize] as usize] {
            swap_u32(ids, i as usize, right as usize);
        }
        if dists[ids[left as usize] as usize] > dists[ids[i as usize] as usize] {
            swap_u32(ids, left as usize, i as usize);
        }

        let temp = ids[i as usize];
        let temp_dist = dists[temp as usize];
        loop {
            loop {
                i += 1;
                if dists[ids[i as usize] as usize] >= temp_dist {
                    break;
                }
            }
            loop {
                j -= 1;
                if dists[ids[j as usize] as usize] <= temp_dist {
                    break;
                }
            }
            if j < i {
                break;
            }
            swap_u32(ids, i as usize, j as usize);
        }
        ids[(left + 1) as usize] = ids[j as usize];
        ids[j as usize] = temp;

        if right - i + 1 >= j - left {
            quicksort(ids, dists, i, right);
            quicksort(ids, dists, left, j - 1);
        } else {
            quicksort(ids, dists, left, j - 1);
            quicksort(ids, dists, i, right);
        }
    }
}

/// `_addTriangle(i0, i1, i2, a, b, c)` of the JS — appends a new triangle and links halfedges.
#[inline]
fn add_triangle(
    triangles: &mut [u32],
    halfedges: &mut [usize],
    triangles_len: &mut usize,
    i0: usize,
    i1: usize,
    i2: usize,
    a: usize,
    b: usize,
    c: usize,
) -> usize {
    let t = *triangles_len;
    triangles[t] = i0 as u32;
    triangles[t + 1] = i1 as u32;
    triangles[t + 2] = i2 as u32;
    link(halfedges, t, a);
    link(halfedges, t + 1, b);
    link(halfedges, t + 2, c);
    *triangles_len += 3;
    t
}

/// `_link(a, b)` of the JS — `halfedges[a] = b; if (b !== -1) halfedges[b] = a;`.
#[inline]
fn link(halfedges: &mut [usize], a: usize, b: usize) {
    halfedges[a] = b;
    if b != EMPTY {
        halfedges[b] = a;
    }
}

/// `_legalize(a)` of the JS — returns `ar`.
#[allow(clippy::too_many_arguments)]
fn legalize(
    triangles: &mut [u32],
    halfedges: &mut [usize],
    coords: &[f64],
    mut a: usize,
    hull_prev: &[u32],
    _hull_next: &[u32],
    hull_tri: &mut [u32],
    hull_start: &mut u32,
) -> u32 {
    let mut i: usize = 0;
    let mut ar: usize = 0;

    // recursion eliminated with a fixed-size stack
    loop {
        let b = halfedges[a];

        let a0 = a - a % 3;
        ar = a0 + (a + 2) % 3;

        if b == EMPTY {
            // convex hull edge
            if i == 0 {
                break;
            }
            i -= 1;
            EDGE_STACK.with(|stack| {
                a = stack.borrow()[i] as usize;
            });
            continue;
        }

        let b0 = b - b % 3;
        let al = a0 + (a + 1) % 3;
        let bl = b0 + (b + 2) % 3;

        let p0 = triangles[ar] as usize;
        let pr = triangles[a] as usize;
        let pl = triangles[al] as usize;
        let p1 = triangles[bl] as usize;

        let illegal = in_circle(
            coords[2 * p0],
            coords[2 * p0 + 1],
            coords[2 * pr],
            coords[2 * pr + 1],
            coords[2 * pl],
            coords[2 * pl + 1],
            coords[2 * p1],
            coords[2 * p1 + 1],
        );

        if illegal {
            triangles[a] = p1 as u32;
            triangles[b] = p0 as u32;

            let hbl = halfedges[bl];

            // edge swapped on the other side of the hull (rare); fix the half-edge reference
            if hbl == EMPTY {
                let mut e = *hull_start;
                loop {
                    if hull_tri[e as usize] == bl as u32 {
                        hull_tri[e as usize] = a as u32;
                        break;
                    }
                    e = hull_prev[e as usize];
                    if e == *hull_start {
                        break;
                    }
                }
            }
            link(halfedges, a, hbl);
            link(halfedges, b, halfedges[ar]);
            link(halfedges, ar, bl);

            let br = b0 + (b + 1) % 3;

            // don't worry about hitting the cap: it can only happen on extremely degenerate input
            if i < EDGE_STACK_CAP {
                EDGE_STACK.with(|stack| {
                    stack.borrow_mut()[i] = br as u32;
                });
                i += 1;
            }
        } else {
            if i == 0 {
                break;
            }
            i -= 1;
            EDGE_STACK.with(|stack| {
                a = stack.borrow()[i] as usize;
            });
        }
    }

    ar as u32
}

// === Robust predicates (bit-exact against the JS) ===

/// Replica of `sum(elen, e, flen, f, h)` of the JS (`fast_expansion_sum_zeroelim`).
/// Writes the result into `h` (preallocated >= elen + flen) and returns the length used.
fn sum(elen: usize, e: &[f64], flen: usize, f: &[f64], h: &mut [f64]) -> usize {
    let mut q;
    let mut q_new;
    let mut hh;
    let mut bvirt;
    let mut eindex = 0usize;
    let mut findex = 0usize;
    let mut enow = e[0];
    let mut fnow = f[0];

    if (fnow > enow) == (fnow > -enow) {
        q = enow;
        eindex += 1;
        if eindex < elen {
            enow = e[eindex];
        }
    } else {
        q = fnow;
        findex += 1;
        if findex < flen {
            fnow = f[findex];
        }
    }
    let mut hindex = 0usize;
    if eindex < elen && findex < flen {
        if (fnow > enow) == (fnow > -enow) {
            q_new = enow + q;
            hh = q - (q_new - enow);
            eindex += 1;
            if eindex < elen {
                enow = e[eindex];
            }
        } else {
            q_new = fnow + q;
            hh = q - (q_new - fnow);
            findex += 1;
            if findex < flen {
                fnow = f[findex];
            }
        }
        q = q_new;
        if hh != 0.0 {
            h[hindex] = hh;
            hindex += 1;
        }
        while eindex < elen && findex < flen {
            if (fnow > enow) == (fnow > -enow) {
                q_new = q + enow;
                bvirt = q_new - q;
                hh = q - (q_new - bvirt) + (enow - bvirt);
                eindex += 1;
                if eindex < elen {
                    enow = e[eindex];
                }
            } else {
                q_new = q + fnow;
                bvirt = q_new - q;
                hh = q - (q_new - bvirt) + (fnow - bvirt);
                findex += 1;
                if findex < flen {
                    fnow = f[findex];
                }
            }
            q = q_new;
            if hh != 0.0 {
                h[hindex] = hh;
                hindex += 1;
            }
        }
    }
    while eindex < elen {
        q_new = q + enow;
        bvirt = q_new - q;
        hh = q - (q_new - bvirt) + (enow - bvirt);
        eindex += 1;
        if eindex < elen {
            enow = e[eindex];
        }
        q = q_new;
        if hh != 0.0 {
            h[hindex] = hh;
            hindex += 1;
        }
    }
    while findex < flen {
        q_new = q + fnow;
        bvirt = q_new - q;
        hh = q - (q_new - bvirt) + (fnow - bvirt);
        findex += 1;
        if findex < flen {
            fnow = f[findex];
        }
        q = q_new;
        if hh != 0.0 {
            h[hindex] = hh;
            hindex += 1;
        }
    }
    if q != 0.0 || hindex == 0 {
        h[hindex] = q;
        hindex += 1;
    }
    hindex
}

#[inline]
fn estimate(elen: usize, e: &[f64]) -> f64 {
    let mut q = e[0];
    for i in 1..elen {
        q += e[i];
    }
    q
}

/// `orient2d` of the JS — uses `orient2dadapt` only if the |det| is small relative to `detsum`.
pub fn orient2d(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> f64 {
    let detleft = (ay - cy) * (bx - cx);
    let detright = (ax - cx) * (by - cy);
    let det = detleft - detright;

    let detsum = (detleft + detright).abs();
    if det.abs() >= CCWERRBOUND_A * detsum {
        return det;
    }

    -orient2dadapt(ax, ay, bx, by, cx, cy, detsum)
}

/// `orient2dadapt` of the JS — adaptive-precision path for near-ties.
/// Bit-exact against the JS — 1-to-1 replication.
#[allow(
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::just_underscores_and_digits
)]
fn orient2dadapt(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64, detsum: f64) -> f64 {
    let (acxtail, acytail, bcxtail, bcytail);

    let acx = ax - cx;
    let bcx = bx - cx;
    let acy = ay - cy;
    let bcy = by - cy;

    let mut bvirt;
    let mut c;
    let mut ahi;
    let mut alo;
    let mut bhi;
    let mut blo;
    let mut s1;
    let mut s0;
    let mut t1;
    let mut t0;
    let mut u3;

    s1 = acx * bcy;
    c = SPLITTER * acx;
    ahi = c - (c - acx);
    alo = acx - ahi;
    c = SPLITTER * bcy;
    bhi = c - (c - bcy);
    blo = bcy - bhi;
    s0 = alo * blo - (s1 - ahi * bhi - alo * bhi - ahi * blo);
    t1 = acy * bcx;
    c = SPLITTER * acy;
    ahi = c - (c - acy);
    alo = acy - ahi;
    c = SPLITTER * bcx;
    bhi = c - (c - bcx);
    blo = bcx - bhi;
    t0 = alo * blo - (t1 - ahi * bhi - alo * bhi - ahi * blo);
    let _i = s0 - t0;
    bvirt = s0 - _i;
    let mut b_buf = [0f64; 4];
    b_buf[0] = s0 - (_i + bvirt) + (bvirt - t0);
    let _j = s1 + _i;
    bvirt = _j - s1;
    let _0 = s1 - (_j - bvirt) + (_i - bvirt);
    let _i = _0 - t1;
    bvirt = _0 - _i;
    b_buf[1] = _0 - (_i + bvirt) + (bvirt - t1);
    u3 = _j + _i;
    bvirt = u3 - _j;
    b_buf[2] = _j - (u3 - bvirt) + (_i - bvirt);
    b_buf[3] = u3;

    let det = estimate(4, &b_buf);
    let errbound = CCWERRBOUND_B * detsum;
    if det >= errbound || -det >= errbound {
        return det;
    }

    bvirt = ax - acx;
    acxtail = ax - (acx + bvirt) + (bvirt - cx);
    bvirt = bx - bcx;
    bcxtail = bx - (bcx + bvirt) + (bvirt - cx);
    bvirt = ay - acy;
    acytail = ay - (acy + bvirt) + (bvirt - cy);
    bvirt = by - bcy;
    bcytail = by - (bcy + bvirt) + (bvirt - cy);

    if acxtail == 0.0 && acytail == 0.0 && bcxtail == 0.0 && bcytail == 0.0 {
        return det;
    }

    let errbound = CCWERRBOUND_C * detsum + RESULTERRBOUND * det.abs();
    let det = det + (acx * bcytail + bcy * acxtail) - (acy * bcxtail + bcx * acytail);
    if det >= errbound || -det >= errbound {
        return det;
    }

    let mut u_buf = [0f64; 4];
    let mut c1_buf = [0f64; 8];
    let mut c2_buf = [0f64; 12];
    let mut d_buf = [0f64; 16];

    s1 = acxtail * bcy;
    c = SPLITTER * acxtail;
    ahi = c - (c - acxtail);
    alo = acxtail - ahi;
    c = SPLITTER * bcy;
    bhi = c - (c - bcy);
    blo = bcy - bhi;
    s0 = alo * blo - (s1 - ahi * bhi - alo * bhi - ahi * blo);
    t1 = acytail * bcx;
    c = SPLITTER * acytail;
    ahi = c - (c - acytail);
    alo = acytail - ahi;
    c = SPLITTER * bcx;
    bhi = c - (c - bcx);
    blo = bcx - bhi;
    t0 = alo * blo - (t1 - ahi * bhi - alo * bhi - ahi * blo);
    let _i = s0 - t0;
    bvirt = s0 - _i;
    u_buf[0] = s0 - (_i + bvirt) + (bvirt - t0);
    let _j = s1 + _i;
    bvirt = _j - s1;
    let _0 = s1 - (_j - bvirt) + (_i - bvirt);
    let _i = _0 - t1;
    bvirt = _0 - _i;
    u_buf[1] = _0 - (_i + bvirt) + (bvirt - t1);
    u3 = _j + _i;
    bvirt = u3 - _j;
    u_buf[2] = _j - (u3 - bvirt) + (_i - bvirt);
    u_buf[3] = u3;
    let c1len = sum(4, &b_buf, 4, &u_buf, &mut c1_buf);

    s1 = acx * bcytail;
    c = SPLITTER * acx;
    ahi = c - (c - acx);
    alo = acx - ahi;
    c = SPLITTER * bcytail;
    bhi = c - (c - bcytail);
    blo = bcytail - bhi;
    s0 = alo * blo - (s1 - ahi * bhi - alo * bhi - ahi * blo);
    t1 = acy * bcxtail;
    c = SPLITTER * acy;
    ahi = c - (c - acy);
    alo = acy - ahi;
    c = SPLITTER * bcxtail;
    bhi = c - (c - bcxtail);
    blo = bcxtail - bhi;
    t0 = alo * blo - (t1 - ahi * bhi - alo * bhi - ahi * blo);
    let _i = s0 - t0;
    bvirt = s0 - _i;
    u_buf[0] = s0 - (_i + bvirt) + (bvirt - t0);
    let _j = s1 + _i;
    bvirt = _j - s1;
    let _0 = s1 - (_j - bvirt) + (_i - bvirt);
    let _i = _0 - t1;
    bvirt = _0 - _i;
    u_buf[1] = _0 - (_i + bvirt) + (bvirt - t1);
    u3 = _j + _i;
    bvirt = u3 - _j;
    u_buf[2] = _j - (u3 - bvirt) + (_i - bvirt);
    u_buf[3] = u3;
    let c2len = sum(c1len, &c1_buf, 4, &u_buf, &mut c2_buf);

    s1 = acxtail * bcytail;
    c = SPLITTER * acxtail;
    ahi = c - (c - acxtail);
    alo = acxtail - ahi;
    c = SPLITTER * bcytail;
    bhi = c - (c - bcytail);
    blo = bcytail - bhi;
    s0 = alo * blo - (s1 - ahi * bhi - alo * bhi - ahi * blo);
    t1 = acytail * bcxtail;
    c = SPLITTER * acytail;
    ahi = c - (c - acytail);
    alo = acytail - ahi;
    c = SPLITTER * bcxtail;
    bhi = c - (c - bcxtail);
    blo = bcxtail - bhi;
    t0 = alo * blo - (t1 - ahi * bhi - alo * bhi - ahi * blo);
    let _i = s0 - t0;
    bvirt = s0 - _i;
    u_buf[0] = s0 - (_i + bvirt) + (bvirt - t0);
    let _j = s1 + _i;
    bvirt = _j - s1;
    let _0 = s1 - (_j - bvirt) + (_i - bvirt);
    let _i = _0 - t1;
    bvirt = _0 - _i;
    u_buf[1] = _0 - (_i + bvirt) + (bvirt - t1);
    u3 = _j + _i;
    bvirt = u3 - _j;
    u_buf[2] = _j - (u3 - bvirt) + (_i - bvirt);
    u_buf[3] = u3;
    let dlen = sum(c2len, &c2_buf, 4, &u_buf, &mut d_buf);

    d_buf[dlen - 1]
}

// === Half-edge helpers (public for `voronoi.rs`) ===
//
// 1-to-1 replica of the static methods of `delaunator@5.1.0.js` documented in
// https://mapbox.github.io/delaunator/#edge-and-triangle — the same helpers that
// `voronoi.ts:107-126` consumes as `triangleOfEdge`, `nextHalfedge`, etc.

/// `Math.floor(e / 3)` — index of the triangle that owns half-edge `e`.
#[inline]
pub fn triangle_of_edge(e: usize) -> usize {
    e / 3
}

/// Next half-edge of the same triangle (e % 3 == 2 → e - 2, otherwise e + 1).
#[inline]
pub fn next_halfedge(e: usize) -> usize {
    if e % 3 == 2 {
        e - 2
    } else {
        e + 1
    }
}

/// Previous half-edge of the same triangle (e % 3 == 0 → e + 2, otherwise e - 1).
/// (Not used in `voronoi.ts` but kept for symmetry with the Delaunator API.)
#[inline]
pub fn prev_halfedge(e: usize) -> usize {
    if e.is_multiple_of(3) {
        e + 2
    } else {
        e - 1
    }
}

/// The 3 half-edges of triangle `t`: `[3t, 3t+1, 3t+2]`.
#[inline]
pub fn edges_of_triangle(t: usize) -> [usize; 3] {
    [3 * t, 3 * t + 1, 3 * t + 2]
}

/// The 3 points of triangle `t`, indices into `triangles` (counter-clockwise).
#[inline]
pub fn points_of_triangle(triangles: &[u32], t: usize) -> [u32; 3] {
    let e = edges_of_triangle(t);
    [triangles[e[0]], triangles[e[1]], triangles[e[2]]]
}

/// Triangles adjacent to triangle `t` via each half-edge. `EMPTY` if the half-edge
/// is on the border (no neighbor) — replicates `trianglesAdjacentToTriangle` of `voronoi.ts:66-73`.
#[inline]
pub fn triangles_adjacent_to_triangle(halfedges: &[usize], t: usize) -> [usize; 3] {
    let e = edges_of_triangle(t);
    let mut out = [EMPTY; 3];
    for i in 0..3 {
        let opposite = halfedges[e[i]];
        if opposite != EMPTY {
            out[i] = triangle_of_edge(opposite);
        }
    }
    out
}

// === Export of public interfaces ===

#[cfg(test)]
mod internal_tests {
    use super::*;

    /// Simple determinism test — same input produces the same output.
    #[test]
    fn triangulate_is_deterministic() {
        let points: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let r1 = from_pairs(&points);
        let r2 = from_pairs(&points);
        assert_eq!(r1.triangles, r2.triangles);
        assert_eq!(r1.halfedges, r2.halfedges);
        assert_eq!(r1.hull, r2.hull);
        // 4 planar points → 2 triangles × 3 = 6 entries.
        assert_eq!(r1.triangles.len(), 6);
    }
}
