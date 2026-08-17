use vor_core::entities::state::State;
use vor_core::pack::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// Number of triangle fan segments per burg icon circle.
const ICON_SEGMENTS: u32 = 12;
/// Circle radius of a burg icon, in map units (FMG `#icon-circle` ~ r=2.5).
const ICON_RADIUS: f32 = 2.5;

/// Builds the burg icon mesh: one circle per burg, colored by the state it
/// belongs to (FMG `#icon-circle`). Harbor burgs get a smaller anchor circle
/// drawn on top (FMG `#anchors > #icon-anchor`).
pub fn build_burg_icons_mesh(pack: &Pack, states: &[State]) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let n = pack.points_n();
    for p in 0..n {
        let burg_id = pack.cells.burg.get(p).copied().unwrap_or(0);
        if burg_id == 0 {
            continue;
        }
        let center = pack.points.get(p).copied().unwrap_or([0.0, 0.0]);
        let sid = pack.cells.state.get(p).copied().unwrap_or(0) as usize;
        let color = match states.get(sid) {
            Some(s) if !s.color.is_empty() => hex_color_to_linear(&s.color),
            _ => [0.7, 0.7, 0.7, 1.0],
        };
        push_circle(&mut mesh, center, ICON_RADIUS, color);
    }

    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0, 0.0];
        mesh.bounds_max = [0.0, 0.0];
    }
    mesh
}

fn push_circle(mesh: &mut HeightmapMesh, center: [f32; 2], radius: f32, color: [f32; 4]) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(HeightmapVertex { pos: center, color });
    for i in 0..ICON_SEGMENTS {
        let a = i as f32 / ICON_SEGMENTS as f32 * std::f32::consts::TAU;
        let x = center[0] + radius * a.cos();
        let y = center[1] + radius * a.sin();
        mesh.vertices.push(HeightmapVertex { pos: [x, y], color });
    }
    for i in 0..ICON_SEGMENTS {
        let v0 = base + 1 + i;
        let v1 = base + 1 + (i + 1) % ICON_SEGMENTS;
        mesh.indices.extend_from_slice(&[base, v0, v1]);
    }
    for v in &mesh.vertices[base as usize..] {
        mesh.bounds_min[0] = mesh.bounds_min[0].min(v.pos[0]);
        mesh.bounds_min[1] = mesh.bounds_min[1].min(v.pos[1]);
        mesh.bounds_max[0] = mesh.bounds_max[0].max(v.pos[0]);
        mesh.bounds_max[1] = mesh.bounds_max[1].max(v.pos[1]);
    }
}
