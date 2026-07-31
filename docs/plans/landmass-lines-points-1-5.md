# Implementation plan: points 1–5 — landmass, lines and smoothing

> **Based on**: `docs/analysis/azgaar-landmass-lines-smoothing.md`
> **Scope**: Feature → SVG pipeline (simplify, clipPoly, fractalize) → Hybrid path builder → Coastline stroke → Isoline engine (connectVertices)

---

## Point 1: Full feature → SVG path pipeline

### Current state
- `coastline.rs:build_fractal_landmass_mesh()` goes directly from `feature.perimeter_vertices` → `fractalize_polygon` → `catmull_rom_closed` → lyon tessellation
- **Missing**: `simplify` (Ramer-Douglas-Peucker), `clipPoly` with `secure=1`

### Plan

#### 1a. `crates/vor-render/src/simplify.rs` — Ramer-Douglas-Peucker + radial distance

Port of Vladimir Agafonkin's `simplify-js`. Two passes:

```rust
pub fn simplify(points: &[[f32; 2]], tolerance: f32) -> Vec<[f32; 2]>;
```

1. **Radial distance**: `radial_distance(points, sq_tolerance)` — discards consecutive points within `sqrt(tolerance)` of the previous one
2. **Ramer-Douglas-Peucker**: `rdp(points, sq_tolerance)` — recursive, finds the point farthest from the baseline. If its perpendicular distance > `tolerance`, split and repeat

Parameter: `tolerance = 0.3` (streets: `simplify(pts, 0.3)`).

#### 1b. `crates/vor-render/src/clip_poly.rs` — Sutherland-Hodgman + secure

Port of `clipPolygon` with `secure=1`:

```rust
pub fn clip_polygon(points: &[[f32; 2]], width: f32, height: f32, secure: bool) -> Vec<[f32; 2]>;
```

- Sutherland-Hodgman against rectangle `[0, 0, width, height]`
- `secure=true`: each vertex on the border (`x===0 || x===width || y===0 || y===height`) is duplicated twice to force the B-spline to pass through the border
- Only used for land features (landmass); ocean layers use `secure=false`

#### 1c. Integrate into the pipeline

Modify `build_fractal_landmass_mesh` (or create a new `build_landmass_mesh_v2`) to:

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

### New/modified files

| File | Action |
|---------|--------|
| `vor-render/src/simplify.rs` | **CREATE**: RDP + radial distance |
| `vor-render/src/clip_poly.rs` | **CREATE**: Sutherland-Hodgman + secure |
| `vor-render/src/coastline.rs` | **MODIFY**: integrate simplify + clipPoly into `build_fractal_landmass_mesh` |
| `vor-render/src/lib.rs` | **MODIFY**: export `simplify` and `clip_poly` |

---

## Point 2: Coastline fractalization

### Current state
- `coastline.rs` ALREADY has `fractalize_polygon` with roughness profile + recursive midpoint displacement
- `make_roughness_profile` with 4 harmonics, contrast 1.5, 256 samples
- Recursive `subdivide_edge` with maxDepth=4, amplitude_decay=0.85 (Azgaar uses 0.9 — minor difference)
- Skipped at map borders

### What's missing

#### 2a. Return smooth/jagged classification metadata

The current `fractalize_polygon` returns `Vec<[f32;2]>` (points without structure). For the hybrid path builder (point 3), we need to know which original spans were subdivided (jagged) and which were not (smooth).

**Solution**: have `fractalize_polygon` also return an index array delimiting the points of each span, or directly return `Vec<CoastlineSpan>`:

```rust
pub struct CoastlineSpan {
    pub start_idx: usize,
    pub end_idx: usize,   // inclusive
    pub is_smooth: bool,  // true = sin subdivisión fractal
}

pub fn fractalize_polygon(...) -> (Vec<[f32; 2]>, Vec<CoastlineSpan>);
```

Alternative: have `fractalize_polygon` return the points with an interleaved structure that allows spans to be reconstructed. The cleanest option is the extra `Vec<CoastlineSpan>`.

#### 2b. Verify parameters against Azgaar

| Parameter | Azgaar | Voronia current | Difference |
|-----------|--------|----------------|------------|
| `maxDepth` | 4 | 4 | ✅ |
| `baseAmplitude` | 1.5 | 1.5 | ✅ |
| `amplitudeDecay` | 0.9 | 0.85 | ⚠️ lower → smoother coastline |
| `minEdge` | 1 | 2 | ⚠️ higher → fewer fractals on short edges |
| `smoothThreshold` | 0.25 | 0.25 | ✅ |
| `lakeSmoothThreshMult` | 2.0 | 2.0 | ✅ |
| `contrast` | 1.5 | 1.5 | ✅ |
| `numHarmonics` | 4 | 4 | ✅ |
| PROFILE_SIZE | 256 | 256 | ✅ |

Align `amplitude_decay` to 0.9 and `min_edge` to 1.0 if visual bit-exactness is the goal.

### Files

| File | Action |
|---------|--------|
| `vor-render/src/coastline.rs` | **MODIFY**: return `CoastlineSpan` metadata, align parameters |

---

## Point 3: Hybrid path builder (B-spline + Catmull-Rom)

### Current state
- `mesh.rs` has `catmull_rom_closed(points, subdivisions)` — uniform Catmull-Rom α=0 with fixed subdivisions
- It is applied to the WHOLE polygon indiscriminately

### What Azgaar does
`buildCoastlinePath()` classifies each original span as:
- **Smooth**: no fractal subdivision → Q midpoint B-spline (D3 `curveBasisClosed` equivalent)
- **Jagged**: with fractal subdivision → centripetal Catmull-Rom τ=0.25

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

Algorithm:
1. **Smooth span** (span without subdivision): emit Q midpoint B-spline
   - `mx = (cpx + npx) / 2; my = (cpy + npy) / 2`
   - Command: `Q cpx,cpy mx,my`
   - The first segment starts at the midpoint of the last→first span for a seamless loop

2. **Jagged span** (span with subdivision): emit centripetal Catmull-Rom τ=0.25
   - For each sub-segment j between span points:
     ```
     cp1x = a.x + (b.x - prev.x) / 8
     cp1y = a.y + (b.y - prev.y) / 8
     cp2x = b.x - (nnext.x - a.x) / 8
     cp2y = b.y - (nnext.y - a.y) / 8
     C cp1x,cp1y cp2x,cp2y bx,by
     ```

3. **Smooth↔jagged transition**: track `at_mid` — if the cursor is at a midpoint (B-spline) and we switch to jagged, emit `L` to the original vertex first

#### 3b. Integrate with tessellation

The `CoastlinePath` with commands must be converted to a lyon path (`lyon::path::Path::builder()`) for tessellation:

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

Lyon supports Bezier curves in tessellation — they don't need to be converted to line segments.

### Files

| File | Action |
|---------|--------|
| `vor-render/src/coastline_path.rs` | **CREATE**: Hybrid path builder (B-spline + Catmull-Rom) |
| `vor-render/src/coastline.rs` | **MODIFY**: use `build_coastline_path` in `build_fractal_landmass_mesh` |
| `vor-render/src/lib.rs` | **MODIFY**: export `coastline_path` |

---

## Point 4: Coastline stroke (coastline line)

### Current state
- Doesn't exist. Only land fill exists.

### What Azgaar does
The coastline is NOT a stroke over the fill — it is a `<use>` of the same path as an **independent line**:
- `#sea_island`: stroke `#1f3846`, width 0.7, opacity 0.5, drop shadow
- `#lake_island`: stroke `#7c8eaf`, width 0.35, opacity 1, no shadow

### Plan

#### 4a. `crates/vor-render/src/coastline_stroke.rs`

Generate a line mesh (LineList) that follows the perimeter of each feature. No retessellation required — reuse the path from point 3 but render it as a **stroke**, not a fill.

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

Implementation:
1. For each land feature (is_land), extract the perimeter
2. Apply the same pipeline: simplify → clipPoly → fractalize → buildCoastlinePath
3. Instead of tessellating with FillTessellator, tessellate with lyon's StrokeTessellator
4. Color according to feature type: sea island vs lake island
5. **Drop shadow**: generate a second displaced path (1px, 1px) with a darker color and low opacity, rendered first

Lyon already supports `StrokeTessellator` with width, line_join and line_cap options:

```rust
use lyon::tessellation::{StrokeOptions, StrokeTessellator};
let options = StrokeOptions::default()
    .with_line_width(stroke_width)
    .with_line_join(LineJoin::Round);
```

#### 4b. Integrate into layers

Add `coastline_stroke` as a layer in `LayerFlags` and register it in the renderer. Draw order: directly above the landmass fill, below the other layers.

### Files

| File | Action |
|---------|--------|
| `vor-render/src/coastline_stroke.rs` | **CREATE**: stroke mesh with lyon StrokeTessellator + drop shadow |
| `vor-render/src/layers.rs` | **MODIFY**: add `CoastlineStroke` flag |
| `vor-render/src/lib.rs` | **MODIFY**: export |

---

## Point 5: Isoline engine (connectVertices)

### Current state
- Doesn't exist. `contour.rs` uses marching squares over a grid (not Voronoi).
- There is no way to walk the Voronoi graph to extract region contours.

### What Azgaar does
`connectVertices` walks the Voronoi vertex graph following the frontier between cells of different types:

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

### Adaptation to the Voronia model

In Voronia, the equivalent of `vertices.c[vertex]` is:
- `vertices.adjacent_cells[t]` — the 3 cell IDs that form the Delaunay triangle `t`

The equivalent of `vertices.v[vertex]` (vertex neighbors) is:
- `vertices.adjacent_vertices[t]` — the 3 neighboring triangles

And `cells.v[cell]` → `vertices.cell_rings[p]`.

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

#### 5b. getFillPath and getBorderPath

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

#### 5c. getIsolines (wrapper)

```rust
/// Itera todas las celdas, encuentra regiones contiguas del mismo tipo,
/// y para cada una genera un chain cerrado via connect_vertices.
pub fn get_isolines(
    pack: &Pack,
    get_type: impl Fn(usize) -> u16,
    options: &IsolineOptions,
) -> Vec<IsolineOutput>;
```

#### 5d. Refactor contour.rs to use the isoline engine

Replace the current marching squares with `connect_vertices` over the Voronoi heightmap:

```rust
// En lugar de iterar grid cells con marching squares:
let h = pack.cells.height[p];
// connect_vertices con same_type = |c| pack.cells.height[c] >= h
```

### Files

| File | Action |
|---------|--------|
| `vor-render/src/isoline.rs` | **CREATE**: connectVertices + getFillPath + getBorderPath + getIsolines |
| `vor-render/src/contour.rs` | **MODIFY**: replace marching squares with the isoline engine (or keep both and switch) |
| `vor-render/src/lib.rs` | **MODIFY**: export `isoline` |

---

## Suggested implementation order

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

## Dependencies between points

```
Punto 5 (Isoline engine) ← independiente, no necesita nada más
Punto 3 (Path builder) ← necesita Punto 2 (CoastlineSpan metadata)
Punto 1 (Pipeline) ← necesita Punto 3 (build_coastline_path)
Punto 4 (Coastline stroke) ← necesita Puntos 1+3 (mismo pipeline para generar paths)

Por tanto: P5 → P2 → P3 → P1 → P4
```

Although P5 is not visually necessary for P1-P4, it is the most important architectural foundation and unlocks all the remaining layers (points 6+). It should be implemented first even though it has no immediate visual effect.

---

## Code conventions for this plan

- `lyon::tessellation::StrokeTessellator` for the coastline stroke (point 4)
- `lyon::path::Path::builder()` with `quadratic_bezier_to` and `cubic_bezier_to` for the hybrid path (point 3)
- `voronoi::{adjacent_cells, adjacent_vertices}` — exactly the mapping of Azgaar's `vertices.c` and `vertices.v`
- Tests with a fixed seed and comparison of chains/paths against fixtures
- `tracing::info!` logging at each pipeline step for visual debugging
