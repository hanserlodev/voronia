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
            // FMG `#sea_island` (public/styles/default.json): opacity 0.5,
            // stroke #1f3846, stroke-width 0.5, filter dropShadow.
            sea_stroke_color: hex_color_to_linear("#1f3846"),
            sea_stroke_width: 0.5,
            sea_opacity: 0.5,
            // FMG `#lake_island`: opacity 1, stroke #7c8eaf, width 0.35.
            lake_stroke_color: hex_color_to_linear("#7c8eaf"),
            lake_stroke_width: 0.35,
            lake_opacity: 1.0,
            // FMG filter #dropShadow: feOffset dx=1 dy=2 (blur not replicated).
            shadow_offset_x: 1.0,
            shadow_offset_y: 2.0,
            shadow_opacity: 0.3,
            shadow_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// FMG auto-filter threshold (`invokeActiveZooming` in public/main.js): the
/// dropShadow is applied while `scale <= 1.5`, removed above it (and a faint
/// blur takes over above 2.6 — not replicated). `scale` is the zoom relative
/// to the initial fit; in Voronia: `fit_extent_y / camera.extent_y`.
pub const SHADOW_MAX_SCALE: f32 = 1.5;

/// Stroke + shadow meshes for the whole map, built in one pass so both share
/// the exact same fractal coastline paths per feature.
pub struct CoastlineMeshes {
    /// All coastline strokes: `#sea_island` + `#lake_island`.
    pub stroke: HeightmapMesh,
    /// Drop-shadow approximation for sea features only (offset hybrid path,
    /// no blur — SVG feGaussianBlur is not replicated).
    pub shadow: HeightmapMesh,
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

/// Accumulates per-feature tessellated stroke geometry into one mesh,
/// optionally offsetting vertices (used by the drop-shadow approximation).
struct MeshAcc {
    mesh: HeightmapMesh,
}

impl MeshAcc {
    fn new() -> Self {
        Self {
            mesh: HeightmapMesh {
                vertices: Vec::new(),
                indices: Vec::new(),
                bounds_min: [f32::INFINITY; 2],
                bounds_max: [f32::NEG_INFINITY; 2],
            },
        }
    }

    fn append(
        &mut self,
        tessellated: lyon::tessellation::VertexBuffers<HeightmapVertex, u32>,
        offset: [f32; 2],
    ) {
        let base = self.mesh.vertices.len() as u32;
        let start = self.mesh.vertices.len();
        self.mesh.vertices.extend(tessellated.vertices);
        self.mesh
            .indices
            .extend(tessellated.indices.iter().map(|i| i + base));
        for v in &mut self.mesh.vertices[start..] {
            v.pos[0] += offset[0];
            v.pos[1] += offset[1];
            self.mesh.bounds_min[0] = self.mesh.bounds_min[0].min(v.pos[0]);
            self.mesh.bounds_min[1] = self.mesh.bounds_min[1].min(v.pos[1]);
            self.mesh.bounds_max[0] = self.mesh.bounds_max[0].max(v.pos[0]);
            self.mesh.bounds_max[1] = self.mesh.bounds_max[1].max(v.pos[1]);
        }
    }

    fn finish(mut self) -> HeightmapMesh {
        if !self.mesh.bounds_min.iter().all(|v| v.is_finite()) {
            self.mesh.bounds_min = [0.0; 2];
            self.mesh.bounds_max = [0.0; 2];
        }
        self.mesh
    }
}

/// Builds the FMG `#coastline` group in a single pass over land features:
/// hybrid-path strokes (`#sea_island` style for ocean features,
/// `#lake_island` for lakes) plus the drop-shadow approximation for sea
/// features (offset copy of the same hybrid path, no blur).
pub fn build_coastline_meshes(
    vertices: &VoronoiVertices,
    features: &[Feature],
    map_width: f32,
    map_height: f32,
    fractal_settings: &FractalSettings,
    stroke_settings: &CoastlineStrokeSettings,
) -> CoastlineMeshes {
    let mut stroke_acc = MeshAcc::new();
    let mut shadow_acc = MeshAcc::new();
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

        // Hybrid coastline path: Q midpoint B-spline on smooth spans,
        // Catmull-Rom on fractalized spans (same geometry as the fill).
        let coastline_path = build_coastline_path(&fractal_pts, &spans);
        let lyon_path = coastline_path_to_lyon(&coastline_path);

        let (stroke_color, stroke_width) = if is_lake {
            (
                [
                    stroke_settings.lake_stroke_color[0],
                    stroke_settings.lake_stroke_color[1],
                    stroke_settings.lake_stroke_color[2],
                    stroke_settings.lake_opacity,
                ],
                stroke_settings.lake_stroke_width,
            )
        } else {
            (
                [
                    stroke_settings.sea_stroke_color[0],
                    stroke_settings.sea_stroke_color[1],
                    stroke_settings.sea_stroke_color[2],
                    stroke_settings.sea_opacity,
                ],
                stroke_settings.sea_stroke_width,
            )
        };

        let mut tess = StrokeTessellator::new();
        let options = StrokeOptions::default()
            .with_line_width(stroke_width)
            .with_line_join(LineJoin::Round)
            .with_line_cap(LineCap::Round);

        let mut stroke_buf: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
            lyon::tessellation::VertexBuffers::new();
        if tess
            .tessellate_path(
                &lyon_path,
                &options,
                &mut BuffersBuilder::new(&mut stroke_buf, StrokeColorCtor(stroke_color)),
            )
            .is_ok()
        {
            stroke_acc.append(stroke_buf, [0.0, 0.0]);
        }

        // Drop shadow: FMG applies filter #dropShadow to #sea_island only
        // (lake_island has filter: null). Offset the tessellated hybrid path;
        // the Gaussian blur is not replicated.
        if !is_lake {
            let shadow_color = [
                stroke_settings.shadow_color[0],
                stroke_settings.shadow_color[1],
                stroke_settings.shadow_color[2],
                stroke_settings.shadow_opacity,
            ];
            let shadow_options = StrokeOptions::default()
                .with_line_width(stroke_settings.sea_stroke_width)
                .with_line_join(LineJoin::Round)
                .with_line_cap(LineCap::Round);
            let mut shadow_buf: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
                lyon::tessellation::VertexBuffers::new();
            if tess
                .tessellate_path(
                    &lyon_path,
                    &shadow_options,
                    &mut BuffersBuilder::new(&mut shadow_buf, StrokeColorCtor(shadow_color)),
                )
                .is_ok()
            {
                shadow_acc.append(
                    shadow_buf,
                    [
                        stroke_settings.shadow_offset_x,
                        stroke_settings.shadow_offset_y,
                    ],
                );
            }
        }
    }

    CoastlineMeshes {
        stroke: stroke_acc.finish(),
        shadow: shadow_acc.finish(),
    }
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
            cell_neighbors: Vec::new(),
            cell_border: Vec::new(),
        };
        let features = vec![make_test_feature()];
        let settings = FractalSettings::default();
        let stroke_settings = CoastlineStrokeSettings::default();
        let meshes = build_coastline_meshes(
            &vertices,
            &features,
            100.0,
            100.0,
            &settings,
            &stroke_settings,
        );
        // A sea feature must produce both a stroke and an offset shadow.
        assert!(
            !meshes.stroke.vertices.is_empty(),
            "stroke should have vertices"
        );
        assert!(
            !meshes.shadow.vertices.is_empty(),
            "sea shadow should have vertices"
        );
        // Shadow is offset by (1, 2) relative to the stroke.
        assert!((meshes.shadow.bounds_min[0] - meshes.stroke.bounds_min[0] - 1.0).abs() < 1e-3);
        assert!((meshes.shadow.bounds_min[1] - meshes.stroke.bounds_min[1] - 2.0).abs() < 1e-3);
    }

    #[test]
    fn lake_feature_has_no_shadow() {
        let vertices = VoronoiVertices {
            positions: vec![[10.0, 10.0], [90.0, 10.0], [90.0, 90.0], [10.0, 90.0]],
            adjacent_cells: Vec::new(),
            adjacent_vertices: Vec::new(),
            cell_rings: Vec::new(),
            cell_neighbors: Vec::new(),
            cell_border: Vec::new(),
        };
        let mut feat = make_test_feature();
        feat.kind = FeatureType::Lake;
        let meshes = build_coastline_meshes(
            &vertices,
            &[feat],
            100.0,
            100.0,
            &FractalSettings::default(),
            &CoastlineStrokeSettings::default(),
        );
        assert!(!meshes.stroke.vertices.is_empty());
        assert!(meshes.shadow.vertices.is_empty());
    }
}
