//! Ocean bathymetry contours (FMG `#oceanLayers`, `src/renderers/ocean-layers.ts`).
//!
//! FMG stores a distance field on the grid cells (`grid.cells.t`, slot `[10]`,
//! restored verbatim on load): water cells get `-1` at the coast down to `-9`
//! in the deepest basins. `OceanLayers.draw()` traces one contour per requested
//! level (default `-6,-3,-1`), relaxes/simplifies the chain, clips it to the
//! canvas and fills each ring `#ecf2f9` at opacity `0.4 / levels.length`.
//! Levels are painted in list order, later ones over earlier ones.

use vor_core::grid::Grid;

use crate::biome::hex_color_to_linear;
use crate::clip_poly::clip_polygon;
use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use crate::isoline::build_curve_basis_closed;

use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};

/// FMG default `layers` attribute (`default.json`).
pub const DEFAULT_LIMITS: [i8; 3] = [-6, -3, -1];

/// FMG bathymetry fill color (`#ecf2f9`) in linear space.
fn bathymetry_color(opacity: f32) -> [f32; 4] {
    let mut c = hex_color_to_linear("#ecf2f9");
    c[3] = opacity;
    c
}

/// `rn(x, 2)` — Math.round to 2 decimals.
fn rn2(x: f32) -> f32 {
    (x * 100.0).round() / 100.0
}

struct ColorCtor([f32; 4]);

impl lyon::tessellation::FillVertexConstructor<HeightmapVertex> for ColorCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::FillVertex<'_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

/// Builds the bathymetry rings mesh for the given grid. `limits` are the
/// depth levels to draw, in paint order (later levels paint on top).
pub fn build_bathymetry_mesh(grid: &Grid, limits: &[i8]) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    if limits.is_empty() || limits.iter().any(|&l| l >= 0) {
        return result;
    }

    let verts = &grid.vertices;
    let n = grid.points_n();
    if n == 0 {
        return result;
    }

    // `grid.cells.t` (distance field), 0 for anything unmarked/boundary.
    let t_of = |c: usize| -> i8 {
        if c < n {
            grid.cells.water_type.get(c).copied().unwrap_or(0)
        } else {
            0
        }
    };

    let mut used = vec![false; n];

    // "Outside" classification for the contour walk (`c0/c1/c2` in FMG):
    // falsy `t` (0/undefined boundary) or exactly one level shallower.
    let outside = |lvl: i8, c: i32| -> bool {
        let tc = if c >= 0 && (c as usize) < n {
            grid.cells.water_type.get(c as usize).copied().unwrap_or(0)
        } else {
            0
        };
        tc == 0 || tc == lvl - 1
    };

    // `findStart(i, t)`: border cells start at a vertex touching a boundary
    // point; interior cells at the ring vertex whose aligned neighbor is
    // "outside" (t < level or falsy).
    let find_start = |i: usize, lvl: i8| -> Option<u32> {
        let ring = verts.cell_rings.get(i)?;
        if ring.is_empty() {
            return None;
        }
        if *verts.cell_border.get(i).unwrap_or(&0) == 1 {
            ring.iter().copied().find(|&v| {
                verts
                    .adjacent_cells
                    .get(v as usize)
                    .map(|cells| cells.iter().any(|&c| c >= 0 && c as usize >= n))
                    .unwrap_or(false)
            })
        } else {
            let neibs = verts.cell_neighbors.get(i)?;
            let idx = neibs
                .iter()
                .position(|&c| t_of(c as usize) < lvl || t_of(c as usize) == 0)?;
            ring.get(idx).copied()
        }
    };

    // Literal port of `connectVertices(start, t)` (ocean variant): walks the
    // boundary where the neighbor classification flips between "outside" and
    // the level itself, marking same-level cells as used along the way.
    let connect_vertices = |start: u32, lvl: i8, used: &mut Vec<bool>| -> Vec<u32> {
        let mut chain: Vec<u32> = Vec::new();
        let mut current = start;
        let mut i = 0usize;
        while i == 0 || (current != start && i < 10_000) {
            let prev = chain.last().copied();
            chain.push(current);

            if let Some(cells) = verts.adjacent_cells.get(current as usize) {
                for &c in cells {
                    if c >= 0 && (c as usize) < n && t_of(c as usize) == lvl {
                        used[c as usize] = true;
                    }
                }
            }

            let adj_cells = verts
                .adjacent_cells
                .get(current as usize)
                .copied()
                .unwrap_or([-1, -1, -1]);
            let adj_verts = verts
                .adjacent_vertices
                .get(current as usize)
                .copied()
                .unwrap_or([-1, -1, -1]);
            let c0 = outside(lvl, adj_cells[0]);
            let c1 = outside(lvl, adj_cells[1]);
            let c2 = outside(lvl, adj_cells[2]);
            let (v0, v1, v2) = (adj_verts[0], adj_verts[1], adj_verts[2]);

            let mut next = current;
            if v0 >= 0 && Some(v0 as u32) != prev && c0 != c1 {
                next = v0 as u32;
            } else if v1 >= 0 && Some(v1 as u32) != prev && c1 != c2 {
                next = v1 as u32;
            } else if v2 >= 0 && Some(v2 as u32) != prev && c0 != c2 {
                next = v2 as u32;
            }
            if next == *chain.last().unwrap() {
                break; // "Next vertex is not found"
            }
            current = next;
            i += 1;
        }
        chain.push(chain[0]);
        chain
    };

    // Trace chains per component (FMG main loop).
    let mut chains: Vec<(i8, Vec<[f32; 2]>)> = Vec::new();
    for cell in 0..n {
        let t = t_of(cell);
        if t > 0 || used[cell] || !limits.contains(&t) {
            continue;
        }
        let start = match find_start(cell, t) {
            Some(s) => s,
            None => continue,
        };
        used[cell] = true;
        let chain = connect_vertices(start, t, &mut used);
        if chain.len() < 4 {
            continue;
        }
        // `relax = 1 + t*-2`: keep every n-th point, plus any vertex touching
        // the map border (boundary points).
        let relax = (1 + t * -2) as usize;
        let touches_border = |v: u32| {
            verts
                .adjacent_cells
                .get(v as usize)
                .map(|cs| cs.iter().any(|&c| c >= 0 && c as usize >= n))
                .unwrap_or(false)
        };
        let relaxed: Vec<u32> = chain
            .iter()
            .enumerate()
            .filter(|&(i, &v)| i % relax == 0 || touches_border(v))
            .map(|(_, &v)| v)
            .collect();
        if relaxed.len() < 4 {
            continue;
        }
        let points: Vec<[f32; 2]> = relaxed
            .iter()
            .filter_map(|&v| verts.positions.get(v as usize).copied())
            .collect();
        let clipped = clip_polygon(
            &points,
            grid.width,
            grid.height,
            false, // FMG clipPoly without `secure`
        );
        chains.push((t, clipped));
    }

    // Paint per level in list order (later paths over earlier ones).
    let opacity = rn2(0.4 / limits.len() as f32);
    let color = bathymetry_color(opacity);
    let mut tess = FillTessellator::new();
    for &lvl in limits {
        for (_, pts) in chains.iter().filter(|(ct, _)| *ct == lvl) {
            if pts.len() < 3 {
                continue;
            }
            // `line().curve(curveBasisClosed)` + `round(d, 1)`.
            let path = build_curve_basis_closed(pts, Some(1.0));
            let mut buf: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
            if tess
                .tessellate_path(
                    &path,
                    &FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::NonZero),
                    &mut BuffersBuilder::new(&mut buf, ColorCtor(color)),
                )
                .is_err()
            {
                continue;
            }
            let base = result.vertices.len() as u32;
            let start = result.vertices.len();
            result.vertices.extend(buf.vertices);
            result.indices.extend(buf.indices.iter().map(|&i| i + base));
            for v in &result.vertices[start..] {
                result.bounds_min[0] = result.bounds_min[0].min(v.pos[0]);
                result.bounds_min[1] = result.bounds_min[1].min(v.pos[1]);
                result.bounds_max[0] = result.bounds_max[0].max(v.pos[0]);
                result.bounds_max[1] = result.bounds_max[1].max(v.pos[1]);
            }
        }
    }

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0; 2];
        result.bounds_max = [0.0; 2];
    }
    result
}
