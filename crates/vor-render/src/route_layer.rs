use vor_core::entities::route::{Route, RouteGroup};

use crate::heightmap::{HeightmapMesh, HeightmapVertex};

fn route_color(group: RouteGroup) -> [f32; 4] {
    match group {
        RouteGroup::Roads => [0.5, 0.3, 0.1, 0.7],
        RouteGroup::Trails => [0.6, 0.5, 0.3, 0.5],
        RouteGroup::Searoutes => [0.2, 0.4, 0.8, 0.5],
    }
}

pub fn build_route_mesh(routes: &[Route]) -> HeightmapMesh {
    let mut verts: Vec<HeightmapVertex> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();

    for r in routes {
        let color = route_color(r.group);
        for pair in r.points.windows(2) {
            let a = [pair[0][0], pair[0][1]];
            let b = [pair[1][0], pair[1][1]];
            let base = verts.len() as u32;
            verts.push(HeightmapVertex { pos: a, color });
            verts.push(HeightmapVertex { pos: b, color });
            idx.push(base);
            idx.push(base + 1);
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
