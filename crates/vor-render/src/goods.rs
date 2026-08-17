//! Goods layer (FMG `draw-goods.ts`): three sub-layers.
//!
//! 1. **goodsCells** — per-cell polygons filled with the good color, opacity
//!    normalized against the global max cell production.
//! 2. **goodsIcons** — a marker circle in each cell with a bonus resource.
//! 3. **goodsBurgs** — plates next to burgs with their top-3 produced goods
//!    (value labels). Requires per-burg production; wired once the economy
//!    module provides it (Fase 7).

use vor_core::entities::good::Good;
use vor_core::pack::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use crate::mesh::build_pack_mesh;

/// Bonus-resource rural production factor (FMG `BONUS_RURAL_PRODUCTION`).
const BONUS_RURAL_PRODUCTION: f32 = 0.25;
/// Bonus-resource cell production cap (FMG `MAX_BONUS_PRODUCTION`).
const MAX_BONUS_PRODUCTION: f32 = 5.0;

/// Per-cell production of a good channel (FMG `Production.getCellProduction`).
///
/// Current approximation: `biome_output[cell.biome] × population` (rural) plus
/// the bonus resource channel when `cells.good[cell] == good.id`. The full
/// multiplier stack (`getModifiers`) and manufactured recipes are Fase 7.
pub fn cell_production(pack: &Pack, good: &Good, cell: usize) -> f32 {
    let biome = pack.cells.biome.get(cell).copied().unwrap_or(0);
    let pop = pack.cells.population.get(cell).copied().unwrap_or(0.0);
    let biome_out = good
        .biome_output
        .get(&biome.to_string())
        .copied()
        .unwrap_or(0.0);
    let rural = biome_out * pop;

    let is_bonus = pack.cells.good.get(cell).copied().unwrap_or(0) == good.id;
    let bonus = if is_bonus {
        (pop * BONUS_RURAL_PRODUCTION).min(MAX_BONUS_PRODUCTION)
    } else {
        0.0
    };
    rural + bonus
}

/// Builds the **goodsCells** sub-layer: one polygon per cell per produced good,
/// opacity `0.1 + 0.9 * normalize(total, 0, maxTotal)` (FMG
/// `draw-goods.ts:buildGoodsCellsContent`).
pub fn build_goods_cells_mesh(pack: &Pack, goods: &[Good]) -> HeightmapMesh {
    let n = pack.points_n();
    // First pass: total production per cell + global max.
    let mut cell_total: Vec<f32> = vec![0.0; n];
    let mut max_total = 0.0f32;

    for (p, total_slot) in cell_total.iter_mut().enumerate() {
        let mut total = 0.0f32;
        for good in goods.iter().filter(|g| g.id != 0 && g.visible) {
            let prod = cell_production(pack, good, p);
            if prod <= 0.0 {
                continue;
            }
            total += prod;
        }
        if total > 0.0 {
            *total_slot = total;
            if total > max_total {
                max_total = total;
            }
        }
    }
    if max_total <= 0.0 {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [0.0, 0.0],
            bounds_max: [0.0, 0.0],
        };
    }

    // FMG emits one polygon for every positive good channel in a cell, not
    // only the dominant good. Merge one cell mesh per visible good; draw order
    // is immaterial because the layer is alpha blended.
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };
    for good in goods.iter().filter(|g| g.id != 0 && g.visible) {
        let color = if good.color.is_empty() {
            [0.5, 0.5, 0.5, 1.0]
        } else {
            hex_color_to_linear(&good.color)
        };
        let good_mesh = build_pack_mesh(&pack.vertices, n, |p| {
            let prod = cell_production(pack, good, p);
            if prod <= 0.0 {
                return [0.0, 0.0, 0.0, 0.0];
            }
            let opacity = 0.1 + 0.9 * (cell_total[p] / max_total);
            [color[0], color[1], color[2], opacity.clamp(0.0, 1.0)]
        });
        let base = mesh.vertices.len() as u32;
        mesh.vertices.extend(good_mesh.vertices);
        mesh.indices
            .extend(good_mesh.indices.into_iter().map(|i| i + base));
        mesh.bounds_min[0] = mesh.bounds_min[0].min(good_mesh.bounds_min[0]);
        mesh.bounds_min[1] = mesh.bounds_min[1].min(good_mesh.bounds_min[1]);
        mesh.bounds_max[0] = mesh.bounds_max[0].max(good_mesh.bounds_max[0]);
        mesh.bounds_max[1] = mesh.bounds_max[1].max(good_mesh.bounds_max[1]);
    }
    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0, 0.0];
        mesh.bounds_max = [0.0, 0.0];
    }
    mesh
}

/// Builds the **goodsIcons** sub-layer: a small marker circle in each cell that
/// has a bonus resource (FMG `draw-goods.ts:buildGoodsIconsContent`).
pub fn build_goods_icons_mesh(pack: &Pack, goods: &[Good]) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let n = pack.points_n();
    const SEGMENTS: u32 = 8;
    const RADIUS: f32 = 1.8;

    for p in 0..n {
        let gid = pack.cells.good.get(p).copied().unwrap_or(0);
        if gid == 0 {
            continue;
        }
        let good = match goods.iter().find(|g| g.id == gid) {
            Some(g) if g.visible && !g.color.is_empty() => g,
            _ => continue,
        };
        let color = hex_color_to_linear(&good.color);
        let center = pack.points.get(p).copied().unwrap_or([0.0, 0.0]);
        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(HeightmapVertex { pos: center, color });
        for i in 0..SEGMENTS {
            let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            mesh.vertices.push(HeightmapVertex {
                pos: [center[0] + RADIUS * a.cos(), center[1] + RADIUS * a.sin()],
                color,
            });
        }
        for i in 0..SEGMENTS {
            let v0 = base + 1 + i;
            let v1 = base + 1 + (i + 1) % SEGMENTS;
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
