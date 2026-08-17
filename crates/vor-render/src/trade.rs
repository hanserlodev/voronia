//! Trade layer (FMG `draw-trade-animation.ts`).
//!
//! FMG animates wagon/ship markers moving along the trade routes between the
//! deal's seller and buyer. Voronia renders a static approximation first: the
//! routes themselves (line between each deal's endpoints, colored by good) as a
//! line layer. The animated markers (Dijkstra over `cells.routes`, Catmull-Rom,
//! rotation) are Fase 8 animation.

use std::collections::HashMap;

use vor_core::entities::burg::Burg;
use vor_core::entities::deal::{Deal, DealEntityType};
use vor_core::entities::good::Good;
use vor_core::entities::market::Market;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// Half-width of a trade route segment.
const ROUTE_HALF_WIDTH: f32 = 0.2;

/// Builds the trade routes mesh: one segment between the seller and buyer of
/// each deal, colored by the traded good.
pub fn build_trade_routes_mesh(
    deals: &[Deal],
    burgs: &[Burg],
    markets: &[Market],
    goods: &[Good],
) -> HeightmapMesh {
    // Entity position lookup.
    let mut positions: HashMap<u32, [f32; 2]> = HashMap::new();
    for b in burgs.iter().filter(|b| b.id != 0) {
        positions.insert(b.id as u32, b.position);
    }
    for m in markets.iter().filter(|m| m.id != 0) {
        if let Some(b) = burgs.iter().find(|b| b.id == m.center_burg_id) {
            positions.insert(100_000 + m.id as u32, b.position);
        }
    }

    let color_of = |good_id: u16| -> [f32; 4] {
        goods
            .iter()
            .find(|g| g.id == good_id)
            .and_then(|g| {
                if g.color.is_empty() {
                    None
                } else {
                    Some(hex_color_to_linear(&g.color))
                }
            })
            .unwrap_or([0.5, 0.5, 0.5, 0.6])
    };

    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    for deal in deals {
        let seller_key = match deal.seller_type {
            DealEntityType::Market => 100_000 + deal.seller,
            _ => deal.seller,
        };
        let buyer_key = match deal.buyer_type {
            DealEntityType::Market => 100_000 + deal.buyer,
            _ => deal.buyer,
        };
        let a = match positions.get(&seller_key) {
            Some(p) => *p,
            None => continue,
        };
        let b = match positions.get(&buyer_key) {
            Some(p) => *p,
            None => continue,
        };
        let color = color_of(deal.good);
        let base = mesh.vertices.len() as u32;
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            continue;
        }
        let nx = -dy / len * ROUTE_HALF_WIDTH;
        let ny = dx / len * ROUTE_HALF_WIDTH;
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
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
        for v in &mesh.vertices[base as usize..][..4] {
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
