//! Parseo de los slots `[0]` (header), `[1]` (settings) y `[2]` (mapCoordinates)
//! del `.map` de Azgaar, hacia `vor_core::{MapHeader, Settings, MapCoordinates}`.
//!
//! Refs bit-exactas:
//! - Header (`load.ts:252-261`): `version|license|date|seed|graphWidth|graphHeight|mapId`.
//! - Settings (`load.ts:263-289`): 27 campos pipe-delimited; el slot `[19]` es `options`
//!   como sub-JSON (resultado de `randomizeOptions()`).
//! - Coordinates (`load.ts:291`): JSON opaco con `latT/latN/latS/lonL/lonR/...`.

use thiserror::Error;
use vor_core::coordinates::MapCoordinates;
use vor_core::settings::{MapHeader, Settings};

#[derive(Debug, Error)]
pub enum HeaderError {
    #[error("slot [{0}] ausente o vacío")]
    Missing(usize),
    #[error("header inválido: esperaba 6-7 campos pipe-delimited, encontró {0}")]
    HeaderShape(usize),
    #[error("settings inválidos: esperaba ≥20 campos pipe-delimited, encontró {0}")]
    SettingsShape(usize),
    #[error("no se pudo parsear el campo `{field}` del header como `{ty}`: {raw}")]
    HeaderParse {
        field: &'static str,
        ty: &'static str,
        raw: String,
    },
    #[error("no se pudo parsear el campo settings[{idx}] (`{field}`) como `{ty}`: {raw}")]
    SettingsParse {
        idx: usize,
        field: &'static str,
        ty: &'static str,
        raw: String,
    },
    #[error("JSON inválido en slot [{0}]: {1}")]
    BadJson(usize, #[source] serde_json::Error),
}

/// Parsea el slot `[0]` → `MapHeader`.
pub fn parse_header(slot0: &str) -> Result<MapHeader, HeaderError> {
    let parts: Vec<&str> = slot0.split('|').collect();
    if !(6..=7).contains(&parts.len()) {
        return Err(HeaderError::HeaderShape(parts.len()));
    }
    let parse_u32 = |raw: &str, field: &'static str| -> Result<u32, HeaderError> {
        raw.parse::<u32>().map_err(|e| HeaderError::HeaderParse {
            field,
            ty: "u32",
            raw: format!("{raw} ({e})"),
        })
    };
    Ok(MapHeader {
        version: parts[0].to_string(),
        license: parts[1].to_string(),
        date: parts[2].to_string(),
        seed: parts[3].to_string(),
        graph_width: parse_u32(parts[4], "graph_width")?,
        graph_height: parse_u32(parts[5], "graph_height")?,
        map_id: if parts.len() > 6 {
            parts[6]
                .parse::<u64>()
                .map_err(|e| HeaderError::HeaderParse {
                    field: "map_id",
                    ty: "u64",
                    raw: format!("{} ({})", parts[6], e),
                })?
        } else {
            0
        },
    })
}

/// Parsea el slot `[1]` → `Settings`.
///
/// Replica `load.ts:263-289`. Solo mapeamos los campos que `vor-core::Settings` ya
/// tiene como fuertes; el resto (`[6]`-`[11]` vacíos por compat, `[14]`/`[15]`/`[18]`
/// migrados a `options`, `[16]`/`[17]` migrados a `options.temperatureEquator/
/// temperatureNorthPole`, `[25]` migrado a `options.longitude`) — los preservamos
/// en el objeto `options` (que llega como slot `[19]` JSON).
pub fn parse_settings(slot1: &str) -> Result<Settings, HeaderError> {
    let parts: Vec<&str> = slot1.split('|').collect();
    if parts.len() < 20 {
        return Err(HeaderError::SettingsShape(parts.len()));
    }
    let parse_f32 = |raw: &str, idx: usize, field: &'static str| -> Result<f32, HeaderError> {
        raw.parse::<f32>().map_err(|e| HeaderError::SettingsParse {
            idx,
            field,
            ty: "f32",
            raw: format!("{raw} ({e})"),
        })
    };
    let parse_u32 = |raw: &str, idx: usize, field: &'static str| -> Result<u32, HeaderError> {
        raw.parse::<u32>().map_err(|e| HeaderError::SettingsParse {
            idx,
            field,
            ty: "u32",
            raw: format!("{raw} ({e})"),
        })
    };

    let options: serde_json::Value = if parts[19].is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(parts[19]).map_err(|e| HeaderError::BadJson(1, e))?
    };

    let parse_bool01 = |raw: &str| -> Option<bool> {
        if raw.is_empty() {
            return None;
        }
        match raw {
            "0" => Some(false),
            "1" => Some(true),
            _ => Some(!matches!(raw, "")),
        }
    };

    Ok(Settings {
        distance_unit: parts[0].to_string(),
        distance_scale: if parts[1].is_empty() {
            0.0
        } else {
            parse_f32(parts[1], 1, "distance_scale")?
        },
        area_unit: parts[2].to_string(),
        height_unit: parts[3].to_string(),
        height_exponent: if parts[4].is_empty() {
            0
        } else {
            parse_u32(parts[4], 4, "height_exponent")?
        },
        temperature_unit: parts[5].to_string(),
        population_rate: if parts[12].is_empty() {
            0.0
        } else {
            parse_f32(parts[12], 12, "population_rate")?
        },
        urbanization: if parts[13].is_empty() {
            0.0
        } else {
            parse_f32(parts[13], 13, "urbanization")?
        },
        options,
        map_name: parts.get(20).map(|s| s.to_string()).unwrap_or_default(),
        hide_labels: parse_bool01(parts.get(21).unwrap_or(&"")).unwrap_or(false),
        style_preset: if parts.get(22).map(|s| s.is_empty()).unwrap_or(true) {
            None
        } else {
            Some(parts[22].to_string())
        },
        // `[23]` rescale_labels bool, `[24]` urban_density, `[26]` growth_rate — preservamos opaco.
        rescale_labels: parse_bool01(parts.get(23).unwrap_or(&""))
            .map(serde_json::Value::Bool)
            .unwrap_or(serde_json::Value::Null),
        urban_density: serde_json::Value::Null,
        growth_rate: serde_json::Value::Null,
    })
}

/// Parsea el slot `[2]` → `MapCoordinates`. JSON opaco con escala lat/lon.
pub fn parse_coordinates(slot2: Option<&str>) -> Result<MapCoordinates, HeaderError> {
    let Some(raw) = slot2 else {
        return Ok(MapCoordinates::default());
    };
    let v: serde_json::Value = serde_json::from_str(raw).map_err(|e| HeaderError::BadJson(2, e))?;
    let g = |k: &str, default: f32| -> f32 {
        v.get(k)
            .and_then(|x| x.as_f64())
            .map(|x| x as f32)
            .unwrap_or(default)
    };
    // Azgaar usa `lonW`/`lonE` en versiones recientes; `lonL`/`lonR` en algunas legacy.
    let lon_l = g("lonL", g("lonW", 0.0));
    let lon_r = g("lonR", g("lonE", 0.0));
    Ok(MapCoordinates {
        lat_t: g("latT", 0.0),
        lat_n: g("latN", 0.0),
        lat_s: g("latS", 0.0),
        lon_l,
        lon_r,
        extras: v,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRAMPLE_HEADER: &str = "1.138.0|File can be loaded in azgaar.github.io/Fantasy-Map-Generator|2026-7-24|279321909|937|945|1784954343635";
    const BRAMPLE_SETTINGS: &str = "km|2|square|m|2|°C|||||||1000|1||||||{\"mapSize\":30,\"latitude\":36}|Sorvik|1|default|1|10||1.7";

    #[test]
    fn header_parses_sorvik_shape() {
        let h = parse_header(BRAMPLE_HEADER).unwrap();
        assert_eq!(h.version, "1.138.0");
        assert_eq!(h.seed, "279321909");
        assert_eq!(h.graph_width, 937);
        assert_eq!(h.graph_height, 945);
        assert_eq!(h.map_id, 1784954343635);
        assert!(h.license.contains("azgaar.github.io"));
        assert_eq!(h.date, "2026-7-24");
    }

    #[test]
    fn header_rejects_short() {
        let bad = "1.0|seed";
        assert!(matches!(
            parse_header(bad),
            Err(HeaderError::HeaderShape(_))
        ));
    }

    #[test]
    fn settings_parses_sorvik_shape() {
        let s = parse_settings(BRAMPLE_SETTINGS).unwrap();
        assert_eq!(s.distance_unit, "km");
        assert_eq!(s.distance_scale, 2.0);
        assert_eq!(s.height_exponent, 2);
        assert_eq!(s.population_rate, 1000.0);
        assert_eq!(s.urbanization, 1.0);
        assert!(!s.options.is_null());
        assert_eq!(s.map_name, "Sorvik");
        assert!(s.options.get("mapSize").is_some());
    }

    #[test]
    fn coordinates_parse_json() {
        let raw = r#"{"latT":54,"latN":44.6,"latS":-9.4,"lonL":-26.7,"lonR":26.8}"#;
        let c = parse_coordinates(Some(raw)).unwrap();
        assert_eq!(c.lat_t, 54.0);
        assert!((c.lat_s - (-9.4)).abs() < 1e-3);
        assert!((c.lon_r - 26.8).abs() < 1e-3);
    }
}
