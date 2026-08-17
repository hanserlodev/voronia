//! Precipitation layer: colored circles over wet land cells, ported from
//! Azgaar's `drawPrecipitation` (`public/modules/ui/layers.js:333`).
//!
//! Azgaar draws one `<circle>` per grid cell with `height >= 20` (land) and
//! `precipitation > 0`, centered on the cell center `grid.points[d]`, with radius
//! `rn(sqrt(prec/4) / cellsNumberModifier, 2)` where `cellsNumberModifier =
//! (cells/10000)^0.25`. The circle color is the CSS `#003dff` (blue).

use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::Grid;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{ColorCtor, HeightmapMesh};

/// Azgaar `rn(n, 2)` — round to 2 decimals.
fn rn(x: f32) -> f32 {
    (x * 100.0).round() / 100.0
}

/// Tessellated circle outline around a cell center (concentric polygon).
fn push_circle(
    mesh: &mut HeightmapMesh,
    tess: &mut FillTessellator,
    opts: &FillOptions,
    cx: f32,
    cy: f32,
    r: f32,
    color: [f32; 4],
) {
    const SEGMENTS: u32 = 24;
    if r <= 0.0 {
        return;
    }
    let mut builder = Path::builder();
    let step = std::f32::consts::TAU / SEGMENTS as f32;
    let first = point(cx + r, cy);
    builder.begin(first);
    for i in 1..SEGMENTS {
        let (s, c) = (step * i as f32).sin_cos();
        builder.line_to(point(cx + r * c, cy + r * s));
    }
    builder.close();
    let path = builder.build();

    let mut verts: VertexBuffers<crate::heightmap::HeightmapVertex, u32> = VertexBuffers::new();
    let mut buffer_builder = BuffersBuilder::new(&mut verts, ColorCtor(color));
    if tess
        .tessellate_path(&path, opts, &mut buffer_builder)
        .is_err()
    {
        return;
    }
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&verts.vertices);
    mesh.indices.extend(verts.indices.iter().map(|i| i + base));
}

/// Builds the precipitation mesh (Azgaar `drawPrecipitation`).
pub fn build_precipitation_mesh(grid: &Grid) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [0.0, 0.0],
        bounds_max: [grid.width, grid.height],
    };

    let n = grid
        .points_n()
        .min(grid.cells.height.len())
        .min(grid.cells.precipitation.len());
    if n == 0 {
        return mesh;
    }

    let cells_number_modifier = (grid.cells_desired as f32 / 10_000.0).powf(0.25);
    let color = hex_color_to_linear("#003dff");
    let mut tess = FillTessellator::new();
    let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);

    for cell in 0..n {
        let h = grid.cells.height[cell];
        let prec = grid.cells.precipitation[cell];
        if h < 20 || prec == 0 {
            continue;
        }
        let r = rn(f32::sqrt(prec as f32 / 4.0) / cells_number_modifier);
        let p = grid.points[cell];
        push_circle(&mut mesh, &mut tess, &opts, p[0], p[1], r, color);
    }

    mesh
}
