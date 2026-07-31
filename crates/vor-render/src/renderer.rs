//! Voronia wgpu renderer (Phase 2): a single layer (heightmap).
//!
//! Responsibilities:
//! - Inline WGSL shader (vertex + fragment) that receives `view_proj` + vertices
//!   with position and per-cell color.
//! - Persistent vertex/index buffers (uploaded once in `set_mesh`).
//! - Camera uniform buffer, refreshed on every `render` with the current camera.
//! - Simple render pass to the surface texture.
//!
//! More layers in Phase 3 (biomes, rivers, etc.): each one will have its own
//! pipeline and index/vertex buffers, but the camera uniform is shared.

use crate::camera::{Camera, CameraUniform};
use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use thiserror::Error;
use wgpu::util::DeviceExt;

/// GPU resources for a render layer (vertex/index buffers).
#[derive(Debug)]
pub struct LayerBuffer {
    pub vertex_buf: Option<wgpu::Buffer>,
    pub index_buf: Option<wgpu::Buffer>,
    pub index_count: u32,
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("wgpu: no surface texture ({0})")]
    SurfaceAcquire(String),
}

/// wgpu renderer with a shared pipeline for map layers.
///
/// Layer 0 is the heightmap (kept in `vertex_buf`/`index_buf`/`index_count`).
/// Additional layers (Phase 3+) live in `layers`.
///
/// Uses 4x MSAA for edge smoothing (removes the jagged edges of Voronoi triangles).
pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,

    pub camera_buf: wgpu::Buffer,
    pub camera_bind_layout: wgpu::BindGroupLayout,
    pub camera_bind: wgpu::BindGroup,
    pub heightmap_pipeline: wgpu::RenderPipeline,
    pub line_pipeline: wgpu::RenderPipeline,

    // MSAA 4x
    pub msaa_count: u32,
    pub msaa_texture: Option<wgpu::Texture>,
    pub msaa_view: Option<wgpu::TextureView>,

    // Layer 0: heightmap (backward compat)
    pub vertex_buf: Option<wgpu::Buffer>,
    pub index_buf: Option<wgpu::Buffer>,
    pub index_count: u32,

    // Additional layers (Phase 3) -- TriangleList pipeline
    pub layers: Vec<LayerBuffer>,

    // Line layers -- LineList pipeline with alpha blending
    pub line_layers: Vec<LayerBuffer>,
}

impl Renderer {
    /// Initializes wgpu on an already-created `surface`.
    pub fn new(
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) -> Self {
        let msaa_count = 4;
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vor-msaa-buffer"),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: msaa_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_view = Some(msaa_texture.create_view(&wgpu::TextureViewDescriptor::default()));

        // Camera uniform buffer (4x4 matrix = 64 bytes).
        let camera_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vor-camera-uniform"),
            size: std::mem::size_of::<CameraUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_layout_entries = &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];

        let camera_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("vor-camera-bgl"),
                entries: camera_bind_layout_entries,
            });

        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vor-camera-bg"),
            layout: &camera_bind_layout,
            entries: std::slice::from_ref(&wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buf.as_entire_binding(),
            }),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vor-pipeline-layout"),
            bind_group_layouts: &[&camera_bind_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vor-heightmap-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        // Vertex buffer layout (SoA-like: pos + color interleaved).
        let vertex_attrs = [
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x4,
            },
        ];
        let vertex_layout = || wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<HeightmapVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attrs,
        };

        let multisample = wgpu::MultisampleState {
            count: msaa_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let color_targets = [Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let heightmap_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vor-heightmap-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &color_targets,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample,
            multiview: None,
            cache: None,
        });

        let line_color_targets = [Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vor-line-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &line_color_targets,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample,
            multiview: None,
            cache: None,
        });

        Self {
            device,
            queue,
            surface,
            surface_config,
            format,
            camera_buf,
            camera_bind_layout,
            camera_bind,
            heightmap_pipeline,
            line_pipeline,
            msaa_count,
            msaa_texture: Some(msaa_texture),
            msaa_view,
            vertex_buf: None,
            index_buf: None,
            index_count: 0,
            layers: Vec::new(),
            line_layers: Vec::new(),
        }
    }

    /// Reconfigures the surface when the window size changes.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vor-msaa-buffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.msaa_count,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.msaa_view = Some(tex.create_view(&wgpu::TextureViewDescriptor::default()));
        self.msaa_texture = Some(tex);
    }

    /// Uploads the tessellated heightmap mesh to the GPU.
    pub fn set_mesh(&mut self, mesh: &HeightmapMesh) {
        let vertex_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vor-heightmap-vbo"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vor-heightmap-ibo"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        self.index_count = mesh.indices.len() as u32;
        self.vertex_buf = Some(vertex_buf);
        self.index_buf = Some(index_buf);
    }

    /// Adds an additional mesh layer. Returns the layer index (for use with
    /// `draw_layer`). Layer 0 is the heightmap (vertex_buf/index_buf); extra
    /// layers start at 1.
    pub fn add_layer_mesh(&mut self, mesh: &HeightmapMesh) -> usize {
        let idx = self.layers.len();
        let vertex_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("vor-layer-{idx}-vbo")),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("vor-layer-{idx}-ibo")),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        self.layers.push(LayerBuffer {
            vertex_buf: Some(vertex_buf),
            index_buf: Some(index_buf),
            index_count: mesh.indices.len() as u32,
        });
        idx + 1 // 0-indexed, +1 because layer 0 is the heightmap
    }

    /// Draws a layer in the render pass. `layer_index=0` -> heightmap;
    /// `layer_index>=1` -> additional layers registered with `add_layer_mesh`.
    pub fn draw_layer<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, layer_index: usize) {
        let (vbo, ibo, count) = if layer_index == 0 {
            (&self.vertex_buf, &self.index_buf, self.index_count)
        } else {
            let idx = layer_index - 1;
            match self.layers.get(idx) {
                Some(l) => (&l.vertex_buf, &l.index_buf, l.index_count),
                None => return,
            }
        };
        if let (Some(vbo), Some(ibo)) = (vbo, ibo) {
            if count == 0 {
                return;
            }
            pass.set_pipeline(&self.heightmap_pipeline);
            pass.set_bind_group(0, &self.camera_bind, &[]);
            pass.set_vertex_buffer(0, vbo.slice(..));
            pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..count, 0, 0..1);
        }
    }

    /// Adds a line layer. Returns the index within `line_layers`.
    pub fn add_line_layer(&mut self, mesh: &HeightmapMesh) -> usize {
        let idx = self.line_layers.len();
        let vertex_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("vor-line-{idx}-vbo")),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("vor-line-{idx}-ibo")),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        self.line_layers.push(LayerBuffer {
            vertex_buf: Some(vertex_buf),
            index_buf: Some(index_buf),
            index_count: mesh.indices.len() as u32,
        });
        idx
    }

    /// Draws a line layer in the render pass using `line_pipeline`.
    pub fn draw_line_layer<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, layer_idx: usize) {
        let Some(layer) = self.line_layers.get(layer_idx) else {
            return;
        };
        let (Some(vbo), Some(ibo)) = (&layer.vertex_buf, &layer.index_buf) else {
            return;
        };
        if layer.index_count == 0 {
            return;
        }
        pass.set_pipeline(&self.line_pipeline);
        pass.set_bind_group(0, &self.camera_bind, &[]);
        pass.set_vertex_buffer(0, vbo.slice(..));
        pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..layer.index_count, 0, 0..1);
    }

    pub fn layer_vertex_count(&self, layer_index: usize) -> usize {
        if layer_index == 0 {
            self.index_count as usize
        } else {
            let idx = layer_index - 1;
            self.layers
                .get(idx)
                .map(|l| l.index_count as usize)
                .unwrap_or(0)
        }
    }

    /// Renders a frame with the given camera. Clears with `clear_color` and
    /// draws the heightmap (if there is a mesh).
    pub fn render(&mut self, camera: &Camera, clear_color: [f64; 4]) -> Result<(), RenderError> {
        let uniform = camera.uniform();
        self.queue
            .write_buffer(&self.camera_buf, 0, bytemuck::cast_slice(&[uniform]));

        let surface_texture = self
            .surface
            .get_current_texture()
            .map_err(|e| RenderError::SurfaceAcquire(e.to_string()))?;
        let resolve_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let msaa_view = self
            .msaa_view
            .as_ref()
            .expect("msaa_view present during normal use");

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("vor-frame-encoder"),
            });

        {
            let [r, g, b, a] = clear_color;
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: msaa_view,
                resolve_target: Some(&resolve_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                    store: wgpu::StoreOp::Store,
                },
            })];

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vor-heightmap-pass"),
                color_attachments: &color_attachments[..],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let (Some(vbo), Some(ibo)) = (&self.vertex_buf, &self.index_buf) {
                pass.set_pipeline(&self.heightmap_pipeline);
                pass.set_bind_group(0, &self.camera_bind, &[]);
                pass.set_vertex_buffer(0, vbo.slice(..));
                pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.index_count, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        Ok(())
    }

    /// Returns a `TextureView` of the surface (resolve target) so that vor-app
    /// can use it in its overlay/egui passes after drawing MSAA layers.
    pub fn resolve_view<'a>(
        &'a self,
        surface_texture: &'a wgpu::SurfaceTexture,
    ) -> wgpu::TextureView {
        surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Returns a reference to the MSAA TextureView.
    pub fn msaa_view(&self) -> Option<&wgpu::TextureView> {
        self.msaa_view.as_ref()
    }

    /// Camera bind group layout (so vor-app can map the same layout if it wants
    /// to share the uniform with one of its own layers).
    pub fn camera_bind_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bind_layout
    }
}

const SHADER_SRC: &str = r#"
@group(0) @binding(0) var<uniform> camera : mat4x4<f32>;

struct VertexIn {
    @location(0) position : vec2<f32>,
    @location(1) color : vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip_position : vec4<f32>,
    @location(0) color : vec4<f32>,
};

@vertex
fn vs_main(in : VertexIn) -> VertexOut {
    var out : VertexOut;
    let world = vec4<f32>(in.position, 0.0, 1.0);
    out.clip_position = camera * world;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in : VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
