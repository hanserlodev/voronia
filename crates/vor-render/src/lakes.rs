use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::feature::{FeatureType, LakeGroup};
use vor_core::Pack;

use crate::clip_poly::clip_polygon;
use crate::coastline::fractalize_polygon;
use crate::coastline::FractalSettings;
use crate::coastline_path::{build_coastline_path, coastline_path_to_lyon};
use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};
use crate::simplify::simplify;

fn lake_color(group: Option<LakeGroup>) -> [f32; 4] {
    match group {
        Some(LakeGroup::Freshwater) => [0.20, 0.45, 0.80, 1.0],
        Some(LakeGroup::Salt) => [0.20, 0.45, 0.80, 1.0],
        Some(LakeGroup::Dry) => [0.70, 0.65, 0.50, 1.0],
        Some(LakeGroup::Sinkhole) => [0.20, 0.45, 0.80, 1.0],
        Some(LakeGroup::Lava) => [0.70, 0.25, 0.10, 1.0],
        None => [0.20, 0.45, 0.80, 1.0],
    }
}

/// Builds the filled lake polygons, applying the same coastline fractal
/// pipeline Azgaar uses in `getFeaturePath()` (draw-features.ts):
/// `simplify(0.3)` → `clipPoly(secure=1)` → `fractalizeCoastline(feature.i,
/// feature.type)` → `buildCoastlinePath` → close → lyon fill.
pub fn build_lake_mesh(
    pack: &Pack,
    map_width: f32,
    map_height: f32,
    settings: &FractalSettings,
) -> HeightmapMesh {
    let mut vertices: Vec<HeightmapVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut bounds_min = [f32::INFINITY, f32::INFINITY];
    let mut bounds_max = [f32::NEG_INFINITY, f32::NEG_INFINITY];
    let mut tess = FillTessellator::new();
    let in_bounds =
        |p: &[f32; 2]| p[0] >= 0.0 && p[0] <= map_width && p[1] >= 0.0 && p[1] <= map_height;

    for feature in &pack.features {
        if feature.kind != FeatureType::Lake {
            continue;
        }
        let color = lake_color(feature.lake_group);
        let raw: Vec<[f32; 2]> = feature
            .perimeter_vertices
            .iter()
            .filter_map(|&vi| pack.vertices.positions.get(vi as usize).copied())
            .filter(in_bounds)
            .collect();
        if raw.len() < 3 {
            continue;
        }
        let simplified = if settings.simplify_tolerance > 0.0 {
            simplify(&raw, settings.simplify_tolerance)
        } else {
            raw
        };
        if simplified.len() < 3 {
            continue;
        }
        let clipped = clip_polygon(&simplified, map_width, map_height, settings.clip_secure);
        if clipped.len() < 3 {
            continue;
        }

        let (fractal_pts, spans) = fractalize_polygon(
            &clipped,
            feature.id as usize,
            true,
            map_width,
            map_height,
            settings,
        );
        let coastline_path = build_coastline_path(&fractal_pts, &spans);
        let path = coastline_path_to_lyon(&coastline_path);

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
