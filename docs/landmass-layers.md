# Azgaar Landmass Layers — Documentation and porting plan

> **Category**: Landmass  
> **Layers**: texture, heightmap, relief, cells, grid, coordinates  
> **Source**: Azgaar's FMP v1.135.2 — `/home/hans/Proyectos/azgaar-fmg/`

---

## 1. Texture

### What it does in Azgaar
Overlays a raster texture image on top of the entire map (paper, parchment, cloth, etc.). It is an SVG `<image>` scaled to the size of the chart. It does not interact with cells or world data — it is purely decorative.

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
**Approach**: Post-process effect in wgpu (texture applied as a blend over the final framebuffer, or as a translucent layer on a fullscreen quad).

| Stage | Description |
|---|---|
| **Current phase** | Not implemented. `LayerFlags.texture` exists but does not render. |
| **Implementation** | Load PNG/JPG texture as `wgpu::Texture`, draw a fullscreen quad with `multiply` or `overlay` blending over the map output. |
| **UI** | Texture selector in the Style tab (dropdown with 9 options, same as Azgaar). X/Y shift controls. |

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
**Approach**: Voronia's heightmap is ALREADY implemented as a wgpu pipeline (layer 0, mesh of triangles colored by altitude). The contours (isolines) would be an additional optional layer.

| Stage | Description |
|---|---|
| **Current phase** | ✅ Heightmap as a triangle mesh with `height_color()` on CPU (linear blue→green→brown→white gradient). |
| **Pending** | Contour isolines (optional). Configurable color schemes (currently fixed). Terracing. |
| **Implementation** | For isolines: generate paths on CPU similar to Azgaar, render as lines in a wgpu shader. Color schemes: uniform lookup table on GPU. |

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
| **Current phase** | Not implemented. `LayerFlags.relief` exists but does not render. |
| **MVP implementation** | Compute positions on CPU (Poisson-disc or cell center), render as instanced triangles/circles. |
| **Full implementation** | Glyph sprites from an atlas, z-ordering, 3 icon sets. |

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
**Current phase**: Not implemented as a toggle layer. Cell borders underlie the border layers (state/province/culture). It can be implemented as lines between neighboring cell centers (Delaunay edges) rendered as wgpu lines.

| Phase | Description |
|---|---|
| **MVP** | Draw Delaunay lines between cell centers with a wgpu line pipeline. |
| **Full** | Same logic as Azgaar: per-cell Voronoi wireframe. |

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
| Phase | Description |
|---|---|
| **MVP** | Implement a simple rectangular grid pattern (horizontal/vertical lines) as a fullscreen shader. |
| **Full** | 5 grid patterns rendered as instanced line geometry. Style controls in the Style tab. |

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
| **Implementation** | Voronia does not have a geographic coordinate system (lat/lon) yet. Depends on Phase 0/geography to define the world projection. |
| **MVP** | Hardcoded graticule (fixed step, no real projection). |
| **Full** | Graticule with configurable projection, zoom-adaptive step, N/S/E/W labels. |

---

## Portability status summary

| Layer | Current Voronia state | Dependencies | Priority |
|---|---|---|---|
| texture | ❌ Not implemented | Load PNG texture, fullscreen quad | Low |
| heightmap | ✅ Base mesh OK. Contours: ❌ | Color schemes, isolines | Medium |
| relief | ❌ Not implemented | Biomes, Poisson-disc, sprite rendering | Low |
| cells | ❌ No toggle. Borders OK | Voronoi/Delaunay geometry | Medium |
| grid | ❌ Not implemented | Grid patterns, line shader | Low |
| coordinates | ❌ Not implemented | World coordinate system | High (dependency) |

### Cross dependencies
- **coordinates** requires the World Data Model to have geographic coordinates (`mapCoordinates` in Azgaar). This comes from the `.map` header (pack → `cells.coordinates`?).
- **relief** needs implemented biomes (✅) + Poisson-disc sampling (new).
- **texture** is independent, it only needs image loading.

### Suggested next step
Implement **cells** as a toggle layer — it is the simplest (wireframe of the already-existing Voronoi mesh) and it is needed for the Phase 6 editors (rivers, routes). Use the existing wgpu line pipeline.
