//! Ruler / measurers layer (FMG `#ruler`, `src/renderers/draw-measurers.ts`).
//!
//! Renders the measurers persisted in the `.map` (slot `[46]`). FMG draws a
//! **double line**: a solid gray polyline with a white dashed line on top
//! (`DEFAULT_STROKE_WIDTH = 2`, `DEFAULT_DASHARRAY = "10"`), plus the
//! distance text at the midpoint (`rn(length · distanceScale) + unit`).
//! Opisometers curve with `curveCatmullRom.alpha(0.5)`; planimeters fill
//! `lightblue` at 0.5 opacity with stroke `#737373`.
//!
//! The interactive measuring tool itself is out of scope — only persisted
//! measurers render.

use vor_core::entities::measurer::Measurer;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use crate::mesh::catmull_rom_open_alpha;

/// FMG `DEFAULT_STROKE_WIDTH`.
pub const STROKE_WIDTH: f32 = 2.0;
/// FMG `DEFAULT_DASHARRAY`.
pub const DASH: f32 = 10.0;
const GRAY: [f32; 4] = [0.29, 0.33, 0.38, 1.0]; // #4a5a69-ish under-layer
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const PLANIMETER_FILL: [f32; 4] = [0.67, 0.83, 0.98, 0.5]; // lightblue @ .5
const PLANIMETER_STROKE: [f32; 4] = [0.45, 0.45, 0.45, 1.0]; // #737373

/// Distance label at the measurer midpoint (world units).
#[derive(Debug, Clone, PartialEq)]
pub struct RulerLabel {
    pub text: String,
    pub x: f32,
    pub y: f32,
}

/// Double-line meshes + distance labels for all measurers.
pub struct RulerLayer {
    /// Gray solid under-lines (+ planimeter fills).
    pub under: HeightmapMesh,
    /// White dashed over-lines.
    pub over: HeightmapMesh,
    pub labels: Vec<RulerLabel>,
}

/// Builds the ruler layer. `distance_scale` converts map units to km for the
/// label text (FMG `distanceScale`).
pub fn build_ruler_layer(measurers: &[Measurer], distance_scale: f32) -> RulerLayer {
    let mut under = empty();
    let mut over = empty();
    let mut labels = Vec::new();
    let mut tess = lyon::tessellation::StrokeTessellator::new();
    let mut fill_tess = lyon::tessellation::FillTessellator::new();

    for m in measurers {
        if m.points.len() < 2 {
            continue;
        }
        match m.kind.as_str() {
            "Planimeter" => {
                // Closed polygon: lightblue fill + gray outline.
                let pts: Vec<[f32; 2]> = m.points.clone();
                let mut builder = lyon::path::Path::builder();
                builder.begin(lyon::geom::point(pts[0][0], pts[0][1]));
                for p in pts.iter().skip(1) {
                    builder.line_to(lyon::geom::point(p[0], p[1]));
                }
                builder.end(true);
                let path = builder.build();
                let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
                    lyon::tessellation::VertexBuffers::new();
                let opts = lyon::tessellation::FillOptions::default()
                    .with_fill_rule(lyon::tessellation::FillRule::NonZero);
                if fill_tess
                    .tessellate_path(
                        &path,
                        &opts,
                        &mut lyon::tessellation::BuffersBuilder::new(
                            &mut out,
                            FillCtor(PLANIMETER_FILL),
                        ),
                    )
                    .is_ok()
                {
                    append(&mut under, out);
                }
                let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
                    lyon::tessellation::VertexBuffers::new();
                if tess
                    .tessellate_path(
                        &path,
                        &lyon::tessellation::StrokeOptions::default().with_line_width(STROKE_WIDTH),
                        &mut lyon::tessellation::BuffersBuilder::new(
                            &mut out,
                            StrokeCtor(PLANIMETER_STROKE),
                        ),
                    )
                    .is_ok()
                {
                    append(&mut over, out);
                }
            }
            "Opisometer" | "RouteOpisometer" => {
                // Curved with alpha 0.5.
                let curved = catmull_rom_open_alpha(&m.points, 0.5, 6);
                push_double_line(&mut under, &mut over, &mut tess, &curved);
                push_label(&mut labels, &curved, m.length, distance_scale);
            }
            _ => {
                // "Ruler": straight segments.
                push_double_line(&mut under, &mut over, &mut tess, &m.points);
                push_label(&mut labels, &m.points, m.length, distance_scale);
            }
        }
    }

    for mesh in [&mut under, &mut over] {
        if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
            mesh.bounds_min = [0.0; 2];
            mesh.bounds_max = [0.0; 2];
        }
    }
    RulerLayer {
        under,
        over,
        labels,
    }
}

fn push_double_line(
    under: &mut HeightmapMesh,
    over: &mut HeightmapMesh,
    tess: &mut lyon::tessellation::StrokeTessellator,
    pts: &[[f32; 2]],
) {
    // Under: solid gray.
    let mut builder = lyon::path::Path::builder();
    builder.begin(lyon::geom::point(pts[0][0], pts[0][1]));
    for p in pts.iter().skip(1) {
        builder.line_to(lyon::geom::point(p[0], p[1]));
    }
    builder.end(false);
    let path = builder.build();
    let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
        lyon::tessellation::VertexBuffers::new();
    if tess
        .tessellate_path(
            &path,
            &lyon::tessellation::StrokeOptions::default().with_line_width(STROKE_WIDTH),
            &mut lyon::tessellation::BuffersBuilder::new(&mut out, StrokeCtor(GRAY)),
        )
        .is_ok()
    {
        append(under, out);
    }
    // Over: white dashed (dash 10).
    for seg in crate::route_layer::dash_segments_pub(pts, [DASH, DASH]) {
        if seg.len() < 2 {
            continue;
        }
        let mut builder = lyon::path::Path::builder();
        builder.begin(lyon::geom::point(seg[0][0], seg[0][1]));
        for p in seg.iter().skip(1) {
            builder.line_to(lyon::geom::point(p[0], p[1]));
        }
        builder.end(false);
        let dashed = builder.build();
        let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
            lyon::tessellation::VertexBuffers::new();
        if tess
            .tessellate_path(
                &dashed,
                &lyon::tessellation::StrokeOptions::default().with_line_width(STROKE_WIDTH),
                &mut lyon::tessellation::BuffersBuilder::new(&mut out, StrokeCtor(WHITE)),
            )
            .is_ok()
        {
            append(over, out);
        }
    }
}

fn push_label(labels: &mut Vec<RulerLabel>, pts: &[[f32; 2]], length: Option<f32>, ds: f32) {
    // Midpoint along the polyline.
    let total: f32 = pts.windows(2).map(|w| dist(w[0], w[1])).sum();
    let half = total / 2.0;
    let mut acc = 0.0;
    let mut mid = pts[pts.len() / 2];
    for w in pts.windows(2) {
        let d = dist(w[0], w[1]);
        if acc + d >= half && d > 1e-6 {
            let t = (half - acc) / d;
            mid = [
                w[0][0] + (w[1][0] - w[0][0]) * t,
                w[0][1] + (w[1][1] - w[0][1]) * t,
            ];
            break;
        }
        acc += d;
    }
    let km = length.unwrap_or(total) * ds.max(0.01);
    let rounded = (km * 10.0).round() / 10.0;
    let text = if (rounded - rounded.trunc()).abs() < 1e-6 {
        format!("{} km", rounded as i64)
    } else {
        format!("{rounded:.1} km")
    };
    labels.push(RulerLabel {
        text,
        x: mid[0],
        y: mid[1],
    });
}

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
}

struct FillCtor([f32; 4]);

impl lyon::tessellation::FillVertexConstructor<HeightmapVertex> for FillCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::FillVertex<'_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

struct StrokeCtor([f32; 4]);

impl lyon::tessellation::StrokeVertexConstructor<HeightmapVertex> for StrokeCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::StrokeVertex<'_, '_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

fn empty() -> HeightmapMesh {
    HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [0.0; 2],
        bounds_max: [0.0; 2],
    }
}

fn append(
    target: &mut HeightmapMesh,
    out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32>,
) {
    let base = target.vertices.len() as u32;
    let start = target.vertices.len();
    target.vertices.extend(out.vertices);
    target.indices.extend(out.indices.iter().map(|&i| i + base));
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
    use vor_core::entities::measurer::Measurer;

    #[test]
    fn ruler_produces_double_line_and_label() {
        let m = Measurer {
            id: 0,
            name: String::new(),
            kind: "Ruler".into(),
            points: vec![[100.0, 100.0], [500.0, 100.0]],
            length: Some(400.0),
        };
        let layer = build_ruler_layer(std::slice::from_ref(&m), 1.0);
        assert!(!layer.under.vertices.is_empty(), "gray base");
        assert!(!layer.over.vertices.is_empty(), "white dash");
        assert_eq!(layer.labels.len(), 1);
        assert_eq!(layer.labels[0].text, "400 km");
        assert!((layer.labels[0].x - 300.0).abs() < 1e-4);
    }

    #[test]
    fn planimeter_fills() {
        let m = Measurer {
            id: 1,
            name: String::new(),
            kind: "Planimeter".into(),
            points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]],
            length: None,
        };
        let layer = build_ruler_layer(std::slice::from_ref(&m), 1.0);
        assert!(!layer.under.vertices.is_empty());
    }
}
