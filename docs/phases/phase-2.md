# Phase 2 — Minimal GPU viewer

> Chronological session log. Format: `docs/phases/phase-0-research.md`.
> Last updated: July 26, 2026 — Phase 2 COMPLETED.

---

## Azgaar reference

- **Azgaar version**: v1.138.0 (recorded in Phase 1 — the geometry of the Sorvik `.map` is regenerated bit-exact against this commit).
- **Cloned local commit**: `51d8e3e` (azgaar-fmg master, Jul 21, 2026 — see the Brample divergence in `docs/phases/phase-1.md`).
- **Test maps**: `Brample 2026-07-22-21-24.map`, `Sorvik 2026-07-24-23-39.map` (committed at `crates/vor-import/tests/reference/`), `XD.map`.
- **Input format**: legacy `.map` (slot-by-slot, 47 slots). The geometry is not stored in the file — it is regenerated from the seed (finding phase-0 §3).

---

## Session chronology (July 26, 2026)

### Initial commit: `758599e docs: registrar Fase 1 en docs/phases/phase-1.md + protocolo checkpoint→phase-md en SKILL.md`

The starting point is the Phase 1 complete tag. Clean working tree.

### Step 1 — Workspace deps and Cargo.toml

Added to the root `Cargo.toml` `[workspace.dependencies]`:
- `wgpu = "22"` — pinned to 22 for compatibility with `egui-wgpu 0.29`. wgpu 23 causes a version conflict in the resolver.
- `winit = "0.30"` with features `x11`, `wayland`, `wayland-dlopen`, `rwh_06`.
- `egui = "0.29"`, `egui-wgpu = "0.29"`, `egui-winit = "0.29"`.
- `pollster = "0.3"` to block async in `main`.
- `bytemuck = "1.21"` (with the `derive` feature) for `Pod`/`Zeroable` on vertex structs.
- `raw-window-handle = "0.6"` (compatible with winit 0.30).

`lyon` bumped from `"0.18"` to `"1.0"` (breaking API — `Path::builder()` instead of `PathBuilder::new()`).

**Files touched**: `Cargo.toml`, `crates/vor-render/Cargo.toml`, `crates/vor-app/Cargo.toml`, `crates/vor-cli/Cargo.toml`.

### Step 2 — `vor-core::VoronoiVertices::cell_rings`

Added the field `cell_rings: Vec<Vec<u32>>` to `vor_core::voronoi::VoronoiVertices` with `#[serde(skip)]`.

**Motivation**: The renderer needs the cell→triangles mapping (`cells.v` of Azgaar) to triangulate Voronoi polygons. It was omitted in Phase 1 as "derivable from the Delaunay" — correct for persistence, but the renderer should not recompute geometry. It is a pragmatic consequence of the SoA principle with serialization skip.

**Propagation**: Both `voronoi_to_vor_core()` functions in `loader.rs` and `regraph.rs` now copy `v.cells.v.clone()` to the new field.

**Files touched**: `crates/vor-core/src/voronoi.rs`, `crates/vor-import/src/mapfile/loader.rs`, `crates/vor-import/src/regraph.rs`.

### Step 3 — `vor-render::Camera`

Module `camera.rs`: 2D orthographic camera with:
- `CameraUniform` (`[f32; 16]` repr(C), Pod/Zeroable) for the uniform buffer.
- `Camera` with `center`, `extent_y`, `aspect`.
- `view_proj()` via `glam::Mat4::orthographic_rh` with Y inversion (+Y world→-Y NDC).
- `screen_to_world()`: surface coordinates to world.
- `zoom_at_cursor()`: scales `extent_y` and compensates the center to keep the point under the cursor fixed (zoom-to-cursor).
- `pan_by_screen_delta()`: shifts the center proportionally to the viewport.
- `frame_bounds()`: initial framing with 10% padding.

**Tests**: 4 unit tests (screen_to_world center, zoom keeps cursor fixed, pan moves center opposite, frame_bounds adjusts extent_y).

### Step 4 — `vor-render::HeightmapLayer`

Module `heightmap.rs`:

- `HeightmapVertex` (repr(C), Pod/Zeroable): `pos: [f32;2]` + `color: [f32;4]`.
- `HeightmapMesh`: `vertices` + `indices` + `bounds_min/max`.
- `build_mesh(grid: &Grid) -> HeightmapMesh`:
  - Iterates `grid.points_n()` real cells (no boundary).
  - For each cell with a non-empty `cell_rings[p]`: builds a closed path from `grid.vertices.positions[ann[t]]`, tessellates with `lyon::FillTessellator`.
  - Colors by height via `height_color(h)` (Azgaar-style ramp: navy blue 0-19, green→brown→white 20-100).
  - Accumulates into a global mesh with index offset.
  - Degenerate cells (tessellation fails) are skipped.
- `height_color(u8) -> [f32;4]`: hardcoded ramp with 5 stops for land.

**Tests**: 3 unit tests (navy blue, bright summit, clamp >100 does not panic).

### Step 5 — `vor-render::Renderer`

Module `renderer.rs`:

- `Renderer` struct: device, queue, surface, surface_config, format, camera_buf + bind group/layout, heightmap pipeline, vertex/index buffers.
- `Renderer::new()`: creates device, surface config, uniform buffer, bind group, shader module and pipeline.
- Inline WGSL shader: vertex `vs_main` receives `VertexIn { position: vec2<f32>, color: vec4<f32> }`, transforms by `camera * vec4(position, 0, 1)`, passes color to the fragment. Fragment `fs_main` returns the color.
- `set_mesh()`: uploads vertex/index buffers to the GPU via `create_buffer_init`.
- `render()` on Renderer is now delegated to vor-app (the renderer exposes `pub` fields: `camera_buf`, `heightmap_pipeline`, `vertex_buf`, `index_buf`, `index_count`, `surface`, `device`, `queue`, `format`).

**Camera API in shader**: `@group(0) @binding(0) var<uniform> camera : mat4x4<f32>`.

### Step 6 (reprise) — vor-app integration fix (Jul 26, post-checkpoint)

After the checkpoint, `vor-app/src/lib.rs` was fully rewritten with:

**Correct winit 0.30 structure**:
- `Window` is created inside `ApplicationHandler::resumed()` (winit 0.30 requires this; previously it was created before `run_app`).
- `Window` is wrapped in `Arc<Window>` for `Surface<'static>`.
- `Camera` and `Renderer` are stored in `State`, without duplicating `device`/`queue` (`renderer.device` and `renderer.queue` are used).
- egui 0.29 API: `Align2::LEFT_TOP` (not `Align::LEFT_TOP`), `on_window_event(&self.window, event)` (no `WindowId`).

**Wgpu 22 API fixes**:
- `Instance::new(InstanceDescriptor)` (not `&InstanceDescriptor`, no reference).
- `DeviceDescriptor` without the `trace` field (removed in wgpu 22).
- `adapter.request_device(&desc, None)` (trace_path is the second argument, not a descriptor field).

**Egui-wgpu 0.29.1 render**:
- `egui_wgpu::Renderer::render` takes `&mut RenderPass<'static>` — `pass.forget_lifetime()` is used (safe, the lifetime is a guard `PhantomData`).
- `Context::tessellate(shapes, pixels_per_point)` (2 args, not 1).
- `ScreenDescriptor::size_in_pixels` = physical pixels (not divided by scale_factor).

**Composite render in 2 passes**:
1. Heightmap with `ClearOp::Clear` (dark blue background `0.02, 0.02, 0.05`).
2. Egui with `LoadOp::Load` on the same surface texture.

**Result**: `cargo check --workspace` → 0 errors. 48 green tests. Clippy 0 warnings. `cargo fmt --all` clean.

### Step 7 — `vor-cli::main.rs`

`lib.rs` attempts to integrate winit + wgpu + egui-wgpu:

- `App` struct implements `winit::application::ApplicationHandler`.
- `State` struct holds window, device, queue, renderer, egui_ctx, egui_winit, egui_renderer, camera, mesh bounds, map_path, cursor, pan state.
- `init_state()` async: creates wgpu instance, adapter, device+queue, surface, Format, Renderer, egui_winit::State, egui_wgpu::Renderer. Uploads mesh to GPU. Frames the camera.
- `handle_window_event()`: CloseRequested, Resized, CursorMoved (pan active), MouseInput (toggle pan), MouseWheel (zoom), RedrawRequested.
- `redraw()` (BROKEN): incomplete merge of the two render passes:
  - The `surface_texture`, `view`, `encoder` definitions are **missing** (lost when refactoring the render block).
  - Pass 1 (heightmap) was removed along with the definitions — only pass 2 (egui) survives.
  - Loose references to `encoder` and `surface_texture` on lines 327-351.

**Compilation errors detected** (24 total):
1. `encoder`, `view`, `surface_texture` not defined.
2. `wgpu::Instance::new` called with `&InstanceDescriptor` — it does not take a reference in wgpu 22.
3. `egui_winit::State::on_window_event` expects `&Window`, not `WindowId`.
4. `Align::LEFT_TOP` removed in egui 0.29 (use `Align::LEFT`).
5. `Renderer` imported twice (`vor_render::Renderer` and `egui_wgpu::Renderer`).
6. `tracing_subscriber::EnvFilter` requires the `env-filter` feature.
7. `output.shapes` is `Vec<ClippedShape>`, `egui_wgpu::Renderer::render` expects `&[ClippedPrimitive]`. `ctx.tessellate()` is missing.
8. `egui::epaint` import not resolved.

### Step 7 — `vor-cli::main.rs`

Created the `vor` binary that delegates to `vor_app::run_cli()`.

---

## Critical findings and decisions

1. **wgpu 22 vs 23**: `egui-wgpu 0.29` depends on wgpu 22. wgpu 23 is not compatible. Cargo's automatic resolution picks wgpu 22 when `"22"` is specified as a workspace dep.
2. **`cell_rings` breaks model purity**: the `cell_rings` field in `VoronoiVertices` is redundant (derivable from the Delaunay) and is not persisted. It is a pragmatic compromise so the renderer can triangulate without recomputing the Delaunay at runtime. Alternative considered: recompute in `Renderer::set_mesh()` via `edgesAroundPoint` over the halfedge array of `vor_import::geometry`. Discarded because `vor-render` should not depend on `vor-import` (hard rule of plan §5).
3. **`lyon` 1.0 API break**: `PathBuilder::new()` no longer exists. Replacement: `Path::builder()` returns an implicit `PathBuilder` with `begin()`/`line_to()`/`end(closed)` methods.
4. **Delicate egui-wgpu integration**: `egui_wgpu::Renderer::render` takes `&mut RenderPass<'_>` and `&[ClippedPrimitive]` (NOT `Vec<ClippedShape>`). It must be tessellated first: `let clipped = output.shapes` + `ctx.tessellate(clipped)`.

---

## Test inventory

| File | Tests | What it validates |
|---|---|---|
| `crates/vor-render/src/camera.rs` | 4 | screen_to_world, zoom_at_cursor, pan, frame_bounds |
| `crates/vor-render/src/heightmap.rs` | 3 | navy blue ramp, bright summit, clamp >100 |

Not run yet (`cargo test` not executed). The existing Phase 1 tests (22 tests in vor-import) must remain green.

---

## Final session state

```
working tree: 15 files modified/created, 0 committed
cargo check --workspace: ✓ everything compiles
cargo test --workspace: ✓ 48 tests (27 unit + 4 bit-exact + 9 e2e + 7 render + 1 doc-ignored)
cargo clippy --all-targets: ✓ 0 warnings
cargo fmt --all: ✓ clean
```

### Phase 2 checklist (master plan §22)

- [x] Configure deps: workspace deps wgpu 22 / winit 0.30 / egui 0.29 / lyon 1.0 / bytemuck / pollster.
- [x] vor-render::Camera — 2D orthographic with pan/zoom, screen→world, frame_bounds (4 tests).
- [x] vor-render::HeightmapLayer — `build_mesh(grid)` triangulates cells with lyon, color ramp (3 tests).
- [x] vor-render::Renderer — wgpu pipeline + WGSL shaders + GPU buffers. `pub` fields.
- [x] vor-app::State — winit 0.30 (ApplicationHandler), wgpu 22, egui 0.29 integrated. Minimal egui overlay.
- [x] vor-cli — `vor` binary that loads a `.map` and opens the viewer.
- [x] Tests/sanity — 48 green tests, clippy 0 warnings, fmt clean.
- [ ] End-to-end test: `cargo run --bin vor -- /path/to/map.map` (pending execution).

**Actual progress**: 100% of Phase 2 code. It compiles, tests pass, clippy/fmt green. The only remaining item is running the binary against a real `.map` to validate the runtime.
