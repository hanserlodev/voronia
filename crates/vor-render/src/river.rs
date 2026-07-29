use vor_core::entities::river::River;
use vor_core::Pack;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use crate::mesh::catmull_rom_open;

/// Simplifica una polilínea con Ramer-Douglas-Peucker (epsilon en pixels).
fn simplify_rdp(points: &[[f32; 2]], epsilon: f32) -> Vec<[f32; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut mask = vec![true; points.len()];

    fn perpendicular_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-12 {
            return ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt();
        }
        let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        let proj = [a[0] + t * dx, a[1] + t * dy];
        ((p[0] - proj[0]).powi(2) + (p[1] - proj[1]).powi(2)).sqrt()
    }

    fn rdp_recursive(points: &[[f32; 2]], mask: &mut [bool], first: usize, last: usize, epsilon: f32) {
        if last <= first + 1 {
            return;
        }
        let mut max_dist = 0.0_f32;
        let mut max_idx = first;
        for i in (first + 1)..last {
            let d = perpendicular_distance(points[i], points[first], points[last]);
            if d > max_dist {
                max_dist = d;
                max_idx = i;
            }
        }
        if max_dist > epsilon {
            rdp_recursive(points, mask, first, max_idx, epsilon);
            rdp_recursive(points, mask, max_idx, last, epsilon);
        } else {
            for i in (first + 1)..last {
                mask[i] = false;
            }
        }
    }

    rdp_recursive(points, &mut mask, 0, points.len() - 1, epsilon);
    points
        .iter()
        .enumerate()
        .filter(|&(i, _)| mask[i])
        .map(|(_, &p)| p)
        .collect()
}

/// Construye la malla de ríos con trazado suave:
/// 1. Simplifica el path de celdas (remove zigzags)
/// 2. Busca la celda desembocadura real en el pack (mouth_cell)
/// 3. Aplica Catmull-Rom con subdivisiones moderadas
pub fn build_river_mesh(pack: &Pack, rivers: &[River]) -> HeightmapMesh {
    let mut vertices: Vec<HeightmapVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut bounds_min = [f32::INFINITY, f32::INFINITY];
    let mut bounds_max = [f32::NEG_INFINITY, f32::NEG_INFINITY];

    // Precomputar mapping grid→pack para búsqueda rápida de mouth_cell
    let grid_to_pack: Vec<u32> = {
        let mut map = vec![u32::MAX; pack.cells.grid_id.len()];
        for (pack_id, &grid_id) in pack.cells.grid_id.iter().enumerate() {
            if (grid_id as usize) < map.len() {
                map[grid_id as usize] = pack_id as u32;
            }
        }
        map
    };

    for r in rivers.iter() {
        let path = &r.cell_path;
        if path.len() < 2 {
            continue;
        }
        let mut raw: Vec<[f32; 2]> = path
            .iter()
            .filter_map(|&ci| pack.points.get(ci as usize).copied())
            .collect();
        if raw.len() < 2 {
            continue;
        }

        // Buscar mouth_cell en el pack (es grid_id → convertirlo a pack_id)
        let mouth_pack = grid_to_pack.get(r.mouth_cell as usize).copied();
        if let Some(mp) = mouth_pack {
            if mp != u32::MAX {
                let last_raw = raw.last().copied().unwrap_or([0.0; 2]);
                let mouth_pos = pack.points.get(mp as usize).copied().unwrap_or(last_raw);
                // Solo agregar si es distinto del último punto
                let dx = mouth_pos[0] - last_raw[0];
                let dy = mouth_pos[1] - last_raw[1];
                if dx * dx + dy * dy > 1.0 {
                    raw.push(mouth_pos);
                }
            }
        }

        // Simplificar para eliminar zigzags de la malla Voronoi
        let simplified = simplify_rdp(&raw, 2.0);

        // Catmull-Rom con pocas subdivisiones (el path ya está simplificado)
        let smooth = catmull_rom_open(&simplified, 5);

        let width = (r.discharge_m3s / 3000.0).clamp(0.8, 5.0);
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
