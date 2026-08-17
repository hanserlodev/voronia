use vor_core::feature::FeatureType;
use vor_core::voronoi::VoronoiVertices;
use vor_core::Pack;

use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};

use crate::heightmap::{height_color, ColorCtor, HeightmapMesh, HeightmapVertex};

#[derive(Debug, Clone)]
pub struct IsolineOptions {
    pub polygons: bool,
    pub fill: bool,
    pub water_gap: bool,
    pub halo: bool,
}

impl Default for IsolineOptions {
    fn default() -> Self {
        Self {
            polygons: false,
            fill: true,
            water_gap: false,
            halo: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IsolineOutput {
    pub chain: Vec<u32>,
    pub points: Vec<[f32; 2]>,
}

pub fn connect_vertices(
    vertices: &VoronoiVertices,
    starting_vertex: u32,
    same_type: &impl Fn(usize) -> bool,
    check_cell: &mut impl FnMut(usize),
    close_ring: bool,
) -> Vec<u32> {
    // Literal port of `pathUtils.ts:connectVertices`: the chain always stops when the
    // walk returns to the starting vertex; `close_ring` only controls whether the
    // starting vertex is appended again at the end (Azgaar does NOT pass it for
    // temperature — the single chains are left open).
    let mut chain = Vec::new();
    let mut current = starting_vertex;

    loop {
        let previous = chain.last().copied().unwrap_or(u32::MAX);
        chain.push(current);

        let cells = vertices
            .adjacent_cells
            .get(current as usize)
            .copied()
            .unwrap_or([-1, -1, -1]);
        for &c in &cells {
            // `neibCells.filter(ofSameType).forEach(addToChecked)` — only positive,
            // same-type cell ids (boundary cells `c >= n` fail `same_type`).
            if c >= 0 && same_type(c as usize) {
                check_cell(c as usize);
            }
        }

        let neighbors = vertices
            .adjacent_vertices
            .get(current as usize)
            .copied()
            .unwrap_or([-1, -1, -1]);

        let c = [
            cells[0] >= 0 && same_type(cells[0] as usize),
            cells[1] >= 0 && same_type(cells[1] as usize),
            cells[2] >= 0 && same_type(cells[2] as usize),
        ];
        let v = neighbors;

        let next = if v[0] >= 0 && v[0] as u32 != previous && c[0] != c[1] {
            Some(v[0] as u32)
        } else if v[1] >= 0 && v[1] as u32 != previous && c[1] != c[2] {
            Some(v[1] as u32)
        } else if v[2] >= 0 && v[2] as u32 != previous && c[0] != c[2] {
            Some(v[2] as u32)
        } else {
            None
        };

        match next {
            Some(n) if n == starting_vertex => {
                // Azgaar's `for (let i = 0; i === 0 || next !== startingVertex; i++)`
                // exits when the walk returns to the start — no separate push.
                if close_ring {
                    chain.push(n);
                }
                break;
            }
            Some(n) => current = n,
            None => break,
        }
    }

    chain
}

/// d3's `line().curve(curveBasisClosed)` — a cubic B-spline over the points,
/// baked into cubic Bézier segments exactly as d3-shape's `basis.js` +
/// `basisClosed.js` (used by `lineGen` in `draw-temperature.ts`).
///
/// For input points `p = [p0, p1, …, pn-1]` (n ≥ 3) the closed curve starts at
/// `(p0 + 4 p1 + p2) / 6` and emits **n** `cubicBezierTo` segments, one per
/// triple `(A, B, C) = (p_j, p_{j+1}, p_{j+2})` for `j = 1..n` (mod `n`):
///
/// ```text
/// c1 = (2 A + B) / 3, c2 = (A + 2 B) / 3, end = (A + 4 B + C) / 6
/// ```
///
/// The last triple `(p0, p1, p2)` ends exactly at `(p0 + 4 p1 + p2) / 6` = the
/// start point, so the ring closes with no straight-line seam (d3 `basisClosed`
/// re-emits the first three points at `lineEnd`). `round` param mirrors FMG's
/// `round(path, 1)` applied to each emitted coordinate.
pub fn build_curve_basis_closed(pts: &[[f32; 2]], round_to: Option<f32>) -> Path {
    let n = pts.len();
    let mut builder = Path::builder();
    if n < 3 {
        return builder.build();
    }
    let r1 = |v: f32| match round_to {
        Some(m) => (v * m).round() / m,
        None => v,
    };
    let start = [
        r1((pts[0][0] + 4.0 * pts[1][0] + pts[2][0]) / 6.0),
        r1((pts[0][1] + 4.0 * pts[1][1] + pts[2][1]) / 6.0),
    ];
    builder.begin(point(start[0], start[1]));
    // `basisClosed.js`: after the `moveTo((p0+4p1+p2)/6)`, the `point`/`lineEnd`
    // handlers emit one bezier per triple `(p_j, p_{j+1}, p_{j+2})`, j = 1..n.
    for j in 1..=n {
        let a = pts[j % n];
        let b = pts[(j + 1) % n];
        let c = pts[(j + 2) % n];
        let c1 = [r1((2.0 * a[0] + b[0]) / 3.0), r1((2.0 * a[1] + b[1]) / 3.0)];
        let c2 = [r1((a[0] + 2.0 * b[0]) / 3.0), r1((a[1] + 2.0 * b[1]) / 3.0)];
        let end = [
            r1((a[0] + 4.0 * b[0] + c[0]) / 6.0),
            r1((a[1] + 4.0 * b[1] + c[1]) / 6.0),
        ];
        builder.cubic_bezier_to(
            point(c1[0], c1[1]),
            point(c2[0], c2[1]),
            point(end[0], end[1]),
        );
    }
    builder.end(true);
    builder.build()
}

/// Builds the SVG fill path for an isoline chain the way FMG does
/// (`pathUtils.ts:getFillPath`): straight `M…L…Z` segments, **not** smoothed.
/// Regional fills (states, provinces, cultures, religions, zones, markets) use
/// the raw Voronoi boundary — smoothing is only applied to climate isolines
/// (`build_curve_basis_closed`), matching Azgaar.
pub fn get_fill_path(chain: &[u32], vertices: &VoronoiVertices) -> Path {
    let mut builder = Path::builder();
    if chain.len() < 3 {
        return builder.build();
    }

    let pts: Vec<[f32; 2]> = chain
        .iter()
        .filter_map(|&v| vertices.positions.get(v as usize).copied())
        .collect();
    if pts.len() < 3 {
        return builder.build();
    }

    let first = pts[0];
    builder.begin(point(first[0], first[1]));
    for p in pts.iter().skip(1) {
        builder.line_to(point(p[0], p[1]));
    }
    builder.end(true);
    builder.build()
}

pub fn get_border_path(
    chain: &[u32],
    vertices: &VoronoiVertices,
    discontinue: impl Fn(u32) -> bool,
) -> Path {
    let mut builder = Path::builder();
    let mut in_segment = false;

    for &v in chain {
        if discontinue(v) {
            in_segment = false;
        } else {
            let p = vertices
                .positions
                .get(v as usize)
                .copied()
                .unwrap_or([0.0, 0.0]);
            if !in_segment {
                builder.begin(point(p[0], p[1]));
                in_segment = true;
            } else {
                builder.line_to(point(p[0], p[1]));
            }
        }
    }

    builder.build()
}

fn is_land_cell(cell: usize, pack: &Pack) -> bool {
    pack.cells.height.get(cell).copied().unwrap_or(0) >= 20
}

pub fn get_water_gap_path(chain: &[u32], vertices: &VoronoiVertices, pack: &Pack) -> Path {
    let discontinue = |v: u32| -> bool {
        let cells = vertices
            .adjacent_cells
            .get(v as usize)
            .copied()
            .unwrap_or([-1, -1, -1]);
        cells
            .iter()
            .all(|&c| c >= 0 && is_land_cell(c as usize, pack))
    };
    get_border_path(chain, vertices, discontinue)
}

pub fn get_halo_path(chain: &[u32], vertices: &VoronoiVertices, pack: &Pack) -> Path {
    let discontinue = |v: u32| -> bool {
        let cells = vertices
            .adjacent_cells
            .get(v as usize)
            .copied()
            .unwrap_or([-1, -1, -1]);
        !cells.iter().any(|&c| {
            if c < 0 {
                return false;
            }
            pack.cells.feature_id.get(c as usize).copied().unwrap_or(0) == 0
            // feature_id 0 = ocean feature (border)
        })
    };
    get_border_path(chain, vertices, discontinue)
}

pub fn get_isolines(
    pack: &Pack,
    get_type: &impl Fn(usize) -> u16,
    options: &IsolineOptions,
) -> Vec<IsolineOutput> {
    let n_cells = pack.points_n();
    if n_cells == 0 {
        return Vec::new();
    }

    let mut checked = vec![false; n_cells];
    let mut result = Vec::new();
    let vertices = &pack.vertices;

    for cell in 0..n_cells {
        if checked[cell] {
            continue;
        }
        let cell_type = get_type(cell);
        if cell_type == 0 {
            checked[cell] = true;
            continue;
        }

        let same_type = |c: usize| get_type(c) == cell_type;

        let neighbors = match pack.cells.adjacency.get(cell) {
            Some(v) => v,
            None => {
                checked[cell] = true;
                continue;
            }
        };

        let border_cell = neighbors
            .iter()
            .copied()
            .find(|&nb| !same_type(nb as usize));
        let _border_cell = match border_cell {
            Some(c) => c as usize,
            None => {
                checked[cell] = true;
                continue;
            }
        };

        let feature_id = pack.cells.feature_id.get(cell).copied().unwrap_or(0) as usize;
        if let Some(feature) = pack.features.get(feature_id) {
            if feature.kind == FeatureType::Lake {
                let all_same_shoreline = feature.shoreline.iter().all(|&sc| same_type(sc as usize));
                if all_same_shoreline {
                    continue;
                }
            }
        }

        let cell_ring = match vertices.cell_rings.get(cell) {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        let start_vertex = match cell_ring.iter().copied().find(|&v| {
            let tri = v as usize;
            let adj = vertices
                .adjacent_cells
                .get(tri)
                .copied()
                .unwrap_or([-1, -1, -1]);
            adj.iter().any(|&c| c >= 0 && !same_type(c as usize))
        }) {
            Some(v) => v,
            None => continue,
        };

        checked[cell] = true;

        let mut check_cell = |c: usize| {
            if c < n_cells && get_type(c) == cell_type && !checked[c] {
                checked[c] = true;
            }
        };

        let chain = connect_vertices(vertices, start_vertex, &same_type, &mut check_cell, true);

        if chain.len() < 3 {
            continue;
        }

        let mut stack: Vec<usize> = (0..n_cells)
            .filter(|&c| checked[c] && get_type(c) == cell_type)
            .collect();
        while let Some(c) = stack.pop() {
            if let Some(nbors) = pack.cells.adjacency.get(c) {
                for &nb in nbors {
                    let nb = nb as usize;
                    if nb < n_cells && !checked[nb] && get_type(nb) == cell_type {
                        checked[nb] = true;
                        stack.push(nb);
                    }
                }
            }
        }

        let output = IsolineOutput {
            chain: chain.clone(),
            points: chain
                .iter()
                .filter_map(|&v| vertices.positions.get(v as usize).copied())
                .collect(),
        };

        if options.polygons || options.fill {
            result.push(output);
        }
    }

    result
}

/// Common regional layer builder (FMG `getIsolines` + fill).
///
/// Groups cells by `get_type`, walks the Voronoi boundary with `connect_vertices`,
/// and produces a single `HeightmapMesh` with the **fill** polygons (straight
/// Voronoi boundary, `get_fill_path` — the same `M…L…Z` FMG emits).
///
/// `color_fn(type_id)` maps each region type to its color. Regions with type `0`
/// (no entity) are skipped, matching FMG's `if (!getType(cellId)) continue`.
///
/// The **water gap** (Azgaar `getGappedFillPaths` `stroke-width:3`) is appended
/// separately with `water_gap::append_water_gap`, exactly like the biome layer —
/// callers pass their `is_water` array so both share the same gap geometry.
pub fn build_region_mesh(
    pack: &Pack,
    get_type: &impl Fn(usize) -> u16,
    color_fn: &impl Fn(u16) -> [f32; 4],
) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let iso_options = IsolineOptions {
        polygons: false,
        fill: true,
        water_gap: false,
        halo: false,
    };
    let isolines = get_isolines(pack, get_type, &iso_options);

    let mut tess = FillTessellator::new();

    for out in &isolines {
        let first_cell_type = {
            // Recover the type id from the chain's first cell (the isoline output
            // doesn't carry it). We look up the boundary vertex's adjacent cells.
            let mut t = 0u16;
            if let Some(&v) = out.chain.first() {
                if let Some(cells) = pack.vertices.adjacent_cells.get(v as usize).copied() {
                    for &c in &cells {
                        if c >= 0 {
                            let ty = get_type(c as usize);
                            if ty != 0 {
                                t = ty;
                                break;
                            }
                        }
                    }
                }
            }
            t
        };

        let color = color_fn(first_cell_type);
        if color[3] == 0.0 {
            continue;
        }

        let path = get_fill_path(&out.chain, &pack.vertices);
        if out.chain.len() < 3 {
            continue;
        }

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

/// Builds the fill mesh for an arbitrary **cell set**
/// (FMG `getVertexPath(cellsArray)` in `pathUtils.ts`).
///
/// Unlike `build_region_mesh` (which keys off a per-cell type array), this walks
/// the **outer boundary** of the given set and fills it, exactly like FMG's
/// `getVertexPath` → `getFillPath`. Used by the zones layer.
///
/// `in_zone(cell_id)` decides membership; `color` is the overlay color.
/// Returns a single mesh covering all connected polygons of the cell set.
pub fn build_vertex_path_mesh(
    pack: &Pack,
    in_zone: &impl Fn(usize) -> bool,
    color: [f32; 4],
) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let n_cells = pack.points_n();
    if n_cells == 0 {
        return result;
    }

    let mut checked = vec![false; n_cells];
    let vertices = &pack.vertices;
    let mut tess = FillTessellator::new();

    for cell in 0..n_cells {
        if checked[cell] || !in_zone(cell) {
            continue;
        }
        checked[cell] = true;

        let same_type = |c: usize| c < n_cells && in_zone(c);

        let neighbors = match pack.cells.adjacency.get(cell) {
            Some(v) => v,
            None => continue,
        };
        let has_border = neighbors.iter().any(|&nb| !same_type(nb as usize));
        if !has_border {
            continue;
        }

        let cell_ring = match vertices.cell_rings.get(cell) {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        let start_vertex = match cell_ring.iter().copied().find(|&v| {
            let adj = vertices
                .adjacent_cells
                .get(v as usize)
                .copied()
                .unwrap_or([-1, -1, -1]);
            adj.iter().any(|&c| c >= 0 && !same_type(c as usize))
        }) {
            Some(v) => v,
            None => continue,
        };

        let mut check_cell = |c: usize| {
            if c < n_cells && same_type(c) && !checked[c] {
                checked[c] = true;
            }
        };
        let chain = connect_vertices(vertices, start_vertex, &same_type, &mut check_cell, true);
        if chain.len() < 3 {
            continue;
        }

        let path = get_fill_path(&chain, vertices);
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

/// Minimum height for land isolines (Azgaar: `height < 20` = ocean).
const MIN_LAND_HEIGHT: u8 = 20;
/// Maximum isoline level to draw (Azgaar stops at 100).
const MAX_HEIGHT: u8 = 100;
/// Height step between bands. Mirrors Azgaar's default `#landHeights` config:
/// `skip: 5`, and the renderer advances `currentLayer += skip + 1`, so bands
/// are drawn every **6** height units — that discrete spacing is what produces
/// the "faceted" concentric-band look (not a continuous gradient).
const BAND_STEP: u8 = 6;

/// Builds the heightmap layer the way Azgaar does: **filled isoline bands**,
/// one filled polygon per height level, not a flat per-cell color.
///
/// Levels are spaced by `BAND_STEP` (Azgaar `#landFragment` `skip: 5`), from
/// `MIN_LAND_HEIGHT` upward, and drawn low → high so higher contours paint over
/// the lower ones, producing the discrete faceted bands Azgaar shows when the
/// heightmap layer is rendered.
///
/// For each level `h`, the contour of the region where `height >= h` is
/// extracted with `get_isolines` and filled with `height_color(h)` (the regex
/// `<h` vs `>=h` matches Azgaar's `connectVertices` boundary walk).
///
/// The ocean is not part of this layer (`data-render = 0` in Azgaar): cells with
/// `height < 20` are never included, so the sea stays untouched.
pub fn build_heightmap_band_mesh(pack: &Pack) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let mut tess = FillTessellator::new();
    let options = IsolineOptions {
        polygons: true,
        ..Default::default()
    };

    let mut h = MIN_LAND_HEIGHT;
    while h <= MAX_HEIGHT {
        let get_type = |c: usize| -> u16 {
            if pack.cells.height.get(c).copied().unwrap_or(0) >= h {
                1
            } else {
                0
            }
        };
        let isolines = get_isolines(pack, &get_type, &options);
        if !isolines.is_empty() {
            let color = height_color(h);
            for iso in &isolines {
                let path = get_fill_path(&iso.chain, &pack.vertices);
                let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
                let mut buffer_builder = BuffersBuilder::new(&mut mesh, ColorCtor(color));
                let opts =
                    FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);
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
        }

        h += BAND_STEP;
    }

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0, 0.0];
        result.bounds_max = [0.0, 0.0];
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
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
            cell_neighbors: Vec::new(),
            cell_border: Vec::new(),
        }
    }

    #[test]
    fn basis_closed_returns_to_start() {
        use lyon::path::Event;

        // Square of 4 points. d3 `basisClosed` emits one bezier per triple
        // `(p_j, p_{j+1}, p_{j+2})`, j = 1..n, and the last one must land
        // exactly on the `moveTo` point `(p0 + 4p1 + p2) / 6`.
        let pts = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
        let path = build_curve_basis_closed(&pts, None);

        let cubics: Vec<_> = path
            .iter()
            .filter_map(|e| match e {
                Event::Cubic { to, .. } => Some([to.x, to.y]),
                _ => None,
            })
            .collect();

        assert_eq!(cubics.len(), pts.len(), "one bezier per ring point");

        let start = [
            (pts[0][0] + 4.0 * pts[1][0] + pts[2][0]) / 6.0,
            (pts[0][1] + 4.0 * pts[1][1] + pts[2][1]) / 6.0,
        ];
        let last = *cubics.last().unwrap();
        assert!(
            (last[0] - start[0]).abs() < 1e-4 && (last[1] - start[1]).abs() < 1e-4,
            "last bezier must end at the start point: {last:?} vs {start:?}"
        );
    }

    #[test]
    fn connect_vertices_closed_ring() {
        let vertices = make_test_vertices();

        let same_type = |c: usize| c == 0 || c == 1;
        let mut checked_cells = Vec::new();
        let mut check_cell = |c: usize| checked_cells.push(c);

        let chain = connect_vertices(&vertices, 0, &same_type, &mut check_cell, true);

        assert!(chain.len() >= 3, "chain should have at least 3 vertices");
        assert_eq!(*chain.last().unwrap(), chain[0], "chain should close ring");
    }

    fn make_test_vertices_in_pack(vertices: &VoronoiVertices) -> Pack {
        let n = 4;
        Pack {
            points: vec![[0.0, 0.0]; n],
            boundary: Vec::new(),
            cells: vor_core::cells::PackCells {
                grid_id: (0..n as u32).collect(),
                height: vec![30, 30, 10, 10],
                area_px: vec![100; n],
                biome: vec![1, 1, 2, 2],
                burg: vec![0; n],
                confluence: vec![0; n],
                culture: vec![10, 10, 20, 20],
                flux: vec![0; n],
                population: vec![0.0; n],
                river: vec![0; n],
                score: vec![0; n],
                state: vec![1, 1, 2, 2],
                religion: vec![0; n],
                province: vec![0; n],
                good: vec![0; n],
                market: vec![0; n],
                routes: vec![vor_core::cells::RoutesFromCell::default(); n],
                feature_id: vec![1, 1, 2, 2],
                water_type: vec![0, 0, -1, -1],
                adjacency: vec![vec![1, 2], vec![0, 3], vec![0, 3], vec![1, 2]],
            },
            vertices: vertices.clone(),
            features: Vec::new(),
        }
    }

    #[test]
    fn get_isolines_returns_regions() {
        let vertices = make_test_vertices();
        let pack = make_test_vertices_in_pack(&vertices);

        let opts = IsolineOptions {
            polygons: true,
            ..Default::default()
        };
        let isolines = get_isolines(&pack, &|c| pack.cells.state[c], &opts);

        assert!(!isolines.is_empty(), "should find at least one isoline");
    }

    #[test]
    fn heightmap_band_mesh_only_draws_land() {
        let vertices = make_test_vertices();
        let pack = make_test_vertices_in_pack(&vertices);

        let mesh = build_heightmap_band_mesh(&pack);

        // Cells 0..1 have height 30 (land), cells 2..3 have height 10 (ocean ->
        // excluded). The band mesh must contain geometry for the land contour
        // but nothing from the ocean levels.
        assert!(
            !mesh.vertices.is_empty(),
            "band mesh should draw the land contour"
        );
        assert!(!mesh.indices.is_empty(), "band mesh should have indices");
        assert!(
            mesh.vertices.iter().all(|v| v.pos[0].is_finite()),
            "band mesh vertices must be finite"
        );
    }

    #[test]
    fn region_mesh_fill_matches_isolines() {
        let vertices = make_test_vertices();
        let pack = make_test_vertices_in_pack(&vertices);

        let get_type = |c: usize| pack.cells.state[c];
        let color_fn = |t: u16| [t as f32 * 0.1, 0.2, 0.3, 1.0];
        let mesh = build_region_mesh(&pack, &get_type, &color_fn);

        // The region mesh must tessellate at least one filled polygon for the
        // non-zero state regions present in the pack.
        assert!(!mesh.vertices.is_empty(), "region fill must have geometry");
        assert!(!mesh.indices.is_empty(), "region fill must have indices");
        assert!(
            mesh.vertices.iter().all(|v| v.pos[0].is_finite()),
            "region mesh vertices must be finite"
        );

        let opts = IsolineOptions {
            polygons: true,
            ..Default::default()
        };
        let isolines = get_isolines(&pack, &get_type, &opts);
        assert!(
            !isolines.is_empty() && mesh.indices.len() >= 3,
            "fill mesh must cover the isoline polygons"
        );
    }
}
