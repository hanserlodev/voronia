//! Compass rose (FMG `#compass`, child 10 of `#viewbox` — moves WITH the
//! map, drawn between coordinates and rivers).
//!
//! FMG instantiates `#defs-compass-rose` with a default transform of
//! `translate(80,80) scale(0.25)` and the group carries
//! `mask="url(#water)"` — the rose is only visible over water. We reproduce:
//!
//! - the long coordinate lines (`±20000` at 8 angles, stroke `#3f3f3f` w1.1)
//!   as stroked quads;
//! - the rose core (rings + star, extent ≈ ±212) as a pre-rasterized texture
//!   quad (`assets/textures/compass.png`, viewBox −220..220) — curves render
//!   pixel-perfect without porting the arc math.
//!
//! Group opacity 0.8; water-only visibility via inverted stencil test.

use crate::biome::hex_color_to_linear;
use crate::heightmap::HeightmapMesh;
use crate::heightmap::HeightmapVertex;
use wgpu::util::DeviceExt;

/// FMG saved/default placement: `translate(80,80) scale(0.25)`.
pub const COMPASS_X: f32 = 80.0;
pub const COMPASS_Y: f32 = 80.0;
/// Rose core half-extent in world units: 212 × scale 0.25.
pub const COMPASS_HALF: f32 = 212.0 * 0.25;
/// `#compass` group opacity from the serialized SVG.
pub const COMPASS_OPACITY: f32 = 0.8;

const COORD_ANGLES_DEG: [f32; 8] = [0.0, 45.0, 22.5, -22.5, 11.25, -11.25, 56.25, -56.25];
const COORD_LEN: f32 = 20000.0;

/// Textured quad overlay for the rose core + a mesh for the coord lines.
pub struct CompassOverlay {
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    /// Coord-line mesh (same style group).
    pub lines_mesh: HeightmapMesh,
}

impl CompassOverlay {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        tex_width: u32,
        tex_height: u32,
        rgba: &[u8],
        msaa_count: u32,
        camera_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let extent = wgpu::Extent3d {
            width: tex_width,
            height: tex_height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vor-compass-tex"),
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
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(tex_width * 4),
                rows_per_image: Some(tex_height),
            },
            extent,
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vor-compass-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vor-compass-bgl"),
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
            label: Some("vor-compass-bg"),
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

        // Quad centered at (COMPASS_X, COMPASS_Y), half-extent COMPASS_HALF.
        let (cx, cy, h) = (COMPASS_X, COMPASS_Y, COMPASS_HALF);
        let mut verts: Vec<f32> = Vec::with_capacity(24);
        for (x, y, u, v) in [
            (cx - h, cy - h, 0.0, 1.0),
            (cx + h, cy - h, 1.0, 1.0),
            (cx + h, cy + h, 1.0, 0.0),
            (cx - h, cy - h, 0.0, 1.0),
            (cx + h, cy + h, 1.0, 0.0),
            (cx - h, cy + h, 0.0, 0.0),
        ] {
            verts.extend_from_slice(&[x, y, u, v]);
        }
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vor-compass-vbo"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vor-compass-shader"),
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

// Group opacity 0.8.
@fragment
fn fs_main(in : VertexOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, tex_sampler, in.frag_uv);
    return vec4<f32>(c.rgb, c.a * 0.8);
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
            label: Some("vor-compass-pipeline-layout"),
            bind_group_layouts: &[camera_layout, &bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vor-compass-pipeline"),
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
            depth_stencil: Some(crate::renderer::stencil_water_test()),
            multisample: wgpu::MultisampleState {
                count: msaa_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Coord lines: 8 rays through the origin (FMG lines span ±20000).
        let line_color = {
            let mut c = hex_color_to_linear("#3f3f3f");
            c[3] = COMPASS_OPACITY;
            c
        };
        let mut lines_mesh = HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [0.0; 2],
            bounds_max: [0.0; 2],
        };
        for deg in COORD_ANGLES_DEG {
            let a = deg.to_radians();
            let (dx, dy) = (a.cos(), a.sin());
            let base = lines_mesh.vertices.len() as u32;
            lines_mesh.vertices.extend_from_slice(&[
                HeightmapVertex {
                    pos: [-dx * COORD_LEN, -dy * COORD_LEN],
                    color: line_color,
                },
                HeightmapVertex {
                    pos: [dx * COORD_LEN, dy * COORD_LEN],
                    color: line_color,
                },
            ]);
            lines_mesh.indices.extend_from_slice(&[base, base + 1]);
        }

        Self {
            bind_group,
            pipeline,
            vertex_buf,
            lines_mesh,
        }
    }

    /// Draws the rose-core quad. Caller binds camera at group 0. The stencil
    /// must be set to the WATER reference by the caller.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, camera: &'a wgpu::BindGroup) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(1, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.draw(0..6, 0..1);
    }
}

impl std::fmt::Debug for CompassOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompassOverlay").finish()
    }
}
