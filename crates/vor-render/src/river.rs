use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, StrokeOptions, StrokeTessellator, VertexBuffers};
use vor_core::entities::river::River;

use crate::heightmap::{HeightmapMesh, HeightmapVertex, StrokeCtor};
use crate::mesh::catmull_rom_open;

pub fn build_river_mesh(points: &[[f32; 2]], rivers: &[River]) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };
    let mut tess = StrokeTessellator::new();

    for r in rivers.iter() {
        let path = &r.cell_path;
        if path.len() < 2 {
            continue;
        }
        let raw: Vec<[f32; 2]> = path
            .iter()
            .filter_map(|&ci| points.get(ci as usize).copied())
            .collect();
        if raw.len() < 2 {
            continue;
        }
        let smooth = catmull_rom_open(&raw, 4);

        let mut builder = Path::builder();
        if let Some(first) = smooth.first() {
            builder.begin(point(first[0], first[1]));
            for pt in smooth.iter().skip(1) {
                builder.line_to(point(pt[0], pt[1]));
            }
            builder.end(false);
        }
        let path = builder.build();

        let width = (r.discharge_m3s / 3000.0).clamp(0.8, 5.0);
        let color = [0.15, 0.45, 0.85, 1.0];

        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, StrokeCtor(color));
        let opts = StrokeOptions::default()
            .with_line_width(width)
            .with_line_cap(lyon::tessellation::LineCap::Round)
            .with_line_join(lyon::tessellation::LineJoin::Round);
        if tess.tessellate_path(&path, &opts, &mut buffer_builder).is_err() {
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
