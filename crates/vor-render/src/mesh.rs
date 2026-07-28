use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::voronoi::VoronoiVertices;

use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};

/// Suaviza la malla de Voronoi moviendo cada vértice hacia el promedio de sus
/// vecinos (Laplacian smoothing). Las celdas se mantienen estancas porque los
/// vértices son compartidos.
///
/// - `factor`: qué tanto moverse hacia el centro de vecinos (0.0 = nada, 0.5 = mitad)
/// - `iterations`: cuántas pasadas de suavizado aplicar
pub fn laplacian_smooth_vertices(
    vertices: &VoronoiVertices,
    factor: f32,
    iterations: usize,
) -> VoronoiVertices {
    let n = vertices.positions.len();
    if n == 0 {
        return vertices.clone();
    }
    let mut smoothed = vertices.positions.clone();
    for _ in 0..iterations {
        let prev = smoothed.clone();
        for v in 0..n {
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            let mut count = 0u32;
            for &nb in &vertices.adjacent_vertices[v] {
                if nb < 0 {
                    continue;
                }
                let p = prev[nb as usize];
                sum_x += p[0];
                sum_y += p[1];
                count += 1;
            }
            if count < 2 {
                continue;
            }
            let avg_x = sum_x / count as f32;
            let avg_y = sum_y / count as f32;
            let p = &prev[v];
            smoothed[v] = [
                p[0] + (avg_x - p[0]) * factor,
                p[1] + (avg_y - p[1]) * factor,
            ];
        }
    }
    let mut result = vertices.clone();
    result.positions = smoothed;
    result
}

/// Construye un `HeightmapMesh` a partir de datos de Voronoi (posiciones + cell_rings)
/// coloreando cada celda según `color_fn(cell_id)`.
/// La malla Voronoi se suaviza con Laplacian smoothing para redondear las celdas
/// sin romper la estanqueidad entre adyacentes.
pub fn build_pack_mesh(
    vertices: &VoronoiVertices,
    points_n: usize,
    color_fn: impl Fn(usize) -> [f32; 4],
) -> HeightmapMesh {
    let smooth_vertices = laplacian_smooth_vertices(vertices, 0.2, 2);

    let mut result = HeightmapMesh {
        vertices: Vec::with_capacity(points_n.saturating_mul(6)),
        indices: Vec::with_capacity(points_n.saturating_mul(9)),
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

        let first_t = ann[0] as usize;
        let first_pos = smooth_vertices
            .positions
            .get(first_t)
            .copied()
            .unwrap_or([0.0, 0.0]);
        let mut builder = Path::builder();
        builder.begin(point(first_pos[0], first_pos[1]));
        for &t in ann.iter().skip(1) {
            let ti = t as usize;
            let pos = smooth_vertices.positions.get(ti).copied().unwrap_or([0.0, 0.0]);
            builder.line_to(point(pos[0], pos[1]));
        }
        builder.end(true);
        let path = builder.build();

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
