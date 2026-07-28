use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::feature::{FeatureType, LakeGroup};
use vor_core::Pack;

use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};

fn lake_color(group: Option<LakeGroup>) -> [f32; 4] {
    match group {
        Some(LakeGroup::Freshwater) => [0.65, 0.76, 0.99, 0.5],
        Some(LakeGroup::Salt) => [0.25, 0.61, 0.54, 0.5],
        Some(LakeGroup::Dry) => [0.79, 0.75, 0.65, 1.0],
        Some(LakeGroup::Sinkhole) => [0.36, 0.79, 0.99, 1.0],
        Some(LakeGroup::Lava) => [0.56, 0.15, 0.05, 0.7],
        None => [0.65, 0.76, 0.99, 0.5],
    }
}

pub fn build_lake_mesh(pack: &Pack) -> HeightmapMesh {
    let mut vertices: Vec<HeightmapVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut bounds_min = [f32::INFINITY, f32::INFINITY];
    let mut bounds_max = [f32::NEG_INFINITY, f32::NEG_INFINITY];
    let mut tess = FillTessellator::new();

    for feature in &pack.features {
        if feature.kind != FeatureType::Lake {
            continue;
        }
        let color = lake_color(feature.lake_group);
        let perim = &feature.perimeter_vertices;
        if perim.len() < 3 {
            continue;
        }
        let first_pos = pack
            .vertices
            .positions
            .get(perim[0] as usize)
            .copied()
            .unwrap_or([0.0; 2]);
        let mut builder = Path::builder();
        builder.begin(point(first_pos[0], first_pos[1]));
        for &v in perim.iter().skip(1) {
            let pos = pack
                .vertices
                .positions
                .get(v as usize)
                .copied()
                .unwrap_or([0.0; 2]);
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
