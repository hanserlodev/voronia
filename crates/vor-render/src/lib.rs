//! `vor-render` -- pipeline wgpu de Voronia.
//!
//! Solo lectura sobre el World Data Model (`vor-core`): nunca lo muta (regla
//! dura del plan sec.5 + `references/architecture.md`). Dibuja capas de mapa
//! (heightmap primero, Fase 2; rios/biomas/burgos/... en Fase 3) cacheando
//! geometria triangulada para no reteselar en cada frame.
//!
//! ## Pipeline
//!
//! ```text
//! World Data Model
//!       |
//!       v
//! HeightmapLayer (CPU, lyon::tessellation) -> vertex/index buffers
//!       |
//!       v
//! Renderer (wgpu) --shader WGSL--> Surface
//!       ^
//!       |
//! Camera ortografica 2D (pan/zoom -> uniform buffer)
//! ```
//!
//! `vor-render` no depende de `vor-import` (la geometria ya esta poblada en
//! `World` cuando el renderer la lee).

pub mod biome;
pub mod border;
pub mod burg;
pub mod camera;
pub mod cells;
pub mod clip_poly;
pub mod coastline;
pub mod coastline_path;
pub mod coastline_stroke;
pub mod contour;
pub mod coordinates;
pub mod culture_layer;
pub mod grid;
pub mod heightmap;
pub mod ice_layer;
pub mod isoline;
pub mod lakes;
pub mod layers;
pub mod mesh;
pub mod population_layer;
pub mod precipitation;
pub mod prng;
pub mod province_layer;
pub mod relief;
pub mod religion_layer;
pub mod renderer;
pub mod river;
pub mod route_layer;
pub mod simplify;
pub mod state_layer;
pub mod temperature;
pub mod text;
pub mod texture;
pub mod water_gap;
pub mod zone_layer;

pub use biome::build_biome_mesh;
pub use border::{build_border_mesh, BorderKind};
pub use burg::build_burg_mesh;
pub use camera::Camera;
pub use cells::build_cell_wireframe;
pub use clip_poly::clip_polygon;
pub use coastline::{
    build_fractal_landmass_mesh, build_landmass_mesh_legacy, fractalize_polygon, FractalSettings,
};
pub use coastline_path::{
    build_coastline_path, coastline_path_to_lyon, CoastlinePath, CoastlineSpan, PathCommand,
};
pub use coastline_stroke::{
    build_coastline_shadow_mesh, build_coastline_stroke_mesh, CoastlineStrokeSettings,
};
pub use contour::build_contour_lines;
pub use coordinates::build_coordinate_lines;
pub use culture_layer::build_culture_mesh;
pub use grid::build_grid_lines;
pub use heightmap::{build_mesh, height_color, HeightmapMesh};
pub use ice_layer::build_ice_mesh;
pub use isoline::{
    connect_vertices, get_border_path, get_fill_path, get_halo_path, get_isolines,
    get_water_gap_path, IsolineOptions, IsolineOutput,
};
pub use lakes::build_lake_mesh;
pub use layers::LayerFlags;
pub use mesh::{build_landmass_mesh, build_pack_mesh, laplacian_smooth_vertices};
pub use population_layer::build_population_mesh;
pub use precipitation::build_precipitation_mesh;
pub use province_layer::build_province_mesh;
pub use relief::build_relief_mesh;
pub use religion_layer::build_religion_mesh;
pub use renderer::Renderer;
pub use river::build_river_mesh;
pub use route_layer::build_route_mesh;
pub use simplify::simplify;
pub use state_layer::build_state_mesh;
pub use temperature::build_temperature_mesh;
pub use text::TextSystem;
pub use texture::TextureOverlay;
pub use water_gap::{append_water_gap, build_water_gap_mesh};
pub use zone_layer::build_zone_mesh;
