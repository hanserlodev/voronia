use vor_core::entities::province::Province;
use vor_core::voronoi::VoronoiVertices;
use vor_core::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::isoline::build_region_mesh;
use crate::water_gap::append_water_gap_raw;

pub fn build_province_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    provinces: &[Province],
    is_water: &[bool],
) -> HeightmapMesh {
    let get_type = |p: usize| pack.cells.province.get(p).copied().unwrap_or(0);
    let color_fn = |pid: u16| match provinces.get(pid as usize) {
        Some(pr) if !pr.color.is_empty() => {
            let mut c = hex_color_to_linear(&pr.color);
            c[3] = 0.7;
            c
        }
        _ => [0.0, 0.0, 0.0, 0.0],
    };
    let mut mesh = build_region_mesh(pack, &get_type, &color_fn);
    append_water_gap_raw(&mut mesh, pack, is_water, |p| {
        let pid = pack.cells.province.get(p).copied().unwrap_or(0);
        color_fn(pid)
    });
    let _ = vertices;
    mesh
}
