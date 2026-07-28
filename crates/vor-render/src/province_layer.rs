use vor_core::entities::province::Province;
use vor_core::Pack;
use vor_core::voronoi::VoronoiVertices;

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::mesh::build_pack_mesh;

pub fn build_province_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    provinces: &[Province],
) -> HeightmapMesh {
    build_pack_mesh(vertices, pack.points_n(), |p| {
        let pid = pack.cells.province.get(p).copied().unwrap_or(0) as usize;
        match provinces.get(pid) {
            Some(pr) if !pr.color.is_empty() => hex_color_to_linear(&pr.color),
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    })
}
