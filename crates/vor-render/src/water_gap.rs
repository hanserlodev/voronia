use vor_core::pack::Pack;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use crate::mesh::laplacian_smooth_vertices;

/// Half-width of the water gap quads, in map units.
///
/// Azgaar renders the gap as `stroke-width: 3` over the isoline border path
/// (`getGappedFillPaths` in `public/modules/ui/layers.js`), so the quad must span
/// 1.5 map units on each side of the shared Voronoi edge (3 total).
const GAP_HALF_WIDTH: f32 = 1.5;

/// Builds the water gap mesh for a thematic layer.
///
/// Draws thin quads along the shared **Voronoi edge** between land cells and
/// water cells, colored with the land cell color according to `color_fn(cell_id)`.
/// This prevents thematic colors from visually "bleeding" into the ocean (Azgaar's
/// "water gap" technique — `getBorderPath` + `stroke` in `getGappedFillPaths`).
///
/// `is_water[i]` must be `true` if cell i is water (ocean or lake).
pub fn build_water_gap_mesh(
    pack: &Pack,
    is_water: &[bool],
    color_fn: impl Fn(usize) -> [f32; 4],
) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };
    append_water_gap(&mut mesh, pack, is_water, color_fn);
    mesh
}

/// Appends water gap geometry to an existing `HeightmapMesh` (modified in-place).
///
/// Useful for merging the water gap directly into the thematic fill mesh,
/// avoiding the need to add an extra layer to the renderer.
///
/// `is_water[i]` must be `true` if cell i is water (ocean or lake).
pub fn append_water_gap(
    mesh: &mut HeightmapMesh,
    pack: &Pack,
    is_water: &[bool],
    color_fn: impl Fn(usize) -> [f32; 4],
) {
    let smooth = laplacian_smooth_vertices(&pack.vertices, 0.2, 2);
    append_water_gap_with_positions(mesh, pack, is_water, color_fn, &smooth.positions);
}

/// Appends the gap on the raw Voronoi vertices. Regional isoline fills use raw
/// vertices (`getFillPath`), so this variant keeps their gap on the exact same
/// boundary. Biome/cell fills continue using `append_water_gap` above because
/// those meshes use Laplacian-smoothed vertices.
pub fn append_water_gap_raw(
    mesh: &mut HeightmapMesh,
    pack: &Pack,
    is_water: &[bool],
    color_fn: impl Fn(usize) -> [f32; 4],
) {
    append_water_gap_with_positions(mesh, pack, is_water, color_fn, &pack.vertices.positions);
}

fn append_water_gap_with_positions(
    mesh: &mut HeightmapMesh,
    pack: &Pack,
    is_water: &[bool],
    color_fn: impl Fn(usize) -> [f32; 4],
    positions: &[[f32; 2]],
) {
    let n = pack.points_n();
    if n == 0 {
        return;
    }

    let is_water_of = |cell: usize| is_water.get(cell).copied().unwrap_or(true);

    for p in 0..n {
        if is_water_of(p) {
            continue; // gap color comes from the land cell; water cells add nothing
        }
        let color_p = color_fn(p);
        if color_p[3] == 0.0 {
            continue;
        }

        let ring = match pack.vertices.cell_rings.get(p) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        let neighbors = match pack.cells.adjacency.get(p) {
            Some(v) => v,
            None => continue,
        };

        for &nb in neighbors {
            let nb = nb as usize;
            if nb >= n || !is_water_of(nb) {
                continue;
            }

            // Find the shared Voronoi edge: the two circumcenters (triangle IDs)
            // whose triangles contain both land cell `p` and water cell `nb`.
            let mut shared = [u32::MAX; 2];
            let mut shared_count = 0usize;
            for &t in ring {
                let tri = pack
                    .vertices
                    .adjacent_cells
                    .get(t as usize)
                    .copied()
                    .unwrap_or([-1; 3]);
                if tri.contains(&(p as i32)) && tri.contains(&(nb as i32)) {
                    if shared_count < 2 {
                        shared[shared_count] = t;
                    }
                    shared_count += 1;
                }
            }
            if shared_count < 2 {
                continue;
            }

            let a = positions
                .get(shared[0] as usize)
                .copied()
                .unwrap_or([0.0, 0.0]);
            let b = positions
                .get(shared[1] as usize)
                .copied()
                .unwrap_or([0.0, 0.0]);
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.001 {
                continue;
            }

            let nx = -dy / len * GAP_HALF_WIDTH;
            let ny = dx / len * GAP_HALF_WIDTH;
            let base = mesh.vertices.len() as u32;
            mesh.vertices.push(HeightmapVertex {
                pos: [a[0] + nx, a[1] + ny],
                color: color_p,
            });
            mesh.vertices.push(HeightmapVertex {
                pos: [a[0] - nx, a[1] - ny],
                color: color_p,
            });
            mesh.vertices.push(HeightmapVertex {
                pos: [b[0] + nx, b[1] + ny],
                color: color_p,
            });
            mesh.vertices.push(HeightmapVertex {
                pos: [b[0] - nx, b[1] - ny],
                color: color_p,
            });
            mesh.indices.extend_from_slice(&[
                base,
                base + 1,
                base + 2,
                base + 1,
                base + 3,
                base + 2,
            ]);
            for v in &mesh.vertices[base as usize..][..4] {
                mesh.bounds_min[0] = mesh.bounds_min[0].min(v.pos[0]);
                mesh.bounds_min[1] = mesh.bounds_min[1].min(v.pos[1]);
                mesh.bounds_max[0] = mesh.bounds_max[0].max(v.pos[0]);
                mesh.bounds_max[1] = mesh.bounds_max[1].max(v.pos[1]);
            }
        }
    }

    if !mesh.bounds_min[0].is_finite() {
        mesh.bounds_min = [0.0, 0.0];
        mesh.bounds_max = [0.0, 0.0];
    }
}
