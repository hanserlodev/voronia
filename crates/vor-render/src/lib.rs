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
pub mod heightmap;
pub mod layers;
pub mod mesh;
pub mod renderer;
pub mod river;

pub use biome::build_biome_mesh;
pub use border::{build_border_mesh, BorderKind};
pub use burg::build_burg_mesh;
pub use camera::Camera;
pub use heightmap::{build_mesh, HeightmapMesh};
pub use layers::LayerFlags;
pub use mesh::build_pack_mesh;
pub use renderer::Renderer;
pub use river::build_river_mesh;
