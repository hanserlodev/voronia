# Fase 3 — Capas completas de renderizado

> Registro cronológico de la sesión. Formato: `docs/fase-0-investigacion.md`.
> Última actualización: 27 julio 2026 — Fase 3 COMPLETADA (todos los fixes aplicados).

---

## Referencia de Azgaar

- **Versión de Azgaar**: v1.138.0 (registrada en Fase 1).
- **Commit clonado local**: `51d8e3e` (azgaar-fmg master, 21 jul 2026).
- **Mapa de prueba**: `Sorvik 2026-07-24-23-39.map` (~7.3K pack cells, 141 ríos, 14 estados, 16 culturas, 226 provincias, 1010 burgos).

---

## Cronología de la sesión (27 julio 2026)

### Punto de partida

Fase 2 completada. Working tree con 15 archivos modificados de Fase 2, todos sin commitar. Se arranca desde el estado post-Fase 2.

### Paso 1 — Extensión de `vor-core` para datos de render

Se agregaron dos campos `#[serde(skip)]`:

**`PackCells::adjacency: Vec<Vec<u32>>`** (`crates/vor-core/src/cells.rs`): IDs de celdas pack adyacentes (vecinos interiores, sin boundary). Poblado por `vor-import::regraph` desde el segundo `calculate_voronoi` en el pack. Es el equivalente a `cells.c[p]` de Azgaar.

**`River::cell_path: Vec<u32>`** (`crates/vor-core/src/entities/river.rs`): Camino de celdas pack que recorre el río, desde `source_cell` hasta `mouth_cell`, siguiendo flujo downhill. Poblado por `vor-import::loader::trace_river_paths()`.

### Paso 2 — Poblado de datos en `vor-import`

**`regraph.rs`**: Se agregó `voronoi.cells.c.clone()` como fuente de `adjacency` en el `PackCells` post-repack.

**`loader.rs`**: Se agregó `trace_river_paths()` que para cada río con id>0 recorre desde `source_cell` siguiendo vecinos con mismo `river_id` y altura decreciente hasta `mouth_cell`. Algoritmo greedy: en cada paso elige el vecino con menor altura.

### Paso 3 — Helper `build_pack_mesh`

Archivo: `crates/vor-render/src/mesh.rs`.

Función genérica que dado un `VoronoiVertices`, `points_n` y un closure `color_fn(usize) -> [f32;4]`, produce un `HeightmapMesh` triangulando polígonos de Voronoi con lyon. Reutiliza el mismo patrón `ColorCtor` de `heightmap.rs`.

### Paso 4 — Capas de render

**`biome.rs`** (`crates/vor-render/src/biome.rs`): `build_biome_mesh(pack, biome_colors)` colorea cada celda pack según `pack.cells.biome[p]` → color del catálogo. Conversión hex `#rrggbb` → `[f32;4]` lineal (gamma 2.2 aproximado).

**`river.rs`** (`crates/vor-render/src/river.rs`): `build_river_mesh(points, rivers)` dibuja cada segmento del `cell_path` de cada río como un quad texturizado (dos triángulos) con grosor proporcional a `discharge_m3s` (1-6 px). Color azul semitransparente `[0.2, 0.4, 0.8, 0.85]`.

**`border.rs`** (`crates/vor-render/src/border.rs`): `build_border_mesh(pack, BorderKind)` itera aristas de `adjacency` y dibuja un segmento (quad) entre centros de celdas con distinto id. Colores: estado rojo `[0.9,0.1,0.1]`, provincia amarillo `[0.7,0.7,0.1]`, cultura naranja `[1.0,0.65,0.0]`.

**`burg.rs`** (`crates/vor-render/src/burg.rs`): `build_burg_mesh(pack)` dibuja un triángulo equilátero de 4px en `pack.points[burg.cell]` por cada burgo. Color rojo `[0.9,0.2,0.1]`.

**`layers.rs`** (`crates/vor-render/src/layers.rs`): `LayerFlags` con 8 flags (heightmap, biomes, rivers, borders: state/province/culture, burgs, labels). `active_indices()` retorna qué capas dibujar. `NUM_LAYERS = 7` (layer 0 = heightmap, layers 1-6 = extras).

### Paso 5 — Multi-layer en Renderer

Se agregó `LayerBuffer` (vertex/index buffer + count) y `layers: Vec<LayerBuffer>` al struct `Renderer`. 

- `add_layer_mesh(mesh) -> usize`: crea buffers GPU y retorna índice (1-based).
- `draw_layer(pass, index)`: dibuja cualquier capa por índice (0 = heightmap legacy, 1+ = extras).

### Paso 6 — Integración en vor-app

**`init_state`**: construye todas las mallas (biomes, rivers, borders x3, burgs), las registra en el renderer vía `add_layer_mesh`, almacena `world: World`, `layer_flags: LayerFlags`, `picked_cell: Option<usize>` en `State`.

**`redraw()`**: 
- Orden de capas: background (clear) → heightmap → biomes → rivers → borders state → borders province → borders culture → burgs → labels (egui overlay) → egui UI.
- `active_indices()` filtra según flags.
- Labels: dibuja nombres de burgo en egui `layer_painter`, proyectando coordenadas mundo→pantalla.
- Panel derecho: checkboxes para cada capa.
- Panel izquierdo: FPS, cursor, info de celda seleccionada.

**Picking**: click derecho → `screen_to_world` → `pick_cell()` (O(n) sobre puntos pack, threshold 20px).

### Hallazgo crítico — egui texture upload (26 jul 2026)

La GUI egui (panel lateral, FPS, labels) no se veía. Causa raíz: egui escribe `output.textures_delta.set` para registrar que el font atlas ha cambiado (primera vez y cada vez que se añade un carácter nuevo), pero el código no llamaba a `egui_renderer.update_texture()`. Sin eso, `egui_wgpu::Renderer::render()` comprueba `self.textures.contains_key(id)`, falla y salta todos los draw calls — incluso rectángulos sólidos.

**Fix**: iterar `output.textures_delta.set` y llamar `update_texture()` para cada `(id, delta)` antes de `update_buffers()`.

### Hallazgo crítico — cull_mode back-face en viewer 2D (27 jul 2026)

Ríos, fronteras (estado/provincia/cultura) y burgos no se renderizaban aunque los meshes se construían correctamente con vértices e índices válidos. Causa raíz: la proyección ortográfica 2D en `camera.rs:77-78` invierte Y explícitamente (`bottom = cy + ey/2 > top = cy - ey/2` → `orthographic_rh` produce `rcp_height = 1/(top - bottom)` negativo). El pipeline usaba `cull_mode: Some(wgpu::Face::Back)` con `front_face: Ccw`. Esto funcionaba para el heightmap porque lyon tesela polígonos de Voronoi en sentido horario (CW) — que sobreviven al Y-flip y aparecen CCW en clip. Pero los quads de ríos/fronteras/burgos se construyen en CCW y tras el Y-flip quedan CW en clip → cullingados → invisibles.

**Fix**: `cull_mode: None` — un viewer 2D con mapa plano nunca necesita back-face culling (no hay geometría ocluida).

### Paso 7 — Validación runtime

```
$ cargo run --bin vor -- "Sorvik 2026-07-24-23-39.map"
INFO  vor_app > cargando mapa: Sorvik...
INFO  vor_app > available wgpu adapters: ["Vulkan Intel", "Vulkan NVIDIA RTX 4060", "Gl Mesa Intel"]
INFO  vor_app > heightmap mesh: 58010 vertices, 114030 indices (bounds [-9.0, -9.0] -> [946.0, 950.0])
INFO  vor_app > meshes: biomes=42179v/82926i, rivers=808v/1212i, borders(s/p/c)=(25632/44952/31856), burgs=3027v/3027i
```

El visor abre correctamente con todas las capas renderizadas, incluyendo ríos (azul, grosor variable por caudal), fronteras (rojo estado, amarillo provincia, naranja cultura) y burgos (triángulos rojos). 47 tests verdes, clippy/fmt limpios.

---

## Archivos tocados

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
A crates/vor-render/src/biome.rs              (capa biomas)
A crates/vor-render/src/river.rs              (capa ríos)
A crates/vor-render/src/border.rs             (capa fronteras)
A crates/vor-render/src/burg.rs               (capa burgos)
A crates/vor-render/src/layers.rs             (LayerFlags)
M crates/vor-render/src/renderer.rs           (+layers vec, add_layer_mesh, draw_layer)
M crates/vor-render/src/lib.rs                (+export new modules)
M crates/vor-app/src/lib.rs                   (+world, layers, picking en State/redraw)
- - - (sesión 2: fixes post-Fase 2) - - -
M crates/vor-render/src/renderer.rs           (cull_mode: None fix)
M .opencode/skills/voronia-dev/references/status.md (fix docs)
A docs/fase-3.md                              (this file, update with hallazgos)
```

---

## Estado final

```
cargo test --workspace:  ✓ 47 tests
cargo clippy --all-targets: ✓ 0 errors, 2 warnings (dead_code pre-existente)
cargo fmt --all:           ✓ limpio
cargo run --bin vor:       ✓ visor abre con TODAS las capas (incluye ríos, fronteras, burgos)
```

### Checklist Fase 3 (plan maestro §23)

- [x] Ríos, fronteras de estados/provincias/culturas, biomas, burgos, labels básicos.
- [x] Sistema de toggles de capas.
- [x] Picking (click → info de celda/entidad).
