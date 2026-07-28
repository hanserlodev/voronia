use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::entities::ice::Ice;

use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};

pub fn build_ice_mesh(ice: &[Ice]) -> HeightmapMesh {
    let mut vertices: Vec<HeightmapVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut bounds_min = [f32::INFINITY, f32::INFINITY];
    let mut bounds_max = [f32::NEG_INFINITY, f32::NEG_INFINITY];
    let color = [0.93, 0.95, 0.99, 0.9];
    let mut tess = FillTessellator::new();

    for ice_elem in ice {
        if ice_elem.vertices.len() < 3 {
            continue;
        }
        let first_pos = ice_elem.vertices[0];
        let mut builder = Path::builder();
        builder.begin(point(first_pos[0], first_pos[1]));
        for &v in ice_elem.vertices.iter().skip(1) {
            builder.line_to(point(v[0], v[1]));
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
        let base = vertices.len() as u32;
        vertices.extend_from_slice(&mesh.vertices);
        indices.extend(mesh.indices.iter().map(|i| i + base));
        for v in &mesh.vertices {
            bounds_min[0] = bounds_min[0].min(v.pos[0]);
            bounds_min[1] = bounds_min[1].min(v.pos[1]);
            bounds_max[0] = bounds_max[0].max(v.pos[0]);
            bounds_max[1] = bounds_max[1].max(v.pos[1]);
        }
    }

    if !bounds_min[0].is_finite() {
        bounds_min = [0.0; 2];
        bounds_max = [0.0; 2];
    }
    HeightmapMesh {
        vertices,
        indices,
        bounds_min,
        bounds_max,
    }
}
