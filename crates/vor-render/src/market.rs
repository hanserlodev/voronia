//! Markets layer (FMG `draw-markets.ts`).
//!
//! Renders the market areas of influence as isoline fills (one per market,
//! `pack.cells.market`) with a low-opacity fill and a darker border, plus a
//! solid circle at the central burg with the market icon.

use vor_core::entities::burg::Burg;
use vor_core::entities::market::Market;
use vor_core::pack::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use crate::isoline::build_region_mesh;

/// Half-width of the market border quads.
const BORDER_HALF_WIDTH: f32 = 0.35;
/// Radius of the market center circle.
const CENTER_RADIUS: f32 = 3.0;
/// Segments of the center circle.
const CENTER_SEGMENTS: u32 = 16;

/// Builds the market **fill** layer: isoline per market id, fill at low opacity
/// (FMG `fill-opacity: 0.03`), border darker (`color.darker()`).
pub fn build_market_fill_mesh(pack: &Pack, markets: &[Market]) -> HeightmapMesh {
    let get_type = |p: usize| pack.cells.market.get(p).copied().unwrap_or(0);
    let color_fn = |mid: u16| -> [f32; 4] {
        match markets.iter().find(|m| m.id == mid) {
            Some(m) if !m.color.is_empty() => {
                let mut c = hex_color_to_linear(&m.color);
                c[3] = 0.03; // FMG `fill-opacity: 0.03`
                c
            }
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    };
    let mut mesh = build_region_mesh(pack, &get_type, &color_fn);
    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0, 0.0];
        mesh.bounds_max = [0.0, 0.0];
    }
    mesh
}

/// Builds the market **border** mesh: darker stroke on the isoline boundaries.
pub fn build_market_border_mesh(pack: &Pack, markets: &[Market]) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let n = pack.points_n();
    for p in 0..n {
        let mid = pack.cells.market.get(p).copied().unwrap_or(0);
        if mid == 0 {
            continue;
        }
        let neighbors = match pack.cells.adjacency.get(p) {
            Some(v) => v,
            None => continue,
        };
        for &nb in neighbors {
            let nb = nb as usize;
            if nb >= n {
                continue;
            }
            if pack.cells.market.get(nb).copied().unwrap_or(0) != mid {
                // Border between two markets.
                let color = markets
                    .iter()
                    .find(|m| m.id == mid)
                    .and_then(|m| {
                        if m.color.is_empty() {
                            None
                        } else {
                            Some(hex_color_to_linear(&m.color))
                        }
                    })
                    .unwrap_or([0.0, 0.0, 0.0, 0.8]);
                let a = pack.points.get(p).copied().unwrap_or([0.0, 0.0]);
                let b = pack.points.get(nb).copied().unwrap_or([0.0, 0.0]);
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let len = (dx * dx + dy * dy).sqrt();
                if len < 0.001 {
                    continue;
                }
                let nx = -dy / len * BORDER_HALF_WIDTH;
                let ny = dx / len * BORDER_HALF_WIDTH;
                let base = mesh.vertices.len() as u32;
                mesh.vertices.extend_from_slice(&[
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
                mesh.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base + 1,
                    base + 3,
                    base + 2,
                ]);
                for v in &mesh.vertices[base as usize..][..4] {
                    mesh.bounds_min[0] = mesh.bounds_min[0].min(v.pos[0]);
                    mesh.bounds_min[1] = mesh.bounds_min[1].min(v.pos[1]);
                    mesh.bounds_max[0] = mesh.bounds_max[0].max(v.pos[0]);
                    mesh.bounds_max[1] = mesh.bounds_max[1].max(v.pos[1]);
                }
            }
        }
    }

    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0, 0.0];
        mesh.bounds_max = [0.0, 0.0];
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
