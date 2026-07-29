use vor_core::entities::river::River;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use crate::mesh::catmull_rom_open;

/// Construye la malla de ríos: segmentos de polilínea suavizada (quads) a lo largo
/// del `cell_path` de cada río, con Catmull-Rom para curvas orgánicas.
///
/// Si el río no alcanza su `mouth_cell` (porque el path tracing usa celdas pack
/// pero mouth_cell está en namespace grid), extiende el último segmento hacia el
/// océano para que la desembocadura sea visible.
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
        let mut raw: Vec<[f32; 2]> = path
            .iter()
            .filter_map(|&ci| points.get(ci as usize).copied())
            .collect();
        if raw.len() < 2 {
            continue;
        }

        // Extender el último segmento hasta la costa: media celda más allá
        // del último centro de celda (donde el path tracing se queda corto porque
        // mouth_cell está en namespace grid y no calza con pack).
        let last = raw.last().copied().unwrap_or([0.0; 2]);
        let prev = raw[raw.len().saturating_sub(2)];
        let dx = last[0] - prev[0];
        let dy = last[1] - prev[1];
        let seg_len = (dx * dx + dy * dy).sqrt().max(1.0);
        let extend = seg_len * 0.3;
        raw.push([last[0] + dx / seg_len * extend, last[1] + dy / seg_len * extend]);

        let smooth = catmull_rom_open(&raw, 30);

        // Ancho base según caudal relativo
        let width = (r.discharge_m3s / 3000.0).clamp(0.8, 5.0);

        // Color azul río
        let color = [0.15, 0.45, 0.85, 1.0];

        for pair in smooth.windows(2) {
            let a = pair[0];
            let b = pair[1];

            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let seg_len = (dx * dx + dy * dy).sqrt();
            if seg_len < 0.001 {
                continue;
            }
            let nx = -dy / seg_len * width;
            let ny = dx / seg_len * width;

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

    if !bounds_min.iter().all(|v| v.is_finite()) {
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
