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
- **Current state**: `vor-render/src/river.rs` — simple quads, fixed width per river, no meandering
- **Required improvement**: Implement Catmull-Rom, progressive width per segment
- Deferred to Phase 3 — the current implementation is functional

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
- **Option A (fast)**: Voronoi cell color by temperature via `build_pack_mesh()` with mapping `pack.cells.grid_id → grid.cells.temperature`. Spectral gradient: blue (−50) → red (+50).
- **Option B (isolines)**: Implement `connectVertices()` and marching-squares-like path walking on the Voronoi graph. Much more work.
- **MVP**: Option A — color pack cells according to the source grid's temperature.
- **Pipeline**: TriangleList
- **File**: `vor-render/src/temperature.rs`

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
- **Option A (cells)**: Color pack Voronoi cells according to the source grid's precipitation. More intense blue where it rains more.
- **Option B (circles)**: Render instanced circles. Requires an instancing shader or sprites — more work.
- **MVP**: Option A — `build_pack_mesh()` with blue intensity proportional to precipitation.
- **Pipeline**: TriangleList
- **File**: `vor-render/src/precipitation.rs`

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
Each ice element is rendered as `<polygon points="..." />`. No stroke on glaciers, thin stroke on icebergs.

### Portability to Voronia
- **Pipeline**: TriangleList
- **Method**: Tessellate `ice.vertices` as a closed polygon with lyon
- **Color**: `[0.93, 0.95, 0.99, 0.9]` — ice white
- **File**: `vor-render/src/ice_layer.rs`

---

## Portability summary

| Layer | Priority | Pipeline | Method | Data source |
|------|-----------|----------|--------|-----------------|
| Lakes | MVP | TriangleList | Tessellate feature perimeters | `pack.features[].lake_group` + `.perimeter_vertices` |
| Rivers | Improvement | TriangleList | Already implemented (quads) | `world.rivers[].cell_path` |
| Temperature | MVP | TriangleList | `build_pack_mesh()` color by temperature via grid_id | `grid.cells.temperature` |
| Precipitation | MVP | TriangleList | `build_pack_mesh()` color by precipitation via grid_id | `grid.cells.precipitation` |
| Ice | MVP | TriangleList | Tessellate `ice.vertices` as a polygon | `world.ice[].vertices` |
