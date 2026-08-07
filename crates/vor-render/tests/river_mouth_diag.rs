//! Guard de regresión para la desembocadura de ríos: carga Sorvik real y comprueba
//! que todo río cuyo path llegue a una celda de agua logra clip a la costa (encuentra
//! una intersección con el anillo Voronoi de la celda de desembocadura). Replica la
//! geometría de `clip_to_coast` de river.rs para poder contabilizar los fallos.

use vor_import::mapfile::{raw, Loader};

const SORVIK_MAP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vor-import/tests/reference/Sorvik-2026-07-24-23-39.map"
);

fn segment_intersection(
    s0: [f32; 2],
    s1: [f32; 2],
    e0: [f32; 2],
    e1: [f32; 2],
) -> Option<[f32; 2]> {
    let r = [s1[0] - s0[0], s1[1] - s0[1]];
    let e = [e1[0] - e0[0], e1[1] - e0[1]];
    let denom = r[0] * e[1] - r[1] * e[0];
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = (s0[1] - e0[1]) * e[0] - (s0[0] - e0[0]) * e[1];
    let u = (s0[1] - e0[1]) * r[0] - (s0[0] - e0[0]) * r[1];
    let t = t / denom;
    let u = u / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some([s0[0] + t * r[0], s0[1] + t * r[1]])
    } else {
        None
    }
}

#[test]
fn diagnose_mouth_clip_failures() {
    let bytes = std::fs::read(SORVIK_MAP_PATH).expect("map");
    let raw = raw::parse(&bytes).expect("parse");
    let result = Loader::load(&raw).expect("load");

    let n = result.world.pack.points_n();
    let points = &result.world.pack.points;
    let vertices = &result.world.pack.vertices;
    let is_water: Vec<bool> =
        (0..n)
            .map(|p| {
                let h = result.world.pack.cells.height.get(p).copied().unwrap_or(0);
                let fid = result
                    .world
                    .pack
                    .cells
                    .feature_id
                    .get(p)
                    .copied()
                    .unwrap_or(0);
                let is_lake =
                    result.world.pack.features.iter().any(|f| {
                        f.id == fid as u32 && f.kind == vor_core::feature::FeatureType::Lake
                    });
                h < 20 || is_lake
            })
            .collect();

    let mut no_transition = 0;
    let mut no_shared_edge = 0;
    let mut no_intersection = 0;
    let mut clipped_ok = 0;
    let mut out_of_points = 0;
    let mut single_cell = 0;

    for r in result.world.rivers.iter() {
        let path = &r.cell_path;
        if path.len() < 2 {
            single_cell += 1;
            continue;
        }
        let mut last_land: Option<(usize, usize)> = None;
        for i in 0..path.len().saturating_sub(1) {
            let a = path[i] as usize;
            let b = path[i + 1] as usize;
            let wa = is_water.get(a).copied().unwrap_or(false);
            let wb = is_water.get(b).copied().unwrap_or(false);
            if !wa && wb {
                last_land = Some((i, i + 1));
            }
        }
        let Some((li, wi)) = last_land else {
            no_transition += 1;
            continue;
        };
        // Reconstruct `raw` the same way build_river_mesh does (maps each path cell
        // through `points`). lpt/wpt must come from the path cell, NOT the path index.
        let lpt = match points.get(path[li] as usize) {
            Some(p) => *p,
            None => {
                out_of_points += 1;
                continue;
            }
        };
        let wpt = match points.get(path[wi] as usize).copied() {
            Some(p) => p,
            None => {
                out_of_points += 1;
                continue;
            }
        };
        let water_cell = path[wi] as usize;
        // Cast toward the water cell's ring polygon; keep closest edge hit.
        let ring = match vertices.cell_rings.get(water_cell) {
            Some(r) if r.len() >= 2 => r,
            _ => {
                no_shared_edge += 1;
                continue;
            }
        };
        let mut best_hit: Option<[f32; 2]> = None;
        let mut best = f32::INFINITY;
        for k in 0..ring.len() {
            let a = match ring.get(k).copied() {
                Some(t) => t as usize,
                None => continue,
            };
            let b = ring.get((k + 1) % ring.len()).map(|&t| t as usize);
            let Some(b) = b else { continue };
            let (Some(pa), Some(pb)) = (
                vertices.positions.get(a).copied(),
                vertices.positions.get(b).copied(),
            ) else {
                continue;
            };
            if let Some(hit) = segment_intersection(lpt, wpt, pa, pb) {
                let d = (hit[0] - lpt[0]).abs() + (hit[1] - lpt[1]).abs();
                if d < best {
                    best = d;
                    best_hit = Some(hit);
                }
            }
        }
        match best_hit {
            Some(_) => clipped_ok += 1,
            None => no_intersection += 1,
        }
    }

    // Every river whose path actually reaches a water cell must clip to the coast.
    // The 15 rivers that end on land (h>=20) legitimately have no mouth to clip.
    assert_eq!(
        clipped_ok + no_transition + no_shared_edge + no_intersection + out_of_points + single_cell,
        result.world.rivers.len(),
        "buckets must sum to total"
    );
    assert_eq!(
        no_intersection, 0,
        "every river reaching water must find a coastline hit"
    );
    assert_eq!(no_shared_edge, 0, "every mouth cell has a Voronoi ring");
    assert_eq!(out_of_points, 0, "every path cell resolves to a point");
    assert!(
        clipped_ok >= result.world.rivers.len() - 15,
        "all water-reaching rivers must clip: clipped={clipped_ok}"
    );
}
