/// Port del `meander()` de Azgaar pathUtils.ts:370-429
pub fn meander_anchors(anchors: &[[f32; 2]], is_water: &[bool]) -> Vec<[f32; 2]> {
    let n = anchors.len();
    if n < 2 {
        return anchors.to_vec();
    }
    let meandering = 0.5f32;
    let cell_count = n;
    let mut step = if is_water.first().copied().unwrap_or(false) {
        1.0
    } else {
        10.0
    };
    let mut result: Vec<[f32; 2]> = Vec::new();
    let mut anchor_indices: Vec<usize> = Vec::new();

    for i in 0..n {
        let p1 = anchors[i];
        anchor_indices.push(result.len());
        result.push(p1);
        if i == n - 1 {
            break;
        }
        let p2 = anchors[i + 1];
        let dx = p2[0] - p1[0];
        let dy = p2[1] - p1[1];
        let dist2 = dx * dx + dy * dy;
        if dist2 <= 25.0 && cell_count >= 6 {
            step += 1.0;
            continue;
        }
        let mut meander_val = meandering + 1.0 / step + (meandering - step / 100.0).max(0.0);
        if i < is_water.len() && is_water[i] {
            meander_val *= 0.25;
        }
        let angle = dy.atan2(dx);
        let sin_m = angle.sin() * meander_val;
        let cos_m = angle.cos() * meander_val;

        if step < 20.0 && (dist2 > 64.0 || (dist2 > 36.0 && cell_count < 5)) {
            let p1x = (p1[0] * 2.0 + p2[0]) / 3.0 - sin_m;
            let p1y = (p1[1] * 2.0 + p2[1]) / 3.0 + cos_m;
            let p2x = (p1[0] + p2[0] * 2.0) / 3.0 + sin_m * 0.5;
            let p2y = (p1[1] + p2[1] * 2.0) / 3.0 - cos_m * 0.5;
            result.push([p1x, p1y]);
            result.push([p2x, p2y]);
        } else if dist2 > 25.0 || cell_count < 6 {
            let mx = (p1[0] + p2[0]) * 0.5 - sin_m;
            let my = (p1[1] + p2[1]) * 0.5 + cos_m;
            result.push([mx, my]);
        }
        step += 1.0;
    }
    relax_acute_angles(&mut result, &anchor_indices);
    result
}

fn corner_cos(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    let ax = a[0] - b[0];
    let ay = a[1] - b[1];
    let cx = c[0] - b[0];
    let cy = c[1] - b[1];
    let la = (ax * ax + ay * ay).sqrt();
    let lc = (cx * cx + cy * cy).sqrt();
    if la == 0.0 || lc == 0.0 {
        return -1.0;
    }
    (ax * cx + ay * cy) / (la * lc)
}

fn reflect_across_line(m: [f32; 2], p: [f32; 2], q: [f32; 2]) -> [f32; 2] {
    let dx = q[0] - p[0];
    let dy = q[1] - p[1];
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return m;
    }
    let t = ((m[0] - p[0]) * dx + (m[1] - p[1]) * dy) / len2;
    let foot_x = p[0] + t * dx;
    let foot_y = p[1] + t * dy;
    [2.0 * foot_x - m[0], 2.0 * foot_y - m[1]]
}

/// Port de `addMeandering()` de Azgaar: meanderea + interpola flux.
/// Retorna `[x, y, flux]` por punto meandereado.
pub fn add_meandering(
    anchors: &[[f32; 2]],
    cell_ids: &[u32],
    cell_flux: &[u16],
    cell_heights: &[u8],
) -> Vec<[f32; 3]> {
    let is_water: Vec<bool> = cell_ids
        .iter()
        .map(|&c| {
            if c == u32::MAX {
                true
            } else {
                cell_heights.get(c as usize).map_or(true, |&h| h < 20)
            }
        })
        .collect();
    let points = meander_anchors(anchors, &is_water);
    let mut flux = vec![0.0f32; points.len()];
    let step = points.len().max(1) as f32;
    // Interpolar flux en los anchor points
    for (i, _cid) in cell_ids.iter().enumerate() {
        let anchor_idx = if i == 0 {
            0
        } else {
            (i as f32 * step / cell_ids.len().max(1) as f32) as usize
        };
        if anchor_idx < flux.len() {
            let cell_id = cell_ids[i];
            let f = if cell_id == u32::MAX {
                cell_ids
                    .get(i.wrapping_sub(1))
                    .and_then(|&p| cell_flux.get(p as usize))
            } else {
                cell_flux.get(cell_id as usize)
            };
            flux[anchor_idx] = f.copied().unwrap_or(0) as f32;
        }
    }
    // Hacer flux monotónico creciente
    let mut max_f = 0.0f32;
    for f in &mut flux {
        if *f > max_f {
            max_f = *f;
        } else {
            *f = max_f;
        }
    }
    points
        .iter()
        .enumerate()
        .map(|(i, &[x, y])| [x, y, flux[i]])
        .collect()
}

/// Port de `relaxAcuteAngles()` de pathUtils.ts:453-506
pub fn relax_acute_angles(points: &mut Vec<[f32; 2]>, anchor_indices: &[usize]) {
    let n = points.len();
    if n < 3 {
        return;
    }
    let mut is_anchor = vec![false; n];
    for &idx in anchor_indices {
        is_anchor[idx] = true;
    }
    let mut prev_anchor = vec![-1i32; n];
    let mut next_anchor = vec![-1i32; n];
    let mut last = -1i32;
    for i in 0..n {
        prev_anchor[i] = last;
        if is_anchor[i] {
            last = i as i32;
        }
    }
    let mut last = -1i32;
    for i in (0..n).rev() {
        next_anchor[i] = last;
        if is_anchor[i] {
            last = i as i32;
        }
    }

    for _ in 0..4 {
        let snapshot: Vec<[f32; 2]> = points.clone();
        let mut flipped_any = false;
        for i in 1..n - 1 {
            if is_anchor[i] {
                continue;
            }
            let p = prev_anchor[i];
            let q = next_anchor[i];
            if p < 0 || q < 0 {
                continue;
            }
            let flipped =
                reflect_across_line(snapshot[i], snapshot[p as usize], snapshot[q as usize]);
            let before = corner_cos(snapshot[i - 1], snapshot[i], snapshot[i + 1]).max(0.0);
            let after = corner_cos(
                if i - 1 == i { flipped } else { snapshot[i - 1] },
                flipped,
                if i + 1 == i { flipped } else { snapshot[i + 1] },
            )
            .max(0.0);
            if after < before - 1e-6 {
                points[i] = flipped;
                flipped_any = true;
            }
        }
        if !flipped_any {
            break;
        }
    }
}
