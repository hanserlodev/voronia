use egui_wgpu::Renderer as EguiRenderer;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tracing::info;
use vor_core::World;
use vor_render::biome::{biome_colors_from_catalog, build_biome_mesh};
use vor_render::border::{build_border_mesh, BorderKind};
use vor_render::burg::build_burg_mesh;
use vor_render::heightmap::{build_mesh, HeightmapMesh};
use vor_render::layers::LayerFlags;
use vor_render::river::build_river_mesh;
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
    world: World,
    layer_flags: LayerFlags,
    picked_cell: Option<usize>,
    autosave_enabled: bool,
    autosave_interval: f32,
    last_autosave: Instant,
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

    let adapter_info = adapter.get_info();
    info!(
        "wgpu adapter selected: {:?} {}",
        adapter_info.backend, adapter_info.name
    );

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

    let world = cfg.world;

    // --- Construir capas adicionales ---
    let biome_colors = biome_colors_from_catalog(&world.biomes);
    let biome_mesh = build_biome_mesh(&world.pack, &biome_colors);
    let river_mesh = build_river_mesh(&world.pack.points, &world.rivers);
    let border_state_mesh = build_border_mesh(&world.pack, BorderKind::State);
    let border_province_mesh = build_border_mesh(&world.pack, BorderKind::Province);
    let border_culture_mesh = build_border_mesh(&world.pack, BorderKind::Culture);
    let burg_mesh = build_burg_mesh(&world.pack);

    info!(
        "meshes: biomes={}v/{}i, rivers={}v/{}i, borders(s/p/c)=({}/{}/{}), burgs={}v/{}i",
        biome_mesh.vertices.len(),
        biome_mesh.indices.len(),
        river_mesh.vertices.len(),
        river_mesh.indices.len(),
        border_state_mesh.vertices.len(),
        border_province_mesh.vertices.len(),
        border_culture_mesh.vertices.len(),
        burg_mesh.vertices.len(),
        burg_mesh.indices.len(),
    );

    let _l_biomes = renderer.add_layer_mesh(&biome_mesh);
    let _l_rivers = renderer.add_layer_mesh(&river_mesh);
    let _l_borders = renderer.add_layer_mesh(&border_state_mesh);
    let _l_bprov = renderer.add_layer_mesh(&border_province_mesh);
    let _l_bcult = renderer.add_layer_mesh(&border_culture_mesh);
    let _l_burgs = renderer.add_layer_mesh(&burg_mesh);

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
        world,
        layer_flags: LayerFlags::default(),
        picked_cell: None,
        autosave_enabled: true,
        autosave_interval: 60.0,
        last_autosave: Instant::now(),
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
                if *button == MouseButton::Right && *state == ElementState::Pressed {
                    let size = [
                        self.window.inner_size().width as f32,
                        self.window.inner_size().height as f32,
                    ];
                    let world = self.camera.screen_to_world(self.cursor_screen, size);
                    self.picked_cell = pick_cell(world, &self.world.pack.points);
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
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };

        let map_path = self.map_path.clone();
        let camera_center = self.camera.center;
        let camera_extent_y = self.camera.extent_y;
        let cursor_screen = self.cursor_screen;
        let surface_size = [
            self.window.inner_size().width as f32,
            self.window.inner_size().height as f32,
        ];
        let world_cursor = self.camera.screen_to_world(cursor_screen, surface_size);
        let picked_cell = self.picked_cell;
        let world = &self.world;

        let show_labels = self.layer_flags.labels;
        let label_data: Vec<(f32, f32, String)> = if show_labels {
            world
                .burgs
                .iter()
                .filter_map(|b| {
                    let pt = world.pack.points.get(b.cell as usize)?;
                    let s = self.camera.world_to_screen(*pt, surface_size);
                    if s[0] >= 0.0
                        && s[0] <= surface_size[0]
                        && s[1] >= 0.0
                        && s[1] <= surface_size[1]
                    {
                        Some((s[0], s[1], b.name.clone()))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        let pop_rate = world.settings.population_rate;
        let layer_flags = &mut self.layer_flags;
        let autosave_enabled = &mut self.autosave_enabled;
        let vorn_save_path = self.map_path.with_extension("vorn");

        let panel_w = 220.0; // SidePanel default width
        let output = self.egui_ctx.run(raw_input, |ctx| {
            // Labels overlay (solo si no caen debajo del panel lateral)
            let painter = ctx.debug_painter();
            for (sx, sy, name) in &label_data {
                if *sx < panel_w + 10.0 {
                    continue;
                }
                painter.text(
                    egui::pos2(*sx, *sy),
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );
            }

            egui::TopBottomPanel::top("vor-title").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Voronia -- {}", map_path.display()));
                });
            });

            egui::SidePanel::left("vor-panel")
                .resizable(false)
                .default_width(220.0)
                .show(ctx, |ui| {
                    ui.heading("visor");
                    ui.label(format!("FPS: {fps:.1}"));
                    ui.label(format!(
                        "centro: ({:.0}, {:.0})",
                        camera_center[0], camera_center[1]
                    ));
                    ui.label(format!("zoom: {camera_extent_y:.0}"));
                    ui.label(format!(
                        "cursor: ({:.0}, {:.0})",
                        world_cursor[0], world_cursor[1]
                    ));
                    ui.separator();

                    ui.heading("capas");
                    ui.checkbox(&mut layer_flags.heightmap, "heightmap");
                    ui.checkbox(&mut layer_flags.biomes, "biomas");
                    ui.checkbox(&mut layer_flags.rivers, "ríos");
                    ui.checkbox(&mut layer_flags.borders_state, "fronteras estados");
                    ui.checkbox(&mut layer_flags.borders_province, "fronteras provincias");
                    ui.checkbox(&mut layer_flags.borders_culture, "fronteras culturas");
                    ui.checkbox(&mut layer_flags.burgs, "burgos");
                    ui.checkbox(&mut layer_flags.labels, "labels");
                    ui.separator();

                    if let Some(cid) = picked_cell {
                        ui.heading(format!("celda #{cid}"));
                        let h = world.pack.cells.height.get(cid).copied().unwrap_or(0);
                        ui.label(format!("altura: {h}"));
                        let bi = world.pack.cells.biome.get(cid).copied().unwrap_or(0);
                        let name = world
                            .biomes
                            .get(bi as usize)
                            .map(|b| b.name.as_str())
                            .unwrap_or("?");
                        ui.label(format!("bioma: {name}"));
                        let sid = world.pack.cells.state.get(cid).copied().unwrap_or(0);
                        let sname = if sid > 0 {
                            world
                                .states
                                .get(sid as usize)
                                .map(|s| s.name.as_str())
                                .unwrap_or("?")
                        } else {
                            "Wildlands"
                        };
                        ui.label(format!("estado: {sname}"));
                        let cid2 = world.pack.cells.culture.get(cid).copied().unwrap_or(0);
                        let cname = if cid2 > 0 {
                            world
                                .cultures
                                .get(cid2 as usize)
                                .map(|c| c.name.as_str())
                                .unwrap_or("?")
                        } else {
                            "Wildlands"
                        };
                        ui.label(format!("cultura: {cname}"));
                        let pid = world.pack.cells.province.get(cid).copied().unwrap_or(0);
                        let pname = if pid > 0 {
                            world
                                .provinces
                                .get(pid as usize)
                                .map(|p| p.name.as_str())
                                .unwrap_or("?")
                        } else {
                            "\u{2014}"
                        };
                        ui.label(format!("provincia: {pname}"));
                        let bid = world.pack.cells.burg.get(cid).copied().unwrap_or(0);
                        let bname = if bid > 0 {
                            world
                                .burgs
                                .iter()
                                .find(|b| b.id == bid)
                                .map(|b| b.name.as_str())
                                .unwrap_or("?")
                        } else {
                            "\u{2014}"
                        };
                        ui.label(format!("burgo: {bname}"));
                        let rid = world.pack.cells.river.get(cid).copied().unwrap_or(0);
                        let rname = if rid > 0 {
                            world
                                .rivers
                                .iter()
                                .find(|r| r.id == rid)
                                .map(|r| r.name.as_str())
                                .unwrap_or("?")
                        } else {
                            "\u{2014}"
                        };
                        ui.label(format!("río: {rname}"));
                        let pop = world.pack.cells.population.get(cid).copied().unwrap_or(0.0);
                        ui.label(format!("población: {:.0} hab", pop * pop_rate));
                    } else {
                        ui.label("click derecho → seleccionar");
                    }
                    ui.separator();
                    ui.heading("autosave");
                    ui.checkbox(autosave_enabled, "autosave cada 60s");
                    if ui.button("save .vorn ahora").clicked() {
                        match vor_format::save::save_world(&vorn_save_path, world) {
                            Ok(_) => {
                                tracing::info!("guardado manual: {}", vorn_save_path.display());
                            }
                            Err(e) => {
                                tracing::warn!("save falló: {e}");
                            }
                        }
                    }
                });
        });

        // Upload textures (font atlas etc.)
        for (tex_id, img_delta) in &output.textures_delta.set {
            self.egui_renderer.update_texture(
                &self.renderer.device,
                &self.renderer.queue,
                *tex_id,
                img_delta,
            );
        }
        for tex_id in &output.textures_delta.free {
            self.egui_renderer.free_texture(tex_id);
        }

        let clipped = self
            .egui_ctx
            .tessellate(output.shapes, self.window.scale_factor() as f32);
        self.egui_winit
            .handle_platform_output(self.window.as_ref(), output.platform_output);

        let screen_size_px = self.window.inner_size();
        let pixels_per_point = self.window.scale_factor() as f32;
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [screen_size_px.width, screen_size_px.height],
            pixels_per_point,
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

        // Pass 1: capas de mapa (clear background + todas las capas activas)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vor-map"),
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

            for layer_idx in self.layer_flags.active_indices() {
                self.renderer.draw_layer(&mut pass, layer_idx);
            }
        }

        // Update egui buffers
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

        // ---- Autosave ----
        if self.autosave_enabled {
            let elapsed = self.last_autosave.elapsed().as_secs_f32();
            if elapsed >= self.autosave_interval {
                let vorn_path = self.map_path.with_extension("vorn");
                match vor_format::save::save_world(&vorn_path, &self.world) {
                    Ok(_) => {
                        tracing::info!("autosave: {}", vorn_path.display());
                        self.last_autosave = Instant::now();
                    }
                    Err(e) => {
                        tracing::warn!("autosave falló: {e}");
                    }
                }
            }
        }

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

/// Encuentra la celda pack más cercana a una coordenada de mundo.
/// Retorna `None` si la distancia supera `threshold*threshold`.
fn pick_cell(world: [f32; 2], points: &[[f32; 2]]) -> Option<usize> {
    let threshold = 400.0; // 20 px de radio al cuadrado
    points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let d2 = (p[0] - world[0]).powi(2) + (p[1] - world[1]).powi(2);
            (i, d2)
        })
        .filter(|&(_, d2)| d2 < threshold)
        .min_by(|&(_, a), &(_, b)| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
}

/// Color hex (#rrggbb) a [f32; 3] lineal aprox.
fn hex_color_to_linear(hex: &str) -> [f32; 3] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(hex.get(0..2).unwrap_or("00"), 16).unwrap_or(0);
    let g = u8::from_str_radix(hex.get(2..4).unwrap_or("00"), 16).unwrap_or(0);
    let b = u8::from_str_radix(hex.get(4..6).unwrap_or("00"), 16).unwrap_or(0);
    fn srgb_to_linear(c: u8) -> f32 {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)]
}

pub fn run_cli() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vor_app=info,vor_render=info,vor_import=info".into()),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let export_only = args.iter().any(|a| a == "--export-vorn");

    if args.len() < 2 {
        anyhow::bail!("uso: vor <path-.map> [--export-vorn]");
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

    if export_only {
        let vorn_path = path.with_extension("vorn");
        vor_format::save::save_world(&vorn_path, &loaded.world)
            .map_err(|e| anyhow::anyhow!("save .vorn: {e}"))?;
        info!("exportado: {}", vorn_path.display());
        return Ok(());
    }

    let mesh = build_mesh(&loaded.world.grid);
    let cfg = ViewerConfig {
        map_path: path,
        world: loaded.world,
        mesh,
    };
    run(cfg).map_err(|e| anyhow::anyhow!("visor: {e}"))
}
