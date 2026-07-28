use vor_core::entities::culture::Culture;
use vor_core::Pack;
use vor_core::voronoi::VoronoiVertices;

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::mesh::build_pack_mesh;

pub fn build_culture_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    cultures: &[Culture],
) -> HeightmapMesh {
    build_pack_mesh(vertices, pack.points_n(), |p| {
        let cid = pack.cells.culture.get(p).copied().unwrap_or(0) as usize;
        match cultures.get(cid) {
            Some(c) if !c.color.is_empty() => hex_color_to_linear(&c.color),
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    })
}
