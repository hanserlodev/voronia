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

    let n = pack.points_n();
    let mut vertices: Vec<HeightmapVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut bounds_min = [f32::INFINITY, f32::INFINITY];
    let mut bounds_max = [f32::NEG_INFINITY, f32::NEG_INFINITY];

    for p in 0..n {
        let pid = ids.get(p).copied().unwrap_or(0);
        let neighbors = match pack.cells.adjacency.get(p) {
            Some(v) => v,
            None => continue,
        };
        for &nb in neighbors {
            if (nb as usize) < n && ids.get(nb as usize).copied().unwrap_or(0) != pid {
                // Border between p and nb: draw segment
                let a = pack.points.get(p).copied().unwrap_or([0.0, 0.0]);
                let b = pack.points.get(nb as usize).copied().unwrap_or([0.0, 0.0]);

                let base = vertices.len() as u32;
                // Line as a thin quad (1 px wide)
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let len = (dx * dx + dy * dy).sqrt();
                if len < 0.001 {
                    continue;
                }
                let nx = -dy / len * 0.5;
                let ny = dx / len * 0.5;

                vertices.push(HeightmapVertex {
                    pos: [a[0] + nx, a[1] + ny],
                    color,
                });
                vertices.push(HeightmapVertex {
                    pos: [a[0] - nx, a[1] - ny],
                    color,
                });
                vertices.push(HeightmapVertex {
                    pos: [b[0] + nx, b[1] + ny],
                    color,
                });
                vertices.push(HeightmapVertex {
                    pos: [b[0] - nx, b[1] - ny],
                    color,
                });

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
