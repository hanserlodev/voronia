use vor_core::entities::culture::Culture;
use vor_core::voronoi::VoronoiVertices;
use vor_core::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::isoline::build_region_mesh;
use crate::water_gap::append_water_gap_raw;

pub fn build_culture_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    cultures: &[Culture],
    is_water: &[bool],
) -> HeightmapMesh {
    let get_type = |p: usize| pack.cells.culture.get(p).copied().unwrap_or(0);
    let color_fn = |cid: u16| match cultures.get(cid as usize) {
        Some(c) if !c.color.is_empty() => {
            let mut color = hex_color_to_linear(&c.color);
            color[3] = 0.6;
            color
        }
        _ => [0.0, 0.0, 0.0, 0.0],
    };
    let mut mesh = build_region_mesh(pack, &get_type, &color_fn);
    append_water_gap_raw(&mut mesh, pack, is_water, |p| {
        let cid = pack.cells.culture.get(p).copied().unwrap_or(0);
        color_fn(cid)
    });
    let _ = vertices;
    mesh
}
