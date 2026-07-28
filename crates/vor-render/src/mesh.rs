use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::feature::Feature;
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
            let pos = smooth_vertices
                .positions
                .get(ti)
                .copied()
                .unwrap_or([0.0, 0.0]);
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

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0, 0.0];
        result.bounds_max = [0.0, 0.0];
    }

    result
}

/// Subdivide un polígono cerrado con Catmull-Rom cúbico uniforme (α=0).
/// Cada arista produce `subdivisions` puntos a lo largo de la curva.
/// Usa 3 subdivisiones por defecto para un suavizado tipo Azgaar.
fn catmull_rom_closed(points: &[[f32; 2]], subdivisions: usize) -> Vec<[f32; 2]> {
    let n = points.len();
    if n < 4 || subdivisions == 0 {
        return points.to_vec();
    }
    let mut result = Vec::with_capacity(n * subdivisions);
    for i in 0..n {
        let p0 = points[(i + n - 1) % n];
        let p1 = points[i];
        let p2 = points[(i + 1) % n];
        let p3 = points[(i + 2) % n];
        for j in 0..subdivisions {
            let t = j as f32 / subdivisions as f32;
            let tt = t * t;
            let ttt = tt * t;
            let x = 0.5
                * (2.0 * p1[0]
                    + (-p0[0] + p2[0]) * t
                    + (2.0 * p0[0] - 5.0 * p1[0] + 4.0 * p2[0] - p3[0]) * tt
                    + (-p0[0] + 3.0 * p1[0] - 3.0 * p2[0] + p3[0]) * ttt);
            let y = 0.5
                * (2.0 * p1[1]
                    + (-p0[1] + p2[1]) * t
                    + (2.0 * p0[1] - 5.0 * p1[1] + 4.0 * p2[1] - p3[1]) * tt
                    + (-p0[1] + 3.0 * p1[1] - 3.0 * p2[1] + p3[1]) * ttt);
            result.push([x, y]);
        }
    }
    result
}

/// Construye la malla base del mapa a partir de **features** (continentes/islas),
/// NO del grid de celdas Voronoi. El perímetro de cada feature se suaviza con
/// Catmull-Rom para costas naturales, y toda la masa terrestre se colorea con
/// `color_fn(feature)`.
///
/// El océano no se renderiza acá — se usa el color de fondo (clear color) del
/// render pass.
pub fn build_landmass_mesh(
    vertices: &VoronoiVertices,
    features: &[Feature],
    color_fn: impl Fn(&Feature) -> [f32; 4],
) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let mut tess = FillTessellator::new();

    for feat in features {
        if !feat.is_land || feat.perimeter_vertices.len() < 3 {
            continue;
        }
        let raw: Vec<[f32; 2]> = feat
            .perimeter_vertices
            .iter()
            .filter_map(|&vi| vertices.positions.get(vi as usize).copied())
            .collect();
        if raw.len() < 3 {
            continue;
        }
        let smooth = catmull_rom_closed(&raw, 3);
        let color = color_fn(feat);

        let mut builder = Path::builder();
        if let Some(first) = smooth.first() {
            builder.begin(point(first[0], first[1]));
            for pt in smooth.iter().skip(1) {
                builder.line_to(point(pt[0], pt[1]));
            }
            builder.end(true);
        }
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

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0, 0.0];
        result.bounds_max = [0.0, 0.0];
    }

    result
}
