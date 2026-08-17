//! Deal (slot `[43]`: `pack.deals` JSON).
//!
//! A trade deal between two entities (market/market, market/burg, ...) for a
//! given good. Consumed by the trade animation layer (Fase 8).

/// Type of the buyer/seller entity (FMG `buyerType`/`sellerType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DealEntityType {
    /// A burg (urban consumer/producer).
    Burg,
    /// A market (regional hub).
    #[default]
    Market,
    /// A state treasury.
    State,
}

/// A trade deal.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Deal {
    /// Id (index in `pack.deals`).
    #[serde(rename = "i")]
    pub id: u32,
    /// Seller entity id (burg id / market id / state id).
    pub seller: u32,
    /// Seller entity type.
    #[serde(default)]
    pub seller_type: DealEntityType,
    /// Buyer entity id.
    pub buyer: u32,
    /// Buyer entity type.
    #[serde(default)]
    pub buyer_type: DealEntityType,
    /// Good id.
    pub good: u16,
    /// Units traded.
    #[serde(default)]
    pub units: f32,
    /// Unit price.
    #[serde(default)]
    pub price: f32,
    /// Tax (paid to the exporting state).
    #[serde(default)]
    pub tax: f32,
}
