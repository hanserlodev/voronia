use vor_core::entities::zone::Zone;
use vor_core::pack::Pack;
use vor_core::voronoi::VoronoiVertices;

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::isoline::build_vertex_path_mesh;

/// Builds the zone layer: one outer-boundary fill per zone (FMG
/// `getVertexPath(cellsArray)`), colored with the zone color at ~35% opacity
/// (Azgaar's overlay look).
pub fn build_zone_mesh(vertices: &VoronoiVertices, pack: &Pack, zones: &[Zone]) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    for zone in zones {
        if zone.hidden || zone.cells.is_empty() {
            continue;
        }
        let color = if zone.color.is_empty() {
            [0.5, 0.5, 0.5, 0.35]
        } else {
            let mut c = hex_color_to_linear(&zone.color);
            c[3] = 0.6;
            c
        };
        let in_zone = |c: usize| zone.cells.contains(&(c as u32));
        let m = build_vertex_path_mesh(pack, &in_zone, color);
        let base = result.vertices.len() as u32;
        result.vertices.extend(m.vertices);
        result
            .indices
            .extend(m.indices.into_iter().map(|i| i + base));
        result.bounds_min[0] = result.bounds_min[0].min(m.bounds_min[0]);
        result.bounds_min[1] = result.bounds_min[1].min(m.bounds_min[1]);
        result.bounds_max[0] = result.bounds_max[0].max(m.bounds_max[0]);
        result.bounds_max[1] = result.bounds_max[1].max(m.bounds_max[1]);
    }

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0, 0.0];
        result.bounds_max = [0.0, 0.0];
    }
    let _ = vertices;
    result
}
