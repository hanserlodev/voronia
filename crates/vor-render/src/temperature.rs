//! Temperature layer: filled isotherm bands over the **grid** cells, ported from
//! Azgaar's `src/renderers/draw-temperature.ts`.
//!
//! Azgaar renders ~5 bands between the real min/max temperature of the map,
//! each band a closed polygon (chain) walked over the Voronoi vertex graph with
//! `connectVertices`, relaxed (1 of every 4 vertices + border vertices) and
//! smoothed with d3 `line().curve(curveBasisClosed)` (a cubic B-spline — NOT a
//! midpoint quadratic). A base rectangle covers the whole map with the color of
//! `minTemp`, then each band paints over it with `scheme(...)` plus a stroke of
//! `color(fill).darker(0.2)`. The layer CSS is honored: `fill-opacity: 0.3`
//! (fills only — the stroke keeps full alpha) and `stroke-width: 1.8`.
//!
//! This is a **faithful** port: the per-cell walk order, the global `checked`
//! array, the exact `findStart`/`ofSameType`, the `curveBasisClosed` smoothing
//! and the `darker(0.2)` stroke of FMG are reproduced so the rendered bands match.

use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, LineCap, LineJoin, StrokeOptions,
    StrokeTessellator, StrokeVertexConstructor, VertexBuffers,
};
use vor_core::Grid;

use crate::heightmap::{spectral_linear, ColorCtor, HeightmapMesh, HeightmapVertex};

const T_MIN: f32 = -50.0;
const DELTA: f32 = 100.0;
/// CSS `#temperature { fill-opacity: 0.3 }` (applies to fills only, not strokes).
const CSS_FILL_OPACITY: f32 = 0.3;
/// CSS `#temperature { stroke-width: 1.8 }`.
const CSS_STROKE_WIDTH: f32 = 1.8;
/// d3 `darker = 0.7`; `color(fill).darker(0.2)` → sRGB channels × `0.7^0.2`.
fn darker_pow() -> f32 {
    0.7_f32.powf(0.2)
}

/// Azgaar `scheme(1 - (t - tMin) / delta)` — the Spectral scale.
fn temp_color(t: f32) -> [f32; 4] {
    spectral_linear(1.0 - (t - T_MIN) / DELTA)
}

/// Pushes a filled rectangle over the whole canvas (the `minTemp` base).
fn fill_rect(out: &mut HeightmapMesh, tess: &mut FillTessellator, color: [f32; 4], w: f32, h: f32) {
    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.line_to(point(w, 0.0));
    builder.line_to(point(w, h));
    builder.line_to(point(0.0, h));
    builder.end(true);
    let path = builder.build();

    let mut verts: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
    let mut buffer_builder = BuffersBuilder::new(&mut verts, ColorCtor(color));
    let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::NonZero);
    if tess
        .tessellate_path(&path, &opts, &mut buffer_builder)
        .is_err()
    {
        return;
    }
    push_mesh(out, &verts);
}

/// d3 `range(start, stop, step)` (exclusive `stop`): `k = max(0, ceil((stop-start)/step))`,
/// values `start + i*step` for `i < k`, empty if `start >= stop` or `step <= 0`.
fn d3_range(start: f32, stop: f32, step: f32) -> Vec<f32> {
    if start >= stop || step <= 0.0 {
        return Vec::new();
    }
    let k = ((stop - start) / step).ceil().max(0.0) as usize;
    (0..k).map(|i| start + i as f32 * step).collect()
}

/// Azgaar's `ofSameType` for temperature — `cells.temp[c] >= t`. Boundary ids
/// (`c >= n`) have no temperature (`undefined` in JS) and are treated as false.
fn is_same_type(grid: &Grid, n: usize, c: usize, t: f32) -> bool {
    if c >= n {
        return false;
    }
    (grid.cells.temperature[c] as f32) >= t
}

/// A Voronoi vertex touches the map border if one of its cells is a boundary
/// point (`c >= n` in Azgaar's `vertices.c[v]`).
fn is_border_vertex(grid: &Grid, n: usize, v: u32) -> bool {
    match grid.vertices.adjacent_cells.get(v as usize) {
        Some(c) => c.iter().any(|&cc| cc >= n as i32),
        None => false,
    }
}

/// Azgaar `findStart(i, t)` (`draw-temperature.ts`), reproduced exactly:
/// - border cell (`cells.b[i]`): first ring vertex whose neighbor cells include
///   a boundary point (`vertices.c[v].some(c => c >= n)`);
/// - interior cell: `cells.v[i][cells.c[i].findIndex(c => cells.temp[c] < t || !cells.temp[c])]`
///   — `cells.c` is aligned with `cells.v` (same `edgesAroundPoint` order). The
///   `!cells.temp[c]` case (temperature exactly `0`) is preserved.
fn find_start(grid: &Grid, n: usize, i: usize, t: f32) -> Option<u32> {
    let ring = grid.vertices.cell_rings.get(i)?;
    let is_border_cell = matches!(grid.vertices.cell_border.get(i), Some(&b) if b != 0);
    if is_border_cell {
        return ring.iter().copied().find(|&v| is_border_vertex(grid, n, v));
    }
    let neighbors = grid.vertices.cell_neighbors.get(i)?;
    let idx = neighbors.iter().position(|&c| {
        let cx = c as usize;
        if cx >= n {
            return true;
        }
        let temp = grid.cells.temperature[cx] as f32;
        temp < t || temp == 0.0
    });
    let idx = idx?;
    if idx >= ring.len() {
        return None;
    }
    Some(ring[idx])
}

struct StrokeColorCtor(pub [f32; 4]);

impl StrokeVertexConstructor<HeightmapVertex> for StrokeColorCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::StrokeVertex<'_, '_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

/// Appends the stroke of a closed basis path with `color(fill).darker(0.2)`.
/// FMG relies on the `#temperature` layer CSS: `fill-opacity: 0.3` and
/// `stroke-width: 1.8`. `fill-opacity` only affects the fill (not the stroke),
/// so the stroke keeps full alpha.
fn add_stroke(out: &mut HeightmapMesh, path: &Path, color: [f32; 4], width: f32) {
    let [r, g, b, _] = color;
    let narrow = darker_pow();
    // Vertex colors are linear-light; d3 darkens in sRGB space, so round-trip.
    #[inline]
    fn to_srgb(v: f32) -> f32 {
        if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        }
    }
    #[inline]
    fn to_linear(v: f32) -> f32 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    let sr = to_linear(to_srgb(r) * narrow);
    let sg = to_linear(to_srgb(g) * narrow);
    let sb = to_linear(to_srgb(b) * narrow);

    let mut verts = VertexBuffers::<HeightmapVertex, u32>::new();
    let mut buffer_builder = BuffersBuilder::new(&mut verts, StrokeColorCtor([sr, sg, sb, 1.0]));
    let opts = StrokeOptions::default()
        .with_line_width(width)
        .with_line_cap(LineCap::Round)
        .with_line_join(LineJoin::Round);
    let mut tess = StrokeTessellator::new();
    if tess
        .tessellate_path(path, &opts, &mut buffer_builder)
        .is_err()
    {
        return;
    }
    push_mesh(out, &verts);
}

/// Builds the temperature mesh (Azgaar `draw-temperature.ts`), one-to-one.
///
/// 1. `step = max(round(|max-min|/5), 1)`, `isolines = range(min+step, max, step)`.
/// 2. A **single global** `checked` (Uint8Array) — every cell whose exact temp is
///    one of the isolines seeds a walk; the `connectVertices` walk marks same-type
///    cells as checked so each connected band is drawn once.
/// 3. `relaxed = chain.filter((v, i) => i % 4 === 0 || borderVertex)`; `< 6` skipped.
/// 4. Chains are gathered per isoline value `t`, then drawn in ascending `t`
///    order (like FMG's `for (const t of isolines)`), each as `curveBasisClosed`
///    + fill + `darker(0.2)` stroke.
pub fn build_temperature_mesh(grid: &Grid) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [0.0, 0.0],
        bounds_max: [grid.width, grid.height],
    };

    let n = grid.points_n().min(grid.cells.temperature.len());
    if n == 0 {
        return mesh;
    }

    let min_temp = grid.cells.temperature[..n]
        .iter()
        .copied()
        .min()
        .unwrap_or(0) as f32;
    let max_temp = grid.cells.temperature[..n]
        .iter()
        .copied()
        .max()
        .unwrap_or(0) as f32;
    let step = (((max_temp - min_temp).abs() / 5.0).round() as i32).max(1) as f32;
    let isolines = d3_range(min_temp + step, max_temp, step);

    // Gather chains per isoline value → Vec of paths per `t`, exactly like FMG's
    // `chains: [t, points][]` array (draw order preserved).
    let mut chains_by_t: Vec<(i32, Vec<Path>)> = Vec::new();

    // Single global `checkedCells` as in FMG.
    let mut checked = vec![false; n];

    for cell in 0..n {
        let cell_temp = grid.cells.temperature[cell] as f32;
        if checked[cell] || !isolines.contains(&cell_temp) {
            continue;
        }
        let Some(start) = find_start(grid, n, cell, cell_temp) else {
            continue;
        };
        checked[cell] = true;

        let same_type = |c: usize| is_same_type(grid, n, c, cell_temp);
        let mut check_cell = |c: usize| {
            if is_same_type(grid, n, c, cell_temp) {
                checked[c] = true;
            }
        };
        let chain = crate::isoline::connect_vertices(
            &grid.vertices,
            start,
            &same_type,
            &mut check_cell,
            false,
        );
        if chain.len() < 6 {
            continue;
        }

        // Relax: `i % 4 === 0 || borderVertex(c => c >= n)`.
        let relaxed: Vec<u32> = chain
            .iter()
            .enumerate()
            .filter(|(i, &v)| i % 4 == 0 || is_border_vertex(grid, n, v))
            .map(|(_, &v)| v)
            .collect();
        if relaxed.len() < 6 {
            continue;
        }

        // `points = relaxed.map(v => vertices.p[v])`, rounded to 1 decimal like
        // FMG's `round(..., 1)`.
        let pts: Vec<[f32; 2]> = relaxed
            .iter()
            .filter_map(|&v| grid.vertices.positions.get(v as usize).copied())
            .collect();
        let path = crate::isoline::build_curve_basis_closed(&pts, Some(10.0));
        match chains_by_t.iter_mut().find(|(t, _)| *t == cell_temp as i32) {
            Some((_, paths)) => paths.push(path),
            None => chains_by_t.push((cell_temp as i32, vec![path])),
        }
    }

    let mut tess = FillTessellator::new();
    let fill_opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::NonZero);

    // Base layer: whole canvas filled with the minTemp color (`stroke: none`),
    // inheriting the CSS `fill-opacity: 0.3`.
    let mut base_color = temp_color(min_temp);
    base_color[3] = CSS_FILL_OPACITY;
    fill_rect(&mut mesh, &mut tess, base_color, grid.width, grid.height);

    // `for (const t of isolines)` — ascending, each band on top of the previous.
    for t in &isolines {
        let t_int = *t as i32;
        let Some(paths) = chains_by_t.iter().find(|(t, _)| *t == t_int) else {
            continue;
        };
        let mut fill = temp_color(*t);
        fill[3] = CSS_FILL_OPACITY;
        for path in &paths.1 {
            let mut verts: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
            let mut buffer_builder = BuffersBuilder::new(&mut verts, ColorCtor(fill));
            if tess
                .tessellate_path(path, &fill_opts, &mut buffer_builder)
                .is_err()
            {
                continue;
            }
            push_mesh(&mut mesh, &verts);
            add_stroke(&mut mesh, path, fill, CSS_STROKE_WIDTH);
        }
    }

    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0, 0.0];
        mesh.bounds_max = [0.0, 0.0];
    }

    mesh
}

#[inline]
fn push_mesh(out: &mut HeightmapMesh, verts: &VertexBuffers<HeightmapVertex, u32>) {
    let base = out.vertices.len() as u32;
    out.vertices.extend_from_slice(&verts.vertices);
    out.indices.extend(verts.indices.iter().map(|i| i + base));
    for v in &verts.vertices {
        out.bounds_min[0] = out.bounds_min[0].min(v.pos[0]);
        out.bounds_min[1] = out.bounds_min[1].min(v.pos[1]);
        out.bounds_max[0] = out.bounds_max[0].max(v.pos[0]);
        out.bounds_max[1] = out.bounds_max[1].max(v.pos[1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vor_core::cells::GridCells;
    use vor_core::voronoi::VoronoiVertices;

    fn make_test_vertices() -> VoronoiVertices {
        let positions = vec![
            [10.0, 10.0],
            [90.0, 10.0],
            [10.0, 90.0],
            [90.0, 90.0],
            [50.0, 50.0],
        ];
        let adjacent_cells = vec![[0, 1, 2], [1, 3, 2], [0, 2, 3], [1, 0, 3], [2, 1, 3]];
        let adjacent_vertices = vec![[3, -1, 1], [0, 2, 4], [4, 3, -1], [1, 4, 2], [-1, 0, 1]];
        let cell_rings = vec![
            vec![0, 2, 3],
            vec![0, 1, 4, 3],
            vec![0, 1, 2],
            vec![1, 4, 2],
        ];
        VoronoiVertices {
            positions,
            adjacent_cells,
            adjacent_vertices,
            cell_rings,
            cell_neighbors: vec![vec![1, 2, 3], vec![0, 3, 2], vec![0, 1, 3], vec![1, 0, 2]],
            cell_border: vec![0, 0, 0, 0],
        }
    }

    #[test]
    fn d3_range_matches_d3() {
        assert_eq!(d3_range(2.0, 10.0, 2.0), vec![2.0, 4.0, 6.0, 8.0]);
        assert_eq!(d3_range(-3.0, 9.0, 3.0), vec![-3.0, 0.0, 3.0, 6.0]);
        assert_eq!(d3_range(5.0, 9.0, 10.0), vec![5.0]);
        assert!(d3_range(10.0, 5.0, 1.0).is_empty());
        assert!(d3_range(5.0, 5.0, 1.0).is_empty());
    }

    #[test]
    fn temp_color_uses_spectral_scale() {
        let cold = temp_color(T_MIN); // t → scheme(1) → last Spectral stop #5e4fa2
        let hot = temp_color(T_MIN + DELTA); // → scheme(0) → first stop #9e0142
        assert_eq!(
            cold[3], 1.0,
            "alpha stays 1.0; fill-opacity is applied separately"
        );
        assert_eq!(hot[3], 1.0);
        assert!(hot[0] > cold[0], "hot end should be redder than cold end");
    }

    #[test]
    fn temperature_mesh_is_finite_and_covers_canvas() {
        let grid = Grid {
            width: 100.0,
            height: 100.0,
            cells_desired: 4,
            points: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            cells: GridCells {
                temperature: vec![10, 10, 4, 0],
                ..Default::default()
            },
            vertices: make_test_vertices(),
            ..Default::default()
        };

        let mesh = build_temperature_mesh(&grid);

        assert!(
            !mesh.vertices.is_empty(),
            "base rect alone guarantees geometry"
        );
        assert!(!mesh.indices.is_empty());
        assert!(
            mesh.vertices
                .iter()
                .all(|v| v.pos[0].is_finite() && v.pos[1].is_finite()),
            "all vertices must be finite"
        );
        assert!(mesh.bounds_max[0] <= grid.width && mesh.bounds_max[1] <= grid.height);
        assert!(mesh.bounds_min[0] >= 0.0 && mesh.bounds_min[1] >= 0.0);
    }
}
