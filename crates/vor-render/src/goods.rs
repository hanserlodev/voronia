//! Goods layer (FMG `draw-goods.ts`): three sub-layers.
//!
//! 1. **goodsCells** — per-cell polygons filled with the good color, opacity
//!    normalized against the global max cell production.
//! 2. **goodsIcons** — a marker circle in each cell with a bonus resource.
//! 3. **goodsBurgs** — plates next to burgs with their top-3 produced goods
//!    (value labels). Requires per-burg production; wired once the economy
//!    module provides it (Fase 7).

use vor_core::entities::good::Good;
use vor_core::pack::Pack;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{ColorCtor, HeightmapMesh, HeightmapVertex};
use crate::mesh::build_pack_mesh;

struct GapColorCtor([f32; 4]);

impl lyon::tessellation::StrokeVertexConstructor<HeightmapVertex> for GapColorCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::StrokeVertex<'_, '_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

fn append_mesh(
    target: &mut HeightmapMesh,
    mesh: lyon::tessellation::VertexBuffers<HeightmapVertex, u32>,
) {
    let base = target.vertices.len() as u32;
    let start = target.vertices.len();
    target.vertices.extend(mesh.vertices);
    target
        .indices
        .extend(mesh.indices.iter().map(|&i| i + base));
    for v in &target.vertices[start..] {
        target.bounds_min[0] = target.bounds_min[0].min(v.pos[0]);
        target.bounds_min[1] = target.bounds_min[1].min(v.pos[1]);
        target.bounds_max[0] = target.bounds_max[0].max(v.pos[0]);
        target.bounds_max[1] = target.bounds_max[1].max(v.pos[1]);
    }
}

/// Bonus-resource rural production factor (FMG `BONUS_RURAL_PRODUCTION`).
const BONUS_RURAL_PRODUCTION: f32 = 0.25;
/// Bonus-resource cell production cap (FMG `MAX_BONUS_PRODUCTION`).
const MAX_BONUS_PRODUCTION: f32 = 5.0;

/// Per-cell production of a good channel (FMG `Production.getCellProduction`).
///
/// Current approximation: `biome_output[cell.biome] × population` (rural) plus
/// the bonus resource channel when `cells.good[cell] == good.id`. The full
/// multiplier stack (`getModifiers`) and manufactured recipes are Fase 7.
pub fn cell_production(pack: &Pack, good: &Good, cell: usize) -> f32 {
    let biome = pack.cells.biome.get(cell).copied().unwrap_or(0);
    let pop = pack.cells.population.get(cell).copied().unwrap_or(0.0);
    let biome_out = good
        .biome_output
        .get(&biome.to_string())
        .copied()
        .unwrap_or(0.0);
    let rural = biome_out * pop;

    let is_bonus = pack.cells.good.get(cell).copied().unwrap_or(0) == good.id;
    let bonus = if is_bonus {
        (pop * BONUS_RURAL_PRODUCTION).min(MAX_BONUS_PRODUCTION)
    } else {
        0.0
    };
    rural + bonus
}

/// Builds the **goodsCells** sub-layer: one polygon per cell per produced good,
/// opacity `0.1 + 0.9 * normalize(total, 0, maxTotal)` (FMG
/// `draw-goods.ts:buildGoodsCellsContent`).
pub fn build_goods_cells_mesh(pack: &Pack, goods: &[Good]) -> HeightmapMesh {
    let n = pack.points_n();
    // First pass: total production per cell + global max.
    let mut cell_total: Vec<f32> = vec![0.0; n];
    let mut max_total = 0.0f32;

    for (p, total_slot) in cell_total.iter_mut().enumerate() {
        let mut total = 0.0f32;
        for good in goods.iter().filter(|g| g.id != 0 && g.visible) {
            let prod = cell_production(pack, good, p);
            if prod <= 0.0 {
                continue;
            }
            total += prod;
        }
        if total > 0.0 {
            *total_slot = total;
            if total > max_total {
                max_total = total;
            }
        }
    }
    if max_total <= 0.0 {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [0.0, 0.0],
            bounds_max: [0.0, 0.0],
        };
    }

    // FMG emits one polygon for every positive good channel in a cell, not
    // only the dominant good. Merge one cell mesh per visible good; draw order
    // is immaterial because the layer is alpha blended.
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };
    for good in goods.iter().filter(|g| g.id != 0 && g.visible) {
        let color = if good.color.is_empty() {
            [0.5, 0.5, 0.5, 1.0]
        } else {
            hex_color_to_linear(&good.color)
        };
        let good_mesh = build_pack_mesh(&pack.vertices, n, |p| {
            let prod = cell_production(pack, good, p);
            if prod <= 0.0 {
                return [0.0, 0.0, 0.0, 0.0];
            }
            let opacity = 0.1 + 0.9 * (cell_total[p] / max_total);
            [color[0], color[1], color[2], opacity.clamp(0.0, 1.0)]
        });
        let base = mesh.vertices.len() as u32;
        mesh.vertices.extend(good_mesh.vertices);
        mesh.indices
            .extend(good_mesh.indices.into_iter().map(|i| i + base));
        mesh.bounds_min[0] = mesh.bounds_min[0].min(good_mesh.bounds_min[0]);
        mesh.bounds_min[1] = mesh.bounds_min[1].min(good_mesh.bounds_min[1]);
        mesh.bounds_max[0] = mesh.bounds_max[0].max(good_mesh.bounds_max[0]);
        mesh.bounds_max[1] = mesh.bounds_max[1].max(good_mesh.bounds_max[1]);
    }
    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0, 0.0];
        mesh.bounds_max = [0.0, 0.0];
    }
    mesh
}

/// Builds the **goodsIcons** sub-layer: a small marker circle in each cell that
/// has a bonus resource (FMG `draw-goods.ts:buildGoodsIconsContent`).
pub fn build_goods_icons_mesh(pack: &Pack, goods: &[Good]) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY, f32::INFINITY],
        bounds_max: [f32::NEG_INFINITY, f32::NEG_INFINITY],
    };

    let n = pack.points_n();
    const SEGMENTS: u32 = 8;
    const RADIUS: f32 = 1.8;

    for p in 0..n {
        let gid = pack.cells.good.get(p).copied().unwrap_or(0);
        if gid == 0 {
            continue;
        }
        let good = match goods.iter().find(|g| g.id == gid) {
            Some(g) if g.visible && !g.color.is_empty() => g,
            _ => continue,
        };
        let color = hex_color_to_linear(&good.color);
        let center = pack.points.get(p).copied().unwrap_or([0.0, 0.0]);
        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(HeightmapVertex { pos: center, color });
        for i in 0..SEGMENTS {
            let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            mesh.vertices.push(HeightmapVertex {
                pos: [center[0] + RADIUS * a.cos(), center[1] + RADIUS * a.sin()],
                color,
            });
        }
        for i in 0..SEGMENTS {
            let v0 = base + 1 + i;
            let v1 = base + 1 + (i + 1) % SEGMENTS;
            mesh.indices.extend_from_slice(&[base, v0, v1]);
        }
        for v in &mesh.vertices[base as usize..] {
            mesh.bounds_min[0] = mesh.bounds_min[0].min(v.pos[0]);
            mesh.bounds_min[1] = mesh.bounds_min[1].min(v.pos[1]);
            mesh.bounds_max[0] = mesh.bounds_max[0].max(v.pos[0]);
            mesh.bounds_max[1] = mesh.bounds_max[1].max(v.pos[1]);
        }
    }

    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0, 0.0];
        mesh.bounds_max = [0.0, 0.0];
    }
    mesh
}

/// FMG `good-*` symbol ids, in `assets/textures/goods/atlas.png` cell order
/// (8 columns × 9 rows of 64 px, rasterized from `index.html #good-icons`).
pub const GOOD_SYMBOL_IDS: &[&str] = &[
    "good-amber",
    "good-arms",
    "good-artillery",
    "good-barrels",
    "good-beer",
    "good-books",
    "good-boots",
    "good-bronze",
    "good-camels",
    "good-candles",
    "good-cattle",
    "good-ceramics",
    "good-cheese",
    "good-clay",
    "good-cloth",
    "good-coal",
    "good-coins",
    "good-copper",
    "good-dates",
    "good-dyes",
    "good-elephants",
    "good-fish",
    "good-furs",
    "good-game",
    "good-garments",
    "good-gemstones",
    "good-glass",
    "good-gold",
    "good-grain",
    "good-gunpowder",
    "good-harnesses",
    "good-hemp",
    "good-honey",
    "good-horses",
    "good-incense",
    "good-ink",
    "good-iron",
    "good-jewelry",
    "good-leather",
    "good-liquor",
    "good-marble",
    "good-oil",
    "good-olives",
    "good-paper",
    "good-pearls",
    "good-perfume",
    "good-ropes",
    "good-sails",
    "good-salt",
    "good-salted-fish",
    "good-saltpeter",
    "good-sand",
    "good-sheep",
    "good-ships",
    "good-silk",
    "good-silver",
    "good-slaves",
    "good-soap",
    "good-spices",
    "good-stone",
    "good-sugar",
    "good-tar",
    "good-tea",
    "good-tin",
    "good-tobacco",
    "good-tools",
    "good-unknown",
    "good-vinegar",
    "good-whales",
    "good-wine",
    "good-wood",
];

/// Atlas layout constants.
pub const ATLAS_COLS: f32 = 8.0;
pub const ATLAS_ROWS: f32 = 9.0;

/// FMG `#goodsIcons` defaults (`default.json`): circle radius = data-size/2
/// (6/2 = 3), stroke-width 0.3, stroke = `darker(2)` of the fill.
pub const ICON_SIZE: f32 = 6.0;

/// One textured quad for the symbol atlas (top-left + size, world units).
#[derive(Debug, Clone, Copy)]
pub struct GoodsIconQuad {
    /// Atlas cell index (row-major over `GOOD_SYMBOL_IDS`).
    pub cell: u8,
    pub x: f32,
    pub y: f32,
    pub s: f32,
}

/// Textured-quad overlay rendering symbol quads from the goods atlas.
pub struct GoodsIconsOverlay {
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    vertex_count: u32,
}

impl GoodsIconsOverlay {
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
        quads: &[GoodsIconQuad],
    ) -> Self {
        use wgpu::util::DeviceExt;
        let extent = wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vor-goods-atlas"),
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
            label: Some("vor-goods-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vor-goods-bgl"),
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
            label: Some("vor-goods-bg"),
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

        let cw = 1.0 / ATLAS_COLS;
        let ch = 1.0 / ATLAS_ROWS;
        let mut verts: Vec<f32> = Vec::with_capacity(quads.len() * 6 * 4);
        for q in quads {
            let col = (q.cell as f32 % ATLAS_COLS) * cw;
            let row = (q.cell as f32 / ATLAS_COLS).floor() * ch;
            let (u0, u1, v0, v1) = (col, col + cw, row, row + ch);
            let (x1, y1) = (q.x + q.s, q.y + q.s);
            for (px, py, u, v) in [
                (q.x, q.y, u0, v0),
                (x1, q.y, u1, v0),
                (x1, y1, u1, v1),
                (q.x, q.y, u0, v0),
                (x1, y1, u1, v1),
                (q.x, y1, u0, v1),
            ] {
                verts.extend_from_slice(&[px, py, u, v]);
            }
        }
        let vertex_count = verts.len() as u32 / 4;
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vor-goods-vbo"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vor-goods-shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vor-goods-pipeline-layout"),
            bind_group_layouts: &[camera_layout, &bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vor-goods-pipeline"),
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

impl std::fmt::Debug for GoodsIconsOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoodsIconsOverlay")
            .field("vertex_count", &self.vertex_count)
            .finish()
    }
}

/// Looks up a good's atlas cell by its `icon` id; `None` when unknown.
pub fn good_atlas_cell(icon: &str) -> Option<u8> {
    GOOD_SYMBOL_IDS
        .iter()
        .position(|&id| id == icon)
        .map(|i| i as u8)
}

/// `Goods.getStroke(hex)` — d3 `color.darker(2)` (sRGB × 0.7²).
fn good_stroke(color: [f32; 4]) -> [f32; 4] {
    crate::heightmap::darken(color, 2.0)
}

/// FMG `#goodsIcons`: a circle (r = size/2, fill = good color, stroke =
/// `darker(2)`, width 0.3) under each bonus-cell icon. The symbol itself is
/// drawn by [`GoodsIconsOverlay`] on top.
pub fn build_goods_icon_circles_mesh(
    pack: &Pack,
    goods: &[vor_core::entities::good::Good],
) -> HeightmapMesh {
    use lyon::tessellation::{FillOptions, FillTessellator, StrokeOptions, StrokeTessellator};
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    let mut fill_tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();
    let fill_opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::NonZero);

    let by_id: std::collections::HashMap<u16, &vor_core::entities::good::Good> =
        goods.iter().map(|g| (g.id, g)).collect();

    let n = pack.points_n();
    for cell in 0..n {
        let good_id = pack.cells.good.get(cell).copied().unwrap_or(0);
        if good_id == 0 {
            continue;
        }
        let Some(good) = by_id.get(&good_id) else {
            continue;
        };
        let Some(center) = pack.points.get(cell) else {
            continue;
        };
        let fill_lin = hex_color_to_linear(&good.color);
        let stroke_lin = good_stroke(fill_lin);
        let (cx, cy) = (center[0], center[1]);
        let r = ICON_SIZE / 2.0;

        // Circle fill.
        let mut builder = lyon::path::Path::builder();
        builder.add_circle(lyon::geom::point(cx, cy), r, lyon::path::Winding::Positive);
        let path = builder.build();
        let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
            lyon::tessellation::VertexBuffers::new();
        if fill_tess
            .tessellate_path(
                &path,
                &fill_opts,
                &mut lyon::tessellation::BuffersBuilder::new(&mut out, ColorCtor(fill_lin)),
            )
            .is_ok()
        {
            append_mesh(&mut mesh, out);
        }
        // Circle stroke (darker(2), width 0.3).
        let mut builder = lyon::path::Path::builder();
        builder.add_circle(lyon::geom::point(cx, cy), r, lyon::path::Winding::Positive);
        let path = builder.build();
        let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
            lyon::tessellation::VertexBuffers::new();
        let sopts = StrokeOptions::default().with_line_width(0.3);
        if stroke_tess
            .tessellate_path(
                &path,
                &sopts,
                &mut lyon::tessellation::BuffersBuilder::new(&mut out, GapColorCtor(stroke_lin)),
            )
            .is_ok()
        {
            append_mesh(&mut mesh, out);
        }
    }

    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0; 2];
        mesh.bounds_max = [0.0; 2];
    }
    mesh
}

/// Symbol quads for the bonus-cell icons (size 6, centered on the cell).
pub fn goods_icon_quads(
    pack: &Pack,
    goods: &[vor_core::entities::good::Good],
) -> Vec<GoodsIconQuad> {
    let by_id: std::collections::HashMap<u16, &vor_core::entities::good::Good> =
        goods.iter().map(|g| (g.id, g)).collect();
    let mut quads = Vec::new();
    let n = pack.points_n();
    for cell in 0..n {
        let good_id = pack.cells.good.get(cell).copied().unwrap_or(0);
        if good_id == 0 {
            continue;
        }
        let Some(good) = by_id.get(&good_id) else {
            continue;
        };
        let Some(cell_idx) = good_atlas_cell(&good.icon) else {
            continue;
        };
        let Some(center) = pack.points.get(cell) else {
            continue;
        };
        quads.push(GoodsIconQuad {
            cell: cell_idx,
            x: center[0] - ICON_SIZE / 2.0,
            y: center[1] - ICON_SIZE / 2.0,
            s: ICON_SIZE,
        });
    }
    quads
}

/// FMG `goodsBurgs` plate constants (`draw-goods.ts:8-16`), at data-size 3
/// (scale = size/3 = 1).
pub const PLATE_ICON: f32 = 3.0;
pub const PLATE_FONT: f32 = 3.5;
pub const PLATE_GAP: f32 = 0.2;
pub const PLATE_ENTRY_GAP: f32 = 0.8;
pub const PLATE_PAD_X: f32 = 1.0;
pub const PLATE_PAD_Y: f32 = 0.6;
pub const PLATE_RX: f32 = 1.0;
/// `#goodsBurgs` group stroke (`default.json`).
pub const PLATE_STROKE: [f32; 4] = {
    // #41414f at alpha 1 — const-friendly literal via hex at runtime instead.
    [0.0, 0.0, 0.0, 1.0]
};
pub const BURG_PLATE_SIZE: f32 = 3.0;

/// A plate value text (world px, font size in world units = 3.5).
#[derive(Debug, Clone, PartialEq)]
pub struct BurgPlateLabel {
    pub text: String,
    pub x: f32,
    pub y: f32,
}

/// Builds the `#goodsBurgs` plates: per burg with production, the top-3 goods
/// by accumulated units (`draw-goods.ts:109-168`). Returns the plate mesh
/// (rects + circles, blended) and the symbol quads + value labels.
pub fn build_goods_burg_plates(
    burgs: &[vor_core::entities::burg::Burg],
    goods: &[vor_core::entities::good::Good],
) -> (HeightmapMesh, Vec<GoodsIconQuad>, Vec<BurgPlateLabel>) {
    use lyon::tessellation::{FillOptions, FillTessellator, StrokeOptions, StrokeTessellator};
    let by_id: std::collections::HashMap<u16, &vor_core::entities::good::Good> =
        goods.iter().map(|g| (g.id, g)).collect();
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [f32::INFINITY; 2],
        bounds_max: [f32::NEG_INFINITY; 2],
    };
    let mut quads = Vec::new();
    let mut labels = Vec::new();
    let mut fill_tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();
    let fill_opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::NonZero);

    let plate_fill = hex_color_to_linear("#f5f5f5");
    let plate_stroke = hex_color_to_linear("#41414f");
    let text_note = hex_color_to_linear("#28282f");
    let _ = text_note;

    for burg in burgs {
        if burg.production.is_empty() {
            continue;
        }
        // Accumulate units per good id.
        let mut produced: std::collections::HashMap<u16, f32> = std::collections::HashMap::new();
        for p in &burg.production {
            *produced.entry(p.good_id).or_insert(0.0) += p.units;
        }
        // Top-3 by value with FMG's cutoff (skip if <= current 3rd value).
        let mut entries: Vec<(u16, f32)> = Vec::new(); // (good_id, value)
        for (&good_id, &raw) in &produced {
            let value = (raw * 10.0).round() / 10.0;
            if value <= 0.0 {
                continue;
            }
            if entries.len() == 3 && value <= entries[2].1 {
                continue;
            }
            let mut i = entries.len();
            while i > 0 && entries[i - 1].1 < value {
                i -= 1;
            }
            entries.insert(i, (good_id, value));
            if entries.len() > 3 {
                entries.pop();
            }
        }
        if entries.is_empty() {
            continue;
        }

        // Geometry (scale = data-size/3 = 1).
        let char_width = 1.2 * BURG_PLATE_SIZE;
        let plate_icon = PLATE_ICON * (BURG_PLATE_SIZE / 3.0);
        let plate_font = PLATE_FONT * (BURG_PLATE_SIZE / 3.0);
        let plate_gap = PLATE_GAP * (BURG_PLATE_SIZE / 3.0);
        let entry_gap = PLATE_ENTRY_GAP * (BURG_PLATE_SIZE / 3.0);
        let pad_x = PLATE_PAD_X * (BURG_PLATE_SIZE / 3.0);
        let pad_y = PLATE_PAD_Y * (BURG_PLATE_SIZE / 3.0);

        let mut widths = Vec::with_capacity(entries.len());
        for &(_, value) in &entries {
            let value_str = format_value(value);
            let width = plate_icon
                + plate_gap
                + value_str.len() as f32 * char_width
                + 0.4 * plate_font * 0.62;
            widths.push(width);
        }
        let content_width: f32 =
            widths.iter().sum::<f32>() + entry_gap * (entries.len() - 1) as f32;
        let plate_width = content_width + pad_x * 2.0;
        let plate_height = plate_icon + pad_y * 2.0;
        let plate_x = burg.position[0] - plate_width / 2.0;
        let plate_y = burg.position[1];
        let icon_y = plate_y + pad_y;
        let mid = icon_y + plate_icon / 2.0;

        // Rect (rx≈1 approximated as sharp rect; documented divergence).
        let mut builder = lyon::path::Path::builder();
        builder.begin(lyon::geom::point(plate_x, plate_y));
        builder.line_to(lyon::geom::point(plate_x + plate_width, plate_y));
        builder.line_to(lyon::geom::point(
            plate_x + plate_width,
            plate_y + plate_height,
        ));
        builder.line_to(lyon::geom::point(plate_x, plate_y + plate_height));
        builder.end(true);
        let rect_path = builder.build();
        let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
            lyon::tessellation::VertexBuffers::new();
        if fill_tess
            .tessellate_path(
                &rect_path,
                &fill_opts,
                &mut lyon::tessellation::BuffersBuilder::new(&mut out, ColorCtor(plate_fill)),
            )
            .is_ok()
        {
            append_mesh(&mut mesh, out);
        }
        let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
            lyon::tessellation::VertexBuffers::new();
        if stroke_tess
            .tessellate_path(
                &rect_path,
                &StrokeOptions::default().with_line_width(0.2),
                &mut lyon::tessellation::BuffersBuilder::new(&mut out, GapColorCtor(plate_stroke)),
            )
            .is_ok()
        {
            append_mesh(&mut mesh, out);
        }

        let mut offset = plate_x + pad_x;
        for ((good_id, value), width) in entries.iter().zip(&widths) {
            let Some(good) = by_id.get(good_id) else {
                continue;
            };
            let fill_lin = hex_color_to_linear(&good.color);
            let stroke_lin = good_stroke(fill_lin);
            // Circle r = plate_icon/2.
            let mut builder = lyon::path::Path::builder();
            builder.add_circle(
                lyon::geom::point(offset + plate_icon / 2.0, mid),
                plate_icon / 2.0,
                lyon::path::Winding::Positive,
            );
            let circle = builder.build();
            let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
                lyon::tessellation::VertexBuffers::new();
            if fill_tess
                .tessellate_path(
                    &circle,
                    &fill_opts,
                    &mut lyon::tessellation::BuffersBuilder::new(&mut out, ColorCtor(fill_lin)),
                )
                .is_ok()
            {
                append_mesh(&mut mesh, out);
            }
            let mut out: lyon::tessellation::VertexBuffers<HeightmapVertex, u32> =
                lyon::tessellation::VertexBuffers::new();
            if stroke_tess
                .tessellate_path(
                    &circle,
                    &StrokeOptions::default().with_line_width(0.2),
                    &mut lyon::tessellation::BuffersBuilder::new(
                        &mut out,
                        GapColorCtor(stroke_lin),
                    ),
                )
                .is_ok()
            {
                append_mesh(&mut mesh, out);
            }
            // Symbol quad (size = plate_icon).
            if let Some(cell) = good_atlas_cell(&good.icon) {
                quads.push(GoodsIconQuad {
                    cell,
                    x: offset,
                    y: icon_y,
                    s: plate_icon,
                });
            }
            // Value label.
            labels.push(BurgPlateLabel {
                text: format_value(*value),
                x: offset + plate_icon + plate_gap,
                y: mid,
            });
            offset += width + entry_gap;
        }
    }

    if !mesh.bounds_min.iter().all(|v| v.is_finite()) {
        mesh.bounds_min = [0.0; 2];
        mesh.bounds_max = [0.0; 2];
    }
    (mesh, quads, labels)
}

/// JS `String(rn(v, 1))`: one decimal, trailing `.0` stripped.
fn format_value(v: f32) -> String {
    let rounded = (v * 10.0).round() / 10.0;
    let s = format!("{rounded:.1}");
    s.strip_suffix(".0").map(str::to_owned).unwrap_or(s)
}

#[cfg(test)]
mod plate_tests {
    use super::*;
    use vor_core::entities::burg::{Burg, BurgProduction};

    fn burg(production: Vec<BurgProduction>) -> Burg {
        Burg {
            id: 1,
            name: "Test".into(),
            cell: 0,
            position: [100.0, 100.0],
            culture: 0,
            state: 0,
            feature: 0,
            population: 0.0,
            kind: Default::default(),
            coat_of_arms: Default::default(),
            is_capital: false,
            port_feature: None,
            has_citadel: false,
            has_plaza: false,
            has_shanty: false,
            has_temple: false,
            has_walls: false,
            locked: false,
            removed: false,
            production,
        }
    }

    #[test]
    fn plates_pick_top3_by_value() {
        let goods = vec![
            Good {
                id: 1,
                name: "A".into(),
                color: "#ff0000".into(),
                icon: "good-wood".into(),
                ..Default::default()
            },
            Good {
                id: 2,
                name: "B".into(),
                color: "#00ff00".into(),
                icon: "good-iron".into(),
                ..Default::default()
            },
            Good {
                id: 3,
                name: "C".into(),
                color: "#0000ff".into(),
                icon: "good-stone".into(),
                ..Default::default()
            },
            Good {
                id: 4,
                name: "D".into(),
                color: "#ffff00".into(),
                icon: "good-amber".into(),
                ..Default::default()
            },
        ];
        let b = burg(vec![
            BurgProduction {
                good_id: 1,
                units: 0.25,
            },
            BurgProduction {
                good_id: 2,
                units: 2.0,
            },
            BurgProduction {
                good_id: 3,
                units: 1.0,
            },
            BurgProduction {
                good_id: 4,
                units: 0.5,
            },
            BurgProduction {
                good_id: 2,
                units: 0.5,
            }, // accumulates to 2.5
        ]);
        let (mesh, quads, labels) = build_goods_burg_plates(&[b], &goods);
        assert!(!mesh.vertices.is_empty());
        assert_eq!(quads.len(), 3, "top-3 plate icons");
        assert_eq!(labels.len(), 3);
        // Sorted desc: 2.5 (good 2), 1.0 (good 3), 0.5 (good 4).
        assert_eq!(labels[0].text, "2.5");
        assert_eq!(labels[1].text, "1");
        assert_eq!(labels[2].text, "0.5");
    }

    #[test]
    fn format_value_strips_trailing_zero() {
        assert_eq!(format_value(5.0), "5");
        assert_eq!(format_value(0.25), "0.3"); // rn(0.25,1) = JS Math.round(2.5)/10 = 0.3
        assert_eq!(format_value(1.44), "1.4");
    }
}
