use egui_wgpu::Renderer as EguiRenderer;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tracing::info;
use vor_core::World;
use vor_render::heightmap::{build_mesh, HeightmapMesh};
use vor_render::{Camera, Renderer};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("winit: {0}")]
    Winit(String),
    #[error(transparent)]
    Wgpu(#[from] wgpu::SurfaceError),
    #[error("render: {0}")]
    Render(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub struct ViewerConfig {
    pub map_path: PathBuf,
    pub world: World,
    pub mesh: HeightmapMesh,
}

pub fn run(cfg: ViewerConfig) -> Result<(), AppError> {
    let event_loop = EventLoop::new().map_err(|e| AppError::Winit(e.to_string()))?;
    let mut app = App {
        cfg: Some(cfg),
        state: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| AppError::Winit(e.to_string()))?;
    Ok(())
}

struct App {
    cfg: Option<ViewerConfig>,
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let cfg = self.cfg.take().expect("cfg consumed twice");

        info!(
            "available wgpu adapters: {:?}",
            pollster::block_on(list_adapters())
        );

        let window = Arc::new(
            _event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(format!("Voronia -- {}", cfg.map_path.display()))
                        .with_inner_size(PhysicalSize::new(1280, 800)),
                )
                .expect("create_window"),
        );

        let state = pollster::block_on(init_state(window, cfg));
        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(ref mut state) = self.state else {
            return;
        };
        if let Err(e) = state.handle_window_event(event_loop, &event) {
            tracing::error!("handle_window_event: {e}");
        }
    }
}

struct State {
    window: Arc<Window>,
    renderer: Renderer,
    egui_ctx: egui::Context,
    egui_winit: egui_winit::State,
    egui_renderer: EguiRenderer,
    camera: Camera,
    mesh_bounds_min: [f32; 2],
    mesh_bounds_max: [f32; 2],
    map_path: PathBuf,
    cursor_screen: [f32; 2],
    pan_active: bool,
    pan_last: Option<[f32; 2]>,
    last_frame: Instant,
}

async fn init_state(window: Arc<Window>, cfg: ViewerConfig) -> State {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

    let surface = instance
        .create_surface(window.clone())
        .expect("create_surface");

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("no wgpu adapter compatible with surface");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("vor-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await
        .expect("request_device");

    let caps = surface.get_capabilities(&adapter);
    let surface_format = caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(caps.formats[0]);

    let mut renderer = Renderer::new(
        surface,
        device,
        queue,
        (size.width, size.height),
        surface_format,
    );

    info!(
        "heightmap mesh: {} vertices, {} indices (bounds {:?} -> {:?})",
        cfg.mesh.vertices.len(),
        cfg.mesh.indices.len(),
        cfg.mesh.bounds_min,
        cfg.mesh.bounds_max
    );
    renderer.set_mesh(&cfg.mesh);

    let mesh_bounds_min = cfg.mesh.bounds_min;
    let mesh_bounds_max = cfg.mesh.bounds_max;

    let mut camera = Camera::new([0.0, 0.0], 1000.0, size.width, size.height);
    camera.frame_bounds(mesh_bounds_min, mesh_bounds_max);

    let egui_ctx = egui::Context::default();
    let pixels_per_point = window.scale_factor() as f32;
    let egui_winit = egui_winit::State::new(
        egui_ctx.clone(),
        egui::ViewportId::ROOT,
        window.as_ref(),
        Some(pixels_per_point),
        None,
        Some(renderer.device.limits().max_texture_dimension_2d as usize),
    );

    let egui_renderer = EguiRenderer::new(&renderer.device, surface_format, None, 1, false);

    State {
        window,
        renderer,
        egui_ctx,
        egui_winit,
        egui_renderer,
        camera,
        mesh_bounds_min,
        mesh_bounds_max,
        map_path: cfg.map_path,
        cursor_screen: [0.0, 0.0],
        pan_active: false,
        pan_last: None,
        last_frame: Instant::now(),
    }
}

impl State {
    fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &WindowEvent,
    ) -> Result<(), AppError> {
        let _ = self.egui_winit.on_window_event(self.window.as_ref(), event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.renderer.resize(size.width, size.height);
                    self.camera.set_viewport(size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_screen = [position.x as f32, position.y as f32];
                if self.pan_active {
                    if let Some(last) = self.pan_last {
                        let size = self.window.inner_size();
                        let delta = [
                            self.cursor_screen[0] - last[0],
                            self.cursor_screen[1] - last[1],
                        ];
                        self.camera
                            .pan_by_screen_delta(delta, [size.width as f32, size.height as f32]);
                    }
                    self.pan_last = Some(self.cursor_screen);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    self.pan_active = *state == ElementState::Pressed;
                    if self.pan_active {
                        self.pan_last = Some(self.cursor_screen);
                    } else {
                        self.pan_last = None;
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let pixels = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 50.0,
                    MouseScrollDelta::PixelDelta(d) => -(d.y as f32),
                };
                let factor = (1.001_f32).powf(pixels);
                let size = self.window.inner_size();
                self.camera.zoom_at_cursor(
                    self.cursor_screen,
                    [size.width as f32, size.height as f32],
                    factor,
                );
            }
            WindowEvent::RedrawRequested => {
                self.redraw()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn redraw(&mut self) -> Result<(), AppError> {
        self.window.request_redraw();

        // ---- Camera uniform ----
        let uniform = self.camera.uniform();
        self.renderer.queue.write_buffer(
            &self.renderer.camera_buf,
            0,
            bytemuck::cast_slice(&[uniform]),
        );

        // ---- Egui frame ----
        let raw_input = self.egui_winit.take_egui_input(self.window.as_ref());
        let map_path = self.map_path.clone();
        let camera_center = self.camera.center;
        let camera_extent_y = self.camera.extent_y;
        let cursor_screen = self.cursor_screen;
        let surface_size = {
            let sz = self.window.inner_size();
            [sz.width as f32, sz.height as f32]
        };
        let world_cursor = self.camera.screen_to_world(cursor_screen, surface_size);
        let mesh_min = self.mesh_bounds_min;
        let mesh_max = self.mesh_bounds_max;

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };

        let output = self.egui_ctx.run(raw_input, |ctx| {
            egui::TopBottomPanel::top("vor-app-top").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Voronia -- {}", map_path.display()));
                });
            });
            egui::Window::new("visor / Fase 2")
                .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 36.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("FPS: {fps:.1}"));
                    ui.label(format!(
                        "center: ({:.0}, {:.0})",
                        camera_center[0], camera_center[1]
                    ));
                    ui.label(format!("extent_y: {camera_extent_y:.0}"));
                    ui.label(format!(
                        "cursor world: ({:.0}, {:.0})",
                        world_cursor[0], world_cursor[1]
                    ));
                    ui.label(format!("mesh bbox: {mesh_min:?} -> {mesh_max:?}"));
                });
        });

        let clipped = self
            .egui_ctx
            .tessellate(output.shapes, self.window.scale_factor() as f32);
        self.egui_winit
            .handle_platform_output(self.window.as_ref(), output.platform_output);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [
                self.window.inner_size().width,
                self.window.inner_size().height,
            ],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        // ---- Wgpu passes ----
        let surface_texture = self.renderer.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("vor-frame"),
                });

        // Pass 1: heightmap (clear background)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vor-heightmap"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let (Some(vbo), Some(ibo)) = (&self.renderer.vertex_buf, &self.renderer.index_buf) {
                pass.set_pipeline(&self.renderer.heightmap_pipeline);
                pass.set_bind_group(0, &self.renderer.camera_bind, &[]);
                pass.set_vertex_buffer(0, vbo.slice(..));
                pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.renderer.index_count, 0, 0..1);
            }
        }

        // Update egui buffers (pobla vertex/index buffers internos de egui-wgpu).
        self.egui_renderer.update_buffers(
            &self.renderer.device,
            &self.renderer.queue,
            &mut encoder,
            &clipped,
            &screen_descriptor,
        );

        // Pass 2: egui overlay (load from prev pass)
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("vor-egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            self.egui_renderer
                .render(&mut pass, &clipped, &screen_descriptor);
        }

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }
}

async fn list_adapters() -> Vec<String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    instance
        .enumerate_adapters(wgpu::Backends::all())
        .iter()
        .map(|a| format!("{:?} {}", a.get_info().backend, a.get_info().name))
        .collect()
}

pub fn run_cli() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vor_app=info,vor_render=info,vor_import=info".into()),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        anyhow::bail!("uso: vor-cli viewer -- <path-al-.map> (los '--' son opcionales)");
    }
    let path_idx = args
        .iter()
        .position(|a| !a.starts_with('-') && a != &args[0])
        .unwrap_or(1);
    let path = PathBuf::from(&args[path_idx]);
    info!("cargando mapa: {}", path.display());

    let bytes = std::fs::read(&path)?;
    let raw =
        vor_import::mapfile::raw::parse(&bytes).map_err(|e| anyhow::anyhow!("parse .map: {e}"))?;
    let loaded = vor_import::mapfile::Loader::load(&raw)
        .map_err(|e| anyhow::anyhow!("Loader::load: {e}"))?;
    let mesh = build_mesh(&loaded.world.grid);
    let cfg = ViewerConfig {
        map_path: path,
        world: loaded.world,
        mesh,
    };
    run(cfg).map_err(|e| anyhow::anyhow!("visor: {e}"))
}
