//! Diagnóstico: ¿la malla del landmass (fuente del stencil) cubre TODAS las
//! celdas de tierra? Usa el CENTROIDE del ring de cada celda (punto central de
//! la celda), que debe caer dentro de la máscara. Agrupa las celdas sin cubrir
//! por feature para distinguir grietas en la costa de features omitidas enteras.

use std::collections::BTreeMap;

use vor_core::feature::FeatureType;
use vor_import::mapfile::{raw, Loader};
use vor_render::coastline::{build_fractal_landmass_mesh, FractalSettings};
use vor_render::mesh::build_pack_mesh;

const SORVIK_MAP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vor-import/tests/reference/Sorvik-2026-07-24-23-39.map"
);

fn point_in_triangle(px: f32, py: f32, tri: &[[f32; 2]; 3]) -> bool {
    let d1 =
        (px - tri[2][0]) * (tri[0][1] - tri[2][1]) - (py - tri[2][1]) * (tri[0][0] - tri[2][0]);
    let d2 =
        (px - tri[0][0]) * (tri[1][1] - tri[0][1]) - (py - tri[0][1]) * (tri[1][0] - tri[0][0]);
    let d3 =
        (px - tri[1][0]) * (tri[2][1] - tri[1][1]) - (py - tri[1][1]) * (tri[2][0] - tri[1][0]);
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

fn ring_centroid(vertices: &vor_core::voronoi::VoronoiVertices, ring: &[u32]) -> [f32; 2] {
    let pts: Vec<[f32; 2]> = ring
        .iter()
        .filter_map(|&t| vertices.positions.get(t as usize).copied())
        .collect();
    if pts.len() < 3 {
        return pts.first().copied().unwrap_or([0.0, 0.0]);
    }
    // Shoelace centroid (handles convex & concave simple polygons)
    let mut area = 0.0f32;
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    for i in 0..pts.len() {
        let j = (i + 1) % pts.len();
        let cross = pts[i][0] * pts[j][1] - pts[j][0] * pts[i][1];
        area += cross;
        cx += (pts[i][0] + pts[j][0]) * cross;
        cy += (pts[i][1] + pts[j][1]) * cross;
    }
    if area.abs() < 1e-9 {
        return pts[0];
    }
    [cx / (3.0 * area), cy / (3.0 * area)]
}

#[test]
fn landmass_mask_covers_all_land_cell_centroids() {
    let bytes = std::fs::read(SORVIK_MAP_PATH).expect("Sorvik.map");
    let raw = raw::parse(&bytes).expect("raw parse");
    let loaded = Loader::load(&raw).expect("loader");
    let world = &loaded.world;
    let pack = &world.pack;
    let n = pack.points_n();

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
    let land_cells = is_water.iter().filter(|&&w| !w).count();

    let mesh = build_fractal_landmass_mesh(
        &pack.vertices,
        &pack.features,
        world.grid.width,
        world.grid.height,
        |_| [1.0, 1.0, 1.0, 1.0],
        &FractalSettings {
            seed: world.header.seed.parse::<u64>().unwrap_or(0),
            ..Default::default()
        },
    );

    let tris: Vec<[[f32; 2]; 3]> = mesh
        .indices
        .chunks_exact(3)
        .map(|c| {
            [
                mesh.vertices[c[0] as usize].pos,
                mesh.vertices[c[1] as usize].pos,
                mesh.vertices[c[2] as usize].pos,
            ]
        })
        .collect();

    let mut uncovered = 0usize;
    let mut by_feature: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    let mut empty_rings_land = 0usize;
    let mut empty_rings_total = 0usize;
    for p in 0..n {
        let ring = match pack.vertices.cell_rings.get(p) {
            Some(r) if !r.is_empty() => r,
            _ => {
                if !is_water[p] {
                    empty_rings_land += 1;
                }
                empty_rings_total += 1;
                continue;
            }
        };
        if is_water[p] {
            continue;
        }
        let center = ring_centroid(&pack.vertices, ring);
        let mut covered = false;
        for tri in &tris {
            if point_in_triangle(center[0], center[1], tri) {
                covered = true;
                break;
            }
        }
        if !covered {
            uncovered += 1;
            let fid = pack.cells.feature_id.get(p).copied().unwrap_or(0) as u32;
            by_feature.entry(fid).or_default().push(p);
        }
    }

    println!(
        "land cells: {land_cells}, mask triangles: {}, uncovered centroids: {uncovered} ({:.2}% of land), empty rings land: {empty_rings_land} / total {empty_rings_total}",
        tris.len(),
        uncovered as f32 * 100.0 / land_cells.max(1) as f32
    );

    // Fill coverage: does build_pack_mesh produce triangles covering every land cell?
    let fill_mesh = build_pack_mesh(&pack.vertices, n, |p| {
        if is_water[p] {
            [0.0, 0.0, 0.0, 1.0]
        } else {
            [1.0, 1.0, 1.0, 1.0]
        }
    });
    let fill_tris: Vec<[[f32; 2]; 3]> = fill_mesh
        .indices
        .chunks_exact(3)
        .map(|c| {
            [
                fill_mesh.vertices[c[0] as usize].pos,
                fill_mesh.vertices[c[1] as usize].pos,
                fill_mesh.vertices[c[2] as usize].pos,
            ]
        })
        .collect();
    let mut fill_missing = 0usize;
    for p in 0..n {
        if is_water[p] {
            continue;
        }
        let ring = match pack.vertices.cell_rings.get(p) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        let center = ring_centroid(&pack.vertices, ring);
        let mut covered = false;
        for tri in &fill_tris {
            if point_in_triangle(center[0], center[1], tri) {
                covered = true;
                break;
            }
        }
        if !covered {
            fill_missing += 1;
        }
    }
    println!(
        "fill mesh triangles: {}, land cells missing from fill: {fill_missing}",
        fill_tris.len()
    );
    for (fid, cells) in &by_feature {
        let feat = world.pack.features.iter().find(|f| f.id == *fid);
        let kind = feat.map(|f| format!("{:?}", f.kind)).unwrap_or_default();
        let is_land = feat.map(|f| f.is_land).unwrap_or(false);
        let perim = feat.map(|f| f.perimeter_vertices.len()).unwrap_or(0);
        let n_cells = feat.map(|f| f.cell_count).unwrap_or(0);
        println!(
            "  feature {fid} ({kind}, is_land={is_land}, perim={perim}, cells={n_cells}): {} uncovered",
            cells.len()
        );
    }

    assert!(
        uncovered as f32 / (land_cells.max(1) as f32) < 0.005,
        "more than 0.5% of land cell centroids not covered by the mask"
    );
}
