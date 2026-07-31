use vor_core::entities::state::State;
use vor_core::pack::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// Builds the burg mesh: one triangle at each burg position, colored by the
/// state it belongs to.
pub fn build_burg_mesh(pack: &Pack, states: &[State]) -> HeightmapMesh {
    let n = pack.points_n();
    let mut vertices: Vec<HeightmapVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut bounds_min = [f32::INFINITY, f32::INFINITY];
    let mut bounds_max = [f32::NEG_INFINITY, f32::NEG_INFINITY];
    let mut added = 0usize;

    for p in 0..n {
        let burg_id = pack.cells.burg.get(p).copied().unwrap_or(0);
        if burg_id == 0 {
            continue;
        }
        added += 1;
        let center = pack.points.get(p).copied().unwrap_or([0.0, 0.0]);
        let sid = pack.cells.state.get(p).copied().unwrap_or(0) as usize;
        let color = match states.get(sid) {
            Some(s) if !s.color.is_empty() => hex_color_to_linear(&s.color),
            _ => [0.7, 0.7, 0.7, 1.0],
        };

        let size = 4.0;
        let base = vertices.len() as u32;
        vertices.push(HeightmapVertex {
            pos: [center[0], center[1] + size],
            color,
        });
        vertices.push(HeightmapVertex {
            pos: [center[0] - size * 0.866, center[1] - size * 0.5],
            color,
        });
        vertices.push(HeightmapVertex {
            pos: [center[0] + size * 0.866, center[1] - size * 0.5],
            color,
        });

        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);

        for v in &vertices[base as usize..][..3] {
            bounds_min[0] = bounds_min[0].min(v.pos[0]);
            bounds_min[1] = bounds_min[1].min(v.pos[1]);
            bounds_max[0] = bounds_max[0].max(v.pos[0]);
            bounds_max[1] = bounds_max[1].max(v.pos[1]);
        }
    }

    if added == 0 {
        return HeightmapMesh {
            vertices,
            indices,
            bounds_min: [0.0, 0.0],
            bounds_max: [0.0, 0.0],
        };
    }

    HeightmapMesh {
        vertices,
        indices,
        bounds_min,
        bounds_max,
    }
}
