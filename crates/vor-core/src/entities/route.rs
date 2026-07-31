//! Route (slot `[37]`: `pack.routes` JSON).
//!
//! Brample example: `{"i":0,"group":"roads","feature":2,"points":[[758.56,351.83,325],...]}`.

/// Route type. Variants confirmed against Azgaar (slot `[37]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RouteGroup {
    #[default]
    Roads,
    Trails,
    Searoutes,
}

/// A route. Id `0` is reserved as "no route" in `PackCells::routes`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Route {
    pub id: u32,
    /// Group (roads/trails/searoutes).
    pub group: RouteGroup,
    /// Id of the feature (island/lake/ocean) the route passes through (in some cases like searoutes).
    #[serde(default)]
    pub feature: u32,
    /// Control points `[x, y, z]` (the `z` usually carries Azgaar's cell id; keep it opaque for now).
    pub points: Vec<[f32; 3]>,
    /// Length in canvas units (matches `d3.length`).
    #[serde(default)]
    pub length: f32,
}
