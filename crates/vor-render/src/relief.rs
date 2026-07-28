use vor_core::Pack;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};

pub fn build_relief_mesh(pack: &Pack) -> HeightmapMesh {
    let n = pack.points_n();
    let mut verts: Vec<HeightmapVertex> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();

    for p in 0..n {
        let h = pack.cells.height.get(p).copied().unwrap_or(0);
        if h < 40 {
            continue;
        }
        let center = pack.points.get(p).copied().unwrap_or([0.0; 2]);
        let size = if h >= 80 {
            6.0
        } else if h >= 60 {
            4.0
        } else {
            2.5
        };
        let color = if h >= 80 {
            [0.6, 0.55, 0.5, 0.7]
        } else if h >= 60 {
            [0.45, 0.55, 0.3, 0.6]
        } else {
            [0.3, 0.5, 0.25, 0.5]
        };

        let mut push_tri = |v0: [f32; 2], v1: [f32; 2], v2: [f32; 2]| {
            let base = verts.len() as u32;
            verts.push(HeightmapVertex { pos: v0, color });
            verts.push(HeightmapVertex { pos: v1, color });
            verts.push(HeightmapVertex { pos: v2, color });
            idx.push(base);
            idx.push(base + 1);
            idx.push(base + 2);
        };

        if h >= 80 {
            push_tri(
                [center[0], center[1] - size],
                [center[0] - size * 0.7, center[1] + size * 0.5],
                [center[0] + size * 0.7, center[1] + size * 0.5],
            );
        } else {
            push_tri(
                [center[0], center[1] - size * 0.6],
                [center[0] - size * 0.5, center[1] + size * 0.4],
                [center[0] + size * 0.5, center[1] + size * 0.4],
            );
        }
    }

    let mut bounds_min = [f32::INFINITY, f32::INFINITY];
    let mut bounds_max = [f32::NEG_INFINITY, f32::NEG_INFINITY];
    for v in &verts {
        bounds_min[0] = bounds_min[0].min(v.pos[0]);
        bounds_min[1] = bounds_min[1].min(v.pos[1]);
        bounds_max[0] = bounds_max[0].max(v.pos[0]);
        bounds_max[1] = bounds_max[1].max(v.pos[1]);
    }
    if !bounds_min.iter().all(|v| v.is_finite()) {
        bounds_min = [0.0; 2];
        bounds_max = [0.0; 2];
    }

    HeightmapMesh {
        vertices: verts,
        indices: idx,
        bounds_min,
        bounds_max,
    }
}
