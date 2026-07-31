# Plan de implementación: puntos 1–5 — masa de tierra, líneas y suavizado

> **Basado en**: `docs/analisis-completo-azgaar-landmass-lines-smoothing.md`
> **Alcance**: Pipeline feature → SVG (simplify, clipPoly, fractalize) → Path builder híbrido → Coastline stroke → Isoline engine (connectVertices)

---

## Punto 1: Pipeline feature → SVG path completo

### Estado actual
- `coastline.rs:build_fractal_landmass_mesh()` va directo de `feature.perimeter_vertices` → `fractalize_polygon` → `catmull_rom_closed` → lyon tessellation
- **Falta**: `simplify` (Ramer-Douglas-Peucker), `clipPoly` con `secure=1`

### Plan

#### 1a. `crates/vor-render/src/simplify.rs` — Ramer-Douglas-Peucker + radial distance

Port de `simplify-js` de Vladimir Agafonkin. Dos pasadas:

```rust
pub fn simplify(points: &[[f32; 2]], tolerance: f32) -> Vec<[f32; 2]>;
```

1. **Radial distance**: `radial_distance(points, sq_tolerance)` — descarta puntos consecutivos dentro de `sqrt(tolerance)` del anterior
2. **Ramer-Douglas-Peucker**: `rdp(points, sq_tolerance)` — recursivo, encuentra el punto más alejado de la línea base. Si su distancia perpendicular > `tolerance`, divide y repite

Parámetro: `tolerance = 0.3` (calles: `simplify(pts, 0.3)`).

#### 1b. `crates/vor-render/src/clip_poly.rs` — Sutherland-Hodgman + secure

Port de `clipPolygon` con `secure=1`:

```rust
pub fn clip_polygon(points: &[[f32; 2]], width: f32, height: f32, secure: bool) -> Vec<[f32; 2]>;
```

- Sutherland-Hodgman contra rectángulo `[0, 0, width, height]`
- `secure=true`: cada vértice en borde (`x===0 || x===width || y===0 || y===height`) se duplica dos veces para forzar B-spline a pasar por el borde
- Solo se usa en features de tierra (landmass); ocean layers usan `secure=false`

#### 1c. Integrar en pipeline

Modificar `build_fractal_landmass_mesh` (o crear nueva `build_landmass_mesh_v2`) para:

```
perimeter_vertices
  → vertices.positions → [[x, y], ...]
  → simplify(points, 0.3)
  → clipPoly(points, map_width, map_height, secure=true)
  → fractalizeCoastline(...)
  → buildCoastlinePath(...) (híbrido, ver punto 3)
  → round a 1 decimal
  → lyon tessellation → HeightmapMesh
```

### Archivos nuevo/modificados

| Archivo | Acción |
|---------|--------|
| `vor-render/src/simplify.rs` | **CREAR**: RDP + radial distance |
| `vor-render/src/clip_poly.rs` | **CREAR**: Sutherland-Hodgman + secure |
| `vor-render/src/coastline.rs` | **MODIFICAR**: integrar simplify + clipPoly en `build_fractal_landmass_mesh` |
| `vor-render/src/lib.rs` | **MODIFICAR**: exportar `simplify` y `clip_poly` |

---

## Punto 2: Fractalización de costa

### Estado actual
- `coastline.rs` YA tiene `fractalize_polygon` con roughness profile + midpoint displacement recursive
- `make_roughness_profile` con 4 armónicos, contraste 1.5, 256 muestras
- `subdivide_edge` recursivo con maxDepth=4, amplitude_decay=0.85 (Azgaar usa 0.9 — diferencia menor)
- Skip en bordes de mapa

### Lo que falta

#### 2a. Retornar metadatos de clasificación smooth/jagged

`fractalize_polygon` actual retorna `Vec<[f32;2]>` (puntos sin estructura). Para el path builder híbrido (punto 3), necesitamos saber qué spans originales fueron subdivididos (jagged) y cuáles no (smooth).

**Solución**: que `fractalize_polygon` retorne también un array de índices delimitando los puntos de cada span, o que directamente retorne `Vec<CoastlineSpan>`:

```rust
pub struct CoastlineSpan {
    pub start_idx: usize,
    pub end_idx: usize,   // inclusive
    pub is_smooth: bool,  // true = sin subdivisión fractal
}

pub fn fractalize_polygon(...) -> (Vec<[f32; 2]>, Vec<CoastlineSpan>);
```

Alternativa: que `fractalize_polygon` retorne los puntos con una estructura intercalada que permita reconstruir spans. Lo más limpio es el `Vec<CoastlineSpan>` adicional.

#### 2b. Verificar parámetros contra Azgaar

| Parámetro | Azgaar | Voronia actual | Diferencia |
|-----------|--------|----------------|------------|
| `maxDepth` | 4 | 4 | ✅ |
| `baseAmplitude` | 1.5 | 1.5 | ✅ |
| `amplitudeDecay` | 0.9 | 0.85 | ⚠️ menor → costa más suave |
| `minEdge` | 1 | 2 | ⚠️ mayor → menos fractales en aristas cortas |
| `smoothThreshold` | 0.25 | 0.25 | ✅ |
| `lakeSmoothThreshMult` | 2.0 | 2.0 | ✅ |
| `contrast` | 1.5 | 1.5 | ✅ |
| `numHarmonics` | 4 | 4 | ✅ |
| PROFILE_SIZE | 256 | 256 | ✅ |

Alinear `amplitude_decay` a 0.9 y `min_edge` a 1.0 si se busca bit-exactitud visual.

### Archivos

| Archivo | Acción |
|---------|--------|
| `vor-render/src/coastline.rs` | **MODIFICAR**: retornar `CoastlineSpan` metadata, alinear parámetros |

---

## Punto 3: Path builder híbrido (B-spline + Catmull-Rom)

### Estado actual
- `mesh.rs` tiene `catmull_rom_closed(points, subdivisions)` — uniform Catmull-Rom α=0 con subdivisiones fijas
- Se aplica a TODO el polígono indistintamente

### Lo que Azgaar hace
`buildCoastlinePath()` clasifica cada span original como:
- **Smooth**: sin subdivisión fractal → Q midpoint B-spline (equivalente D3 `curveBasisClosed`)
- **Jagged**: con subdivisión fractal → centripetal Catmull-Rom τ=0.25

### Plan

#### 3a. `crates/vor-render/src/coastline_path.rs` — Hybrid path builder

```rust
pub struct CoastlinePath {
    pub path: Vec<[f32; 2]>,
    pub commands: Vec<PathCommand>,
}

pub enum PathCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadraticTo(f32, f32, f32, f32),  // cx, cy, x, y
    CubicTo(f32, f32, f32, f32, f32, f32), // c1x, c1y, c2x, c2y, x, y
    Close,
}

pub fn build_coastline_path(
    fractal_points: &[[f32; 2]],
    spans: &[CoastlineSpan],
) -> CoastlinePath;
```

Algoritmo:
1. **Smooth span** (span sin subdivisión): emite Q midpoint B-spline
   - `mx = (cpx + npx) / 2; my = (cpy + npy) / 2`
   - Comando: `Q cpx,cpy mx,my`
   - El primer segmento arranca en el midpoint del último→primer span para loop seamless

2. **Jagged span** (span con subdivisión): emite centripetal Catmull-Rom τ=0.25
   - Para cada sub-segmento j entre puntos del span:
     ```
     cp1x = a.x + (b.x - prev.x) / 8
     cp1y = a.y + (b.y - prev.y) / 8
     cp2x = b.x - (nnext.x - a.x) / 8
     cp2y = b.y - (nnext.y - a.y) / 8
     C cp1x,cp1y cp2x,cp2y bx,by
     ```

3. **Transición smooth↔jagged**: trackear `at_mid` — si el cursor está en un midpoint (B-spline) y pasamos a jagged, emitir `L` al vértice original primero

#### 3b. Integrar con tessellation

El `CoastlinePath` con comandos debe convertirse a camino lyon (`lyon::path::Path::builder()`) para teselar:

```rust
fn coastline_path_to_lyon(path: &CoastlinePath) -> lyon::path::Path {
    let mut builder = Path::builder();
    for cmd in &path.commands {
        match cmd {
            MoveTo(x, y) => builder.begin(point(*x, *y)),
            LineTo(x, y) => builder.line_to(point(*x, *y)),
            QuadraticTo(cx, cy, x, y) =>
                builder.quadratic_bezier_to(point(*cx, *cy), point(*x, *y)),
            CubicTo(c1x, c1y, c2x, c2y, x, y) =>
                builder.cubic_bezier_to(point(*c1x, *c1y), point(*c2x, *c2y), point(*x, *y)),
            Close => builder.end(true),
        }
    }
    builder.build()
}
```

Lyon soporta curvas Bezier en teselación — no necesita que estén convertidas a segmentos de recta.

### Archivos

| Archivo | Acción |
|---------|--------|
| `vor-render/src/coastline_path.rs` | **CREAR**: Hybrid path builder (B-spline + Catmull-Rom) |
| `vor-render/src/coastline.rs` | **MODIFICAR**: usar `build_coastline_path` en `build_fractal_landmass_mesh` |
| `vor-render/src/lib.rs` | **MODIFICAR**: exportar `coastline_path` |

---

## Punto 4: Coastline stroke (línea de costa)

### Estado actual
- No existe. Solo hay relleno de tierra.

### Lo que Azgaar hace
La línea de costa NO es un stroke sobre el relleno — es un `<use>` del mismo path como **línea independiente**:
- `#sea_island`: stroke `#1f3846`, width 0.7, opacity 0.5, drop shadow
- `#lake_island`: stroke `#7c8eaf`, width 0.35, opacity 1, sin shadow

### Plan

#### 4a. `crates/vor-render/src/coastline_stroke.rs`

Generar mesh de líneas (LineList) que siga el perímetro de cada feature. No requiere reteselar — reutiliza el path del punto 3 pero lo renderiza como **trazo**, no como relleno.

```rust
pub struct CoastlineStrokeSettings {
    pub sea_stroke_color: [f32; 4],   // #1f3846 → linear
    pub sea_stroke_width: f32,        // 0.7
    pub sea_opacity: f32,            // 0.5
    pub lake_stroke_color: [f32; 4],  // #7c8eaf → linear
    pub lake_stroke_width: f32,       // 0.35
    pub lake_opacity: f32,           // 1.0
}

pub fn build_coastline_stroke_mesh(
    vertices: &VoronoiVertices,
    features: &[Feature],
    map_width: f32,
    map_height: f32,
    fractal_settings: &FractalSettings,
    stroke_settings: &CoastlineStrokeSettings,
) -> HeightmapMesh;
```

Implementación:
1. Para cada feature de tierra (is_land), extraer perímetro
2. Aplicar mismo pipeline: simplify → clipPoly → fractalize → buildCoastlinePath
3. En lugar de teselar con FillTessellator, teselar con StrokeTessellator de lyon
4. Color según feature type: sea island vs lake island
5. **Drop shadow**: generar un segundo path desplazado (1px, 1px) con color más oscuro y baja opacidad, renderizado primero

Lyon ya soporta `StrokeTessellator` con opciones de width, line_join, line_cap:

```rust
use lyon::tessellation::{StrokeOptions, StrokeTessellator};
let options = StrokeOptions::default()
    .with_line_width(stroke_width)
    .with_line_join(LineJoin::Round);
```

#### 4b. Integrar en layers

Agregar `coastline_stroke` como layer en `LayerFlags` y registrarlo en el renderer. Orden de dibujo: justo encima de landmass fill, debajo de los demás layers.

### Archivos

| Archivo | Acción |
|---------|--------|
| `vor-render/src/coastline_stroke.rs` | **CREAR**: Stroke mesh con lyon StrokeTessellator + drop shadow |
| `vor-render/src/layers.rs` | **MODIFICAR**: agregar `CoastlineStroke` flag |
| `vor-render/src/lib.rs` | **MODIFICAR**: exportar |

---

## Punto 5: Isoline engine (connectVertices)

### Estado actual
- No existe. `contour.rs` usa marching squares sobre grid (no Voronoi).
- No hay forma de caminar el grafo de Voronoi para extraer contornos de regiones.

### Lo que Azgaar hace
`connectVertices` camina el grafo de vértices Voronoi siguiendo la frontera entre celdas de distinto tipo:

```typescript
connectVertices({vertices, startingVertex, ofSameType, addToChecked, closeRing}):
  chain = []
  current = startingVertex
  loop:
    previous = chain[-1]
    chain.push(current)
    vertices.c[current].filter(ofSameType).forEach(addToChecked)
    c1,c2,c3 = vertices.c[current].map(ofSameType)
    v1,v2,v3 = vertices.v[current]
    if v1 != previous && c1 != c2 → next = v1
    else if v2 != previous && c2 != c3 → next = v2
    else if v3 != previous && c1 != c3 → next = v3
    until next == startingVertex
```

### Adaptación al modelo Voronia

En Voronia, el equivalente de `vertices.c[vertex]` es:
- `vertices.adjacent_cells[t]` — los 3 IDs de celdas que forman el triángulo Delaunay `t`

El equivalente de `vertices.v[vertex]` (vecinos de vértice) es:
- `vertices.adjacent_vertices[t]` — los 3 triángulos vecinos

Y `cells.v[cell]` → `vertices.cell_rings[p]`.

### Plan

#### 5a. `crates/vor-render/src/isoline.rs` — connectVertices

```rust
pub struct IsolineOptions {
    pub close_ring: bool,
    pub polygons: bool,
    pub fill: bool,
    pub water_gap: bool,
    pub halo: bool,
}

pub struct IsolineOutput {
    /// vertex IDs del chain, ordenados
    pub chain: Vec<u32>,
    /// puntos 2D resultantes (`vertices.positions[v]`)
    pub points: Vec<[f32; 2]>,
}

/// Camina el grafo de Voronoi desde starting_vertex hasta cerrar el loop
/// o hasta no encontrar más vecinos.
///
/// `same_type(cell_id)` debe retornar true si la celda pertenece al tipo
/// que estamos delineando (ej: `cells.culture[cell] == target_culture`).
///
/// `check_cell(cell_id)` se llama para cada celda del tipo que se "visita"
/// (marcar como checked para no reprocesar).
pub fn connect_vertices(
    vertices: &VoronoiVertices,
    starting_vertex: u32,
    same_type: impl Fn(usize) -> bool,
    check_cell: impl Fn(usize),
    close_ring: bool,
) -> Vec<u32>;
```

#### 5b. getFillPath y getBorderPath

```rust
/// Convierte un chain de vértices en un path cerrado suave (B-spline).
pub fn get_fill_path(chain: &[u32], vertices: &VoronoiVertices) -> lyon::path::Path;

/// Genera path con comandos M/L, rompiendo donde `discontinue(vertex_id)` es true.
/// Útil para water gaps y halos.
pub fn get_border_path(
    chain: &[u32],
    vertices: &VoronoiVertices,
    discontinue: impl Fn(u32) -> bool,
) -> lyon::path::Path;
```

#### 5c. getIsolines (envoltura)

```rust
/// Itera todas las celdas, encuentra regiones contiguas del mismo tipo,
/// y para cada una genera un chain cerrado via connect_vertices.
pub fn get_isolines(
    pack: &Pack,
    get_type: impl Fn(usize) -> u16,
    options: &IsolineOptions,
) -> Vec<IsolineOutput>;
```

#### 5d. Refactorizar contour.rs para usar isoline engine

Reemplazar marching squares actual con `connect_vertices` sobre el heightmap de Voronoi:

```rust
// En lugar de iterar grid cells con marching squares:
let h = pack.cells.height[p];
// connect_vertices con same_type = |c| pack.cells.height[c] >= h
```

### Archivos

| Archivo | Acción |
|---------|--------|
| `vor-render/src/isoline.rs` | **CREAR**: connectVertices + getFillPath + getBorderPath + getIsolines |
| `vor-render/src/contour.rs` | **MODIFICAR**: reemplazar marching squares con isoline engine (o mantener ambos y switchear) |
| `vor-render/src/lib.rs` | **MODIFICAR**: exportar `isoline` |

---

## Orden de implementación sugerido

```
Semana 1: Punto 5 (Isoline engine) — es la base de todo lo demás
   5a: connect_vertices
   5b: getFillPath, getBorderPath
   5c: getIsolines (envoltura)
   Tests: validar chain contra mapa Azgaar conocido

Semana 2: Punto 1 (Pipeline completo)
   1a: simplify.rs (RDP + radial distance)
   1b: clip_poly.rs (Sutherland-Hodgman + secure=1)
   1c: Integrar en coastline.rs
   Tests: validar contra simplify-js con fixture

Semana 3: Puntos 2 + 3 (Fractalización + Path builder híbrido)
   2a: CoastlineSpan metadata
   3a: build_coastline_path (B-spline + Catmull-Rom con transición)
   3b: Integrar con lyon tessellation
   Tests: validar path output contra Azgaar SVG

Semana 4: Punto 4 (Coastline stroke)
   4a: CoastlineStrokeSettings + build_coastline_stroke_mesh
   4b: Integrar en LayerFlags + renderer
   Tests: visual
```

---

## Dependencias entre puntos

```
Punto 5 (Isoline engine) ← independiente, no necesita nada más
Punto 3 (Path builder) ← necesita Punto 2 (CoastlineSpan metadata)
Punto 1 (Pipeline) ← necesita Punto 3 (build_coastline_path)
Punto 4 (Coastline stroke) ← necesita Puntos 1+3 (mismo pipeline para generar paths)

Por tanto: P5 → P2 → P3 → P1 → P4
```

Aunque P5 no es necesario para P1-P4 visualmente, es la base arquitectónica más importante y destraba todo el resto de capas (puntos 6+). Conviene implementarlo primero aunque no tenga efecto visual inmediato.

---

## Convenciones de código para este plan

- `lyon::tessellation::StrokeTessellator` para coastline stroke (punto 4)
- `lyon::path::Path::builder()` con `quadratic_bezier_to` y `cubic_bezier_to` para path híbrido (punto 3)
- `voronoi::{adjacent_cells, adjacent_vertices}` — exactamente el mapeo de `vertices.c` y `vertices.v` de Azgaar
- Tests con semilla fija y comparación de chains/paths contra fixtures
- `tracing::info!` logging en cada paso del pipeline para debug visual
