use vor_core::voronoi::VoronoiVertices;
use vor_core::Grid;
use vor_core::Pack;

use crate::heightmap::HeightmapMesh;
use crate::mesh::build_pack_mesh;

fn precipitation_color(prec: f32, max_prec: f32) -> [f32; 4] {
    let intensity = if max_prec > 0.0 {
        (prec / max_prec).min(1.0)
    } else {
        0.0
    };
    let r = 0.05 + 0.1 * intensity;
    let g = 0.15 + 0.4 * intensity;
    let b = 0.50 + 0.5 * intensity;
    [r, g, b, 0.75]
}

pub fn build_precipitation_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    grid: &Grid,
) -> HeightmapMesh {
    let max_prec = grid.cells.precipitation.iter().copied().max().unwrap_or(0) as f32;

    build_pack_mesh(vertices, pack.points_n(), |pack_id| {
        let grid_id = pack.cells.grid_id.get(pack_id).copied().unwrap_or(0) as usize;
        let prec = grid.cells.precipitation.get(grid_id).copied().unwrap_or(0) as f32;
        precipitation_color(prec, max_prec)
    })
}
