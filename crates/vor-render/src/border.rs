//! Region borders (FMG `#borders > #stateBorders` / `#provinceBorders`),
//! ported from `src/renderers/draw-borders.ts`.
//!
//! FMG traces **maximal continuous vertex chains** per border pair (two-pass:
//! provinces first, then states; oriented dedupe keys; `cellId--` retry so a
//! cell can start several chains) and emits ONE combined open path per group
//! (`M x,y x,y ...`, no `Z`; closed loops repeat the first vertex). Dashes
//! therefore flow continuously along each chain.
//!
//! Styles (`default.json`): state `#56566d` w1 dash `2` butt · province
//! `#56566d` w0.5 dots `0 2` round — group opacity 0.8.
//!
//! Water gates (three, like FMG): the bordering neighbor must be land, the
//! start vertex must touch a land B-side cell, and every chain vertex must
//! touch ≥1 A-side cell and ≥1 land B-side cell — chains never cross water.
//!
//! `BorderKind::Culture` does NOT exist in FMG (`#cultureBorders` is absent);
//! it is a documented Voronia extension using the same chain walker.

use std::collections::HashSet;

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

/// Builds one border-kind mesh: continuous chains stroked with the group
/// style (dash/dots flow along the whole chain, like the SVG `d`).
pub fn build_border_mesh(pack: &Pack, kind: BorderKind) -> HeightmapMesh {
    let style = style_of(&kind);
    let chains = get_border_chains(pack, &kind);

    let mut acc = MeshAcc::new();
    for chain in &chains {
        match style.dash {
            None => {
                for seg in chain.windows(2) {
                    push_quad(&mut acc, seg[0], seg[1], style.width, style.color);
                }
            }
            Some([on, off]) if on > 0.0 => {
                for seg in crate::route_layer::dash_segments_pub(chain, [on, off]) {
                    for run in seg.windows(2) {
                        push_quad(&mut acc, run[0], run[1], style.width, style.color);
                    }
                }
            }
            Some([0.0, off]) => {
                // Round-cap dots (dasharray `0 <gap>`): dot diameter = stroke
                // width, spaced `off` apart along the whole chain.
                let mut d = 0.0;
                for seg in chain.windows(2) {
                    let dx = seg[1][0] - seg[0][0];
                    let dy = seg[1][1] - seg[0][1];
                    let len = (dx * dx + dy * dy).sqrt();
                    if len < 1e-9 {
                        continue;
                    }
                    let ux = dx / len;
                    let uy = dy / len;
                    while d <= len {
                        push_dot(
                            &mut acc,
                            [seg[0][0] + ux * d, seg[0][1] + uy * d],
                            style.width / 2.0,
                            style.color,
                        );
                        d += off;
                    }
                    d -= len;
                }
            }
            Some(_) => {}
        }
    }

    HeightmapMesh {
        vertices: acc.vertices,
        indices: acc.indices,
        bounds_min: acc.bounds_min,
        bounds_max: acc.bounds_max,
    }
}

/// Port of `draw-borders.ts`: two passes (provinces → states; cultures as a
/// Voronia extension), oriented dedupe keys, `cell--` retry, and one chain
/// per region pair.
fn get_border_chains(pack: &Pack, kind: &BorderKind) -> Vec<Vec<[f32; 2]>> {
    let ids: &[u16] = match kind {
        BorderKind::State => &pack.cells.state,
        BorderKind::Province => &pack.cells.province,
        BorderKind::Culture => &pack.cells.culture,
    };
    let heights = &pack.cells.height;
    let is_land = |c: usize| heights.get(c).copied().unwrap_or(0) >= 20;
    let n = pack.points_n();

    let mut chains: Vec<Vec<[f32; 2]>> = Vec::new();
    let mut checked: HashSet<(u16, u16, usize)> = HashSet::new();

    for cell in 0..n {
        let id = ids.get(cell).copied().unwrap_or(0);
        // FMG skips water + neutral cells (states); provinces/cultures need
        // an id too (0 = none).
        if id == 0 || !is_land(cell) {
            continue;
        }
        let mut retried = false;
        let mut idx = cell;
        loop {
            let mut found = false;
            let Some(neibs) = pack.cells.adjacency.get(idx) else {
                break;
            };
            for &nb_u32 in neibs {
                let nb = nb_u32 as usize;
                if nb >= n || !is_land(nb) {
                    continue;
                }
                let nid = ids.get(nb).copied().unwrap_or(0);
                if nid == 0 || nid == id {
                    continue;
                }
                // Orientation: only trigger from the higher-id side (FMG
                // `id > neibId`), except cultures which also require nothing
                // more (extension mirrors the state rules).
                let (p1, p2) = if id > nid { (id, nid) } else { (nid, id) };
                // Provinces: FMG only draws intra-state borders.
                if matches!(kind, BorderKind::Province)
                    && pack.cells.state.get(cell).copied().unwrap_or(0)
                        != pack.cells.state.get(nb).copied().unwrap_or(0)
                {
                    continue;
                }
                if !checked.insert((p1, p2, idx)) {
                    continue;
                }
                let chain = get_border(pack, idx, nb, ids, &is_land);
                if chain.len() > 1 {
                    chains.push(chain);
                }
                found = true;
                // FMG `cellId--` retry: the same cell may start several
                // chains (different pairs).
                break;
            }
            if found && !retried {
                retried = true;
                idx = cell; // re-scan the same cell for further pairs
                continue;
            }
            break;
        }
    }

    chains
}

/// Port of `getBorder({type, fromCell, toCell})`: start vertex on the shared
/// edge, `checkVertex` gates, two-run walk; returns world-space points
/// (closed loops repeat their first point).
fn get_border(
    pack: &Pack,
    from_cell: usize,
    to_cell: usize,
    ids: &[u16],
    is_land: &impl Fn(usize) -> bool,
) -> Vec<[f32; 2]> {
    let vertices = &pack.vertices;
    let n = pack.points_n();
    let a_id = ids.get(from_cell).copied().unwrap_or(0);
    let b_id = ids.get(to_cell).copied().unwrap_or(0);

    let touches = |v: u32, pred: &dyn Fn(usize) -> bool| -> bool {
        vertices
            .adjacent_cells
            .get(v as usize)
            .map(|cells| {
                cells
                    .iter()
                    .any(|&c| c >= 0 && (c as usize) < n && pred(c as usize))
            })
            .unwrap_or(false)
    };
    let is_a = |c: usize| ids.get(c).copied().unwrap_or(0) == a_id;
    let is_b_land = |c: usize| ids.get(c).copied().unwrap_or(0) == b_id && is_land(c);
    let check_vertex = |v: u32| -> bool { touches(v, &is_a) && touches(v, &is_b_land) };

    // Start: first vertex of fromCell adjacent to toCell (which is land).
    let ring = match vertices.cell_rings.get(from_cell) {
        Some(r) if r.len() >= 3 => r,
        _ => return Vec::new(),
    };
    let start = match ring.iter().copied().find(|&v| {
        vertices
            .adjacent_cells
            .get(v as usize)
            .map(|cs| cs.iter().any(|&c| c >= 0 && c as usize == to_cell))
            .unwrap_or(false)
    }) {
        Some(v) => v,
        None => return Vec::new(),
    };

    let max_iterations = vertices.adjacent_cells.len() * 2;
    let walk = |first: u32, blocked: Option<u32>| -> (Vec<u32>, bool) {
        let mut chain = vec![first];
        let mut prev = blocked;
        let mut cur = first;
        for _ in 0..max_iterations {
            let adj_cells = vertices
                .adjacent_cells
                .get(cur as usize)
                .copied()
                .unwrap_or([-1, -1, -1]);
            let adj_verts = vertices
                .adjacent_vertices
                .get(cur as usize)
                .copied()
                .unwrap_or([-1, -1, -1]);
            let cls = |c: i32| -> bool { c >= 0 && (c as usize) < n && is_a(c as usize) };
            let (c0, c1, c2) = (cls(adj_cells[0]), cls(adj_cells[1]), cls(adj_cells[2]));
            let (v0, v1, v2) = (adj_verts[0], adj_verts[1], adj_verts[2]);

            let mut next = -1i32;
            if v0 >= 0 && Some(v0 as u32) != prev && c0 != c1 && check_vertex(v0 as u32) {
                next = v0;
            } else if v1 >= 0 && Some(v1 as u32) != prev && c1 != c2 && check_vertex(v1 as u32) {
                next = v1;
            } else if v2 >= 0 && Some(v2 as u32) != prev && c0 != c2 && check_vertex(v2 as u32) {
                next = v2;
            }
            if next < 0 || next as u32 == first {
                if next as u32 == first {
                    chain.push(first); // closed loop repeats the start
                    return (chain, true);
                }
                break; // stuck: dead end (coast / junction)
            }
            chain.push(next as u32);
            prev = Some(cur);
            cur = next as u32;
        }
        (chain, false)
    };

    let (mut chain_ids, closed) = walk(start, None);
    if !closed {
        // FMG second run: restart from the reached extreme to cover the whole
        // chain. We walk the opposite direction from `start` blocking the
        // first-run successor, then prepend (reversed) without duplicating.
        let second = chain_ids.get(1).copied();
        if let Some(second) = second {
            let (back, _) = walk(start, Some(second));
            if back.len() > 1 {
                let mut prefix: Vec<u32> = back[..back.len() - 1].iter().rev().copied().collect();
                prefix.extend_from_slice(&chain_ids);
                chain_ids = prefix;
            }
        }
    }

    if chain_ids.len() < 2 {
        return Vec::new();
    }
    chain_ids
        .iter()
        .filter_map(|&v| vertices.positions.get(v as usize).copied())
        .collect()
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
            bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
        }
    }
}

fn push_quad(acc: &mut MeshAcc, a: [f32; 2], b: [f32; 2], width: f32, color: [f32; 4]) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return;
    }
    let len = len_sq.sqrt();
    let nx = -dy / len * width;
    let ny = dx / len * width;
    let base = acc.vertices.len() as u32;
    acc.vertices.extend_from_slice(&[
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
    acc.indices
        .extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    for v in &acc.vertices[base as usize..][..4] {
        acc.bounds_min[0] = acc.bounds_min[0].min(v.pos[0]);
        acc.bounds_min[1] = acc.bounds_min[1].min(v.pos[1]);
        acc.bounds_max[0] = acc.bounds_max[0].max(v.pos[0]);
        acc.bounds_max[1] = acc.bounds_max[1].max(v.pos[1]);
    }
}

fn push_dot(acc: &mut MeshAcc, center: [f32; 2], radius: f32, color: [f32; 4]) {
    const SEGMENTS: usize = 8;
    let base = acc.vertices.len() as u32;
    acc.vertices.push(HeightmapVertex { pos: center, color });
    for i in 0..SEGMENTS {
        let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        acc.vertices.push(HeightmapVertex {
            pos: [center[0] + radius * a.cos(), center[1] + radius * a.sin()],
            color,
        });
    }
    for i in 0..SEGMENTS {
        let v0 = base + 1 + i as u32;
        let v1 = base + 1 + ((i + 1) % SEGMENTS) as u32;
        acc.indices.extend_from_slice(&[base, v0, v1]);
    }
    for v in &acc.vertices[base as usize..] {
        acc.bounds_min[0] = acc.bounds_min[0].min(v.pos[0]);
        acc.bounds_min[1] = acc.bounds_min[1].min(v.pos[1]);
        acc.bounds_max[0] = acc.bounds_max[0].max(v.pos[0]);
        acc.bounds_max[1] = acc.bounds_max[1].max(v.pos[1]);
    }
}
