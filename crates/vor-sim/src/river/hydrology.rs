use vor_core::cells::PackCells;
use vor_core::feature::FeatureType;
use vor_core::grid::Grid;
use vor_core::pack::Pack;

use crate::river::MIN_FLUX_TO_FORM_RIVER;

pub fn alter_heights(pack: &Pack) -> Vec<f32> {
    let cells = &pack.cells;
    cells
        .height
        .iter()
        .enumerate()
        .map(|(i, &h)| {
            if h < 20 {
                h as f32
            } else {
                let t_i = if h > 50 { 10.0 } else { 5.0 };
                let mean_t = cells
                    .adjacency
                    .get(i)
                    .map(|neighbors| {
                        if neighbors.is_empty() {
                            return t_i;
                        }
                        let sum: f32 = neighbors
                            .iter()
                            .filter_map(|&n| cells.height.get(n as usize))
                            .map(|&nh| if nh >= 20 { 10.0 } else { 1.0 })
                            .sum();
                        sum / neighbors.len() as f32
                    })
                    .unwrap_or(t_i);
                h as f32 + t_i / 100.0 + mean_t / 10000.0
            }
        })
        .collect()
}

pub fn resolve_depressions(pack: &Pack, h: &mut Vec<f32>) {
    let cells = &pack.cells;
    let n = cells.len();
    if n == 0 {
        return;
    }
    let max_iterations = n.min(500);
    let mut land: Vec<usize> = (0..n)
        .filter(|&i| h[i] >= 20.0 && !cells.adjacency.get(i).map_or(true, |a| a.is_empty()))
        .collect();
    land.sort_by(|&a, &b| h[a].partial_cmp(&h[b]).unwrap_or(std::cmp::Ordering::Equal));

    for _ in 0..max_iterations {
        let mut depressions: u32 = 0;
        for &i in &land {
            if h[i] >= 100.0 {
                continue;
            }
            let mut min_neighbor = f32::MAX;
            if let Some(neighbors) = cells.adjacency.get(i) {
                for &n in neighbors {
                    let n = n as usize;
                    if n < h.len() && h[n] < min_neighbor {
                        min_neighbor = h[n];
                    }
                }
            }
            if min_neighbor >= 100.0 || h[i] > min_neighbor {
                continue;
            }
            depressions += 1;
            h[i] = min_neighbor + 0.1;
        }
        if depressions == 0 {
            break;
        }
    }
}

/// Port de Azgaar Lakes.defineClimateData (lakes.ts:49-84)
pub fn define_lake_climate(pack: &mut Pack, h: &[f32], grid: &Grid) {
    for feat in &mut pack.features {
        if feat.kind != FeatureType::Lake {
            continue;
        }
        let mut flux: f32 = 0.0;
        for &sc in &feat.shoreline {
            let sc = sc as usize;
            let gid = pack.cells.grid_id.get(sc).copied().unwrap_or(0) as usize;
            let prec = grid.cells.precipitation.get(gid).copied().unwrap_or(0);
            flux += prec as f32;
        }
        feat.entering_flux = flux;

        let temp: f32 = feat
            .shoreline
            .first()
            .map(|&sc| {
                let gid = pack.cells.grid_id.get(sc as usize).copied().unwrap_or(0) as usize;
                grid.cells.temperature.get(gid).copied().unwrap_or(20) as f32
            })
            .unwrap_or(20.0);

        let h_m = ((feat.lake_height - 18.0).max(0.0)).powf(0.5);
        let evap = ((700.0 * (temp + 0.006 * h_m)) / 50.0 + 75.0) / (80.0 - temp);
        let evaporation = evap * feat.cell_count as f32;

        let lowest = feat
            .shoreline
            .iter()
            .min_by(|&&a, &&b| {
                h[a as usize]
                    .partial_cmp(&h[b as usize])
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied();

        feat.closed = false;
        if !feat.touches_border {
            if evaporation >= flux {
                feat.closed = true;
            } else if let Some(lc) = lowest {
                feat.out_cell = Some(lc);
            }
        }
    }
}

/// Port de Azgaar Lakes.detectCloseLakes (lakes.ts:87-126)
pub fn detect_close_lakes(pack: &Pack, h: &[f32]) -> Vec<bool> {
    let mut closed = vec![false; pack.features.len()];
    for (fi, feat) in pack.features.iter().enumerate() {
        if feat.kind != FeatureType::Lake {
            continue;
        }
        if feat.touches_border {
            continue;
        }
        let elevation_limit = 20.0f32;
        let max_h = feat.lake_height + elevation_limit;
        if max_h >= 99.0 {
            continue;
        }
        // BFS desde lowest shoreline cell para ver si alcanza océano
        let mut visited = vec![false; pack.cells.len()];
        let mut queue = std::collections::VecDeque::new();
        if let Some(&start) = feat.shoreline.iter().min_by(|&&a, &&b| {
            h[a as usize]
                .partial_cmp(&h[b as usize])
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            queue.push_back(start as usize);
            visited[start as usize] = true;
        }
        let mut is_deep = true;
        while let Some(cell) = queue.pop_front() {
            let cell_feat_id = pack.cells.feature_id.get(cell).copied().unwrap_or(0) as usize;
            if let Some(cf) = pack.features.get(cell_feat_id) {
                if cf.kind == FeatureType::Ocean
                    || (cf.kind == FeatureType::Lake
                        && cf.id != feat.id
                        && cf.lake_height < feat.lake_height)
                {
                    is_deep = false;
                    break;
                }
            }
            if let Some(neighbors) = pack.cells.adjacency.get(cell) {
                for &n in neighbors {
                    let n = n as usize;
                    if !visited[n] && h[n] < max_h {
                        visited[n] = true;
                        queue.push_back(n);
                    }
                }
            }
        }
        closed[fi] = is_deep;
    }
    closed
}

pub fn drain_water(pack: &mut Pack, h: &mut [f32], grid: &Grid) {
    let n = pack.cells.len();
    if n == 0 {
        return;
    }
    if pack.cells.flux.is_empty() {
        pack.cells.flux = vec![0u16; n];
    }
    if pack.cells.river.is_empty() {
        pack.cells.river = vec![0u16; n];
    }
    if pack.cells.confluence.is_empty() {
        pack.cells.confluence = vec![0u16; n];
    }

    let modifier = ((n as f32) / 10000.0).powf(0.25);

    let mut river_next: u16 = 1;
    let mut river_parents: Vec<Option<u16>> = Vec::new();

    let prec: Vec<u16> = (0..n)
        .map(|i| {
            if h[i] >= 20.0 {
                let gid = pack.cells.grid_id.get(i).copied().unwrap_or(0) as usize;
                grid.cells.precipitation.get(gid).copied().unwrap_or(0)
            } else {
                0
            }
        })
        .collect();

    // Lake outlet map: cell_id -> lake outlet river_id
    // Precompute lake out_cells
    let mut lake_out_cells: Vec<Option<u32>> = vec![None; n];
    for (fi, feat) in pack.features.iter().enumerate() {
        if feat.kind == FeatureType::Lake && !feat.closed {
            if let Some(out) = feat.out_cell {
                lake_out_cells[out as usize] = Some(feat.id);
            }
        }
    }

    let mut land: Vec<usize> = (0..n).filter(|&i| h[i] >= 20.0).collect();
    land.sort_by(|&a, &b| h[b].partial_cmp(&h[a]).unwrap_or(std::cmp::Ordering::Equal));

    for &i in &land {
        let precip = prec[i] as f32 / modifier;
        if precip > 0.0 {
            pack.cells.flux[i] = (pack.cells.flux[i] as f32 + precip).min(u16::MAX as f32) as u16;
        }

        // Lake outlet handling
        if let Some(lake_id) = lake_out_cells[i] {
            let li = lake_id as usize;
            if let Some(lake_feat) = pack.features.get(li) {
                let mut lake_cell = None;
                if let Some(neighbors) = pack.cells.adjacency.get(i) {
                    for &n in neighbors {
                        let n = n as usize;
                        if n < h.len() && h[n] < 20.0 {
                            let nfid = pack.cells.feature_id.get(n).copied().unwrap_or(0) as usize;
                            if pack.features.get(nfid).map_or(false, |f| f.id == lake_id) {
                                lake_cell = Some(n);
                                break;
                            }
                        }
                    }
                }
                if let Some(lc) = lake_cell {
                    let lake_flux =
                        (lake_feat.entering_flux - 0.0/*evap omitted for now*/).max(0.0) as u16;
                    pack.cells.flux[lc] =
                        (pack.cells.flux[lc] as f32 + lake_flux as f32).min(u16::MAX as f32) as u16;
                    if pack.cells.river[lc] == 0 {
                        pack.cells.river[lc] = river_next;
                        river_next += 1;
                    }
                    // Flow down from lake outlet
                    let outlet_river = pack.cells.river[lc];
                    let mut min_h = h[i];
                    let mut down = i;
                    if let Some(neighbors) = pack.cells.adjacency.get(i) {
                        for &n in neighbors {
                            let n = n as usize;
                            if n < h.len() && h[n] < min_h {
                                min_h = h[n];
                                down = n;
                            }
                        }
                    }
                    if down != i {
                        flow_down(&mut pack.cells, h, down, i);
                    }
                }
            }
        }

        if pack.cells.flux[i] >= MIN_FLUX_TO_FORM_RIVER as u16 && pack.cells.river[i] == 0 {
            pack.cells.river[i] = river_next;
            if river_next as usize >= river_parents.len() {
                river_parents.resize(river_next as usize + 1, None);
            }
            river_next += 1;
        }

        // Find downhill
        let mut min_h = h[i];
        let mut down = i;
        if let Some(neighbors) = pack.cells.adjacency.get(i) {
            for &n in neighbors {
                let n = n as usize;
                if n < h.len() && h[n] < min_h {
                    min_h = h[n];
                    down = n;
                }
            }
        }
        if down != i && h[i] > h[down] {
            flow_down(&mut pack.cells, h, down, i);
        }
    }
}

fn flow_down(cells: &mut PackCells, h: &[f32], to_cell: usize, from_cell: usize) {
    let from_flux = cells.flux[from_cell];
    let from_river = cells.river[from_cell];
    let to_flux = cells.flux[to_cell].saturating_sub(cells.confluence[to_cell]);
    let to_river = cells.river[to_cell];

    if to_river != 0 {
        if from_flux > to_flux {
            cells.confluence[to_cell] =
                (cells.confluence[to_cell] as u16 + cells.flux[to_cell]).min(u16::MAX);
            cells.river[to_cell] = from_river;
        } else {
            cells.confluence[to_cell] =
                (cells.confluence[to_cell] as u16 + from_flux).min(u16::MAX);
        }
    } else {
        cells.river[to_cell] = from_river;
    }

    if to_cell < h.len() && h[to_cell] >= 20.0 {
        cells.flux[to_cell] =
            (cells.flux[to_cell] as f32 + from_flux as f32).min(u16::MAX as f32) as u16;
    }
}
