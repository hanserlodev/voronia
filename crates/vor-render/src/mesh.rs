use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::feature::Feature;
use vor_core::voronoi::VoronoiVertices;

use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};

/// Smooths the Voronoi mesh by moving each vertex toward the average of its
/// neighbors (Laplacian smoothing). Cells stay watertight because the vertices
/// are shared.
///
/// - `factor`: how much to move toward the neighbor center (0.0 = none, 0.5 = half)
/// - `iterations`: how many smoothing passes to apply
pub fn laplacian_smooth_vertices(
    vertices: &VoronoiVertices,
    factor: f32,
    iterations: usize,
) -> VoronoiVertices {
    let n = vertices.positions.len();
    if n == 0 {
        return vertices.clone();
    }
    let mut smoothed = vertices.positions.clone();
    for _ in 0..iterations {
        let prev = smoothed.clone();
        for v in 0..n {
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            let mut count = 0u32;
            for &nb in &vertices.adjacent_vertices[v] {
                if nb < 0 {
                    continue;
                }
                let p = prev[nb as usize];
                sum_x += p[0];
                sum_y += p[1];
                count += 1;
            }
            if count < 2 {
                continue;
            }
            let avg_x = sum_x / count as f32;
            let avg_y = sum_y / count as f32;
            let p = &prev[v];
            smoothed[v] = [
                p[0] + (avg_x - p[0]) * factor,
                p[1] + (avg_y - p[1]) * factor,
            ];
        }
    }
    let mut result = vertices.clone();
    result.positions = smoothed;
    result
}

/// Builds a `HeightmapMesh` from Voronoi data (positions + cell_rings),
/// coloring each cell according to `color_fn(cell_id)`.
/// The Voronoi mesh is smoothed with Laplacian smoothing to round the cells
/// without breaking watertightness between adjacent ones.
pub fn build_pack_mesh(
    vertices: &VoronoiVertices,
    points_n: usize,
    color_fn: impl Fn(usize) -> [f32; 4],
) -> HeightmapMesh {
    let smooth_vertices = laplacian_smooth_vertices(vertices, 0.2, 2);

    let mut result = HeightmapMesh {
        vertices: Vec::with_capacity(points_n.saturating_mul(6)),
        indices: Vec::with_capacity(points_n.saturating_mul(9)),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let mut tess = FillTessellator::new();

    for p in 0..points_n {
        let ann = match vertices.cell_rings.get(p) {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        let color = color_fn(p);

        let first_t = ann[0] as usize;
        let first_pos = smooth_vertices
            .positions
            .get(first_t)
            .copied()
            .unwrap_or([0.0, 0.0]);
        let mut builder = Path::builder();
        builder.begin(point(first_pos[0], first_pos[1]));
        for &t in ann.iter().skip(1) {
            let ti = t as usize;
            let pos = smooth_vertices
                .positions
                .get(ti)
                .copied()
                .unwrap_or([0.0, 0.0]);
            builder.line_to(point(pos[0], pos[1]));
        }
        builder.end(true);
        let path = builder.build();

        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, ColorCtor(color));
        let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);

        if tess
            .tessellate_path(&path, &opts, &mut buffer_builder)
            .is_err()
        {
            continue;
        }

        let base = result.vertices.len() as u32;
        result.vertices.extend_from_slice(&mesh.vertices);
        result.indices.extend(mesh.indices.iter().map(|i| i + base));

        for v in &mesh.vertices {
            result.bounds_min[0] = result.bounds_min[0].min(v.pos[0]);
            result.bounds_min[1] = result.bounds_min[1].min(v.pos[1]);
            result.bounds_max[0] = result.bounds_max[0].max(v.pos[0]);
            result.bounds_max[1] = result.bounds_max[1].max(v.pos[1]);
        }
    }

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0, 0.0];
        result.bounds_max = [0.0, 0.0];
    }

    result
}

/// Builds a white mesh covering exactly the land cells (where `is_land(p)` is
/// `true`), using the RAW (unsmoothed) Voronoi vertices.
///
/// The fractal landmass (`build_fractal_landmass_mesh`) can shrink below the
/// land cells on small islands, leaving those cells outside the stencil mask
/// and therefore unpainted (holes that "fight the sea"). Merging this mesh into
/// the mask layer guarantees every land cell is covered: a paint-bucket fill of
/// the landmass shape. The raw vertices are used so the mask never shrinks
/// below the actual cell polygons (Laplacian smoothing would contract them).
pub fn build_land_cells_mask_mesh(
    vertices: &VoronoiVertices,
    points_n: usize,
    is_land: impl Fn(usize) -> bool,
) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::with_capacity(points_n.saturating_mul(3)),
        indices: Vec::with_capacity(points_n.saturating_mul(6)),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let mut tess = FillTessellator::new();
    const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

    for p in 0..points_n {
        if !is_land(p) {
            continue;
        }
        let ann = match vertices.cell_rings.get(p) {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        let first_t = ann[0] as usize;
        let first_pos = vertices
            .positions
            .get(first_t)
            .copied()
            .unwrap_or([0.0, 0.0]);
        let mut builder = Path::builder();
        builder.begin(point(first_pos[0], first_pos[1]));
        for &t in ann.iter().skip(1) {
            let ti = t as usize;
            let pos = vertices.positions.get(ti).copied().unwrap_or([0.0, 0.0]);
            builder.line_to(point(pos[0], pos[1]));
        }
        builder.end(true);
        let path = builder.build();

        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, ColorCtor(WHITE));
        let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);

        if tess
            .tessellate_path(&path, &opts, &mut buffer_builder)
            .is_err()
        {
            continue;
        }

        let base = result.vertices.len() as u32;
        result.vertices.extend_from_slice(&mesh.vertices);
        result.indices.extend(mesh.indices.iter().map(|i| i + base));

        for v in &mesh.vertices {
            result.bounds_min[0] = result.bounds_min[0].min(v.pos[0]);
            result.bounds_min[1] = result.bounds_min[1].min(v.pos[1]);
            result.bounds_max[0] = result.bounds_max[0].max(v.pos[0]);
            result.bounds_max[1] = result.bounds_max[1].max(v.pos[1]);
        }
    }

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0, 0.0];
        result.bounds_max = [0.0, 0.0];
    }

    result
}

/// Subdivides a closed polygon with uniform cubic Catmull-Rom (α=0).
/// Each edge produces `subdivisions` points along the curve.
/// Uses 3 subdivisions by default for an Azgaar-like smoothing.
pub(crate) fn catmull_rom_closed(points: &[[f32; 2]], subdivisions: usize) -> Vec<[f32; 2]> {
    let n = points.len();
    if n < 4 || subdivisions == 0 {
        return points.to_vec();
    }
    let mut result = Vec::with_capacity(n * subdivisions);
    for i in 0..n {
        let p0 = points[(i + n - 1) % n];
        let p1 = points[i];
        let p2 = points[(i + 1) % n];
        let p3 = points[(i + 2) % n];
        for j in 0..subdivisions {
            let t = j as f32 / subdivisions as f32;
            let tt = t * t;
            let ttt = tt * t;
            let x = 0.5
                * (2.0 * p1[0]
                    + (-p0[0] + p2[0]) * t
                    + (2.0 * p0[0] - 5.0 * p1[0] + 4.0 * p2[0] - p3[0]) * tt
                    + (-p0[0] + 3.0 * p1[0] - 3.0 * p2[0] + p3[0]) * ttt);
            let y = 0.5
                * (2.0 * p1[1]
                    + (-p0[1] + p2[1]) * t
                    + (2.0 * p0[1] - 5.0 * p1[1] + 4.0 * p2[1] - p3[1]) * tt
                    + (-p0[1] + 3.0 * p1[1] - 3.0 * p2[1] + p3[1]) * ttt);
            result.push([x, y]);
        }
    }
    result
}

/// d3 `curveCatmullRom.alpha(a)` for open lines, ported from the d3-shape
/// control-point math (`h_` in d3.min.js): per segment `p_i → p_{i+1}` the
/// bézier controls are
///   `cp1 = (p_i·u − p_{i−1}·l12² + p_{i+1}·l01²)/c`, `u = 2l01²+3l01·l12+l12²`,
///   `c = 3l01(l01+l12)` (symmetric for cp2 with l23),
/// where `l_ab = |p_a − p_b|^alpha`. Mirrors d3's boundary handling: the path
/// **starts at the second point** (`p0` only influences the first control
/// point) and the final control point is the raw last point.
pub(crate) fn catmull_rom_open_alpha(
    points: &[[f32; 2]],
    alpha: f32,
    subdivisions: usize,
) -> Vec<[f32; 2]> {
    let n = points.len();
    if n < 3 || subdivisions == 0 {
        return points.to_vec();
    }
    let a = alpha.max(0.0);
    let eps = 1e-6;
    let la = |p: [f32; 2], q: [f32; 2]| -> f32 {
        let dx = p[0] - q[0];
        let dy = p[1] - q[1];
        (dx * dx + dy * dy).sqrt().powf(a)
    };

    let mut result = Vec::with_capacity((n - 2) * (subdivisions + 1) + 1);
    result.push(points[1]);
    for i in 1..=(n - 2) {
        let p_im1 = points[i - 1];
        let p_i = points[i];
        let p_i1 = points[i + 1];
        let l01 = la(p_im1, p_i);
        let l12 = la(p_i, p_i1);
        // cp1
        let cp1 = if l01 > eps {
            let u = 2.0 * l01 * l01 + 3.0 * l01 * l12 + l12 * l12;
            let c = 3.0 * l01 * (l01 + l12);
            [
                (p_i[0] * u - p_im1[0] * l12 * l12 + p_i1[0] * l01 * l01) / c,
                (p_i[1] * u - p_im1[1] * l12 * l12 + p_i1[1] * l01 * l01) / c,
            ]
        } else {
            p_i
        };
        // cp2 (l23 = 0 on the final segment → raw endpoint, like d3's lineEnd)
        let cp2 = if i + 2 < n {
            let p_i2 = points[i + 2];
            let l23 = la(p_i1, p_i2);
            if l23 > eps {
                let f = 2.0 * l23 * l23 + 3.0 * l23 * l12 + l12 * l12;
                let s = 3.0 * l23 * (l23 + l12);
                [
                    (p_i1[0] * f + p_i[0] * l23 * l23 - p_i2[0] * l12 * l12) / s,
                    (p_i1[1] * f + p_i[1] * l23 * l23 - p_i2[1] * l12 * l12) / s,
                ]
            } else {
                p_i1
            }
        } else {
            p_i1
        };
        for j in 1..=subdivisions {
            let t = j as f32 / subdivisions as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let mt = 1.0 - t;
            let w0 = mt * mt * mt;
            let w1 = 3.0 * mt * mt * t;
            let w2 = 3.0 * mt * t2;
            let w3 = t3;
            result.push([
                w0 * p_i[0] + w1 * cp1[0] + w2 * cp2[0] + w3 * p_i1[0],
                w0 * p_i[1] + w1 * cp1[1] + w2 * cp2[1] + w3 * p_i1[1],
            ]);
        }
    }
    result
}

/// Builds the base map mesh from **features** (continents/islands), NOT from
/// the Voronoi cell grid. Each feature's perimeter is smoothed with Catmull-Rom
/// for natural coastlines, and the whole landmass is colored with
/// `color_fn(feature)`.
///
/// The ocean is not rendered here -- the background color (clear color) of the
/// render pass is used instead.
pub fn build_landmass_mesh(
    vertices: &VoronoiVertices,
    features: &[Feature],
    color_fn: impl Fn(&Feature) -> [f32; 4],
) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let mut tess = FillTessellator::new();

    for feat in features {
        if !feat.is_land || feat.perimeter_vertices.len() < 3 {
            continue;
        }
        let raw: Vec<[f32; 2]> = feat
            .perimeter_vertices
            .iter()
            .filter_map(|&vi| vertices.positions.get(vi as usize).copied())
            .collect();
        if raw.len() < 3 {
            continue;
        }
        let smooth = catmull_rom_closed(&raw, 3);
        let color = color_fn(feat);

        let mut builder = Path::builder();
        if let Some(first) = smooth.first() {
            builder.begin(point(first[0], first[1]));
            for pt in smooth.iter().skip(1) {
                builder.line_to(point(pt[0], pt[1]));
            }
            builder.end(true);
        }
        let path = builder.build();

        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, ColorCtor(color));
        let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);
        if tess
            .tessellate_path(&path, &opts, &mut buffer_builder)
            .is_err()
        {
            continue;
        }

        let base = result.vertices.len() as u32;
        result.vertices.extend_from_slice(&mesh.vertices);
        result.indices.extend(mesh.indices.iter().map(|i| i + base));
        for v in &mesh.vertices {
            result.bounds_min[0] = result.bounds_min[0].min(v.pos[0]);
            result.bounds_min[1] = result.bounds_min[1].min(v.pos[1]);
            result.bounds_max[0] = result.bounds_max[0].max(v.pos[0]);
            result.bounds_max[1] = result.bounds_max[1].max(v.pos[1]);
        }
    }

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0, 0.0];
        result.bounds_max = [0.0, 0.0];
    }

    result
}

#[cfg(test)]
mod alpha_tests {
    use super::*;

    #[test]
    fn alpha_curve_starts_at_second_point_and_differs_from_straight() {
        // Asymmetric control spacing: alpha parameterization must bend the
        // curve differently than a uniform Catmull-Rom would.
        let pts = vec![[0.0, 0.0], [10.0, 1.0], [30.0, 2.0], [70.0, 0.0]];
        let out = catmull_rom_open_alpha(&pts, 0.1, 4);
        // d3 curveCatmullRomOpen starts at the SECOND input point.
        assert_eq!(out[0], pts[1]);
        // Ends exactly at the last point.
        assert_eq!(*out.last().unwrap(), *pts.last().unwrap());
        // Interior samples deviate from the straight polyline.
        let mid = &out[out.len() / 2];
        let straight_y =
            pts[1][1] + (pts[3][1] - pts[1][1]) * (mid[0] - pts[1][0]) / (pts[3][0] - pts[1][0]);
        assert!(
            (mid[1] - straight_y).abs() > 1e-3,
            "curve should bend, got {mid:?} vs straight y {straight_y}"
        );
    }

    #[test]
    fn alpha_curve_short_input_passthrough() {
        let pts = vec![[0.0, 0.0], [5.0, 5.0]];
        assert_eq!(catmull_rom_open_alpha(&pts, 0.1, 4), pts);
    }
}
