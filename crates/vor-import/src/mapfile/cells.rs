//! Parseo de los slots de datos por-celda del `.map`:
//! - `[6]` gridGeneral: JSON con `spacing/cellsX/cellsY/boundary/points/cellsDesired`
//!   (la geometría del grid que Azgaar serializa — confirmado slot-vs-real en Sorvik).
//! - `[7]`-`[11]` grid.cells TypedArrays (CSV de números).
//! - `[16]`-`[27]` pack.cells TypedArrays (CSV).
//! - `[36]` pack.cells.routes — JSON adjacency map (cellId → {destCellId: routeId}).
//! - `[40]`+`[44]` pack.cells.good + market (UI/economía — preservar, no interpretar en Fase 1).
//!
//! Los TypedArrays de Azgaar se serializan como CSV literal (con `,` como separador y
//! `Number(x)` como conversión). `Uint8Array.from(csv.split(","), Number)`y similar —
//! para mantener bit-exactitud contra Azgaar, replicamos el `Number()` JS para todos
//! los specs (incluyendo NaN por `""` y `-` para negativos), pero como los datos del .map
//! generados por Azgaar nunca tienen `""` para slots typed (rellean `"0"` o ``), confiamos
//! en un parser robusto que cae a `0` por defecto (no propagamos `NaN` al modelo fuerte).
//!
//! **Lo que sí es crítico**: el *largo* de cada TypedArray debe calzar con el largo de
//! `pack.cells.i` (= la cantidad de cells del pack), que viene determinada por la
//! regeneración de geometría (`reGraph`) — *no* leída del archivo. Si los counts
//! no calzan, es un warning (mapa corrupto o versión mismatch), no un hard fail.

#![allow(non_snake_case)]

use serde::Deserialize;
use thiserror::Error;
use vor_core::cells::{GridCells, PackCells, RoutesFromCell};

#[derive(Debug, Error)]
pub enum CellError {
    #[error("slot [{0}] esperado pero ausente o vacío")]
    Missing(usize),
    #[error("JSON inválido en slot [{0}]: {1}")]
    BadJson(usize, #[source] serde_json::Error),
    #[error("slot [{slot}] TypedArray: se esperaba {expected} elementos, se encontraron {actual}")]
    CountMismatch {
        slot: usize,
        expected: usize,
        actual: usize,
    },
}

// ---------------------------------------------------------------------------
// Grid general (slot `[6]` JSON)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GridGeneral {
    pub spacing: f32,
    pub cellsX: u32,
    pub cellsY: u32,
    pub cellsDesired: u32,
    pub points: Vec<[f32; 2]>,
    #[serde(default)]
    pub boundary: Vec<[f32; 2]>,
    #[serde(default)]
    pub features: serde_json::Value,
}

pub fn parse_grid_general(slot6: Option<&str>) -> Result<GridGeneral, CellError> {
    let Some(raw) = slot6 else {
        return Err(CellError::Missing(6));
    };
    serde_json::from_str(raw).map_err(|e| CellError::BadJson(6, e))
}

/// Extrae del slot `[6]` JSON las features del grid (`grid.features`) como una lista
/// de tipos Index-u32→FeatureType. usado por `re_graph` para distinguir lake/no-lake.
///
/// `grid.features` es `[0, {i:1, type:"ocean"}, {i:2, type:"island"}, ...]` —
/// el slot `[0]` es un placeholder numérico, los demás son objects.
pub fn parse_grid_features_kind(
    slot6: Option<&str>,
) -> Result<Vec<vor_core::feature::FeatureType>, CellError> {
    let Some(raw) = slot6 else {
        return Ok(Vec::new());
    };
    let v: serde_json::Value = serde_json::from_str(raw).map_err(|e| CellError::BadJson(6, e))?;
    let Some(arr) = v.get("features").and_then(|f| f.as_array()) else {
        return Ok(Vec::new());
    };
    let max_id = arr
        .iter()
        .skip(1) // index 0 is `0` placeholder.
        .filter_map(|f| f.get("i").and_then(|i| i.as_u64()).map(|i| i as usize))
        .max()
        .unwrap_or(0);
    let mut out: Vec<vor_core::feature::FeatureType> = (0..=max_id)
        .map(|_| vor_core::feature::FeatureType::default())
        .collect();
    for f in arr.iter().skip(1) {
        let Some(id) = f.get("i").and_then(|i| i.as_u64()).map(|i| i as usize) else {
            continue;
        };
        let Some(kind_str) = f.get("type").and_then(|t| t.as_str()) else {
            continue;
        };
        let kind = match kind_str {
            "ocean" => vor_core::feature::FeatureType::Ocean,
            "island" | "continent" | "isle" | "lake_island" => {
                vor_core::feature::FeatureType::Island
            }
            "lake" => vor_core::feature::FeatureType::Lake,
            _ => vor_core::feature::FeatureType::default(),
        };
        if id < out.len() {
            out[id] = kind;
        }
    }
    Ok(out)
}

/// Parsea el sub-JSON `grid.features` del slot `[6]` → `Vec<Feature>` (sin `perimeter_vertices`
/// — Azgaar serializa `vertices: []` y `shoreline: []` para grid.features, así que los dejamos
/// vacíos). Útil para poblar `vor_core::Grid::features` con el catálogo pre-markup.
pub fn parse_grid_features(
    slot6: Option<&str>,
) -> Result<Vec<vor_core::feature::Feature>, CellError> {
    let Some(raw) = slot6 else {
        return Ok(Vec::new());
    };
    let v: serde_json::Value = serde_json::from_str(raw).map_err(|e| CellError::BadJson(6, e))?;
    let Some(arr) = v.get("features").and_then(|f| f.as_array()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(arr.len().saturating_sub(1));
    for entry in arr.iter().skip(1) {
        if !entry.is_object() {
            continue;
        }
        let id = entry.get("i").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
        let kind_str = entry
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("ocean");
        let land = entry.get("land").and_then(|l| l.as_bool()).unwrap_or(false);
        let border = entry
            .get("border")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);
        let cells = entry.get("cells").and_then(|c| c.as_u64()).unwrap_or(0) as u32;
        let first_cell = entry.get("firstCell").and_then(|f| f.as_u64()).unwrap_or(0) as u32;
        let name = entry
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());
        let kind = match kind_str {
            "ocean" => vor_core::feature::FeatureType::Ocean,
            "island" | "continent" | "isle" | "lake_island" => {
                vor_core::feature::FeatureType::Island
            }
            "lake" => vor_core::feature::FeatureType::Lake,
            _ => vor_core::feature::FeatureType::default(),
        };
        out.push(vor_core::feature::Feature {
            id,
            is_land: land,
            touches_border: border,
            kind,
            land_group: if land {
                Some(vor_core::feature::LandGroup::Island)
            } else {
                None
            },
            lake_group: None,
            cell_count: cells,
            first_cell,
            perimeter_vertices: Vec::new(),
            name,
            shoreline: Vec::new(),
            lake_height: 0.0,
            inlets: Vec::new(),
            outlet_river: None,
            entering_flux: 0.0,
            closed: false,
            out_cell: None,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// TypedArray parsers para slots CSV de `.map`.
// ---------------------------------------------------------------------------

fn parse_u8(s: Option<&str>) -> Vec<u8> {
    let Some(raw) = s else {
        return Vec::new();
    };
    raw.split(',')
        .filter(|t| !t.is_empty())
        .map(|t| t.trim().parse::<f64>().unwrap_or(0.0).clamp(0.0, 255.0) as u8)
        .collect()
}
fn parse_i8(s: Option<&str>) -> Vec<i8> {
    let Some(raw) = s else {
        return Vec::new();
    };
    raw.split(',')
        .filter(|t| !t.is_empty())
        .map(|t| {
            let v = t.trim().parse::<f64>().unwrap_or(0.0);
            // JS Int8Array trunca hacia 0 con ToInt8 (`|0` casero, normal-bracket `Number(x)|0`).
            // Pérdida plabable en estos datos no negativos para temp. Replicamos con `as i32` como ToInt32
            // y luego castear a i8 — matches JS Int8Array comportamiento.
            (v as i32 & 0xFF) as i8
        })
        .collect()
}
fn parse_u16(s: Option<&str>) -> Vec<u16> {
    #[allow(clippy::unnecessary_cast, clippy::cast_possible_truncation)]
    fn conv(t: &str) -> u16 {
        let v = t.trim().parse::<f64>().unwrap_or(0.0);
        // ToUint16 de JS: `Number(t) & 0xFFFF` (bitand Uint16 storage).
        let i = v as i32 as u32;
        (i & 0xFFFF) as u16
    }
    let Some(raw) = s else {
        return Vec::new();
    };
    raw.split(',').filter(|t| !t.is_empty()).map(conv).collect()
}
fn parse_f32(s: Option<&str>) -> Vec<f32> {
    let Some(raw) = s else {
        return Vec::new();
    };
    raw.split(',')
        .filter(|t| !t.is_empty())
        .map(|t| t.trim().parse::<f64>().unwrap_or(0.0) as f32)
        .collect()
}

/// Parsea los slots `[7]`-`[11]` → `GridCells`. Los 5 slots están esperados (no opcionales
/// en Azgaar); si falta alguno, se llena con `Vec::new()` y el caller chequea shapes.
pub fn parse_grid_cells(
    slot7: Option<&str>,
    slot8: Option<&str>,
    slot9: Option<&str>,
    slot10: Option<&str>,
    slot11: Option<&str>,
) -> GridCells {
    GridCells {
        height: parse_u8(slot7),
        precipitation: parse_u16(slot8),
        feature_id: parse_u16(slot9),
        water_type: parse_i8(slot10),
        temperature: parse_i8(slot11),
    }
}

/// Parsea los slots `[16]`-`[27]` (más `[36]`/`[40]`/`[44]`) → `PackCells`.
///
/// El `grid_id` array no se persiste en el `.map` (Azgaar lo pobla durante `reGraph`),
/// pero en Voronia lo obtenemos del `re_graph` directamente. Acá dejamos `grid_id`
/// vacío — el caller (loader) debe poblarlo.
#[allow(clippy::too_many_arguments)]
pub fn parse_pack_cells(
    slot16_biome: Option<&str>,
    slot17_burg: Option<&str>,
    slot18_conf: Option<&str>,
    slot19_culture: Option<&str>,
    slot20_flux: Option<&str>,
    slot21_pop: Option<&str>,
    slot22_river: Option<&str>,
    slot24_score: Option<&str>,
    slot25_state: Option<&str>,
    slot26_religion: Option<&str>,
    slot27_province: Option<&str>,
    slot36_routes: Option<&str>,
    slot40_good: Option<&str>,
    slot44_market: Option<&str>,
) -> PackCells {
    PackCells {
        grid_id: Vec::new(), // populated by the loader after `re_graph`.
        height: Vec::new(),  // idem — comes from grid.cells.h via regraph.
        area_px: Vec::new(),
        biome: parse_u8(slot16_biome),
        burg: parse_u16(slot17_burg),
        confluence: parse_u16(slot18_conf),
        culture: parse_u16(slot19_culture),
        flux: parse_u16(slot20_flux),
        population: parse_f32(slot21_pop),
        river: parse_u16(slot22_river),
        score: parse_u16(slot24_score),
        state: parse_u16(slot25_state),
        religion: parse_u16(slot26_religion),
        province: parse_u16(slot27_province),
        good: parse_u16(slot40_good),
        market: parse_u16(slot44_market),
        routes: parse_routes_map(slot36_routes),
        feature_id: Vec::new(), // populated by loader after re_graph (from grid_id → grid.feature_id)
        adjacency: Vec::new(),  // populated by `re_graph`.
    }
}

/// Parsea el slot `[36]` → `Vec<RoutesFromCell>`.
///
/// JSON shape: `{"originCellId": {"destCellId": routeId, ...}, ...}`. El `Vec` se indexa
/// por originCellId (con holes en `Vec::new()` si hay ids salteados).
fn parse_routes_map(slot36: Option<&str>) -> Vec<RoutesFromCell> {
    let Some(raw) = slot36 else {
        return Vec::new();
    };
    let v: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(obj) = v.as_object() else {
        return Vec::new();
    };
    let max_origin = obj
        .keys()
        .filter_map(|k| k.parse::<usize>().ok())
        .max()
        .unwrap_or(0);
    let mut out: Vec<RoutesFromCell> = (0..=max_origin)
        .map(|_| RoutesFromCell::default())
        .collect();
    for (origin_str, dests) in obj {
        let Ok(origin) = origin_str.parse::<usize>() else {
            continue;
        };
        let Some(dests_obj) = dests.as_object() else {
            continue;
        };
        let mut to = Vec::new();
        for (dest_str, route_id) in dests_obj {
            let Ok(dest) = dest_str.parse::<u32>() else {
                continue;
            };
            let Some(rid) = route_id.as_u64() else {
                continue;
            };
            to.push((dest, rid as u32));
        }
        if origin < out.len() {
            out[origin] = RoutesFromCell { to };
        }
    }
    out
}
