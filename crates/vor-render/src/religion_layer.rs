use vor_core::entities::religion::Religion;
use vor_core::Pack;
use vor_core::voronoi::VoronoiVertices;

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::mesh::build_pack_mesh;

pub fn build_religion_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    religions: &[Religion],
) -> HeightmapMesh {
    build_pack_mesh(vertices, pack.points_n(), |p| {
        let rid = pack.cells.religion.get(p).copied().unwrap_or(0) as usize;
        match religions.get(rid) {
            Some(r) if !r.color.is_empty() => hex_color_to_linear(&r.color),
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    })
}
