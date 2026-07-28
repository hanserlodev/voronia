use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::voronoi::VoronoiVertices;

use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};

fn chaikin_smooth(poly: &[[f32; 2]], iterations: usize) -> Vec<[f32; 2]> {
    if poly.len() < 3 || iterations == 0 {
        return poly.to_vec();
    }
    let mut result = poly.to_vec();
    for _ in 0..iterations {
        let n = result.len();
        let mut smoothed = Vec::with_capacity(n * 2);
        for i in 0..n {
            let a = result[i];
            let b = result[(i + 1) % n];
            smoothed.push([a[0] * 0.75 + b[0] * 0.25, a[1] * 0.75 + b[1] * 0.25]);
            smoothed.push([a[0] * 0.25 + b[0] * 0.75, a[1] * 0.25 + b[1] * 0.75]);
        }
        result = smoothed;
    }
    result
}

fn polygon_from_cell_ring(vertices: &VoronoiVertices, ann: &[u32]) -> Vec<[f32; 2]> {
    ann.iter()
        .map(|&t| {
            vertices
                .positions
                .get(t as usize)
                .copied()
                .unwrap_or([0.0, 0.0])
        })
        .collect()
}

fn build_path_from_poly(poly: &[[f32; 2]]) -> Path {
    let mut builder = Path::builder();
    if let Some(first) = poly.first() {
        builder.begin(point(first[0], first[1]));
        for pt in poly.iter().skip(1) {
            builder.line_to(point(pt[0], pt[1]));
        }
        builder.end(true);
    }
    builder.build()
}

/// Construye un `HeightmapMesh` a partir de datos de Voronoi (posiciones + cell_rings)
/// coloreando cada celda según `color_fn(cell_id)`.
/// Los polígonos de celda se suavizan con Chaikin corner cutting (1 iteración)
/// para eliminar la apariencia puntiaguda de los raw Voronoi.
pub fn build_pack_mesh(
    vertices: &VoronoiVertices,
    points_n: usize,
    color_fn: impl Fn(usize) -> [f32; 4],
) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::with_capacity(points_n.saturating_mul(12)),
        indices: Vec::with_capacity(points_n.saturating_mul(18)),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let mut tess = FillTessellator::new();

    for p in 0..points_n {
        let ann = match vertices.cell_rings.get(p) {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        let color = color_fn(p);

        let raw_poly = polygon_from_cell_ring(vertices, ann);
        let smoothed = chaikin_smooth(&raw_poly, 1);
        let path = build_path_from_poly(&smoothed);

        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, ColorCtor(color));
        let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);

        if tess
            .tessellate_path(&path, &opts, &mut buffer_builder)
            .is_err()
        {
            continue;
        }

        let base = result.vertices.len() as u32;
        result.vertices.extend_from_slice(&mesh.vertices);
        result.indices.extend(mesh.indices.iter().map(|i| i + base));

        for v in &mesh.vertices {
            result.bounds_min[0] = result.bounds_min[0].min(v.pos[0]);
            result.bounds_min[1] = result.bounds_min[1].min(v.pos[1]);
            result.bounds_max[0] = result.bounds_max[0].max(v.pos[0]);
            result.bounds_max[1] = result.bounds_max[1].max(v.pos[1]);
        }
    }

    if !result.bounds_min[0].is_finite() {
        result.bounds_min = [0.0, 0.0];
        result.bounds_max = [0.0, 0.0];
    }

    result
}
