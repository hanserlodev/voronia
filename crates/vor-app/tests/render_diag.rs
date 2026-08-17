//! Diagnóstico visual: reproduce el render de biomes recortado por la landmask
//! (el stencil) como SVG, para inspeccionar el resultado sin GPU.
//!
//! Genera `/tmp/voronia_diag_biome.svg` con:
//!  - fondo océano
//!  - landmass (capa 0, blanca) = fuente del stencil
//!  - fill de biomes + water gap recortados por el landmass (clip)
//!  - stroke de costa (linea roja) para ver la relacion con el recorte

use std::fmt::Write;

use vor_core::feature::FeatureType;
use vor_import::mapfile::{raw, Loader};
use vor_render::biome::{biome_colors_from_catalog, build_biome_coast_fill, build_biome_mesh};
use vor_render::coastline::{build_fractal_landmass_mesh, FractalSettings};
use vor_render::water_gap::append_water_gap;

const SORVIK_MAP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vor-import/tests/reference/Sorvik-2026-07-24-23-39.map"
);

fn hex(c: [f32; 4]) -> String {
    let to8 = |x: f32| -> u8 { (x.clamp(0.0, 1.0) * 255.0).round() as u8 };
    format!("#{:02x}{:02x}{:02x}", to8(c[0]), to8(c[1]), to8(c[2]))
}

fn pt_in_tri(pt: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let d1 = (pt[0] - c[0]) * (a[1] - c[1]) - (pt[1] - c[1]) * (a[0] - c[0]);
    let d2 = (pt[0] - a[0]) * (b[1] - a[1]) - (pt[1] - a[1]) * (b[0] - a[0]);
    let d3 = (pt[0] - b[0]) * (c[1] - b[1]) - (pt[1] - b[1]) * (c[0] - b[0]);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

fn point_in_cell(vertices: &vor_core::voronoi::VoronoiVertices, ring: &[u32], p: [f32; 2]) -> bool {
    let pts: Vec<[f32; 2]> = ring
        .iter()
        .filter_map(|&t| vertices.positions.get(t as usize).copied())
        .collect();
    if pts.len() < 3 {
        return false;
    }
    let mut inside = false;
    let n = pts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let (xi, yi) = (pts[i][0], pts[i][1]);
        let (xj, yj) = (pts[j][0], pts[j][1]);
        let intersects = (yi > p[1]) != (yj > p[1])
            && p[0] < (xj - xi) * (p[1] - yi) / (yj - yi).abs().max(1e-9) + xi;
        if intersects {
            inside = !inside;
        }
    }
    inside
}

fn ring_distance(
    vertices: &vor_core::voronoi::VoronoiVertices,
    ring: &[u32],
    p: [f32; 2],
) -> Option<f32> {
    let pts: Vec<[f32; 2]> = ring
        .iter()
        .filter_map(|&t| vertices.positions.get(t as usize).copied())
        .collect();
    if pts.len() < 2 {
        return None;
    }
    let mut best = f32::INFINITY;
    for i in 0..pts.len() {
        let j = (i + 1) % pts.len();
        let a = pts[i];
        let b = pts[j];
        let abx = b[0] - a[0];
        let aby = b[1] - a[1];
        let l2 = abx * abx + aby * aby;
        let t = ((p[0] - a[0]) * abx + (p[1] - a[1]) * aby).clamp(0.0, l2.max(1e-9)) / l2.max(1e-9);
        let px = a[0] + abx * t;
        let py = a[1] + aby * t;
        let d = ((p[0] - px).powi(2) + (p[1] - py).powi(2)).sqrt();
        best = best.min(d);
    }
    Some(best)
}

#[test]
fn dump_biome_render_svg() {
    let bytes = std::fs::read(SORVIK_MAP_PATH).expect("Sorvik.map");
    let raw = raw::parse(&bytes).expect("raw parse");
    let loaded = Loader::load(&raw).expect("loader");
    let world = &loaded.world;
    let pack = &world.pack;
    let n = pack.points_n();
    let verts = &pack.vertices;

    let is_water: Vec<bool> = (0..n)
        .map(|p| {
            let h = pack.cells.height.get(p).copied().unwrap_or(0);
            let fid = pack.cells.feature_id.get(p).copied().unwrap_or(0);
            let is_lake = world
                .pack
                .features
                .iter()
                .any(|f| f.id == fid as u32 && f.kind == FeatureType::Lake);
            h < 20 || is_lake
        })
        .collect();

    let mask_mesh = build_fractal_landmass_mesh(
        verts,
        &pack.features,
        world.grid.width,
        world.grid.height,
        |_| [1.0, 1.0, 1.0, 1.0],
        &FractalSettings {
            seed: world.header.seed.parse::<u64>().unwrap_or(0),
            ..Default::default()
        },
    );

    let biome_colors = biome_colors_from_catalog(&world.biomes);
    let coast_fill = build_biome_coast_fill(&mask_mesh, pack, &is_water, &biome_colors);
    println!(
        "coast fill: {}v/{}i",
        coast_fill.vertices.len(),
        coast_fill.indices.len()
    );
    let mut biome_mesh = build_biome_mesh(pack, &biome_colors);
    if !coast_fill.indices.is_empty() {
        let shift = coast_fill.vertices.len() as u32;
        biome_mesh
            .vertices
            .splice(0..0, coast_fill.vertices.clone());
        biome_mesh.indices.splice(0..0, coast_fill.indices.clone());
        for idx in biome_mesh.indices.iter_mut().skip(coast_fill.indices.len()) {
            *idx += shift;
        }
    }
    append_water_gap(&mut biome_mesh, pack, &is_water, |p| {
        let bi = pack.cells.biome.get(p).copied().unwrap_or(0) as usize;
        biome_colors
            .get(bi)
            .copied()
            .unwrap_or([0.0, 0.0, 0.0, 1.0])
    });

    let mut mask_mesh = build_fractal_landmass_mesh(
        verts,
        &pack.features,
        world.grid.width,
        world.grid.height,
        |_| [1.0, 1.0, 1.0, 1.0],
        &FractalSettings {
            seed: world.header.seed.parse::<u64>().unwrap_or(0),
            ..Default::default()
        },
    );
    let land_cells_mask = vor_render::mesh::build_land_cells_mask_mesh(verts, n, |p| !is_water[p]);
    println!(
        "land cells mask: {}v/{}i",
        land_cells_mask.vertices.len(),
        land_cells_mask.indices.len()
    );
    if !land_cells_mask.indices.is_empty() {
        let shift = land_cells_mask.vertices.len() as u32;
        mask_mesh
            .vertices
            .splice(0..0, land_cells_mask.vertices.clone());
        mask_mesh
            .indices
            .splice(0..0, land_cells_mask.indices.clone());
        for idx in mask_mesh
            .indices
            .iter_mut()
            .skip(land_cells_mask.indices.len())
        {
            *idx += shift;
        }
        mask_mesh.bounds_min = land_cells_mask.bounds_min;
        mask_mesh.bounds_max = land_cells_mask.bounds_max;
    }

    let (min_x, max_x, min_y, max_y) = {
        let mut mn = [f32::INFINITY; 2];
        let mut mx = [f32::NEG_INFINITY; 2];
        for &p in &pack.points {
            mn[0] = mn[0].min(p[0]);
            mn[1] = mn[1].min(p[1]);
            mx[0] = mx[0].max(p[0]);
            mx[1] = mx[1].max(p[1]);
        }
        (mn[0], mx[0], mn[1], mx[1])
    };
    let w = (max_x - min_x).max(1.0);
    let h = (max_y - min_y).max(1.0);

    let mut svg = String::with_capacity(4 * 1024 * 1024);
    let _ = write!(
        svg,
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="{min_x} {min_y} {w} {h}">
<rect x="{min_x}" y="{min_y}" width="{w}" height="{h}" fill="#05050d"/>
<clipPath id="land">{}</clipPath>
"##,
        {
            let mut s = String::new();
            for c in mask_mesh.indices.chunks_exact(3) {
                let a = mask_mesh.vertices[c[0] as usize].pos;
                let b = mask_mesh.vertices[c[1] as usize].pos;
                let d = mask_mesh.vertices[c[2] as usize].pos;
                let _ = write!(
                    s,
                    "<polygon points=\"{},{},{},{},{},{}\"/>",
                    a[0], a[1], b[0], b[1], d[0], d[1]
                );
            }
            s
        }
    );

    // landmass base (blanca)
    let _ = write!(svg, "<g clip-path=\"url(#land)\"><polygon points=\"{min_x},{min_y} {max_x},{min_y} {max_x},{max_y} {min_x},{max_y}\" fill=\"#ffffff\"/></g>\n");

    // fill de biomes + water gap recortados
    let _ = write!(svg, "<g clip-path=\"url(#land)\">{}</g>\n", {
        let mut s = String::new();
        for c in biome_mesh.indices.chunks_exact(3) {
            let a = biome_mesh.vertices[c[0] as usize].pos;
            let b = biome_mesh.vertices[c[1] as usize].pos;
            let d = biome_mesh.vertices[c[2] as usize].pos;
            let col = hex(biome_mesh.vertices[c[0] as usize].color);
            let _ = write!(
                s,
                "<polygon points=\"{},{},{},{},{},{}\" fill=\"{col}\" stroke=\"none\"/>",
                a[0], a[1], b[0], b[1], d[0], d[1]
            );
        }
        s
    });

    // stroke de costa de cada feature (rojo) para comparar con el recorte
    let _ = write!(
        svg,
        "<g fill=\"none\" stroke=\"#ff0000\" stroke-width=\"1.2\">{}</g>\n",
        {
            let mut s = String::new();
            for feat in &world.pack.features {
                if !feat.is_land || feat.kind == FeatureType::Lake {
                    continue;
                }
                let mut path = String::new();
                for (i, &t) in feat.perimeter_vertices.iter().enumerate() {
                    let pos = verts.positions.get(t as usize).copied().unwrap_or([0.0; 2]);
                    let cmd = if i == 0 { "M" } else { "L" };
                    let _ = write!(path, "{cmd}{},{},", pos[0], pos[1]);
                }
                if !path.is_empty() {
                    path.pop();
                    let _ = write!(s, "<path d=\"{path} Z\"/>");
                }
            }
            s
        }
    );

    let _ = write!(svg, "</svg>\n");

    let out = "/tmp/voronia_diag_biome.svg";
    std::fs::write(out, &svg).expect("write svg");
    println!("wrote {out} ({} bytes)", svg.len());

    // --- Numerical diagnostics on the rendered geometry ---
    // Precompute land rings + bboxes for fast point-in-cell tests.
    let mut land_rings: Vec<(Vec<[f32; 2]>, [f32; 4])> = Vec::new();
    for p in 0..n {
        if is_water[p] {
            continue;
        }
        if let Some(r) = pack.vertices.cell_rings.get(p) {
            let pts: Vec<[f32; 2]> = r
                .iter()
                .filter_map(|&t| pack.vertices.positions.get(t as usize).copied())
                .collect();
            if pts.len() < 3 {
                continue;
            }
            let mut bb = [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ];
            for pt in &pts {
                bb[0] = bb[0].min(pt[0]);
                bb[1] = bb[1].min(pt[1]);
                bb[2] = bb[2].max(pt[0]);
                bb[3] = bb[3].max(pt[1]);
            }
            land_rings.push((pts, bb));
        }
    }
    let point_in_any_land = |pt: [f32; 2]| {
        land_rings.iter().any(|(pts, bb)| {
            pt[0] >= bb[0] && pt[0] <= bb[2] && pt[1] >= bb[1] && pt[1] <= bb[3] && {
                let mut inside = false;
                let npts = pts.len();
                for i in 0..npts {
                    let j = (i + 1) % npts;
                    let (xi, yi) = (pts[i][0], pts[i][1]);
                    let (xj, yj) = (pts[j][0], pts[j][1]);
                    if (yi > pt[1]) != (yj > pt[1])
                        && pt[0] < (xj - xi) * (pt[1] - yi) / (yj - yi).abs().max(1e-9) + xi
                    {
                        inside = !inside;
                    }
                }
                inside
            }
        })
    };
    let mut coast_samples_outside = 0usize;
    let mut coast_samples_total = 0usize;
    let mut max_outside_d = 0.0f32;
    for c in mask_mesh.indices.chunks_exact(3) {
        let a = mask_mesh.vertices[c[0] as usize].pos;
        let b = mask_mesh.vertices[c[1] as usize].pos;
        let d = mask_mesh.vertices[c[2] as usize].pos;
        for e in [[a, b], [b, d], [d, a]] {
            let (p0, p1) = (e[0], e[1]);
            for k in 1..4 {
                let t = k as f32 / 4.0;
                let pt = [p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t];
                if !point_in_any_land(pt) {
                    coast_samples_outside += 1;
                    let d = land_rings
                        .iter()
                        .filter_map(|(pts, _)| {
                            let mut best = f32::INFINITY;
                            for i in 0..pts.len() {
                                let j = (i + 1) % pts.len();
                                let a = pts[i];
                                let b = pts[j];
                                let abx = b[0] - a[0];
                                let aby = b[1] - a[1];
                                let l2 = abx * abx + aby * aby;
                                let t = ((pt[0] - a[0]) * abx + (pt[1] - a[1]) * aby)
                                    .clamp(0.0, l2.max(1e-9))
                                    / l2.max(1e-9);
                                let px = a[0] + abx * t;
                                let py = a[1] + aby * t;
                                best =
                                    best.min(((pt[0] - px).powi(2) + (pt[1] - py).powi(2)).sqrt());
                            }
                            Some(best)
                        })
                        .fold(f32::INFINITY, f32::min);
                    if d.is_finite() && d > max_outside_d {
                        max_outside_d = d;
                    }
                }
                coast_samples_total += 1;
            }
        }
    }
    println!(
        "fractal coast samples: {coast_samples_total}, outside land cells: {coast_samples_outside} ({:.1}%), max dist outside: {max_outside_d:.2}",
        coast_samples_outside as f32 * 100.0 / coast_samples_total.max(1) as f32
    );

    // Raster-style area analysis: grid over the map, classify each sample.
    let grid = 150usize;
    let (mut mask_only, mut land_only, mut both, mut neither) = (0usize, 0usize, 0usize, 0usize);
    let mask_tris: Vec<[[f32; 2]; 3]> = mask_mesh
        .indices
        .chunks_exact(3)
        .map(|c| {
            [
                mask_mesh.vertices[c[0] as usize].pos,
                mask_mesh.vertices[c[1] as usize].pos,
                mask_mesh.vertices[c[2] as usize].pos,
            ]
        })
        .collect();
    let pt_in_mask = |pt: [f32; 2]| mask_tris.iter().any(|t| pt_in_tri(pt, t[0], t[1], t[2]));
    for gy in 0..grid {
        let y = min_y + h * (gy as f32 + 0.5) / grid as f32;
        for gx in 0..grid {
            let x = min_x + w * (gx as f32 + 0.5) / grid as f32;
            let pt = [x, y];
            let in_mask = pt_in_mask(pt);
            let in_land = point_in_any_land(pt);
            match (in_mask, in_land) {
                (true, true) => both += 1,
                (true, false) => mask_only += 1,
                (false, true) => land_only += 1,
                (false, false) => neither += 1,
            }
        }
    }
    let tot = (both + mask_only + land_only + neither).max(1) as f32;
    println!(
        "area: mask+land {both} ({:.1}%), mask only {mask_only} ({:.1}%), land only {land_only} ({:.1}%), neither {neither} ({:.1}%)",
        both as f32 * 100.0 / tot,
        mask_only as f32 * 100.0 / tot,
        land_only as f32 * 100.0 / tot,
        neither as f32 * 100.0 / tot
    );

    // After the coast-fill fix: does the merged biome mesh (coast fill + cells)
    // now cover the mask-only area too? The white halo should be gone.
    let merged_tris: Vec<[[f32; 2]; 3]> = biome_mesh
        .indices
        .chunks_exact(3)
        .map(|c| {
            [
                biome_mesh.vertices[c[0] as usize].pos,
                biome_mesh.vertices[c[1] as usize].pos,
                biome_mesh.vertices[c[2] as usize].pos,
            ]
        })
        .collect();
    let pt_in_merged = |pt: [f32; 2]| merged_tris.iter().any(|t| pt_in_tri(pt, t[0], t[1], t[2]));
    let (mut merged_both, mut merged_mask_only) = (0usize, 0usize);
    for gy in 0..grid {
        let y = min_y + h * (gy as f32 + 0.5) / grid as f32;
        for gx in 0..grid {
            let x = min_x + w * (gx as f32 + 0.5) / grid as f32;
            let pt = [x, y];
            let in_mask = pt_in_mask(pt);
            let in_merged = pt_in_merged(pt);
            match (in_mask, in_merged) {
                (true, true) => merged_both += 1,
                (true, false) => merged_mask_only += 1,
                _ => {}
            }
        }
    }
    println!(
        "merged: mask covered by biome {merged_both} ({:.1}%), mask only (unpainted halo) {merged_mask_only} ({:.1}%)",
        merged_both as f32 * 100.0 / tot,
        merged_mask_only as f32 * 100.0 / tot
    );

    // ---- Small islands analysis ----
    // Land cells not covered by the fractal landmask = islands that "fight the
    // sea" and leave holes. Count per-feature: how many of its perimeter
    // vertices fall inside the mask, and how many land cells it owns.
    let mask_tris_ref = &mask_tris;
    let mut small_land: Vec<(usize, usize, usize, f32)> = Vec::new();
    for feat in &pack.features {
        if !feat.is_land || feat.kind == FeatureType::Lake {
            continue;
        }
        let own_cells = (0..n)
            .filter(|&p| {
                let fid = pack.cells.feature_id.get(p).copied().unwrap_or(0);
                fid == feat.id as u16
            })
            .count();
        if own_cells == 0 {
            continue;
        }
        let mut in_mask = 0usize;
        let mut total = 0usize;
        for &vi in &feat.perimeter_vertices {
            if let Some(pos) = verts.positions.get(vi as usize) {
                total += 1;
                if mask_tris_ref
                    .iter()
                    .any(|t| pt_in_tri(*pos, t[0], t[1], t[2]))
                {
                    in_mask += 1;
                }
            }
        }
        if total > 0 {
            let frac = in_mask as f32 / total as f32;
            if own_cells <= 12 || frac < 0.9 {
                small_land.push((own_cells, in_mask, total, frac));
            }
        }
    }
    small_land.sort_by_key(|(c, _, _, _)| *c);
    println!("small/edge land features: {}", small_land.len());
    for (cells, in_m, tot, frac) in small_land.iter().take(20) {
        println!(
            "  feature: own_cells={cells}, mask coverage {in_m}/{tot} ({:.0}%)",
            frac * 100.0
        );
    }
    let lost: usize = small_land.iter().map(|(c, _, _, _)| c).sum();
    println!(
        "total land cells in small/edge features: {lost} ({} of total land cells)",
        (0..n).filter(|&p| !is_water[p]).count()
    );

    // Every land cell centroid must be inside the union mask mesh.
    let mask_tris2: Vec<[[f32; 2]; 3]> = mask_mesh
        .indices
        .chunks_exact(3)
        .map(|c| {
            [
                mask_mesh.vertices[c[0] as usize].pos,
                mask_mesh.vertices[c[1] as usize].pos,
                mask_mesh.vertices[c[2] as usize].pos,
            ]
        })
        .collect();
    let mut uncovered = 0usize;
    for p in 0..n {
        if is_water[p] {
            continue;
        }
        let ring = match verts.cell_rings.get(p) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        let pts: Vec<[f32; 2]> = ring
            .iter()
            .filter_map(|&t| verts.positions.get(t as usize).copied())
            .collect();
        if pts.len() < 3 {
            continue;
        }
        let (mut ax, mut ay) = (0.0f32, 0.0f32);
        let mut area2 = 0.0f32;
        for i in 0..pts.len() {
            let (x0, y0) = (pts[i][0], pts[i][1]);
            let (x1, y1) = (pts[(i + 1) % pts.len()][0], pts[(i + 1) % pts.len()][1]);
            let cross = x0 * y1 - x1 * y0;
            area2 += cross;
            ax += (x0 + x1) * cross;
            ay += (y0 + y1) * cross;
        }
        if area2.abs() < 1e-6 {
            continue;
        }
        let c = [ax / (3.0 * area2), ay / (3.0 * area2)];
        if !mask_tris2.iter().any(|t| pt_in_tri(c, t[0], t[1], t[2])) {
            uncovered += 1;
            println!("  uncovered land cell centroid: p={p} at {:?}", c);
        }
    }
    println!("land cell centroids uncovered by union mask: {uncovered}");
}
