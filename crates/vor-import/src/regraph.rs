//! Porte bit-exacto de `reGraph` (`azgaar-fmg/public/main.js:1157-1209`).
//!
//! Repacking del grid jitterizado (`grid`) al pack con celdas reducidas y puntos
//! extra en las costas. Produce los `pack.cells` y `pack.vertices` finales — es la
//! malla sobre la que operan los subsistemas de cultura/estado/bioma/etc.
//!
//! ## Bit-exactitud — por qué importa este porte
//!
//! Si Voronia produce una malla de pack distinta (aunque sea por 1 celda o por orden
//! distinto en `newCells.p`), el mapeo `pack.cells.g[packId] → gridId` queda distinto,
//! y todos los atributos que Azgaar serializa indexados por id de pack (bioma, state,
//! burg, ...) se aplican a celdas equivocadas — bug silencioso sin runtime error.
//! (`vor-core/src/pack.rs` §"Critically").
//!
//! `reGraph` toma ~9 parámetros (puntos/grid/boundary/atributos). Mantenemos la
//! firma simple en vez de introducir un struct wrapper — los parámetros reflejan 1-a-1
//! al algoritmo Azgaar.

#![allow(clippy::too_many_arguments)]

use crate::geometry::voronoi::{calculate_voronoi, Voronoi};
use vor_core::cells::PackCells;
use vor_core::pack::Pack;
use vor_core::{feature::FeatureType, voronoi::VoronoiVertices};

/// Reproduce `reGraph` de Azgaar (`main.js:1157-1209`).
///
/// **Input** y sus roles:
/// - `grid_points`: `grid.points` (10000 en Brample).
/// - `grid_boundary`: `grid.boundary` (200 en Brample).
/// - `grid_voronoi`: topología del grid (output de `calculate_voronoi(allPoints, pointsN)`
///   previamente computada). Contiene `cells.c` (vecinos de cada celda), `cells.b` (border flag).
/// - `grid_height`: `grid.cells.h` (slot `[7]`).
/// - `grid_water_type`: `grid.cells.t` (slot `[10]`). -2 lago, -1 agua costera, 1 tierra costera, otro.
/// - `grid_feature_id`: `grid.cells.f` (slot `[9]`).
/// - `grid_features`: `grid.features` (vec de Feature; se indexa por `grid_feature_id[i]`).
///
/// **Output**: `(Pack, new_points_f64)` — el `Pack` con todo el modelo `vor-core`, **más**
/// los `new_points` en f64. Esto último es útil cuando el caller quiere bit-exactitud
/// completa: `Pack.points` está en `f32` (cap fijo del model), pero el cálculo interno
/// de `reGraph` opera en f64 (igual que el JS de Azgaar). Para validar bit-exactitud
/// contra el JS, comparar el `new_points_f64` — no `pack.points` (que pierde ~1e-5 de
/// precisión en coords > 100 por el cast f32).
///
/// El caller es responsable de poblar los demás campos de `PackCells` (biome, culture,
/// state, ...) cuando se importen desde el `.map`.
pub fn re_graph(
    grid_points: &[[f64; 2]],
    grid_boundary: &[[f64; 2]],
    grid_voronoi: &Voronoi,
    grid_height: &[u8],
    grid_water_type: &[i8],
    grid_feature_id: &[u16],
    grid_features_kind: &[FeatureType],
    spacing: f64,
) -> (Pack, Vec<[f64; 2]>) {
    let _ = grid_voronoi; // topología del grid consumida abajo via grid_voronoi.cells.c/b
    let points_n = grid_points.len();

    // `newCells = { p: [], g: [], h: [] }` — puntos del pack antes del segundo `calculateVoronoi`.
    let mut new_points: Vec<[f64; 2]> = Vec::new();
    let mut new_g: Vec<u32> = Vec::new();
    let mut new_h: Vec<u8> = Vec::new();
    let spacing2 = spacing * spacing;

    // `for (const i of gridCells.i)` — `i` itera los ids 0..nPoints en orden ascendente.
    for i in 0..points_n {
        let i = i as u32;
        let height = grid_height[i as usize];
        let typ = grid_water_type[i as usize];

        // Filtro 1: deep ocean non-coastal. `height < 20 && type !== -1 && type !== -2`.
        if height < 20 && typ != -1 && typ != -2 {
            continue;
        }
        // Filtro 2: lago no-costero. `type === -2 && (i % 4 === 0 || features[gridCells.f[i]].type === "lake")`.
        if typ == -2
            && (i.is_multiple_of(4)
                || grid_features_kind[grid_feature_id[i as usize] as usize] == FeatureType::Lake)
        {
            continue;
        }

        let [x, y] = grid_points[i as usize];
        add_new_point(i, x, y, height, &mut new_points, &mut new_g, &mut new_h);

        // Puntos extra para cells costeras. `if (type === 1 || type === -1)`.
        if typ == 1 || typ == -1 {
            // `if (gridCells.b[i]) continue;` —.skip near-border cells.
            if grid_voronoi.cells.b[i as usize] != 0 {
                continue;
            }
            // Iterar `gridCells.c[i]` — vecinos de la celda i (interiores, boundary filtrados).
            let neighbors = &grid_voronoi.cells.c[i as usize];
            for &e in neighbors {
                // `if (i > e) return;` — solo procesa cuando i < e (cada par una vez).
                if i > e {
                    continue;
                }
                // `if (gridCells.t[e] === type)` — mismo tipo de cell (misma costa).
                let e_type = grid_water_type[e as usize];
                if e_type != typ {
                    continue;
                }
                let [ex, ey] = grid_points[e as usize];
                // `const dist2 = (y - points[e][1]) ** 2 + (x - points[e][0]) ** 2;`
                let dist2 = (y - ey).powi(2) + (x - ex).powi(2);
                if dist2 < spacing2 {
                    continue;
                }
                // Punto medio, rn a 1 decimal.
                // `const x1 = rn((x + points[e][0]) / 2, 1);`
                let x1 = crate::numbers::rn((x + ex) / 2.0, 1);
                let y1 = crate::numbers::rn((y + ey) / 2.0, 1);
                add_new_point(i, x1, y1, height, &mut new_points, &mut new_g, &mut new_h);
            }
        }
    }

    // `calculateVoronoi(newCells.p, grid.boundary)` — segundo Voronoi.
    let all_points_n = new_points.len();
    let mut all_points = new_points.clone();
    all_points.extend(grid_boundary.iter().cloned());
    let delaunay = crate::geometry::delaunay::from_pairs(&all_points);
    let voronoi = calculate_voronoi(&delaunay, &all_points, all_points_n as u32);

    // `pack.cells.area`: para cada cellId, area del polígono = abs(polygonArea(cells.v[cellId].map(v => vertices.p[v]))), capped a UINT16_MAX.
    // Azgaar: `Math.abs(d3.polygonArea(getPackPolygon(cellId)))` y luego `Math.min(area, TYPED_ARRAY_MAX.UINT16)`.
    // UINT16_MAX = 65535. Clamp directo en f64 + cast a u16 (no puede saturar): clampRaw sea min(raw, 65535.0).
    let n_pack_cells = voronoi.cells.b.len();
    let mut area_px = Vec::with_capacity(n_pack_cells);
    for cell_id in 0..n_pack_cells {
        let verts: Vec<[f64; 2]> = voronoi.cells.v[cell_id]
            .iter()
            .map(|&t| voronoi.vertices.p[t as usize])
            .collect();
        let raw = polygon_area_signed(&verts).abs();
        // `Math.min(area, TYPED_ARRAY_MAX.UINT16)` = 65535. Cast a u16 sin saturar (raw clamp
        // garantiza que raw <= 65535.0 → cast as u16 seguro).
        let capped = raw.min(u16::MAX as f64) as u16;
        area_px.push(capped);
    }

    // Poblar `PackCells`. Solo `grid_id`, `height`, `area_px` — los demás quedan vacíos
    // (la parser los completa) y los defaults de `Default` para `Vec<T>` son vacíos.
    let pack_cells = PackCells {
        grid_id: new_g,
        height: new_h,
        area_px,
        ..Default::default()
    };

    // Convertir `voronoi.vertices` al formato `vor-core::VoronoiVertices` (i32 con -1 = EMPTY).
    let vertices = voronoi_to_vor_core(&voronoi);

    let pack = Pack {
        points: new_points
            .iter()
            .map(|&[x, y]| [x as f32, y as f32])
            .collect(),
        boundary: grid_boundary
            .iter()
            .map(|&[x, y]| [x as f32, y as f32])
            .collect(),
        cells: pack_cells,
        vertices,
        features: Vec::new(), // features se completan al poblar desde el .map
    };
    (pack, new_points)
}

/// `addNewPoint(i, x, y, height)`: push triple (p, g, h). Closure en JS; acá lo
/// desdoblamos pasando los Vecs mutables por separado (Rust no tiene closures con
/// captura mutable cómoda en este loop shape).
#[inline]
fn add_new_point(
    i: u32,
    x: f64,
    y: f64,
    height: u8,
    new_points: &mut Vec<[f64; 2]>,
    new_g: &mut Vec<u32>,
    new_h: &mut Vec<u8>,
) {
    new_points.push([x, y]);
    new_g.push(i);
    new_h.push(height);
}

/// `d3.polygonArea` (`d3-polygon/src/area.js`) — shoelace signed area.
/// `area = sum(a[1]*b[0] - a[0]*b[1]) / 2` donde `b = polygon[i]`, `a = polygon[i-1]`.
/// `area/2` en f64. JS no aplica `Math.abs` (lo hace el caller).
#[inline]
fn polygon_area_signed(polygon: &[[f64; 2]]) -> f64 {
    let n = polygon.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    // a = polygon[n-1] para i=0; luego a = b (polygon[i-1]); b = polygon[i].
    let mut b = polygon[n - 1];
    for item in polygon.iter() {
        let a = b;
        b = *item;
        area += a[1] * b[0] - a[0] * b[1];
    }
    area / 2.0
}

/// Convierte `vor-import::Voronoi::vertices` a `vor-core::VoronoiVertices`.
///
/// `vertices.p` (`[f64;2]` con `floor` ya truncado) se castea a `f32` (cap fijo.
/// `vertices.v` (`[usize;3]` con `EMPTY` para borde) se convierte a `[i32;3]` con `-1` para
/// `EMPTY` (interfaz de Azgaar que `vor-core` ya usa).
fn voronoi_to_vor_core(voronoi: &Voronoi) -> VoronoiVertices {
    use crate::geometry::delaunay::EMPTY;

    let n_tri = voronoi.vertices.p.len();
    let mut positions = Vec::with_capacity(n_tri);
    let mut adjacent_cells = Vec::with_capacity(n_tri);
    let mut adjacent_vertices = Vec::with_capacity(n_tri);

    for t in 0..n_tri {
        positions.push([
            voronoi.vertices.p[t][0] as f32,
            voronoi.vertices.p[t][1] as f32,
        ]);
        // `vertices.c[t]` ya es `[u32;3]` (cell ids).
        adjacent_cells.push([
            voronoi.vertices.c[t][0] as i32,
            voronoi.vertices.c[t][1] as i32,
            voronoi.vertices.c[t][2] as i32,
        ]);
        // `vertices.v[t]` es `[usize;3]` con EMPTY para borde → convertir a -1.
        adjacent_vertices.push([
            if voronoi.vertices.v[t][0] == EMPTY {
                -1
            } else {
                voronoi.vertices.v[t][0] as i32
            },
            if voronoi.vertices.v[t][1] == EMPTY {
                -1
            } else {
                voronoi.vertices.v[t][1] as i32
            },
            if voronoi.vertices.v[t][2] == EMPTY {
                -1
            } else {
                voronoi.vertices.v[t][2] as i32
            },
        ]);
    }

    VoronoiVertices {
        positions,
        adjacent_cells,
        adjacent_vertices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::delaunay::from_pairs;
    use crate::geometry::place_points;
    use vor_core::feature::FeatureType;

    /// Sanity básico: si todas las cells son tierra interior con height=20, type=1, no
    /// estamos cerca de borde, y todos tienen distancia > spacing, el pack duplica
    /// (aprox) al grid por extras costeros. Como no hay costeras en sentido "vecinos
    /// con type === 1", depende del número de vecinos con `e === 1` y `i < e`.
    ///
    /// Testeo más simple: todos los puntos en océano profundo (h<20, type=otro) →
    /// descarte total → pack vacío.
    #[test]
    fn regraph_all_deep_ocean_yields_empty_pack() {
        let placed = place_points(200.0, 200.0, 100, "1");
        let grid_points: Vec<[f64; 2]> = placed.points.iter().map(|&[x, y]| [x, y]).collect();
        let grid_boundary: Vec<[f64; 2]> = placed.boundary.iter().map(|&[x, y]| [x, y]).collect();
        let n = grid_points.len();

        // Topología del grid (Voronoi) — necesaria para el input.
        let mut all_points = grid_points.clone();
        all_points.extend(grid_boundary.iter().cloned());
        let delaunay = from_pairs(&all_points);
        let grid_voronoi = calculate_voronoi(&delaunay, &all_points, n as u32);

        // h=10 (<20), type=0 (no -1, no -2) → descarte por filtro 1.
        let grid_height = vec![10u8; n];
        let grid_water_type = vec![0i8; n];
        let grid_feature_id = vec![0u16; n];
        let grid_features_kind = vec![FeatureType::Ocean];

        let (pack, new_pts) = re_graph(
            &grid_points,
            &grid_boundary,
            &grid_voronoi,
            &grid_height,
            &grid_water_type,
            &grid_feature_id,
            &grid_features_kind,
            placed.spacing,
        );

        assert_eq!(pack.points.len(), 0, "todo descartado, sin points");
        assert!(new_pts.is_empty());
        assert!(pack.cells.grid_id.is_empty());
        assert!(pack.cells.height.is_empty());
        assert!(pack.cells.area_px.is_empty());
    }

    /// All cells interior land (h=50, type=otro positivo): no se descartan, pero no se
    /// agregan extras porque type no es costera.
    #[test]
    fn regraph_all_interior_land_yields_one_point_per_cell() {
        let placed = place_points(200.0, 200.0, 100, "1");
        let grid_points: Vec<[f64; 2]> = placed.points.iter().map(|&[x, y]| [x, y]).collect();
        let grid_boundary: Vec<[f64; 2]> = placed.boundary.iter().map(|&[x, y]| [x, y]).collect();
        let n = grid_points.len();

        let mut all_points = grid_points.clone();
        all_points.extend(grid_boundary.iter().cloned());
        let delaunay = from_pairs(&all_points);
        let grid_voronoi = calculate_voronoi(&delaunay, &all_points, n as u32);

        // h=50 (>=20), type=2 (no -1,-2, no coast) — no se descarta, no puntos extra.
        let grid_height = vec![50u8; n];
        let grid_water_type = vec![2i8; n];
        let grid_feature_id = vec![0u16; n];
        let grid_features_kind = vec![FeatureType::Ocean];

        let (pack, new_pts) = re_graph(
            &grid_points,
            &grid_boundary,
            &grid_voronoi,
            &grid_height,
            &grid_water_type,
            &grid_feature_id,
            &grid_features_kind,
            placed.spacing,
        );

        assert_eq!(pack.points.len(), n, "1 punto por cell (sin extras)");
        assert_eq!(new_pts.len(), n);
        assert_eq!(pack.cells.grid_id.len(), n);
        assert_eq!(pack.cells.height.len(), n);
        assert_eq!(
            pack.cells.area_px.len(),
            pack.points.len(),
            "area_px por celda del pack"
        );
        // grid_id debe ser [0..n] (en orden, sin descartes ni extras).
        assert_eq!(pack.cells.grid_id, (0..n as u32).collect::<Vec<u32>>());
        assert_eq!(pack.cells.height, vec![50u8; n]);
    }

    /// Determinismo: misma entrada → misma salida.
    #[test]
    fn regraph_is_deterministic() {
        let placed = place_points(200.0, 200.0, 100, "1");
        let grid_points: Vec<[f64; 2]> = placed.points.iter().map(|&[x, y]| [x, y]).collect();
        let grid_boundary: Vec<[f64; 2]> = placed.boundary.iter().map(|&[x, y]| [x, y]).collect();
        let n = grid_points.len();

        let mut all_points = grid_points.clone();
        all_points.extend(grid_boundary.iter().cloned());
        let delaunay = from_pairs(&all_points);
        let grid_voronoi = calculate_voronoi(&delaunay, &all_points, n as u32);

        let grid_height = vec![50u8; n];
        let grid_water_type = vec![2i8; n];
        let grid_feature_id = vec![0u16; n];
        let grid_features_kind = vec![FeatureType::Ocean];

        let (a, a_pts) = re_graph(
            &grid_points,
            &grid_boundary,
            &grid_voronoi,
            &grid_height,
            &grid_water_type,
            &grid_feature_id,
            &grid_features_kind,
            placed.spacing,
        );
        let (b, b_pts) = re_graph(
            &grid_points,
            &grid_boundary,
            &grid_voronoi,
            &grid_height,
            &grid_water_type,
            &grid_feature_id,
            &grid_features_kind,
            placed.spacing,
        );

        assert_eq!(a.points, b.points, "points determinista");
        assert_eq!(a_pts, b_pts, "new_pts determinista");
        assert_eq!(a.cells.grid_id, b.cells.grid_id);
        assert_eq!(a.cells.height, b.cells.height);
        assert_eq!(a.cells.area_px, b.cells.area_px);
    }

    /// `polygonArea_signed` — casos triviales.
    #[test]
    fn polygon_area_unit_square() {
        // Unit square en sentido CCW: [0,0], [1,0], [1,1], [0,1].
        // shoelace = (0*1 - 0*0) + (0*1 - 1*1) + (1*0 - 1*1) + (1*0 - 0*0)
        //         = 0 - 1 - 1 + 0 = -2, /2 = -1.0. Negativo → CW; abs → 1.0.
        let poly = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let area = polygon_area_signed(&poly).abs();
        assert!(
            (area - 1.0).abs() < 1e-9,
            "unit square area = 1.0, got {area}"
        );
    }
}
