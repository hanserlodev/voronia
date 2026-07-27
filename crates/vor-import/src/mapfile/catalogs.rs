//! Parseo de los slots de catálogos JSON del `.map`:
//! - `[3]` biomas (pipe-CSV de 3 sub-campos: colors, habitability, names).
//! - `[4]` notes (JSON).
//! - `[12]` features (JSON; **no** mapeamos todos los campos — vértices/shoreline
//!   son too heavy en Fase 1, se preservan opacos).
//! - `[13]` cultures; `[14]` states; `[15]` burgs; `[29]` religions; `[30]` provinces;
//!   `[32]` rivers; `[35]` markers; `[37]` routes; `[38]` zones; `[39]` ice;
//!   `[46]` measurers.
//! - `[31]` namebases (formato custom `/`-delimited, `|`-separated fields: `name|min|max|d|m|b[|prob]`).
//!
//! ## Naming mismatches con `vor-core`
//!
//! Azgaar usa `center`/`mapId`/`i` siempre para entidades, mientras que
//! `vor-core::entities::*` usa `center_cell`/`id` etc. Resolvemos con structs
//! intermedios (`XRaw`) que reflejan el JSON exacto, y un mapeo explícito a los tipos
//! fuertes del core. Esto evita el truco de `#[serde(flatten)]` que es más frágil.

#![allow(non_snake_case)]

use serde::Deserialize;
use thiserror::Error;
use vor_core::entities::{
    biome::Biome,
    burg::Burg,
    culture::Culture,
    ice::{Ice, IceKind},
    marker::Marker,
    measurer::Measurer,
    namebase::NameBase,
    note::Note,
    province::Province,
    religion::{Religion, ReligionExpansion, ReligionType},
    river::River,
    route::{Route, RouteGroup},
    state::State,
    zone::Zone,
};
use vor_core::feature::{Feature, FeatureType};

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("slot [{0}] esperado pero ausente o vacío")]
    Missing(usize),
    #[error("JSON inválido en slot [{0}]: {1}")]
    BadJson(usize, #[source] serde_json::Error),
    #[error("biomas malformados en slot [3]: esperaba 3 sub-campos, encontró {0}")]
    BiomeShape(usize),
}

// ---------------------------------------------------------------------------
// Biomas (slot `[3]`, pipe-CSV de 3 sub-campos: `colors|habitabilities|names`)
// ---------------------------------------------------------------------------

/// Parsea el slot `[3]` → `Vec<Biome>`. Id implícito por orden (0 = Marine / océano).
pub fn parse_biomes(slot3: &str) -> Result<Vec<Biome>, CatalogError> {
    let parts: Vec<&str> = slot3.split('|').collect();
    if parts.len() != 3 {
        return Err(CatalogError::BiomeShape(parts.len()));
    }
    let colors: Vec<&str> = parts[0].split(',').collect();
    let habitabilities: Vec<&str> = parts[1].split(',').collect();
    let names: Vec<&str> = parts[2].split(',').collect();
    let n = colors.len();
    if habitabilities.len() != n || names.len() != n {
        return Err(CatalogError::BiomeShape(parts.len()));
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let id = i as u8;
        let color = colors[i].to_string();
        let habitability = habitabilities[i].parse::<f32>().unwrap_or(0.0);
        let name = names[i].to_string();
        out.push(Biome {
            id,
            color,
            habitability,
            move_cost: 50.0,
            name,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Features (slot `[12]` JSON) — mapeamos tipo fuerte, preservamos opacos
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct FeatureRaw {
    pub i: u32,
    #[serde(rename = "type", default)]
    pub kind_str: String,
    #[serde(default)]
    pub land: bool,
    #[serde(default)]
    pub border: bool,
    #[serde(default)]
    pub cells: u32,
    #[serde(rename = "firstCell", default)]
    pub first_cell: u32,
    #[serde(default)]
    pub vertices: serde_json::Value,
    #[serde(default)]
    pub area: serde_json::Value,
    #[serde(default)]
    pub shoreline: serde_json::Value,
    #[serde(default)]
    pub height: serde_json::Value,
    #[serde(default)]
    pub name: Option<String>,
}

/// Parsea el slot `[12]` → `Vec<Feature>`. Slot `[0]` de Azgaar es `0` (placeholder numérico);
/// se salta en el mapeo (los ids reales arrancan en 1).
pub fn parse_features(slot12: Option<&str>) -> Result<Vec<Feature>, CatalogError> {
    let Some(raw) = slot12 else {
        return Ok(Vec::new());
    };
    let v: Vec<serde_json::Value> =
        serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(12, e))?;
    let mut out = Vec::with_capacity(v.len().saturating_sub(1));
    for entry in v.iter().skip(1) {
        // El slot [0] suele ser `0` (placeholder numérico, no object) en Azgaar.
        if !entry.is_object() {
            continue;
        }
        let fr: FeatureRaw =
            serde_json::from_value(entry.clone()).map_err(|e| CatalogError::BadJson(12, e))?;
        let kind = match fr.kind_str.as_str() {
            "ocean" => FeatureType::Ocean,
            "island" | "continent" | "isle" | "lake_island" => FeatureType::Island,
            "lake" => FeatureType::Lake,
            _ => FeatureType::default(),
        };
        out.push(Feature {
            id: fr.i,
            is_land: fr.land,
            touches_border: fr.border,
            kind,
            land_group: if fr.land {
                Some(vor_core::feature::LandGroup::Island)
            } else {
                None
            },
            lake_group: None,
            cell_count: fr.cells,
            first_cell: fr.first_cell,
            perimeter_vertices: Vec::new(),
            name: fr.name,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Notes (slot `[4]` JSON) — keys `id` (str), `name`, `legend`
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct NoteRaw {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub legend: String,
}

pub fn parse_notes(slot4: Option<&str>) -> Result<Vec<Note>, CatalogError> {
    let Some(raw) = slot4 else {
        return Ok(Vec::new());
    };
    // Azgaar serializa notes con emoji/no-BMP chars como `\uXXXX` escapes en el JSON,
    // pero pueden aparecer *lone surrogates* (`\uD800`-`\uDBFF` o `\uDC00`-`\uDFFF`
    // sin su contraparte), válidos en algunas rutas de JS pero ilegales en JSON RFC 8259.
    // `serde_json` los rechaza con "unexpected end of hex escape" o "lone surrogate".
    // Estrategia lossy: reemplazamos escapes `\uXXXX` sin su par (donde el próximo
    // char no es `\u` formando surrogate pair) por un replacement char `?`. El dato
    // se preserva parcial; los `notes` son texto legend libre sin invariantes duras.
    let cleaned = sanitize_lone_surrogates(raw);
    let v: Vec<NoteRaw> =
        serde_json::from_str(&cleaned).map_err(|e| CatalogError::BadJson(4, e))?;
    Ok(v.into_iter()
        .enumerate()
        .map(|(i, n)| Note {
            id: i as u32,
            content: format!("{}\n\n{}", n.name, n.legend),
            linked_id: None,
        })
        .collect())
}

/// Reemplaza escapes `\uXXXX` solitarios (sin su contraparte de surrogate pair) por `?`.
fn sanitize_lone_surrogates(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 5 < bytes.len() && bytes[i] == b'\\' && bytes[i + 1] == b'u' {
            let hex = &s[i + 2..i + 6];
            if let Ok(code) = u16::from_str_radix(hex, 16) {
                let is_high = (0xD800..=0xDBFF).contains(&code);
                let is_low = (0xDC00..=0xDFFF).contains(&code);
                if is_high {
                    // Si el próximo no es `\uXXXX` con low surrogate, es lone.
                    if i + 11 < bytes.len() && bytes[i + 6] == b'\\' && bytes[i + 7] == b'u' {
                        let next_hex = &s[i + 8..i + 12];
                        if let Ok(next_code) = u16::from_str_radix(next_hex, 16) {
                            if (0xDC00..=0xDFFF).contains(&next_code) {
                                // Pair válido — preservar ambos escapes.
                                out.push_str(&s[i..i + 12]);
                                i += 12;
                                continue;
                            }
                        }
                    }
                    // Lone high surrogate — reemplazar por `?`.
                    out.push('?');
                    i += 6;
                    continue;
                }
                if is_low {
                    // Lone low surrogate — reemplazar por `?`.
                    out.push('?');
                    i += 6;
                    continue;
                }
                // BMP escape normal — preservar.
                out.push_str(&s[i..i + 6]);
                i += 6;
                continue;
            }
        }
        // Default case: copiar el byte como UTF-8.
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

// ---------------------------------------------------------------------------
// Culture (slot `[13]` JSON) — `i`, `name`, `base`, `type`, `center`, `code`, ...
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CultureRaw {
    pub i: u16,
    pub name: String,
    pub base: u16,
    #[serde(rename = "type", default)]
    pub kind_str: String,
    #[serde(default)]
    pub center: u32,
    #[serde(default)]
    pub origins: serde_json::Value,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub expansionism: f32,
    #[serde(default)]
    pub shield: serde_json::Value,
}

pub fn parse_cultures(slot13: Option<&str>) -> Result<Vec<Culture>, CatalogError> {
    let Some(raw) = slot13 else {
        return Ok(Vec::new());
    };
    let v: Vec<serde_json::Value> =
        serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(13, e))?;
    let mut out = Vec::with_capacity(v.len().saturating_sub(1));
    for entry in v.iter().skip(1) {
        if !entry.is_object() {
            continue;
        }
        let c: CultureRaw =
            serde_json::from_value(entry.clone()).map_err(|e| CatalogError::BadJson(13, e))?;
        let kind = match c.kind_str.as_str() {
            "River" => vor_core::entities::culture::CultureType::River,
            "Lake" => vor_core::entities::culture::CultureType::Lake,
            "Naval" => vor_core::entities::culture::CultureType::Naval,
            "Nomadic" => vor_core::entities::culture::CultureType::Nomadic,
            "Hunting" => vor_core::entities::culture::CultureType::Hunting,
            "Highland" => vor_core::entities::culture::CultureType::Highland,
            _ => vor_core::entities::culture::CultureType::Generic,
        };
        out.push(Culture {
            id: c.i,
            name: c.name,
            namebase_id: c.base,
            origins: json_origins_to_u16(&c.origins),
            shield: vor_core::entities::coat_of_arms::CoatOfArms::default(),
            center_cell: c.center,
            code: c.code,
            color: c.color,
            expansionism: c.expansionism,
            kind,
            area_px: 0,
            cells: 0,
            rural_pop: 0.0,
            urban_pop: 0.0,
            locked: false,
            removed: false,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// State (slot `[14]` JSON) — grande; preservamos `diplomacy`/`campaigns`/`military` opaco
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct StateRaw {
    pub i: u16,
    pub name: String,
    #[serde(default)]
    pub center: u32,
    #[serde(default)]
    pub culture: u16,
    #[serde(rename = "type", default)]
    pub kind_str: String,
    #[serde(default)]
    pub expansionism: f32,
    #[serde(default)]
    pub capital: Option<u16>,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub neighbors: Vec<u16>,
    #[serde(default)]
    pub provinces: Vec<u16>,
    #[serde(default)]
    pub form: String,
    #[serde(default)]
    pub formName: String,
    #[serde(default)]
    pub fullName: String,
    #[serde(default)]
    pub area: u32,
    #[serde(default)]
    pub cells: u32,
    #[serde(default)]
    pub burgs: u32,
    #[serde(default)]
    pub urban: f32,
    #[serde(default)]
    pub rural: f32,
    #[serde(default)]
    pub coa: serde_json::Value,
    #[serde(default)]
    pub diplomacy: serde_json::Value,
    #[serde(default)]
    pub campaigns: serde_json::Value,
    #[serde(default)]
    pub military: serde_json::Value,
}

pub fn parse_states(slot14: Option<&str>) -> Result<Vec<State>, CatalogError> {
    let Some(raw) = slot14 else {
        return Ok(Vec::new());
    };
    let v: Vec<serde_json::Value> =
        serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(14, e))?;
    let mut out = Vec::with_capacity(v.len().saturating_sub(1));
    for entry in v.iter().skip(1) {
        if !entry.is_object() {
            continue;
        }
        let s: StateRaw =
            serde_json::from_value(entry.clone()).map_err(|e| CatalogError::BadJson(14, e))?;
        let kind = match s.kind_str.as_str() {
            "River" => vor_core::entities::culture::CultureType::River,
            "Naval" => vor_core::entities::culture::CultureType::Naval,
            "Nomadic" => vor_core::entities::culture::CultureType::Nomadic,
            "Hunting" => vor_core::entities::culture::CultureType::Hunting,
            "Highland" => vor_core::entities::culture::CultureType::Highland,
            _ => vor_core::entities::culture::CultureType::Generic,
        };
        let form = match s.form.as_str() {
            "Monarchy" => vor_core::entities::state::GovernmentForm::Monarchy,
            "Republic" => vor_core::entities::state::GovernmentForm::Republic,
            "Theocracy" => vor_core::entities::state::GovernmentForm::Theocracy,
            "Union" => vor_core::entities::state::GovernmentForm::Union,
            _ => vor_core::entities::state::GovernmentForm::Anarchy,
        };
        out.push(State {
            id: s.i,
            name: s.name,
            form,
            full_name: s.fullName,
            color: s.color,
            center_cell: s.center,
            pole_of_inaccessibility: [0.0, 0.0],
            culture: s.culture,
            kind,
            expansionism: s.expansionism,
            area_px: s.area,
            burg_count: s.burgs,
            cell_count: s.cells,
            rural_pop: s.rural,
            urban_pop: s.urban,
            neighbors: s.neighbors,
            provinces: s.provinces,
            coat_of_arms: vor_core::entities::coat_of_arms::CoatOfArms::default(),
            locked: false,
            removed: false,
            diplomacy: s.diplomacy,
            campaigns: s.campaigns,
            military: s.military,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Burg (slot `[15]` JSON)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct BurgRaw {
    #[serde(default)]
    pub i: u16,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub cell: u32,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub state: u16,
    #[serde(default)]
    pub culture: u16,
    #[serde(default)]
    pub feature: u32,
    #[serde(default)]
    pub capital: u8,
    #[serde(default)]
    pub port: u8,
    #[serde(default)]
    pub population: f32,
    #[serde(rename = "type", default)]
    pub kind_str: String,
    #[serde(default)]
    pub citadel: u8,
    #[serde(default)]
    pub walls: u8,
    #[serde(default)]
    pub shanty: u8,
    #[serde(default)]
    pub temple: u8,
    #[serde(default)]
    pub locked: u8,
    #[serde(default)]
    pub removed: u8,
    #[serde(default)]
    pub coa: serde_json::Value,
}

pub fn parse_burgs(slot15: Option<&str>) -> Result<Vec<Burg>, CatalogError> {
    let Some(raw) = slot15 else {
        return Ok(Vec::new());
    };
    let v: Vec<serde_json::Value> =
        serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(15, e))?;
    let mut out = Vec::with_capacity(v.len().saturating_sub(1));
    for entry in v.iter().skip(1) {
        if !entry.is_object() {
            continue;
        }
        let b: BurgRaw =
            serde_json::from_value(entry.clone()).map_err(|e| CatalogError::BadJson(15, e))?;
        let kind = match b.kind_str.as_str() {
            "River" => vor_core::entities::culture::CultureType::River,
            "Naval" => vor_core::entities::culture::CultureType::Naval,
            "Nomadic" => vor_core::entities::culture::CultureType::Nomadic,
            "Hunting" => vor_core::entities::culture::CultureType::Hunting,
            "Highland" => vor_core::entities::culture::CultureType::Highland,
            _ => vor_core::entities::culture::CultureType::Generic,
        };
        out.push(Burg {
            id: b.i,
            name: b.name,
            cell: b.cell,
            position: [b.x, b.y],
            culture: b.culture,
            state: b.state,
            feature: b.feature,
            population: b.population,
            kind,
            coat_of_arms: vor_core::entities::coat_of_arms::CoatOfArms::default(),
            is_capital: b.capital != 0,
            port_feature: (b.port != 0).then_some(b.feature),
            has_citadel: b.citadel != 0,
            has_plaza: false,
            has_shanty: b.shanty != 0,
            has_temple: b.temple != 0,
            has_walls: b.walls != 0,
            locked: b.locked != 0,
            removed: b.removed != 0,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Religion (slot `[29]` JSON) — opcional
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ReligionRaw {
    pub i: u16,
    pub name: String,
    #[serde(rename = "type", default)]
    pub kind_str: String,
    #[serde(default)]
    pub form: String,
    #[serde(default)]
    pub expansion: String,
    #[serde(default)]
    pub center: u32,
    #[serde(default)]
    pub culture: u16,
    #[serde(default)]
    pub origins: serde_json::Value,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub expansionism: f32,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub deity: serde_json::Value,
}

fn json_origins_to_u16(o: &serde_json::Value) -> Vec<u16> {
    let Some(arr) = o.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_u64().map(|x| x as u16))
        .collect()
}

pub fn parse_religions(slot29: Option<&str>) -> Result<Vec<Religion>, CatalogError> {
    let Some(raw) = slot29 else {
        return Ok(Vec::new());
    };
    let v: Vec<serde_json::Value> =
        serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(29, e))?;
    let mut out = Vec::with_capacity(v.len());
    for entry in v.iter() {
        if !entry.is_object() {
            continue;
        }
        let r: ReligionRaw =
            serde_json::from_value(entry.clone()).map_err(|e| CatalogError::BadJson(29, e))?;
        let kind = match r.kind_str.as_str() {
            "Organized" => ReligionType::Organized,
            "Heresy" => ReligionType::Heresy,
            "Cult" => ReligionType::Cult,
            _ => ReligionType::Folk,
        };
        let expansion = match r.expansion.as_str() {
            "global" => ReligionExpansion::Global,
            _ => ReligionExpansion::Culture,
        };
        out.push(Religion {
            id: r.i,
            name: r.name,
            origins: json_origins_to_u16(&r.origins),
            color: r.color,
            kind,
            expansion,
            center_cell: r.center,
            cells: 0,
            area_px: 0,
            locked: false,
            removed: false,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Province (slot `[30]` JSON) — opcional
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ProvinceRaw {
    pub i: u16,
    pub name: String,
    pub state: u16,
    #[serde(default)]
    pub center: u32,
    #[serde(default)]
    pub burg: Option<u16>,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub fullName: String,
}

pub fn parse_provinces(slot30: Option<&str>) -> Result<Vec<Province>, CatalogError> {
    let Some(raw) = slot30 else {
        return Ok(Vec::new());
    };
    let v: Vec<serde_json::Value> =
        serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(30, e))?;
    let mut out = Vec::with_capacity(v.len().saturating_sub(1));
    for entry in v.iter().skip(1) {
        if !entry.is_object() {
            continue;
        }
        let p: ProvinceRaw =
            serde_json::from_value(entry.clone()).map_err(|e| CatalogError::BadJson(30, e))?;
        out.push(Province {
            id: p.i,
            name: p.name,
            state: p.state,
            culture: 0,
            capital: p.burg,
            color: p.color,
            center_cell: p.center,
            pole_of_inaccessibility: [0.0, 0.0],
            burgs: Vec::new(),
            cells: 0,
            area_px: 0,
            rural_pop: 0.0,
            urban_pop: 0.0,
            locked: false,
            removed: false,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// River (slot `[32]` JSON) — opcional
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RiverRaw {
    pub i: u16,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub source: u32,
    #[serde(default)]
    pub mouth: u32,
    #[serde(default)]
    pub parent: Option<u16>,
    #[serde(default)]
    pub basin: u16,
    #[serde(default)]
    pub discharge: f32,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub width: f32,
}

pub fn parse_rivers(slot32: Option<&str>) -> Result<Vec<River>, CatalogError> {
    let Some(raw) = slot32 else {
        return Ok(Vec::new());
    };
    let v: Vec<RiverRaw> = serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(32, e))?;
    Ok(v.into_iter()
        .map(|r| River {
            id: r.i,
            name: r.name,
            source_cell: r.source,
            mouth_cell: r.mouth,
            parent_river: r.parent,
            basin_id: r.basin,
            discharge_m3s: r.discharge,
            length_km: r.length,
            width_km: r.width,
            cell_path: Vec::new(), // populated by loader after geometry is available
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Marker (slot `[35]` JSON)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct MarkerRaw {
    pub i: u32,
    #[serde(default)]
    pub icon: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub dx: i32,
    #[serde(default)]
    pub px: i32,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub cell: u32,
}

pub fn parse_markers(slot35: Option<&str>) -> Result<Vec<Marker>, CatalogError> {
    let Some(raw) = slot35 else {
        return Ok(Vec::new());
    };
    let v: Vec<MarkerRaw> = serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(35, e))?;
    Ok(v.into_iter()
        .map(|m| Marker {
            id: m.i,
            icon: m.icon,
            kind: m.kind,
            label_dx: m.dx,
            label_px: m.px,
            position: [m.x, m.y],
            cell: m.cell,
            legend: None,
            note_id: None,
            removed: false,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Route (slot `[37]` JSON) — opcional
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RouteRaw {
    pub i: u32,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub feature: u32,
    #[serde(default)]
    pub points: Vec<[f32; 3]>,
}

pub fn parse_routes(slot37: Option<&str>) -> Result<Vec<Route>, CatalogError> {
    let Some(raw) = slot37 else {
        return Ok(Vec::new());
    };
    let v: Vec<RouteRaw> = serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(37, e))?;
    Ok(v.into_iter()
        .map(|r| Route {
            id: r.i,
            group: match r.group.as_str() {
                "trails" => RouteGroup::Trails,
                "searoutes" => RouteGroup::Searoutes,
                _ => RouteGroup::Roads,
            },
            feature: r.feature,
            points: r.points,
            length: 0.0,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Zone (slot `[38]` JSON) — opcional
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ZoneRaw {
    pub i: u32,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
    pub kind_str: String,
    #[serde(default)]
    pub cells: Vec<u32>,
    #[serde(default)]
    pub color: String,
}

pub fn parse_zones(slot38: Option<&str>) -> Result<Vec<Zone>, CatalogError> {
    let Some(raw) = slot38 else {
        return Ok(Vec::new());
    };
    let v: Vec<ZoneRaw> = serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(38, e))?;
    Ok(v.into_iter()
        .map(|z| Zone {
            id: z.i,
            name: z.name,
            color: z.color,
            cells: z.cells,
            style: Some(z.kind_str),
            description: None,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Ice (slot `[39]` JSON) — opcional
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct IceRaw {
    pub i: u32,
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
    #[serde(rename = "type", default)]
    pub kind_str: String,
}

pub fn parse_ice(slot39: Option<&str>) -> Result<Vec<Ice>, CatalogError> {
    let Some(raw) = slot39 else {
        return Ok(Vec::new());
    };
    let v: Vec<IceRaw> = serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(39, e))?;
    Ok(v.into_iter()
        .map(|i| Ice {
            id: i.i,
            kind: match i.kind_str.as_str() {
                "iceberg" => IceKind::Iceberg,
                _ => IceKind::Glacier,
            },
            vertices: i.points,
            cell: None,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Measurers (slot `[46]` JSON) — opcional
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct MeasurerRaw {
    #[serde(default)]
    #[serde(rename = "type")]
    pub kind_str: String,
    pub points: Vec<[f32; 2]>,
}

pub fn parse_measurers(slot46: Option<&str>) -> Result<Vec<Measurer>, CatalogError> {
    let Some(raw) = slot46 else {
        return Ok(Vec::new());
    };
    let v: Vec<MeasurerRaw> =
        serde_json::from_str(raw).map_err(|e| CatalogError::BadJson(46, e))?;
    Ok(v.into_iter()
        .enumerate()
        .map(|(i, m)| Measurer {
            id: i as u32,
            name: m.kind_str,
            points: m.points,
            length: None,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// NameBases (slot `[31]` custom) — `/` entre entries, `|` entre campos,
// 5..6 campos: `name|min|max|d|m|b[|prob]`
// ---------------------------------------------------------------------------

/// Formato Azgaar: `"German|5|12|lt|0|/English|6|11||0.1|/..."`.
/// Los campos son: `name|min|max|d|m|b` (obligatorios), con un `prob` opcional al final
/// (`multiword_probability` en `vor-core::NameBase`) que Azgaar añadió post-1.138.
pub fn parse_namebases(slot31: Option<&str>) -> Result<Vec<NameBase>, CatalogError> {
    let Some(raw) = slot31 else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, entry) in raw.split('/').enumerate() {
        if entry.is_empty() {
            continue;
        }
        let fields: Vec<&str> = entry.split('|').collect();
        if fields.len() < 5 {
            continue;
        }
        let min_length = fields[1].parse::<u32>().unwrap_or(0);
        let max_length = fields[2].parse::<u32>().unwrap_or(0);
        let m_str = fields[3];
        let m_val: u32 = m_str.parse().unwrap_or(0);
        let b = fields[4].to_string();
        let prob = fields.get(5).and_then(|s| s.parse::<f32>().ok());
        out.push(NameBase {
            id: i as u16,
            name: fields[0].to_string(),
            min_length,
            max_length,
            d: m_str.to_string(),
            m: m_val.to_string(),
            b,
            multiword_probability: prob,
        });
    }
    Ok(out)
}
