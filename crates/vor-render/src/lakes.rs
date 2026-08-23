//! Lakes layer (FMG `#lakes`, drawn inside `drawFeatures`,
//! `src/renderers/draw-features.ts:20-87`).
//!
//! Each lake reuses the coastline fractal pipeline (`simplify(0.3)` →
//! `clipPoly(secure)` → `fractalizeCoastline` with the lake smooth threshold →
//! hybrid path). FMG splits lakes into 6 styled subgroups (`default.json:122-163`);
//! the group comes with the imported features (`Feature.lake_group`):
//!
//! | group      | fill      | stroke    | width | opacity |
//! |------------|-----------|-----------|-------|---------|
//! | freshwater | `#a6c1fd` | `#5f799d` | 0.7   | 0.5     |
//! | salt       | `#409b8a` | `#388985` | 0.7   | 0.5     |
//! | sinkhole   | `#5bc9fd` | `#53a3b0` | 0.7   | 1.0     |
//! | frozen     | `#cdd4e7` | —         | 0     | 0.95    |
//! | lava       | `#90270d` | `#f93e0c` | 2.0   | 0.7     |
//! | dry        | `#c9bfa7` | `#8e816f` | 0.7   | 1.0     |
//!
//! The `crumpled` filter of lava lakes is not replicated (SVG filter).

use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, StrokeOptions, StrokeTessellator, VertexBuffers,
};
use vor_core::feature::{FeatureType, LakeGroup};
use vor_core::Pack;

use crate::biome::hex_color_to_linear;
use crate::clip_poly::clip_polygon;
use crate::coastline::fractalize_polygon;
use crate::coastline::FractalSettings;
use crate::coastline_path::{build_coastline_path, coastline_path_to_lyon};
use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};
use crate::simplify::simplify;

/// Style of one FMG lake subgroup, in linear RGBA.
#[derive(Debug, Clone, Copy)]
pub struct LakeStyle {
    pub fill: [f32; 4],
    pub stroke: [f32; 4],
    pub stroke_width: f32,
}

impl LakeStyle {
    pub fn fill_opacity(&self) -> f32 {
        self.fill[3]
    }
}

/// FMG subgroup styles (`default.json:122-163`). `stroke` alpha carries the
/// group opacity too (the `<use>` elements inherit one group opacity; we bake
/// it into both fill and stroke channels).
fn lake_style(group: Option<LakeGroup>) -> LakeStyle {
    let style = |fill: &str, stroke: &str, width: f32, opacity: f32| LakeStyle {
        fill: {
            let mut c = hex_color_to_linear(fill);
            c[3] = opacity;
            c
        },
        stroke: {
            let mut c = hex_color_to_linear(stroke);
            c[3] = opacity;
            c
        },
        stroke_width: width,
    };
    match group {
        Some(LakeGroup::Salt) => style("#409b8a", "#388985", 0.7, 0.5),
        Some(LakeGroup::Sinkhole) => style("#5bc9fd", "#53a3b0", 0.7, 1.0),
        Some(LakeGroup::Frozen) => style("#cdd4e7", "#cdd4e7", 0.0, 0.95),
        Some(LakeGroup::Lava) => style("#90270d", "#f93e0c", 2.0, 0.7),
        Some(LakeGroup::Dry) => style("#c9bfa7", "#8e816f", 0.7, 1.0),
        // freshwater + missing group → freshwater (FMG default subgroup).
        _ => style("#a6c1fd", "#5f799d", 0.7, 0.5),
    }
}

/// Fill + stroke meshes for the whole lake layer (both alpha-blended).
pub struct LakeMeshes {
    pub fill: HeightmapMesh,
    pub stroke: HeightmapMesh,
}

struct StrokeColorCtor([f32; 4]);

impl lyon::tessellation::StrokeVertexConstructor<HeightmapVertex> for StrokeColorCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::StrokeVertex<'_, '_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

/// Builds the lake layer: filled lake polygons plus their shore strokes,
/// applying the same coastline fractal pipeline Azgaar uses in
/// `getFeaturePath()` (`draw-features.ts`): `simplify(0.3)` → `clipPoly(secure=1)`
/// → `fractalizeCoastline(feature.i, feature.type)` → `buildCoastlinePath` → close.
pub fn build_lake_meshes(
    pack: &Pack,
    map_width: f32,
    map_height: f32,
    settings: &FractalSettings,
) -> LakeMeshes {
    let mut fill_mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::INFINITY],
    };
    let mut stroke_mesh = fill_mesh.clone();
    let mut tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();
    let in_bounds =
        |p: &[f32; 2]| p[0] >= 0.0 && p[0] <= map_width && p[1] >= 0.0 && p[1] <= map_height;

    for feature in &pack.features {
        if feature.kind != FeatureType::Lake {
            continue;
        }
        let style = lake_style(feature.lake_group);
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

        // Fill (SVG default fill-rule nonzero).
        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::NonZero);
        if tess
            .tessellate_path(
                &path,
                &opts,
                &mut BuffersBuilder::new(&mut mesh, ColorCtor(style.fill)),
            )
            .is_ok()
        {
            append(&mut fill_mesh, mesh);
        }

        // Shore stroke (frozen lakes have stroke-width 0 → skip).
        if style.stroke_width > 0.0 {
            let mut smesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
            let sopts = StrokeOptions::default()
                .with_line_width(style.stroke_width)
                .with_line_join(lyon::tessellation::LineJoin::Round)
                .with_line_cap(lyon::tessellation::LineCap::Round);
            if stroke_tess
                .tessellate_path(
                    &path,
                    &sopts,
                    &mut BuffersBuilder::new(&mut smesh, StrokeColorCtor(style.stroke)),
                )
                .is_ok()
            {
                append(&mut stroke_mesh, smesh);
            }
        }
    }

    for m in [&mut fill_mesh, &mut stroke_mesh] {
        if !m.bounds_min.iter().all(|v| v.is_finite()) {
            m.bounds_min = [0.0; 2];
            m.bounds_max = [0.0; 2];
        }
    }
    LakeMeshes {
        fill: fill_mesh,
        stroke: stroke_mesh,
    }
}

fn append(target: &mut HeightmapMesh, mesh: VertexBuffers<HeightmapVertex, u32>) {
    let base = target.vertices.len() as u32;
    let start = target.vertices.len();
    target.vertices.extend(mesh.vertices);
    target
        .indices
        .extend(mesh.indices.iter().map(|&i| i + base));
    for v in &target.vertices[start..] {
        target.bounds_min[0] = target.bounds_min[0].min(v.pos[0]);
        target.bounds_min[1] = target.bounds_min[1].min(v.pos[1]);
        target.bounds_max[0] = target.bounds_max[0].max(v.pos[0]);
        target.bounds_max[1] = target.bounds_max[1].max(v.pos[1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subgroup_styles_match_fmg_defaults() {
        let fw = lake_style(None);
        // freshwater #a6c1fd at opacity 0.5: check alpha + hue ordering
        // (linear blue > green > red for that hex).
        assert!((fw.fill_opacity() - 0.5).abs() < 1e-6);
        assert!(fw.fill[2] > fw.fill[1] && fw.fill[1] > fw.fill[0]);
        assert_eq!(fw.stroke_width, 0.7);

        let frozen = lake_style(Some(LakeGroup::Frozen));
        assert_eq!(frozen.stroke_width, 0.0);
        assert!((frozen.fill_opacity() - 0.95).abs() < 1e-6);

        let lava = lake_style(Some(LakeGroup::Lava));
        assert_eq!(lava.stroke_width, 2.0);
        // Lava fill is red-dominant (#90270d).
        assert!(lava.fill[0] > lava.fill[1] && lava.fill[0] > lava.fill[2]);

        let salt = lake_style(Some(LakeGroup::Salt));
        // Teal-ish: green and blue both above red (#409b8a).
        assert!(salt.fill[1] > salt.fill[0] && salt.fill[2] > salt.fill[0]);
    }
}
