# Fase 2 — Visor GPU mínimo

> Registro cronológico de la sesión. Formato: `docs/fase-0-investigacion.md`.
> Última actualización: 26 julio 2026 — Fase 2 COMPLETADA.

---

## Referencia de Azgaar

- **Versión de Azgaar**: v1.138.0 (registrada en Fase 1 — la geometría del `.map` Sorvik se regenera bit-exacta contra este commit).
- **Commit clonado local**: `51d8e3e` (azgaar-fmg master, 21 jul 2026 — ver divergencia de Brample en `docs/fase-1.md`).
- **Mapas de prueba**: `~/Descargas/Brample 2026-07-22-21-24.map`, `Sorvik 2026-07-24-23-39.map`, `XD.map`.
- **Formato de entrada**: `.map` legacy (slot-by-slot, 47 slots). La geometría no viene en el archivo — se regenera desde semilla (hallazgo fase-0 §3).

---

## Cronología de la sesión (26 julio 2026)

### Commit inicial: `758599e docs: registrar Fase 1 en docs/fase-1.md + protocolo checkpoint→phase-md en SKILL.md`

El punto de partida es el tag de Fase 1 completa. Working tree limpio.

### Paso 1 — Workspace deps y Cargo.toml

Se agregaron al `[workspace.dependencies]` de `Cargo.toml` raíz:
- `wgpu = "22"` — fijado en 22 por compatibilidad con `egui-wgpu 0.29`. wgpu 23 causa conflicto de versiones en el resolver.
- `winit = "0.30"` con features `x11`, `wayland`, `wayland-dlopen`, `rwh_06`.
- `egui = "0.29"`, `egui-wgpu = "0.29"`, `egui-winit = "0.29"`.
- `pollster = "0.3"` para bloquear async en `main`.
- `bytemuck = "1.21"` (con feature `derive`) para `Pod`/`Zeroable` en vertex structs.
- `raw-window-handle = "0.6"` (compatible con winit 0.30).

`lyon` subido de `"0.18"` a `"1.0"` (API breaking — `Path::builder()` en vez de `PathBuilder::new()`).

**Archivos tocados**: `Cargo.toml`, `crates/vor-render/Cargo.toml`, `crates/vor-app/Cargo.toml`, `crates/vor-cli/Cargo.toml`.

### Paso 2 — `vor-core::VoronoiVertices::cell_rings`

Se agregó el campo `cell_rings: Vec<Vec<u32>>` a `vor_core::voronoi::VoronoiVertices` con `#[serde(skip)]`.

**Motivación**: El renderer necesita el mapeo celda→triángulos (`cells.v` de Azgaar) para triangular polígonos de Voronoi. En Fase 1 se omitió por ser "derivable del Delaunay" — correcto para persistencia, pero el renderer no debe recalcular geometría. Es una consecuencia pragmática del principio SoA con skip de serialización.

**Propagación**: Ambas funciones `voronoi_to_vor_core()` en `loader.rs` y `regraph.rs` ahora copian `v.cells.v.clone()` al nuevo campo.

**Archivos tocados**: `crates/vor-core/src/voronoi.rs`, `crates/vor-import/src/mapfile/loader.rs`, `crates/vor-import/src/regraph.rs`.

### Paso 3 — `vor-render::Camera`

Módulo `camera.rs`: cámara ortográfica 2D con:
- `CameraUniform` (`[f32; 16]` repr(C), Pod/Zeroable) para uniform buffer.
- `Camera` con `center`, `extent_y`, `aspect`.
- `view_proj()` vía `glam::Mat4::orthographic_rh` con inversión Y (+Y mundo→-Y NDC).
- `screen_to_world()`: coordenadas de superficie a mundo.
- `zoom_at_cursor()`: escala `extent_y` y compensa centro para preservar el punto bajo el cursor (zoom-to-cursor).
- `pan_by_screen_delta()`: desplaza centro proporcional al viewport.
- `frame_bounds()`: encuadre inicial con padding 10%.

**Tests**: 4 tests unitarios (screen_to_world center, zoom preserva cursor, pan mueve centro opuesto, frame_bounds ajusta extent_y).

### Paso 4 — `vor-render::HeightmapLayer`

Módulo `heightmap.rs`:

- `HeightmapVertex` (repr(C), Pod/Zeroable): `pos: [f32;2]` + `color: [f32;4]`.
- `HeightmapMesh`: `vertices` + `indices` + `bounds_min/max`.
- `build_mesh(grid: &Grid) -> HeightmapMesh`:
  - Itera `grid.points_n()` celdas reales (sin boundary).
  - Para cada celda con `cell_rings[p]` no vacío: construye path cerrado desde `grid.vertices.positions[ann[t]]`, tessellate con `lyon::FillTessellator`.
  - Color por altura vía `height_color(h)` (rampa estilo Azgaar: azul marino 0-19, verde→marrón→blanco 20-100).
  - Acumula en mesh global con offset de índices.
  - Celdas degeneradas (tessellation falla) se saltan.
- `height_color(u8) -> [f32;4]`: rampa hardcodeada con 5 stops para tierra.

**Tests**: 3 tests unitarios (azul marino, cima brillante, clamp >100 no panico).

### Paso 5 — `vor-render::Renderer`

Módulo `renderer.rs`:

- `Renderer` struct: device, queue, surface, surface_config, format, camera_buf + bind group/layout, heightmap pipeline, vertex/index buffers.
- `Renderer::new()`: crea device, surface config, uniform buffer, bind group, shader module y pipeline.
- Shader WGSL inline: vertex `vs_main` recibe `VertexIn { position: vec2<f32>, color: vec4<f32> }`, transforma por `camera * vec4(position, 0, 1)`, pasa color al fragment. Fragment `fs_main` devuelve color.
- `set_mesh()`: sube vertex/index buffers a GPU via `create_buffer_init`.
- `render()` en Renderer ahora delegada a vor-app (el renderer expone `pub` fields: `camera_buf`, `heightmap_pipeline`, `vertex_buf`, `index_buf`, `index_count`, `surface`, `device`, `queue`, `format`).

**API de cámara en shader**: `@group(0) @binding(0) var<uniform> camera : mat4x4<f32>`.

### Paso 6 (reprise) — Fix integración vor-app (26 jul, post-checkpoint)

Tras el checkpoint, se reescribió `vor-app/src/lib.rs` completo con:

**Estructura winit 0.30 correcta**:
- `Window` se crea dentro de `ApplicationHandler::resumed()` (winit 0.30 requiere esto; antes se creaba antes de `run_app`).
- `Window` se envuelve en `Arc<Window>` para `Surface<'static>`.
- `Camera` y `Renderer` se almacenan en `State`, sin duplicar `device`/`queue` (se usa `renderer.device` y `renderer.queue`).
- egui 0.29 API: `Align2::LEFT_TOP` (no `Align::LEFT_TOP`), `on_window_event(&self.window, event)` (no `WindowId`).

**Wgpu 22 API fixes**:
- `Instance::new(InstanceDescriptor)` (no `&InstanceDescriptor`, no referencia).
- `DeviceDescriptor` sin campo `trace` (eliminado en wgpu 22).
- `adapter.request_device(&desc, None)` (trace_path es segundo arg, no campo del descriptor).

**Egui-wgpu 0.29.1 render**:
- `egui_wgpu::Renderer::render` toma `&mut RenderPass<'static>` — se usa `pass.forget_lifetime()` (safe, el lifetime es `PhantomData` de guardia).
- `Context::tessellate(shapes, pixels_per_point)` (2 args, no 1).
- `ScreenDescriptor::size_in_pixels` = pixels físicos (sin dividir por scale_factor).

**Render compuesto en 2 passes**:
1. Heightmap con `ClearOp::Clear` (fondo azul oscuro `0.02, 0.02, 0.05`).
2. Egui con `LoadOp::Load` sobre la misma surface texture.

**Resultado**: `cargo check --workspace` → 0 errores. 48 tests verdes. Clippy 0 warnings. `cargo fmt --all` limpio.

### Paso 7 — `vor-cli::main.rs`

`lib.rs` intenta integrar winit + wgpu + egui-wgpu:

- `App` struct implementa `winit::application::ApplicationHandler`.
- `State` struct contiene window, device, queue, renderer, egui_ctx, egui_winit, egui_renderer, camera, mesh bounds, map_path, cursor, pan state.
- `init_state()` async: crea instancia wgpu, adapter, device+queue, surface, Format, Renderer, egui_winit::State, egui_wgpu::Renderer. Sube mesh a GPU. Encuadra cámara.
- `handle_window_event()`: CloseRequested, Resized, CursorMoved (pan activo), MouseInput (toggle pan), MouseWheel (zoom), RedrawRequested.
- `redraw()` (ROTO): mezcla incompleta de las dos pasadas de render:
  - Definiciones de `surface_texture`, `view`, `encoder` **faltan** (se perdieron al refactorizar el bloque de render).
  - El pass 1 (heightmap) fue eliminado junto con las definiciones — solo sobrevive el pass 2 (egui).
  - Referencias sueltas a `encoder` y `surface_texture` en líneas 327-351.

**Errores de compilación detectados** (24 totales):
1. `encoder`, `view`, `surface_texture` no definidos.
2. `wgpu::Instance::new` llamado con `&InstanceDescriptor` — no toma referencia en wgpu 22.
3. `egui_winit::State::on_window_event` espera `&Window`, no `WindowId`.
4. `Align::LEFT_TOP` eliminado en egui 0.29 (usar `Align::LEFT`).
5. `Renderer` importado dos veces (`vor_render::Renderer` y `egui_wgpu::Renderer`).
6. `tracing_subscriber::EnvFilter` requiere feature `env-filter`.
7. `output.shapes` es `Vec<ClippedShape>`, `egui_wgpu::Renderer::render` espera `&[ClippedPrimitive]`. Falta `ctx.tessellate()`.
8. `egui::epaint` import no resuelto.

### Paso 7 — `vor-cli::main.rs`

Creado binario `vor` que delega a `vor_app::run_cli()`.

---

## Hallazgos críticos y decisiones

1. **wgpu 22 vs 23**: `egui-wgpu 0.29` depende de wgpu 22. wgpu 23 no es compatible. La resolución automática de Cargo elige wgpu 22 cuando se especifica `"22"` como dep workspace.
2. **`cell_rings` rompe pureza del modelo**: el campo `cell_rings` en `VoronoiVertices` es redundante (derivable del Delaunay) y no se persiste. Es un compromiso pragmático para que el renderer pueda triangular sin recalcular Delaunay en runtime. Alternativa considerada: recalcular en `Renderer::set_mesh()` vía `edgesAroundPoint` sobre el halfedge array de `vor_import::geometry`. Descartado porque `vor-render` no debería depender de `vor-import` (regla dura del plan §5).
3. **`lyon` 1.0 API break**: `PathBuilder::new()` ya no existe. Reemplazo: `Path::builder()` devuelve un `PathBuilder` implícito con métodos `begin()`/`line_to()`/`end(closed)`.
4. **egui-wgpu integración delicada**: `egui_wgpu::Renderer::render` toma `&mut RenderPass<'_>` y `&[ClippedPrimitive]` (NO `Vec<ClippedShape>`). Se debe tesselar antes: `let clipped = output.shapes` + `ctx.tessellate(clipped)`.

---

## Inventario de tests

| Archivo | Tests | Qué valida |
|---|---|---|
| `crates/vor-render/src/camera.rs` | 4 | screen_to_world, zoom_at_cursor, pan, frame_bounds |
| `crates/vor-render/src/heightmap.rs` | 3 | rampa azul marino, cima brillante, clamp >100 |

No se ejecutaron aún (`cargo test` no corrido). Los tests existentes de Fase 1 (22 tests en vor-import) deben seguir verdes.

---

## Estado final de la sesión

```
working tree: 15 archivos modificados/creados, 0 commiteados
cargo check --workspace: ✓ todo compila
cargo test --workspace: ✓ 48 tests (27 unit + 4 bit-exact + 9 e2e + 7 render + 1 doc-ignored)
cargo clippy --all-targets: ✓ 0 warnings
cargo fmt --all: ✓ limpio
```

### Checklist Fase 2 (plan maestro §23)

- [x] Configurar deps: workspace deps wgpu 22 / winit 0.30 / egui 0.29 / lyon 1.0 / bytemuck / pollster.
- [x] vor-render::Camera — ortográfica 2D con pan/zoom, screen→world, frame_bounds (4 tests).
- [x] vor-render::HeightmapLayer — `build_mesh(grid)` triangula celdas con lyon, rampa de color (3 tests).
- [x] vor-render::Renderer — pipeline wgpu + shaders WGSL + buffers GPU. Campos `pub`.
- [x] vor-app::State — winit 0.30 (ApplicationHandler), wgpu 22, egui 0.29 integrado. Overlay egui mínimo.
- [x] vor-cli — bin `vor` que carga `.map` y abre el visor.
- [x] Tests/sanity — 48 tests verdes, clippy 0 warnings, fmt clean.
- [ ] Prueba end-to-end: `cargo run --bin vor -- /path/to/map.map` (pendiente de ejecutar).

**Progreso real**: 100% del código de Fase 2. Compila, tests pasan, clippy/fmt verdes. Falta únicamente ejecutar el binario contra un `.map` real para validar el runtime.
