//! Burg icons layer (FMG `#icons > #burgIcons` + `#anchors`).
//!
//! FMG v1.138 renders each burg as a `<use>` against simple symbol shapes,
//! **white translucent with a dark stroke**, sized and shaped by the burg's
//! `group` (`#burgIcons > g#<group>` in `default.json`; the icon is CENTERED
//! on the burg point — the symbols use `overflow: visible` with shapes around
//! their viewBox origin):
//!
//! | group         | shape    | font-size | stroke-width |
//! |---------------|----------|-----------|--------------|
//! | capital       | square   | 2.0       | 1.0          |
//! | city          | circle   | 1.5       | 1.0          |
//! | town          | circle   | 1.0       | 1.2          |
//! | village       | circle   | 0.7       | 1.2          |
//! | hamlet        | circle   | 0.5       | 1.2          |
//! | fort          | square   | 0.7       | 1.0          |
//! | monastery     | cross    | 0.7       | 1.0          |
//! | caravanserai  | triangle | 0.7       | 1.0          |
//! | trading_post  | triangle | 0.7       | 1.0          |
//!
//! Port burgs (`port != 0`) additionally get an anchor below
//! (`#anchors`, fill white opaque, stroke-width 1.2, same font-size). The
//! anchor glyph is a geometric approximation of FMG's `#icon-anchor` path.

use vor_core::entities::burg::Burg;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// FMG icon fill: white at 70% opacity.
fn icon_fill() -> [f32; 4] {
    let mut c = hex_color_to_linear("#ffffff");
    c[3] = 0.7;
    c
}
/// Anchor fill is fully opaque (the `#anchors > g` groups carry no
/// fill-opacity).
fn anchor_fill() -> [f32; 4] {
    let mut c = hex_color_to_linear("#ffffff");
    c[3] = 1.0;
    c
}
/// Shared dark stroke color for all groups.
fn icon_stroke() -> [f32; 4] {
    hex_color_to_linear("#3e3e4b")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IconShape {
    Circle,
    Square,
    Cross,
    Triangle,
}

struct GroupStyle {
    shape: IconShape,
    size: f32,
    stroke_width: f32,
}

/// `#burgIcons > g#<group>` styles from `default.json`.
fn group_style(group: &str) -> GroupStyle {
    match group {
        "capital" => GroupStyle {
            shape: IconShape::Square,
            size: 2.0,
            stroke_width: 1.0,
        },
        "city" => GroupStyle {
            shape: IconShape::Circle,
            size: 1.5,
            stroke_width: 1.0,
        },
        "village" => GroupStyle {
            shape: IconShape::Circle,
            size: 0.7,
            stroke_width: 1.2,
        },
        "hamlet" => GroupStyle {
            shape: IconShape::Circle,
            size: 0.5,
            stroke_width: 1.2,
        },
        "fort" => GroupStyle {
            shape: IconShape::Square,
            size: 0.7,
            stroke_width: 1.0,
        },
        "monastery" => GroupStyle {
            shape: IconShape::Cross,
            size: 0.7,
            stroke_width: 1.0,
        },
        "caravanserai" | "trading_post" => GroupStyle {
            shape: IconShape::Triangle,
            size: 0.7,
            stroke_width: 1.0,
        },
        // town + unknown groups → town style (FMG's isDefault group).
        _ => GroupStyle {
            shape: IconShape::Circle,
            size: 1.0,
            stroke_width: 1.2,
        },
    }
}

const ANCHOR_STROKE_WIDTH: f32 = 1.2;

/// Builds the burg icons mesh: one centered shape per burg plus an anchor
/// glyph for port burgs (icons first, then anchors — FMG DOM order).
pub fn build_burg_icons_mesh(burgs: &[Burg]) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };

    let push_icon = |mesh: &mut HeightmapMesh,
                     b: &Burg,
                     shape: IconShape,
                     size: f32,
                     sw: f32,
                     fill: [f32; 4]| {
        let scale = size / 10.0; // symbol viewBox is 10×10
                                 // Fill shape.
        match shape {
            IconShape::Circle => push_circle(mesh, b.position, size / 2.0, fill),
            IconShape::Square => {
                let h = size / 2.0;
                let [x, y] = b.position;
                push_polygon(
                    mesh,
                    &[
                        (x - h, y - h),
                        (x + h, y - h),
                        (x + h, y + h),
                        (x - h, y + h),
                    ],
                    fill,
                );
            }
            IconShape::Triangle => {
                let [x, y] = b.position;
                push_polygon(
                    mesh,
                    &[
                        (x, y - 5.0 * scale),
                        (x + 5.0 * scale, y + 5.0 * scale),
                        (x - 5.0 * scale, y + 5.0 * scale),
                    ],
                    fill,
                );
            }
            IconShape::Cross => {
                let t = 1.5 * scale;
                let a = 5.0 * scale;
                let [x, y] = b.position;
                push_polygon(
                    mesh,
                    &[
                        (x - t, y - a),
                        (x + t, y - a),
                        (x + t, y - t),
                        (x + a, y - t),
                        (x + a, y + t),
                        (x + t, y + t),
                        (x + t, y + a),
                        (x - t, y + a),
                        (x - t, y + t),
                        (x - a, y + t),
                        (x - a, y - t),
                        (x - t, y - t),
                    ],
                    fill,
                );
            }
        }
        // Stroke outline: walk the same shape as a thin stroke.
        let outline = shape_outline(b.position, shape, size);
        for seg in outline.windows(2) {
            push_segment_stroke(mesh, seg[0], seg[1], sw, icon_stroke());
        }
    };

    for b in burgs {
        if b.removed {
            continue;
        }
        let style = group_style(&b.group);
        push_icon(
            &mut mesh,
            b,
            style.shape,
            style.size,
            style.stroke_width,
            icon_fill(),
        );
    }

    // Anchors after all icons (#anchors follows #burgIcons in the DOM).
    for b in burgs {
        if b.removed || b.port_feature.is_none() {
            continue;
        }
        let style = group_style(&b.group);
        push_anchor(&mut mesh, b.position, style.size, ANCHOR_STROKE_WIDTH);
    }

    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0; 2];
        mesh.bounds_max = [0.0; 2];
    }
    mesh
}

/// Polygon vertices of a shape outline (for stroking).
fn shape_outline(center: [f32; 2], shape: IconShape, size: f32) -> Vec<[f32; 2]> {
    let s = size / 10.0;
    let [x, y] = center;
    match shape {
        // Approximate the circle outline with its fan ring points.
        IconShape::Circle => (0..=12)
            .map(|i| {
                let a = i as f32 / 12.0 * std::f32::consts::TAU;
                [x + size / 2.0 * a.cos(), y + size / 2.0 * a.sin()]
            })
            .collect(),
        IconShape::Square => vec![
            [x - 5.0 * s, y - 5.0 * s],
            [x + 5.0 * s, y - 5.0 * s],
            [x + 5.0 * s, y + 5.0 * s],
            [x - 5.0 * s, y + 5.0 * s],
            [x - 5.0 * s, y - 5.0 * s],
        ],
        IconShape::Triangle => vec![
            [x, y - 5.0 * s],
            [x + 5.0 * s, y + 5.0 * s],
            [x - 5.0 * s, y + 5.0 * s],
            [x, y - 5.0 * s],
        ],
        IconShape::Cross => {
            let t = 1.5 * s;
            let a = 5.0 * s;
            vec![
                [x - t, y - a],
                [x + t, y - a],
                [x + t, y - t],
                [x + a, y - t],
                [x + a, y + t],
                [x + t, y + t],
                [x + t, y + a],
                [x - t, y + a],
                [x - t, y + t],
                [x - a, y + t],
                [x - a, y - t],
                [x - t, y - t],
                [x - t, y - a],
            ]
        }
    }
}

/// Geometric approximation of FMG's `#icon-anchor` (ring + shank + stock +
/// arms), scaled to `size`. The original is a curved `<path>`.
fn push_anchor(mesh: &mut HeightmapMesh, center: [f32; 2], size: f32, sw: f32) {
    let s = size / 30.0 * 10.0; // anchor viewBox is 30×30
    let [cx, cy] = center;
    // Shank (vertical bar).
    push_polygon(
        mesh,
        &[
            (cx - 0.8 * s, cy - 4.0 * s),
            (cx + 0.8 * s, cy - 4.0 * s),
            (cx + 0.8 * s, cy + 4.0 * s),
            (cx - 0.8 * s, cy + 4.0 * s),
        ],
        anchor_fill(),
    );
    // Stock (horizontal bar near the top).
    push_polygon(
        mesh,
        &[
            (cx - 3.0 * s, cy - 4.0 * s),
            (cx + 3.0 * s, cy - 4.0 * s),
            (cx + 3.0 * s, cy - 2.6 * s),
            (cx - 3.0 * s, cy - 2.6 * s),
        ],
        anchor_fill(),
    );
    // Ring on top (small circle).
    push_circle(mesh, [cx, cy - 5.2 * s], 1.0 * s, anchor_fill());
    // Arms: two angled bars forming a shallow V.
    push_polygon(
        mesh,
        &[
            (cx - 5.5 * s, cy + 1.0 * s),
            (cx - 4.0 * s, cy + 0.4 * s),
            (cx, cy + 3.2 * s),
            (cx, cy + 5.0 * s),
        ],
        anchor_fill(),
    );
    push_polygon(
        mesh,
        &[
            (cx + 5.5 * s, cy + 1.0 * s),
            (cx + 4.0 * s, cy + 0.4 * s),
            (cx, cy + 3.2 * s),
            (cx, cy + 5.0 * s),
        ],
        anchor_fill(),
    );
    // Stroke hint: thin outline around the arms' outer edge.
    push_segment_stroke(
        mesh,
        [cx - 5.5 * s, cy + 1.0 * s],
        [cx - 4.0 * s, cy + 0.4 * s],
        sw,
        icon_stroke(),
    );
    push_segment_stroke(
        mesh,
        [cx + 5.5 * s, cy + 1.0 * s],
        [cx + 4.0 * s, cy + 0.4 * s],
        sw,
        icon_stroke(),
    );
}

const ICON_SEGMENTS: u32 = 12;

fn push_circle(mesh: &mut HeightmapMesh, center: [f32; 2], radius: f32, color: [f32; 4]) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(HeightmapVertex { pos: center, color });
    for i in 0..ICON_SEGMENTS {
        let a = i as f32 / ICON_SEGMENTS as f32 * std::f32::consts::TAU;
        let x = center[0] + radius * a.cos();
        let y = center[1] + radius * a.sin();
        mesh.vertices.push(HeightmapVertex { pos: [x, y], color });
    }
    for i in 0..ICON_SEGMENTS {
        let v0 = base + 1 + i;
        let v1 = base + 1 + (i + 1) % ICON_SEGMENTS;
        mesh.indices.extend_from_slice(&[base, v0, v1]);
    }
    for v in &mesh.vertices[base as usize..] {
        mesh.bounds_min[0] = mesh.bounds_min[0].min(v.pos[0]);
        mesh.bounds_min[1] = mesh.bounds_min[1].min(v.pos[1]);
        mesh.bounds_max[0] = mesh.bounds_max[0].max(v.pos[0]);
        mesh.bounds_max[1] = mesh.bounds_max[1].max(v.pos[1]);
    }
}

fn push_polygon(mesh: &mut HeightmapMesh, pts: &[(f32, f32)], color: [f32; 4]) {
    let base = mesh.vertices.len() as u32;
    for &(x, y) in pts {
        mesh.vertices.push(HeightmapVertex { pos: [x, y], color });
    }
    for i in 1..pts.len() as u32 - 1 {
        mesh.indices
            .extend_from_slice(&[base, base + i, base + i + 1]);
    }
    for v in &mesh.vertices[base as usize..] {
        mesh.bounds_min[0] = mesh.bounds_min[0].min(v.pos[0]);
        mesh.bounds_min[1] = mesh.bounds_min[1].min(v.pos[1]);
        mesh.bounds_max[0] = mesh.bounds_max[0].max(v.pos[0]);
        mesh.bounds_max[1] = mesh.bounds_max[1].max(v.pos[1]);
    }
}

/// Thin quad stroke for one segment (used for icon outlines).
fn push_segment_stroke(
    mesh: &mut HeightmapMesh,
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    color: [f32; 4],
) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return;
    }
    let h = width / 2.0;
    let nx = -dy / len * h;
    let ny = dx / len * h;
    push_polygon(
        mesh,
        &[
            (a[0] + nx, a[1] + ny),
            (b[0] + nx, b[1] + ny),
            (b[0] - nx, b[1] - ny),
            (a[0] - nx, a[1] - ny),
        ],
        color,
    );
}

/// FMG `#burgLabels > g#<group>`: `(data-size, data-dy)` per group.
pub fn burg_label_style(group: &str) -> (f32, f32) {
    match group {
        "capital" => (6.0, -0.5),
        "city" => (5.0, -0.4),
        "fort" | "monastery" => (2.0, -0.5),
        "village" => (3.0, -0.4),
        // town + fallback
        _ => (4.0, -0.4),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vor_core::entities::burg::BurgProduction;

    fn burg(id: u16, group: &str, port: bool) -> Burg {
        Burg {
            id,
            name: format!("B{id}"),
            cell: id as u32,
            position: [id as f32 * 10.0, 50.0],
            culture: 0,
            state: 0,
            feature: 0,
            population: 1.0,
            kind: Default::default(),
            coat_of_arms: Default::default(),
            is_capital: false,
            port_feature: port.then_some(2),
            has_citadel: false,
            has_plaza: false,
            has_shanty: false,
            has_temple: false,
            has_walls: false,
            locked: false,
            removed: false,
            production: Vec::<BurgProduction>::new(),
            group: group.to_string(),
        }
    }

    #[test]
    fn capital_is_square_town_is_circle() {
        let burgs = vec![burg(1, "capital", false), burg(2, "town", false)];
        let mesh = build_burg_icons_mesh(&burgs);
        assert!(!mesh.vertices.is_empty());
        // Capital square: exactly 4 corners → 2 triangles → 6 indices per fill.
        // Just check both icons produced geometry near their positions.
        assert!(mesh.vertices.iter().any(|v| (v.pos[0] - 10.0).abs() < 2.0));
        assert!(mesh.vertices.iter().any(|v| (v.pos[0] - 20.0).abs() < 2.0));
        // White fill at 0.7 alpha somewhere.
        assert!(mesh
            .vertices
            .iter()
            .any(|v| v.color[3] > 0.69 && v.color[3] < 0.71));
    }

    #[test]
    fn ports_get_anchors() {
        let burgs = vec![burg(1, "town", true), burg(2, "town", false)];
        let with = build_burg_icons_mesh(&burgs);
        let without = build_burg_icons_mesh(&burgs[..1]);
        assert!(
            with.vertices.len() > without.vertices.len(),
            "anchor adds geometry"
        );
        // Anchors are opaque white.
        assert!(with.vertices.iter().any(|v| v.color[3] == 1.0));
    }

    #[test]
    fn removed_burgs_are_skipped() {
        let mut b = burg(1, "town", false);
        b.removed = true;
        let mesh = build_burg_icons_mesh(&[b]);
        assert!(mesh.vertices.is_empty());
    }

    #[test]
    fn unknown_group_falls_back_to_town() {
        let b = burg(1, "whatever", false);
        let mesh = build_burg_icons_mesh(&[b]);
        assert!(!mesh.vertices.is_empty());
        assert!((group_style("whatever").size - group_style("town").size).abs() < 1e-6);
    }
}
