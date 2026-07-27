use vor_core::entities::river::River;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// Construye la malla de ríos: segmentos de polilínea (quads) entre celdas
/// consecutivas del `cell_path` de cada río.
pub fn build_river_mesh(points: &[[f32; 2]], rivers: &[River]) -> HeightmapMesh {
    let mut vertices: Vec<HeightmapVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut bounds_min = [f32::INFINITY, f32::INFINITY];
    let mut bounds_max = [f32::NEG_INFINITY, f32::NEG_INFINITY];

    for r in rivers.iter() {
        let path = &r.cell_path;
        if path.len() < 2 {
            continue;
        }

        // Ancho base según caudal relativo (1-4 px)
        let width = (r.discharge_m3s / 5000.0).clamp(1.0, 6.0);

        // Color azul río
        let color = [0.2, 0.4, 0.8, 0.85];

        for pair in path.windows(2) {
            let a = points.get(pair[0] as usize).copied().unwrap_or([0.0, 0.0]);
            let b = points.get(pair[1] as usize).copied().unwrap_or([0.0, 0.0]);

            // Quad orientado perpendicular al segmento
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.001 {
                continue;
            }
            let nx = -dy / len * width;
            let ny = dx / len * width;

            let base = vertices.len() as u32;
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

            indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);

            for v in &vertices[base as usize..][..4] {
                bounds_min[0] = bounds_min[0].min(v.pos[0]);
                bounds_min[1] = bounds_min[1].min(v.pos[1]);
                bounds_max[0] = bounds_max[0].max(v.pos[0]);
                bounds_max[1] = bounds_max[1].max(v.pos[1]);
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
