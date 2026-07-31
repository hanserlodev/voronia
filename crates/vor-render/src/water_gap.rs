use vor_core::pack::Pack;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// Builds the water gap mesh for a thematic layer.
///
/// Draws thin quads (1px) along the edges between land cells and water cells,
/// colored with the land cell color according to `color_fn(cell_id)`. This
/// prevents thematic colors from visually "bleeding" into the ocean (Azgaar's
/// "water gap" technique).
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
    let n = pack.points_n();

    for p in 0..n {
        let water_p = is_water.get(p).copied().unwrap_or(true);
        if color_fn(p)[3] == 0.0 {
            continue;
        }
        let neighbors = match pack.cells.adjacency.get(p) {
            Some(v) => v,
            None => continue,
        };
        for &nb in neighbors {
            let nb_idx = nb as usize;
            if nb_idx >= n {
                continue;
            }
            let water_nb = is_water.get(nb_idx).copied().unwrap_or(true);
            if water_p == water_nb {
                continue;
            }
            let land_idx = if water_p { nb_idx } else { p };
            let a = pack.points.get(p).copied().unwrap_or([0.0, 0.0]);
            let b = pack.points.get(nb_idx).copied().unwrap_or([0.0, 0.0]);
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.001 {
                continue;
            }
            let gap_color = color_fn(land_idx);
            if gap_color[3] == 0.0 {
                continue;
            }
            let nx = -dy / len * 0.5;
            let ny = dx / len * 0.5;
            let base = mesh.vertices.len() as u32;
            mesh.vertices.push(HeightmapVertex {
                pos: [a[0] + nx, a[1] + ny],
                color: gap_color,
            });
            mesh.vertices.push(HeightmapVertex {
                pos: [a[0] - nx, a[1] - ny],
                color: gap_color,
            });
            mesh.vertices.push(HeightmapVertex {
                pos: [b[0] + nx, b[1] + ny],
                color: gap_color,
            });
            mesh.vertices.push(HeightmapVertex {
                pos: [b[0] - nx, b[1] - ny],
                color: gap_color,
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
