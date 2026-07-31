use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct River {
    pub id: u16,
    pub name: String,
    pub source_cell: u32,
    pub mouth_cell: u32,
    #[serde(default)]
    pub parent_river: Option<u16>,
    #[serde(default)]
    pub basin_id: u16,
    #[serde(default)]
    pub discharge_m3s: f32,
    #[serde(default)]
    pub length_km: f32,
    #[serde(default)]
    pub width_km: f32,
    #[serde(default)]
    pub width_factor: f32,
    #[serde(default)]
    pub source_width_km: f32,
    #[serde(default)]
    pub type_name: String,
    #[serde(skip)]
    pub cell_path: Vec<u32>,
    #[serde(skip)]
    pub meandered_points: Vec<[f32; 2]>,
}
