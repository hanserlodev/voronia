//! State labels (FMG `#labels > #states`, `draw-state-labels.ts` port 1:1).
//!
//! Path = [ray1end, pole, ray2end]: two near-opposite rays scored as a pair
//! (length x horizontality x curvature), probed with find_closest_cell
//! (uniform grid = FMG quadtree), smoothed with a natural cubic spline
//! (d3 curveNatural). Text renders per-character along the arc
//! (startOffset 50%) with real per-glyph UVs; 2-line labels offset
//! perpendicular (tspan dy); fallback to 1 line via the 6-sample check.

use vor_core::entities::state::State;
use vor_core::pack::Pack;

const ANGLE_STEP: f32 = 9.0;
const LENGTH_START: f32 = 5.0;
const LENGTH_STEP: f32 = 5.0;
const LENGTH_MAX: f32 = 300.0;
pub const STATE_LABEL_SIZE: f32 = 22.0;

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
}

pub struct CellGrid {
    cells_x: usize,
    cell_size: f32,
    buckets: Vec<Vec<u32>>,
    points: Vec<[f32; 2]>,
}

impl CellGrid {
    pub fn new(points: &[[f32; 2]], width: f32, height: f32, cell_size: f32) -> Self {
        let cells_x = ((width / cell_size).ceil() as usize).max(1);
        let cells_y = ((height / cell_size).ceil() as usize).max(1);
        let mut buckets = vec![Vec::new(); cells_x * cells_y];
        for (i, p) in points.iter().enumerate() {
            let gx = ((p[0] / cell_size) as usize).min(cells_x - 1);
            let gy = ((p[1] / cell_size) as usize).min(cells_y - 1);
            buckets[gy * cells_x + gx].push(i as u32);
        }
        Self {
            cells_x,
            cell_size,
            buckets,
            points: points.to_vec(),
        }
    }

    pub fn find_closest(&self, x: f32, y: f32) -> Option<usize> {
        let gx = ((x / self.cell_size) as i32).max(0) as usize;
        let gy = ((y / self.cell_size) as i32).max(0) as usize;
        let rows = self.buckets.len() / self.cells_x;
        let mut best: Option<(usize, f32)> = None;
        for oy in -1..=1i32 {
            for ox in -1..=1i32 {
                let cx = gx as i32 + ox;
                let cy = gy as i32 + oy;
                if cx < 0 || cy < 0 || cx as usize >= self.cells_x || cy as usize >= rows {
                    continue;
                }
                for &i in &self.buckets[cy as usize * self.cells_x + cx as usize] {
                    let p = self.points[i as usize];
                    let d2 = (p[0] - x).powi(2) + (p[1] - y).powi(2);
                    if best.map(|(_, bd)| d2 < bd).unwrap_or(true) {
                        best = Some((i as usize, d2));
                    }
                }
            }
        }
        best.map(|(i, _)| i)
    }
}

pub fn natural_spline(points: &[[f32; 2]], sub: usize) -> Vec<[f32; 2]> {
    let n = points.len();
    if n < 3 || sub == 0 {
        return points.to_vec();
    }
    let mut h = vec![0.0f32; n - 1];
    for i in 0..n - 1 {
        h[i] = dist(points[i + 1], points[i]).max(1e-6);
    }
    let (mut rx, mut ry) = (vec![0.0f32; n], vec![0.0f32; n]);
    for i in 1..n - 1 {
        rx[i] = 6.0
            * ((points[i + 1][0] - points[i][0]) / h[i]
                - (points[i][0] - points[i - 1][0]) / h[i - 1]);
        ry[i] = 6.0
            * ((points[i + 1][1] - points[i][1]) / h[i]
                - (points[i][1] - points[i - 1][1]) / h[i - 1]);
    }
    let (mut cp, mut dx, mut dy) = (vec![0.0f32; n], vec![0.0f32; n], vec![0.0f32; n]);
    cp[1] = 2.0 * (h[0] + h[1]);
    dx[1] = rx[1];
    dy[1] = ry[1];
    for i in 2..n - 1 {
        let m = h[i - 1] / cp[i - 1];
        cp[i] = 2.0 * (h[i - 1] + h[i]) - h[i - 1] * m;
        dx[i] = rx[i] - dx[i - 1] * m;
        dy[i] = ry[i] - dy[i - 1] * m;
    }
    let (mut mx, mut my) = (vec![0.0f32; n], vec![0.0f32; n]);
    for i in (1..n - 1).rev() {
        mx[i] = (dx[i] - h[i] * mx[i + 1]) / cp[i];
        my[i] = (dy[i] - h[i] * my[i + 1]) / cp[i];
    }
    let mut out = Vec::with_capacity((n - 1) * (sub + 1));
    out.push(points[0]);
    for i in 0..n - 1 {
        let hi = h[i];
        for j in 1..=sub {
            let t = j as f32 / sub as f32;
            // S(t) = lerp - h^2/6 * t(1-t) * [M_i(2-t) + M_{i+1}(1+t)]
            let bend = hi * hi / 6.0 * t * (1.0 - t);
            let cm = bend * (mx[i] * (2.0 - t) + mx[i + 1] * (1.0 + t));
            let cy2 = bend * (my[i] * (2.0 - t) + my[i + 1] * (1.0 + t));
            out.push([
                points[i][0] * (1.0 - t) + points[i + 1][0] * t - cm,
                points[i][1] * (1.0 - t) + points[i + 1][1] * t - cy2,
            ]);
        }
    }
    out
}

fn score_angle(angle: f32) -> f32 {
    let norm = (angle % 180.0).abs();
    let h = (norm - 90.0).abs() / 90.0;
    if (h - 1.0).abs() < 1e-6 {
        return 1.0;
    }
    if h >= 0.75 {
        0.9
    } else if h >= 0.5 {
        0.6
    } else if h >= 0.25 {
        0.5
    } else if h >= 0.15 {
        0.2
    } else {
        0.1
    }
}

fn score_curv(a1: f32, a2: f32) -> f32 {
    let mut d = (a1 - a2).abs() % 360.0;
    if d > 180.0 {
        d = 360.0 - d;
    }
    let p1 = (a1 % 180.0 - 90.0).abs();
    let p2 = (a2 % 180.0 - 90.0).abs();
    let sim = 1.0 - (p1 - p2).abs() / 90.0;
    if (d - 180.0).abs() < 1e-6 {
        1.0
    } else if d < 90.0 {
        0.0
    } else if d < 120.0 {
        0.6 * sim
    } else if d < 140.0 {
        0.7 * sim
    } else if d < 160.0 {
        0.8 * sim
    } else {
        sim
    }
}

pub fn split_in_two(s: &str) -> Vec<String> {
    let half = s.chars().count() as f32 / 2.0;
    let ar: Vec<&str> = s.split(' ').collect();
    if ar.len() < 2 {
        return vec![s.to_string()];
    }
    let (mut first, mut middle, mut last, mut rest) =
        (String::new(), String::new(), String::new(), String::new());
    for (d, w) in ar.iter().enumerate() {
        let mut w = w.to_string();
        if d + 1 != ar.len() {
            w.push(' ');
        }
        rest.push_str(&w);
        if first.is_empty() || (rest.len() as f32) < half {
            first.push_str(&w);
        } else if middle.is_empty() {
            middle = w;
        } else {
            last.push_str(&w);
        }
    }
    if last.is_empty() {
        return vec![first, middle];
    }
    if first.len() < last.len() {
        vec![first + &middle, last]
    } else {
        vec![first, middle + &last]
    }
}

fn minmax(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi).round()
}

/// Computes the smoothed label path per state (FMG getLabelPaths + prolong).
pub fn compute_state_label_paths(
    pack: &Pack,
    states: &[State],
    letter_length: f32,
) -> Vec<Vec<[f32; 2]>> {
    let w_max = pack.points.iter().fold(0.0f32, |m, p| m.max(p[0]));
    let h_max = pack.points.iter().fold(0.0f32, |m, p| m.max(p[1]));
    let grid = CellGrid::new(&pack.points, w_max, h_max, 25.0);
    let n = pack.points_n();
    let state_ids = &pack.cells.state;
    let feature_of = |c: usize| pack.cells.feature_id.get(c).copied().unwrap_or(0) as usize;
    let mut out = Vec::with_capacity(states.len());

    for st in states {
        if st.id == 0 || st.removed {
            out.push(Vec::new());
            continue;
        }
        let cells_n = (0..n)
            .filter(|&c| state_ids.get(c).copied().unwrap_or(0) == st.id)
            .count();
        let offset = if cells_n < 40 {
            0.0
        } else if cells_n < 200 {
            5.0
        } else {
            10.0
        };
        let max_lake = cells_n as f32 / 20.0;

        let inside = |x: f32, y: f32| -> bool {
            if x < 0.0 || x > w_max || y < 0.0 || y > h_max {
                return false;
            }
            let Some(cell) = grid.find_closest(x, y) else {
                return false;
            };
            let fid = feature_of(cell);
            if let Some(f) = pack.features.get(fid) {
                if f.kind == vor_core::feature::FeatureType::Lake {
                    // Inner lake: every shoreline vertex touches only this
                    // state's cells (approximated via adjacent cells of the
                    // perimeter vertices).
                    let inner = f.perimeter_vertices.iter().all(|&v| {
                        pack.vertices
                            .adjacent_cells
                            .get(v as usize)
                            .map(|cs| {
                                cs.iter().all(|&c| {
                                    c < 0
                                        || state_ids.get(c as usize).copied().unwrap_or(0) == st.id
                                })
                            })
                            .unwrap_or(false)
                    });
                    let _ = max_lake;
                    return inner;
                }
            }
            state_ids.get(cell).copied().unwrap_or(0) == st.id
        };

        // Fallback: importer may leave pole at (0,0) if the .map lacks it.
        let [mut px, mut py] = st.pole_of_inaccessibility;
        if px == 0.0 && py == 0.0 {
            if let Some(p) = pack.points.get(st.center_cell as usize) {
                px = p[0];
                py = p[1];
            }
        }
        let mut rays: Vec<(f32, f32, f32, f32)> = Vec::new(); // angle, len, x, y
        let mut angle = 0.0f32;
        while angle < 360.0 {
            let (dx, dy) = (angle.to_radians().cos(), angle.to_radians().sin());
            let mut len = 0.0f32;
            let (mut ex, mut ey) = (px, py);
            let mut l = LENGTH_START;
            while l < LENGTH_MAX {
                let (x, y) = (px + l * dx, py + l * dy);
                let o1 = (x - dy * offset, y + dx * offset);
                let o2 = (x + dy * offset, y - dx * offset);
                if !inside(x, y) || !inside(o1.0, o1.1) || !inside(o2.0, o2.1) {
                    break;
                }
                len = l;
                ex = x;
                ey = y;
                l += LENGTH_STEP;
            }
            rays.push((angle, len, ex, ey));
            angle += ANGLE_STEP;
        }

        type RayPair = ((f32, f32, f32, f32), (f32, f32, f32, f32));
        let mut best: Option<RayPair> = None;
        let mut best_score = -f32::INFINITY;
        for i in 0..rays.len() {
            let s1 = rays[i].1 * score_angle(rays[i].0);
            for j in i + 1..rays.len() {
                let s2 = rays[j].1 * score_angle(rays[j].0);
                let pair = (s1 + s2) * score_curv(rays[i].0, rays[j].0);
                if pair > best_score {
                    best_score = pair;
                    best = Some((rays[i], rays[j]));
                }
            }
        }
        let Some((r1, r2)) = best else {
            out.push(Vec::new());
            continue;
        };

        let mut path = vec![[r1.2, r1.3], [px, py], [r2.2, r2.3]];
        if r1.2 > r2.2 {
            path.reverse();
        }

        // Modes + ratio (auto).
        let path_px = {
            let sp = natural_spline(&path, 8);
            sp.windows(2).map(|w| dist(w[0], w[1])).sum::<f32>()
        };
        let path_letters = path_px / letter_length.max(1e-6);
        let full = &st.full_name;
        let name = &st.name;
        let (lines, ratio): (Vec<String>, f32) = if path_letters > full.chars().count() as f32 * 2.0
        {
            let r = path_letters / full.chars().count() as f32;
            (vec![full.clone()], minmax(r * 70.0, 70.0, 170.0))
        } else {
            let lines = split_in_two(full);
            let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1) as f32;
            let r = path_letters / longest;
            (lines, minmax(r * 60.0, 70.0, 150.0))
        };
        let _ = (name, ratio);

        // Prolongation if path shorter than the longest line.
        let longest_px = lines
            .iter()
            .map(|l| l.chars().count() as f32 * letter_length)
            .fold(0.0f32, f32::max);
        // Degenerate path (all probes failed): skip instead of exploding.
        if path_px < letter_length * 2.0 {
            out.push(Vec::new());
            continue;
        }
        if path_px < longest_px {
            let (p1, p2) = (path[0], path[path.len() - 1]);
            let (dx, dy) = ((p2[0] - p1[0]) / 2.0, (p2[1] - p1[1]) / 2.0);
            let m = longest_px / path_px;
            path[0] = [p1[0] + dx - dx * m, p1[1] + dy - dy * m];
            let last = path.len() - 1;
            path[last] = [p2[0] - dx + dx * m, p2[1] - dy + dy * m];
        }

        out.push(natural_spline(&path, 8));
    }
    out
}

pub struct GlyphCell {
    pub pen: u32,
    pub bm_w: u32,
    pub adv: u32,
    pub ymin: i32,
    pub h: u32,
}

pub struct LabelStrip {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub chars: Vec<GlyphCell>,
}

pub fn rasterize_text(font: &fontdue::Font, text: &str, px: f32) -> LabelStrip {
    let vlm = font.vertical_line_metrics(px);
    let ascent = vlm.as_ref().map(|m| m.ascent).unwrap_or(px * 0.8);
    let descent = vlm.as_ref().map(|m| m.descent).unwrap_or(-px * 0.2);
    let height = (ascent - descent).ceil() as u32 + 4;
    let baseline: i32 = ascent.round() as i32;
    let mut width = 0u32;
    let mut chars = Vec::new();
    let mut glyphs = Vec::new();
    for c in text.chars() {
        let (m, bm) = font.rasterize(c, px);
        chars.push(GlyphCell {
            pen: width,
            bm_w: m.width as u32,
            adv: m.advance_width as u32 + 1,
            ymin: m.ymin,
            h: m.height as u32,
        });
        width += m.advance_width as u32 + 1;
        glyphs.push((m, bm));
    }
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for (gi, (_, bm)) in glyphs.iter().enumerate() {
        let g = &chars[gi];
        let y0 = (baseline - g.ymin - g.h as i32).max(0) as u32;
        for row in 0..g.h {
            for col in 0..g.bm_w {
                let a = bm[(row * g.bm_w + col) as usize];
                if a == 0 {
                    continue;
                }
                let dx = g.pen + col;
                let dy = y0 + row;
                if dx < width && dy < height {
                    let d = ((dy * width + dx) * 4) as usize;
                    rgba[d] = 0x3e;
                    rgba[d + 1] = 0x3e;
                    rgba[d + 2] = 0x4b;
                    rgba[d + 3] = a;
                }
            }
        }
    }
    LabelStrip {
        rgba,
        width,
        height,
        chars,
    }
}

pub fn rasterize_strips(font_bytes: &[u8], names: &[String], px: f32) -> Vec<(String, LabelStrip)> {
    match fontdue::Font::from_bytes(
        font_bytes,
        fontdue::FontSettings {
            collection_index: 0,
            ..Default::default()
        },
    ) {
        Ok(f) => names
            .iter()
            .map(|n| (n.clone(), rasterize_text(&f, n, px)))
            .collect(),
        Err(_) => Vec::new(),
    }
}

pub fn measure_letter(font_bytes: &[u8], px: f32) -> f32 {
    match fontdue::Font::from_bytes(
        font_bytes,
        fontdue::FontSettings {
            collection_index: 0,
            ..Default::default()
        },
    ) {
        Ok(f) => rasterize_text(&f, "Example", px).width as f32 / 7.0,
        Err(_) => px * 0.5,
    }
}

pub struct StateLabelsMesh {
    pub atlas_rgba: Vec<u8>,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub vertices: Vec<[f32; 4]>,
    pub vertex_count: u32,
}

fn point_at(pts: &[[f32; 2]], target: f32) -> ([f32; 2], f32) {
    let mut acc = 0.0f32;
    for w in pts.windows(2) {
        let d = dist(w[0], w[1]);
        if acc + d >= target && d > 1e-6 {
            let t = (target - acc) / d;
            return (
                [
                    w[0][0] + (w[1][0] - w[0][0]) * t,
                    w[0][1] + (w[1][1] - w[0][1]) * t,
                ],
                (w[1][1] - w[0][1]).atan2(w[1][0] - w[0][0]),
            );
        }
        acc += d;
    }
    (*pts.last().unwrap_or(&[0.0; 2]), 0.0)
}

/// Builds per-char quads along each path (startOffset 50%, 2-line
/// perpendicular offsets via tspan dy, real per-glyph UVs).
pub fn build_label_mesh(
    strips: &[(String, LabelStrip)],
    paths: &[Vec<[f32; 2]>],
    states: &[State],
    font_world: f32,
    letter_length: f32,
) -> StateLabelsMesh {
    let atlas_w = strips
        .iter()
        .map(|(_, s)| s.width)
        .max()
        .unwrap_or(1)
        .max(1);
    let sh = strips.first().map(|(_, s)| s.height).unwrap_or(1).max(1);
    let atlas_h = (sh * strips.len() as u32).max(1);
    let mut atlas = vec![0u8; (atlas_w * atlas_h * 4) as usize];
    for (i, (_, s)) in strips.iter().enumerate() {
        let yo = (i as u32 * sh) as usize;
        for r in 0..s.height as usize {
            let src = r * s.width as usize * 4;
            let dst = (yo + r) * atlas_w as usize * 4;
            let cp = (s.width as usize).min(atlas_w as usize) * 4;
            if dst + cp <= atlas.len() && src + cp <= s.rgba.len() {
                atlas[dst..dst + cp].copy_from_slice(&s.rgba[src..src + cp]);
            }
        }
    }
    let mut vertices: Vec<[f32; 4]> = Vec::new();
    for (idx, (text, strip)) in strips.iter().enumerate() {
        // paths[i] aligns with states[i] (vec order), NOT with st.id — the
        // importer skips the neutral placeholder, so ids are offset by one.
        let Some((st_idx, _st)) = states.iter().enumerate().find(|(_, st)| &st.name == text) else {
            continue;
        };
        let Some(path) = paths.get(st_idx) else {
            continue;
        };
        if path.len() < 2 {
            continue;
        }
        let total: f32 = path.windows(2).map(|w| dist(w[0], w[1])).sum();
        // Auto mode (FMG getLinesAndRatio): full 1-line if it fits doubled.
        let letters = total / letter_length.max(1e-6);
        let full_len = text.chars().count() as f32;
        let (lines, ratio): (Vec<String>, f32) = if letters > full_len * 2.0 {
            (
                vec![text.clone()],
                minmax(letters / full_len * 70.0, 70.0, 170.0),
            )
        } else {
            let lines = split_in_two(text);
            let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1) as f32;
            (lines, minmax(letters / longest * 60.0, 70.0, 150.0))
        };
        let fs = font_world * ratio / 100.0;
        let scale = fs / sh as f32;
        for (li, line_text) in lines.iter().enumerate() {
            // tspan dy: first line (n-1)/-2 em, subsequent +1 em each.
            let dy_em = if lines.len() > 1 {
                if li == 0 {
                    (lines.len() - 1) as f32 / -2.0
                } else {
                    li as f32 - 0.5
                }
            } else {
                0.0
            };
            let n_off = dy_em * fs;
            let text_px: f32 = strip
                .chars
                .iter()
                .take(line_text.chars().count())
                .map(|g| g.adv as f32 * scale)
                .sum();
            if text_px > total {
                continue;
            }
            let mut arc = (total - text_px) / 2.0; // startOffset 50%
            for gc in strip.chars.iter().take(line_text.chars().count()) {
                let gw = gc.bm_w as f32 * scale;
                let gh = gc.h as f32 * scale;
                let (mid, ang) = point_at(path, arc + gw / 2.0);
                let (ux, uy) = (ang.cos(), ang.sin());
                let (upx, upy) = (uy, -ux); // screen up
                let cx = mid[0] + upx * n_off;
                let cy = mid[1] + upy * n_off;
                // Per-glyph UV rect from its pen position in the strip,
                // shifted by the strip's Y offset inside the atlas.
                let strip_y = idx as f32 * sh as f32;
                let y_top = (baseline_row(strip) - gc.ymin - gc.h as i32).max(0) as f32 + strip_y;
                let u0 = gc.pen as f32 / atlas_w as f32;
                let u1 = (gc.pen + gc.bm_w) as f32 / atlas_w as f32;
                let v0 = y_top / atlas_h as f32;
                let v1 = (y_top + gc.h as f32) / atlas_h as f32;
                let top_off = (baseline_row(strip) - gc.ymin) as f32 * scale;
                let bot_off = top_off - gh;
                let hw = gw / 2.0;
                for (oa, op, u, v) in [
                    (-hw, top_off, u0, v0),
                    (hw, top_off, u1, v0),
                    (hw, bot_off, u1, v1),
                    (-hw, top_off, u0, v0),
                    (hw, bot_off, u1, v1),
                    (-hw, bot_off, u0, v1),
                ] {
                    vertices.push([cx + oa * ux + op * upx, cy + oa * uy + op * upy, u, v]);
                }
                arc += gc.adv as f32 * scale;
            }
        }
    }

    let vertex_count = vertices.len() as u32;
    StateLabelsMesh {
        atlas_rgba: atlas,
        atlas_width: atlas_w,
        atlas_height: atlas_h,
        vertices,
        vertex_count,
    }
}

fn baseline_row(s: &LabelStrip) -> i32 {
    // Rasterize_text bakes baseline at `ascent` with 4px bottom padding;
    // ascent = height - 4 - |descent| ≈ height - 4 - height/5.
    s.height as i32 - 4 - (s.height as i32 - 4) / 5
}

/// Overlay that renders a textured-quad mesh (state labels).
pub struct StateLabelsOverlay {
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    vertex_count: u32,
}

impl StateLabelsOverlay {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        mesh: &StateLabelsMesh,
        msaa_count: u32,
        camera_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        use wgpu::util::DeviceExt;
        let extent = wgpu::Extent3d {
            width: mesh.atlas_width.max(1),
            height: mesh.atlas_height.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vor-state-labels-tex"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &mesh.atlas_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(extent.width * 4),
                rows_per_image: Some(extent.height),
            },
            extent,
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vor-state-labels-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vor-state-labels-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vor-state-labels-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let mut flat: Vec<f32> = Vec::with_capacity(mesh.vertices.len() * 4);
        for v in &mesh.vertices {
            flat.extend_from_slice(v);
        }
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vor-state-labels-vbo"),
            contents: bytemuck::cast_slice(&flat),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let vertex_count = mesh.vertex_count;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vor-state-labels-shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
@group(0) @binding(0) var<uniform> camera : mat4x4<f32>;
@group(1) @binding(0) var tex : texture_2d<f32>;
@group(1) @binding(1) var tex_sampler : sampler;
struct VIn { @location(0) position : vec2<f32>, @location(1) uv : vec2<f32> }
struct VOut { @builtin(position) clip_position : vec4<f32>, @location(0) frag_uv : vec2<f32> }
@vertex fn vs_main(in : VIn) -> VOut {
    var out : VOut;
    out.clip_position = camera * vec4<f32>(in.position, 0.0, 1.0);
    out.frag_uv = in.uv;
    return out;
}
@fragment fn fs_main(in : VOut) -> @location(0) vec4<f32> {
    return textureSample(tex, tex_sampler, in.frag_uv);
}
"#
                .into(),
            ),
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: 16,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        };
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vor-state-labels-pl"),
            bind_group_layouts: &[camera_layout, &bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vor-state-labels-pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(crate::renderer::stencil_passthrough()),
            multisample: wgpu::MultisampleState {
                count: msaa_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            bind_group,
            pipeline,
            vertex_buf,
            vertex_count,
        }
    }

    /// Draws the rotated character quads.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, camera: &'a wgpu::BindGroup) {
        if self.vertex_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(1, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}

impl std::fmt::Debug for StateLabelsOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateLabelsOverlay")
            .field("vertex_count", &self.vertex_count)
            .finish()
    }
}
