use vor_core::entities::river::River;

/// Port simplificado de `specify()` de Azgaar.
pub fn specify_common(rivers: &mut [River]) {
    for i in 0..rivers.len() {
        let id = rivers[i].id;
        let basin = rivers[i].parent_river.map_or(id, |p| {
            rivers
                .iter()
                .find(|r| r.id == p)
                .map(|r| r.basin_id)
                .unwrap_or(id)
        });
        rivers[i].basin_id = basin;
        if rivers[i].name.is_empty() {
            rivers[i].name = format!("River {}", id);
        }
        if rivers[i].type_name.is_empty() {
            rivers[i].type_name = "River".into();
        }
    }
}

pub fn remove_river(rivers: &mut Vec<River>, river_id: u16) {
    let cascade: Vec<u16> = rivers
        .iter()
        .filter(|r| r.id == river_id || r.parent_river == Some(river_id) || r.basin_id == river_id)
        .map(|r| r.id)
        .collect();
    rivers.retain(|r| !cascade.contains(&r.id));
}

pub fn get_next_id(rivers: &[River]) -> u16 {
    rivers.iter().map(|r| r.id).max().unwrap_or(0) + 1
}

pub fn get_approximate_length(points: &[[f32; 2]]) -> f32 {
    if points.len() < 2 {
        return 0.0;
    }
    let mut len = 0.0;
    for w in points.windows(2) {
        let dx = w[1][0] - w[0][0];
        let dy = w[1][1] - w[0][1];
        len += (dx * dx + dy * dy).sqrt();
    }
    len
}
