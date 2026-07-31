use vor_core::entities::river::River;
use vor_core::feature::FeatureType;
use vor_core::pack::Pack;

/// Port de Azgaar `resolveLakeDrainFeature()`. river-generator.ts:535-557
pub fn resolve_lake_drain_feature(
    pack: &Pack,
    rivers: &[River],
    lake_feature_id: u32,
) -> Option<u32> {
    let lake = pack.features.get(lake_feature_id as usize)?;
    if lake.kind != FeatureType::Lake {
        return None;
    }
    let outlet = lake.outlet_river?;

    let mut visited = vec![false; rivers.len() + 1];
    let mut river_id = outlet;

    loop {
        if river_id as usize >= visited.len() || visited[river_id as usize] {
            return None;
        }
        visited[river_id as usize] = true;
        let river = rivers.iter().find(|r| r.id == river_id)?;
        let last_cell = river.cell_path.last().copied()?;
        if last_cell == u32::MAX {
            return None;
        }
        let feat_id = *pack.cells.feature_id.get(last_cell as usize)?;
        let feat = pack.features.get(feat_id as usize)?;
        match feat.kind {
            FeatureType::Ocean => return Some(feat.id),
            FeatureType::Lake => {
                if let Some(next_outlet) = feat.outlet_river {
                    river_id = next_outlet;
                } else {
                    return Some(feat.id);
                }
            }
            _ => return None,
        }
    }
}

/// Port de Azgaar `resolveDrainFeature()`. river-generator.ts:560-582
pub fn resolve_drain_feature(pack: &Pack, rivers: &[River], cell_id: u32) -> Option<u32> {
    let start_river = pack.cells.river.get(cell_id as usize).copied()?;
    if start_river == 0 {
        return None;
    }
    let mut visited = vec![false; rivers.len() + 1];
    let mut river_id = start_river;

    loop {
        if river_id as usize >= visited.len() || visited[river_id as usize] {
            return None;
        }
        visited[river_id as usize] = true;
        let river = rivers.iter().find(|r| r.id == river_id)?;
        let last_cell = river.cell_path.last().copied()?;
        if last_cell == u32::MAX {
            return None;
        }
        let feat_id = *pack.cells.feature_id.get(last_cell as usize)?;
        let feat = pack.features.get(feat_id as usize)?;
        match feat.kind {
            FeatureType::Ocean => return Some(feat.id),
            FeatureType::Lake => {
                if let Some(next_outlet) = feat.outlet_river {
                    river_id = next_outlet;
                } else {
                    return Some(feat.id);
                }
            }
            _ => return None,
        }
    }
}

/// Port de Azgaar `isNavigable()`.
pub fn is_navigable(pack: &Pack, cell_id: u32) -> bool {
    let c = cell_id as usize;
    c < pack.cells.river.len()
        && pack.cells.river[c] != 0
        && pack.cells.flux[c] >= crate::river::MIN_NAVIGABLE_FLUX
}
