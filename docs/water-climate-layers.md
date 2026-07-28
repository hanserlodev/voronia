# Azgaar Water & Climate Layers — Documentación y plan de portabilidad

> **Categoría**: Water & Climate
> **Capas**: lakes, rivers, temperature, precipitation, ice
> **Fuente**: Azgaar's FMP v1.135.2 — `/home/hans/Proyectos/azgaar-fmg/`

## 1. Lakes

### Qué hace en Azgaar
Renderiza polígonos de lagos rellenos con color por grupo (freshwater, salt, sinkhole, lava, dry). Los lagos son **features** en `pack.features` (slot `[12]`), dibujados por `drawFeatures()` en el grupo SVG `#lakes`.

### Datos que consume
| Slot | Campo Voronia | Tipo | Descripción |
|------|---------------|------|-------------|
| `[12]` | `pack.features` | `Vec<Feature>` | Features donde `kind == Lake` |
| `[12].vertices` | `feature.perimeter_vertices` | `Vec<u32>` | IDs de vértices que forman el perímetro |
| `pack.vertices.p` | `pack.vertices.positions` | `[[f32;2]]` | Coordenadas de cada vértice |
| `[12].group` | `feature.lake_group` | `Option<LakeGroup>` | Freshwater, Salt, Dry, Sinkhole, Lava |

### Implementación en Azgaar
`drawFeatures()` itera `pack.features` (excluyendo océano y features de tierra). Para cada feature `type === "lake"`:
1. `getFeaturePath()` resuelve `feature.vertices` → coordenadas → simplifica → aplica fractal → construye SVG `<path>`
2. El `<path>` se coloca en el subgrupo correspondiente (`#freshwater`, `#salt`, etc.)
3. Toggle via clase CSS `.hidden` en `#lakes`

### Esquema de color (default.json)
| Lake Group | Fill | Stroke | Stroke Width | Opacity |
|---|---|---|---|---|
| Freshwater | `#a6c1fd` | `#5f799d` | 0.7 | 0.5 |
| Salt | `#409b8a` | `#388985` | 0.7 | 0.5 |
| Sinkhole | `#5bc9fd` | `#53a3b0` | 0.7 | 1.0 |
| Lava | `#90270d` | `#f93e0c` | 2 | 0.7 |
| Dry | `#c9bfa7` | `#8e816f` | 0.7 | 1.0 |

### Portabilidad a Voronia
- **Pipeline**: TriangleList (malla rellena por feature, no por celda)
- **Método**: Tessellar cada `feature.perimeter_vertices` → `positions` como polígono cerrado con lyon
- **Color**: Según `feature.lake_group`, valores sRGB directos con alpha
- **Grid vs Pack**: Pack (features post-repack)
- **Archivo**: `vor-render/src/lakes.rs`

---

## 2. Rivers

### Qué hace en Azgaar
Renderiza ríos como paths rellenos con ancho variable según caudal (flux). Los ríos meandrean entre celdas del pack.

### Datos que consume
| Slot | Campo Voronia | Tipo |
|------|---------------|------|
| `[32]` | `world.rivers` | `Vec<River>` |
| N/A | `river.cell_path` | `Vec<u32>` |
| `pack.points` | `pack.points` | `[[f32;2]]` |

### Ancho en Azgaar (`getOffset()`)
```
fluxWidth = min(flux^0.7 / 500, 1.0)
lengthWidth = pointIndex * (1/200) + LENGTH_PROGRESSION[pointIndex]
offset = widthFactor * (lengthWidth + fluxWidth) + startingWidth
riverWidth = (offset / 1.5)^1.8  // mouth width in km
```

### Meandering
`addMeandering()` interpola 7 puntos de control entre cada par de celdas consecutivas, aplicando Catmull-Rom con pertubación aleatoria.

### Portabilidad a Voronia
- **Pipeline**: TriangleList (ribbon de triángulos, ya implementado)
- **Estado actual**: `vor-render/src/river.rs` — quads simples, ancho fijo por río, sin meandering
- **Mejora necesaria**: Implementar Catmull-Rom, ancho progresivo por segmento
- Diferido a Fase 3 — la implementación actual es funcional

---

## 3. Temperature

### Qué hace en Azgaar
Renderiza isolíneas de temperatura (isotermas) rellenas con gradiente Spectral (azul=frio, rojo=calor). Se generan ~5 bandas entre min y max de temperatura.

### Datos que consume
| Slot | Campo Voronia | Tipo |
|------|---------------|------|
| `[11]` | `grid.cells.temperature` | `Vec<i8>` |
| `grid.points` | `grid.points` | `[[f32;2]]` |
| `options.temperatureEquator/Pole` | — | — |

### Generación de isolíneas en Azgaar
1. `tMin = -50`, `tMax = 50`. Se obtiene min/max real de `grid.cells.temp`.
2. `step = max(round(|min - max| / 5), 1)`
3. Para cada step entre `min + step` y `max`: caminar el grafo de vértices Voronoi con `connectVertices()`, generar cadena cerrada de vértices al nivel de temperatura.
4. Relajar la cadena (keep every 4th vertex).
5. Rellenar un `<path>` con el color del step.

### Esquema de color
Usa `d3.interpolateSpectral` (divergente azul↔rojo):
```javascript
const scheme = scaleSequential(interpolateSpectral);
const fill = scheme(1 - (t - tMin) / delta);  // tMin=-50, tMax=50
```

### Portabilidad a Voronia
- **Opción A (rápida)**: Color de celda Voronoi por temperatura vía `build_pack_mesh()` con mapeo `pack.cells.grid_id → grid.cells.temperature`. Gradiente Spectral: azul (−50) → rojo (+50).
- **Opción B (aislínas)**: Implementar `connectVertices()` y marching-squares-like path walking en el grafo Voronoi. Mucho más trabajo.
- **MVP**: Opción A — colorear celdas del pack según temperatura del grid de origen.
- **Pipeline**: TriangleList
- **Archivo**: `vor-render/src/temperature.rs`

---

## 4. Precipitation

### Qué hace en Azgaar
Renderiza círculos azules en el centro de cada celda de tierra del grid. El radio es proporcional a `sqrt(precipitación / 4)`.

### Datos que consume
| Slot | Campo Voronia | Tipo |
|------|---------------|------|
| `[8]` | `grid.cells.precipitation` | `Vec<u16>` |
| `grid.points` | `grid.points` | `[[f32;2]]` |
| `grid.cells.height` | `grid.cells.height` | `Vec<u8>` |

### Render en Azgaar
```javascript
cells.i.filter(i => cells.h[i] >= 20 && cells.prec[i]).forEach(i => {
  const r = sqrt(prec / 4) / cellsNumberModifier;
  circles.push({ cx: grid.points[i][0], cy: grid.points[i][1], r });
});
```
Filtrado a celdas de tierra (`height >= 20`) con precipitación no nula.

### Portabilidad a Voronia
- **Opción A (celdas)**: Colorear celdas Voronoi del pack según precipitación del grid de origen. Azul más intenso donde más llueve.
- **Opción B (círculos)**: Renderizar círculos instanciados. Requiere shader de instancias o sprites — más trabajo.
- **MVP**: Opción A — `build_pack_mesh()` con intensidad azul proporcional a precipitación.
- **Pipeline**: TriangleList
- **Archivo**: `vor-render/src/precipitation.rs`

---

## 5. Ice

### Qué hace en Azgaar
Renderiza dos tipos de hielo como polígonos:
- **Glaciares**: polígonos grandes en tierra (alta altura + temperatura fría)
- **Icebergs**: polígonos pequeños en agua fría

### Datos que consume
| Slot | Campo Voronia | Tipo |
|------|---------------|------|
| `[39]` | `world.ice` | `Vec<Ice>` |
| — | `ice.vertices` | `Vec<[f32;2]>` |
| — | `ice.kind` | `IceKind::{Glacier,Iceberg}` |

### Render en Azgaar
Cada ice element se renderiza como `<polygon points="..." />`. Sin stroke en glaciares, stroke fino en icebergs.

### Portabilidad a Voronia
- **Pipeline**: TriangleList
- **Método**: Tessellar `ice.vertices` como polígono cerrado con lyon
- **Color**: `[0.93, 0.95, 0.99, 0.9]` — blanco hielo
- **Archivo**: `vor-render/src/ice_layer.rs`

---

## Resumen de portabilidad

| Capa | Prioridad | Pipeline | Método | Fuente de datos |
|------|-----------|----------|--------|-----------------|
| Lakes | MVP | TriangleList | Tessellar perímetros de features | `pack.features[].lake_group` + `.perimeter_vertices` |
| Rivers | Mejora | TriangleList | Ya implementado (quads) | `world.rivers[].cell_path` |
| Temperature | MVP | TriangleList | `build_pack_mesh()` color por temperatura vía grid_id | `grid.cells.temperature` |
| Precipitation | MVP | TriangleList | `build_pack_mesh()` color por precipitación vía grid_id | `grid.cells.precipitation` |
| Ice | MVP | TriangleList | Tessellar `ice.vertices` como polígono | `world.ice[].vertices` |
