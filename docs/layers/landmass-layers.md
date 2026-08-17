# Azgaar Landmass Layers — Documentation and porting plan

> **Category**: Landmass  
> **Layers**: texture, heightmap, relief, cells, grid, coordinates  
> **Source**: Azgaar's FMP v1.135.2 — the local azgaar-fmg reference checkout

---

## 1. Texture

### What it does in Azgaar
Overlays a raster texture image on top of the entire map (paper, parchment, cloth, etc.). It is an SVG `<image>` scaled to the size of the chart. It does not interact with cells or world data — it is purely decorative.

> **Z-order note**: The `#texture` group is inserted *before* `#landmass` in the SVG
> (`load.ts` `.insert("g", "#landmass")`), so it acts as the **canvas/paper
> background** underneath the ocean and land, not an overlay drawn on top of them.
> The ocean pages are drawn **translucent** (`ocean-layers.ts` uses a low
> `fill-opacity`) so the paper shows through the sea; the opaque continents stay
> above it.

### Source code
| File | Lines | Role |
|---|---|---|
| `layers.js` | 783-796 | `drawTexture()` — creates/appends an SVG `<image>` |
| `style.js` | 539-572 | Texture selector (9 options: none, folded-paper, gray-paper, etc.) + X/Y shift |
| `load.ts` | 347, 390-394 | Initializes the `#texture` group in SVG |
| `index.html` | 782-796 | Texture selector in the Style editor |

### Exact implementation (`layers.js:783-796`)
```javascript
function drawTexture() {
  const x = Number(texture.attr("data-x") || 0);
  const y = Number(texture.attr("data-y") || 0);
  const href = texture.attr("data-href");

  texture
    .append("image")
    .attr("preserveAspectRatio", "xMidYMid slice")
    .attr("x", x)
    .attr("y", y)
    .attr("width", graphWidth - x)
    .attr("height", graphHeight - y)
    .attr("href", href);
}
```

### Data consumed
- `data-href`: URL/path of the texture image
- `data-x`, `data-y`: texture offset (shift)
- `graphWidth`, `graphHeight`: SVG dimensions

### Portability to Voronia
**Approach**: Texture is drawn as the first thing of the map render pass (Pass 1),
on a fullscreen quad with an opaque `REPLACE` blend, *before* the ocean and the
land layers — so it is the paper the map is drawn on. The ocean quad uses a
dedicated alpha-blended pipeline (`ocean_pipeline`, see `renderer.rs`) with
`alpha < 1.0` so the paper shows through the sea, matching Azgaar.

| Stage | Description |
|---|---|
| **Current phase** | 🟡 **A medias / Implemented (partial)**. `TextureOverlay` (`texture.rs`) draws the texture at the start of Pass 1, MSAA-aware, `ClampToEdge` edge (slice-to-fit). Ocean is translucent (`[0.16, 0.35, 0.66, 0.55]` in `lib.rs`). |
| **Implementation** | World-anchored textured quad (full world rect, transformed by the camera matrix) so the paper pans/zooms *with* the map, like Azgaar's `#texture` image inside the `#viewbox`. Loads PNG/JPG as `wgpu::Texture`, opaque `REPLACE` blend, placed before `draw_ocean`. |
| **Pending** | X/Y shift controls (`data-x`/`data-y` → texture offset, Azgaar `texture.attr("data-x")`). The quad currently always covers the whole world rect with no offset. |

---

## 2. Heightmap

### What it does in Azgaar
Renders altitude isolines (contours) colored by a configurable color scheme. Separates ocean (height < 20) from land (height >= 20). Each altitude level is a closed SVG `<path>` forming elevation bands. Supports terracing (parallel shadow on each contour).

### Source code
| File | Lines | Role |
|---|---|---|
| `draw-heightmap.ts` | 1-198 | **Complete TS implementation** |
| `style.js` | 43-70 | Color schemes (`heightmapColorSchemes`) |
| `layers.js` | 263-276 | `toggleHeight()` + refresh in `drawLayers()` |
| `load.ts` | 348 | Initializes `#terrs` with subgroups `#oceanHeights` and `#landHeights` |

### Algorithm (`draw-heightmap.ts`)
```
1. Limpiar grupos #oceanHeights y #landHeights
2. Ordenar celdas por altura asc
3. Para cada altura h (0-100):
   a. Saltar si no alcanza el skip configurado
   b. Encontrar celdas en el borde de ese nivel de altura (células con vecinos más bajos)
   c. Para cada celda borde, caminar vértices formando una cadena cerrada (connectVertices)
   d. Simplificar cadena (simplifyLine)
   e. Generar path SVG con línea curva (configurable: basis, cardinal, catmullRom, etc.)
4. Render paths:
   - h=0: rectángulo base océano con color scheme(1) 
   - h=20: rectángulo base tierra con color scheme(0.8)
   - Para cada h con path: dibujar path relleno con color del scheme + terracing opcional
```

### Key function `connectVertices()` (`draw-heightmap.ts:161-187`)
```
Input: cells, vertices, start_vertex, h, used[]
Output: chain (lista de vértices formando contorno cerrado)

- Desde start_vertex, camina por el grafo de vértices
- En cada paso: elige el vecino que cruza una celda en el lado opuesto del contorno
- Marca celdas como "used" para no reprocesarlas
- Termina cuando vuelve a start_vertex
- Máximo 100K iteraciones (seguro contra loops infinitos)
```

### Color schemes (`style.js:43-70`)
Azgaar has multiple schemes: elevation, wiki, grayscale, wiki2, elevation2, fancy, wiki3, palettes. Each scheme is a function `(t: number) => string` that maps normalized altitude (0-1) to a CSS color.

### Portability to Voronia
**Approach**: Voronia renders the heightmap as **filled isoline bands** (one tessellated polygon per height level, drawn low → high), matching Azgaar's discrete faceted look — not a continuous per-cell gradient.

| Stage | Description |
|---|---|
| **Current phase** | ✅ `build_heightmap_band_mesh()` (`isoline.rs`) fills one polygon per height level. |
| **Color scheme** | ✅ Spectral/"bright" ramp (`SPECTRAL_STOPS` in `heightmap.rs`), interpolating to linear RGBA for the shader. |
| **Ocean** | ✅ Ocean excluded (`height < 20`), matching Azgaar `#oceanHeights data-render = 0`. |
| **Implementation** | `build_heightmap_band_mesh` iterates `h = 20, 26, ..., 100` (`BAND_STEP = 6`, mirroring `#landHeights` `skip: 5` → `currentLayer += skip + 1`), extracts each contour with `get_isolines` (option `polygons: true`) and fills it with `height_color(h)`. |
| **Pending** | Configurable color schemes (currently fixed to "bright"). Terracing. Contour simplification/curving options. Band step currently hardcoded to the Azgaar default. |

> Algoritmo del facetado: cada banda `h` pinta la región donde `height >= h`, así que
> los niveles superiores cubren a los inferiores y producen los anillos concéntricos
> discretos de Azgaar (el mar queda fuera).

### Estado en Voronia (documentado 7 ago 2026)
✅ **Completo**. `build_heightmap_band_mesh()` (`isoline.rs`) rellena un polígono por nivel de altura con `BAND_STEP = 6` sobre el esquema Spectral/"bright" (`SPECTRAL_STOPS` en `heightmap.rs`), océano excluido (`height < 20`). El tooltip de celda reporta la altura en metros reales (`Settings::height_m`, port de `getHeight` de Azgaar). Opcional: schemes configurables y terracing — mejorable, no bloqueante.

---

## 3. Relief

### What it does in Azgaar
Draws SVG relief icons (mountains, hills, trees) over the map using Poisson-disc sampling. Each cell with altitude < 50 receives biome icons (trees); cells with altitude >= 50 receive relief icons (mountains/hills). Icons are sorted by Z (y + size). Supports multiple icon sets (simple, detailed, 3d).

### Source code
| File | Lines | Role |
|---|---|---|
| `draw-relief-icons.ts` | 1-150 | Complete TS implementation |
| `index.html` | 2942-3427 | SVG icon definitions (`<symbol>`) |
| `layers.js` | 746-757 | `toggleRelief()` |
| `style.js` | 788-800 | Re-draw on preset/attribute changes |

### Algorithm (`draw-relief-icons.ts:17-91`)
```
1. Leer densidad y tamaño desde atributos del grupo #terrain
2. Para cada celda i:
   a. Si height < 50: biome icons (poisson-disc sampling)
      - Coníferas, deciduos, palmeras, cactus, etc. según bioma
   b. Si height >= 50: relief icons (poisson-disc sampling)
      - Tipo: mount, hill, cliff según altura y temperatura
3. Ordenar todos los íconos por (y + size) → z-order
4. Renderizar como <use href="#relief-{tipo}">
```

### Poisson-disc sampling (`draw-relief-icons.ts:41-58`)
```javascript
function placeBiomeIcons(cellIndex, density, size) {
  const attempts = 30;
  const minDist = 1 / density;
  for (let attempt = 0; attempt < attempts; attempt++) {
    const x = random(cell.x - cell.w/2, cell.x + cell.w/2);
    const y = random(cell.y - cell.h/2, cell.y + cell.h/2);
    // check distance to existing points
    if (tooClose(x, y, minDist)) continue;
    icons.push({ x, y, type: getBiomeIcon(cell), size });
  }
}
```

### Available icon sets
| Set | Source |
|---|---|
| simple | Basic lines (triangle for mountain, circle for tree) |
| detailed | More elaborate SVGs with shadows |
| 3d | 3D perspective |

### Portability to Voronia
**Approach**: Icons as glyph instances (wgpu indirect draw) or textured sprites. For low complexity: colored points on screen (circle = tree, triangle = mountain).

| Stage | Description |
|---|---|
| **Current phase** | 🟡 **A medias / Implemented (partial)**. `build_relief_mesh()` (`relief.rs`) genera triángulos por celda con `height >= 40`; 3 niveles de tamaño/color según `h >= 80 / 60 / 40` (montaña, colina, tierras altas). Se sube como capa mesh en `lib.rs` (renderer.add_layer_mesh). |
| **Pending (MVW → Full)** | Va del placeholder geométrico a los relief icons de Azgaar: Poisson-disc sampling dentro de cada celda (no un solo triángulo en el centro), `placeBiomeIcons()` para `height < 50` (coníferas, deciduos, palmeras… según bioma), tipo `mount/hill/cliff` según altura+temperature, z-order (y + size), y los 3 sets de iconos (simple/detailed/3d). |

---

## 4. Cells

### What it does in Azgaar
Renders the cell (Voronoi) polygons as border lines. Essentially a wireframe of the polygonal mesh. Useful for debug and editing (river/burg/route editors activate it automatically). Supports both grid (pre-pack) and pack cells depending on the customization mode.

### Source code
| File | Lines | Role |
|---|---|---|
| `layers.js` | 446-451 | `drawCells()` — polyline path of cell borders |
| `layers.js` | 434-444 | `toggleCells()` |
| Multiple editors | — | Activate `toggleCells()` when editing |

### Exact implementation (`layers.js:446-451`)
```javascript
function drawCells() {
  const cells = customization === 1 ? grid.cells.i : pack.cells.i;
  const polygon = customization === 1 ? getGridPolygon : getPackPolygon;
  const paths = Array.from(cells).map(i => "M" + polygon(i));
  ensureEl("cells").innerHTML = `<path d="${paths.join("")}" />`;
}
```

`getGridPolygon` / `getPackPolygon` return the SVG coordinates of each cell's polygon (e.g. `"10,20 L 30,40 L 50,60 Z"`). All cells are combined into a single `<path>` for performance.

### Portability to Voronia
**Current phase**: ✅ **Completo**. `build_cell_wireframe()` (`cells.rs`) dibuja el wireframe por celda (Voronoi) usando `VoronoiVertices.cell_rings` + `positions`, como línea cap en la line layer. Toggle `cells` en `ui/layers.rs`.

| Phase | Description |
|---|---|
| **Done** | Per-cell Voronoi wireframe (line layer). |
| **Full (futuro)** | `customization === 1` grid (pre-pack) wireframe instead of pack; auto-toggle in editors (river/burg/route). |

---

## 5. Grid

### What it does in Azgaar
Overlays a configurable SVG grid over the map. Uses an SVG `<pattern>` to define the grid tile. Supports types: pointyHex, flatHex, square, square45deg, triangle. The grid scales with zoom (it is not geographic coordinate-based, it is a design grid).

### Source code
| File | Lines | Role |
|---|---|---|
| `layers.js` | 632-659 | `drawGrid()` — SVG pattern + rect |
| `style.js` | 493-603 | Grid configuration: type, scale, stroke, dash, shift |
| `index.html` | 1087-1121 | Grid type and scale selector |

### Exact implementation (`layers.js:632-659`)
```javascript
function drawGrid() {
  gridOverlay.selectAll("*").remove();
  const pattern = "#pattern_" + (gridOverlay.attr("type") || "pointyHex");
  const stroke = gridOverlay.attr("stroke") || "#808080";
  const width = gridOverlay.attr("stroke-width") || 0.5;
  const dasharray = gridOverlay.attr("stroke-dasharray") || null;
  const linecap = gridOverlay.attr("stroke-linecap") || null;
  const scale = gridOverlay.attr("scale") || 1;
  const dx = gridOverlay.attr("dx") || 0;
  const dy = gridOverlay.attr("dy") || 0;
  const tr = `scale(${scale}) translate(${dx} ${dy})`;

  d3.select(pattern)
    .attr("stroke", stroke)
    .attr("stroke-width", width)
    .attr("stroke-dasharray", dasharray)
    .attr("stroke-linecap", linecap)
    .attr("patternTransform", tr);
  gridOverlay
    .append("rect")
    .attr("width", maxWidth)
    .attr("height", maxHeight)
    .attr("fill", "url(" + pattern + ")")
    .attr("stroke", "none");
}
```

### Predefined grid patterns
Defined in `index.html` as SVG `<pattern>`s:
- `#pattern_pointyHex`: Pointy hexagons
- `#pattern_flatHex`: Flat hexagons
- `#pattern_square`: Squares
- `#pattern_square45deg`: Squares rotated 45°
- `#pattern_triangle`: Triangles

### Portability to Voronia
**Current phase**: ✅ **Completo**. `build_grid_lines()` (`grid.rs`) replicó el `#pattern_pointyHex` de Azgaar: segmentos del tile pointy-hex (25×43.4) repetidos en mosaico con culls por rango, como line layer. Toggle `grid` en `ui/layers.rs`.

| Phase | Description |
|---|---|
| **Done** | Pointy-hex tiling (paridad `pattern_pointyHex`), 3 tests. |
| **Full** | Los otros 4 patterns (flatHex, square, square45deg, triangle) + controles de estilo (scale, stroke, dash, shift) en el Style tab. |

---

## 6. Coordinates

### What it does in Azgaar
Renders graticule lines (latitude/longitude) with coordinate labels (e.g. 10°N, 20°E). Uses `d3.geoGraticule()` to generate the lines and `d3.geoEquirectangular()` to project them to the SVG. Automatically adapts the grid step based on zoom. The labels are placed at the map edges.

### Source code
| File | Lines | Role |
|---|---|---|
| `layers.js` | 673-731 | `drawCoordinates()` — everything: graticule + labels |
| `main.js` | 225 | Redraw on pan/zoom |

### Exact implementation (`layers.js:673-731`)
```javascript
function drawCoordinates() {
  coordinates.selectAll("*").remove();
  const steps = [0.5, 1, 2, 5, 10, 15, 30];
  const goal = mapCoordinates.lonT / scale / 10;
  const step = steps.reduce((p, c) => (Math.abs(c - goal) < Math.abs(p - goal) ? c : p));
  const desired = +coordinates.attr("data-size");
  coordinates.attr("font-size", Math.max(rn(desired / scale ** 0.8, 2), 0.1));
  const graticule = d3.geoGraticule()
    .extent([[mapCoordinates.lonW, mapCoordinates.latN],
             [mapCoordinates.lonE + 0.1, mapCoordinates.latS + 0.1]])
    .stepMajor([400, 400])
    .stepMinor([step, step]);
  const projection = d3.geoEquirectangular()
    .fitSize([graphWidth, graphHeight], graticule());
  // ... dibujar líneas y etiquetas ...
}
```

### Data consumed
- `mapCoordinates`: { lonW, lonE, latN, latS, lonT } — geographic bounds of the world
- `graphWidth`, `graphHeight`: size of the SVG viewbox
- `scale`: current zoom factor

### Portability to Voronia
| Phase | Description |
|---|---|
| **Implementation** | Voronia importa `mapCoordinates` del header del `.map` (rango geográfico) y proyecta lat/lon → mundo con la inversa de `lon_at_x`/`lat_at_y`. |
| **Done** | `build_coordinate_graticule()` (`coordinates.rs`) genera las líneas + labels con `pick_step()` adaptativo al zoom (`steps = [0.5,1,2,5,10,15,30]`, `goal = lonT/zoom/10`, como FMG). `TextSystem` multi-label pegada los labels al borde superior (longitud) e izquierdo (latitud) de la vista, reposicionados en cada pan/zoom. |
| **Pending / a medias** | Config de proyección/step manual en la UI; redibujado completo del graticule al cambiar zoom (el step aún se fija al cargar); N/S/E/W completa cuando el `zoom` cruza thresholds. |

---

## Portability status summary

> Actualizado: 7 ago 2026. Leyenda: ✅ completo · 🟡 a medias (básico funcionando, faltan opciones/refinamientos de Azgaar) · ❌ sin implementar.

| Layer | Current Voronia state | Dependencies | Priority |
|---|---|---|---|
| texture | 🟡 A medias — `TextureOverlay` world-anchored (papel que pan/zoom con el mapa), REPLACE, antes del ocean. Falta: X/Y shift | PNG/JPG loader | Low |
| heightmap | ✅ Completo — bandas de isoline rellenas (`BAND_STEP` 6), scheme bright/Spectral, océano excluido, conversión `height_m` real | isolines | Medium |
| relief | 🟡 A medias — `build_relief_mesh()` triángulos por celda con `h ≥ 40`, 3 tamaños/colores. Falta: Poisson-disc, iconos por bioma, z-order, sets | biomes, Poisson-disc, sprites | Low |
| cells | ✅ Completo — wireframe Voronoi por celda (line layer) + toggle UI | Voronoi/Delaunay geometry | Medium |
| grid | ✅ Completo — pattern pointy-hex replicado (`pattern_pointyHex`), 3 tests | line shader | Low |
| coordinates | 🟡 A medias — graticule con step zoom-adaptive, labels en bordes de vista (Texto multi). Falta: redibujado completo de step al zoomear, proyección/config UI | World coordinate system | High |

### Cross dependencies
- **coordinates** requiere que el World Data Model tenga coordinates geográficas (`mapCoordinates` en Azgaar). Ya llega del header del `.map` (import). La proyección equirect fue extraída a `vor-render::coordinates` (`lon_at_x`/`lat_at_y`/`pick_step`).
- **relief** necesita biomes (✅) + Poisson-disc sampling (nuevo).
- **texture** es independiente — solo necesita el loader PNG/JPG ya integrado.
- **texture** is independent, it only needs image loading.

### Suggested next step
· cells: done (toggle + wireframe).
· coordinates: re-draw completo del graticule al cambiar el step (zoom) y controles de proyección en la UI.
· relief: primero el acercamiento facilitado (biome icons con Poisson-disc), luego los sets.
