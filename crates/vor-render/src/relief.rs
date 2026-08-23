//! Relief icons layer (FMG `#terrain`, `src/renderers/draw-relief-icons.ts`).
//!
//! Icons are scattered inside each land cell with a **Poisson-disc** sampler
//! (`graphUtils.ts:poissonDiscSampler`, k=3), density/size driven:
//!
//! - `h < 20` or river cell → no icons.
//! - `h < 50`: biome icons. Radius `2 / (iconsDensity/100) / density`; the cell
//!   is rejected outright when `rand > iconsDensity*10` (sparse biomes).
//!   Icon picked uniformly from the biome's table (`biomes.ts` icons), grass
//!   drawn 1.2× taller; icon half-size `h = (4 + rand) * size`.
//! - `h ≥ 50`: relief icons — mount (`(h−45)·mod`) or hill
//!   (`clamp((h−40)·mod, 3, 6)`), `mod = 0.2 · size`, radius `2 / density`.
//!
//! Instances are sorted by `y + size` (painter's order) and rendered from a
//! texture atlas of the FMG "simple" set symbols (`relief-*-1`), rasterized
//! from `index.html` into `assets/textures/relief/atlas.png` (3×3, 256 px
//! cells). In the simple set `mountSnow`→`mount`, `coniferSnow`→`conifer`,
//! `cactus`/`deadTree`→`dune`, so the temperature swap is a no-op and the
//! grid temperature is not needed.
//!
//! Determinism: FMG consumes the shared `Math.random` Alea stream; Voronia
//! uses a dedicated `Alea("{seed}_relief")` stream (documented divergence).

use crate::prng::Alea;
use vor_core::pack::Pack;

/// Atlas cell order (3×3, 256 px each) — must match `atlas.png` layout.
pub const SYMBOLS: &[&str] = &[
    "mount",
    "hill",
    "conifer",
    "deciduous",
    "acacia",
    "palm",
    "swamp",
    "dune",
    "grass",
];

/// FMG `biomes.ts` `iconsDensity` per biome id (0 = no biome icons).
const ICONS_DENSITY: [u16; 13] = [0, 3, 2, 120, 120, 120, 120, 150, 150, 100, 5, 0, 250];

/// FMG `biomes.ts` `icons` tables mapped to atlas indices (uniform pick like
/// `b[Math.floor(Math.random()*b.length)]` — the object values are ignored in
/// JS). `cactus`/`deadTree` collapse to `dune` (simple set).
const BIOME_ICONS: [&[u8]; 13] = [
    &[],           // 0: none
    &[7, 7, 7],    // 1: dune, cactus→dune, deadTree→dune
    &[7, 7],       // 2: dune, deadTree→dune
    &[4, 8],       // 3: acacia, grass
    &[8],          // 4: grass
    &[4, 5],       // 5: acacia, palm
    &[3],          // 6: deciduous
    &[4, 5, 3, 6], // 7: acacia, palm, deciduous, swamp
    &[3, 6],       // 8: deciduous, swamp
    &[2],          // 9: conifer (coniferSnow→conifer)
    &[8],          // 10: grass
    &[],           // 11: none
    &[6],          // 12: swamp
];

/// Relief icon placement options (FMG `#terrain` attrs).
#[derive(Debug, Clone)]
pub struct ReliefSettings {
    /// FMG `density` attr (default 0.4).
    pub density: f32,
    /// FMG `size` attr (default 1; UI range 0.5–3).
    pub size: f32,
}

impl Default for ReliefSettings {
    fn default() -> Self {
        Self {
            density: 0.4,
            size: 1.0,
        }
    }
}

/// One placed icon: atlas `symbol`, top-left corner `(x, y)` and edge size
/// `s` (world units) — exactly FMG's `<use x y width height>`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReliefIcon {
    pub symbol: u8,
    pub x: f32,
    pub y: f32,
    pub s: f32,
}

/// `rn(x, 2)` — Math.round to 2 decimals.
fn rn2(x: f32) -> f32 {
    (x * 100.0).round() / 100.0
}

/// Poisson-disc sampler — literal port of `graphUtils.ts:poissonDiscSampler`
/// (mbostock's algorithm, `k` attempts per active point, default 3), with the
/// shared RNG injected instead of `Math.random`.
pub fn poisson_disc(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    r: f32,
    k: usize,
    rng: &mut Alea,
) -> Vec<[f32; 2]> {
    let mut out = Vec::new();
    if x1 < x0 || y1 < y0 || r <= 0.0 {
        return out;
    }
    let width = x1 - x0;
    let height = y1 - y0;
    let r2 = r * r;
    let r2_3 = 3.0 * r2;
    let cell_size = r * std::f32::consts::FRAC_1_SQRT_2;
    let grid_width = (width / cell_size).ceil() as usize;
    let grid_height = (height / cell_size).ceil() as usize;
    if grid_width == 0 || grid_height == 0 {
        return out;
    }
    let mut grid: Vec<Option<[f32; 2]>> = vec![None; grid_width * grid_height];
    let mut queue: Vec<[f32; 2]> = Vec::new();

    let far = |grid: &Vec<Option<[f32; 2]>>, x: f32, y: f32| -> bool {
        let i = (x / cell_size) as usize;
        let j = (y / cell_size) as usize;
        let i0 = i.saturating_sub(2);
        let j0 = j.saturating_sub(2);
        let i1 = (i + 3).min(grid_width);
        let j1 = (j + 3).min(grid_height);
        for jj in j0..j1 {
            let o = jj * grid_width;
            for ii in i0..i1 {
                if let Some(s) = grid[o + ii] {
                    let dx = s[0] - x;
                    let dy = s[1] - y;
                    if dx * dx + dy * dy < r2 {
                        return false;
                    }
                }
            }
        }
        true
    };

    // yield sample(width/2, height/2)
    let first = [width / 2.0, height / 2.0];
    grid[grid_width * ((first[1] / cell_size) as usize) + (first[0] / cell_size) as usize] =
        Some(first);
    queue.push(first);
    out.push([first[0] + x0, first[1] + y0]);

    'pick: while !queue.is_empty() {
        let i = (rng.next_f64() * queue.len() as f64) as usize;
        let parent = queue[i];
        for _ in 0..k {
            let a = 2.0 * std::f64::consts::PI * rng.next_f64();
            let rr = (rng.next_f64() * r2_3 as f64 + r2 as f64).sqrt() as f32;
            let x = parent[0] + rr * a.cos() as f32;
            let y = parent[1] + rr * a.sin() as f32;
            if (0.0..width).contains(&x) && (0.0..height).contains(&y) && far(&grid, x, y) {
                let p = [x, y];
                grid[grid_width * ((y / cell_size) as usize) + (x / cell_size) as usize] = Some(p);
                queue.push(p);
                out.push([x + x0, y + y0]);
                continue 'pick;
            }
        }
        queue.remove(i); // queue.splice(i, 1)
    }
    out
}

/// d3 `polygonContains` — even-odd ray casting on a closed ring.
pub fn polygon_contains(ring: &[[f32; 2]], pt: [f32; 2]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut contains = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (ring[i][0], ring[i][1]);
        let (xj, yj) = (ring[j][0], ring[j][1]);
        if ((yi > pt[1]) != (yj > pt[1])) && (pt[0] < (xj - xi) * (pt[1] - yi) / (yj - yi) + xi) {
            contains = !contains;
        }
        j = i;
    }
    contains
}

/// Places all relief icons for the map (FMG main loop). `seed` is the world
/// seed; the RNG stream is `Alea("{seed}_relief")`.
pub fn build_relief_instances(
    pack: &Pack,
    seed: u64,
    settings: &ReliefSettings,
) -> Vec<ReliefIcon> {
    let mut rng = Alea::new(&format!("{seed}_relief"));
    let mut relief: Vec<ReliefIcon> = Vec::new();
    let density = if settings.density > 0.0 {
        settings.density
    } else {
        0.4
    };
    let size = if settings.size != 0.0 {
        settings.size
    } else {
        1.0
    };
    let m0d = 0.2 * size; // size modifier (FMG `mod`)

    let n = pack.points_n();
    for i in 0..n {
        let height = pack.cells.height.get(i).copied().unwrap_or(0);
        if height < 20 {
            continue; // no icons on water
        }
        if pack.cells.river.get(i).copied().unwrap_or(0) != 0 {
            continue; // no icons on rivers
        }
        let biome = pack.cells.biome.get(i).copied().unwrap_or(0) as usize;
        if (height as usize) < 50 && ICONS_DENSITY.get(biome).copied().unwrap_or(0) == 0 {
            continue; // no icons for this biome
        }

        // Cell polygon (Voronoi ring) + extent.
        let ring = match pack.vertices.cell_rings.get(i) {
            Some(r) if r.len() >= 3 => r,
            _ => continue,
        };
        let polygon: Vec<[f32; 2]> = ring
            .iter()
            .filter_map(|&v| pack.vertices.positions.get(v as usize).copied())
            .collect();
        if polygon.len() < 3 {
            continue;
        }
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for p in &polygon {
            min_x = min_x.min(p[0]);
            max_x = max_x.max(p[0]);
            min_y = min_y.min(p[1]);
            max_y = max_y.max(p[1]);
        }

        if height < 50 {
            // placeBiomeIcons
            let d = ICONS_DENSITY[biome] as f32 / 100.0;
            let radius = 2.0 / d / density;
            if rng.next_f64() > (d * 10.0) as f64 {
                continue;
            }
            let table = BIOME_ICONS[biome];
            for [cx, cy] in poisson_disc(min_x, min_y, max_x, max_y, radius, 3, &mut rng) {
                if !polygon_contains(&polygon, [cx, cy]) {
                    continue;
                }
                let mut h = (4.0 + rng.next_f64() as f32) * size;
                // getBiomeIcon: uniform pick; coniferSnow swap is a no-op in
                // the simple set.
                let symbol = table[(rng.next_f64() * table.len() as f64) as usize];
                if symbol == 8 {
                    h *= 1.2; // grass rides taller
                }
                relief.push(ReliefIcon {
                    symbol,
                    x: rn2(cx - h),
                    y: rn2(cy - h),
                    s: rn2(h * 2.0),
                });
            }
        } else {
            // placeReliefIcons: mount/hill by height (snow variant collapses
            // to mount in the simple set).
            let radius = 2.0 / density;
            let symbol: u8 = if height > 70 { 0 } else { 1 }; // mount | hill
            let icon_size = if height > 70 {
                (height as f32 - 45.0) * m0d
            } else {
                ((height as f32 - 40.0) * m0d).clamp(3.0, 6.0)
            };
            for [cx, cy] in poisson_disc(min_x, min_y, max_x, max_y, radius, 3, &mut rng) {
                if !polygon_contains(&polygon, [cx, cy]) {
                    continue;
                }
                relief.push(ReliefIcon {
                    symbol,
                    x: rn2(cx - icon_size),
                    y: rn2(cy - icon_size),
                    s: rn2(icon_size * 2.0),
                });
            }
        }
    }

    // Sort by y + size (painter's order).
    relief.sort_by(|a, b| {
        (a.y + a.s)
            .partial_cmp(&(b.y + b.s))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    relief
}

/// Textured-quad overlay that renders the icon instances from the symbol
/// atlas (one draw call, alpha blending, no stencil — FMG `#terrain` has no
/// mask).
pub struct ReliefIconsOverlay {
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    vertex_count: u32,
}

impl ReliefIconsOverlay {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        atlas_width: u32,
        atlas_height: u32,
        atlas_rgba: &[u8],
        msaa_count: u32,
        camera_layout: &wgpu::BindGroupLayout,
        icons: &[ReliefIcon],
    ) -> Self {
        // Upload atlas (Rgba8UnormSrgb, straight alpha).
        let extent = wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vor-relief-atlas"),
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
            atlas_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(atlas_width * 4),
                rows_per_image: Some(atlas_height),
            },
            extent,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vor-relief-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vor-relief-bgl"),
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
            label: Some("vor-relief-bg"),
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

        // Quad per icon with atlas sub-rect UVs.
        let cols = 3.0f32;
        let rows = 3.0f32;
        let cw = 1.0 / cols;
        let ch = 1.0 / rows;
        let mut verts: Vec<f32> = Vec::with_capacity(icons.len() * 6 * 4);
        for icon in icons {
            let col = (icon.symbol as f32 % cols) * cw;
            let row = (icon.symbol as f32 / cols).floor() * ch;
            let (u0, u1) = (col, col + cw);
            let (v0, v1) = (row, row + ch);
            let (x0, y0) = (icon.x, icon.y);
            let (x1, y1) = (icon.x + icon.s, icon.y + icon.s);
            for (px, py, u, v) in [
                (x0, y0, u0, v0),
                (x1, y0, u1, v0),
                (x1, y1, u1, v1),
                (x0, y0, u0, v0),
                (x1, y1, u1, v1),
                (x0, y1, u0, v1),
            ] {
                verts.extend_from_slice(&[px, py, u, v]);
            }
        }
        let vertex_count = verts.len() as u32 / 4;
        let vertex_buf = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("vor-relief-vbo"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vor-relief-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vor-relief-pipeline-layout"),
            bind_group_layouts: &[camera_layout, &bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vor-relief-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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

    /// Draws all icon quads. Caller binds the shared camera group at index 0.
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

impl std::fmt::Debug for ReliefIconsOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReliefIconsOverlay")
            .field("vertex_count", &self.vertex_count)
            .finish()
    }
}

const SHADER_SRC: &str = r#"
@group(0) @binding(0) var<uniform> camera : mat4x4<f32>;
@group(1) @binding(0) var tex : texture_2d<f32>;
@group(1) @binding(1) var tex_sampler : sampler;

struct VertexIn {
    @location(0) position : vec2<f32>,
    @location(1) uv : vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip_position : vec4<f32>,
    @location(0) frag_uv : vec2<f32>,
};

@vertex
fn vs_main(in : VertexIn) -> VertexOut {
    var out : VertexOut;
    out.clip_position = camera * vec4<f32>(in.position, 0.0, 1.0);
    out.frag_uv = in.uv;
    return out;
}

@fragment
fn fs_main(in : VertexOut) -> @location(0) vec4<f32> {
    return textureSample(tex, tex_sampler, in.frag_uv);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisson_is_deterministic_and_covers() {
        let a = poisson_disc(0.0, 0.0, 100.0, 100.0, 10.0, 3, &mut Alea::new("42_relief"));
        let b = poisson_disc(0.0, 0.0, 100.0, 100.0, 10.0, 3, &mut Alea::new("42_relief"));
        assert_eq!(a, b, "same seed must produce identical sampling");
        assert!(!a.is_empty(), "100×100 with r=10 should sample points");
        let c = poisson_disc(0.0, 0.0, 100.0, 100.0, 10.0, 3, &mut Alea::new("43_relief"));
        assert_ne!(a, c, "different seed should differ");
        // All points inside bounds.
        assert!(a
            .iter()
            .all(|p| (0.0..=100.0).contains(&p[0]) && (0.0..=100.0).contains(&p[1])));
    }

    #[test]
    fn poisson_respects_radius() {
        let pts = poisson_disc(0.0, 0.0, 50.0, 50.0, 8.0, 3, &mut Alea::new("r8"));
        for (i, a) in pts.iter().enumerate() {
            for b in pts.iter().skip(i + 1) {
                let d2 = (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2);
                assert!(d2 >= 64.0 * 0.98, "points closer than r: {a:?} {b:?}");
            }
        }
    }

    #[test]
    fn polygon_contains_basic_square() {
        let sq = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        assert!(polygon_contains(&sq, [5.0, 5.0]));
        assert!(!polygon_contains(&sq, [15.0, 5.0]));
        assert!(!polygon_contains(&sq, [-1.0, 5.0]));
    }

    #[test]
    fn biome_tables_map_to_simple_set_symbols() {
        // Cactus/deadTree collapse to dune (7) in the simple set.
        assert!(BIOME_ICONS[1].iter().all(|&s| s == 7));
        // Savanna mixes acacia (4) and grass (8).
        assert_eq!(BIOME_ICONS[3], &[4, 8]);
        // Every referenced symbol is a valid atlas index.
        for table in BIOME_ICONS.iter() {
            assert!(table.iter().all(|&s| (s as usize) < SYMBOLS.len()));
        }
        // Dense biomes have icons; barren ones don't.
        assert_eq!(ICONS_DENSITY[0], 0);
        assert_eq!(ICONS_DENSITY[11], 0);
        assert!(ICONS_DENSITY[12] == 250);
    }

    #[test]
    fn hill_size_clamps_like_fmg() {
        // mod = 0.2 * size(1); hill: clamp((h-40)*mod, 3, 6)
        let m0d = 0.2;
        let hill = |h: u8| ((h as f32 - 40.0) * m0d).clamp(3.0, 6.0);
        assert_eq!(hill(41), 3.0); // clamped up
        assert_eq!(hill(65), 5.0);
        assert_eq!(hill(70), 6.0); // clamped down
                                   // mount: (h-45)*mod unclamped
        let mount = |h: u8| (h as f32 - 45.0) * m0d;
        assert!((mount(71) - 5.2).abs() < 1e-6);
    }
}
