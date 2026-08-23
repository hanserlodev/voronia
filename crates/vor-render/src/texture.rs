use wgpu::util::DeviceExt;

/// World-anchored textured quad rendered as the background canvas (paper/texture)
/// at the very start of the map pass, before the ocean and the land layers.
///
/// The quad is expressed in *world* coordinates (the full world rect) and its
/// vertex shader transforms it with the shared camera matrix, so the paper pans
/// and zooms *with* the map — exactly like Azgaar's `#texture` SVG image, which
/// lives inside the world `#viewbox`. This avoids the "overlay filter" look where
/// a screen-fixed texture slides over the map while the camera moves.
///
/// It uses an opaque REPLACE blend so it never darkens the layers on top of it.
pub struct TextureOverlay {
    pub texture: wgpu::Texture,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
}

impl TextureOverlay {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        rgba: &[u8],
        msaa_count: u32,
        world_min: [f32; 2],
        world_max: [f32; 2],
        camera_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture = Self::upload_texture(device, queue, width, height, rgba);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vor-texture-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vor-texture-bgl"),
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
            label: Some("vor-texture-bg"),
            layout: &bind_group_layout,
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

        // World-space quad covering the whole world rect. UVs map 0..1 across it
        // so the texture is looked up over the full world (the paper canvas).
        // Positions are in Azgaar world pixels (+Y down), transformed by camera.
        let (minx, miny, maxx, maxy) = (world_min[0], world_min[1], world_max[0], world_max[1]);
        let verts: [f32; 24] = [
            // position (x2) + uv (x2)
            minx, miny, 0.0, 1.0, //
            maxx, miny, 1.0, 1.0, //
            maxx, maxy, 1.0, 0.0, //
            minx, miny, 0.0, 1.0, //
            maxx, maxy, 1.0, 0.0, //
            minx, maxy, 0.0, 0.0, //
        ];
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vor-texture-vbo"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vor-texture-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        let vertex_attrs = [
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
        ];
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: 16,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attrs,
        };

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vor-texture-pipeline-layout"),
            // group 0: camera (shared uniform, bound by the caller); group 1:
            // this overlay's texture.
            bind_group_layouts: &[camera_layout, &bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vor-texture-pipeline"),
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
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
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
            texture,
            sampler,
            bind_group,
            bind_group_layout,
            pipeline,
            vertex_buf,
        }
    }

    fn upload_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> wgpu::Texture {
        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vor-texture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texel_layout = wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        };
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            texel_layout,
            extent,
        );

        texture
    }

    /// Draws the world-anchored paper quad. The caller must pass the renderer's
    /// shared [camera bind group](wgpu::BindGroup) (group 0); the texture bind
    /// group is bound at group 1.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, camera: &'a wgpu::BindGroup) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(1, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.draw(0..6, 0..1);
    }
}

impl std::fmt::Debug for TextureOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextureOverlay").finish()
    }
}

const SHADER_SRC: &str = r#"
@group(0) @binding(0) var<uniform> camera : mat4x4<f32>;

struct VertexIn {
    @location(0) position : vec2<f32>,
    @location(1) uv : vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip_position : vec4<f32>,
    @location(0) frag_uv : vec2<f32>,
};

@group(1) @binding(0) var tex : texture_2d<f32>;
@group(1) @binding(1) var tex_sampler : sampler;

@vertex
fn vs_main(in : VertexIn) -> VertexOut {
    var out : VertexOut;
    let world = vec4<f32>(in.position, 0.0, 1.0);
    out.clip_position = camera * world;
    out.frag_uv = in.uv;
    return out;
}

@fragment
fn fs_main(in : VertexOut) -> @location(0) vec4<f32> {
    return textureSample(tex, tex_sampler, in.frag_uv);
}
"#;
