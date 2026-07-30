use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::entities::river::River;

use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};
use crate::mesh::catmull_rom_open;

/// Port of Azgaar's `meander()` from pathUtils.ts.
/// Inserts intermediate points between cell centers with perpendicular displacement.
/// Meander magnitude decreases downstream (as `step` increases).
fn meander_anchors(anchors: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let n = anchors.len();
    if n < 2 {
        return anchors.to_vec();
    }
    let meandering = 0.5f32;
    let cell_count = n;
    let mut step = 10f32;
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

        let meander_val = meandering + 1.0 / step + (meandering - step / 100.0).max(0.0);
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

fn relax_acute_angles(points: &mut [[f32; 2]], anchor_indices: &[usize]) {
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

    let acute_cost = |pos: &[[f32; 2]], i: usize| -> f32 {
        if i == 0 || i >= n - 1 {
            return 0.0;
        }
        let cos = corner_cos(pos[i - 1], pos[i], pos[i + 1]);
        if cos > 0.0 { cos } else { 0.0 }
    };

    for _ in 0..4 {
        let snapshot: Vec<[f32; 2]> = points.to_vec();
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
            let flipped = reflect_across_line(snapshot[i], snapshot[p as usize], snapshot[q as usize]);

            let before = acute_cost(&snapshot, i - 1) + acute_cost(&snapshot, i) + acute_cost(&snapshot, i + 1);

            let after = acute_cost_vec(&snapshot, i - 1, flipped, i)
                + acute_cost_vec(&snapshot, i, flipped, i)
                + acute_cost_vec(&snapshot, i + 1, flipped, i);

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

// Azgaar's exact width formulas (getOffset + getWidth from width.ts)
// LENGTH_PROGRESSION = Fibonacci / 200: [0.005, 0.005, 0.01, 0.015, 0.025, 0.04, 0.065, 0.105, 0.17]
const FLUX_FACTOR: f32 = 500.0;
const MAX_FLUX_WIDTH: f32 = 1.0;
const LENGTH_STEP_WIDTH: f32 = 1.0 / 200.0;
const LENGTH_PROGRESSION: [f32; 9] = [
    1.0 / 200.0, 1.0 / 200.0, 2.0 / 200.0, 3.0 / 200.0,
    5.0 / 200.0, 8.0 / 200.0, 13.0 / 200.0, 21.0 / 200.0, 34.0 / 200.0,
];

fn get_offset(flux: f32, point_index: usize, width_factor: f32, starting_width: f32) -> f32 {
    if point_index == 0 {
        return starting_width;
    }
    let flux_width = (flux / FLUX_FACTOR).powf(0.7).min(MAX_FLUX_WIDTH);
    let prog_idx = point_index.min(LENGTH_PROGRESSION.len() - 1);
    let length_width =
        point_index as f32 * LENGTH_STEP_WIDTH + LENGTH_PROGRESSION[prog_idx];
    width_factor * (length_width + flux_width) + starting_width
}

fn get_width(offset: f32) -> f32 {
    (offset / 1.5).powf(1.8)
}

fn acute_cost_vec(pos: &[[f32; 2]], i: usize, flipped: [f32; 2], flip_idx: usize) -> f32 {
    if i == 0 || i >= pos.len() - 1 {
        return 0.0;
    }
    let a = if i - 1 == flip_idx { flipped } else { pos[i - 1] };
    let b = if i == flip_idx { flipped } else { pos[i] };
    let c = if i + 1 == flip_idx { flipped } else { pos[i + 1] };
    let cos = corner_cos(a, b, c);
    if cos > 0.0 { cos } else { 0.0 }
}

pub fn build_river_mesh(points: &[[f32; 2]], rivers: &[River], km_per_px: f32) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };
    let mut tess = FillTessellator::new();

    for r in rivers.iter() {
        let path = &r.cell_path;
        if path.len() < 2 {
            continue;
        }
        let mut raw: Vec<[f32; 2]> = path
            .iter()
            .filter_map(|&ci| points.get(ci as usize).copied())
            .collect();
        if raw.len() < 2 {
            continue;
        }
        // Extend river past the last land cell into the sea
        // Direction = average of last 2 segments, length = 1.5x avg (min 8px)
        let ext = {
            let last = raw.last().copied().unwrap();
            let n_raw = raw.len();
            let mut dx_sum = 0.0;
            let mut dy_sum = 0.0;
            let mut seg_count = 0;
            for j in (1.max(n_raw.saturating_sub(3))..n_raw).rev() {
                let dx = raw[j][0] - raw[j - 1][0];
                let dy = raw[j][1] - raw[j - 1][1];
                if dx != 0.0 || dy != 0.0 {
                    let d = (dx * dx + dy * dy).sqrt();
                    dx_sum += dx / d;
                    dy_sum += dy / d;
                    seg_count += 1;
                }
            }
            if seg_count > 0 {
                let avg_len = {
                    let mut len_sum = 0.0;
                    let mut cnt = 0;
                    for j in (1.max(n_raw.saturating_sub(3))..n_raw).rev() {
                        let dx = raw[j][0] - raw[j - 1][0];
                        let dy = raw[j][1] - raw[j - 1][1];
                        len_sum += (dx * dx + dy * dy).sqrt();
                        cnt += 1;
                    }
                    len_sum / cnt as f32
                };
                let scale = (avg_len * 1.5).max(8.0);
                let norm = (dx_sum * dx_sum + dy_sum * dy_sum).sqrt();
                [last[0] + dx_sum / norm * scale, last[1] + dy_sum / norm * scale]
            } else {
                last
            }
        };
        raw.push(ext);
        let meandered = meander_anchors(&raw);
        let smooth = catmull_rom_open(&meandered, 4);
        let n = smooth.len();
        if n < 2 {
            continue;
        }

        let color = [0.15, 0.45, 0.85, 1.0];
        let discharge = r.discharge_m3s.max(1.0);
        let wf = r.width_factor.max(0.1);
        let sw = r.source_width_km.max(0.05);
        let k2p = km_per_px.recip();
        let cell_count = raw.len();

        // Azgaar's exact getOffset/getWidth per vertex, converted to pixel units.
        // Use effective cell index (based on t × cell_count) instead of smooth index i.
        let mut left_bank: Vec<[f32; 2]> = Vec::with_capacity(n);
        let mut right_bank: Vec<[f32; 2]> = Vec::with_capacity(n);

        for i in 0..n {
            let prev = if i == 0 { smooth[i] } else { smooth[i - 1] };
            let next = if i == n - 1 { smooth[i] } else { smooth[i + 1] };
            let angle = (prev[1] - next[1]).atan2(prev[0] - next[0]);

            let t = i as f32 / (n - 1).max(1) as f32;
            let flux = discharge * (0.1 + 0.9 * t);
            let cell_idx = ((cell_count - 1) as f32 * t).round() as usize;
            let offset = get_offset(flux, cell_idx, wf, sw);
            let width = get_width(offset) * k2p;

            let sin_o = angle.sin() * width;
            let cos_o = angle.cos() * width;
            left_bank.push([smooth[i][0] - sin_o, smooth[i][1] + cos_o]);
            right_bank.push([smooth[i][0] + sin_o, smooth[i][1] - cos_o]);
        }

        // Filled polygon: left bank forward + right bank reversed
        let mut builder = Path::builder();
        if let Some(first) = left_bank.first() {
            builder.begin(point(first[0], first[1]));
            for pt in left_bank.iter().skip(1) {
                builder.line_to(point(pt[0], pt[1]));
            }
            for pt in right_bank.iter().rev() {
                builder.line_to(point(pt[0], pt[1]));
            }
            builder.end(true);
        }
        let path = builder.build();

        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, ColorCtor(color));
        let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);
        if tess.tessellate_path(&path, &opts, &mut buffer_builder).is_err() {
            continue;
        }

        let base = result.vertices.len() as u32;
        result.vertices.extend_from_slice(&mesh.vertices);
        result.indices.extend(mesh.indices.iter().map(|i| i + base));
        for v in &mesh.vertices {
            result.bounds_min[0] = result.bounds_min[0].min(v.pos[0]);
            result.bounds_min[1] = result.bounds_min[1].min(v.pos[1]);
            result.bounds_max[0] = result.bounds_max[0].max(v.pos[0]);
            result.bounds_max[1] = result.bounds_max[1].max(v.pos[1]);
        }
    }

    if !result.bounds_min[0].is_finite() {
        result.bounds_min = [0.0, 0.0];
        result.bounds_max = [0.0, 0.0];
    }

    result
}
