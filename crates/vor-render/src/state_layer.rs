use vor_core::entities::state::State;
use vor_core::voronoi::VoronoiVertices;
use vor_core::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::mesh::build_pack_mesh;

pub fn build_state_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    states: &[State],
) -> HeightmapMesh {
    build_pack_mesh(vertices, pack.points_n(), |p| {
        let sid = pack.cells.state.get(p).copied().unwrap_or(0) as usize;
        match states.get(sid) {
            Some(s) if !s.color.is_empty() => hex_color_to_linear(&s.color),
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    })
}
