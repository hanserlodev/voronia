//! Market (slot `[42]`: `pack.markets` JSON).
//!
//! A market is an economic hub centered on a burg, with a territory of cells
//! (`pack.cells.market`) and per-good stock/price. Voronia models the runtime
//! fields consumed by the markets layer (Fase 7) and the trade animation
//! (Fase 8).

use std::collections::BTreeMap;

/// Per-good market state (stock + price).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketGood {
    /// Current stock in units.
    #[serde(default)]
    pub stock: f32,
    /// Current price.
    #[serde(default)]
    pub price: f32,
}

/// A market.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    /// Id (index in `pack.markets`; 0 = placeholder).
    #[serde(rename = "i")]
    pub id: u16,
    /// Id of the central burg.
    #[serde(rename = "centerBurgId")]
    pub center_burg_id: u16,
    /// Hex color.
    #[serde(default)]
    pub color: String,
    /// Name (optional).
    #[serde(default)]
    pub name: String,
    /// Per-good stock/price, keyed by good id.
    #[serde(default)]
    pub goods: BTreeMap<String, MarketGood>,
    /// `true` if removed by the user.
    #[serde(default)]
    pub removed: bool,
}
