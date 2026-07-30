use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LandGroup {
    Continent,
    Island,
    Isle,
    LakeIsland,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LakeGroup {
    Freshwater,
    Salt,
    Dry,
    Sinkhole,
    Lava,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FeatureType {
    #[default]
    Ocean,
    Island,
    Lake,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Feature {
    pub id: u32,
    pub is_land: bool,
    pub touches_border: bool,
    pub kind: FeatureType,
    pub land_group: Option<LandGroup>,
    pub lake_group: Option<LakeGroup>,
    pub cell_count: u32,
    pub first_cell: u32,
    pub perimeter_vertices: Vec<u32>,
    pub name: Option<String>,
    #[serde(default)]
    pub shoreline: Vec<u32>,
    #[serde(default)]
    pub lake_height: f32,
    #[serde(default)]
    pub inlets: Vec<u16>,
    #[serde(default)]
    pub outlet_river: Option<u16>,
    #[serde(default)]
    pub entering_flux: f32,
    #[serde(skip)]
    pub closed: bool,
    #[serde(skip)]
    pub out_cell: Option<u32>,
}

impl Feature {
    pub fn placeholder() -> Self {
        Self {
            id: 0,
            is_land: false,
            touches_border: false,
            kind: FeatureType::Ocean,
            land_group: None,
            lake_group: None,
            cell_count: 0,
            first_cell: 0,
            perimeter_vertices: Vec::new(),
            name: None,
            shoreline: Vec::new(),
            lake_height: 0.0,
            inlets: Vec::new(),
            outlet_river: None,
            entering_flux: 0.0,
            closed: false,
            out_cell: None,
        }
    }
}
