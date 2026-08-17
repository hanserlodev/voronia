//! Good (slot `[41]`: `pack.goods` JSON).
//!
//! Tradable resources/products that flow through the economy. Canonical
//! catalogue lives in FMG's `GOODS_DATA`; the `.map` carries the evaluated
//! `pack.goods` array. Voronia models the runtime fields so the economy layers
//! (goods render, markets, trade) can consume them directly instead of holding
//! opaque `serde_json::Value`.

use std::collections::BTreeMap;

/// A tradable good.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Good {
    /// Id (index in `pack.goods`; 0 = reserved/none). Serialized as `i`.
    #[serde(rename = "i")]
    pub id: u16,
    /// Display name ("Stone", "Wood", ...).
    #[serde(default)]
    pub name: String,
    /// Hex color.
    #[serde(default)]
    pub color: String,
    /// SVG icon id (`#good-stone`, ...) — keep as string (used by the icons sub-layer).
    #[serde(default)]
    pub icon: String,
    /// Probability (0–100) of placing a bonus resource cell of this good.
    #[serde(default)]
    pub chance: f32,
    /// Raw/hybrid distribution expression (FMG DSL) — opaque string for now.
    #[serde(default)]
    pub distribution: String,
    /// Units produced per 1 rural population point per cycle, keyed by biome id.
    #[serde(default)]
    pub biome_output: BTreeMap<String, f32>,
    /// Multipliers (biome, cultureType, etc.) — opaque for now.
    #[serde(default)]
    pub multipliers: serde_json::Value,
    /// Semantic tags (e.g. "construction").
    #[serde(default)]
    pub tags: Vec<String>,
    /// Unit label ("pallet", ...).
    #[serde(default)]
    pub unit: String,
    /// Alternative recipes for manufactured/hybrid goods: each recipe is a
    /// sparse `{goodId -> amount}` map producing 1 unit of this good.
    #[serde(default)]
    pub recipes: Vec<BTreeMap<String, f32>>,
    /// Demand coverage per sector (e.g. `{"construction": 1}`).
    #[serde(default)]
    pub demand_coverage: serde_json::Value,
    /// `true` if visible in the goods layer (FMG `good.visible`).
    #[serde(default)]
    pub visible: bool,
    /// `true` if removed by the user.
    #[serde(default)]
    pub removed: bool,
}

impl Good {
    /// Reserved placeholder for id 0.
    #[inline]
    pub fn placeholder() -> Self {
        Self {
            id: 0,
            name: String::new(),
            color: String::new(),
            icon: String::new(),
            chance: 0.0,
            distribution: String::new(),
            biome_output: BTreeMap::new(),
            multipliers: serde_json::Value::Null,
            tags: Vec::new(),
            unit: String::new(),
            recipes: Vec::new(),
            demand_coverage: serde_json::Value::Null,
            visible: true,
            removed: false,
        }
    }

    /// `true` if the good is raw (has `distribution`, no recipes).
    #[inline]
    pub fn is_raw(&self) -> bool {
        !self.distribution.is_empty() && self.recipes.is_empty()
    }

    /// `true` if the good is manufactured (has recipes, no distribution).
    #[inline]
    pub fn is_manufactured(&self) -> bool {
        self.distribution.is_empty() && !self.recipes.is_empty()
    }

    /// `true` if hybrid (both channels).
    #[inline]
    pub fn is_hybrid(&self) -> bool {
        !self.distribution.is_empty() && !self.recipes.is_empty()
    }
}
