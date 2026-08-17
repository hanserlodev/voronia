use vor_core::entities::state::State;
use vor_core::voronoi::VoronoiVertices;
use vor_core::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::isoline::build_region_mesh;
use crate::water_gap::append_water_gap_raw;

pub fn build_state_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    states: &[State],
    is_water: &[bool],
) -> HeightmapMesh {
    let get_type = |p: usize| pack.cells.state.get(p).copied().unwrap_or(0);
    let color_fn = |sid: u16| match states.get(sid as usize) {
        Some(s) if !s.color.is_empty() => {
            let mut c = hex_color_to_linear(&s.color);
            c[3] = 0.4;
            c
        }
        _ => [0.0, 0.0, 0.0, 0.0],
    };
    let mut mesh = build_region_mesh(pack, &get_type, &color_fn);
    append_water_gap_raw(&mut mesh, pack, is_water, |p| {
        let sid = pack.cells.state.get(p).copied().unwrap_or(0);
        color_fn(sid)
    });
    let _ = vertices;
    mesh
}
