use vor_core::entities::zone::Zone;
use vor_core::pack::Pack;
use vor_core::voronoi::VoronoiVertices;

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::mesh::build_pack_mesh;

pub fn build_zone_mesh(vertices: &VoronoiVertices, pack: &Pack, zones: &[Zone]) -> HeightmapMesh {
    let mut cell_colors = vec![[0.0f32; 4]; pack.points_n()];

    for zone in zones {
        let color = if zone.color.is_empty() {
            [0.5, 0.5, 0.5, 0.3]
        } else {
            let mut c = hex_color_to_linear(&zone.color);
            c[3] = 0.35;
            c
        };
        for &c in &zone.cells {
            if let Some(dst) = cell_colors.get_mut(c as usize) {
                *dst = color;
            }
        }
    }

    build_pack_mesh(vertices, pack.points_n(), |p| cell_colors[p])
}
