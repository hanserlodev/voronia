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
use vor_render::biome::{biome_colors_from_catalog, build_biome_coast_fill, build_biome_mesh};
use vor_render::border::{build_border_mesh, BorderKind};
use vor_render::burg::build_burg_icons_mesh;
use vor_render::cells::build_cell_wireframe;
use vor_render::coastline::{build_fractal_landmass_mesh, FractalSettings};
use vor_render::coordinates::{build_coordinate_graticule, GraticuleLabel};
use vor_render::culture_layer::build_culture_mesh;
use vor_render::grid::build_grid_lines;
use vor_render::heightmap::HeightmapMesh;
use vor_render::ice_layer::build_ice_meshes;
use vor_render::lakes::build_lake_meshes;
use vor_render::layers::LayerFlags;
use vor_render::mesh::build_land_cells_mask_mesh;
use vor_render::population_layer::build_population_bars_mesh;
use vor_render::precipitation::build_precipitation_mesh;
use vor_render::province_layer::build_province_mesh;
use vor_render::relief::{build_relief_instances, ReliefIconsOverlay, ReliefSettings};
use vor_render::religion_layer::build_religion_mesh;
use vor_render::river::build_river_mesh;
use vor_render::route_layer::build_route_mesh;
use vor_render::state_layer::build_state_mesh;
use vor_render::temperature::build_temperature_mesh;
use vor_render::water_gap::append_water_gap;
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
    hover_cell: Option<usize>,
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
    /// FMG `data-x`/`data-y` paper shift, world units.
    texture_shift: [f32; 2],
    texture_overlay: Option<vor_render::TextureOverlay>,
    /// FMG `#oceanPattern`: tiled pattern image over the ocean base.
    ocean_pattern: Option<vor_render::OceanPatternOverlay>,
    /// FMG `#terrain`: relief icons atlas overlay.
    relief_overlay: Option<ReliefIconsOverlay>,
    /// FMG `#tempLabels`: isotherm label anchors (world px + °C).
    temp_labels: Vec<vor_render::temperature::TemperatureLabel>,
    /// FMG `temperatureScale` select (°C default).
    temp_unit: vor_render::temperature::TempUnit,
    /// FMG `g#wind` direction glyphs (world px).
    wind_glyphs: Vec<vor_render::precipitation::WindGlyph>,
    text_system: Option<vor_render::TextSystem>,

    /// Runtime-registered indices (lines + economy) for the FMG-ordered
    /// draw sequence.
    dyn_ids: vor_render::layers::DynamicLayerIds,
    /// Graticule labels (lat/long) drawn in world space when the coordinates
    /// layer is enabled.
    graticule_labels: Vec<GraticuleLabel>,
    /// Current graticule step in degrees (rebuilt on zoom like FMG
    /// `drawCoordinates`).
    coordinate_step: f32,
    /// Camera `extent_y` right after initial framing — reference for zoom
    /// scaling of text/graticule labels.
    fit_extent_y: f32,
}

async fn init_state(window: Arc<Window>, mut cfg: ViewerConfig) -> State {
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

    let mesh_bounds_min = cfg.mesh.bounds_min;
    let mesh_bounds_max = cfg.mesh.bounds_max;

    let world = cfg.world;

    // World bounds: the full world rectangle (Azgaar draws the sea filling the
    // world up to its edge; outside the world there is empty canvas, not more
    // ocean). The initial frame and the ocean quad both use these.
    let world_bounds_min = [0.0f32, 0.0f32];
    let world_bounds_max = [world.grid.width, world.grid.height];
    // FMG `#oceanBase` fill (default.json): #466eab, fully opaque.
    renderer.set_ocean(
        world_bounds_min,
        world_bounds_max,
        vor_render::biome::hex_color_to_linear("#466eab"),
    );

    // --- FMG `#ocean` group: bathymetry rings + tileable pattern ---
    let bathymetry_mesh = vor_render::ocean_layers::build_bathymetry_mesh(
        &world.grid,
        &vor_render::ocean_layers::DEFAULT_LIMITS,
    );
    info!(
        "ocean bathymetry: {}v/{}i",
        bathymetry_mesh.vertices.len(),
        bathymetry_mesh.indices.len()
    );
    // Registered later, after the fixed LAYER_* block (see below) so the
    // dynamic layers never shift the constants' `layers[idx-1]` mapping.

    let ocean_pattern_overlay = load_ocean_pattern(
        &renderer.device,
        &renderer.queue,
        renderer.format,
        renderer.msaa_count,
        world_bounds_min,
        world_bounds_max,
        renderer.camera_bind_layout(),
        "pattern1",
    );

    // --- Heightmap color overlay (layer 1) ---
    // Azgaar parity: filled isoline bands per height level (not flat per-cell
    // colors). The ocean is excluded (`height < 20`), matching `data-render: 0`.
    // Each band is fully opaque, so it uses the opaque layer path and no longer
    // relies on per-cell alpha to hide the sea.
    let heightmap_color_mesh = vor_render::build_heightmap_band_mesh(&world.pack);
    info!(
        "heightmap color overlay: {}v/{}i",
        heightmap_color_mesh.vertices.len(),
        heightmap_color_mesh.indices.len()
    );

    // --- Precompute water mask for the water gap ---
    let is_water: Vec<bool> = {
        let n = world.pack.points_n();
        let mut w = Vec::with_capacity(n);
        for p in 0..n {
            let h = world.pack.cells.height.get(p).copied().unwrap_or(0);
            let fid = world.pack.cells.feature_id.get(p).copied().unwrap_or(0);
            let is_lake = world
                .pack
                .features
                .iter()
                .any(|f| f.id == fid as u32 && f.kind == vor_core::feature::FeatureType::Lake);
            w.push(h < 20 || is_lake);
        }
        w
    };
    info!(
        "is_water array: {} cells, {} water",
        world.pack.points_n(),
        is_water.iter().filter(|&&w| w).count()
    );

    // --- Mask source (layer 0, stencil) = union of the fractal landmass and
    // the land cells ---
    // The fractal landmass can shrink below the land cells on small islands,
    // leaving those cells outside the mask and therefore unpainted (holes that
    // "fight the sea"). Merging a white mesh of every land cell into the mask
    // guarantees a paint-bucket fill of the landmass shape.
    let land_cells_mask =
        build_land_cells_mask_mesh(&world.pack.vertices, world.pack.points_n(), |p| {
            !is_water[p]
        });
    info!(
        "land cells mask: {}v/{}i",
        land_cells_mask.vertices.len(),
        land_cells_mask.indices.len()
    );
    if !land_cells_mask.indices.is_empty() {
        let shift = land_cells_mask.vertices.len() as u32;
        cfg.mesh
            .vertices
            .splice(0..0, land_cells_mask.vertices.clone());
        cfg.mesh
            .indices
            .splice(0..0, land_cells_mask.indices.clone());
        for idx in cfg
            .mesh
            .indices
            .iter_mut()
            .skip(land_cells_mask.indices.len())
        {
            *idx += shift;
        }
        cfg.mesh.bounds_min = land_cells_mask.bounds_min;
        cfg.mesh.bounds_max = land_cells_mask.bounds_max;
    }
    info!(
        "mask mesh (fractal ∪ cells): {}v/{}i",
        cfg.mesh.vertices.len(),
        cfg.mesh.indices.len()
    );
    renderer.set_mesh(&cfg.mesh);

    // --- Coastline strokes + drop shadow (FMG `#coastline`, always drawn) ---
    // Same fractal pipeline as the landmass fill: simplify → clipPoly →
    // fractalize → hybrid path, stroked with sea/lake island styles.
    let coastline_meshes = vor_render::build_coastline_meshes(
        &world.pack.vertices,
        &world.pack.features,
        world.grid.width,
        world.grid.height,
        &FractalSettings {
            seed: world.header.seed.parse::<u64>().unwrap_or(0),
            ..Default::default()
        },
        &vor_render::CoastlineStrokeSettings::default(),
    );
    info!(
        "coastline stroke: {}v/{}i, shadow: {}v/{}i",
        coastline_meshes.stroke.vertices.len(),
        coastline_meshes.stroke.indices.len(),
        coastline_meshes.shadow.vertices.len(),
        coastline_meshes.shadow.indices.len()
    );
    // Registered later, after the fixed LAYER_* block (see below).

    // --- Additional layers (draw order: bottom→top) ---

    // 1. Relief (FMG `#terrain`): Poisson-scattered symbol icons rendered
    // from the atlas overlay (drawn via `DrawItem::Relief`, not a mesh).
    let relief_icons = build_relief_instances(
        &world.pack,
        world.header.seed.parse::<u64>().unwrap_or(0),
        &ReliefSettings::default(),
    );
    info!("relief icons: {} instances", relief_icons.len());
    let relief_overlay = load_relief_atlas(
        &renderer.device,
        &renderer.queue,
        renderer.format,
        renderer.msaa_count,
        renderer.camera_bind_layout(),
        &relief_icons,
    );

    // 2. Biomes (landmass color)
    let biome_colors = biome_colors_from_catalog(&world.biomes);
    let mut biome_mesh = build_biome_mesh(&world.pack, &biome_colors);
    // Coast fill: the fractal coastline protrudes beyond the outermost land
    // cells (fractal displacement), leaving a white halo where the mask covers
    // but the cell fill does not. Recolor the fractal landmass with the nearest
    // land cell's biome and prepend it, so the cell fill paints on top and the
    // biome color reaches exactly up to the fractal coast.
    let coast_fill = build_biome_coast_fill(&cfg.mesh, &world.pack, &is_water, &biome_colors);
    if !coast_fill.indices.is_empty() {
        let shift = coast_fill.vertices.len() as u32;
        biome_mesh
            .vertices
            .splice(0..0, coast_fill.vertices.clone());
        biome_mesh.indices.splice(0..0, coast_fill.indices.clone());
        for idx in biome_mesh.indices.iter_mut().skip(coast_fill.indices.len()) {
            *idx += shift;
        }
        biome_mesh.bounds_min = coast_fill.bounds_min;
        biome_mesh.bounds_max = coast_fill.bounds_max;
        info!(
            "coast fill: {}v/{}i prepended to biomes mesh",
            coast_fill.vertices.len(),
            coast_fill.indices.len()
        );
    }
    append_water_gap(&mut biome_mesh, &world.pack, &is_water, |p| {
        let bi = world.pack.cells.biome.get(p).copied().unwrap_or(0) as usize;
        biome_colors
            .get(bi)
            .copied()
            .unwrap_or([0.0, 0.0, 0.0, 1.0])
    });
    info!(
        "biomes mesh + water gap: {}v/{}i",
        biome_mesh.vertices.len(),
        biome_mesh.indices.len()
    );

    // 3. Climate: temperature, precipitation, ice
    let temp_mesh = build_temperature_mesh(&world.grid);
    let temp_labels = vor_render::temperature::temperature_labels(&world.grid);
    info!(
        "temperature mesh: {}v/{}i",
        temp_mesh.vertices.len(),
        temp_mesh.indices.len()
    );
    // Temperature fills are semi-transparent (CSS `fill-opacity: 0.3`), so the
    // layer must be drawn with the alpha-blended pipeline to blend over biomes
    // instead of overwriting them. FMG masks it to land too (`mask: #land`),
    // so it registers masked + blended.

    let prec_mesh = build_precipitation_mesh(&world.grid);
    let wind_glyphs = vor_render::precipitation::wind_glyphs(
        &world.coordinates,
        &world.grid.points,
        world.grid.cells_x as usize,
        world.grid.cells_y as usize,
        world.grid.width,
        world.grid.height,
    );
    info!(
        "precipitation mesh: {}v/{}i",
        prec_mesh.vertices.len(),
        prec_mesh.indices.len()
    );

    let ice_meshes = build_ice_meshes(&world.ice);
    info!(
        "ice mesh: fill {}v/{}i, stroke {}v/{}i, shadow {}v/{}i",
        ice_meshes.fill.vertices.len(),
        ice_meshes.fill.indices.len(),
        ice_meshes.stroke.vertices.len(),
        ice_meshes.stroke.indices.len(),
        ice_meshes.shadow.vertices.len(),
        ice_meshes.shadow.indices.len()
    );

    // 4. Water: lakes, rivers
    let lake_meshes = build_lake_meshes(
        &world.pack,
        world.grid.width,
        world.grid.height,
        &FractalSettings {
            seed: world.header.seed.parse::<u64>().unwrap_or(0),
            ..Default::default()
        },
    );
    info!(
        "lake mesh: fill {}v/{}i, stroke {}v/{}i",
        lake_meshes.fill.vertices.len(),
        lake_meshes.fill.indices.len(),
        lake_meshes.stroke.vertices.len(),
        lake_meshes.stroke.indices.len()
    );

    let river_mesh = build_river_mesh(
        &world.pack.points,
        &world.rivers,
        world.settings.distance_scale,
        world.grid.width,
        world.grid.height,
    );
    info!(
        "rivers mesh: {}v/{}i",
        river_mesh.vertices.len(),
        river_mesh.indices.len()
    );

    // 5. Human geography fills: states, provinces, cultures, religions, population, zones
    // Each one carries its own water gap to keep colors from bleeding into the ocean.
    // States/provinces/cultures/religions use the common isoline engine
    // (`build_region_mesh` + `append_water_gap`), matching FMG `getIsolines`.

    let state_mesh = build_state_mesh(&world.pack.vertices, &world.pack, &world.states, &is_water);
    info!(
        "state fill + water gap: {}v/{}i",
        state_mesh.vertices.len(),
        state_mesh.indices.len()
    );

    let province_mesh = build_province_mesh(
        &world.pack.vertices,
        &world.pack,
        &world.provinces,
        &is_water,
    );
    info!(
        "province fill + water gap: {}v/{}i",
        province_mesh.vertices.len(),
        province_mesh.indices.len()
    );

    let culture_mesh = build_culture_mesh(
        &world.pack.vertices,
        &world.pack,
        &world.cultures,
        &is_water,
    );
    info!(
        "culture fill + water gap: {}v/{}i",
        culture_mesh.vertices.len(),
        culture_mesh.indices.len()
    );

    let religion_mesh = build_religion_mesh(
        &world.pack.vertices,
        &world.pack,
        &world.religions,
        &is_water,
    );
    info!(
        "religion fill + water gap: {}v/{}i",
        religion_mesh.vertices.len(),
        religion_mesh.indices.len()
    );

    let population_mesh = build_population_bars_mesh(
        &world.pack.vertices,
        &world.pack,
        &world.burgs,
        1.0, // urbanization factor (Azgaar default `urbanization` = 1)
    );
    info!(
        "population bars: {}v/{}i",
        population_mesh.vertices.len(),
        population_mesh.indices.len()
    );

    let zone_mesh = build_zone_mesh(&world.pack.vertices, &world.pack, &world.zones);
    info!(
        "zones: {}v/{}i",
        zone_mesh.vertices.len(),
        zone_mesh.indices.len()
    );

    // 6. Borders & markers on top
    let border_state_mesh = build_border_mesh(&world.pack, BorderKind::State);
    let border_province_mesh = build_border_mesh(&world.pack, BorderKind::Province);
    let border_culture_mesh = build_border_mesh(&world.pack, BorderKind::Culture);
    info!(
        "borders (state/province/culture): {}v/{}i / {}v/{}i / {}v/{}i",
        border_state_mesh.vertices.len(),
        border_state_mesh.indices.len(),
        border_province_mesh.vertices.len(),
        border_province_mesh.indices.len(),
        border_culture_mesh.vertices.len(),
        border_culture_mesh.indices.len(),
    );

    let burg_mesh = build_burg_icons_mesh(&world.pack, &world.states);
    info!(
        "burgs: {}v/{}i",
        burg_mesh.vertices.len(),
        burg_mesh.indices.len()
    );

    // --- Fixed layer block: registered EXACTLY in `LAYER_*` constant order ---
    // `Renderer::draw_layer(idx)` reads `self.layers[idx - 1]`, so THIS
    // registration order is the mapping between the `LayerFlags::LAYER_*`
    // constants (FMG `#viewbox` z-order) and the meshes. Reordering either
    // side without the other makes toggles activate the wrong layer — the
    // debug asserts below catch that in dev builds.
    let l_heightmap = renderer.add_layer_mesh(&heightmap_color_mesh); // #terrs
    assert_eq!(l_heightmap, vor_render::layers::LayerFlags::LAYER_HEIGHTMAP);
    let l_lakes = renderer.add_layer_mesh_blended(&lake_meshes.fill); // #lakes
    assert_eq!(l_lakes, vor_render::layers::LayerFlags::LAYER_LAKES);
    let l_biomes = renderer.add_layer_mesh_masked(&biome_mesh, false); // #biomes
    assert_eq!(l_biomes, vor_render::layers::LayerFlags::LAYER_BIOMES);
    // FMG `#rivers` carries `mask: url(#land)` (index.css) — the stencil of
    // the fractal landmass cuts each river exactly at the coastline.
    let l_rivers = renderer.add_layer_mesh_masked(&river_mesh, false); // #rivers
    assert_eq!(l_rivers, vor_render::layers::LayerFlags::LAYER_RIVERS);
    // Slot 5 (#terrain) stays occupied with an empty mesh so the LAYER_*
    // constants remain contiguous; the real relief drawing is the
    // `ReliefIconsOverlay` reached via `DrawItem::Relief`.
    let _relief_mesh = empty_heightmap_mesh();
    let l_relief = renderer.add_layer_mesh(&_relief_mesh); // #terrain
    assert_eq!(l_relief, vor_render::layers::LayerFlags::LAYER_RELIEF);
    let l_religion = renderer.add_layer_mesh_masked(&religion_mesh, true); // #relig
    assert_eq!(
        l_religion,
        vor_render::layers::LayerFlags::LAYER_RELIGION_FILL
    );
    let l_culture = renderer.add_layer_mesh_masked(&culture_mesh, true); // #cults
    assert_eq!(
        l_culture,
        vor_render::layers::LayerFlags::LAYER_CULTURE_FILL
    );
    let l_state = renderer.add_layer_mesh_masked(&state_mesh, true); // #regions
    assert_eq!(l_state, vor_render::layers::LayerFlags::LAYER_STATE_FILL);
    let l_province = renderer.add_layer_mesh_masked(&province_mesh, true); // #provs
    assert_eq!(
        l_province,
        vor_render::layers::LayerFlags::LAYER_PROVINCE_FILL
    );
    let l_zones = renderer.add_layer_mesh_blended(&zone_mesh); // #zones
    assert_eq!(l_zones, vor_render::layers::LayerFlags::LAYER_ZONES);
    let l_borders = renderer.add_layer_mesh_blended(&border_state_mesh); // #borders
    assert_eq!(
        l_borders,
        vor_render::layers::LayerFlags::LAYER_BORDER_STATE
    );
    let l_bprov = renderer.add_layer_mesh_blended(&border_province_mesh);
    assert_eq!(
        l_bprov,
        vor_render::layers::LayerFlags::LAYER_BORDER_PROVINCE
    );
    let l_bcult = renderer.add_layer_mesh_blended(&border_culture_mesh);
    assert_eq!(
        l_bcult,
        vor_render::layers::LayerFlags::LAYER_BORDER_CULTURE
    );
    // FMG `#temperature` has NO mask (index.css) — it paints over the ocean too.
    let l_temp = renderer.add_layer_mesh_blended(&temp_mesh); // #temperature
    assert_eq!(l_temp, vor_render::layers::LayerFlags::LAYER_TEMPERATURE);
    // FMG `#ice`: opacity 0.9 → alpha-blended; shadow/stroke are dynamic layers.
    let l_ice = renderer.add_layer_mesh_blended(&ice_meshes.fill); // #ice
    assert_eq!(l_ice, vor_render::layers::LayerFlags::LAYER_ICE);
    // FMG `#prec` has no mask by default (style-editor Clipping is optional).
    let l_prec = renderer.add_layer_mesh(&prec_mesh); // #prec
    assert_eq!(l_prec, vor_render::layers::LayerFlags::LAYER_PRECIPITATION);
    let l_population = renderer.add_layer_mesh_blended(&population_mesh); // #population
    assert_eq!(
        l_population,
        vor_render::layers::LayerFlags::LAYER_POPULATION
    );
    let l_burgs = renderer.add_layer_mesh(&burg_mesh); // #icons
    assert_eq!(l_burgs, vor_render::layers::LayerFlags::LAYER_BURGS);

    // --- Dynamic layers (indices captured in `dyn_ids`) ---
    // Registered AFTER the fixed block so `LAYER_*` ↔ `layers[idx-1]` stays
    // contiguous; their real indices travel inside `DynamicLayerIds`, so
    // position is free.
    let ocean_bathymetry_idx = renderer.add_layer_mesh_blended(&bathymetry_mesh);
    let layer_coastline_shadow_idx = renderer.add_layer_mesh_blended(&coastline_meshes.shadow);
    let layer_coastline_stroke_idx = renderer.add_layer_mesh_blended(&coastline_meshes.stroke);
    let layer_lake_stroke_idx = renderer.add_layer_mesh_blended(&lake_meshes.stroke);
    let layer_ice_shadow_idx = renderer.add_layer_mesh_blended(&ice_meshes.shadow);
    let layer_ice_stroke_idx = renderer.add_layer_mesh_blended(&ice_meshes.stroke);

    // Extra economy layers (goods / markets / trade) are registered AFTER all
    // the fixed-constant layers (0..18) so `active_indices()` keeps matching.
    // They are drawn explicitly in the render loop via the indices stored here.

    // 6b. Goods (cells + icons) — FMG `#goods` sub-layers.
    let goods_cells_mesh = vor_render::goods::build_goods_cells_mesh(&world.pack, &world.goods);
    let layer_goods_cells_idx = renderer.add_layer_mesh_masked(&goods_cells_mesh, true);
    info!(
        "goods cells mesh: {}v/{}i (layer {})",
        goods_cells_mesh.vertices.len(),
        goods_cells_mesh.indices.len(),
        layer_goods_cells_idx
    );
    let goods_icons_mesh = vor_render::goods::build_goods_icons_mesh(&world.pack, &world.goods);
    let layer_goods_icons_idx = renderer.add_layer_mesh_masked(&goods_icons_mesh, false);
    info!(
        "goods icons mesh: {}v/{}i (layer {})",
        goods_icons_mesh.vertices.len(),
        goods_icons_mesh.indices.len(),
        layer_goods_icons_idx
    );

    // 6c. Markets (fill + border + center) — FMG `#markets`.
    let market_fill_mesh = vor_render::market::build_market_fill_mesh(&world.pack, &world.markets);
    let layer_market_fill_idx = renderer.add_layer_mesh_blended(&market_fill_mesh);
    let market_border_mesh =
        vor_render::market::build_market_border_mesh(&world.pack, &world.markets);
    let layer_market_border_idx = renderer.add_layer_mesh_blended(&market_border_mesh);
    let market_center_mesh =
        vor_render::market::build_market_center_mesh(&world.markets, &world.burgs);
    let layer_market_center_idx = renderer.add_layer_mesh(&market_center_mesh);
    info!(
        "markets mesh: fill {}v/{}i, border {}v/{}i, center {}v/{}i (layers {}/{}/{})",
        market_fill_mesh.vertices.len(),
        market_fill_mesh.indices.len(),
        market_border_mesh.vertices.len(),
        market_border_mesh.indices.len(),
        market_center_mesh.vertices.len(),
        market_center_mesh.indices.len(),
        layer_market_fill_idx,
        layer_market_border_idx,
        layer_market_center_idx
    );

    // 6d. Trade routes — FMG `#tradeAnimation` (static approximation).
    let trade_mesh = vor_render::trade::build_trade_routes_mesh(
        &world.deals,
        &world.burgs,
        &world.markets,
        &world.goods,
    );
    let layer_trade_idx = renderer.add_layer_mesh_blended(&trade_mesh);
    info!(
        "trade routes mesh: {}v/{}i (layer {})",
        trade_mesh.vertices.len(),
        trade_mesh.indices.len(),
        layer_trade_idx
    );

    // --- Line layers (cells, grid, coordinates) ---
    let cells_mesh = build_cell_wireframe(&world.pack.vertices, world.pack.points_n());
    info!(
        "cells wireframe: {}v/{}i",
        cells_mesh.vertices.len(),
        cells_mesh.indices.len()
    );
    let line_cells_idx = renderer.add_line_layer(&cells_mesh);

    // FMG fills the grid pattern rect over the whole canvas
    // (max(mapWidth,graphWidth) × max(mapHeight,graphHeight)), not over the
    // landmass bounding box.
    let grid_mesh = build_grid_lines(world_bounds_min, world_bounds_max);
    info!(
        "grid lines: {}v/{}i",
        grid_mesh.vertices.len(),
        grid_mesh.indices.len()
    );
    let line_grid_idx = renderer.add_line_layer(&grid_mesh);

    // Initial step at scale = 1 (FMG: goal = lonT / scale / 10).
    let coordinate_step = vor_render::coordinates::pick_step(
        (world.coordinates.lon_r - world.coordinates.lon_l) / 10.0,
    );
    let coord_graticule = build_coordinate_graticule(
        &world.coordinates,
        world.grid.width,
        world.grid.height,
        coordinate_step,
    );
    info!(
        "coordinate graticule: {} lines ({:.0}° step), {} labels",
        coord_graticule.lines.vertices.len() / 2,
        coordinate_step,
        coord_graticule.labels.len()
    );
    let line_coordinates_idx = renderer.add_line_layer(&coord_graticule.lines);

    let route_mesh = build_route_mesh(&world.routes);
    info!(
        "route lines: {}v/{}i",
        route_mesh.vertices.len(),
        route_mesh.indices.len()
    );
    let line_routes_idx = renderer.add_line_layer(&route_mesh);

    let mut camera = Camera::new([0.0, 0.0], 1000.0, size.width, size.height);
    camera.frame_bounds(world_bounds_min, world_bounds_max);

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
        renderer.msaa_count,
        world_bounds_min,
        world_bounds_max,
        renderer.camera_bind_layout(),
        &texture_name,
    );

    let text_system = {
        let ts = vor_render::TextSystem::new(
            &renderer.device,
            &renderer.queue,
            surface_format,
            (size.width, size.height),
            renderer.msaa_count,
        );
        info!("glyphon TextSystem initialized");
        Some(ts)
    };

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
        hover_cell: None,
        autosave_enabled: true,
        autosave_interval: 60.0,
        last_autosave: Instant::now(),
        edit_buffer,
        dirty: false,
        texture_name,
        texture_shift: [0.0, 0.0],
        texture_overlay,
        ocean_pattern: ocean_pattern_overlay,
        relief_overlay,
        temp_labels,
        temp_unit: vor_render::temperature::TempUnit::C,
        wind_glyphs,
        text_system,
        dyn_ids: vor_render::layers::DynamicLayerIds {
            cells_line: line_cells_idx,
            grid_line: line_grid_idx,
            coordinates_line: line_coordinates_idx,
            routes_line: line_routes_idx,
            goods_cells: layer_goods_cells_idx,
            goods_icons: layer_goods_icons_idx,
            market_fill: layer_market_fill_idx,
            market_border: layer_market_border_idx,
            market_center: layer_market_center_idx,
            trade: layer_trade_idx,
            coastline_shadow: layer_coastline_shadow_idx,
            coastline_stroke: layer_coastline_stroke_idx,
            ocean_bathymetry: ocean_bathymetry_idx,
            lake_stroke: layer_lake_stroke_idx,
            ice_shadow: layer_ice_shadow_idx,
            ice_stroke: layer_ice_stroke_idx,
        },
        graticule_labels: coord_graticule.labels,
        coordinate_step,
        fit_extent_y: camera.extent_y,
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
                    if let Some(ts) = &mut self.text_system {
                        ts.resize(&self.renderer.queue, size.width, size.height);
                    }
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
                } else {
                    let size = [
                        self.window.inner_size().width as f32,
                        self.window.inner_size().height as f32,
                    ];
                    let world = self.camera.screen_to_world(self.cursor_screen, size);
                    self.hover_cell = pick_cell(world, &self.world.pack.points);
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

        // Cell tooltip (Azgaar-style): show info about the cell under the cursor.
        // Built here (outside the egui closure) and rendered with the debug painter.
        let hover_text: Option<String> = self.hover_cell.map(|cid| {
            let h = world.pack.cells.height.get(cid).copied().unwrap_or(0);
            let height_m = world.settings.height_m(h);
            let bi = world.pack.cells.biome.get(cid).copied().unwrap_or(0);
            let biome = world
                .biomes
                .get(bi as usize)
                .map(|b| b.name.as_str())
                .unwrap_or("?");
            format!(
                "Cell #{cid}  ·  Height: {height_m:.*}{}  ·  Biome: {biome}",
                0, world.settings.height_unit
            )
        });

        // Destructure mutable refs to pass into the FnOnce closure
        let last_texture = self.texture_name.clone();
        let texture_name = &mut self.texture_name;
        let texture_shift = &mut self.texture_shift;
        let temp_unit = &mut self.temp_unit;
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
        let dyn_ids = &self.dyn_ids;
        let mesh_bounds_min = self.mesh_bounds_min;
        let mesh_bounds_max = self.mesh_bounds_max;

        let output = self.egui_ctx.run(raw_input, |ctx| {
            // Cell tooltip (Azgaar-style): small box under the cursor.
            if let Some(text) = &hover_text {
                let painter = ctx.debug_painter();
                let w = 280.0;
                let h = 24.0;
                let x = (surface_size[0] - w) * 0.5;
                let y = surface_size[1] - 44.0;
                let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));
                painter.rect_filled(
                    rect,
                    4.0,
                    egui::Color32::from_rgba_unmultiplied(20, 22, 26, 220),
                );
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(13.0),
                    egui::Color32::from_rgb(240, 240, 240),
                );
            }

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
                        ui::TabId::Style => {
                            ui::style_tab(ui, texture_name, texture_shift, temp_unit)
                        }
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
                    dyn_ids,
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

        // FMG `scale` relative to the initial fit: >1 zoomed in, <1 out.
        let zoom_scale = self.fit_extent_y / self.camera.extent_y.max(1.0);

        // drawCoordinates redraws on every pan/zoom with
        // `goal = lonT / scale / 10`; we rebuild when the picked step changes.
        if self.layer_flags.coordinates {
            let goal = (self.world.coordinates.lon_r - self.world.coordinates.lon_l)
                / zoom_scale.max(1e-6)
                / 10.0;
            let new_step = vor_render::coordinates::pick_step(goal);
            if new_step != self.coordinate_step {
                let grat = build_coordinate_graticule(
                    &self.world.coordinates,
                    self.world.grid.width,
                    self.world.grid.height,
                    new_step,
                );
                self.renderer
                    .update_line_layer(self.dyn_ids.coordinates_line, &grat.lines);
                self.graticule_labels = grat.labels;
                self.coordinate_step = new_step;
            }
        }

        // ---- Wgpu passes ----
        let surface_texture = self.renderer.surface.get_current_texture()?;
        let resolve_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let msaa_view = self
            .renderer
            .msaa_view
            .as_ref()
            .expect("msaa_view present when the renderer was created with MSAA");

        // Update viewport (glyphon)
        if let Some(ref mut ts) = self.text_system {
            ts.resize(
                &self.renderer.queue,
                screen_size_px.width,
                screen_size_px.height,
            );
        }

        // Screen-space text overlays (glyphon), one combined batch:
        // graticule labels (#coordinates) + isotherm labels (#tempLabels).
        let mut text_labels: Vec<vor_render::Label> = Vec::new();
        let surface = [screen_size_px.width as f32, screen_size_px.height as f32];

        // Graticule labels: font size follows the FMG formula
        // (`desired / scale**0.8` world units). Longitude labels ride the
        // viewport's top edge, latitude labels the left edge.
        if self.layer_flags.coordinates {
            let font_px = vor_render::coordinates::label_font_px(12.0, zoom_scale);
            for gl in &self.graticule_labels {
                let margin = 4.0;
                let xy = if gl.is_latitude {
                    let s = self.camera.world_to_screen([0.0, gl.world_y], surface);
                    [margin, s[1] - font_px * 0.6]
                } else {
                    let s = self.camera.world_to_screen([gl.world_x, 0.0], surface);
                    [s[0] + margin, margin]
                };
                // FMG labels: fill #333333.
                text_labels.push(vor_render::Label::new(
                    &gl.text,
                    xy[0],
                    xy[1],
                    font_px,
                    [0.2, 0.2, 0.2, 1.0],
                ));
            }
        }

        // FMG `#tempLabels`: font-size 8px (world units → scales with zoom),
        // fill #000 at full opacity (overrides the group's 0.3 fill-opacity).
        if self.layer_flags.temperature {
            let font_px = (8.0 * zoom_scale).max(1.0);
            for tl in &self.temp_labels {
                let s = self.camera.world_to_screen([tl.x, tl.y], surface);
                if s[0] < -40.0
                    || s[1] < -40.0
                    || s[0] > surface[0] + 40.0
                    || s[1] > surface[1] + 40.0
                {
                    continue;
                }
                text_labels.push(vor_render::Label::new(
                    vor_render::temperature::convert_temperature(tl.temp_c, self.temp_unit),
                    s[0],
                    s[1] - font_px * 0.6,
                    font_px,
                    [0.0, 0.0, 0.0, 1.0],
                ));
            }
        }

        // FMG `g#wind`: 32px direction glyphs, fill inherited #003dff.
        if self.layer_flags.precipitation {
            let font_px = (32.0 * zoom_scale).max(1.0);
            for wg in &self.wind_glyphs {
                let s = self.camera.world_to_screen([wg.x, wg.y], surface);
                if s[0] < -60.0
                    || s[1] < -60.0
                    || s[0] > surface[0] + 60.0
                    || s[1] > surface[1] + 60.0
                {
                    continue;
                }
                text_labels.push(vor_render::Label::new(
                    wg.ch.to_string(),
                    s[0],
                    s[1] - font_px * 0.7,
                    font_px,
                    [0.0, 0.24, 1.0, 1.0], // #003dff
                ));
            }
        }

        if !text_labels.is_empty() {
            if let Some(ref mut ts) = self.text_system {
                ts.prepare(&self.renderer.device, &self.renderer.queue, &text_labels);
            }
        }

        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("vor-frame"),
                });

        // FMG auto-filter (invokeActiveZooming): the coastline drop-shadow is
        // applied while the zoom scale relative to the initial fit is ≤ 1.5
        // (a faint blur takes over above 2.6 — not replicated).
        let draw_opts = vor_render::layers::DrawOptions {
            coastline_shadow: zoom_scale <= vor_render::SHADOW_MAX_SCALE,
        };

        // Pass 1: map layers (renders to 4x MSAA, resolves to surface)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vor-map"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa_view,
                    resolve_target: Some(&resolve_view),
                    ops: wgpu::Operations {
                        // Neutral canvas outside the world (Azgaar shows no sea
                        // beyond the world edge — only inside it).
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.13,
                            g: 0.14,
                            b: 0.17,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: self.renderer.stencil_view().map(|view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: None,
                        stencil_ops: Some(wgpu::Operations {
                            // Clear the landmask every frame; layer 0 (the
                            // fractal landmass) stamps stencil = 1 right after.
                            load: wgpu::LoadOp::Clear(0),
                            store: wgpu::StoreOp::Store,
                        }),
                    }
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Ocean background: covers the world rectangle first (FMG #ocean),
            // then the tileable pattern rect (#oceanPattern, 20% opacity).
            self.renderer.draw_ocean(&mut pass);
            if let Some(ref pat) = self.ocean_pattern {
                pat.draw(&mut pass, &self.renderer.camera_bind);
            }

            // All map layers in the exact FMG `#viewbox` z-order (meshes and
            // line layers interleaved). PNG export iterates the same sequence.
            for item in self.layer_flags.draw_sequence(&self.dyn_ids, &draw_opts) {
                match item {
                    vor_render::layers::DrawItem::Mesh(idx) => {
                        self.renderer.draw_layer(&mut pass, idx);
                    }
                    vor_render::layers::DrawItem::Line(idx) => {
                        self.renderer.draw_line_layer(&mut pass, idx);
                    }
                    vor_render::layers::DrawItem::Texture => {
                        if let Some(ref tex) = self.texture_overlay {
                            // FMG `data-x`/`data-y` shift + stencil mask test
                            // (ref must match the mask pipelines).
                            tex.set_shift_world(&self.renderer.queue, self.texture_shift);
                            pass.set_stencil_reference(1);
                            tex.draw(&mut pass, &self.renderer.camera_bind);
                        }
                    }
                    vor_render::layers::DrawItem::Relief => {
                        if let Some(ref overlay) = self.relief_overlay {
                            overlay.draw(&mut pass, &self.renderer.camera_bind);
                        }
                    }
                }
            }

            // Text overlay (glyphon, inside the MSAA pass)
            if let Some(ref ts) = self.text_system {
                ts.render(&mut pass);
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

        // Pass 2: egui overlay (on the already-resolved surface)
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("vor-egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &resolve_view,
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

        // Trim glyphon atlas (frees the cache of unused glyphs)
        if let Some(ref mut ts) = self.text_system {
            ts.trim();
        }

        // ---- Texture reload (if name changed) ----
        if self.texture_name != last_texture {
            self.texture_overlay = load_texture(
                &self.renderer.device,
                &self.renderer.queue,
                self.renderer.format,
                self.renderer.msaa_count,
                [0.0, 0.0],
                [self.world.grid.width, self.world.grid.height],
                self.renderer.camera_bind_layout(),
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
                        tracing::warn!("autosave failed: {e}");
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
#[allow(clippy::too_many_arguments)]
fn load_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    msaa_count: u32,
    world_min: [f32; 2],
    world_max: [f32; 2],
    camera_layout: &wgpu::BindGroupLayout,
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
                    device,
                    queue,
                    format,
                    w,
                    h,
                    &rgba,
                    msaa_count,
                    world_min,
                    world_max,
                    camera_layout,
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

/// Loads an ocean tile pattern from assets/textures/ocean/ (FMG
/// `#oceanPattern` images, drawn at 20% opacity over the ocean base).
#[allow(clippy::too_many_arguments)]
fn load_ocean_pattern(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    msaa_count: u32,
    world_min: [f32; 2],
    world_max: [f32; 2],
    camera_layout: &wgpu::BindGroupLayout,
    name: &str,
) -> Option<vor_render::OceanPatternOverlay> {
    let asset_path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "assets",
        "textures",
        "ocean",
        &format!("{}.png", name),
    ]
    .iter()
    .collect();
    let rgba = match image::ImageReader::open(&asset_path) {
        Ok(reader) => match reader.decode() {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                tracing::warn!("failed to decode ocean pattern {name}: {e}");
                return None;
            }
        },
        Err(e) => {
            tracing::warn!("ocean pattern not found: {} ({e})", asset_path.display());
            return None;
        }
    };
    let (w, h) = rgba.dimensions();
    tracing::info!("loaded ocean pattern: {name} ({w}x{h})");
    Some(vor_render::OceanPatternOverlay::new(
        device,
        queue,
        format,
        w,
        h,
        &rgba,
        msaa_count,
        world_min,
        world_max,
        camera_layout,
        0.2,
    ))
}

/// An empty `HeightmapMesh` (used to keep fixed layer slots occupied when a
/// layer moved to a dedicated overlay pipeline).
fn empty_heightmap_mesh() -> vor_render::HeightmapMesh {
    vor_render::HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [0.0; 2],
        bounds_max: [0.0; 2],
    }
}

/// Loads the relief symbol atlas (assets/textures/relief/atlas.png, 3×3
/// cells rasterized from FMG's simple-set `<symbol>` defs) and builds the
/// icon overlay for the given instances.
#[allow(clippy::too_many_arguments)]
fn load_relief_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    msaa_count: u32,
    camera_layout: &wgpu::BindGroupLayout,
    icons: &[vor_render::relief::ReliefIcon],
) -> Option<ReliefIconsOverlay> {
    let asset_path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "assets",
        "textures",
        "relief",
        "atlas.png",
    ]
    .iter()
    .collect();
    let rgba = match image::ImageReader::open(&asset_path) {
        Ok(reader) => match reader.decode() {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                tracing::warn!("failed to decode relief atlas: {e}");
                return None;
            }
        },
        Err(e) => {
            tracing::warn!("relief atlas not found: {} ({e})", asset_path.display());
            return None;
        }
    };
    let (w, h) = rgba.dimensions();
    Some(ReliefIconsOverlay::new(
        device,
        queue,
        format,
        w,
        h,
        &rgba,
        msaa_count,
        camera_layout,
        icons,
    ))
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
        anyhow::bail!("usage: vor <path-to-.map> [--export-vorn]");
    }
    let path_idx = args
        .iter()
        .position(|a| !a.starts_with('-') && a != &args[0])
        .unwrap_or(1);
    let path = PathBuf::from(&args[path_idx]);
    info!("loading map: {}", path.display());

    let bytes = std::fs::read(&path)?;
    let raw =
        vor_import::mapfile::raw::parse(&bytes).map_err(|e| anyhow::anyhow!("parse .map: {e}"))?;
    let loaded = vor_import::mapfile::Loader::load(&raw)
        .map_err(|e| anyhow::anyhow!("Loader::load: {e}"))?;

    if export_only {
        let vorn_path = path.with_extension("vorn");
        vor_format::save::save_world(&vorn_path, &loaded.world)
            .map_err(|e| anyhow::anyhow!("save .vorn: {e}"))?;
        info!("exported: {}", vorn_path.display());
        return Ok(());
    }

    let landmass_mesh = build_fractal_landmass_mesh(
        &loaded.world.pack.vertices,
        &loaded.world.pack.features,
        loaded.world.grid.width,
        loaded.world.grid.height,
        |_feat| [1.0, 1.0, 1.0, 1.0],
        &FractalSettings {
            seed: loaded.world.header.seed.parse::<u64>().unwrap_or(0),
            ..Default::default()
        },
    );
    info!(
        "landmass mesh (features): {} vertices, {} indices",
        landmass_mesh.vertices.len(),
        landmass_mesh.indices.len()
    );
    let cfg = ViewerConfig {
        map_path: path,
        world: loaded.world,
        mesh: landmass_mesh,
    };
    run(cfg).map_err(|e| anyhow::anyhow!("viewer: {e}"))
}
