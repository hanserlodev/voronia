**Última actualización**: 30 julio 2026 — Render de costas con paridad bit-exacta contra Azgaar

## Fase actual

**Fase 7 — Motor de generación procedural nativo**: ⏳ **EN PROGRESO** (30 jul 2026). Hidrología (ríos) completa + render de costas con paridad exacta.

## Port de ríos — Estado final

### vor-core
- **River model**: `width_factor`, `source_width_km`, `type_name`, `meandered_points` añadidos
- **PackCells**: `feature_id: Vec<u16>` añadido
- **Feature**: `shoreline`, `lake_height`, `inlets`, `outlet_river`, `entering_flux`, `closed`, `out_cell` — todos los campos de lago necesarios para hidrología

### vor-import
- **RiverRaw**: parsea `widthFactor`, `sourceWidth`, `type`, `cells` (con `-1`→`u32::MAX`), `points`
- **FeatureRaw**: `shoreline` y `height` mapeados a Feature
- **PackCells feature_id**: poblado desde grid cells via `grid_id` mapping

### vor-sim (motor de simulación procedural)

| Módulo | Azgaar | Voronia |
|--------|--------|---------|
| hydrology | `alterHeights()` | ✅ |
| | `resolveDepressions()` | ✅ (Priority-Flood) |
| | `Lakes.defineClimateData()` | ✅ (Penman evaporation) |
| | `Lakes.detectCloseLakes()` | ✅ (BFS desde shoreline) |
| | `drainWater()` + lake outlets | ✅ |
| | `flowDown()` + confluencias | ✅ |
| width | `getOffset()`, `getSourceWidth()`, `getWidth()` | ✅ fórmulas exactas |
| meander | `meander()`, `relaxAcuteAngles()`, `addMeandering()` | ✅ |
| river_def | `defineRivers()`, `calculateConfluenceFlux()` | ✅ |
| | `downcutRivers()` | ✅ |
| specify | `specify()`, `getParent()`, `getBasin()`, `getName()`, `getType()` | ✅ simplificado |
| | `remove()`, `getNextId()`, `getApproximateLength()` | ✅ |
| resolve | `resolveLakeDrainFeature()`, `resolveDrainFeature()` | ✅ |
| | `isNavigable()` | ✅ |

### Constantes de Azgaar replicadas
`MIN_FLUX_TO_FORM_RIVER=30`, `MIN_NAVIGABLE_FLUX=100`, `FLUX_FACTOR=500`, `MAX_FLUX_WIDTH=1`, `LENGTH_FACTOR=200`, `MAX_DOWNCUT=5`, `WATER_MEANDER_SCALE=0.25`

### Tests
**79 tests total** (67 existentes + 12 nuevos de vor-sim).
- width: fórmulas getOffset/getSourceWidth/getWidth
- meander: 2 puntos, 1 punto sin cambios
- specify: getApproximateLength, getNextId
- rn: redondeo estilo JS Math.round
- Todos los tests de importación (Sorvik) verdes

## TextSystem (glyphon) — Funcional pero sin usar

Se integró glyphon 0.6 como sistema de renderizado de texto GPU en `vor-render::TextSystem`.

### Qué funciona
- `TextSystem` con `FontSystem` (907 font faces), `SwashCache`, `TextAtlas`, `Viewport`
- Dos `TextRenderer`: uno MSAA (sample_count=4, para el pass del mapa) y otro no-MSAA (para debug)
- `prepare()`: sube glifos a GPU fuera del render pass (queue.write_texture + queue.write_buffer)
- `render()`: dibuja dentro de cualquier render pass
- `render_debug_no_msaa()`: dibuja directamente sobre la resolved surface (útil para debug)
- Atlas comparte textura entre ambos renderers

### Cómo usar (para la comunidad)
```rust
// Antes del render pass:
ts.prepare(device, queue, "Texto", x, y, font_size, [r, g, b, a]);

// Dentro del render pass MSAA:
ts.render(&mut pass);

// Opcional: pass debug sin MSAA:
ts.render_debug_no_msaa(&mut encoder, &resolve_view);

// Al final del frame:
ts.trim();
```

### Bugs resueltos
- El test label se ponia en (50,50) → quedaba oculto bajo el panel egui de 240px.
- El debug pass no-MSAA era no-op porque `renderer_no_msaa` nunca se preparaba.
- Ahora `prepare()` prepara ambos renderers.

### Limitaciones conocidas
- Solo un `TextArea` por frame (el `prepare` resetea el buffer cada vez). Para múltiples labels, hay que llamar `prepare` varias veces o usar un solo buffer con múltiples líneas.
- glyphon renderiza en coordenadas de pantalla, no de mundo. Para world-space labels, el llamante debe proyectar a screen coordinates.

## Costas con paridad bit-exacta — COMPLETO (30 jul 2026)

Pipeline de costas idéntico al de Azgaar: `simplify(0.3)` → `clipPoly(secure=1)` → `fractalize` → `buildCoastlinePath` → lyon. Documentación completa del proceso (bugs, fixes, algoritmos) en `docs/coastline-paridad-azgaar.md`.

### Decisiones clave
- **PRNG**: port de `Alea@1.0.1` a `vor-render/src/prng/alea.rs` (bit-exacto, verificado contra el fixture JS en `vor-import/tests/reference/alea-1.0.1.original.js`). Vive en vor-render porque vor-render no puede depender de vor-import.
- **Semilla fractal**: el campo `seed` del header del `.map` (string, ej. `"123456"`), NO `map_id`. Azgaar usa `Alea("{seed}_c{featureIndex}")`.
- **Stream PRNG compartido**: el perfil de rugosidad y la subdivisión de aristas consumen el mismo `Alea` (una sola closure `rand`), como el JS.

### Bugs corregidos
1. PRNG casero (`hash_f32`) → `Alea` (las costas no calzaban punto a punto).
2. Semilla `map_id.wrapping_add(2654435761)` → `header.seed.parse::<u64>()` (las costas no cambiaban con el seed del mapa).
3. `ni = spans[(i+1)%m].end_idx` → `ni = spans[i].end_idx` en `build_coastline_path` (rompía la Catmull-Rom y el midpoint B-spline → curvas que se salían de la costa).
4. `roughness_contrast` hardcodeado en `1.5` → parametrizado (`powf(contrast)`).
5. Dos instancias `Alea` separadas (perfil + subdivisión) → una sola compartida.

### Defaults alineados con Azgaar
`amplitude_decay=0.9`, `min_edge=1.0`, `base_amplitude=1.5`, `max_depth=4`, `smooth_threshold=0.25`, `roughness_contrast=1.5`, `profile_harmonics=4`, `lake_smooth_thresh_mult=2.0`, `simplify_tolerance=0.3`, `clip_secure=true`.

### Módulos nuevos en vor-render
- `simplify.rs` (simplify-js: radial distance + RDP)
- `clip_poly.rs` (Sutherland-Hodgman con secure)
- `coastline_path.rs` (buildCoastlinePath: Catmull-Rom `1/8` + B-spline midpoint + 3 tests)
- `coastline_stroke.rs` (stroke + sombra de costa)
- `isoline.rs` (motor de isolines: connect_vertices, get_isolines, halos)
- `water_gap.rs` (máscara de agua para que capas humanas no sangren al océano)
- `text.rs` (TextSystem glyphon — ver sección abajo)
- `prng/alea.rs` (Alea@1.0.1 bit-exacto)

### Integración en vor-app
- `FractalSettings { seed: header.seed.parse::<u64>().unwrap_or(0) }`
- `append_water_gap` en biomes + states + provinces + cultures + religions (color de agua por capa)
- `TextSystem` init/resize/render/trim integrado en el frame loop

## Pendiente (post-Fase 7)
- Integrar vor-sim::generate() con vor-app (generación nativa vs solo import)
- Unificar meander (vor-render tiene copia, vor-sim tiene original)
- Tests end-to-end de generate() con un mundo real
- Optimizar: todo el pipeline es O(n²) en el peor caso (resolveDepressions)
- Documentación de las sesiones de costas previas sin commitear (landmass-drawing-analysis, plan-puntos-1-5, plan-renderizado-completo, analisis-completo) ya están en git
- Verificar visualmente la paridad de costas contra un mapa real de Azgaar (falta screenshot de comparación)
