# Exhaustive analysis: how Azgaar draws the entire landmass

> **Date**: July 30, 2026
> **Source**: Azgaar's FMP — `/home/hans/Proyectos/azgaar-fmg/`
> **Purpose**: Complete reference for the native land rendering port in Voronia
> **Covers**: the ENTIRE landmass drawing pipeline — from the base geometry to the thematic layers

---

## Table of contents

1. [Layer architecture (Z-order)](#1-layer-architecture-z-order)
2. [Base: Features (land, ocean, coastline, lakes)](#2-base-features-land-ocean-coastline-lakes)
3. [Coastline fractalization](#3-coastline-fractalization)
4. [Ocean: base and bathymetric](#4-ocean-base-and-bathymetric)
5. [Shared isoline engine](#5-shared-isoline-engine)
6. [Heightmap (elevation contours)](#6-heightmap-elevation-contours)
7. [Biomes](#7-biomes)
8. [Cultures](#8-cultures)
9. [Religions](#9-religions)
10. [States](#10-states)
11. [Provinces](#11-provinces)
12. [Borders](#12-borders)
13. [Rivers](#13-rivers)
14. [Routes (roads, trails, sea routes)](#14-routes-roads-trails-sea-routes)
15. [Relief icons](#15-relief-icons)
16. [Burgs (settlements)](#16-burgs-settlements)
17. [State labels (curved text)](#17-state-labels-curved-text)
18. [Burg labels](#18-burg-labels)
19. [Temperature (isotherms)](#19-temperature-isotherms)
20. [Precipitation](#20-precipitation)
21. [Population](#21-population)
22. [Ice layer](#22-ice-layer)
23. [Goods](#23-goods)
24. [Markets](#24-markets)
25. [Emblems (coats of arms)](#25-emblems-coats-of-arms)
26. [Military](#26-military)
27. [Satellite texture (3D)](#27-satellite-texture-3d)
28. [Overlays: texture, grid, coordinates](#28-overlays-texture-grid-coordinates)
29. [Key architectural patterns](#29-key-architectural-patterns)
30. [Portability status to Voronia](#30-portability-status-to-voronia)

---

## 1. Layer architecture (Z-order)

**File**: `public/modules/ui/layers.js:225-261` — `drawLayers()`

It is the master drawing function, called on every map generation/refresh. The visual order (Z-order) is determined by the order of the `<g>` groups in the SVG DOM.

| Z | SVG group | Renderer | What it draws |
|---|-----------|-------------|------------|
| 1 | `#ocean` | template | Base ocean rectangle + pattern |
| 2 | `#oceanLayers` | `OceanLayers` | Bathymetric contour rings |
| 3 | `#oceanPattern` | — | Ocean texture image |
| 4 | `#coastline` | `drawFeatures` | Coastlines (sea + lakes) |
| 5 | `#lakes` | `drawFeatures` | Lake fill (fresh/salt/sinkhole...) |
| 6 | `#landmass` | (mask) | Land paths for reuse |
| 7 | `#terrs` | `drawHeightmap` | Elevation contours (ocean + land) |
| 8 | `#texture` | `drawTexture` | Background texture (paper, parchment...) |
| 9 | `#biomes` | `drawBiomes` | Biome fill per cell |
| 10 | `#cells` | `drawCells` | Voronoi cell wireframe (debug) |
| 11 | `#gridOverlay` | `drawGrid` | Hex/square grid |
| 12 | `#coordinates` | `drawCoordinates` | Lat/lon graticule + labels |
| 13 | `#compass` | — | Compass rose |
| 14 | `#rivers` | `drawRivers` | River polygons |
| 15 | `#terrain` | `drawReliefIcons` | Relief icons (mountains, trees) |
| 16 | `#relig` | `drawReligions` | Religions fill |
| 17 | `#cults` | `drawCultures` | Cultures fill |
| 18 | `#regions` | `drawStates` | States fill + halos |
| 19 | `#provs` | `drawProvinces` | Provinces fill + labels |
| 20 | `#zones` | `drawZones` | Zones fill |
| 21 | `#borders` | `drawBorders` | State + province border lines |
| 22 | `#routes` | `drawRoutes` | Roads, trails, sea routes |
| 23 | `#temperature` | `drawTemperature` | Colored isotherms + labels |
| 24 | `#prec` | `drawPrecipitation` | Precipitation circles |
| 25 | `#population` | `drawPopulation` | Population bars |
| 26 | `#ice` | `drawIce` | Glaciers + icebergs |
| 27 | `#goods` | `drawGoods` | Goods production |
| 28 | `#markets` | `drawMarketsLayer` | Market influence zones |
| 29 | `#tradeAnimation` | `TradeAnimation` | Trade route animation |
| 30 | `#emblems` | `drawEmblems` | Burg/province/state coats of arms |
| 31 | `#labels` | `drawLabels` | State (curved) + burg labels |
| 32 | `#icons` | `drawBurgIcons` | Settlement markers + anchors |
| 33 | `#armies` | `drawMilitary` | Regiment rectangles |
| 34 | `#markers` | `drawMarkers` | User markers |
| 35 | `#ruler` | `drawMeasurers` | Distance/area measurement |
| 36 | `#fogging` | — | State focus fog |
| 37 | `#vignette` | template | Dark vignette on the edges |
| 38 | `#scaleBar` | `drawScalebar` | Scale bar |
| 39 | `#legend` | — | Map legend |

> **Note**: There is no explicit "land fill". Land is filled indirectly through the active layer (heightmap, biomes, states, cultures, etc.).

---

## 2. Base: Features (land, ocean, coastline, lakes)

**File**: `src/renderers/draw-features.ts:20-74` — `featuresRenderer()`  
**File**: `src/renderers/draw-features.ts:76-87` — `featurePathRenderer(feature)`

### Data it consumes
- `pack.features[]` — array of Feature with `type` (landmass, sea_island, lake_island, lake), `vertices[]`, `group`, `i`
- `pack.vertices.p` — vertex coordinates

### Algorithm
```
1. Para cada feature no-oceánico:
   a. Mapear vértices → coordenadas (pack.vertices.p[vertex])
   b. Simplificar con Ramer-Douglas-Peucker (tolerancia 0.3px)
   c. Recortar a límites del mapa (clipPoly)
   d. Fractalizar costa (midpoint displacement)
   e. Generar path SVG (B-spline + Catmull-Rom mixto)
   f. Guardar como <path id="feature_N">
2. Máscaras:
   - #deftemp > #land: relleno blanco para tierra, negro para lagos → <mask id="land">
   - #deftemp > #water: relleno negro para tierra, blanco para lagos → <mask id="water">
3. Línea de costa (#coastline):
   - <g id="sea_island">: features tipo sea_island
   - <g id="lake_island">: features tipo lake_island
4. Lagos (#lakes): agrupados por feature.group
```

### `buildCoastlinePath()` — Building the path (`coastline-fractal.ts:194-252`)
- **Smooth spans** (no subdivision between original vertices): B-spline Q midpoint (`curveBasisClosed`)
- **Jagged spans** (subdivided by fractal): centripetal Catmull-Rom (α=0.5) for each sub-point

---

## 3. Coastline fractalization

**File**: `src/renderers/coastline-fractal.ts`

### Parameters
```typescript
const defaultCoastSettings = {
  enabled: true,
  maxDepth: 4,
  baseAmplitude: 1.5,
  amplitudeDecay: 0.9,
  minEdge: 1,
  smoothThreshold: 0.25,
  roughnessContrast: 1.5,
  profileHarmonics: 4,
  lakeSmoothThreshMult: 2.0
};
```

### Algorithm
1. **Deterministic PRNG**: `Alea(seed + "_c" + featureIndex)` for each feature
2. **Roughness profile**: Sum of harmonic cosines (seam-free, `PROFILE_SIZE=256`)
3. **Recursive midpoint displacement**: `subdivideEdge()` displaces midpoints perpendicularly by `(rand()-0.5) * sqrt(edgeLength) * amplitude * roughness`
4. Lakes receive a smoother profile (`smoothThreshold * 2.0`)
5. Edges on the map boundary are never subdivided

### Visual effect
The combination of B-spline (smooth spans) + Catmull-Rom (fractal spans) produces fluid coastlines that hide the angularity of the Voronoi tessellation while retaining detail in rough areas.

---

## 4. Ocean: base and bathymetric

### 4a. Base ocean
**File**: `index.html:326-328` — SVG `<pattern id="oceanic">`

- Base SVG fill: `#466eab`
- Optional pattern: overlaid image (6 options + Kiwiroo)
- Style controls: configurable hex color, pattern opacity (0-1)

### 4b. Bathymetric layers
**File**: `src/renderers/ocean-layers.ts:67-109` — `OceanModule.draw()`

Draws concentric contour rings in the ocean at depth levels:
```typescript
const outline = this.oceanLayers.attr("layers");
// Valores: "none" | "random" | "-6,-3,-1" (default) | "-9,-6,-3,-1" | etc.
```

**Algorithm**:
1. Parse the `layers` attribute for depth limits (e.g. `-6, -3, -1`)
2. For each limit `t`, find cells where `grid.cells.t[i] === t`
3. Walk vertices with `connectVertices()` → closed contour chains
4. **Relax**: keep every Nth point (`N = 1 + abs(t) * -2`)
5. Clip to map bounds
6. Render as SVG `<path>` with fill `#ecf2f9` and `fill-opacity = 0.4 / numLimits`

**Presets**: "No outline", "Random", "Standard 3" (-6,-3,-1), "Indented 3" (-6,-4,-2), "Smooth 6", "Smooth 9"

---

## 5. Shared isoline engine

**File**: `src/utils/pathUtils.ts:84-177` — `getIsolines()`

**This is the most critical rendering algorithm**. It generates SVG fill paths for any cell-indexed attribute. All regional color layers (biomes, cultures, religions, states, provinces) share this same engine.

### Algorithm
```
1. Iterar sobre todas las celdas; saltar celdas ya procesadas o tipo null
2. Para cada celda no procesada de tipo T:
   a. Encontrar un vecino de tipo diferente
   b. Saltar si es un lago interno (todos los vecinos de costa son mismo tipo)
   c. Encontrar vértice inicial en el borde entre T y no-T
   d. Caminar vértices con connectVertices() → polígono cerrado
3. Para cada cadena de vértices, generar hasta 4 formatos de path:
   - fill: "M x0,y0 L x1,y1 ... Z" — polígono de relleno sólido
   - waterGap: path con discontinuidades en vértices de costa → evita que el color se desborde al océano
   - halo: path con discontinuidades en bordes del mapa → para efectos de glow
   - polygons: arrays de coordenadas crudos (para cálculo de polo de inaccesibilidad)
```

### `connectVertices()` — The fundamental vertex walker
```typescript
function connectVertices({ vertices, startingVertex, ofSameType, addToChecked, closeRing }) {
  // Sigue la teselación Voronoi: cada vértice conecta 3 celdas
  // En cada vértice, revisa qué celdas adyacentes son "del mismo tipo"
  // Sigue la arista entre celdas del mismo tipo y diferente tipo
  // Termina cuando vuelve al startingVertex
}
```

**This walker is the foundation of**:
- `getIsolines()` → all regional fills
- `drawBorders()` → state/province borders
- `drawHeightmap()` → contour lines
- `OceanLayers.draw()` → bathymetric contours
- `drawTemperature()` → isotherm lines

---

## 6. Heightmap (elevation contours)

**File**: `src/renderers/draw-heightmap.ts:51-196` — `heightmapRenderer()`

### Data it consumes
- `grid.cells.h` — heights (0-100, ≥20 = land)
- `grid.vertices` — vertex positions and connectivity
- Two SVG groups: `#oceanHeights` and `#landHeights` inside `#terrs`

### Algorithm
```
1. Ordenar todas las celdas por altura ascendente
2. Contornos de océano (alturas 0-19):
   - Caminar líneas de contorno en cada nivel, saltando por skip
3. Contornos de tierra (alturas 20-100):
   - Caminar líneas de contorno desde altura 20
   - Saltar por skip configurable
4. Para cada banda de altura: usar connectVertices()
5. Simplificar saltando cada N-ésimo vértice
6. Renderizar:
   - height === 0: rect fill océano base, color = scheme(1.0)
   - height === 20: rect fill tierra base, color = scheme(0.8)
   - Para cada height con path: fill = scheme(1 - height/100)
   - Si terracing activo: translate(0.7, 1.4) + fill más oscuro
```

### Color schemes (`style.js`)
Presets: bright (Spectral), light (RdYlGn), natural, green, olive, livid, monochrome, or custom comma-separated hex stops.

### Style controls
- Render ocean heights toggle
- Terracing power (0=off)
- Reduce layers (skip)
- Simplify line
- Line style (curveBasisClosed, linear, step)
- Color scheme

---

## 7. Biomes

**File**: `public/modules/ui/layers.js:302-316` — `drawBiomes()`

### Data it consumes
- `pack.cells.biome[]` — biome ID per cell
- `biomesData.color[]` — color per biome ID
- `biomesData.i[]` — array of biome indices

### Algorithm
```typescript
const isolines = getIsolines(pack, cellId => cells.biome[cellId], { fill: true, waterGap: true });
Object.entries(isolines).forEach(([index, { fill, waterGap }]) => {
  const color = biomesData.color[index];
  bodyPaths.push(getGappedFillPaths("biome", fill, waterGap, color, index));
});
```

Uses the isoline engine with `{fill: true, waterGap: true}`. `getGappedFillPaths()` generates:
- `<path d="{fill}" fill="{color}" id="biome{index}">` — fill
- `<path d="{waterGap}" fill="none" stroke="{color}" stroke-width="3" id="biome-gap{index}">` — edge against water

---

## 8. Cultures

**File**: `public/modules/ui/layers.js:480-494` — `drawCultures()`

### Data it consumes
- `pack.cells.culture[]` — culture ID per cell
- `pack.cultures[].color` — color per culture

### Algorithm
Identical to biomes: `getIsolines(pack, cellId => cells.culture[cellId], { fill: true, waterGap: true })`, then fill with `cultures[index].color`.

---

## 9. Religions

**File**: `public/modules/ui/layers.js:509-523` — `drawReligions()`

### Data it consumes
- `pack.cells.religion[]` — religion ID per cell
- `pack.religions[].color` — color per religion

### Algorithm
Identical to cultures: `getIsolines(pack, cellId => cells.religion[cellId], { fill: true, waterGap: true })`.

---

## 10. States

**File**: `public/modules/ui/layers.js:537-566` — `drawStates()`

### Data it consumes
- `pack.cells.state[]` — state ID per cell
- `pack.states[].color` — color per state

### Algorithm
```typescript
const isolines = getIsolines(pack, cellId => cells.state[cellId], { fill: true, waterGap: true, halo: renderHalo });
Object.entries(isolines).forEach(([index, { fill, waterGap, halo }]) => {
  bodyPaths.push(getGappedFillPaths("state", fill, waterGap, color, index));
  if (renderHalo) {
    haloPaths.push(`<path id="state-border${index}" d="${halo}" clip-path="url(#state-clip${index})" stroke="${darkerColor}"/>`);
  }
});
```

- **Water gap**: stroke where the state border touches water
- **Halo**: when shape-rendering is "geometricPrecision", renders a blurred border beneath the state (group `#statesHalo` with `filter="blur(5px)"`)

---

## 11. Provinces

**File**: `public/modules/ui/layers.js:592-617` — `drawProvinces()`

### Data it consumes
- `pack.cells.province[]` — province ID per cell
- `pack.provinces[].color` — color
- `pack.provinces[].pole` or `pack.cells.p[province.center]` — label position

### Algorithm
- Uses the isoline engine with `{fill: true, waterGap: true}`
- Renders province labels as SVG `<text>` in the `#provinceLabels` group
- Labels are placed at the `pole` (pole of inaccessibility) or at the central cell

---

## 12. Borders

**File**: `src/renderers/draw-borders.ts:7-165` — `bordersRenderer()`

### Data it consumes
- `pack.cells.state[]`, `pack.cells.province[]`
- `pack.cells.h[]` (≥20 = land)
- `pack.cells.v[]`, `pack.cells.c[]`
- `pack.vertices.c[]`, `pack.vertices.v[]`, `pack.vertices.p[]`

### Algorithm (does NOT use the isoline engine — it is a specialized line renderer)
```
1. Para cada celda con provincia:
   a. Si algún vecino tiene provincia diferente pero mismo estado → borde provincial
   b. Caminar vértices con getVerticesLine() → path de borde
2. Para cada celda con estado:
   a. Si algún vecino de tierra (h≥20) tiene estado diferente → borde estatal
3. getVerticesLine():
   - Dos pasadas: primera encuentra arista de borde, segunda traza el borde completo
   - Sigue aristas entre celdas de diferente tipo
   - Usa vertices.v[current] (3 vértices vecinos) para determinar el próximo paso
```

### Styles (`auto-update.js:41-52`)
| Border | Opacity | Stroke | Width | Dasharray |
|-------|----------|--------|-------|-----------|
| State | 0.8 | `#56566d` | 1 | `"2"` |
| Province | 0.8 | `#56566d` | 0.5 | `"1"` |

---

## 13. Rivers

**See also**: `docs/analisis-rios-azgaar.md` (exhaustive 977-line analysis)

### Files
- `public/modules/ui/layers.js:810-831` — `drawRivers()`
- `src/generators/river-generator.ts:425-456` — `RiverModule.getRiverPath()`
- `src/generators/river-generator.ts:369-387` — `RiverModule.addMeandering()`
- `src/generators/river-generator.ts:400-418` — `RiverModule.getOffset()`

### Drawing algorithm
1. **Meandering**: `addMeandering()` interpolates between cell centers with meander factor 0.5 (more meander upstream, less in water: `WATER_MEANDER_SCALE = 0.25`)
2. **Path generation**: For each point, computes an offset (width) based on flow + position → two banks (left bank, right bank)
3. **Offset/Width**: `FLUX_FACTOR = 500`, `MAX_FLUX_WIDTH = 1`, `LENGTH_STEP_WIDTH = 1/200`, Fibonacci progression
4. **Rendering**: closed SVG polygon with `curveCatmullRom.alpha(0.1)` on both banks

---

## 14. Routes (roads, trails, sea routes)

**File**: `public/modules/ui/layers.js:845-862` — `drawRoutes()`
**File**: `src/generators/routes-generator.ts:867-873` — `RoutesModule.getPath()`

### Data it consumes
- `pack.routes[]` — array with `i`, `group` ("roads"|"trails"|"searoutes"), `points[]`

### Algorithm
```typescript
getPath({ group, points }) {
  const curve = {
    roads: curveCatmullRom.alpha(0.1),
    trails: curveCatmullRom.alpha(0.1),
    searoutes: curveCatmullRom.alpha(0.5)
  };
  return round(lineGen(points.map(p => [p[0], p[1]])));
}
```

Rendered as SVG `<path>` with `fill="none"`, grouped by type in `#roads`, `#trails`, `#searoutes`.

---

## 15. Relief icons

**File**: `src/renderers/draw-relief-icons.ts:17-148` — `reliefIconsRenderer()`

### Data it consumes
- `pack.cells.h[]`, `pack.cells.r[]`, `pack.cells.biome[]`
- `grid.cells.temp[]` (for snowline)
- `biomesData.iconsDensity[]`, `biomesData.icons[][]`

### Algorithm
```
1. Para cada celda de tierra (h≥20) sin río:
   a. Lowlands (h < 50): biome icons via poisson-disc sampling
      - Densidad según biomesData.iconsDensity[biome]
      - Tipo: conifer, coniferSnow, swamp, cactus, deadTree
   b. Highlands (h ≥ 50): relief icons
      - h > 70 + temp < 0: montañas con nieve
      - h > 70: montañas
      - otherwise: colinas
      - Size escala con altura
2. Poisson-disc sampling evita solapamiento
3. Íconos ordenados por y + size (painter's algorithm)
4. Render: <use href="#relief-{type}-{variant}" x y width height>
```

### Key parameters
```typescript
const density = terrain.attr("density") || 0.4; // 0.3-0.8
const size = 2 * (terrain.attr("size") || 1); // 0.2-4.0
```

### Icon sets: "simple" (minimal), "gray", "colored" (more detailed)

---

## 16. Burgs (settlements)

**File**: `src/renderers/draw-burg-icons.ts:10-115` — `burgIconsRenderer()`

### Data it consumes
- `pack.burgs[]` — `i`, `x`, `y`, `group`, `port`, `removed`
- `options.burgs.groups[]` — ordered groups that define hierarchy

### Algorithm
1. Create icon groups ordered by hierarchy
2. For each burg: `<use href="#icon-{shape}" id="burg{i}" x="{x}" y="{y}">`
3. If `burg.port`: also an anchor icon at the same position
4. Shapes: circle, square, triangle, cross, star, circled, squared, star-circled, star-squared, and Watabou variants (capital, city, town, village, hamlet, fort, monastery, caravanserai, trade post)

---

## 17. State labels (curved text)

**File**: `src/renderers/draw-state-labels.ts:25-373` — `stateLabelsRenderer()`

### Algorithm (complex raycasting)
```
1. Desde el pole de cada estado, emitir rayos cada 9° hacia afuera
2. Para cada rayo, avanzar de 5px en 5px hasta salir del estado
3. Encontrar el mejor par de rayos (izquierdo + derecho) que:
   - Sean suficientemente largos para el nombre del estado
   - Preferir horizontales o casi-horizontales
   - Preferir ángulos obtusos (180° = recta = mejor)
   - Score = longitud × horizontalidad × curvatura
4. Conectar los dos endpoints a través del pole con curveNatural
5. SVG <text><textPath> a lo largo del path generado
6. Validación: bounding box dentro del estado (6 puntos de muestra rotados)
7. Fallback a nombre corto si no cabe el completo
```

### Key constants
```typescript
ANGLE_STEP = 9;        // grados entre rayos
LENGTH_START = 5;      // paso inicial
LENGTH_STEP = 5;       // incremento
LENGTH_MAX = 300;      // longitud máxima
```

---

## 18. Burg labels

**File**: `src/renderers/draw-burg-labels.ts:10-91` — `burgLabelsRenderer()`

Simple SVG `<text>` at the burg's coordinates with offset (`dx`, `dy` in em). Grouped by burg group, each with independent styling.

---

## 19. Temperature (isotherms)

**File**: `src/renderers/draw-temperature.ts:19-136` — `temperatureRenderer()`

### Algorithm
1. Compute min/max temperature and auto-determine step size
2. For each isotherm level, walk vertices → isolines
3. Relax: keep every 4th vertex (or edges)
4. Each band is filled with the Spectral scale: `fill = scheme(1 - (t - tMin) / delta)` where scheme = `interpolateSpectral`
5. Stroke: `darker(fill, 0.2)`
6. Labels at the top-center and bottom-center of each band

---

## 20. Precipitation

**File**: `public/modules/ui/layers.js:333-359` — `drawPrecipitation()`

### Algorithm
- For each land cell with precipitation data: circle centered on the cell
- Radius = `sqrt(prec/4) / cellsModifier`
- Appearance animation (800ms transition from r=0 to the computed radius)

---

## 21. Population

**File**: `public/modules/ui/layers.js:394-432` — `drawPopulation()`

### Algorithm
- **Rural**: Vertical lines from the cell center downward, length proportional to population
- **Urban**: Vertical lines from the burg center downward, length proportional to population × urbanization
- 2000ms transition animation

---

## 22. Ice layer

**File**: `src/renderers/draw-ice.ts:10-74` — `iceRenderer()`

### Data it consumes
- `pack.ice[]` — array of Ice with `type` ("glacier"|"iceberg"), `points`, `offset`

Rendered as an SVG `<polygon>` with the stored points. Individual glaciers/icebergs are redrawable via `redrawIceberg()` and `redrawGlacier()`.

---

## 23. Goods

**File**: `src/renderers/draw-goods.ts:31-168` — `drawGoods()`

Three sub-layers:
- **`goodsCells`**: Cell polygons colored by production type, opacity normalized to the global maximum
- **`goodsIcons`**: Good icons at cell level (SVG symbols) with optional circles
- **`goodsBurgs`**: Burg plates — rounded rectangles with top-3 produced goods + values

---

## 24. Markets

**File**: `src/renderers/draw-markets.ts:17-118` — `drawMarketsLayer()`

- Each market receives its influence zone (isoline polygon) filled with the market's color
- The central burg receives a circle with an emoji (default ⚖️)
- Hover highlight animation

---

## 25. Emblems (coats of arms)

**File**: `src/renderers/draw-emblems.ts:28-156` — `emblemsRenderer()`

- Burg, province and state coats of arms rendered as SVG `<use>`
- Sizes auto-calculated from the number of entities and map dimensions
- D3 force simulation (`forceCollide`) to separate overlapping emblems

---

## 26. Military

**File**: `src/renderers/draw-military.ts:13-169`

Regiments as colored rectangles with troop counts, unit icons and images. Colored by state with darker borders. Animated movement along paths.

---

## 27. Satellite texture (3D)

**File**: `src/renderers/draw-satellite-texture.ts:474-580` — `generateSatelliteTexture()`

**Completely separate render path** for the 3D view (Three.js/WebGL). Uses custom GLSL shaders on a fullscreen quad.

### Data it consumes
- Height with erosion + coastline data
- Climate data (temp, prec, height)
- Biomes per cell

### Fragment shader pipeline
1. **Input textures**: Climate (temp+128/R, prec/G, height/B), Biome (albedo RGB, density A)
2. **Per-pixel computations**:
   - **Slope** via central differences on the height field
   - **Dithering noise** (FBM 5-octave value noise)
   - **Biome albedo** with UV wobble for natural edge variation
   - **Rock** on steep slopes, stratum bands, cliff darkening
   - **Sand/gravel** on coastal beaches
   - **Snow** based on temperature + altitude
   - **Riparian darkening** near drains
   - **Cavity shading** (dark gullies, bright crests)
   - **Hillshade** (Swiss-relief style: warm NW sun, cool blue shadow)
   - **Aerial perspective** (highlands fading pale toward the sky)
   - **Water**: bathymetric shelf-to-abyss, climate-tinted lagoons, foam line
   - **Lakes**: by group (fresh/salt/sinkhole/dry/lava/frozen)
   - **Rivers**: deep teal channel, sediment banks, white water on slopes, ice in cold climates

---

## 28. Overlays: texture, grid, coordinates

### 28a. Decorative texture
**File**: `public/modules/ui/layers.js:783-796` — `drawTexture()`

```javascript
function drawTexture() {
  texture.append("image")
    .attr("preserveAspectRatio", "xMidYMid slice")
    .attr("x", x).attr("y", y)
    .attr("width", graphWidth - x).attr("height", graphHeight - y)
    .attr("href", href);
}
```

### 28b. Grid
**File**: `public/modules/ui/layers.js:632-659` — `drawGrid()`

- SVG pattern (`<pattern>`) for tiles: pointyHex, flatHex, square, square45deg, triangle
- Configurable scale, stroke, dash, shift

### 28c. Coordinates (graticule)
**File**: `public/modules/ui/layers.js:673-731` — `drawCoordinates()`

- Uses `d3.geoGraticule()` + `d3.geoEquirectangular()`
- Step adapts to zoom (possible steps: 0.5, 1, 2, 5, 10, 15, 30)
- N/S/E/W labels on the map edges

---

## 29. Key architectural patterns

### Pattern 1: Universal isoline engine
`getIsolines()` + `connectVertices()` are the **universal regional rendering engine**. Any cell-indexed attribute produces SVG fills with the same algorithm: group cells → walk Voronoi edges → generate closed polygons → optional water gaps.

### Pattern 2: Water Gap
Each region is rendered with an **additional stroke** (`waterGap`) of the same color as the fill. Where the regional border touches water or the map edge, the stroke is interrupted — this prevents colors from visually bleeding into the ocean.

### Pattern 3: Land/water masks
Two SVG masks are generated in `#deftemp`:
- `<mask id="land">`: white for land, black for lakes
- `<mask id="water">`: black for land, white for lakes
Used for clipping layers that must not extend outside the land or the water.

### Pattern 4: Coastline fractalization
Pipeline: RDP simplification (0.3px) → clip to bounds → fractal midpoint displacement (4 levels, per-feature seed) → hybrid B-spline/Catmull-Rom path.

### Pattern 5: No explicit land fill
There is no global "land fill". Instead:
- The base land layer is the heightmap's rect fill at height 20
- Thematic layers render **individual regional polygons** that collectively cover all the land
- The `#land` mask exists for clipping, not as a visual fill

---

## 30. Portability status to Voronia

| Layer | Current Voronia | Dependencies | Priority |
|------|---------------|-------------|-----------|
| **Features** (land/ocean/coastline) | ❌ Not implemented | Voronoi geometry, masks | **High** |
| **Coastline fractal** | ❌ Not implemented | Midpoint displacement, Catmull-Rom | **High** |
| **Ocean base** | ❌ Fixed color `[0.20, 0.45, 0.80]` | Configurable color, pattern | Medium |
| **Ocean bathymetric** | ❌ Not implemented | connectVertices(), contours | Low |
| **Heightmap contours** | ✅ Base mesh OK. Contours ❌ | connectVertices(), color schemes | Medium |
| **Biomes** | ❌ Not implemented | getIsolines(), waterGap | **High** |
| **Cultures** | ❌ Not implemented | getIsolines(), waterGap | **High** |
| **Religions** | ❌ Not implemented | getIsolines(), waterGap | **High** |
| **States** | ❌ Not implemented | getIsolines(), waterGap, halo | **High** |
| **Provinces** | ❌ Not implemented | getIsolines(), waterGap, labels | **High** |
| **Borders** | ❌ Not implemented | getVerticesLine(), dashed strokes | **High** |
| **Rivers** | ✅ Implemented in vor-sim + vor-render | — | ✅ Done |
| **Routes** | ❌ Not implemented | Catmull-Rom paths | Medium |
| **Relief icons** | ❌ Not implemented | Poisson-disc, sprites, biomes | Low |
| **Burg icons** | ❌ Not implemented | Sprites, hierarchy | Medium |
| **State labels** | ❌ Not implemented | Raycasting, curved textPath | Medium |
| **Burg labels** | ❌ Not implemented | SVG text, offset | Medium |
| **Temperature** | ❌ Not implemented | Isolines, Spectral scale | Low |
| **Precipitation** | ❌ Not implemented | Circles, radius per prec | Low |
| **Population** | ❌ Not implemented | Vertical bars | Low |
| **Ice** | ❌ Not implemented | Polygons | Low |
| **Goods** | ❌ Not implemented | Multiple sub-layers | Low |
| **Markets** | ❌ Not implemented | Isolines, emoji | Low |
| **Emblems** | ❌ Not implemented | D3 force, SVG use | Low |
| **Military** | ❌ Not implemented | Rectangles, animation | Low |
| **Satellite texture** | ❌ Not implemented | GLSL shaders, full pipeline | Very low |
| **Texture overlay** | ❌ Partial (texture load OK, blend no) | Fullscreen quad, blend modes | Low |
| **Grid overlay** | ❌ Not implemented | Patterns, line shader | Low |
| **Coordinates** | ❌ Not implemented | Geographic projection | Medium |

### Critical dependencies (implementation order)
1. **`connectVertices()`** + **`getIsolines()`** — prerequisite for ALL thematic layers (biomes, cultures, religions, states, provinces, borders, heightmap contours)
2. Land/water masks — clipping of thematic layers
3. Coastline fractal — visual quality of the coastline
4. Per-layer configurable color palette — Azgaar-style schemes
5. Water gap technique — so thematic layers do not bleed into the ocean

### Note on the isoline engine
Azgaar's `connectVertices()` is an algorithm that walks over the Voronoi tessellation following edges between cells of different types. **It does not require the Delaunay geometry** — it only needs:
- `cells.c[i]` — neighbors of each cell
- `cells.v[i]` — vertices of each cell
- `vertices.c[v]` — cells that share a vertex
- `vertices.v[v]` — neighbors of each vertex (3 in standard Voronoi)

All of this is already available in `vor-core` (PackCells).

---

> **Document generated from the analysis of the Azgaar FMG v1.135.2 source code**.
> For the rivers analysis, see `docs/analisis-rios-azgaar.md` (977 lines).
> For previous landmass layers documentation, see `docs/landmass-layers.md` (initial, less detailed analysis).
