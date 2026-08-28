//! Markers layer (FMG `#markers`, `src/renderers/draw-markers.ts`).
//!
//! Each marker is a nested `<svg>` (viewBox 30×30) anchored **bottom-center**
//! on its point (`viewX = x − size/2`, `viewY = y − size`), containing a pin
//! shape (fill `#fff` stroke `#000`) and an emoji `<text>` icon at
//! `(dx%, dy%)` with font-size `px` (defaults dx50/dy50/px12, size 30).
//!
//! Zoom rescale (`main.js:608-621`): `zoomSize = max(rn(size/5 + 24/scale, 2), 1)`
//! when `rescale` is set (default). The emoji text renders via the glyphon
//! batch; the pin geometry via this mesh. Group opacity: none (opaque).

use vor_core::entities::marker::Marker;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// Marker world size at scale = 1.
pub const MARKER_SIZE: f32 = 30.0;
/// Emoji font size in world units inside the marker box.
pub const MARKER_ICON_PX: f32 = 12.0;

const PIN_FILL: [f32; 4] = [1.0, 1.0, 1.0, 1.0]; // #ffffff
const PIN_STROKE: [f32; 4] = [0.0, 0.0, 0.0, 1.0]; // #000000

/// Pin shapes from `pinShapes` (`draw-markers.ts:32-56`), in marker-box
/// coordinates (0..30, y down, tip at bottom-center ≈ (15, 28)). Only the
/// kinds referenced by real maps are tabulated; unknown kinds fall back to
/// `bubble`.
fn pin_outline(kind: &str) -> Vec<(f32, f32)> {
    match kind {
        // Rounded bubble: circle-ish top + tail to the tip.
        "bubble" | "" => vec![
            (15.0, 2.0),
            (22.0, 2.0),
            (27.0, 7.0),
            (27.0, 14.0),
            (22.0, 19.0),
            (17.5, 19.0),
            (15.0, 28.0),
            (12.5, 19.0),
            (8.0, 19.0),
            (3.0, 14.0),
            (3.0, 7.0),
            (8.0, 2.0),
        ],
        // Classic map pin: teardrop.
        "pin" => vec![
            (15.0, 2.0),
            (23.0, 10.0),
            (21.0, 18.0),
            (15.0, 28.0),
            (9.0, 18.0),
            (7.0, 10.0),
        ],
        "square" => vec![(3.0, 3.0), (27.0, 3.0), (27.0, 27.0), (3.0, 27.0)],
        "squarish" => vec![
            (6.0, 3.0),
            (24.0, 3.0),
            (27.0, 6.0),
            (27.0, 24.0),
            (24.0, 27.0),
            (6.0, 27.0),
            (3.0, 24.0),
            (3.0, 6.0),
        ],
        "diamond" => vec![(15.0, 2.0), (27.0, 14.0), (15.0, 26.0), (3.0, 14.0)],
        "hex" => vec![
            (15.0, 2.0),
            (25.4, 8.0),
            (25.4, 20.0),
            (15.0, 26.0),
            (4.6, 20.0),
            (4.6, 8.0),
        ],
        "hexy" => vec![
            (11.0, 2.0),
            (19.0, 2.0),
            (27.0, 14.0),
            (19.0, 26.0),
            (11.0, 26.0),
            (3.0, 14.0),
        ],
        "shieldy" | "shield" => vec![
            (4.0, 3.0),
            (26.0, 3.0),
            (26.0, 16.0),
            (15.0, 28.0),
            (4.0, 16.0),
        ],
        "pentagon" => vec![
            (15.0, 2.0),
            (27.0, 11.0),
            (22.4, 26.0),
            (7.6, 26.0),
            (3.0, 11.0),
        ],
        "heptagon" => vec![
            (15.0, 2.0),
            (24.8, 6.2),
            (27.7, 16.5),
            (21.2, 25.2),
            (8.8, 25.2),
            (2.3, 16.5),
            (5.2, 6.2),
        ],
        "circle" => {
            let mut pts = Vec::with_capacity(12);
            for i in 0..12 {
                let a = i as f32 / 12.0 * std::f32::consts::TAU;
                pts.push((15.0 + 12.0 * a.cos(), 15.0 + 12.0 * a.sin()));
            }
            pts
        }
        _ => vec![
            (15.0, 2.0),
            (22.0, 2.0),
            (27.0, 7.0),
            (27.0, 14.0),
            (22.0, 19.0),
            (17.5, 19.0),
            (15.0, 28.0),
            (12.5, 19.0),
            (8.0, 19.0),
            (3.0, 14.0),
            (3.0, 7.0),
            (8.0, 2.0),
        ], // bubble fallback
    }
}

/// World-space quad for one marker pin + metadata for the glyphon emoji.
#[derive(Debug, Clone)]
pub struct MarkerLabel {
    /// Emoji/icon text of the marker.
    pub text: String,
    /// Center of the icon area (world units).
    pub x: f32,
    pub y: f32,
}

/// Builds the marker pins mesh (world units, scale = 1 → box of
/// [`MARKER_SIZE`]) and returns the per-marker emoji label positions
/// (icon center in world units).
pub fn build_marker_pins(markers: &[Marker]) -> (HeightmapMesh, Vec<MarkerLabel>) {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    let mut labels = Vec::new();

    for m in markers {
        if m.removed {
            continue;
        }
        let size = MARKER_SIZE * m.size.unwrap_or(1.0);
        let view_x = m.position[0] - size / 2.0;
        let view_y = m.position[1] - size;
        let scale = size / 30.0;
        let outline: Vec<(f32, f32)> = pin_outline(&m.kind)
            .iter()
            .map(|&(x, y)| (view_x + x * scale, view_y + y * scale))
            .collect();

        push_polygon(&mut mesh, &outline, PIN_FILL);
        // Stroke outline as thin segments (SVG stroke default w≈1 user unit).
        for pair in outline.windows(2) {
            push_segment_stroke(&mut mesh, pair[0], pair[1], PIN_STROKE);
        }
        // Close the loop stroke.
        if let (Some(last), Some(first)) = (outline.last(), outline.first()) {
            push_segment_stroke(&mut mesh, *last, *first, PIN_STROKE);
        }

        // Icon center: (dx%, dy%) defaults 50/50 of the box.
        labels.push(MarkerLabel {
            text: m.icon.clone(),
            x: view_x + size / 2.0,
            y: view_y + size / 2.0,
        });
    }

    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0; 2];
        mesh.bounds_max = [0.0; 2];
    }
    (mesh, labels)
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

fn push_segment_stroke(mesh: &mut HeightmapMesh, a: (f32, f32), b: (f32, f32), color: [f32; 4]) {
    const W: f32 = 0.35; // ~1 SVG user unit at scale 1 → thin
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return;
    }
    let nx = -dy / len * W / 2.0;
    let ny = dx / len * W / 2.0;
    push_polygon(
        mesh,
        &[
            (a.0 + nx, a.1 + ny),
            (b.0 + nx, b.1 + ny),
            (b.0 - nx, b.1 - ny),
            (a.0 - nx, a.1 - ny),
        ],
        color,
    );
}

/// Emoji atlas + upright quads for the marker icons, rasterized with the
/// bundled Noto Emoji (monochrome) — glyphon has no emoji font, so the icons
/// are baked like the state labels. Returns a generic textured-quad mesh.
pub fn build_marker_emoji_mesh(
    font_bytes: &[u8],
    markers: &[Marker],
) -> Option<crate::state_labels::StateLabelsMesh> {
    let Ok(font) = fontdue::Font::from_bytes(
        font_bytes,
        fontdue::FontSettings {
            collection_index: 0,
            ..Default::default()
        },
    ) else {
        return None;
    };
    let px = 48.0f32;
    let mut unique: Vec<String> = Vec::new();
    for m in markers {
        if !m.removed && !unique.contains(&m.icon) {
            unique.push(m.icon.clone());
        }
    }
    if unique.is_empty() {
        return None;
    }
    let strips: Vec<(String, crate::state_labels::LabelStrip)> = unique
        .iter()
        .map(|ic| {
            (
                ic.clone(),
                crate::state_labels::rasterize_text(&font, ic, px),
            )
        })
        .collect();
    let strip_w = strips
        .iter()
        .map(|(_, s)| s.width)
        .max()
        .unwrap_or(1)
        .max(1);
    let strip_h = strips.first().map(|(_, s)| s.height).unwrap_or(1).max(1);
    let cols = 8u32;
    let rows = (strips.len() as u32).div_ceil(cols);
    let atlas_w = cols * strip_w;
    let atlas_h = rows * strip_h;
    let mut atlas = vec![0u8; (atlas_w * atlas_h * 4) as usize];
    let mut cell_of: std::collections::HashMap<String, u8> = Default::default();
    for (i, (ic, st)) in strips.iter().enumerate() {
        let col = (i as u32 % cols) * strip_w;
        let row = (i as u32 / cols) * strip_h;
        for r in 0..st.height {
            let src = (r * st.width * 4) as usize;
            let dst = ((row + r) * atlas_w * 4 + col * 4) as usize;
            let copy = (st.width * 4) as usize;
            if dst + copy <= atlas.len() && src + copy <= st.rgba.len() {
                atlas[dst..dst + copy].copy_from_slice(&st.rgba[src..src + copy]);
            }
        }
        cell_of.insert(ic.clone(), i as u8);
    }

    let cw = 1.0 / cols as f32;
    let ch = 1.0 / rows as f32;
    let mut vertices: Vec<[f32; 4]> = Vec::new();
    for m in markers {
        if m.removed {
            continue;
        }
        let Some(&cell) = cell_of.get(&m.icon) else {
            continue;
        };
        let sz = MARKER_SIZE * m.size.unwrap_or(1.0);
        let view_x = m.position[0] - sz / 2.0;
        let view_y = m.position[1] - sz;
        let icon_edge = MARKER_ICON_PX / 30.0 * sz * 1.6;
        let cx = view_x + sz * 0.5;
        let cy = view_y + sz * 0.5;
        let (u0, v0) = (
            (u32::from(cell) % cols) as f32 * cw,
            (u32::from(cell) as f32 / cols as f32).floor() * ch,
        );
        let (u1, v1) = (u0 + cw, v0 + ch);
        let (x0, y0, x1, y1) = (
            cx - icon_edge / 2.0,
            cy - icon_edge / 2.0,
            cx + icon_edge / 2.0,
            cy + icon_edge / 2.0,
        );
        for (px_, py_, u, v) in [
            (x0, y0, u0, v0),
            (x1, y0, u1, v0),
            (x1, y1, u1, v1),
            (x0, y0, u0, v0),
            (x1, y1, u1, v1),
            (x0, y1, u0, v1),
        ] {
            vertices.push([px_, py_, u, v]);
        }
    }
    let vertex_count = vertices.len() as u32;
    Some(crate::state_labels::StateLabelsMesh {
        atlas_rgba: atlas,
        atlas_width: atlas_w,
        atlas_height: atlas_h,
        vertices,
        vertex_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(kind: &str, removed: bool) -> Marker {
        Marker {
            id: 1,
            icon: "🌋".into(),
            kind: kind.into(),
            label_dx: 0,
            label_px: 0,
            position: [500.0, 500.0],
            cell: 0,
            legend: None,
            note_id: None,
            removed,
            size: None,
        }
    }

    #[test]
    fn pin_geometry_and_anchor() {
        let (mesh, labels) = build_marker_pins(&[marker("bubble", false)]);
        assert!(!mesh.vertices.is_empty());
        assert_eq!(labels.len(), 1);
        // Bottom-center anchor: box spans x∈[485,515], y∈[470,500].
        let l = &labels[0];
        assert!((l.x - 500.0).abs() < 1e-4 && (l.y - 485.0).abs() < 1e-4);
        // Bubble spans y 2..28 of the 30-unit box → world [472, 498].
        assert!(
            (mesh.bounds_min[1] - 472.0).abs() < 0.3 && (mesh.bounds_max[1] - 498.0).abs() < 0.3
        );
    }

    #[test]
    fn removed_markers_skipped_and_unknown_kind_falls_back() {
        let (mesh_removed, _) = build_marker_pins(&[marker("pin", true)]);
        assert!(mesh_removed.vertices.is_empty());
        let (_, labels) = build_marker_pins(&[marker("not-a-kind", false)]);
        assert_eq!(labels.len(), 1); // bubble fallback still renders
    }

    #[test]
    fn emoji_mesh_builds_from_font() {
        let font = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/NotoEmoji-Regular.ttf"
        ))
        .unwrap_or_default();
        if font.is_empty() {
            return; // font asset missing — skip
        }
        let markers = vec![marker("volcanoes", false)];
        let mesh = build_marker_emoji_mesh(&font, &markers);
        assert!(mesh.is_some(), "emoji mesh should build with font present");
        let mesh = mesh.unwrap();
        assert!(mesh.vertex_count > 0);
    }
}
