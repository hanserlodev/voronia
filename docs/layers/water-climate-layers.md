# Azgaar Water & Climate Layers — Documentation and porting plan

> **Category**: Water & Climate
> **Layers**: lakes, rivers, temperature, precipitation, ice
> **Source**: Azgaar's FMP v1.135.2 — the local azgaar-fmg reference checkout

## 1. Lakes

### What it does in Azgaar
Renders filled lake polygons with a color per group (freshwater, salt, sinkhole, lava, dry). Lakes are **features** in `pack.features` (slot `[12]`), drawn by `drawFeatures()` in the SVG group `#lakes`.

### Data consumed
| Slot | Voronia Field | Type | Description |
|------|---------------|------|-------------|
| `[12]` | `pack.features` | `Vec<Feature>` | Features where `kind == Lake` |
| `[12].vertices` | `feature.perimeter_vertices` | `Vec<u32>` | Vertex IDs that form the perimeter |
| `pack.vertices.p` | `pack.vertices.positions` | `[[f32;2]]` | Coordinates of each vertex |
| `[12].group` | `feature.lake_group` | `Option<LakeGroup>` | Freshwater, Salt, Dry, Sinkhole, Lava |

### Implementation in Azgaar
`drawFeatures()` iterates over `pack.features` (excluding ocean and land features). For each feature `type === "lake"`:
1. `getFeaturePath()` resolves `feature.vertices` → coordinates → simplifies → applies fractal → builds an SVG `<path>`
2. The `<path>` is placed in the corresponding subgroup (`#freshwater`, `#salt`, etc.)
3. Toggle via CSS class `.hidden` on `#lakes`

### Color scheme (default.json)
| Lake Group | Fill | Stroke | Stroke Width | Opacity |
|---|---|---|---|---|
| Freshwater | `#a6c1fd` | `#5f799d` | 0.7 | 0.5 |
| Salt | `#409b8a` | `#388985` | 0.7 | 0.5 |
| Sinkhole | `#5bc9fd` | `#53a3b0` | 0.7 | 1.0 |
| Lava | `#90270d` | `#f93e0c` | 2 | 0.7 |
| Dry | `#c9bfa7` | `#8e816f` | 0.7 | 1.0 |

### Portability to Voronia
- **Pipeline**: TriangleList (mesh filled per feature, not per cell)
- **Method**: Tessellate each `feature.perimeter_vertices` → `positions` as a closed polygon with lyon
- **Color**: Based on `feature.lake_group`, direct sRGB values with alpha
- **Grid vs Pack**: Pack (features post-repack)
- **File**: `vor-render/src/lakes.rs`

### Estado actual (7 ago 2026)
✅ **Rellenos implementados** — `build_lake_mesh(pack, map_width, map_height, &FractalSettings)` (`lakes.rs`) tessella cada feature `Lake` como polígono cerrado con lyon (`FillTessellator`), coloreado por `lake_group` (Freshwater/Salt/Dry/Sinkhole/Lava) con su alfa.

🟡 **Fractalización de bordes — recién portada**. Ahora usa el **mismo pipeline fractal de las costas** que Azgaar aplica en `featurePathRenderer()` (`src/renderers/draw-features.ts`): `simplify(0.3)` → `clipPoly(1)` → `fractalizeCoastline(feature.id, feature.type)` → `buildCoastlinePath` → `Z` → fill lyon. La semilla por feature es `"{seed}_c{featureId}"` con `is_lake=true` (activa `lake_smooth_thresh_mult`). Pendiente: verificación visual de paridad de bordes.

---

## 2. Rivers

### What it does in Azgaar
Renders rivers as filled paths with variable width according to flow (flux). The rivers meander between pack cells.

### Data consumed
| Slot | Voronia Field | Type |
|------|---------------|------|
| `[32]` | `world.rivers` | `Vec<River>` |
| N/A | `river.cell_path` | `Vec<u32>` |
| `pack.points` | `pack.points` | `[[f32;2]]` |

### Width in Azgaar (`getOffset()`)
```
fluxWidth = min(flux^0.7 / 500, 1.0)
lengthWidth = pointIndex * (1/200) + LENGTH_PROGRESSION[pointIndex]
offset = widthFactor * (lengthWidth + fluxWidth) + startingWidth
riverWidth = (offset / 1.5)^1.8  // mouth width in km
```

### Meandering
`addMeandering()` interpolates 7 control points between each pair of consecutive cells, applying Catmull-Rom with random perturbation.

### Portability to Voronia
- **Pipeline**: TriangleList (triangle ribbon, already implemented)
- **Current state**: ✅ **~90%**. `vor-sim::river` es el port completo de `hydrology`/`width`/`meander`/`river_def`/`specify`/`river_def` (ver `references/status.md` «Port de ríos», 12 tests). El render en `vor-render/src/river.rs` genera un ribbon de triángulos con meandros (`vor-render::mesh`).
- **Required improvement** (10% restante): pulido del ribbon y paridad visual final contra Azgaar.

---

## 3. Temperature

### What it does in Azgaar
Renders temperature isolines (isotherms) filled with a Spectral gradient (blue=cold, red=hot). ~5 bands are generated between the min and max temperature.

### Data consumed
| Slot | Voronia Field | Type |
|------|---------------|------|
| `[11]` | `grid.cells.temperature` | `Vec<i8>` |
| `grid.points` | `grid.points` | `[[f32;2]]` |
| `options.temperatureEquator/Pole` | — | — |

### Isoline generation in Azgaar
1. `tMin = -50`, `tMax = 50`. The actual min/max is obtained from `grid.cells.temp`.
2. `step = max(round(|min - max| / 5), 1)`
3. For each step between `min + step` and `max`: walk the Voronoi vertex graph with `connectVertices()`, generate a closed chain of vertices at the temperature level.
4. Relax the chain (keep every 4th vertex).
5. Fill a `<path>` with the step's color.

### Color scheme
Uses `d3.interpolateSpectral` (diverging blue↔red):
```javascript
const scheme = scaleSequential(interpolateSpectral);
const fill = scheme(1 - (t - tMin) / delta);  // tMin=-50, tMax=50
```

### Portability to Voronia
- **Pipeline**: TriangleList
- **Method**: ✅ **Portada** — `build_temperature_mesh(&grid)` (`temperature.rs`) replica exactamente `draw-temperature.ts`: banda base = rect del mapa entero con el color de `minTemp`, `step = max(round(|maxTemp-minTemp|/5), 1)`, y por cada nivel `t ∊ [min+step, max)` camina el grafo de la grilla con `connect_vertices(&grid.vertices, start, |c| temp[c] >= t, ... , close_ring)`, relax 1-de-cada-4 + vértices de borde, y rellena con `scheme(1 - (t-tMin)/delta)`. El color usa la misma rampa espectral de heightmap (tMin=-50, delta=100).
- **File**: `vor-render/src/temperature.rs`
- **Pendiente**: verificación visual de paridad de los isolines vs FMG. Paridad a nivel fuente **verificada el 10 ago 2026** contra `draw-temperature.ts`, `pathUtils.ts`, d3 `basisClosed.js` y el CSS `#temperature` real (`fill-opacity:0.3`, `stroke-width:1.8`) — ver `docs/analysis/fmg-temperature-precipitation.md` §4 (divergencias corregidas: loop de `build_curve_basis_closed`, alpha del fill, stroke-width, `d3_range`, `fill-rule`, pipeline blended).

---

## 4. Precipitation

### What it does in Azgaar
Renders blue circles at the center of each land grid cell. The radius is proportional to `sqrt(precipitación / 4)`.

### Data consumed
| Slot | Voronia Field | Type |
|------|---------------|------|
| `[8]` | `grid.cells.precipitation` | `Vec<u16>` |
| `grid.points` | `grid.points` | `[[f32;2]]` |
| `grid.cells.height` | `grid.cells.height` | `Vec<u8>` |

### Rendering in Azgaar
```javascript
cells.i.filter(i => cells.h[i] >= 20 && cells.prec[i]).forEach(i => {
  const r = sqrt(prec / 4) / cellsNumberModifier;
  circles.push({ cx: grid.points[i][0], cy: grid.points[i][1], r });
});
```
Filtered to land cells (`height >= 20`) with non-zero precipitation.

### Portability to Voronia
- **Pipeline**: TriangleList
- **Method**: ✅ **Portada** — `build_precipitation_mesh(&grid)` (`precipitation.rs`) replica `drawPrecipitation`: por cada celda con `height >= 20 && prec > 0`, dibuja un círculo (polígono 24 segmentos) centrado en `grid.points[i]` con radio `rn(sqrt(prec/4)/cellsNumberModifier, 2)`, color `#003dff`. El `cellsNumberModifier = (cells/10000)⁰·²⁵` usa `grid.cells_desired`.
- **File**: `vor-render/src/precipitation.rs`
- **Pendiente**: verificación visual de paridad de tamaños de círculo vs FMG. CSS `#prec` verificado contra `public/styles/default.json` (10 ago 2026): `fill:#003dff`, `stroke-width:0` → el port (fill sin stroke) ya coincide.

---

## 5. Ice

### What it does in Azgaar
Renders two types of ice as polygons:
- **Glaciers**: large polygons on land (high altitude + cold temperature)
- **Icebergs**: small polygons in cold water

### Data consumed
| Slot | Voronia Field | Type |
|------|---------------|------|
| `[39]` | `world.ice` | `Vec<Ice>` |
| — | `ice.vertices` | `Vec<[f32;2]>` |
| — | `ice.kind` | `IceKind::{Glacier,Iceberg}` |

### Rendering in Azgaar
Each ice element is rendered as `<polygon points="..." />`. The `#ice` layer CSS (default.json): `fill:#f1f8fe`, `stroke:#e8f0f6`, `stroke-width:0.5`, `opacity:0.9`, `filter:url(#dropShadow01)`. Icebergs can carry a runtime `offset` (translate transform) for the drift animation; saved maps may omit it.

### Portability to Voronia
- **Pipeline**: TriangleList
- **Method**: Tessellate `ice.vertices` as a closed polygon with lyon
- **Color**: `[0.93, 0.95, 0.99, 0.9]` — ice white (aproximación del `#f1f8fe` con opacity 0.9; sin stroke aún)
- **File**: `vor-render/src/ice_layer.rs`

### Estado actual (10 ago 2026)
✅ **Implementado** — `build_ice_mesh(&world.ice)` (`ice_layer.rs`) tessella cada `Ice` como polígono cerrado con lyon y cableado en `vor-app` (`add_layer_mesh`). Está implementado aunque la doc anterior lo marcaba como "no implementado". **Pendiente de paridad CSS**: fill lineal real de `#f1f8fe`, stroke `#e8f0f6` 0.5, opacity 0.9 (exige pipeline blended) y el `dropShadow01` filter.

---

## Portability summary

> Actualizado: 10 ago 2026. Leyenda: ✅ completo/avanzado · 🟡 a medias · ❌ sin implementar.

| Layer | Priority | State | Method | Data source |
|------|-----------|----------|--------|-----------------|
| Lakes | MVP | ✅ Rellenos + bordes fractálizados (pipeline costas) implementados; pendiente verificación visual | Tessellate feature perimeters | `pack.features[].lake_group` + `.perimeter_vertices` |
| Rivers | Improvement | ✅ 90% — port completo de hidrología/meander/witdh | Ribbon de triángulos (`vor-render/src/river.rs`) | `world.rivers[].cell_path` |
| Temperature | MVP | ✅ Portada (`build_temperature_mesh`) + **paridad verificada contra fuentes reales (10 ago 2026)**: CSS `fill-opacity:0.3`/`stroke-width:1.8`, `curveBasisClosed` exacto, `d3.range`, pipeline blended | Bandas de isolines rellenas sobre la grilla | `grid.cells.temperature` |
| Precipitation | MVP | ✅ Portada (`build_precipitation_mesh`), CSS `#prec` verificado (`fill:#003dff`, stroke-width 0); pendiente verificación visual de tamaños | Círculos `#003dff` en celdas tierra 20+ prec 0, radio por densidad | `grid.cells.precipitation` |
| Ice | MVP | ✅ Implementado (`build_ice_mesh`); **pendiente paridad CSS** (fill `#f1f8fe`, stroke `#e8f0f6` 0.5, opacity 0.9, blended) | Tessellate `ice.vertices` as a polygon | `world.ice[].vertices` |
