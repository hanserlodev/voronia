use vor_core::entities::religion::Religion;
use vor_core::voronoi::VoronoiVertices;
use vor_core::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::isoline::build_region_mesh;
use crate::water_gap::append_water_gap_raw;

pub fn build_religion_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    religions: &[Religion],
    is_water: &[bool],
) -> HeightmapMesh {
    let get_type = |p: usize| pack.cells.religion.get(p).copied().unwrap_or(0);
    let color_fn = |rid: u16| match religions.get(rid as usize) {
        Some(r) if !r.color.is_empty() => {
            let mut color = hex_color_to_linear(&r.color);
            color[3] = 0.7;
            color
        }
        _ => [0.0, 0.0, 0.0, 0.0],
    };
    let mut mesh = build_region_mesh(pack, &get_type, &color_fn);
    append_water_gap_raw(&mut mesh, pack, is_water, |p| {
        let rid = pack.cells.religion.get(p).copied().unwrap_or(0);
        color_fn(rid)
    });
    let _ = vertices;
    mesh
}
