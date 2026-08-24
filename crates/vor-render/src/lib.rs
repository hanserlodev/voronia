//! `vor-render` -- the Voronia wgpu rendering pipeline.
//!
//! Read-only access to the World Data Model (`vor-core`): it never mutates it
//! (hard rule from plan sec.5). It draws map
//! layers (heightmap first, Phase 2; rivers/biomes/burgs/... in Phase 3),
//! caching triangulated geometry so it is not re-tessellated every frame.
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
//! Orthographic 2D camera (pan/zoom -> uniform buffer)
//! ```
//!
//! `vor-render` does not depend on `vor-import` (the geometry is already
//! populated in `World` when the renderer reads it).

pub mod biome;
pub mod border;
pub mod burg;
pub mod camera;
pub mod cells;
pub mod clip_poly;
pub mod coastline;
pub mod coastline_path;
pub mod coastline_stroke;
pub mod coordinates;
pub mod culture_layer;
pub mod goods;
pub use goods::{
    build_goods_burg_plates, build_goods_icon_circles_mesh, goods_icon_quads, BurgPlateLabel,
    GoodsIconQuad, GoodsIconsOverlay,
};
pub mod grid;
pub mod heightmap;
pub mod ice_layer;
pub mod isoline;
pub mod lakes;
pub mod layers;
pub mod market;
pub mod mesh;
pub mod ocean_layers;
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
pub mod trade;
pub mod water_gap;
pub mod zone_layer;

pub use biome::{
    biome_colors_from_catalog, build_biome_coast_fill, build_biome_isolines_meshes,
    build_biome_mesh, BiomeIsolineMeshes,
};
pub use border::{build_border_mesh, BorderKind};
pub use burg::build_burg_icons_mesh;
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
    build_coastline_meshes, CoastlineMeshes, CoastlineStrokeSettings, SHADOW_MAX_SCALE,
};
pub use coordinates::{build_coordinate_graticule, GraticuleLabel, GraticuleMesh};
pub use culture_layer::build_culture_mesh;
pub use grid::build_grid_lines;
pub use heightmap::{build_mesh, height_color, HeightmapMesh};
pub use ice_layer::{build_ice_meshes, IceMeshes};
pub use isoline::{
    build_heightmap_band_mesh, build_region_mesh, build_vertex_path_mesh, connect_vertices,
    get_border_path, get_fill_path, get_halo_path, get_isolines, get_water_gap_path,
    IsolineOptions, IsolineOutput,
};
pub use lakes::{build_lake_meshes, LakeMeshes};
pub use layers::LayerFlags;
pub use mesh::{
    build_land_cells_mask_mesh, build_landmass_mesh, build_pack_mesh, laplacian_smooth_vertices,
};
pub use population_layer::build_population_bars_mesh;
pub use precipitation::build_precipitation_mesh;
pub use province_layer::build_province_mesh;
pub use relief::{
    build_relief_instances, poisson_disc, ReliefIcon, ReliefIconsOverlay, ReliefSettings, SYMBOLS,
};
pub use religion_layer::build_religion_mesh;
pub use renderer::{stencil_passthrough, Renderer, STENCIL_FORMAT};
pub use river::{build_river_mesh, get_offset, get_width};
pub use route_layer::{build_route_group_meshes, RouteMeshes};
pub use simplify::simplify;
pub use state_layer::build_state_mesh;
pub use temperature::build_temperature_mesh;
pub use text::{Label, TextSystem};
pub use texture::{OceanPatternOverlay, TextureOverlay};
pub use water_gap::{append_water_gap, build_water_gap_mesh};
pub use zone_layer::build_zone_mesh;
