//! Zones layer (FMG `#zones`, `drawZone` in `layers.js:978-990`).
//!
//! Each zone is drawn as its fused cell outline (`getVertexPath`) filled with
//! a hatch pattern selected by the zone kind. The patterns are
//! `userSpaceOnUse` tiles of black lines or dots, reproduced here as clipped
//! line/dot geometry at tile scale 1:1 with group opacity 0.6.
//!
//! Pattern mapping: invasion uses hatch1, rebels hatch3, proselytism and
//! crusade hatch6 (dots), disease hatch12, eruption and fault hatch5,
//! avalanche hatch7, flood hatch2, tsunami hatch13.

use vor_core::entities::zone::Zone;
use vor_core::pack::Pack;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// One resolved zone fill: either a hatch pattern or a solid color.
enum ZoneFill {
    Hatch(&'static HatchSpec),
    Solid([f32; 4]),
}

/// Resolves the zone fill from `zone.color` — THE source of truth: FMG stores
/// the chosen pattern there as `url(#hatchN)` (the generator varies it per
/// zone), or a plain hex for custom-drawn zones (solid overlay). The kind is
/// only a lowercase-insensitive fallback when the color carries neither.
fn resolve_zone_fill(color: &str, kind: &str) -> Option<ZoneFill> {
    let color = color.trim();
    if let Some(rest) = color.strip_prefix("url(#hatch") {
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = num.parse::<u8>() {
            return hatch_by_number(n).map(ZoneFill::Hatch);
        }
        return None;
    }
    if color.starts_with('#') && color.len() >= 7 {
        let mut c = crate::biome::hex_color_to_linear(color);
        c[3] = 0.6; // #zones group opacity
        return Some(ZoneFill::Solid(c));
    }
    // Fallback: kind table (FMG generator defaults), case-insensitive.
    hatch_by_kind(kind.to_ascii_lowercase().as_str()).map(ZoneFill::Hatch)
}

const SPACING4: f32 = 4.0;

struct LineFamily {
    /// Direction angle in degrees (0 = +x). FMG applies `patternTransform`
    /// rotate(θ) to a BASE line (vertical = 90° / horizontal = 0°), so the
    /// final direction is base + θ.
    angle_deg: f32,
    /// Distance between adjacent lines (tile size).
    spacing: f32,
}

struct HatchSpec {
    families: &'static [LineFamily],
    width: f32,
    /// Dots pattern (hatch6): dot radius + grid spacing + phase offset.
    dots: Option<(f32, f32, f32)>,
}

/// Pattern geometry from the `hatch*` `<pattern>` defs (`index.html`).
/// Family angles = base line angle + patternTransform rotation.
fn hatch_by_number(n: u8) -> Option<&'static HatchSpec> {
    static F135: [LineFamily; 1] = [LineFamily {
        angle_deg: 135.0,
        spacing: SPACING4,
    }];
    static F45: [LineFamily; 1] = [LineFamily {
        angle_deg: 45.0,
        spacing: SPACING4,
    }];
    static F0: [LineFamily; 1] = [LineFamily {
        angle_deg: 0.0,
        spacing: SPACING4,
    }];
    static GRID45: [LineFamily; 2] = [
        LineFamily {
            angle_deg: 135.0,
            spacing: SPACING4,
        },
        LineFamily {
            angle_deg: 45.0,
            spacing: SPACING4,
        },
    ];
    static H1: HatchSpec = HatchSpec {
        families: &F135,
        width: 2.0,
        dots: None,
    };
    static H2: HatchSpec = HatchSpec {
        families: &F0,
        width: 2.0,
        dots: None,
    };
    static H3: HatchSpec = HatchSpec {
        families: &F45,
        width: 2.0,
        dots: None,
    };
    static H5: HatchSpec = HatchSpec {
        families: &GRID45,
        width: 1.5,
        dots: None,
    };
    static H6: HatchSpec = HatchSpec {
        families: &[],
        width: 0.0,
        dots: Some((1.0, 5.0, 2.5)),
    };
    static H7: HatchSpec = HatchSpec {
        families: &F45,
        width: 1.5,
        dots: None,
    };
    static H12: HatchSpec = HatchSpec {
        families: &GRID45,
        width: 1.5,
        dots: None,
    };
    static H13: HatchSpec = HatchSpec {
        families: &GRID45,
        width: 1.5,
        dots: None,
    };
    match n {
        2 => Some(&H2),
        3 => Some(&H3),
        5 => Some(&H5),
        6 => Some(&H6),
        7 => Some(&H7),
        12 => Some(&H12),
        13 => Some(&H13),
        // hatch1 + any unknown pattern → default diagonal.
        _ => Some(&H1),
    }
}

/// Kind fallback (lowercase), per the Sorvik fixture + generator draw calls.
fn hatch_by_kind(kind: &str) -> Option<&'static HatchSpec> {
    let n = match kind {
        "invasion" => 1,
        "flood" => 13,
        "rebels" => 3,
        "eruption" => 7,
        "fault" => 2,
        "avalanche" | "disaster" => 5,
        "proselytism" | "crusade" => 6,
        "disease" => 12,
        "tsunami" => 13,
        _ => return None,
    };
    hatch_by_number(n)
}

/// Emits the hatch line/dot geometry for one zone polygon.
fn emit_hatch(result: &mut HeightmapMesh, polygon: &[[f32; 2]], spec: &HatchSpec) {
    use lyon::tessellation::{BuffersBuilder, StrokeOptions, StrokeTessellator};
    let black = [0.0, 0.0, 0.0, 0.6]; // #zones group opacity
    let mut stroke_tess = StrokeTessellator::new();

    if let Some((dot_r, spacing, phase)) = spec.dots {
        let min_x = polygon.iter().fold(f32::INFINITY, |m, p| m.min(p[0]));
        let max_x = polygon.iter().fold(f32::NEG_INFINITY, |m, p| m.max(p[0]));
        let min_y = polygon.iter().fold(f32::INFINITY, |m, p| m.min(p[1]));
        let max_y = polygon.iter().fold(f32::NEG_INFINITY, |m, p| m.max(p[1]));
        // FMG dot center sits at (phase, phase) of the tile → world lattice
        // offset by `phase` from the origin-aligned grid.
        let first_x = min_x - ((min_x - phase) % spacing + spacing) % spacing;
        let mut gx = first_x;
        while gx <= max_x {
            let first_y = min_y - ((min_y - phase) % spacing + spacing) % spacing;
            let mut gy = first_y;
            while gy <= max_y {
                if crate::relief::polygon_contains(polygon, [gx, gy]) {
                    push_circle(result, [gx, gy], dot_r, black);
                }
                gy += spacing;
            }
            gx += spacing;
        }
    }

    for family in spec.families {
        let angle = family.angle_deg.to_radians();
        let dir = [angle.cos(), angle.sin()];
        let normal = [-dir[1], dir[0]];
        let mut d_min = f32::INFINITY;
        let mut d_max = f32::NEG_INFINITY;
        for p in polygon {
            let d = p[0] * normal[0] + p[1] * normal[1];
            d_min = d_min.min(d);
            d_max = d_max.max(d);
        }
        if d_max < d_min {
            continue;
        }
        let start = (d_min / family.spacing).floor() * family.spacing;
        let mut offset = start;
        while offset <= d_max {
            let p0 = [normal[0] * offset, normal[1] * offset];
            for span in inside_spans(polygon, p0, dir) {
                let a = [p0[0] + dir[0] * span.0, p0[1] + dir[1] * span.0];
                let b = [p0[0] + dir[0] * span.1, p0[1] + dir[1] * span.1];
                let mut builder = lyon::path::Path::builder();
                builder.begin(lyon::geom::point(a[0], a[1]));
                builder.line_to(lyon::geom::point(b[0], b[1]));
                builder.end(false);
                let path = builder.build();
                let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
                    lyon::tessellation::VertexBuffers::new();
                if stroke_tess
                    .tessellate_path(
                        &path,
                        &StrokeOptions::default().with_line_width(spec.width),
                        &mut BuffersBuilder::new(&mut out, HatchColorCtor(black)),
                    )
                    .is_ok()
                {
                    append_mesh(result, out);
                }
            }
            offset += family.spacing;
        }
    }
}

/// Builds the zone layer: hatch-line geometry per zone (black strokes at
/// group opacity 0.6). Zones with an unknown kind produce no geometry
/// (FMG only defines hatching for its own zone kinds).
pub fn build_zone_hatch_mesh(pack: &Pack, zones: &[Zone]) -> HeightmapMesh {
    use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator};
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    let mut fill_tess = FillTessellator::new();

    for zone in zones {
        if zone.hidden || zone.cells.is_empty() {
            continue;
        }
        let Some(fill) = resolve_zone_fill(&zone.color, &zone.kind) else {
            continue;
        };
        let in_zone = |c: usize| zone.cells.contains(&(c as u32));
        let polygon = vertex_path_polygon(pack, &in_zone);
        if polygon.len() < 3 {
            continue;
        }

        match fill {
            ZoneFill::Solid(color) => {
                // Custom zones: plain hex overlay at group opacity.
                let mut builder = lyon::path::Path::builder();
                builder.begin(lyon::geom::point(polygon[0][0], polygon[0][1]));
                for p in polygon.iter().skip(1) {
                    builder.line_to(lyon::geom::point(p[0], p[1]));
                }
                builder.end(true);
                let path = builder.build();
                let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
                    lyon::tessellation::VertexBuffers::new();
                let opts =
                    FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);
                if fill_tess
                    .tessellate_path(
                        &path,
                        &opts,
                        &mut BuffersBuilder::new(&mut out, HatchColorCtor(color)),
                    )
                    .is_ok()
                {
                    append_mesh(&mut result, out);
                }
            }
            ZoneFill::Hatch(spec) => {
                emit_hatch(&mut result, &polygon, spec);
            }
        }
    }

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0; 2];
        result.bounds_max = [0.0; 2];
    }
    result
}

/// Even-odd inside spans `(t0, t1)` of the line `origin + t·dir` within the
/// polygon (scanline via edge intersections).
fn inside_spans(polygon: &[[f32; 2]], origin: [f32; 2], dir: [f32; 2]) -> Vec<(f32, f32)> {
    let mut ts: Vec<f32> = Vec::new();
    let n = polygon.len();
    for i in 0..n {
        let a = polygon[i];
        let b = polygon[(i + 1) % n];
        // Solve origin + t·dir == a + s·(b−a) for t and s.
        let e = [b[0] - a[0], b[1] - a[1]];
        let denom = dir[0] * e[1] - dir[1] * e[0];
        if denom.abs() < 1e-9 {
            continue;
        }
        let ox = a[0] - origin[0];
        let oy = a[1] - origin[1];
        let t = (ox * e[1] - oy * e[0]) / denom;
        let s = (ox * dir[1] - oy * dir[0]) / denom;
        if (0.0..=1.0).contains(&s) {
            ts.push(t);
        }
    }
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut spans = Vec::new();
    for pair in ts.chunks(2) {
        if pair.len() == 2 && (pair[1] - pair[0]).abs() > 1e-6 {
            spans.push((pair[0], pair[1]));
        }
    }
    spans
}

/// Extracts the fused outer boundary of a set of cells as a closed polygon
/// (same walk as `build_vertex_path_mesh`, without tessellation).
fn vertex_path_polygon(pack: &Pack, in_zone: &impl Fn(usize) -> bool) -> Vec<[f32; 2]> {
    // Reuse the mesh builder's boundary walk by tessellating nothing: instead
    // call it with a degenerate color and take the outline from the isoline
    // engine — simplest correct approach is to reuse connect_vertices through
    // build_vertex_path_mesh's internal chain. To avoid duplicating that walk
    // here we trace one chain with get_isolines semantics directly.
    let n_cells = pack.points_n();
    let vertices = &pack.vertices;
    let mut checked = vec![false; n_cells];
    for cell in 0..n_cells {
        if checked[cell] || !in_zone(cell) {
            continue;
        }
        let same_type = |c: usize| in_zone(c);
        let neighbors = match pack.cells.adjacency.get(cell) {
            Some(v) => v.clone(),
            None => {
                checked[cell] = true;
                continue;
            }
        };
        if !neighbors.iter().any(|&nb| !same_type(nb as usize)) {
            checked[cell] = true;
            continue;
        }
        let ring = match vertices.cell_rings.get(cell) {
            Some(r) if !r.is_empty() => r.clone(),
            _ => continue,
        };
        let start_vertex = match ring.iter().copied().find(|&v| {
            let adj = vertices
                .adjacent_cells
                .get(v as usize)
                .copied()
                .unwrap_or([-1, -1, -1]);
            adj.iter().any(|&c| c >= 0 && !same_type(c as usize))
        }) {
            Some(v) => v,
            None => continue,
        };
        checked[cell] = true;
        let mut check_cell = |c: usize| {
            if c < n_cells && same_type(c) && !checked[c] {
                checked[c] = true;
            }
        };
        let chain = crate::isoline::connect_vertices(
            vertices,
            start_vertex,
            &same_type,
            &mut check_cell,
            true,
        );
        // Flood-fill remaining same-type cells so multi-part zones emit each
        // part on subsequent iterations (approximation: single outline covers
        // the common case).
        let mut stack: Vec<usize> = (0..n_cells)
            .filter(|&c| checked[c] && same_type(c))
            .collect();
        while let Some(c) = stack.pop() {
            if let Some(nbors) = pack.cells.adjacency.get(c) {
                for &nb in nbors {
                    let nb = nb as usize;
                    if nb < n_cells && !checked[nb] && same_type(nb) {
                        checked[nb] = true;
                        stack.push(nb);
                    }
                }
            }
        }
        return chain
            .iter()
            .filter_map(|&v| vertices.positions.get(v as usize).copied())
            .collect();
    }
    Vec::new()
}

struct HatchColorCtor([f32; 4]);

impl lyon::tessellation::FillVertexConstructor<HeightmapVertex> for HatchColorCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::FillVertex<'_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

impl lyon::tessellation::StrokeVertexConstructor<HeightmapVertex> for HatchColorCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::StrokeVertex<'_, '_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

fn push_circle(mesh: &mut HeightmapMesh, center: [f32; 2], radius: f32, color: [f32; 4]) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(HeightmapVertex { pos: center, color });
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        mesh.vertices.push(HeightmapVertex {
            pos: [center[0] + radius * a.cos(), center[1] + radius * a.sin()],
            color,
        });
    }
    for i in 0..8 {
        let v0 = base + 1 + i;
        let v1 = base + 1 + (i + 1) % 8;
        mesh.indices.extend_from_slice(&[base, v0, v1]);
    }
    for v in &mesh.vertices[base as usize..] {
        mesh.bounds_min[0] = mesh.bounds_min[0].min(v.pos[0]);
        mesh.bounds_min[1] = mesh.bounds_min[1].min(v.pos[1]);
        mesh.bounds_max[0] = mesh.bounds_max[0].max(v.pos[0]);
        mesh.bounds_max[1] = mesh.bounds_max[1].max(v.pos[1]);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_is_the_source_of_truth() {
        // url(#hatchN) wins over the kind.
        match resolve_zone_fill("url(#hatch12)", "Rebels") {
            Some(ZoneFill::Hatch(spec)) => {
                assert_eq!(spec.width, 1.5, "hatch12 is w1.5");
                assert_eq!(spec.families.len(), 2);
            }
            _ => panic!("expected hatch"),
        }
        // Plain hex → solid fill at group opacity.
        match resolve_zone_fill("#ff0000", "") {
            Some(ZoneFill::Solid(c)) => assert!((c[3] - 0.6).abs() < 1e-6),
            _ => panic!("expected solid"),
        }
    }

    #[test]
    fn capitalized_kinds_fall_back_correctly() {
        // Sorvik stores Capitalized kinds; the fallback lowercases them.
        match resolve_zone_fill("", "Invasion") {
            Some(ZoneFill::Hatch(spec)) => {
                assert!(
                    (spec.families[0].angle_deg - 135.0).abs() < 1e-6,
                    "hatch1 = vertical rot+45 → 135°"
                );
                assert!((spec.width - 2.0).abs() < 1e-6);
            }
            _ => panic!("expected hatch"),
        }
        match resolve_zone_fill("", "Crusade") {
            Some(ZoneFill::Hatch(spec)) => assert!(spec.dots.is_some()),
            _ => panic!("expected dots"),
        }
        assert!(resolve_zone_fill("", "NotAKind").is_none());
    }

    #[test]
    fn angles_are_base_plus_rotation_not_bare_rotation() {
        // flood → hatch2 (horizontal base 0° + rot 0) — NOT 90°.
        match resolve_zone_fill("url(#hatch2)", "Flood") {
            Some(ZoneFill::Hatch(spec)) => {
                assert!((spec.families[0].angle_deg - 0.0).abs() < 1e-6);
            }
            _ => panic!("expected hatch"),
        }
        // rebels → hatch3 (vertical base 90° + rot −45) → 45°.
        match resolve_zone_fill("url(#hatch3)", "Rebels") {
            Some(ZoneFill::Hatch(spec)) => {
                assert!((spec.families[0].angle_deg - 45.0).abs() < 1e-6);
            }
            _ => panic!("expected hatch"),
        }
    }

    #[test]
    fn inside_spans_on_square() {
        let sq = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let spans = inside_spans(&sq, [-5.0, 5.0], [1.0, 0.0]);
        assert_eq!(spans.len(), 1);
        assert!((spans[0].0 - 5.0).abs() < 1e-5 && (spans[0].1 - 15.0).abs() < 1e-5);
        let spans = inside_spans(&sq, [-5.0, 50.0], [1.0, 0.0]);
        assert!(spans.is_empty());
    }
}
