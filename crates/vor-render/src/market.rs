//! Markets layer (FMG `draw-markets.ts`).
//!
//! Renders the market areas of influence as isoline fills (one per market,
//! `pack.cells.market`) with a low-opacity fill and a darker border, plus a
//! solid circle at the central burg with the market icon.

use vor_core::entities::burg::Burg;
use vor_core::entities::market::Market;
use vor_core::pack::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::ColorCtor;
use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// Radius of the market center circle.
/// FMG: `r = max(rn(3 + 1/scale, 2), 2)` — 4 at load scale.
const CENTER_RADIUS: f32 = 4.0;
/// Segments of the center circle.
const CENTER_SEGMENTS: u32 = 16;

/// Builds the market **fill** layer: isolines per market id wrapped in
/// `line().curve(curveBasisClosed)` (FMG `draw-markets.ts:30,47`), fill at
/// `fill-opacity: 0.03`.
pub fn build_market_fill_mesh(pack: &Pack, markets: &[Market]) -> HeightmapMesh {
    use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator};
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    let mut tess = FillTessellator::new();
    let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);
    for market in markets {
        let mid = market.id;
        let color = market_color(market, 0.03); // FMG `fill-opacity: 0.03`
        if color[3] == 0.0 {
            continue;
        }
        let get_type =
            |c: usize| -> u16 { u16::from(pack.cells.market.get(c).copied().unwrap_or(0) == mid) };
        let iso_opts = crate::isoline::IsolineOptions {
            polygons: true,
            ..Default::default()
        };
        for iso in crate::isoline::get_isolines(pack, &get_type, &iso_opts) {
            // FMG wraps the ring in `line().curve(curveBasisClosed)`.
            let path = crate::isoline::build_curve_basis_closed(&iso.points, None);
            let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
                lyon::tessellation::VertexBuffers::new();
            if tess
                .tessellate_path(
                    &path,
                    &opts,
                    &mut BuffersBuilder::new(&mut out, ColorCtor(color)),
                )
                .is_ok()
            {
                append_mesh(&mut mesh, out);
            }
        }
    }
    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0; 2];
        mesh.bounds_max = [0.0; 2];
    }
    mesh
}

fn market_color(m: &Market, alpha: f32) -> [f32; 4] {
    if m.color.is_empty() {
        return [0.0; 4];
    }
    let mut c = hex_color_to_linear(&m.color);
    c[3] = alpha;
    c
}

/// Builds the market **border** stroke: `darker(fill)` at width 0.7 and
/// stroke-opacity 0.8 over the same curved rings (FMG clips it to the own
/// fill via clip-path — we stroke the full ring, documented approximation).
pub fn build_market_border_mesh(pack: &Pack, markets: &[Market]) -> HeightmapMesh {
    use lyon::tessellation::{BuffersBuilder, LineCap, LineJoin, StrokeOptions, StrokeTessellator};
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    let mut tess = StrokeTessellator::new();
    let opts = StrokeOptions::default()
        .with_line_width(0.7)
        .with_line_join(LineJoin::Round)
        .with_line_cap(LineCap::Round);
    for market in markets {
        if market.color.is_empty() {
            continue;
        }
        let base_fill = hex_color_to_linear(&market.color);
        // `color.darker()` (d3 default k=1): sRGB channels × 0.7.
        let mut stroke = crate::heightmap::darken(base_fill, 1.0);
        stroke[3] = 0.8; // FMG `stroke-opacity: 0.8`
        let get_type = |c: usize| -> u16 {
            u16::from(pack.cells.market.get(c).copied().unwrap_or(0) == market.id)
        };
        let iso_opts = crate::isoline::IsolineOptions {
            polygons: true,
            ..Default::default()
        };
        for iso in crate::isoline::get_isolines(pack, &get_type, &iso_opts) {
            let path = crate::isoline::build_curve_basis_closed(&iso.points, None);
            let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
                lyon::tessellation::VertexBuffers::new();
            if tess
                .tessellate_path(
                    &path,
                    &opts,
                    &mut BuffersBuilder::new(&mut out, StrokeColorCtor(stroke)),
                )
                .is_ok()
            {
                append_mesh(&mut mesh, out);
            }
        }
    }
    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0; 2];
        mesh.bounds_max = [0.0; 2];
    }
    mesh
}

/// Builds the market **center** mesh: a solid circle at each market's central
/// burg (FMG `draw-markets.ts` circle + icon).
pub fn build_market_center_mesh(markets: &[Market], burgs: &[Burg]) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    for market in markets.iter().filter(|m| m.id != 0) {
        let burg = match burgs.iter().find(|b| b.id == market.center_burg_id) {
            Some(b) => b,
            None => continue,
        };
        let color = if market.color.is_empty() {
            [0.0, 0.0, 0.0, 0.8]
        } else {
            hex_color_to_linear(&market.color)
        };
        let center = burg.position;
        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(HeightmapVertex { pos: center, color });
        for i in 0..CENTER_SEGMENTS {
            let a = i as f32 / CENTER_SEGMENTS as f32 * std::f32::consts::TAU;
            mesh.vertices.push(HeightmapVertex {
                pos: [
                    center[0] + CENTER_RADIUS * a.cos(),
                    center[1] + CENTER_RADIUS * a.sin(),
                ],
                color,
            });
        }
        for i in 0..CENTER_SEGMENTS {
            let v0 = base + 1 + i;
            let v1 = base + 1 + (i + 1) % CENTER_SEGMENTS;
            mesh.indices.extend_from_slice(&[base, v0, v1]);
        }
        for v in &mesh.vertices[base as usize..] {
            mesh.bounds_min[0] = mesh.bounds_min[0].min(v.pos[0]);
            mesh.bounds_min[1] = mesh.bounds_min[1].min(v.pos[1]);
            mesh.bounds_max[0] = mesh.bounds_max[0].max(v.pos[0]);
            mesh.bounds_max[1] = mesh.bounds_max[1].max(v.pos[1]);
        }
    }

    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0, 0.0];
        mesh.bounds_max = [0.0, 0.0];
    }
    mesh
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

fn append_mesh(
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
