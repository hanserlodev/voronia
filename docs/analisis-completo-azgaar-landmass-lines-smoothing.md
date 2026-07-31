# Complete analysis: how Azgaar draws landmass, lines and smoothing

> **Date**: July 30, 2026
> **Source**: Azgaar's FMP — `/home/hans/Proyectos/azgaar-fmg/` (local commit)
> **Purpose**: Exact reference for porting to Voronia the complete landmass, lines and smoothing pipeline
> **Covers**: feature polygons → simplify → clip → fractalize → B-spline/Catmull-Rom → coastline stroke → isoline engine (connectVertices + getIsolines) → human geography fills + water gaps + halos → borders → heightmap contours → ocean bathymetry → river curves → SVG output

> ⚠️ **IMMEDIATE IMPLEMENTATION SCOPE**: Only items **1–5** (feature → SVG path pipeline, fractalization, hybrid path builder, coastline stroke). The rest (items 6–16: isoline engine, human geography fills, halos, borders, heightmap contours, ocean bathymetry, etc.) remains documented for future implementation.

---

## Table of contents

1. [Complete landmass pipeline (feature → SVG path)](#1-complete-landmass-pipeline)
2. [Coastline fractalization (midpoint displacement + roughness profile)](#2-coastline-fractalization)
3. [Hybrid path builder: B-spline + Catmull-Rom](#3-hybrid-path-builder)
4. [Coastline stroke (coastline line)](#4-coastline-stroke)
5. [Isoline engine: connectVertices](#5-isoline-engine)
6. [Isoline engine: getIsolines](#6-getisolines)
7. [Human geography fills (states, cultures, religions, provinces)](#7-human-geography-fills)
8. [Water gap technique](#8-water-gap)
9. [State halos](#9-state-halos)
10. [Borders](#10-borders)
11. [Heightmap contours](#11-heightmap-contours)
12. [Ocean bathymetric layers](#12-ocean-bathymetric-layers)
13. [River smoothing (Catmull-Rom + meander)](#13-river-smoothing)
14. [Curves available in Azgaar (D3 catalog + custom)](#14-curves-available)
15. [SVG styles summary](#15-svg-styles-summary)
16. [Voronia equivalences](#16-voronia-equivalences)

---

## 1. Complete landmass pipeline

**File**: `src/renderers/draw-features.ts:76-87` — `featurePathRenderer()`

Each land feature (continent, island, lake, island-in-lake) goes through this exact pipeline:

```
feature.vertices (IDs de vértices)
  → pack.vertices.p[vertex] → array de [x, y]
  → simplify(points, 0.3)         # Ramer-Douglas-Peucker, tolerancia 0.3px
  → clipPoly(points, W, H, 1)    # Clip a bordes del mapa, secure=1 duplica
                                   #   puntos de borde para B-spline
  → fractalizeCoastline(...)      # Midpoint displacement recursivo
  → buildCoastlinePath(...)       # B-spline (spans suaves) + Catmull-Rom
                                   #   (spans fractalizados) → SVG path string
  → round(path, 1)               # Redondea números a 1 decimal
  → + "Z"                        # Cierra el path
```

### 1.1 simplify (simplify-js)

**File**: `public/libs/simplify.js`

Library by Vladimir Agafonkin. Combines two algorithms:
1. **Radial distance**: removes consecutive points within the squared tolerance
2. **Ramer-Douglas-Peucker**: recursive, finds the point farthest from the baseline; if it exceeds the tolerance, it splits and repeats

Called with `simplify(points, 0.3)` — 0.3 pixel tolerance, without `highestQuality` → uses both algorithms.

### 1.2 clipPoly with secure=1

**File**: `src/utils/commonUtils.ts:16-37`

```typescript
const clipped = clipPolygon(points, [0, 0, graphWidth, graphHeight]);
// secure=1: duplica puntos de borde para que B-spline pase por ellos
for (const point of clipped) {
  secured.push(point);
  if (point[0] === 0 || point[0] === graphWidth ||
      point[1] === 0 || point[1] === graphHeight) {
    secured.push(point, point); // duplicado doble
  }
}
```

Without `secure`, D3's B-spline (`curveBasisClosed`) arcs away from the map edge, leaving a gap. By duplicating the boundary points, the curve passes exactly through them.

**Only `draw-features.ts` uses `secure=1`**. Ocean-layers and others do not need it because their paths are not closed with `curveBasisClosed`.

---

## 2. Coastline fractalization

**File**: `src/renderers/coastline-fractal.ts`

### 2.1 Roughness profile (per-feature)

Each feature receives a unique roughness profile via a deterministic PRNG (Alea with seed `${seed}_c${featureIndex}`).

```typescript
function makeRoughnessProfile(rand, contrast, numHarmonics = 4): Float32Array {
  // Suma de N cosenos armónicos (wrap-around, sin costura)
  for (let k = 1; k <= numHarmonics; k++) {
    const amp = rand();
    const phase = rand() * Math.PI * 2;
    for (let i = 0; i < PROFILE_SIZE; i++) {
      profile[i] += amp * cos(2πk·i/PROFILE_SIZE + phase);
    }
  }
  // Normalizar [0,1] y elevar a contraste
  // contrast=1.5 → zonas calmadas se vuelven más calmadas, rugosas más rugosas
  profile[i] = ((profile[i] - min) / range) ** contrast;
}
```

- `PROFILE_SIZE = 256` samples around the perimeter
- `numHarmonics = 4` → ~4 rough zones around the perimeter
- `contrast = 1.5` → accentuates the difference between calm and rough

### 2.2 Recursive midpoint displacement

```
subdivideEdge(x0, y0, x1, y1, t0, t1, depth, amplitude, profile, rand):
  if depth==0 OR len < minEdge → return
  tm = midT(t0, t1)           # posición normalizada circular
  roughness = sampleProfile(profile, tm)
  if roughness < smoothThreshold → return  # zona calmada, no subdividir
  
  # Desplazamiento perpendicular
  disp = (rand()-0.5) * sqrt(len) * amplitude * roughness
  mx = (x0+x1)/2 + (-dy/len) * disp   # perpendicular izquierda
  my = (y0+y1)/2 + (dx/len) * disp
  
  subdivideEdge(x0,y0, mx,my, t0,tm, depth-1, nextAmp, ...)
  resultPts.push([mx, my])
  subdivideEdge(mx,my, x1,y1, tm,t1, depth-1, nextAmp, ...)
```

Default parameters:
- `maxDepth = 4` → up to 16 segments per original edge
- `baseAmplitude = 1.5` → peak displacement
- `amplitudeDecay = 0.9` → 90% of amplitude per level
- `minEdge = 1` → edges < 1px are not subdivided
- `smoothThreshold = 0.25` → zones with roughness < 0.25 are not subdivided
- `lakeSmoothThreshMult = 2.0` → lakes have threshold 0.5 (calmer)

### 2.3 Skip on map edges

Edges where BOTH vertices are on the map boundary (`x===0 || x===graphWidth || y===0 || y===graphHeight`) are not fractalized — they keep the original straight line.

---

## 3. Hybrid path builder

**File**: `src/renderers/coastline-fractal.ts:194-252` — `buildCoastlinePath()`

### 3.1 smooth/jagged classification

```typescript
// smooth[i] = true si el span original i→i+1 NO tiene subdivisión fractal
smooth[i] = (b > a ? b - a : b + N - a) === 1;
// Si solo hay 1 vértice entre orig[i] y orig[i+1], es smooth.
// Si hay más (fractal sub-points), es jagged.
```

### 3.2 Smooth spans: Q midpoint B-spline

Exact equivalent of D3's `curveBasisClosed`:

```
M→(midpoint del último→primer span)   # arranque seamless
Q cpx,cpy mx,my                        # Q = quadratic bezier
```

Where `mx = (cpx + npx) / 2`, `my = (cpy + npy) / 2`. This produces smooth arcs that hide the angularity of the Voronoi tessellation.

### 3.3 Jagged spans: centripetal Catmull-Rom

For spans with fractal subdivision, centripetal Catmull-Rom (α~0.5, although the formula used is equivalent to tension ~0.25):

```
for each sub-segment j:
  cp1x = a.x + (b.x - prev.x) / 8
  cp1y = a.y + (b.y - prev.y) / 8
  cp2x = b.x - (nnext.x - a.x) / 8
  cp2y = b.y - (nnext.y - a.y) / 8
  C cp1x,cp1y cp2x,cp2y bx,by
```

The division by 8 produces tangents = 1/4 of the difference, equivalent to Catmull-Rom tension τ=0.25.

### 3.4 Seamless smooth↔jagged transition

The path starts at the midpoint of the last span (if smooth) so that the closed loop has no seam. The `atMid` variable tracks whether the cursor is at a midpoint (B-spline) or at an original vertex. When transitioning from smooth to jagged, an `L` is emitted to the original vertex first.

---

## 4. Coastline stroke (coastline line)

**File**: `src/services/io/auto-update.ts:191-204`

The coastline is NOT a stroke over the land fill — it is a `<use>` that reuses the feature's own path but rendered as a **line** without fill.

### 4.1 DOM structure

```
<g id="coastline" fill="none" stroke-linejoin="round">
  <g id="sea_island">     <!-- islas oceánicas -->
    <use href="#feature_N"/>    <!-- stroke opaco -->
  </g>
  <g id="lake_island">    <!-- islas en lagos -->
    <use href="#feature_M"/>    <!-- stroke más fino -->
  </g>
</g>
```

### 4.2 Styles

| Group | opacity | stroke | stroke-width | filter |
|-------|---------|--------|-------------|--------|
| `#sea_island` | 0.5 | `#1f3846` | 0.7 | `url(#dropShadow)` |
| `#lake_island` | 1 | `#7c8eaf` | 0.35 | none |

### 4.3 Difference from fill

The land fill is NOT rendered directly — land is visible through the thematic layers (heightmap, biomes, states, etc.). The coastline stroke is subtle (~0.5px), semi-transparent, and on oceanic islands it carries a drop shadow for depth.

---

## 5. Isoline engine: connectVertices

**File**: `src/utils/pathUtils.ts:261-311`

It is the fundamental algorithm that walks the Voronoi graph to trace the outline of a region of cells of the same type.

### 5.1 Algorithm

```
connectVertices({vertices, startingVertex, ofSameType, addToChecked, closeRing}):
  chain = []
  current = startingVertex
  loop:
    previous = chain[-1]
    chain.push(current)
    
    // Marcar celdas adyacentes del mismo tipo como visitadas
    vertices.c[current].filter(ofSameType).forEach(addToChecked)
    
    // Evaluar los 3 vértices vecinos
    c1,c2,c3 = vertices.c[current].map(ofSameType)  // boolean: cell pertenece al tipo?
    v1,v2,v3 = vertices.v[current]                   // los 3 vecinos
    
    if v1 != previous && c1 != c2 → next = v1
    else if v2 != previous && c2 != c3 → next = v2
    else if v3 != previous && c1 != c3 → next = v3
    
    until next == startingVertex  // cerramos el loop
```

### 5.2 Decision logic

For each current vertex, it examines its 3 adjacent cells (`vertices.c[vertex]` = normally 3 contiguous cells). If two consecutive cells are of different types, the vertex is on the border and the neighbor connecting those two cells is the next one in the chain.

```
   c1    c2
     \  /
      v   →  si c1 != c2, la arista v→v1 cruza la frontera
     /  \
   c3    (implícito)
```

### 5.3 Variants

| Variant | File | Difference |
|----------|---------|------------|
| General (`pathUtils.ts`) | `src/utils/pathUtils.ts:261` | Takes the `ofSameType`, `addToChecked`, `closeRing` callbacks |
| Heightmap (`draw-heightmap.ts`) | `src/renderers/draw-heightmap.ts:162` | Specialized for height: compares `cells.h[c] < h` |
| Ocean (`ocean-layers.ts`) | `src/renderers/ocean-layers.ts:35` | Specialized for oceanic temperature layers |

---

## 6. Isoline engine: getIsolines

**File**: `src/utils/pathUtils.ts:84-177`

### 6.1 Flow

```
getIsolines(pack, getType, options):
  checkedCells = Uint8Array(len)
  
  for each cell:
    if checked OR !getType(cell) → skip
    type = getType(cell)
    
    ofSameType = cellId => getType(cellId) === type
    ofDifferentType = cellId => getType(cellId) !== type
    
    // Buscar celda vecina de distinto tipo
    onborderCell = cells.c[cell].find(ofDifferentType)
    if not found → continue (región sin borde exterior)
    
    // Skip si es lago interno (todas las celdas del shoreline son del mismo tipo)
    feature = features[cells.f[onborderCell]]
    if feature.type==="lake" && feature.shoreline.every(ofSameType) → continue
    
    // Encontrar vértice de arranque
    startingVertex = cells.v[cell].find(v => vertices.c[v].some(ofDifferentType))
    
    vertexChain = connectVertices({vertices, startingVertex, ofSameType, addToChecked, closeRing: true})
    
    addIsolineTo(type, vertices, vertexChain, isolines, options)
```

### 6.2 Output: `addIsolineTo`

Depending on `options`, it generates:

| Option | Output | Description |
|--------|--------|-------------|
| `polygons` | `isolines[type].polygons` | `vertexChain.map(v => vertices.p[v])` — arrays of points |
| `fill` | `isolines[type].fill` | SVG string with fill paths (via `getFillPath`) |
| `waterGap` | `isolines[type].waterGap` | Paths interrupted on land (via `getBorderPath`) |
| `halo` | `isolines[type].halo` | Paths interrupted at the map edge (via `getBorderPath`) |

### 6.3 `getFillPath`

**File**: `src/utils/pathUtils.ts:49-82`

```typescript
function getFillPath(vertices, vertexChain): string {
  return vertexChain.map(v => vertices.p[v]).join(" ");
  // Pasado por lineGen().curve(curveBasisClosed) en el llamante
}
```

### 6.4 `getBorderPath`

**File**: `src/utils/pathUtils.ts:25-47`

Generates an SVG path with M/L commands, breaking the path where `discontinue(vertex)` is true. This produces multiple sub-paths instead of one continuous path.

```
getBorderPath(vertices, vertexChain, discontinue):
  for each vertex:
    if discontinue(vertex):
      break path (insertar "M" en el próximo)
    else:
      emitir "L vertices.p[vertex]" (o "M" si es inicio de segmento)
```

---

## 7. Human geography fills

All human fill layers follow the same pattern:

```
getIsolines(pack, cellId => cells.{type}[cellId], { fill: true, waterGap: true [, halo: bool] })
→ por cada tipo:
    <path d="{fill}" fill="{color}" id="{type}{index}" />
    <path d="{waterGap}" fill="none" stroke="{color}" stroke-width="3" id="{type}-gap{index}" />
    [<clipPath id="state-clip{index}"><use href="#state{index}"/></clipPath>]
    [<path d="{halo}" stroke="{darkerColor}" clip-path="url(#state-clip{index})" />]
```

### 7.1 Per-layer pattern

| Layer | File (layers.js) | getType | options | Extra |
|------|--------------------|---------|---------|-------|
| Cultures | `drawCultures():480` | `cells.culture` | `{fill, waterGap}` | — |
| Religions | `drawReligions():509` | `cells.religion` | `{fill, waterGap}` | — |
| States | `drawStates():537` | `cells.state` | `{fill, waterGap, halo}` | Halo only if `shapeRendering==="geometricPrecision"` |
| Provinces | `drawProvinces():592` | `cells.province` | `{fill, waterGap}` | — |
| Zones | `drawZones():978` | `cells.zone` | `{fill}` | No water gap |

### 7.2 Key difference: isolines vs borders

**getIsolines** → generates **fill** paths + water gaps + halos. The paths are **closed curves** (B-spline via D3).

**drawBorders** → generates **line** paths (stroke) between cells of different states/provinces. The paths are **straight segments** (join of vertices with `M...L...L...`). There is no smoothing in borders.

---

## 8. Water gap technique

### 8.1 Purpose

To prevent a region's color from visually "bleeding" into the ocean/lake at the edges. Azgaar draws a thin stroke of the same color as the fill on the edges that touch water.

### 8.2 Implementation

`getBorderPath(vertices, vertexChain, isLandVertex)` where:
```typescript
const isLandVertex = (vertexId) => vertices.c[vertexId].every(i => cells.h[i] >= 20);
```

When the vertex is surrounded ONLY by land cells (height ≥ 20), the path is broken. When it is on the coast (land/water mix), the path continues. The result is a path that only draws on edges against water.

### 8.3 Style

```html
<path d="{waterGap}" fill="none" stroke="{color}" stroke-width="3" ... />
```

Stroke-width 3 is deliberately thick to cover any anti-aliasing or gap. Since it is the same color as the fill, it blends in visually.

---

## 9. State halos

### 9.1 Purpose

When `shapeRendering === "geometricPrecision"`, states have a halo (inner shadow) that visually separates them.

### 9.2 Implementation

```typescript
// 1. Path del halo: solo vértices en borde de mapa
const isBorderVertex = (vertexId) => vertices.c[vertexId].some(i => cells.b[i]);
// 2. Color más oscuro
const haloColor = d3.color(color).darker().hex();
// 3. Render con clip-path para que quede DENTRO del estado
<clipPath id="state-clip{index}"><use href="#state{index}"/></clipPath>
<path d="{halo}" stroke="{haloColor}" clip-path="url(#state-clip{index})" />
```

The halo is only drawn where the state's path touches the map edge.

---

## 10. Borders

**File**: `src/renderers/draw-borders.ts`

### 10.1 Fundamental difference from getIsolines

`drawBorders` does NOT use `getIsolines`. It uses a separate algorithm that:
1. Iterates over cells looking for pairs (cellA, cellB) of different states/provinces
2. For each pair, finds an initial vertex on the border
3. Walks the vertex graph with `getVerticesLine()` (similar to `connectVertices` but local)
4. Output: `M x0,y0 x1,y1 x2,y2 ...` (straight segments, no smoothing)
5. Marks cell-state pairs as checked to avoid duplication

### 10.2 Output

```typescript
select("#stateBorders").append("path").attr("d", statePath.join(" "));
select("#provinceBorders").append("path").attr("d", provincePath.join(" "));
```

No explicit stroke attributes in the renderer — they are inherited from CSS `#borders { stroke-linejoin: round; fill: none; }` and SVG defaults. The color/width is set dynamically from the editor.

### 10.3 Visual style (from editor)

- State borders: stroke `#000`, stroke-width `1.2` (when highlighted)
- Province borders: stroke `#999`, stroke-width `0.5-1.0` (SVG defaults)
- No smoothing — the lines follow the Voronoi edges directly

---

## 11. Heightmap contours

**File**: `src/renderers/draw-heightmap.ts`

### 11.1 Algorithm

```
for each unique height h in sorted cells:
  if used[cell] → skip
  if not on border (no neighbor with lower h) → skip
  
  startingVertex = cells.v[cell].find(v => vertices.c[v].some(c => cells.h[c] < h))
  chain = connectVertices(cells, vertices, startingVertex, h, used)
  // connectVertices especializado: compara cells.h[c] < h
  
  points = simplifyLine(chain, relax).map(v => vertices.p[v])
  // simplifyLine: toma cada N-ésimo vértice (N = relax+1)
  
  path = round(lineGen(points) || "")
  // lineGen con curveBasisClosed por defecto
```

### 11.2 Configuration

Two separate SVG groups:
- `#oceanHeights`: heights < 20 (ocean), conditional render
- `#landHeights`: heights >= 20 (land), always renders

Configurable attributes via DOM:
- `skip`: every N height levels (default 1)
- `relax`: simplification (stride, default 0)
- `curve`: D3 curve type (default `curveBasisClosed`)
- `scheme`: color scheme
- `terracing`: terracing shadow (offset + darker)

### 11.3 Rendering

```typescript
// Paths se agrupan por altura y se renderizan como rect base + paths coloreados
if (terracing) {
  group.append("path").attr("d", path)
    .attr("transform", "translate(.7,1.4)")
    .attr("fill", color(fillColor).darker(terracing));
}
group.append("path").attr("d", path).attr("fill", fillColor);
```

Terracing gives a 3D effect by offsetting a darker copy by 0.7px in X, 1.4px in Y.

---

## 12. Ocean bathymetric layers

**File**: `src/renderers/ocean-layers.ts`

### 12.1 Algorithm

Similar to heightmap contours but for oceanic temperature layers (`cells.t`):

```
for each cell with t < 0 (oceánica):
  if used or not in limits → skip
  start = findStart(i, t)  # celda en borde de esta capa
  chain = connectVertices(start, t)  # variante ocean-layers
  
  # Simplificar: cada (-t*2)-ésimo vértice + siempre preservar bordes de mapa
  relax = 1 + t * -2  # t=-1→3, t=-2→5, t=-3→7
  relaxed = chain.filter((v, i) => !(i % relax) || vertices.c[v].some(c => c >= pointsN))
  
  points = clipPoly(relaxed.map(v => vertices.p[v]), W, H)
  path = round(lineGen(points) || "")
  
  append path con fill="#ecf2f9" y fill-opacity = 0.4/limits.length
```

### 12.2 Differences from heightmap

- layers.js calls `getIsolines(pack, cellId => cells.t[cellId], { polygons: true })` to obtain the polygons
- Uses `curveBasisClosed` by default (via `lineGen`)
- Total opacity `0.4 / num_limits` distributed among the layers
- No water gap or halos

---

## 13. River smoothing

**File**: `src/generators/river-generator.ts:426-455`

### 13.1 Catmull-Rom for banks

```typescript
this.lineGen.curve(curveCatmullRom.alpha(0.1));
// Right bank: lineGen(riverPointsRight.reverse())
// Left bank: reverse of lineGen(riverPointsLeft)
```

`alpha=0.1` → very close to uniform Catmull-Rom (α=0), produces smoother curves than centripetal (α=0.5).

### 13.2 Meander + relaxAcuteAngles

**File**: `src/utils/pathUtils.ts:370-506`

```
meander(cells, cellPositions, options):
  for each consecutive pair of anchor points:
    dist² = (x2-x1)² + (y2-y1)²
    
    meanderVal = meandering + 1/step + max(meandering - step/100, 0)
    if near water: meanderVal *= WATER_MEANDER_SCALE (0.25)
    
    if step < 20 && (dist² > 64 || (dist² > 36 && cellCount < 5)):
      # 2 control points (cubic bezier)
      p1x = (2x1+x2)/3 - sin(angle) * meanderVal
      p1y = (2y1+y2)/3 + cos(angle) * meanderVal
      p2x = (x1+2x2)/3 + sin(angle) * meanderVal / 2
      p2y = (y1+2y2)/3 - cos(angle) * meanderVal / 2
    elif dist² > 25 || cellCount < 6:
      # 1 control point (quadratic bezier)
      p1x = (x1+x2)/2 - sin(angle) * meanderVal
      p1y = (y1+y2)/2 + cos(angle) * meanderVal
    
    relaxAcuteAngles(points, anchorIndices)
```

`relaxAcuteAngles` iterates over the control points and reflects those forming acute angles (< 60°) across the anchor-to-anchor baseline.

### 13.3 Difference from the coastline path

While the coastline uses **centripetal Catmull-Rom** (α=0.5, implicit in the Hermite formula with division by 8), rivers use **uniform Catmull-Rom** (α=0.1 ~ α=0). The coastline prioritizes the curve passing close to the fractalized points; rivers prioritize overall smoothness.

---

## 14. Curves available in Azgaar

### 14.1 Complete D3 catalog

From `draw-heightmap.ts:27-45`, Azgaar exposes ALL the D3 curves:

| Curve | Type | Use in Azgaar |
|-------|------|---------------|
| `curveBasis` | Open B-spline | — |
| `curveBasisClosed` | Closed B-spline | **Default** for heightmap, temperature, markets, ocean, isofeatures |
| `curveBasisOpen` | Open B-spline | — |
| `curveCardinal` | Open Cardinal | — |
| `curveCardinalClosed` | Closed Cardinal | — |
| `curveCardinalOpen` | Open Cardinal | — |
| `curveCatmullRom` | Open Catmull-Rom | Rivers (α=0.1) |
| `curveCatmullRomClosed` | Closed Catmull-Rom | — |
| `curveCatmullRomOpen` | Open Catmull-Rom | — |
| `curveLinear` | Straight segments | User selectable |
| `curveLinearClosed` | Closed straight segments | — |
| `curveMonotoneX` | Monotone X | — |
| `curveMonotoneY` | Monotone Y | — |
| `curveNatural` | Natural spline | — |
| `curveStep` | Stepped | — |
| `curveStepAfter` | Stepped after | — |
| `curveStepBefore` | Stepped before | — |

### 14.2 Custom curve: coastline hybrid

`buildCoastlinePath` is a **custom hybrid** curve that does NOT exist in D3: it combines B-spline (smooth spans) with centripetal Catmull-Rom (fractalized spans) depending on whether the original span was subdivided or not.

---

## 15. SVG styles summary

| Element | Fill | Stroke | Stroke-width | Opacity | Filter |
|----------|------|--------|-------------|----------|--------|
| Coastline (sea island) | none | `#1f3846` | 0.7 | 0.5 | `url(#dropShadow)` |
| Coastline (lake island) | none | `#7c8eaf` | 0.35 | 1 | none |
| Lakes | group color | — | — | 0.5-1 | — |
| Heightmap contours | color scheme | — | — | 1 | offset terracing |
| State fill | `states[index].color` | — | — | 1 | — |
| State water gap | none | `states[index].color` | 3 | 1 | — |
| State halo | — | `color.darker()` | SVG default | — | clip-path to the state |
| Culture fill | `cultures[index].color` | — | — | 1 | — |
| Religion fill | `religions[index].color` | — | — | 1 | — |
| Province fill | `provinces[index].color` | — | — | 1 | — |
| State borders | none | `#000` (default) | 1.2 (hover) | 1 | — |
| Province borders | none | `#999` (default) | 0.5-1 | 1 | — |
| Ocean layers | `#ecf2f9` | — | — | `0.4/num` | — |
| Rivers left bank | `#6f9db3` (default) | — | — | 1 | — |
| Rivers right bank | `#0f2631` (default) | — | — | 1 | — |

---

## 16. Voronia equivalences

| Azgaar system | Voronia status | Voronia file |
|----------------|---------------|-----------------|
| `simplify(points, 0.3)` | Not implemented (features are used raw) | `mesh.rs` / new |
| `clipPoly(points, W, H, 1)` | Implemented in `mesh.rs` (bbox clip) | `mesh.rs` |
| `fractalizeCoastline()` | **Partial**: fractal midpoint displacement yes, roughness profile yes | `coastline.rs` |
| `buildCoastlinePath()` hybrid | **Partial**: Catmull-Rom yes, missing smooth spans + B-spline | `coastline.rs` |
| Coastline stroke (line) | **Not implemented**: only fill exists | new `coastline_stroke.rs` |
| `connectVertices()` | **Not implemented**: core of the isoline engine | new `isoline.rs` |
| `getIsolines()` | **Not implemented**: wrapper that iterates cells | new `isoline.rs` |
| `getBorderPath()` water gap | **Partial**: `water_gap.rs` implements coastline cell detection | `water_gap.rs` |
| `getFillPath()` | Not implemented (for smoothed B-spline fill) | new |
| State/culture/religion fills + water gap | **Done**: `state_layer.rs` etc. with `append_water_gap` | several |
| State halos | **Not implemented** | new |
| Borders cell-to-cell | **Done**: `border.rs` with straight segments | `border.rs` |
| Heightmap contours | **Partial**: `contour.rs` uses marching squares over a grid, not over Voronoi | `contour.rs` |
| Ocean bathymetric rings | **Not implemented** | new |
| River Catmull-Rom α=0.1 | **Done**: `river.rs` uses `build_river_mesh` with meander + width | `river.rs` |
| River meander + relax | **Done**: exact port in `vor-sim` | `vor-sim/src/river/meander.rs` |

### 16.1 What would give the most visual impact (recommended order)

1. **connectVertices + getIsolines** — unlocks smooth borders, correct heightmap contours, ocean layers, and replaces the current marching squares
2. **Hybrid buildCoastlinePath** — replaces the simple Catmull-Rom with B-spline + Catmull-Rom per span
3. **Coastline stroke** — the coastline line over the land fill
4. **State halos** — visual separation between states
5. **Ocean bathymetric rings** — oceanic depth

### 16.2 Note on B-spline in wgpu

D3 generates B-spline curves as SVG paths (Q/C commands). In wgpu there is no native path renderer. Options:
- **Option A (CPU)**: Tessellate the paths into triangles on the CPU before uploading to the GPU
- **Option B (GPU)**: Geometry shader that evaluates B-spline/Catmull-Rom in the vertex shader
- **Option C (lyon)**: Lyon has `tessellate_path`, which supports Bezier curves → triangles

Voronia already uses lyon for tessellation, so Option C is the most natural: generate SVG paths (M/Q/C commands) and tessellate them with lyon.

---

*End of analysis — July 30, 2026*
