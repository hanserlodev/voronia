# Phase 3 — Complete rendering layers

> Chronological session log. Format: `docs/fase-0-investigacion.md`.
> Last updated: July 27, 2026 — Phase 3 COMPLETED (all fixes applied).

---

## Azgaar reference

- **Azgaar version**: v1.138.0 (recorded in Phase 1).
- **Cloned local commit**: `51d8e3e` (azgaar-fmg master, Jul 21, 2026).
- **Test map**: `Sorvik 2026-07-24-23-39.map` (~7.3K pack cells, 141 rivers, 14 states, 16 cultures, 226 provinces, 1010 burgs).

---

## Session chronology (July 27, 2026)

### Starting point

Phase 2 completed. Working tree with 15 modified files from Phase 2, none committed. Work starts from the post-Phase 2 state.

### Step 1 — `vor-core` extension for render data

Added two `#[serde(skip)]` fields:

**`PackCells::adjacency: Vec<Vec<u32>>`** (`crates/vor-core/src/cells.rs`): IDs of adjacent pack cells (interior neighbors, no boundary). Populated by `vor-import::regraph` from the second `calculate_voronoi` in the pack. It is the equivalent of Azgaar's `cells.c[p]`.

**`River::cell_path: Vec<u32>`** (`crates/vor-core/src/entities/river.rs`): The path of pack cells the river traverses, from `source_cell` to `mouth_cell`, following downhill flow. Populated by `vor-import::loader::trace_river_paths()`.

### Step 2 — Data population in `vor-import`

**`regraph.rs`**: Added `voronoi.cells.c.clone()` as the source of `adjacency` in the post-repack `PackCells`.

**`loader.rs`**: Added `trace_river_paths()` which, for each river with id>0, traverses from `source_cell` following neighbors with the same `river_id` and decreasing height down to `mouth_cell`. Greedy algorithm: at each step it picks the neighbor with the lowest height.

### Step 3 — `build_pack_mesh` helper

File: `crates/vor-render/src/mesh.rs`.

Generic function that, given a `VoronoiVertices`, `points_n` and a `color_fn(usize) -> [f32;4]` closure, produces a `HeightmapMesh` by triangulating Voronoi polygons with lyon. It reuses the same `ColorCtor` pattern from `heightmap.rs`.

### Step 4 — Render layers

**`biome.rs`** (`crates/vor-render/src/biome.rs`): `build_biome_mesh(pack, biome_colors)` colors each pack cell according to `pack.cells.biome[p]` → catalog color. Hex `#rrggbb` → linear `[f32;4]` conversion (approximate gamma 2.2).

**`river.rs`** (`crates/vor-render/src/river.rs`): `build_river_mesh(points, rivers)` draws each segment of each river's `cell_path` as a textured quad (two triangles) with thickness proportional to `discharge_m3s` (1-6 px). Semi-transparent blue color `[0.2, 0.4, 0.8, 0.85]`.

**`border.rs`** (`crates/vor-render/src/border.rs`): `build_border_mesh(pack, BorderKind)` iterates `adjacency` edges and draws a segment (quad) between the centers of cells with a different id. Colors: state red `[0.9,0.1,0.1]`, province yellow `[0.7,0.7,0.1]`, culture orange `[1.0,0.65,0.0]`.

**`burg.rs`** (`crates/vor-render/src/burg.rs`): `build_burg_mesh(pack)` draws a 4px equilateral triangle at `pack.points[burg.cell]` for each burg. Red color `[0.9,0.2,0.1]`.

**`layers.rs`** (`crates/vor-render/src/layers.rs`): `LayerFlags` with 8 flags (heightmap, biomes, rivers, borders: state/province/culture, burgs, labels). `active_indices()` returns which layers to draw. `NUM_LAYERS = 7` (layer 0 = heightmap, layers 1-6 = extras).

### Step 5 — Multi-layer in Renderer

Added `LayerBuffer` (vertex/index buffer + count) and `layers: Vec<LayerBuffer>` to the `Renderer` struct.

- `add_layer_mesh(mesh) -> usize`: creates GPU buffers and returns the index (1-based).
- `draw_layer(pass, index)`: draws any layer by index (0 = legacy heightmap, 1+ = extras).

### Step 6 — Integration in vor-app

**`init_state`**: builds all meshes (biomes, rivers, borders x3, burgs), registers them in the renderer via `add_layer_mesh`, stores `world: World`, `layer_flags: LayerFlags`, `picked_cell: Option<usize>` in `State`.

**`redraw()`**:
- Layer order: background (clear) → heightmap → biomes → rivers → borders state → borders province → borders culture → burgs → labels (egui overlay) → egui UI.
- `active_indices()` filters by flags.
- Labels: draws burg names in the egui `layer_painter`, projecting world→screen coordinates.
- Right panel: checkboxes for each layer.
- Left panel: FPS, cursor, info of the selected cell.

**Picking**: right click → `screen_to_world` → `pick_cell()` (O(n) over pack points, 20px threshold).

### Critical finding — egui texture upload (Jul 26, 2026)

The egui GUI (side panel, FPS, labels) was not visible. Root cause: egui writes to `output.textures_delta.set` to register that the font atlas has changed (first time and every time a new character is added), but the code did not call `egui_renderer.update_texture()`. Without it, `egui_wgpu::Renderer::render()` checks `self.textures.contains_key(id)`, fails, and skips all draw calls — even solid rectangles.

**Fix**: iterate `output.textures_delta.set` and call `update_texture()` for each `(id, delta)` before `update_buffers()`.

### Critical finding — back-face cull_mode in the 2D viewer (Jul 27, 2026)

Rivers, borders (state/province/culture) and burgs were not rendered even though the meshes were built correctly with valid vertices and indices. Root cause: the 2D orthographic projection in `camera.rs:77-78` explicitly inverts Y (`bottom = cy + ey/2 > top = cy - ey/2` → `orthographic_rh` produces a negative `rcp_height = 1/(top - bottom)`). The pipeline used `cull_mode: Some(wgpu::Face::Back)` with `front_face: Ccw`. This worked for the heightmap because lyon tessellates Voronoi polygons clockwise (CW) — they survive the Y-flip and appear CCW in clip space. But the river/border/burg quads are built CCW and after the Y-flip end up CW in clip space → culled → invisible.

**Fix**: `cull_mode: None` — a 2D viewer with a flat map never needs back-face culling (there is no occluded geometry).

### Step 7 — Runtime validation

```
$ cargo run --bin vor -- "Sorvik 2026-07-24-23-39.map"
INFO  vor_app > cargando mapa: Sorvik...
INFO  vor_app > available wgpu adapters: ["Vulkan Intel", "Vulkan NVIDIA RTX 4060", "Gl Mesa Intel"]
INFO  vor_app > heightmap mesh: 58010 vertices, 114030 indices (bounds [-9.0, -9.0] -> [946.0, 950.0])
INFO  vor_app > meshes: biomes=42179v/82926i, rivers=808v/1212i, borders(s/p/c)=(25632/44952/31856), burgs=3027v/3027i
```

The viewer opens correctly with all layers rendered, including rivers (blue, thickness varying by discharge), borders (red state, yellow province, orange culture) and burgs (red triangles). 47 green tests, clippy/fmt clean.

---

## Files touched

```
M crates/vor-core/src/cells.rs                (+adjacency field)
M crates/vor-core/src/entities/river.rs       (+cell_path field)
M crates/vor-import/src/regraph.rs            (+adjacency from voronoi.cells.c)
M crates/vor-import/src/mapfile/loader.rs     (+trace_river_paths)
M crates/vor-import/src/mapfile/cells.rs      (+adjacency: Vec::new())
M crates/vor-import/src/mapfile/catalogs.rs   (+cell_path: Vec::new())
M crates/vor-render/src/camera.rs             (+world_to_screen)
M crates/vor-render/src/heightmap.rs          (+pub(crate) ColorCtor)
A crates/vor-render/src/mesh.rs               (helper build_pack_mesh)
A crates/vor-render/src/biome.rs              (biome layer)
A crates/vor-render/src/river.rs              (rivers layer)
A crates/vor-render/src/border.rs             (borders layer)
A crates/vor-render/src/burg.rs               (burgs layer)
A crates/vor-render/src/layers.rs             (LayerFlags)
M crates/vor-render/src/renderer.rs           (+layers vec, add_layer_mesh, draw_layer)
M crates/vor-render/src/lib.rs                (+export new modules)
M crates/vor-app/src/lib.rs                   (+world, layers, picking en State/redraw)
- - - (session 2: post-Phase 2 fixes) - - -
M crates/vor-render/src/renderer.rs           (cull_mode: None fix)
M .opencode/skills/voronia-dev/references/status.md (fix docs)
A docs/fase-3.md                              (this file, update with findings)
```

---

## Final state

```
cargo test --workspace:  ✓ 47 tests
cargo clippy --all-targets: ✓ 0 errors, 2 warnings (pre-existing dead_code)
cargo fmt --all:           ✓ clean
cargo run --bin vor:       ✓ viewer opens with ALL layers (includes rivers, borders, burgs)
```

### Phase 3 checklist (master plan §23)

- [x] Rivers, state/province/culture borders, biomes, burgs, basic labels.
- [x] Layer toggle system.
- [x] Picking (click → cell/entity info).
