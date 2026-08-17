//! Bit-exact port of `reGraph` (`azgaar-fmg/public/main.js:1157-1209`).
//!
//! Repacking of the jittered grid (`grid`) into the pack with reduced cells and extra
//! points on the coasts. Produces the final `pack.cells` and `pack.vertices` — it is the
//! mesh on which the culture/state/biome/etc. subsystems operate.
//!
//! ## Bit-exactness — why this port matters
//!
//! If Voronia produces a different pack mesh (even by 1 cell or by a different order in
//! `newCells.p`), the `pack.cells.g[packId] → gridId` mapping ends up different, and all
//! attributes that Azgaar serializes indexed by pack id (biome, state, burg, ...) get
//! applied to the wrong cells — silent bug without runtime error.
//! (`vor-core/src/pack.rs` §"Critically").
//!
//! `reGraph` takes ~9 parameters (points/grid/boundary/attributes). We keep the simple
//! signature instead of introducing a wrapper struct — the parameters map 1-to-1 to the
//! Azgaar algorithm.

#![allow(clippy::too_many_arguments)]

use crate::geometry::voronoi::{calculate_voronoi, Voronoi};
use vor_core::cells::PackCells;
use vor_core::pack::Pack;
use vor_core::{feature::FeatureType, voronoi::VoronoiVertices};

/// Reproduces `reGraph` of Azgaar (`main.js:1157-1209`).
///
/// **Input** and their roles:
/// - `grid_points`: `grid.points` (10000 in Brample).
/// - `grid_boundary`: `grid.boundary` (200 in Brample).
/// - `grid_voronoi`: grid topology (output of `calculate_voronoi(allPoints, pointsN)`
///   previously computed). Contains `cells.c` (neighbors of each cell), `cells.b` (border flag).
/// - `grid_height`: `grid.cells.h` (slot `[7]`).
/// - `grid_water_type`: `grid.cells.t` (slot `[10]`). -2 lake, -1 coastal water, 1 coastal land, otherwise.
/// - `grid_feature_id`: `grid.cells.f` (slot `[9]`).
/// - `grid_features`: `grid.features` (vec of Feature; indexed by `grid_feature_id[i]`).
///
/// **Output**: `(Pack, new_points_f64)` — the `Pack` with the full `vor-core` model, **plus**
/// the `new_points` in f64. The latter is useful when the caller wants full bit-exactness:
/// `Pack.points` is in `f32` (fixed cap of the model), but the internal computation of
/// `reGraph` operates in f64 (same as the Azgaar JS). To validate bit-exactness against the
/// JS, compare the `new_points_f64` — not `pack.points` (which loses ~1e-5 of
/// precision on coords > 100 due to the f32 cast).
///
/// The caller is responsible for populating the remaining `PackCells` fields (biome, culture,
/// state, ...) when importing from the `.map`.
pub fn re_graph(
    grid_points: &[[f64; 2]],
    grid_boundary: &[[f64; 2]],
    grid_voronoi: &Voronoi,
    grid_height: &[u8],
    grid_water_type: &[i8],
    grid_feature_id: &[u16],
    grid_features_kind: &[FeatureType],
    spacing: f64,
) -> (Pack, Vec<[f64; 2]>) {
    let _ = grid_voronoi; // grid topology consumed below via grid_voronoi.cells.c/b
    let points_n = grid_points.len();

    // `newCells = { p: [], g: [], h: [] }` — pack points before the second `calculateVoronoi`.
    let mut new_points: Vec<[f64; 2]> = Vec::new();
    let mut new_g: Vec<u32> = Vec::new();
    let mut new_h: Vec<u8> = Vec::new();
    let spacing2 = spacing * spacing;

    // `for (const i of gridCells.i)` — `i` iterates the ids 0..nPoints in ascending order.
    for i in 0..points_n {
        let i = i as u32;
        let height = grid_height[i as usize];
        let typ = grid_water_type[i as usize];

        // Filter 1: deep ocean non-coastal. `height < 20 && type !== -1 && type !== -2`.
        if height < 20 && typ != -1 && typ != -2 {
            continue;
        }
        // Filter 2: non-coastal lake. `type === -2 && (i % 4 === 0 || features[gridCells.f[i]].type === "lake")`.
        if typ == -2
            && (i.is_multiple_of(4)
                || grid_features_kind[grid_feature_id[i as usize] as usize] == FeatureType::Lake)
        {
            continue;
        }

        let [x, y] = grid_points[i as usize];
        add_new_point(i, x, y, height, &mut new_points, &mut new_g, &mut new_h);

        // Extra points for coastal cells. `if (type === 1 || type === -1)`.
        if typ == 1 || typ == -1 {
            // `if (gridCells.b[i]) continue;` — skip near-border cells.
            if grid_voronoi.cells.b[i as usize] != 0 {
                continue;
            }
            // Iterate `gridCells.c[i]` — neighbors of cell i (interiors, boundary filtered).
            let neighbors = &grid_voronoi.cells.c[i as usize];
            for &e in neighbors {
                // `if (i > e) return;` — only processes when i < e (each pair once).
                if i > e {
                    continue;
                }
                // `if (gridCells.t[e] === type)` — same cell type (same coast).
                let e_type = grid_water_type[e as usize];
                if e_type != typ {
                    continue;
                }
                let [ex, ey] = grid_points[e as usize];
                // `const dist2 = (y - points[e][1]) ** 2 + (x - points[e][0]) ** 2;`
                let dist2 = (y - ey).powi(2) + (x - ex).powi(2);
                if dist2 < spacing2 {
                    continue;
                }
                // Midpoint, rn to 1 decimal.
                // `const x1 = rn((x + points[e][0]) / 2, 1);`
                let x1 = crate::numbers::rn((x + ex) / 2.0, 1);
                let y1 = crate::numbers::rn((y + ey) / 2.0, 1);
                add_new_point(i, x1, y1, height, &mut new_points, &mut new_g, &mut new_h);
            }
        }
    }

    // `calculateVoronoi(newCells.p, grid.boundary)` — second Voronoi.
    let all_points_n = new_points.len();
    let mut all_points = new_points.clone();
    all_points.extend(grid_boundary.iter().cloned());
    let delaunay = crate::geometry::delaunay::from_pairs(&all_points);
    let voronoi = calculate_voronoi(&delaunay, &all_points, all_points_n as u32);

    // `pack.cells.area`: for each cellId, polygon area = abs(polygonArea(cells.v[cellId].map(v => vertices.p[v]))), capped to UINT16_MAX.
    // Azgaar: `Math.abs(d3.polygonArea(getPackPolygon(cellId)))` and then `Math.min(area, TYPED_ARRAY_MAX.UINT16)`.
    // UINT16_MAX = 65535. Direct clamp in f64 + cast to u16 (cannot saturate): clampRaw is min(raw, 65535.0).
    let n_pack_cells = voronoi.cells.b.len();
    let mut area_px = Vec::with_capacity(n_pack_cells);
    for cell_id in 0..n_pack_cells {
        let verts: Vec<[f64; 2]> = voronoi.cells.v[cell_id]
            .iter()
            .map(|&t| voronoi.vertices.p[t as usize])
            .collect();
        let raw = polygon_area_signed(&verts).abs();
        // `Math.min(area, TYPED_ARRAY_MAX.UINT16)` = 65535. Cast to u16 without saturating (raw clamp
        // guarantees raw <= 65535.0 → cast as u16 is safe).
        let capped = raw.min(u16::MAX as f64) as u16;
        area_px.push(capped);
    }

    // Populate `PackCells`. Only `grid_id`, `height`, `area_px`, `adjacency` — the rest
    // stays empty (the parser completes them) and `Default` for `Vec<T>` is empty.
    let pack_cells = PackCells {
        grid_id: new_g,
        height: new_h,
        area_px,
        adjacency: voronoi.cells.c.clone(),
        ..Default::default()
    };

    // Convert `voronoi.vertices` to the `vor-core::VoronoiVertices` format (i32 with -1 = EMPTY).
    let vertices = voronoi_to_vor_core(&voronoi);

    let pack = Pack {
        points: new_points
            .iter()
            .map(|&[x, y]| [x as f32, y as f32])
            .collect(),
        boundary: grid_boundary
            .iter()
            .map(|&[x, y]| [x as f32, y as f32])
            .collect(),
        cells: pack_cells,
        vertices,
        features: Vec::new(), // features are completed when populating from the .map
    };
    (pack, new_points)
}

/// `addNewPoint(i, x, y, height)`: push triple (p, g, h). Closure in JS; here we
/// unroll it by passing the mutable Vecs separately (Rust has no convenient closures
/// with mutable capture in this loop shape).
#[inline]
fn add_new_point(
    i: u32,
    x: f64,
    y: f64,
    height: u8,
    new_points: &mut Vec<[f64; 2]>,
    new_g: &mut Vec<u32>,
    new_h: &mut Vec<u8>,
) {
    new_points.push([x, y]);
    new_g.push(i);
    new_h.push(height);
}

/// `d3.polygonArea` (`d3-polygon/src/area.js`) — shoelace signed area.
/// `area = sum(a[1]*b[0] - a[0]*b[1]) / 2` where `b = polygon[i]`, `a = polygon[i-1]`.
/// `area/2` in f64. The JS does not apply `Math.abs` (the caller does).
#[inline]
fn polygon_area_signed(polygon: &[[f64; 2]]) -> f64 {
    let n = polygon.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    // a = polygon[n-1] for i=0; then a = b (polygon[i-1]); b = polygon[i].
    let mut b = polygon[n - 1];
    for item in polygon.iter() {
        let a = b;
        b = *item;
        area += a[1] * b[0] - a[0] * b[1];
    }
    area / 2.0
}

/// Converts `vor-import::Voronoi::vertices` to `vor-core::VoronoiVertices`.
///
/// `vertices.p` (`[f64;2]` with `floor` already truncated) is cast to `f32` (fixed cap).
/// `vertices.v` (`[usize;3]` with `EMPTY` for border) is converted to `[i32;3]` with `-1` for
/// `EMPTY` (Azgaar interface that `vor-core` already uses).
fn voronoi_to_vor_core(voronoi: &Voronoi) -> VoronoiVertices {
    use crate::geometry::delaunay::EMPTY;

    let n_tri = voronoi.vertices.p.len();
    let mut positions = Vec::with_capacity(n_tri);
    let mut adjacent_cells = Vec::with_capacity(n_tri);
    let mut adjacent_vertices = Vec::with_capacity(n_tri);

    for t in 0..n_tri {
        positions.push([
            voronoi.vertices.p[t][0] as f32,
            voronoi.vertices.p[t][1] as f32,
        ]);
        // `vertices.c[t]` is already `[u32;3]` (cell ids).
        adjacent_cells.push([
            voronoi.vertices.c[t][0] as i32,
            voronoi.vertices.c[t][1] as i32,
            voronoi.vertices.c[t][2] as i32,
        ]);
        // `vertices.v[t]` is `[usize;3]` with EMPTY for border → convert to -1.
        adjacent_vertices.push([
            if voronoi.vertices.v[t][0] == EMPTY {
                -1
            } else {
                voronoi.vertices.v[t][0] as i32
            },
            if voronoi.vertices.v[t][1] == EMPTY {
                -1
            } else {
                voronoi.vertices.v[t][1] as i32
            },
            if voronoi.vertices.v[t][2] == EMPTY {
                -1
            } else {
                voronoi.vertices.v[t][2] as i32
            },
        ]);
    }

    VoronoiVertices {
        positions,
        adjacent_cells,
        adjacent_vertices,
        cell_rings: voronoi.cells.v.clone(),
        cell_neighbors: voronoi.cells.c.clone(),
        cell_border: voronoi.cells.b.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::delaunay::from_pairs;
    use crate::geometry::place_points;
    use vor_core::feature::FeatureType;

    /// Basic sanity: if all cells are interior land with height=20, type=1, we are not
    /// near a border, and all are at distance > spacing, the pack duplicates
    /// (approx) the grid by coastal extras. Since there are no coastal cells in the
    /// "neighbors with type === 1" sense, it depends on the number of neighbors with
    /// `e === 1` and `i < e`.
    ///
    /// Simpler test: all points in deep ocean (h<20, type=other) → total discard → empty pack.
    #[test]
    fn regraph_all_deep_ocean_yields_empty_pack() {
        let placed = place_points(200.0, 200.0, 100, "1");
        let grid_points: Vec<[f64; 2]> = placed.points.iter().map(|&[x, y]| [x, y]).collect();
        let grid_boundary: Vec<[f64; 2]> = placed.boundary.iter().map(|&[x, y]| [x, y]).collect();
        let n = grid_points.len();

        // Grid topology (Voronoi) — needed for the input.
        let mut all_points = grid_points.clone();
        all_points.extend(grid_boundary.iter().cloned());
        let delaunay = from_pairs(&all_points);
        let grid_voronoi = calculate_voronoi(&delaunay, &all_points, n as u32);

        // h=10 (<20), type=0 (not -1, not -2) → discarded by filter 1.
        let grid_height = vec![10u8; n];
        let grid_water_type = vec![0i8; n];
        let grid_feature_id = vec![0u16; n];
        let grid_features_kind = vec![FeatureType::Ocean];

        let (pack, new_pts) = re_graph(
            &grid_points,
            &grid_boundary,
            &grid_voronoi,
            &grid_height,
            &grid_water_type,
            &grid_feature_id,
            &grid_features_kind,
            placed.spacing,
        );

        assert_eq!(pack.points.len(), 0, "all discarded, no points");
        assert!(new_pts.is_empty());
        assert!(pack.cells.grid_id.is_empty());
        assert!(pack.cells.height.is_empty());
        assert!(pack.cells.area_px.is_empty());
    }

    /// All cells interior land (h=50, type=other positive): none discarded, but no
    /// extras added because type is not coastal.
    #[test]
    fn regraph_all_interior_land_yields_one_point_per_cell() {
        let placed = place_points(200.0, 200.0, 100, "1");
        let grid_points: Vec<[f64; 2]> = placed.points.iter().map(|&[x, y]| [x, y]).collect();
        let grid_boundary: Vec<[f64; 2]> = placed.boundary.iter().map(|&[x, y]| [x, y]).collect();
        let n = grid_points.len();

        let mut all_points = grid_points.clone();
        all_points.extend(grid_boundary.iter().cloned());
        let delaunay = from_pairs(&all_points);
        let grid_voronoi = calculate_voronoi(&delaunay, &all_points, n as u32);

        // h=50 (>=20), type=2 (not -1,-2, not coast) — not discarded, no extra points.
        let grid_height = vec![50u8; n];
        let grid_water_type = vec![2i8; n];
        let grid_feature_id = vec![0u16; n];
        let grid_features_kind = vec![FeatureType::Ocean];

        let (pack, new_pts) = re_graph(
            &grid_points,
            &grid_boundary,
            &grid_voronoi,
            &grid_height,
            &grid_water_type,
            &grid_feature_id,
            &grid_features_kind,
            placed.spacing,
        );

        assert_eq!(pack.points.len(), n, "1 point per cell (no extras)");
        assert_eq!(new_pts.len(), n);
        assert_eq!(pack.cells.grid_id.len(), n);
        assert_eq!(pack.cells.height.len(), n);
        assert_eq!(
            pack.cells.area_px.len(),
            pack.points.len(),
            "area_px per pack cell"
        );
        // grid_id must be [0..n] (in order, no discards or extras).
        assert_eq!(pack.cells.grid_id, (0..n as u32).collect::<Vec<u32>>());
        assert_eq!(pack.cells.height, vec![50u8; n]);
    }

    /// Determinism: same input → same output.
    #[test]
    fn regraph_is_deterministic() {
        let placed = place_points(200.0, 200.0, 100, "1");
        let grid_points: Vec<[f64; 2]> = placed.points.iter().map(|&[x, y]| [x, y]).collect();
        let grid_boundary: Vec<[f64; 2]> = placed.boundary.iter().map(|&[x, y]| [x, y]).collect();
        let n = grid_points.len();

        let mut all_points = grid_points.clone();
        all_points.extend(grid_boundary.iter().cloned());
        let delaunay = from_pairs(&all_points);
        let grid_voronoi = calculate_voronoi(&delaunay, &all_points, n as u32);

        let grid_height = vec![50u8; n];
        let grid_water_type = vec![2i8; n];
        let grid_feature_id = vec![0u16; n];
        let grid_features_kind = vec![FeatureType::Ocean];

        let (a, a_pts) = re_graph(
            &grid_points,
            &grid_boundary,
            &grid_voronoi,
            &grid_height,
            &grid_water_type,
            &grid_feature_id,
            &grid_features_kind,
            placed.spacing,
        );
        let (b, b_pts) = re_graph(
            &grid_points,
            &grid_boundary,
            &grid_voronoi,
            &grid_height,
            &grid_water_type,
            &grid_feature_id,
            &grid_features_kind,
            placed.spacing,
        );

        assert_eq!(a.points, b.points, "deterministic points");
        assert_eq!(a_pts, b_pts, "deterministic new_pts");
        assert_eq!(a.cells.grid_id, b.cells.grid_id);
        assert_eq!(a.cells.height, b.cells.height);
        assert_eq!(a.cells.area_px, b.cells.area_px);
    }

    /// `polygonArea_signed` — trivial cases.
    #[test]
    fn polygon_area_unit_square() {
        // Unit square in CCW order: [0,0], [1,0], [1,1], [0,1].
        // shoelace = (0*1 - 0*0) + (0*1 - 1*1) + (1*0 - 1*1) + (1*0 - 0*0)
        //         = 0 - 1 - 1 + 0 = -2, /2 = -1.0. Negative → CW; abs → 1.0.
        let poly = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let area = polygon_area_signed(&poly).abs();
        assert!(
            (area - 1.0).abs() < 1e-9,
            "unit square area = 1.0, got {area}"
        );
    }
}
