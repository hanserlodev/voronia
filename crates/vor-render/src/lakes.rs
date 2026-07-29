use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::feature::{FeatureType, LakeGroup};
use vor_core::Pack;

use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};
use crate::mesh::catmull_rom_closed;

fn lake_color(group: Option<LakeGroup>) -> [f32; 4] {
    match group {
        Some(LakeGroup::Freshwater) => [0.25, 0.50, 0.85, 1.0],
        Some(LakeGroup::Salt) => [0.15, 0.55, 0.45, 1.0],
        Some(LakeGroup::Dry) => [0.70, 0.65, 0.50, 1.0],
        Some(LakeGroup::Sinkhole) => [0.10, 0.60, 0.90, 1.0],
        Some(LakeGroup::Lava) => [0.70, 0.25, 0.10, 1.0],
        None => [0.25, 0.50, 0.85, 1.0],
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
        let raw: Vec<[f32; 2]> = feature
            .perimeter_vertices
            .iter()
            .filter_map(|&vi| pack.vertices.positions.get(vi as usize).copied())
            .collect();
        if raw.len() < 3 {
            continue;
        }
        let smooth = catmull_rom_closed(&raw, 5);

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

    if !bounds_min.iter().all(|v| v.is_finite()) {
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
