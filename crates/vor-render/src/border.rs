use std::collections::HashMap;

use vor_core::pack::Pack;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// Type of border to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderKind {
    State,
    Province,
    Culture,
}

/// Builds the border mesh: segments between neighboring cells that belong to a
/// different state/province/culture.
pub fn build_border_mesh(pack: &Pack, kind: BorderKind) -> HeightmapMesh {
    let (ids, color): (&[u16], [f32; 4]) = match kind {
        BorderKind::State => (&pack.cells.state, [0.9, 0.1, 0.1, 0.9]),
        BorderKind::Province => (&pack.cells.province, [0.7, 0.7, 0.1, 0.7]),
        BorderKind::Culture => (&pack.cells.culture, [1.0, 0.65, 0.0, 0.8]),
    };

    let mut vertices: Vec<HeightmapVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut bounds_min = [f32::INFINITY, f32::INFINITY];
    let mut bounds_max = [f32::NEG_INFINITY, f32::NEG_INFINITY];

    // Each Voronoi edge belongs to two cell rings. Keep the first side and
    // emit only when the second side has a different non-zero region id. This
    // draws the actual shared circumcenter edge, not a center-to-center chord,
    // and prevents the duplicated borders produced by iterating adjacency both
    // ways.
    let mut edges: HashMap<(u32, u32), (usize, u16)> = HashMap::new();
    for (cell, ring) in pack.vertices.cell_rings.iter().enumerate() {
        let id = ids.get(cell).copied().unwrap_or(0);
        if id == 0 || ring.len() < 2 {
            continue;
        }
        for i in 0..ring.len() {
            let a_id = ring[i];
            let b_id = ring[(i + 1) % ring.len()];
            if a_id == b_id {
                continue;
            }
            let key = if a_id < b_id {
                (a_id, b_id)
            } else {
                (b_id, a_id)
            };
            if let Some((other_cell, other_id)) = edges.remove(&key) {
                if other_id == id || other_id == 0 {
                    continue;
                }
                let a = pack
                    .vertices
                    .positions
                    .get(key.0 as usize)
                    .copied()
                    .unwrap_or([0.0, 0.0]);
                let b = pack
                    .vertices
                    .positions
                    .get(key.1 as usize)
                    .copied()
                    .unwrap_or([0.0, 0.0]);
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let len = (dx * dx + dy * dy).sqrt();
                if len < 0.001 {
                    continue;
                }
                let width = match kind {
                    BorderKind::State => 0.5,
                    BorderKind::Province => 0.25,
                    BorderKind::Culture => 0.35,
                };
                let nx = -dy / len * width;
                let ny = dx / len * width;
                let base = vertices.len() as u32;
                vertices.extend_from_slice(&[
                    HeightmapVertex {
                        pos: [a[0] + nx, a[1] + ny],
                        color,
                    },
                    HeightmapVertex {
                        pos: [a[0] - nx, a[1] - ny],
                        color,
                    },
                    HeightmapVertex {
                        pos: [b[0] + nx, b[1] + ny],
                        color,
                    },
                    HeightmapVertex {
                        pos: [b[0] - nx, b[1] - ny],
                        color,
                    },
                ]);
                indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base + 1,
                    base + 3,
                    base + 2,
                ]);
                for v in &vertices[base as usize..][..4] {
                    bounds_min[0] = bounds_min[0].min(v.pos[0]);
                    bounds_min[1] = bounds_min[1].min(v.pos[1]);
                    bounds_max[0] = bounds_max[0].max(v.pos[0]);
                    bounds_max[1] = bounds_max[1].max(v.pos[1]);
                }
                let _ = other_cell;
            } else {
                edges.insert(key, (cell, id));
            }
        }
    }

    if !bounds_min[0].is_finite() {
        bounds_min = [0.0, 0.0];
        bounds_max = [0.0, 0.0];
    }

    HeightmapMesh {
        vertices,
        indices,
        bounds_min,
        bounds_max,
    }
}
