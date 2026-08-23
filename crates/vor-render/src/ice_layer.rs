//! Ice layer (FMG `#ice`, `src/renderers/draw-ice.ts`).
//!
//! Glaciers and icebergs are rendered as **raw polygons** (no smoothing —
//! FMG draws them straight from `pack.ice`), all sharing the group style
//! (`default.json:254-260`): `opacity 0.9`, fill `#f1f8fe`, stroke `#e8f0f6`
//! width 0.5, filter `dropShadow01` (offset `.2,.3`, blur `.1` — the blur is
//! not replicated). Iceberg `offset` translations are applied per entity.
//!
//! Three meshes are produced, in FMG paint order: shadow (offset black copy)
//! → fill → stroke. All three are alpha-blended at the group opacity.

use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, LineCap, LineJoin, StrokeOptions,
    StrokeTessellator, StrokeVertexConstructor, VertexBuffers,
};
use vor_core::entities::ice::Ice;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};

/// FMG `#ice` group opacity.
pub const ICE_OPACITY: f32 = 0.9;
/// FMG `#ice` fill `#f1f8fe` (linear, group opacity baked).
fn ice_fill() -> [f32; 4] {
    let mut c = hex_color_to_linear("#f1f8fe");
    c[3] = ICE_OPACITY;
    c
}
/// FMG `#ice` stroke `#e8f0f6` width 0.5.
fn ice_stroke() -> [f32; 4] {
    let mut c = hex_color_to_linear("#e8f0f6");
    c[3] = ICE_OPACITY;
    c
}
/// `dropShadow01`: SourceAlpha (black) offset by (0.2, 0.3).
fn ice_shadow() -> [f32; 4] {
    [0.0, 0.0, 0.0, ICE_OPACITY]
}

pub const SHADOW_OFFSET: [f32; 2] = [0.2, 0.3];
pub const STROKE_WIDTH: f32 = 0.5;

struct StrokeColorCtor([f32; 4]);

impl StrokeVertexConstructor<HeightmapVertex> for StrokeColorCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::StrokeVertex<'_, '_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

/// Shadow + fill + stroke meshes for the whole ice layer.
pub struct IceMeshes {
    pub shadow: HeightmapMesh,
    pub fill: HeightmapMesh,
    pub stroke: HeightmapMesh,
}

/// Builds the three ice meshes (raw polygons, per-entity offset translate).
pub fn build_ice_meshes(ice: &[Ice]) -> IceMeshes {
    let empty = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    let mut shadow_mesh = empty.clone();
    let mut fill_mesh = empty.clone();
    let mut stroke_mesh = empty;
    let mut tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();
    let fill_opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::NonZero);
    let stroke_opts = StrokeOptions::default()
        .with_line_width(STROKE_WIDTH)
        .with_line_join(LineJoin::Round)
        .with_line_cap(LineCap::Round);

    for ice_elem in ice {
        if ice_elem.vertices.len() < 3 {
            continue;
        }
        let [dx, dy] = ice_elem.offset.unwrap_or([0.0, 0.0]);
        let pts: Vec<[f32; 2]> = ice_elem
            .vertices
            .iter()
            .map(|p| [p[0] + dx, p[1] + dy])
            .collect();

        let mut builder = Path::builder();
        builder.begin(point(pts[0][0], pts[0][1]));
        for v in pts.iter().skip(1) {
            builder.line_to(point(v[0], v[1]));
        }
        builder.end(true);
        let path = builder.build();

        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        if tess
            .tessellate_path(
                &path,
                &fill_opts,
                &mut BuffersBuilder::new(&mut mesh, ColorCtor(ice_fill())),
            )
            .is_ok()
        {
            append(&mut fill_mesh, mesh);
        }

        let mut smesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        if stroke_tess
            .tessellate_path(
                &path,
                &stroke_opts,
                &mut BuffersBuilder::new(&mut smesh, StrokeColorCtor(ice_stroke())),
            )
            .is_ok()
        {
            append(&mut stroke_mesh, smesh);
        }

        // dropShadow01 approximation: offset black copy (no blur).
        let mut shmesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        if tess
            .tessellate_path(
                &path,
                &fill_opts,
                &mut BuffersBuilder::new(&mut shmesh, ColorCtor(ice_shadow())),
            )
            .is_ok()
        {
            for v in &mut shmesh.vertices {
                v.pos[0] += SHADOW_OFFSET[0];
                v.pos[1] += SHADOW_OFFSET[1];
            }
            append(&mut shadow_mesh, shmesh);
        }
    }

    for m in [&mut shadow_mesh, &mut fill_mesh, &mut stroke_mesh] {
        if !m.bounds_min.iter().all(|v| v.is_finite()) {
            m.bounds_min = [0.0; 2];
            m.bounds_max = [0.0; 2];
        }
    }
    IceMeshes {
        shadow: shadow_mesh,
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
    use vor_core::entities::ice::IceKind;

    #[test]
    fn ice_meshes_carry_fmg_styles_and_offset() {
        let ice = vec![Ice {
            id: 1,
            kind: IceKind::Iceberg,
            vertices: vec![[10.0, 10.0], [30.0, 10.0], [20.0, 30.0]],
            cell: None,
            offset: Some([5.0, 7.0]),
            size: None,
        }];
        let meshes = build_ice_meshes(&ice);
        assert!(!meshes.fill.vertices.is_empty());
        assert!(!meshes.stroke.vertices.is_empty());
        assert!(!meshes.shadow.vertices.is_empty());
        // Offset applied to fill and shadow (stroke tessellation insets by
        // half the line width, so it only gets a slack check below).
        assert!(
            (meshes.fill.bounds_min[0] - 15.0).abs() < 1e-4,
            "{:?}",
            meshes.fill.bounds_min
        );
        assert!((meshes.fill.bounds_min[1] - 17.0).abs() < 1e-4);
        assert!((meshes.stroke.bounds_min[0] - 15.0).abs() < 0.5);
        // Shadow is additionally offset by (0.2, 0.3).
        assert!((meshes.shadow.bounds_min[0] - meshes.fill.bounds_min[0] - 0.2).abs() < 1e-4);
        // Fill alpha = group opacity 0.9.
        assert!((meshes.fill.vertices[0].color[3] - 0.9).abs() < 1e-6);
    }
}
