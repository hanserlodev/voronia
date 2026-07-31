use vor_core::feature::{Feature, FeatureType};
use vor_core::voronoi::VoronoiVertices;

use crate::clip_poly::clip_polygon;
use crate::coastline_path::{build_coastline_path, coastline_path_to_lyon, CoastlineSpan};
use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};
use crate::mesh::catmull_rom_closed;
use crate::prng::Alea;
use crate::simplify::simplify;

use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};

#[derive(Debug, Clone)]
pub struct FractalSettings {
    pub enabled: bool,
    pub max_depth: u32,
    pub base_amplitude: f32,
    pub amplitude_decay: f32,
    pub min_edge: f32,
    pub smooth_threshold: f32,
    pub roughness_contrast: f32,
    pub profile_harmonics: u32,
    pub lake_smooth_thresh_mult: f32,
    pub simplify_tolerance: f32,
    pub clip_secure: bool,
    pub seed: u64,
}

impl Default for FractalSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_depth: 4,
            base_amplitude: 1.5,
            amplitude_decay: 0.9,
            min_edge: 1.0,
            smooth_threshold: 0.25,
            roughness_contrast: 1.5,
            profile_harmonics: 4,
            lake_smooth_thresh_mult: 2.0,
            simplify_tolerance: 0.3,
            clip_secure: true,
            seed: 0,
        }
    }
}

/// Build a smooth closed roughness envelope via sum-of-cosine harmonics, exactly
/// like Azgaar's `makeRoughnessProfile()`.  Consumes `rand` to advance the shared
/// PRNG stream (Alea) so that displacement values later in the pipeline depend on
/// this sequence — same stream sharing as the JS source.
fn make_roughness_profile(
    rand: &mut dyn FnMut() -> f32,
    contrast: f32,
    num_harmonics: u32,
) -> Vec<f32> {
    let size = 256usize;
    let mut profile = vec![0.0f32; size];
    let pi2 = std::f32::consts::TAU;
    for k in 1..=num_harmonics {
        let amp = rand();
        let phase = rand() * pi2;
        for (i, p) in profile.iter_mut().enumerate() {
            *p += amp * (pi2 * k as f32 * i as f32 / size as f32 + phase).cos();
        }
    }
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &v in &profile {
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    let range = (max - min).max(1e-9);
    for v in &mut profile {
        *v = ((*v - min) / range).powf(contrast);
    }
    profile
}

fn sample_profile(profile: &[f32], t: f32) -> f32 {
    let size = profile.len();
    let pos = t.rem_euclid(1.0) * size as f32;
    let i = (pos.floor() as usize).min(size - 1);
    let f = pos - pos.floor();
    let j = (i + 1) % size;
    profile[i] * (1.0 - f) + profile[j] * f
}

fn mid_t(t0: f32, t1: f32) -> f32 {
    let diff = t1 - t0;
    if diff.abs() <= 0.5 {
        return t0 + diff * 0.5;
    }
    let t = t0 + (diff - diff.signum()) * 0.5;
    t.rem_euclid(1.0)
}

#[allow(clippy::too_many_arguments)]
fn subdivide_edge(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    t0: f32,
    t1: f32,
    depth: u32,
    amplitude: f32,
    profile: &[f32],
    rand: &mut dyn FnMut() -> f32,
    settings: &FractalSettings,
    result: &mut Vec<[f32; 2]>,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    if depth == 0 || len < settings.min_edge {
        return;
    }
    let tm = mid_t(t0, t1);
    let roughness = sample_profile(profile, tm);
    if roughness < settings.smooth_threshold {
        return;
    }
    let px = -dy / len;
    let py = dx / len;
    // Azgaar: (rand() - 0.5) * Math.sqrt(len) * amplitude * roughness;
    let disp = (rand() - 0.5) * len.sqrt() * amplitude * roughness;
    let mx = (x0 + x1) * 0.5 + px * disp;
    let my = (y0 + y1) * 0.5 + py * disp;
    let next_amp = amplitude * settings.amplitude_decay;
    subdivide_edge(
        x0,
        y0,
        mx,
        my,
        t0,
        tm,
        depth - 1,
        next_amp,
        profile,
        rand,
        settings,
        result,
    );
    result.push([mx, my]);
    subdivide_edge(
        mx,
        my,
        x1,
        y1,
        tm,
        t1,
        depth - 1,
        next_amp,
        profile,
        rand,
        settings,
        result,
    );
}

fn is_on_border(x: f32, y: f32, width: f32, height: f32) -> bool {
    x <= 0.0 || x >= width || y <= 0.0 || y >= height
}

pub fn fractalize_polygon(
    points: &[[f32; 2]],
    feature_index: usize,
    is_lake: bool,
    map_width: f32,
    map_height: f32,
    settings: &FractalSettings,
) -> (Vec<[f32; 2]>, Vec<CoastlineSpan>) {
    let n = points.len();
    if n < 3 || !settings.enabled {
        let spans: Vec<CoastlineSpan> = (0..n)
            .map(|i| CoastlineSpan {
                start_idx: i,
                end_idx: (i + 1) % n,
                is_smooth: true,
            })
            .collect();
        return (points.to_vec(), spans);
    }

    let effective_threshold = if is_lake {
        (settings.smooth_threshold * settings.lake_smooth_thresh_mult).min(1.0)
    } else {
        settings.smooth_threshold
    };
    let local_settings = FractalSettings {
        smooth_threshold: effective_threshold,
        ..*settings
    };

    // Azgaar: const rand = Alea(`${seed}_c${featureIndex}`);
    let seed_str = format!("{}_c{}", settings.seed, feature_index);
    let mut rng = Alea::new(&seed_str);
    // Shared PRNG closure — advances through profile generation first, then
    // through edge subdivision, exactly like Azgaar's single `rand` reference.
    let mut rand = || rng.next_f64() as f32;

    let profile = make_roughness_profile(
        &mut rand,
        settings.roughness_contrast,
        settings.profile_harmonics,
    );

    let total_len: f32 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(n)
        .map(|(a, b)| {
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            (dx * dx + dy * dy).sqrt()
        })
        .sum();
    if total_len < 1e-9 {
        let spans: Vec<CoastlineSpan> = (0..n)
            .map(|i| CoastlineSpan {
                start_idx: i,
                end_idx: (i + 1) % n,
                is_smooth: true,
            })
            .collect();
        return (points.to_vec(), spans);
    }

    let seg_lens: Vec<f32> = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(n)
        .map(|(a, b)| {
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            (dx * dx + dy * dy).sqrt()
        })
        .collect();

    let mut cum = 0.0f32;
    let t_params: Vec<f32> = seg_lens
        .iter()
        .map(|l| {
            let t = cum / total_len;
            cum += l;
            t
        })
        .collect();

    let mut result_pts: Vec<[f32; 2]> = Vec::new();
    let mut original_indices: Vec<usize> = Vec::with_capacity(n);

    for i in 0..n {
        original_indices.push(result_pts.len());
        result_pts.push(points[i]);

        let j = (i + 1) % n;
        if is_on_border(points[i][0], points[i][1], map_width, map_height)
            && is_on_border(points[j][0], points[j][1], map_width, map_height)
        {
            continue;
        }

        subdivide_edge(
            points[i][0],
            points[i][1],
            points[j][0],
            points[j][1],
            t_params[i],
            t_params[j],
            local_settings.max_depth,
            local_settings.base_amplitude,
            &profile,
            &mut rand,
            &local_settings,
            &mut result_pts,
        );
    }

    let total_pts = result_pts.len();
    let mut spans = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let start_idx = original_indices[i];
        let end_idx = original_indices[j];
        let num_points = if end_idx >= start_idx {
            end_idx - start_idx + 1
        } else {
            (total_pts - start_idx) + end_idx + 1
        };
        let is_smooth = num_points == 2;
        spans.push(CoastlineSpan {
            start_idx,
            end_idx,
            is_smooth,
        });
    }

    (result_pts, spans)
}

#[allow(dead_code)]
fn tessellate_fill(points: &[[f32; 2]], color: [f32; 4]) -> HeightmapMesh {
    let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
    let mut tess = FillTessellator::new();

    if points.len() < 3 {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [f32::INFINITY; 2],
            bounds_max: [f32::NEG_INFINITY; 2],
        };
    }

    let mut builder = Path::builder();
    builder.begin(point(points[0][0], points[0][1]));
    for pt in points.iter().skip(1) {
        builder.line_to(point(pt[0], pt[1]));
    }
    builder.end(true);
    let path = builder.build();

    let mut buffer_builder = BuffersBuilder::new(&mut mesh, ColorCtor(color));
    let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);
    if tess
        .tessellate_path(&path, &opts, &mut buffer_builder)
        .is_err()
    {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [f32::INFINITY; 2],
            bounds_max: [f32::NEG_INFINITY; 2],
        };
    }

    let mut bounds_min = [f32::INFINITY; 2];
    let mut bounds_max = [f32::NEG_INFINITY; 2];
    for v in &mesh.vertices {
        bounds_min[0] = bounds_min[0].min(v.pos[0]);
        bounds_min[1] = bounds_min[1].min(v.pos[1]);
        bounds_max[0] = bounds_max[0].max(v.pos[0]);
        bounds_max[1] = bounds_max[1].max(v.pos[1]);
    }

    HeightmapMesh {
        vertices: mesh.vertices,
        indices: mesh.indices,
        bounds_min,
        bounds_max,
    }
}

pub fn build_fractal_landmass_mesh(
    vertices: &VoronoiVertices,
    features: &[Feature],
    map_width: f32,
    map_height: f32,
    color_fn: impl Fn(&Feature) -> [f32; 4],
    settings: &FractalSettings,
) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    let mut tess = FillTessellator::new();

    for feat in features {
        if !feat.is_land || feat.perimeter_vertices.len() < 3 {
            continue;
        }
        let raw: Vec<[f32; 2]> = feat
            .perimeter_vertices
            .iter()
            .filter_map(|&vi| vertices.positions.get(vi as usize).copied())
            .collect();
        if raw.len() < 3 {
            continue;
        }

        let is_lake = feat.kind == FeatureType::Lake;
        let color = color_fn(feat);

        let simplified = if settings.simplify_tolerance > 0.0 {
            simplify(&raw, settings.simplify_tolerance)
        } else {
            raw
        };

        let clipped = clip_polygon(&simplified, map_width, map_height, settings.clip_secure);

        let (fractal_pts, spans) = fractalize_polygon(
            &clipped,
            feat.id as usize,
            is_lake,
            map_width,
            map_height,
            settings,
        );

        let coastline_path = build_coastline_path(&fractal_pts, &spans);
        let lyon_path = coastline_path_to_lyon(&coastline_path);

        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, ColorCtor(color));
        let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);
        if tess
            .tessellate_path(&lyon_path, &opts, &mut buffer_builder)
            .is_err()
        {
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

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0; 2];
        result.bounds_max = [0.0; 2];
    }
    result
}

pub fn build_landmass_mesh_legacy(
    vertices: &VoronoiVertices,
    features: &[Feature],
    color_fn: impl Fn(&Feature) -> [f32; 4],
) -> HeightmapMesh {
    let mut result = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    let mut tess = FillTessellator::new();

    for feat in features {
        if !feat.is_land || feat.perimeter_vertices.len() < 3 {
            continue;
        }
        let raw: Vec<[f32; 2]> = feat
            .perimeter_vertices
            .iter()
            .filter_map(|&vi| vertices.positions.get(vi as usize).copied())
            .collect();
        if raw.len() < 3 {
            continue;
        }
        let smooth = catmull_rom_closed(&raw, 3);
        let color = color_fn(feat);

        let mut builder = Path::builder();
        if let Some(first) = smooth.first() {
            builder.begin(point(first[0], first[1]));
            for pt in smooth.iter().skip(1) {
                builder.line_to(point(pt[0], pt[1]));
            }
            builder.end(true);
        }
        let path = builder.build();

        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, ColorCtor(color));
        let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);
        if tess
            .tessellate_path(&path, &opts, &mut buffer_builder)
            .is_err()
        {
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

    if !result.bounds_min.iter().all(|v| v.is_finite()) {
        result.bounds_min = [0.0; 2];
        result.bounds_max = [0.0; 2];
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractalize_closed_polygon() {
        let pts = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
        let settings = FractalSettings::default();
        let (result, spans) = fractalize_polygon(&pts, 0, false, 500.0, 500.0, &settings);
        assert!(result.len() >= pts.len(), "fractalize should add points");
        assert_eq!(
            spans.len(),
            pts.len(),
            "should have one span per original edge"
        );
    }

    #[test]
    fn smooth_span_when_no_subdivision() {
        let pts = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
        let mut settings = FractalSettings::default();
        settings.enabled = false;
        let (_, spans) = fractalize_polygon(&pts, 0, false, 500.0, 500.0, &settings);
        assert!(
            spans.iter().all(|s| s.is_smooth),
            "all spans should be smooth when fractalization disabled"
        );
    }
}
