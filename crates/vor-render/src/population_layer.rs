use vor_core::Pack;
use vor_core::voronoi::VoronoiVertices;

use crate::heightmap::HeightmapMesh;
use crate::mesh::build_pack_mesh;

fn pop_color(pop: f32, max_pop: f32) -> [f32; 4] {
    if pop <= 0.0 || max_pop <= 0.0 {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let t = (pop / max_pop).sqrt().min(1.0);
    let r = 0.1 + 0.9 * t;
    let g = 0.9 * (1.0 - t * 0.8);
    let b = 0.4 * (1.0 - t).max(0.0);
    [r, g, b, 0.6 + 0.4 * t]
}

pub fn build_population_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
) -> HeightmapMesh {
    let max_pop = pack
        .cells
        .population
        .iter()
        .copied()
        .fold(0.0_f32, f32::max);

    build_pack_mesh(vertices, pack.points_n(), |p| {
        let pop = pack.cells.population.get(p).copied().unwrap_or(0.0);
        pop_color(pop, max_pop)
    })
}
