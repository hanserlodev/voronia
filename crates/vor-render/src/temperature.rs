use vor_core::voronoi::VoronoiVertices;
use vor_core::Grid;
use vor_core::Pack;

use crate::heightmap::HeightmapMesh;
use crate::mesh::build_pack_mesh;

fn spectral_color(t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    let stops = [
        (0.00_f32, [0.23_f32, 0.30_f32, 0.67_f32]),
        (0.25_f32, [0.20_f32, 0.63_f32, 0.78_f32]),
        (0.50_f32, [0.96_f32, 0.96_f32, 0.70_f32]),
        (0.75_f32, [0.96_f32, 0.55_f32, 0.26_f32]),
        (1.00_f32, [0.62_f32, 0.04_f32, 0.19_f32]),
    ];
    let mut prev = stops.iter().copied().next().unwrap();
    for &s in &stops[1..] {
        if t <= s.0 {
            let span = (s.0 - prev.0).max(1e-6);
            let frac = (t - prev.0) / span;
            let r = prev.1[0] + (s.1[0] - prev.1[0]) * frac;
            let g = prev.1[1] + (s.1[1] - prev.1[1]) * frac;
            let b = prev.1[2] + (s.1[2] - prev.1[2]) * frac;
            return [r, g, b, 0.85];
        }
        prev = s;
    }
    let last = stops.iter().copied().last().unwrap().1;
    [last[0], last[1], last[2], 0.85]
}

pub fn build_temperature_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    grid: &Grid,
) -> HeightmapMesh {
    build_pack_mesh(vertices, pack.points_n(), |pack_id| {
        let grid_id = pack.cells.grid_id.get(pack_id).copied().unwrap_or(0) as usize;
        let temp = grid.cells.temperature.get(grid_id).copied().unwrap_or(0) as f32;
        let t = ((temp + 50.0) / 100.0).clamp(0.0, 1.0);
        spectral_color(1.0 - t)
    })
}
