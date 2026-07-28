mod png_export;
mod svg_export;
mod ui;

use egui_wgpu::Renderer as EguiRenderer;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tracing::info;
use vor_core::World;
use vor_edit::EditBuffer;
use vor_render::biome::{biome_colors_from_catalog, build_biome_mesh};
use vor_render::border::{build_border_mesh, BorderKind};
use vor_render::burg::build_burg_mesh;
use vor_render::cells::build_cell_wireframe;
use vor_render::contour::build_contour_lines;
use vor_render::coordinates::build_coordinate_lines;
use vor_render::culture_layer::build_culture_mesh;
use vor_render::grid::build_grid_lines;
use vor_render::heightmap::{build_mesh, HeightmapMesh};
use vor_render::ice_layer::build_ice_mesh;
use vor_render::lakes::build_lake_mesh;
use vor_render::layers::LayerFlags;
use vor_render::population_layer::build_population_mesh;
use vor_render::precipitation::build_precipitation_mesh;
use vor_render::province_layer::build_province_mesh;
use vor_render::relief::build_relief_mesh;
use vor_render::religion_layer::build_religion_mesh;
use vor_render::river::build_river_mesh;
use vor_render::route_layer::build_route_mesh;
use vor_render::state_layer::build_state_mesh;
use vor_render::temperature::build_temperature_mesh;
use vor_render::zone_layer::build_zone_mesh;
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

const PANEL_WIDTH: f32 = 240.0;

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
    edit_buffer: EditBuffer,
    dirty: bool,
    active_tab: ui::TabId,
    show_export_modal: bool,
    show_save_modal: bool,
    show_load_modal: bool,
    show_new_modal: bool,
    texture_name: String,
    texture_overlay: Option<vor_render::TextureOverlay>,

    // Indices de capas de líneas en renderer.line_layers
    line_cells_idx: usize,
    line_grid_idx: usize,
    line_contours_idx: usize,
    line_coordinates_idx: usize,
    line_routes_idx: usize,
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

    // --- Capas adicionales (orden de dibujo: bottom→top) ---

    // 1. Relief (landmass shading)
    let relief_mesh = build_relief_mesh(&world.pack);
    info!("relief mesh: {}v/{}i", relief_mesh.vertices.len(), relief_mesh.indices.len());
    let _l_relief = renderer.add_layer_mesh(&relief_mesh);

    // 2. Biomes (landmass color)
    let biome_colors = biome_colors_from_catalog(&world.biomes);
    let biome_mesh = build_biome_mesh(&world.pack, &biome_colors);
    info!("biomes mesh: {}v/{}i", biome_mesh.vertices.len(), biome_mesh.indices.len());
    let _l_biomes = renderer.add_layer_mesh(&biome_mesh);

    // 3. Climate: temperature, precipitation, ice
    let temp_mesh = build_temperature_mesh(&world.pack.vertices, &world.pack, &world.grid);
    info!("temperature mesh: {}v/{}i", temp_mesh.vertices.len(), temp_mesh.indices.len());
    let _l_temp = renderer.add_layer_mesh(&temp_mesh);

    let prec_mesh = build_precipitation_mesh(&world.pack.vertices, &world.pack, &world.grid);
    info!("precipitation mesh: {}v/{}i", prec_mesh.vertices.len(), prec_mesh.indices.len());
    let _l_prec = renderer.add_layer_mesh(&prec_mesh);

    let ice_mesh = build_ice_mesh(&world.ice);
    info!("ice mesh: {}v/{}i", ice_mesh.vertices.len(), ice_mesh.indices.len());
    let _l_ice = renderer.add_layer_mesh(&ice_mesh);

    // 4. Water: lakes, rivers
    let lake_mesh = build_lake_mesh(&world.pack);
    info!("lake mesh: {}v/{}i", lake_mesh.vertices.len(), lake_mesh.indices.len());
    let _l_lakes = renderer.add_layer_mesh(&lake_mesh);

    let river_mesh = build_river_mesh(&world.pack.points, &world.rivers);
    info!("rivers mesh: {}v/{}i", river_mesh.vertices.len(), river_mesh.indices.len());
    let _l_rivers = renderer.add_layer_mesh(&river_mesh);

    // 5. Human geography fills: states, provinces, cultures, religions, population, zones
    let state_mesh = build_state_mesh(&world.pack.vertices, &world.pack, &world.states);
    info!("state fill: {}v/{}i", state_mesh.vertices.len(), state_mesh.indices.len());
    let _l_state = renderer.add_layer_mesh(&state_mesh);

    let province_mesh = build_province_mesh(&world.pack.vertices, &world.pack, &world.provinces);
    info!("province fill: {}v/{}i", province_mesh.vertices.len(), province_mesh.indices.len());
    let _l_province = renderer.add_layer_mesh(&province_mesh);

    let culture_mesh = build_culture_mesh(&world.pack.vertices, &world.pack, &world.cultures);
    info!("culture fill: {}v/{}i", culture_mesh.vertices.len(), culture_mesh.indices.len());
    let _l_culture = renderer.add_layer_mesh(&culture_mesh);

    let religion_mesh = build_religion_mesh(&world.pack.vertices, &world.pack, &world.religions);
    info!("religion fill: {}v/{}i", religion_mesh.vertices.len(), religion_mesh.indices.len());
    let _l_religion = renderer.add_layer_mesh(&religion_mesh);

    let population_mesh = build_population_mesh(&world.pack.vertices, &world.pack);
    info!("population: {}v/{}i", population_mesh.vertices.len(), population_mesh.indices.len());
    let _l_population = renderer.add_layer_mesh(&population_mesh);

    let zone_mesh = build_zone_mesh(&world.pack.vertices, &world.pack, &world.zones);
    info!("zones: {}v/{}i", zone_mesh.vertices.len(), zone_mesh.indices.len());
    let _l_zones = renderer.add_layer_mesh(&zone_mesh);

    // 6. Borders & markers on top
    let border_state_mesh = build_border_mesh(&world.pack, BorderKind::State);
    let border_province_mesh = build_border_mesh(&world.pack, BorderKind::Province);
    let border_culture_mesh = build_border_mesh(&world.pack, BorderKind::Culture);
    info!(
        "borders (state/province/culture): {}v/{}i / {}v/{}i / {}v/{}i",
        border_state_mesh.vertices.len(), border_state_mesh.indices.len(),
        border_province_mesh.vertices.len(), border_province_mesh.indices.len(),
        border_culture_mesh.vertices.len(), border_culture_mesh.indices.len(),
    );
    let _l_borders = renderer.add_layer_mesh(&border_state_mesh);
    let _l_bprov = renderer.add_layer_mesh(&border_province_mesh);
    let _l_bcult = renderer.add_layer_mesh(&border_culture_mesh);

    let burg_mesh = build_burg_mesh(&world.pack, &world.states);
    info!("burgs: {}v/{}i", burg_mesh.vertices.len(), burg_mesh.indices.len());
    let _l_burgs = renderer.add_layer_mesh(&burg_mesh);

    // --- Line layers (cells, grid, contours, coordinates) ---
    let cells_mesh = build_cell_wireframe(&world.pack.vertices, world.pack.points_n());
    info!(
        "cells wireframe: {}v/{}i",
        cells_mesh.vertices.len(),
        cells_mesh.indices.len()
    );
    let line_cells_idx = renderer.add_line_layer(&cells_mesh);

    let grid_mesh = build_grid_lines(mesh_bounds_min, mesh_bounds_max);
    info!(
        "grid lines: {}v/{}i",
        grid_mesh.vertices.len(),
        grid_mesh.indices.len()
    );
    let line_grid_idx = renderer.add_line_layer(&grid_mesh);

    let contour_mesh = build_contour_lines(&world.grid);
    info!(
        "contour lines: {}v/{}i",
        contour_mesh.vertices.len(),
        contour_mesh.indices.len()
    );
    let line_contours_idx = renderer.add_line_layer(&contour_mesh);

    let coord_mesh = build_coordinate_lines(mesh_bounds_min, mesh_bounds_max);
    info!(
        "coordinate lines: {}v/{}i",
        coord_mesh.vertices.len(),
        coord_mesh.indices.len()
    );
    let line_coordinates_idx = renderer.add_line_layer(&coord_mesh);

    let route_mesh = build_route_mesh(&world.routes);
    info!(
        "route lines: {}v/{}i",
        route_mesh.vertices.len(),
        route_mesh.indices.len()
    );
    let line_routes_idx = renderer.add_line_layer(&route_mesh);

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

    let edit_buffer = EditBuffer::default();

    // Load default texture
    let texture_name = "marble-big".to_string();
    let texture_overlay = load_texture(
        &renderer.device,
        &renderer.queue,
        surface_format,
        &texture_name,
    );

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
        edit_buffer,
        dirty: false,
        texture_name,
        texture_overlay,
        line_cells_idx,
        line_grid_idx,
        line_contours_idx,
        line_coordinates_idx,
        line_routes_idx,
        active_tab: ui::TabId::Layers,
        show_export_modal: false,
        show_save_modal: false,
        show_load_modal: false,
        show_new_modal: false,
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
        let world = &mut self.world;

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

        let vorn_save_path = self.map_path.with_extension("vorn");

        // Destructure mutable refs to pass into the FnOnce closure
        let last_texture = self.texture_name.clone();
        let texture_name = &mut self.texture_name;
        let _texture_overlay = &mut self.texture_overlay;
        let active_tab = &mut self.active_tab;
        let layer_flags = &mut self.layer_flags;
        let edit_buffer = &mut self.edit_buffer;
        let dirty = &mut self.dirty;
        let show_export = &mut self.show_export_modal;
        let show_save = &mut self.show_save_modal;
        let show_load = &mut self.show_load_modal;
        let show_new = &mut self.show_new_modal;
        let autosave_enabled = &mut self.autosave_enabled;
        let camera = &mut self.camera;
        let renderer = &self.renderer;
        let mesh_bounds_min = self.mesh_bounds_min;
        let mesh_bounds_max = self.mesh_bounds_max;

        let output = self.egui_ctx.run(raw_input, |ctx| {
            // Labels overlay (clip below sidebar)
            let painter = ctx.debug_painter();
            for (sx, sy, name) in &label_data {
                if *sx < PANEL_WIDTH + 10.0 {
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

            // TopBar
            egui::TopBottomPanel::top("vor-title").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Voronia -- {}", map_path.display()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("FPS: {fps:.1}"));
                        ui.label(format!("zoom: {camera_extent_y:.0}"));
                    });
                });
            });

            // SidePanel with tabs
            egui::SidePanel::left("vor-panel")
                .resizable(false)
                .default_width(PANEL_WIDTH)
                .show(ctx, |ui| {
                    // --- Tab bar ---
                    ui.horizontal(|ui| {
                        for tab in &ui::TabId::ALL {
                            let selected = *active_tab == *tab;
                            if ui.selectable_label(selected, tab.label()).clicked() {
                                *active_tab = *tab;
                            }
                        }
                    });
                    ui.separator();

                    // --- Tab content ---
                    match *active_tab {
                        ui::TabId::Layers => ui::layers_tab(ui, layer_flags),
                        ui::TabId::Info => ui::info_tab(ui, picked_cell, world, edit_buffer, dirty),
                        ui::TabId::Tools => ui::tools_tab(ui, world, dirty),
                        ui::TabId::Options => ui::options_tab(ui),
                        ui::TabId::Style => ui::style_tab(ui, texture_name),
                        ui::TabId::About => ui::about_tab(ui),
                    }

                    // --- Debug info ---
                    ui.separator();
                    ui.label(format!(
                        "center: ({:.0}, {:.0})",
                        camera_center[0], camera_center[1]
                    ));
                    ui.label(format!(
                        "cursor: ({:.0}, {:.0})",
                        world_cursor[0], world_cursor[1]
                    ));

                    // --- Sticky footer ---
                    ui::footer_bar(
                        ui,
                        show_export,
                        show_save,
                        show_load,
                        show_new,
                        camera,
                        mesh_bounds_min,
                        mesh_bounds_max,
                        &surface_size,
                    );
                });

            // --- Modals (outside sidebar, centered) ---
            if *show_export {
                ui::export_modal(
                    ctx,
                    show_export,
                    renderer,
                    camera,
                    layer_flags,
                    &vorn_save_path,
                    world,
                );
            }
            if *show_save {
                ui::save_modal(ctx, show_save, world, &vorn_save_path, autosave_enabled);
            }
            if *show_load {
                ui::load_modal(ctx, show_load);
            }
            if *show_new {
                ui::new_map_modal(ctx, show_new);
            }
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

        // Pass 1: capas de mapa
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

            // Line layers
            if self.layer_flags.cells {
                self.renderer
                    .draw_line_layer(&mut pass, self.line_cells_idx);
            }
            if self.layer_flags.grid {
                self.renderer.draw_line_layer(&mut pass, self.line_grid_idx);
            }
            if self.layer_flags.contours {
                self.renderer
                    .draw_line_layer(&mut pass, self.line_contours_idx);
            }
            if self.layer_flags.coordinates {
                self.renderer
                    .draw_line_layer(&mut pass, self.line_coordinates_idx);
            }
            if self.layer_flags.routes {
                self.renderer
                    .draw_line_layer(&mut pass, self.line_routes_idx);
            }
        }

        // Pass 1.5: texture overlay (multiply blend over map)
        if self.layer_flags.texture {
            if let Some(ref tex) = self.texture_overlay {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("vor-texture"),
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
                });
                tex.draw(&mut pass);
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

        // Pass 2: egui overlay
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

        // ---- Texture reload (if name changed) ----
        if self.texture_name != last_texture {
            self.texture_overlay = load_texture(
                &self.renderer.device,
                &self.renderer.queue,
                self.renderer.format,
                &self.texture_name,
            );
        }

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

fn pick_cell(world: [f32; 2], points: &[[f32; 2]]) -> Option<usize> {
    let threshold = 400.0;
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

#[allow(dead_code)]
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

/// Texture names available as map overlays.
/// "none" means no texture (disabled).
pub const TEXTURES: &[&str] = &[
    "none",
    "marble-big",
    "marble-small",
    "gray-paper",
    "folded-paper-big",
    "folded-paper-small",
    "soiled-paper",
    "antique-big",
    "antique-small",
    "ocean",
];

/// Loads a texture from assets/textures/, returning None if name is "none" or loading fails.
fn load_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    name: &str,
) -> Option<vor_render::TextureOverlay> {
    if name == "none" {
        return None;
    }
    let asset_path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "assets",
        "textures",
        &format!("{}.jpg", name),
    ]
    .iter()
    .collect();
    let asset_path = if asset_path.exists() {
        asset_path
    } else {
        // Try .png
        let png: PathBuf = [
            env!("CARGO_MANIFEST_DIR"),
            "..",
            "..",
            "assets",
            "textures",
            &format!("{}.png", name),
        ]
        .iter()
        .collect();
        if png.exists() {
            png
        } else {
            tracing::warn!("texture not found: {name}");
            return None;
        }
    };
    match image::ImageReader::open(&asset_path) {
        Ok(reader) => match reader.decode() {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                tracing::info!("loaded texture: {name} ({w}x{h})");
                Some(vor_render::TextureOverlay::new(
                    device, queue, format, w, h, &rgba,
                ))
            }
            Err(e) => {
                tracing::warn!("failed to decode texture {name}: {e}");
                None
            }
        },
        Err(e) => {
            tracing::warn!("failed to open texture {name}: {e}");
            None
        }
    }
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
