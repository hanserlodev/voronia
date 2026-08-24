//! Region borders (FMG `#borders > #stateBorders` / `#provinceBorders`).
//!
//! Styles from `default.json` (v1.138):
//! - `#stateBorders`: opacity 0.8, stroke `#56566d`, width **1**, dash `2`,
//!   linecap butt.
//! - `#provinceBorders`: opacity 0.8, stroke `#56566d`, width **0.5**, dash
//!   `0 2` (round-cap dots spaced 2), linecap round.
//!
//! `BorderKind::Culture` has no FMG default (`#cultureBorders` is absent);
//! it keeps a Voronia-specific solid amber style, documented as an extension.
//!
//! Geometry note: FMG traces full border chains via `getBorderPath`; we emit
//! one quad per shared Voronoi edge (equivalent strokes, per-edge dashes).

use std::collections::HashMap;

use vor_core::pack::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{HeightmapMesh, HeightmapVertex};

pub enum BorderKind {
    State,
    Province,
    Culture,
}

struct BorderStyle {
    color: [f32; 4],
    width: f32,
    /// `[on, off]` dash pattern; `on == 0` means round-cap dots spaced `off`.
    dash: Option<[f32; 2]>,
}

fn style_of(kind: &BorderKind) -> BorderStyle {
    let styled = |hex: &str, width: f32, dash: Option<[f32; 2]>| -> BorderStyle {
        let mut color = hex_color_to_linear(hex);
        color[3] = 0.8; // group opacity
        BorderStyle { color, width, dash }
    };
    match kind {
        BorderKind::State => styled("#56566d", 1.0, Some([2.0, 2.0])),
        BorderKind::Province => styled("#56566d", 0.5, Some([0.0, 2.0])),
        BorderKind::Culture => BorderStyle {
            color: [1.0, 0.65, 0.0, 0.8],
            width: 0.35,
            dash: None,
        },
    }
}

/// Builds one border-kind mesh over the pack's shared Voronoi edges.
pub fn build_border_mesh(pack: &Pack, kind: BorderKind) -> HeightmapMesh {
    let (ids, style): (&[u16], BorderStyle) = match kind {
        BorderKind::State => (&pack.cells.state, style_of(&BorderKind::State)),
        BorderKind::Province => (&pack.cells.province, style_of(&BorderKind::Province)),
        BorderKind::Culture => (&pack.cells.culture, style_of(&BorderKind::Culture)),
    };

    let mut acc = MeshAcc::new();

    // Each Voronoi edge belongs to two cell rings. Keep the first side and
    // emit only when the second side has a different non-zero region id.
    let mut edges: HashMap<(u32, u32), (usize, u16)> = HashMap::new();
    for (cell, ring) in pack.vertices.cell_rings.iter().enumerate() {
        let id = ids.get(cell).copied().unwrap_or(0);
        if id == 0 || ring.len() < 2 {
            continue;
        }
        for i in 0..ring.len() {
            let a_id = ring[i];
            let b_id = ring[(i + 1) % ring.len()];
            if a_id == b_id {
                continue;
            }
            let key = if a_id < b_id {
                (a_id, b_id)
            } else {
                (b_id, a_id)
            };
            if let Some((other_cell, other_id)) = edges.remove(&key) {
                let _ = other_cell;
                if other_id == id || other_id == 0 {
                    continue;
                }
                let a = pack
                    .vertices
                    .positions
                    .get(key.0 as usize)
                    .copied()
                    .unwrap_or([0.0, 0.0]);
                let b = pack
                    .vertices
                    .positions
                    .get(key.1 as usize)
                    .copied()
                    .unwrap_or([0.0, 0.0]);

                match style.dash {
                    None => push_quad(&mut acc, a, b, style.width, style.color),
                    Some([on, off]) if on > 0.0 => {
                        // Regular dash: reuse the routes dash walker.
                        for seg in crate::route_layer::dash_segments_pub(&[a, b], [on, off]) {
                            for run in seg.windows(2) {
                                push_quad(&mut acc, run[0], run[1], style.width, style.color);
                            }
                        }
                    }
                    Some([0.0, off]) => {
                        // Round-cap dots (dasharray `0 <gap>`): dot diameter =
                        // stroke width, spaced `off` apart.
                        let dx = b[0] - a[0];
                        let dy = b[1] - a[1];
                        let len = (dx * dx + dy * dy).sqrt();
                        if len < 1e-6 {
                            continue;
                        }
                        let ux = dx / len;
                        let uy = dy / len;
                        let mut d = 0.0;
                        while d <= len {
                            push_dot(
                                &mut acc,
                                [a[0] + ux * d, a[1] + uy * d],
                                style.width / 2.0,
                                style.color,
                            );
                            d += off;
                        }
                    }
                    Some(_) => {}
                }
            } else {
                edges.insert(key, (cell, id));
            }
        }
    }

    if !acc.bounds_min.iter().all(|v| v.is_finite()) {
        acc.bounds_min = [0.0; 2];
        acc.bounds_max = [0.0; 2];
    }

    HeightmapMesh {
        vertices: acc.vertices,
        indices: acc.indices,
        bounds_min: acc.bounds_min,
        bounds_max: acc.bounds_max,
    }
}

struct MeshAcc {
    vertices: Vec<HeightmapVertex>,
    indices: Vec<u32>,
    bounds_min: [f32; 2],
    bounds_max: [f32; 2],
}

impl MeshAcc {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [f32::INFINITY, f32::INFINITY],
            bounds_max: [f32::NEG_INFINITY, f32::INFINITY - 1.0],
        }
    }
}

fn push_quad(acc: &mut MeshAcc, a: [f32; 2], b: [f32; 2], width: f32, color: [f32; 4]) {
    let vertices = &mut acc.vertices;
    let indices = &mut acc.indices;
    let (bounds_min, bounds_max) = (&mut acc.bounds_min, &mut acc.bounds_max);
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return;
    }
    let nx = -dy / len * width;
    let ny = dx / len * width;
    let base = vertices.len() as u32;
    vertices.extend_from_slice(&[
        HeightmapVertex {
            pos: [a[0] + nx, a[1] + ny],
            color,
        },
        HeightmapVertex {
            pos: [a[0] - nx, a[1] - ny],
            color,
        },
        HeightmapVertex {
            pos: [b[0] + nx, b[1] + ny],
            color,
        },
        HeightmapVertex {
            pos: [b[0] - nx, b[1] - ny],
            color,
        },
    ]);
    indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    for v in &vertices[base as usize..][..4] {
        bounds_min[0] = bounds_min[0].min(v.pos[0]);
        bounds_min[1] = bounds_min[1].min(v.pos[1]);
        bounds_max[0] = bounds_max[0].max(v.pos[0]);
        bounds_max[1] = bounds_max[1].max(v.pos[1]);
    }
}

fn push_dot(acc: &mut MeshAcc, center: [f32; 2], radius: f32, color: [f32; 4]) {
    let vertices = &mut acc.vertices;
    let indices = &mut acc.indices;
    let (bounds_min, bounds_max) = (&mut acc.bounds_min, &mut acc.bounds_max);
    const SEGMENTS: usize = 8;
    let base = vertices.len() as u32;
    vertices.push(HeightmapVertex { pos: center, color });
    for i in 0..SEGMENTS {
        let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        vertices.push(HeightmapVertex {
            pos: [center[0] + radius * a.cos(), center[1] + radius * a.sin()],
            color,
        });
    }
    for i in 0..SEGMENTS {
        let v0 = base + 1 + i as u32;
        let v1 = base + 1 + ((i + 1) % SEGMENTS) as u32;
        indices.extend_from_slice(&[base, v0, v1]);
    }
    for v in &vertices[base as usize..] {
        bounds_min[0] = bounds_min[0].min(v.pos[0]);
        bounds_min[1] = bounds_min[1].min(v.pos[1]);
        bounds_max[0] = bounds_max[0].max(v.pos[0]);
        bounds_max[1] = bounds_max[1].max(v.pos[1]);
    }
}
