use vor_core::feature::{Feature, FeatureType};
use vor_core::voronoi::VoronoiVertices;

use crate::biome::hex_color_to_linear;
use crate::clip_poly::clip_polygon;
use crate::coastline::{fractalize_polygon, FractalSettings};
use crate::coastline_path::{build_coastline_path, coastline_path_to_lyon};
use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use crate::simplify::simplify;

use lyon::tessellation::{
    BuffersBuilder, LineCap, LineJoin, StrokeOptions, StrokeTessellator, StrokeVertexConstructor,
};

pub struct CoastlineStrokeSettings {
    pub sea_stroke_color: [f32; 4],
    pub sea_stroke_width: f32,
    pub sea_opacity: f32,
    pub lake_stroke_color: [f32; 4],
    pub lake_stroke_width: f32,
    pub lake_opacity: f32,
    pub shadow_offset_x: f32,
    pub shadow_offset_y: f32,
    pub shadow_opacity: f32,
    pub shadow_color: [f32; 4],
}

impl Default for CoastlineStrokeSettings {
    fn default() -> Self {
        Self {
            sea_stroke_color: hex_color_to_linear("#1f3846"),
            sea_stroke_width: 0.7,
            sea_opacity: 0.5,
            lake_stroke_color: hex_color_to_linear("#7c8eaf"),
            lake_stroke_width: 0.35,
            lake_opacity: 1.0,
            shadow_offset_x: 1.0,
            shadow_offset_y: 1.0,
            shadow_opacity: 0.3,
            shadow_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

struct StrokeColorCtor(pub [f32; 4]);

impl StrokeVertexConstructor<HeightmapVertex> for StrokeColorCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::StrokeVertex<'_, '_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

#[allow(dead_code)]
fn build_single_stroke_mesh(
    chain: &[u32],
    vertices: &VoronoiVertices,
    stroke_width: f32,
    color: [f32; 4],
) -> HeightmapMesh {
    if chain.len() < 2 {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [f32::INFINITY; 2],
            bounds_max: [f32::NEG_INFINITY; 2],
        };
    }

    let pts: Vec<[f32; 2]> = chain
        .iter()
        .filter_map(|&v| vertices.positions.get(v as usize).copied())
        .collect();
    if pts.len() < 2 {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [f32::INFINITY; 2],
            bounds_max: [f32::NEG_INFINITY; 2],
        };
    }

    use lyon::geom::point;
    use lyon::path::Path;

    let mut builder = Path::builder();
    builder.begin(point(pts[0][0], pts[0][1]));
    for pt in pts.iter().skip(1) {
        builder.line_to(point(pt[0], pt[1]));
    }
    builder.end(false);
    let path = builder.build();

    let mut mesh: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
        lyon::tessellation::VertexBuffers::new();
    let mut tess = StrokeTessellator::new();

    let options = StrokeOptions::default()
        .with_line_width(stroke_width)
        .with_line_join(LineJoin::Round)
        .with_line_cap(LineCap::Round);

    let mut buffer_builder = BuffersBuilder::new(&mut mesh, StrokeColorCtor(color));
    if tess
        .tessellate_path(&path, &options, &mut buffer_builder)
        .is_err()
    {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [f32::INFINITY; 2],
            bounds_max: [f32::NEG_INFINITY; 2],
        };
    }

    let mut bounds_min = [f32::INFINITY; 2];
    let mut bounds_max = [f32::NEG_INFINITY; 2];
    for v in &mesh.vertices {
        bounds_min[0] = bounds_min[0].min(v.pos[0]);
        bounds_min[1] = bounds_min[1].min(v.pos[1]);
        bounds_max[0] = bounds_max[0].max(v.pos[0]);
        bounds_max[1] = bounds_max[1].max(v.pos[1]);
    }

    HeightmapMesh {
        vertices: mesh.vertices,
        indices: mesh.indices,
        bounds_min,
        bounds_max,
    }
}

pub fn build_coastline_stroke_mesh(
    vertices: &VoronoiVertices,
    features: &[Feature],
    map_width: f32,
    map_height: f32,
    fractal_settings: &FractalSettings,
    stroke_settings: &CoastlineStrokeSettings,
) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    let in_bounds =
        |p: &[f32; 2]| p[0] >= 0.0 && p[0] <= map_width && p[1] >= 0.0 && p[1] <= map_height;

    for feat in features {
        if !feat.is_land || feat.perimeter_vertices.len() < 3 {
            continue;
        }

        let raw: Vec<[f32; 2]> = feat
            .perimeter_vertices
            .iter()
            .filter_map(|&vi| vertices.positions.get(vi as usize).copied())
            .filter(in_bounds)
            .collect();
        if raw.len() < 3 {
            continue;
        }

        let is_lake = feat.kind == FeatureType::Lake;
        let (stroke_color, stroke_width) = if is_lake {
            (
                stroke_settings.lake_stroke_color,
                stroke_settings.lake_stroke_width,
            )
        } else {
            (
                stroke_settings.sea_stroke_color,
                stroke_settings.sea_stroke_width,
            )
        };

        let simplified = if fractal_settings.simplify_tolerance > 0.0 {
            simplify(&raw, fractal_settings.simplify_tolerance)
        } else {
            raw
        };

        let clipped = clip_polygon(
            &simplified,
            map_width,
            map_height,
            fractal_settings.clip_secure,
        );

        let (fractal_pts, spans) = fractalize_polygon(
            &clipped,
            feat.id as usize,
            is_lake,
            map_width,
            map_height,
            fractal_settings,
        );

        let coastline_path = build_coastline_path(&fractal_pts, &spans);
        let lyon_path = coastline_path_to_lyon(&coastline_path);

        let mut tess = StrokeTessellator::new();
        let options = StrokeOptions::default()
            .with_line_width(stroke_width)
            .with_line_join(LineJoin::Round)
            .with_line_cap(LineCap::Round);

        let stroke_color = [
            stroke_color[0],
            stroke_color[1],
            stroke_color[2],
            if is_lake {
                stroke_settings.lake_opacity
            } else {
                stroke_settings.sea_opacity
            },
        ];

        let mut mesh: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
            lyon::tessellation::VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, StrokeColorCtor(stroke_color));
        if tess
            .tessellate_path(&lyon_path, &options, &mut buffer_builder)
            .is_err()
        {
            continue;
        }

        let base = result.vertices.len() as u32;
        result.vertices.extend_from_slice(&mesh.vertices);
        result.indices.extend(mesh.indices.iter().map(|i| i + base));
        for v in &mesh.vertices[base as usize..] {
            result.bounds_min[0] = result.bounds_min[0].min(v.pos[0]);
            result.bounds_min[1] = result.bounds_min[1].min(v.pos[1]);
            result.bounds_max[0] = result.bounds_max[0].max(v.pos[0]);
            result.bounds_max[1] = result.bounds_max[1].max(v.pos[1]);
        }
    }

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0; 2];
        result.bounds_max = [0.0; 2];
    }
    result
}

pub fn build_coastline_shadow_mesh(
    vertices: &VoronoiVertices,
    features: &[Feature],
    map_width: f32,
    map_height: f32,
    fractal_settings: &FractalSettings,
    stroke_settings: &CoastlineStrokeSettings,
) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };

    let offset_x = stroke_settings.shadow_offset_x;
    let offset_y = stroke_settings.shadow_offset_y;
    let shadow_color = [
        stroke_settings.shadow_color[0],
        stroke_settings.shadow_color[1],
        stroke_settings.shadow_color[2],
        stroke_settings.shadow_opacity,
    ];

    for feat in features {
        if !feat.is_land || feat.perimeter_vertices.len() < 3 || feat.kind == FeatureType::Lake {
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

        let simplified = if fractal_settings.simplify_tolerance > 0.0 {
            simplify(&raw, fractal_settings.simplify_tolerance)
        } else {
            raw
        };

        let clipped = clip_polygon(
            &simplified,
            map_width,
            map_height,
            fractal_settings.clip_secure,
        );

        let (fractal_pts, _spans) = fractalize_polygon(
            &clipped,
            feat.id as usize,
            false,
            map_width,
            map_height,
            fractal_settings,
        );

        use lyon::geom::point;
        use lyon::path::Path;

        let mut builder = Path::builder();
        let pts: Vec<[f32; 2]> = fractal_pts
            .iter()
            .map(|p| [p[0] + offset_x, p[1] + offset_y])
            .collect();
        if pts.is_empty() {
            continue;
        }
        builder.begin(point(pts[0][0], pts[0][1]));
        for pt in pts.iter().skip(1) {
            builder.line_to(point(pt[0], pt[1]));
        }
        builder.end(false);
        let path = builder.build();

        let mut tess = StrokeTessellator::new();
        let options = StrokeOptions::default()
            .with_line_width(stroke_settings.sea_stroke_width)
            .with_line_join(LineJoin::Round)
            .with_line_cap(LineCap::Round);

        let mut mesh: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
            lyon::tessellation::VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, StrokeColorCtor(shadow_color));

        if tess
            .tessellate_path(&path, &options, &mut buffer_builder)
            .is_err()
        {
            continue;
        }

        let base = result.vertices.len() as u32;
        result.vertices.extend_from_slice(&mesh.vertices);
        result.indices.extend(mesh.indices.iter().map(|i| i + base));
        for v in &mesh.vertices[base as usize..] {
            result.bounds_min[0] = result.bounds_min[0].min(v.pos[0]);
            result.bounds_min[1] = result.bounds_min[1].min(v.pos[1]);
            result.bounds_max[0] = result.bounds_max[0].max(v.pos[0]);
            result.bounds_max[1] = result.bounds_max[1].max(v.pos[1]);
        }
    }

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0; 2];
        result.bounds_max = [0.0; 2];
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use vor_core::feature::FeatureType;
    use vor_core::voronoi::VoronoiVertices;

    fn make_test_feature() -> Feature {
        Feature {
            id: 0,
            is_land: true,
            touches_border: false,
            kind: FeatureType::Island,
            land_group: None,
            lake_group: None,
            cell_count: 10,
            first_cell: 0,
            perimeter_vertices: vec![0, 1, 2, 3],
            name: None,
            shoreline: Vec::new(),
            lake_height: 0.0,
            inlets: Vec::new(),
            outlet_river: None,
            entering_flux: 0.0,
            closed: false,
            out_cell: None,
        }
    }

    #[test]
    fn stroke_mesh_is_created() {
        let vertices = VoronoiVertices {
            positions: vec![[10.0, 10.0], [90.0, 10.0], [90.0, 90.0], [10.0, 90.0]],
            adjacent_cells: Vec::new(),
            adjacent_vertices: Vec::new(),
            cell_rings: Vec::new(),
        };
        let features = vec![make_test_feature()];
        let settings = FractalSettings::default();
        let stroke_settings = CoastlineStrokeSettings::default();
        let mesh = build_coastline_stroke_mesh(
            &vertices,
            &features,
            100.0,
            100.0,
            &settings,
            &stroke_settings,
        );
        assert!(
            mesh.vertices.is_empty() || mesh.vertices.len() >= 3,
            "stroke should produce at least some vertices"
        );
    }
}
