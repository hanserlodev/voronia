use vor_core::entities::burg::Burg;
use vor_core::voronoi::VoronoiVertices;
use vor_core::Pack;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// Half-width of each population bar in map units.
const BAR_HALF_WIDTH: f32 = 0.4;

/// Rural bar color (Azgaar's `#rural` CSS — `#4d4d4d`).
const RURAL_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 0.8];
/// Urban bar color (Azgaar's `#urban` CSS — `#d0240f`).
const URBAN_COLOR: [f32; 4] = [0.82, 0.14, 0.06, 0.9];

fn push_bar(mesh: &mut HeightmapMesh, cx: f32, base_y: f32, height: f32, color: [f32; 4]) {
    if height <= 0.0 {
        return;
    }
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(HeightmapVertex {
        pos: [cx - BAR_HALF_WIDTH, base_y],
        color,
    });
    mesh.vertices.push(HeightmapVertex {
        pos: [cx + BAR_HALF_WIDTH, base_y],
        color,
    });
    mesh.vertices.push(HeightmapVertex {
        pos: [cx - BAR_HALF_WIDTH, base_y - height],
        color,
    });
    mesh.vertices.push(HeightmapVertex {
        pos: [cx + BAR_HALF_WIDTH, base_y - height],
        color,
    });
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    for v in &mesh.vertices[base as usize..][..4] {
        mesh.bounds_min[0] = mesh.bounds_min[0].min(v.pos[0]);
        mesh.bounds_min[1] = mesh.bounds_min[1].min(v.pos[1]);
        mesh.bounds_max[0] = mesh.bounds_max[0].max(v.pos[0]);
        mesh.bounds_max[1] = mesh.bounds_max[1].max(v.pos[1]);
    }
}

/// Builds the population layer the way FMG does: **vertical bars**, not a
/// heatmap. Two sub-layers:
/// - **rural**: one bar per cell with `cells.pop > 0`, height `pop / 5`.
/// - **urban**: one bar per burg (not removed), height `(population/5)*urbanization`.
///
/// (FMG `public/modules/ui/layers.js:drawPopulation`.)
pub fn build_population_bars_mesh(
    vertices: &VoronoiVertices,
    pack: &Pack,
    burgs: &[Burg],
    urbanization: f32,
) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    // Rural bars.
    for p in 0..pack.points_n() {
        let pop = pack.cells.population.get(p).copied().unwrap_or(0.0);
        if pop <= 0.0 {
            continue;
        }
        let [x, y] = pack.points.get(p).copied().unwrap_or([0.0, 0.0]);
        push_bar(&mut mesh, x, y, pop / 5.0, RURAL_COLOR);
    }

    // Urban bars.
    for burg in burgs.iter().filter(|b| b.id != 0 && !b.removed) {
        let [x, y] = burg.position;
        let height = (burg.population / 5.0) * urbanization;
        push_bar(&mut mesh, x, y, height, URBAN_COLOR);
    }

    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0, 0.0];
        mesh.bounds_max = [0.0, 0.0];
    }
    let _ = vertices;
    mesh
}
