# Paridad de costas con Azgaar — proceso de debugging

> **Fecha**: sesión post-Fase 7 (render).
> **Fuente de referencia**: `azgaar/app/Fantasy-Map-Generator/src/renderers/coastline-fractal.ts` (y `landmass/landmass.ts`).
> **Relacionado**: `docs/analisis-completo-azgaar-landmass-lines-smoothing.md`, `docs/plan-puntos-1-5-landmass-lines.md`, `docs/landmass-drawing-analysis.md`.

Este documento registra el proceso de hacer que el render de costas de Voronia reproduzca **bit-exacto** la geometría de las costas de Azgaar (mismo seed = misma costa). No es una guía de usuario ni reemplaza el plan; es el registro de los bugs encontrados, por qué ocurrían y qué se hizo para corregirlos.

---

## Por qué la paridad exacta

El `.map`/JSON de Azgaar **no guarda la geometría** — solo atributos por celda. Al cargar un mapa, Azgaar recalcula la malla completa con su PRNG y su semilla. Si Voronia genera una malla distinta (aunque sea sutilmente distinta), los atributos del mapa importado quedan ubicados sobre celdas equivocadas: **datos incorrectos en silencio, sin errores visibles**.

La paridad se exige a dos niveles:

1. **Malla / grilla** (grid → pack → Delaunay/Voronoi): ya resuelto en Fases 1–6 (ver `docs/fase-{1..6}.md`).
2. **Render de costas**: el trazado fractal de la costa sobre el perímetro de cada feature de tierra. Esto es lo que cubre esta sesión.

## Pipeline de costas implementado

Azgaar, para cada feature de tierra, encadena exactamente esto:

```
simplify(points, 0.3)                 → simplify-js (radial distance + RDP)
→ clipPoly(points, W, H, secure=1)   → Sutherland-Hodgman
→ fractalize(points, "seed_c{i}")     → Alea PRNG, roughness profile + subdivisión
→ buildCoastlinePath(...)             → Catmull-Rom centrípeta + B-spline midpoint
→ (lyon tessellation)                 → relleno EvenOdd
```

El equivalente en `vor-render` quedó repartido así:

| Paso de Azgaar | Módulo de Voronia |
|---|---|
| `simplify` (simplify-js) | `vor-render/src/simplify.rs` |
| `clipPoly` (Sutherland-Hodgman, secure) | `vor-render/src/clip_poly.rs` |
| `fractalize` + `makeRoughnessProfile` + `subdivideEdge` | `vor-render/src/coastline.rs` |
| `buildCoastlinePath` (Catmull-Rom) | `vor-render/src/coastline_path.rs` |
| stroke/shadow de costa | `vor-render/src/coastline_stroke.rs` |
| isolines (get_isolines, connect_vertices, halos) | `vor-render/src/isoline.rs` |
| máscara de agua (water gap) | `vor-render/src/water_gap.rs` |
| texto GPU (glyphon) | `vor-render/src/text.rs` |
| PRNG `Alea@1.0.1` | `vor-render/src/prng/alea.rs` |

---

## PRNG: `Alea` (bit-exacto)

Azgaar usa el PRNG `alea` de Johannes Baagøe (npm `alea` 1.0.1) en casi todo lo generativo, con seeds string. El `.map` guarda los seeds en el header.

Se portó `Alea@1.0.1` a `vor-render/src/prng/alea.rs` con estos métodos:

- `Alea::new(seed: &str)` — estado interno `s0/s1/s2/c` inicializado con la función **mash** (algoritmo de David Bau, 32 bits).
- `next_f64() -> f64` — devuelve `(c() + s0()) * 2^-32` (un `f64` en `[0,1)`).
- `next_u32() -> u32`, `next_fract53() -> f64`.

**Verificación**: el port se testeó contra el fuente original `vor-import/tests/reference/alea-1.0.1.original.js` (bit-exacto). El port vive en `vor-render` (no en `vor-import`) porque `vor-render` no puede depender de `vor-import` por la regla de arquitectura — pero el fixture de referencia sigue en `vor-import/tests/reference/`.

---

## Bugs encontrados y corregidos

### Bug 1 — PRNG equivocado: `hash_f32` en vez de `Alea`

**Síntoma**: las costas eran "parecidas pero no iguales" a Azgaar. Mismo seed, misma forma general, pero la fractalización no coincidía punto a punto.

**Causa**: `coastline.rs` usaba un hash casero (`hash64`/`hash_f32` basado en multiplicación por constantes de SplitMix64) para generar números pseudoaleatorios. Azgaar usa `Alea(format!("{}_c{}", seed, feature_index))`.

**Fix**: se reemplazó el hash por `Alea::new(&seed_str)` y el perfil de rugosidad + la subdivisión de aristas consumen el **mismo** stream PRNG (`&mut dyn FnMut() -> f32`), como en el JS original donde el `rand` es una única referencia compartida.

### Bug 2 — Semilla equivocada: `map_id` en vez de `header.seed`

**Síntoma**: las costas no cambiaban al cambiar el seed de un mapa (o cambiaban de forma que no tenía relación con el mapa cargado).

**Causa**: en `crates/vor-app/src/lib.rs` el seed se derivaba de `loaded.world.header.map_id.wrapping_add(2654435761)`. Azgaar usa el **campo `seed` del header del `.map`** (string, ej. `"123456"`), que es la semilla con la que se generó la grilla — y por lo tanto la que permite que la costa calce con la geografía.

**Fix**:
```rust
// antes
seed: loaded.world.header.map_id.wrapping_add(2654435761),
// después
seed: loaded.world.header.seed.parse::<u64>().unwrap_or(0),
```

**Nota**: el campo `seed` del header es un string numérico; se parsea a `u64` y se serializa como parte del seed string `"{seed}_c{featureIndex}"` que consume `Alea`.

### Bug 3 — Índice del span siguiente en `buildCoastlinePath`

**Síntoma**: tramos "suaves" (sin fractalizar) que se dibujaban con Catmull-Rom rota — curvas que se salían de la costa, picos puntiagudos donde no debía haberlos.

**Causa**: en `coastline_path.rs`, dentro del bucle de Catmull-Rom centrípeta:

```rust
// ANTES (bug) — apuntaba al INICIO del siguiente span
let ni = spans[(i + 1) % m].end_idx;

// DESPUÉS (correcto) — apunta al FIN del span actual
let ni = spans[i].end_idx;
```

`spans[i].end_idx` ya es el índice del siguiente punto "original" del feature (el vértice sin fractalizar), que es exactamente el punto donde termina la interpolación de este tramo. Usar el span siguiente mezclaba dos tramos contiguos y rompía la continuidad de la curva.

**Efecto secundario del fix**: también arregló el cálculo del **midpoint B-spline** de los tramos suaves, que dependía del mismo índice.

### Bug 4 — `roughness_contrast` hardcodeado

**Síntoma**: el perfil de rugosidad no respetaba el parámetro `roughness_contrast` del header.

**Causa**: `make_roughness_profile` normalizaba con `.powf(1.5)` fijo, ignorando el `roughnessContrast` que Azgaar lee del header (default `1.5`).

**Fix**: el perfil ahora recibe el contraste como parámetro y aplica `powf(contrast)`.

### Bug 5 — Stream PRNG compartido (no descubierto en esta sesión, sí relevante)

El perfil de rugosidad y la subdivisión de aristas comparten el **mismo** `Alea`. En el port inicial se creaban dos instancias separadas; eso también rompía la paridad (los desplazamientos dependen de la secuencia completa desde el inicio). Ahora hay una sola instancia y una sola closure `rand`.

---

## Parámetros por defecto alineados con Azgaar

`FractalSettings::default()` ahora refleja los defaults del renderer de Azgaar:

| Parámetro | Valor | Fuente en Azgaar |
|---|---|---|
| `amplitude_decay` | `0.9` | slider default |
| `min_edge` | `1.0` | constante |
| `base_amplitude` | `1.5` | constante |
| `max_depth` | `4` | constante |
| `smooth_threshold` | `0.25` | constante |
| `roughness_contrast` | `1.5` | header `roughnessContrast` |
| `profile_harmonics` | `4` | constante |
| `lake_smooth_thresh_mult` | `2.0` | constante |
| `simplify_tolerance` | `0.3` | `simplify(pts, 0.3)` |
| `clip_secure` | `true` | `clipPoly(..., 1)` |

Detalles del algoritmo portado bit-exacto:

- **Desplazamiento de arista**: `(rand() - 0.5) * sqrt(len) * amplitude * roughness` sobre la normal `(-dy/len, dx/len)` del punto medio.
- **Roughness profile**: suma de `num_harmonics` cosenos, cada armónico con `amp = rand()` y `phase = rand() * 2π`, normalizado a `[0,1]` y elevado a `roughness_contrast`. Tamaño fijo 256. Se muestrea con interpolación lineal con `t.rem_euclid(1.0)` (perfil cerrado).
- **`mid_t`**: promedio circular de `t0/t1` (maneja el wrap del cierre del polígono).
- **Smooth threshold** (criterio de parada): si `roughness(t_mid) < smooth_threshold`, no se subdivide (tramo queda suave). Para lagos, el umbral se multiplica por `lake_smooth_thresh_mult` (2.0).
- **Aristas sobre el borde del mapa**: si ambos extremos están en el borde (`x<=0 || x>=W || y<=0 || y>=H`), no se fractalizan (se evita la fractalización del borde del mapa).
- **Spans**: un `CoastlineSpan` por arista original; `is_smooth` es `true` si la arista no produjo puntos nuevos (`num_points == 2`).

---

## `buildCoastlinePath` — Catmull-Rom + B-spline (punto a punto)

El path builder reproduce `buildCoastlinePath` del JS:

- Si el span es **suave**: el path recorre la arista con un **quadratic Bézier** del punto medio al punto medio (B-spline midpoint, `(a+b)/2`), o un `LineTo` al punto medio cuando el span anterior fue jagged.
- Si el span es **jagged** (fractalizado): cada par de puntos consecutivos se une con **cubic Bézier** usando los vecinos anterior y siguiente con el factor `1/8` de Catmull-Rom:
  ```
  cp1 = a + (b - prev) / 8
  cp2 = b - (nnext - a) / 8
  ```
- El punto de inicio es el midpoint del último tramo si es suave, o `p0` si el último tramo es jagged (`at_mid`).
- `coastline_path_to_lyon` convierte los `PathCommand` a un `lyon::Path` para teselar con `FillOptions::default().with_fill_rule(EvenOdd)`.

**Tests** (`coastline_path.rs`): `smooth_span_produces_quad_bezier`, `jagged_span_produces_cubic_bezier`, `start_point_jagged_last_span` — cubren los dos casos de emisión de comandos y el punto de partida.

---

## Módulos nuevos de esta tanda

### `simplify.rs` — Ramer-Douglas-Peucker + radial distance
Port de `simplify-js` (Vladimir Agafonkin): primera pasada de distancia radial con `sq_tolerance`, segunda de RDP recursivo. Tolerancia usada: `0.3`. Export público: `simplify`.

### `clip_poly.rs` — Sutherland-Hodgman con "secure"
Port de `clipPoly` de Azgaar: recorta el polígono al rectángulo del mapa. Con `secure=true` (1) no se degenera en rectángulos/segmentos en el borde — evita artefactos de teselación cuando el feature toca el borde. Export público: `clip_polygon`.

### `coastline_stroke.rs` — stroke y sombra de costa
Genera el contorno (stroke) y la sombra de las costas (línea más gruesa y oscura bajo el stroke), para el look de Azgaar. Exports: `build_coastline_stroke_mesh`, `build_coastline_shadow_mesh`, `CoastlineStrokeSettings`.

### `isoline.rs` — motor de isolines (connect_vertices)
Port del motor de isolines de Azgaar (`connectVertices`, `getIsolines`, paths de borde/halo). Se usa para isoheight, isotherm, isobar, etc. Exports: `connect_vertices`, `get_isolines`, `get_border_path`, `get_fill_path`, `get_halo_path`, `get_water_gap_path`, `IsolineOptions`, `IsolineOutput`.

### `water_gap.rs` — máscara de agua
Para que los colores de capas humanas (estados, provincias, culturas, religiones, biomas) no sangren al océano: genera un gap de agua sobre las celdas de agua (`h < 20` o lago), pintadas con el color de fondo del océano. `append_water_gap` muta un mesh existente agregando vértices/triángulos; `build_water_gap_mesh` lo crea desde cero.

### `text.rs` — TextSystem (glyphon)
Sistema de texto GPU con `glyphon 0.6`: `FontSystem` + `SwashCache` + `TextAtlas` + `Viewport`. Dos renderers (uno MSAA para el pass del mapa, uno no-MSAA para debug). `prepare()` sube glifos fuera del render pass; `render()` dibuja dentro de cualquier pass; `render_debug_no_msaa()` sobre la resolved surface. Ver sección "TextSystem" en `references/status.md`.

---

## Integración en vor-app

- `build_fractal_landmass_mesh` se llama con `FractalSettings { seed: header.seed.parse::<u64>().unwrap_or(0), ..Default::default() }` (Bug 2).
- Las capas de human geography (states, provinces, cultures, religions) y biomes ahora llevan `append_water_gap`, con el color de agua resuelto por capa (color del catálogo para biomes, `hex_color_to_linear` para las demás).
- `TextSystem` se inicializa en `init_state`, se redimensiona en resize, se usa dentro del MSAA pass y se hace `trim()` al final del frame.

## Cómo se verifica

1. **Determinismo**: mismo seed + mismos parámetros = mismo mesh, siempre (tests con seed fija).
2. **Bit-exactitud del PRNG**: tests contra el fuente JS de referencia (`alea-1.0.1.original.js`).
3. **Visual**: comparar el render de Voronia contra el mapa original de Azgaar con el mismo seed — costas y atributos deben calzar.
4. **Tests**: `cargo test --workspace` verde (99 tests, 1 ignored); `cargo test --package vor-render` (21 tests).

## Checklist de la sesión

- [x] Port de `Alea@1.0.1` a `vor-render/src/prng/alea.rs` + fixture de referencia
- [x] Reemplazo del PRNG hash en `coastline.rs` por `Alea("seed_c{featureIndex}")`
- [x] Stream PRNG compartido entre perfil y subdivisión
- [x] `roughness_contrast` parametrizado (Bug 4)
- [x] `simplify` + `clipPoly` en el pipeline (`simplify.rs`, `clip_poly.rs`)
- [x] Fix `ni = spans[i].end_idx` en `buildCoastlinePath` (Bug 3)
- [x] `FractalSettings` defaults alineados con Azgaar
- [x] Fix semilla: `header.seed` en vez de `map_id` (Bug 2)
- [x] `buildCoastlinePath` + `coastline_path_to_lyon` + 3 tests
- [x] Stroke/shadow de costa (`coastline_stroke.rs`)
- [x] Motor de isolines (`isoline.rs`)
- [x] Water gap en capas humanas + biomes (`water_gap.rs`)
- [x] TextSystem glyphon (`text.rs`)
- [x] Tests verdes, clippy sin errores nuevos, fmt limpio
