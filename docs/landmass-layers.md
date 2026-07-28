# Azgaar Landmass Layers — Documentación y plan de portabilidad

> **Categoría**: Landmass  
> **Capas**: texture, heightmap, relief, cells, grid, coordinates  
> **Fuente**: Azgaar's FMP v1.135.2 — `/home/hans/Proyectos/azgaar-fmg/`

---

## 1. Texture

### Qué hace en Azgaar
Superpone una imagen de textura raster sobre todo el mapa (papel, pergamino, tela, etc.). Es un `<image>` SVG escalado al tamaño del gráfico. No interactúa con celdas ni datos del mundo — es puramente decorativa.

### Código fuente
| Archivo | Líneas | Rol |
|---|---|---|
| `layers.js` | 783-796 | `drawTexture()` — crea/appendea un `<image>` SVG |
| `style.js` | 539-572 | Selector de textura (9 opciones: none, folded-paper, gray-paper, etc.) + shift X/Y |
| `load.ts` | 347, 390-394 | Inicializa grupo `#texture` en SVG |
| `index.html` | 782-796 | Selector de textura en Style editor |

### Implementación exacta (`layers.js:783-796`)
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

### Datos que consume
- `data-href`: URL/path de la imagen de textura
- `data-x`, `data-y`: offset de la textura (shift)
- `graphWidth`, `graphHeight`: dimensiones del SVG

### Portabilidad a Voronia
**Enfoque**: Post-process effect en wgpu (textura aplicada como blend sobre el framebuffer final, o como capa translúcida en un fullscreen quad).

| Etapa | Descripción |
|---|---|
| **Fase actual** | No implementado. `LayerFlags.texture` existe pero no renderiza. |
| **Implementación** | Cargar textura PNG/JPG como `wgpu::Texture`, dibujar un fullscreen quad con blending `multiply` o `overlay` sobre la salida del mapa. |
| **UI** | Selector de textura en tab Style (dropdown con 9 opciones, mismo que Azgaar). Controles shift X/Y. |

---

## 2. Heightmap

### Qué hace en Azgaar
Renderiza isolíneas de altura (contornos) coloreadas por un esquema de color configurable. Separa océano (height < 20) de tierra (height >= 20). Cada nivel de altura es un `<path>` SVG cerrado que forma bandas de elevación. Soporta terracing (sombra paralela en cada contorno).

### Código fuente
| Archivo | Líneas | Rol |
|---|---|---|
| `draw-heightmap.ts` | 1-198 | **Implementación completa TS** |
| `style.js` | 43-70 | Esquemas de color (`heightmapColorSchemes`) |
| `layers.js` | 263-276 | `toggleHeight()` + refresh en `drawLayers()` |
| `load.ts` | 348 | Inicializa `#terrs` con subgrupos `#oceanHeights` y `#landHeights` |

### Algoritmo (`draw-heightmap.ts`)
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

### Función clave `connectVertices()` (`draw-heightmap.ts:161-187`)
```
Input: cells, vertices, start_vertex, h, used[]
Output: chain (lista de vértices formando contorno cerrado)

- Desde start_vertex, camina por el grafo de vértices
- En cada paso: elige el vecino que cruza una celda en el lado opuesto del contorno
- Marca celdas como "used" para no reprocesarlas
- Termina cuando vuelve a start_vertex
- Máximo 100K iteraciones (seguro contra loops infinitos)
```

### Esquemas de color (`style.js:43-70`)
Azgaar tiene múltiples esquemas: elevation, wiki, grayscale, wiki2, elevation2, fancy, wiki3, palettes. Cada esquema es una función `(t: number) => string` que mapea altura normalizada (0-1) a color CSS.

### Portabilidad a Voronia
**Enfoque**: El heightmap de Voronia YA está implementado como pipeline wgpu (layer 0, mesh de triángulos coloreados por altura). Los contornos (isolíneas) serían una capa adicional opcional.

| Etapa | Descripción |
|---|---|
| **Fase actual** | ✅ Heightmap como mesh de triángulos con `height_color()` en CPU (gradiente lineal azul→verde→marrón→blanco). |
| **Pendiente** | Isolíneas de contorno (opcional). Esquemas de color configurables (ahora es fijo). Terracing. |
| **Implementación** | Para isolíneas: generar paths en CPU similar a Azgaar, renderizar como líneas en shader wgpu. Esquemas de color: lookup table uniform en GPU. |

---

## 3. Relief

### Qué hace en Azgaar
Dibuja íconos SVG de relieve (montañas, colinas, árboles) sobre el mapa usando muestreo Poisson-disc. Cada celda con altura < 50 recibe íconos de bioma (árboles), celdas con altura >= 50 reciben íconos de relieve (montañas/colinas). Los íconos se ordenan por Z (y + size). Soporta múltiples sets de íconos (simple, detailed, 3d).

### Código fuente
| Archivo | Líneas | Rol |
|---|---|---|
| `draw-relief-icons.ts` | 1-150 | Implementación completa TS |
| `index.html` | 2942-3427 | Definiciones SVG de íconos (`<symbol>`) |
| `layers.js` | 746-757 | `toggleRelief()` |
| `style.js` | 788-800 | Re-dibujado en cambios de preset/atributo |

### Algoritmo (`draw-relief-icons.ts:17-91`)
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

### Sets de íconos disponibles
| Set | Source |
|---|---|
| simple | Líneas básicas (triángulo para montaña, círculo para árbol) |
| detailed | SVG más elaborados con sombras |
| 3d | Perspectiva 3D |

### Portabilidad a Voronia
**Enfoque**: Íconos como instancias de glifo (wgpu indirect draw) o sprites texturizados. Para complejidad baja: puntos coloridos en pantalla (círculo = árbol, triángulo = montaña).

| Etapa | Descripción |
|---|---|
| **Fase actual** | No implementado. `LayerFlags.relief` existe pero no renderiza. |
| **Implementación MVP** | Calcular posiciones en CPU (Poisson-disc o centro de celda), renderizar como triángulos/círculos instanciados. |
| **Implementación completa** | Sprites de glifo desde atlas, z-ordering, 3 sets de íconos. |

---

## 4. Cells

### Qué hace en Azgaar
Renderiza los polígonos de celda (Voronoi) como líneas de borde. Esencialmente es un wireframe de la malla poligonal. Útil para debug y edición (river/burg/route editors lo activan automáticamente). Soporta tanto grid (pre-pack) como pack cells según el modo de customización.

### Código fuente
| Archivo | Líneas | Rol |
|---|---|---|
| `layers.js` | 446-451 | `drawCells()` — polyline path de bordes de celda |
| `layers.js` | 434-444 | `toggleCells()` |
| Múltiples editors | — | Activan `toggleCells()` al editar |

### Implementación exacta (`layers.js:446-451`)
```javascript
function drawCells() {
  const cells = customization === 1 ? grid.cells.i : pack.cells.i;
  const polygon = customization === 1 ? getGridPolygon : getPackPolygon;
  const paths = Array.from(cells).map(i => "M" + polygon(i));
  ensureEl("cells").innerHTML = `<path d="${paths.join("")}" />`;
}
```

`getGridPolygon` / `getPackPolygon` devuelven las coordenadas SVG del polígono de cada celda (ej: `"10,20 L 30,40 L 50,60 Z"`). Todas las celdas se combinan en un solo `<path>` para performance.

### Portabilidad a Voronia
**Fase actual**: No implementado como capa toggle. Los bordes de celda subyacen a las capas de borde (state/province/culture). Se puede implementar como líneas entre centros de celda vecinos (Delaunay edges) renderizadas como líneas wgpu.

| Fase | Descripción |
|---|---|
| **MVP** | Dibujar líneas Delaunay entre centros de celda con un pipeline de líneas wgpu. |
| **Completo** | Misma lógica que Azgaar: wireframe Voronoi por celda. |

---

## 5. Grid

### Qué hace en Azgaar
Superpone una cuadrícula SVG configurable sobre el mapa. Usa SVG `<pattern>` para definir el tile de la grilla. Soporta tipos: pointyHex, flatHex, square, square45deg, triangle. La grilla se escala con el zoom (no es coordenada geográfica, es de diseño).

### Código fuente
| Archivo | Líneas | Rol |
|---|---|---|
| `layers.js` | 632-659 | `drawGrid()` — patrón SVG + rect |
| `style.js` | 493-603 | Configuración de grid: tipo, escala, stroke, dash, shift |
| `index.html` | 1087-1121 | Selector de tipo de grid, escala |

### Implementación exacta (`layers.js:632-659`)
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

### Patrones de grid predefinidos
Definidos en `index.html` como `<pattern>` SVG:
- `#pattern_pointyHex`: Hexágonos en punta
- `#pattern_flatHex`: Hexágonos planos
- `#pattern_square`: Cuadrados
- `#pattern_square45deg`: Cuadrados rotados 45°
- `#pattern_triangle`: Triángulos

### Portabilidad a Voronia
| Fase | Descripción |
|---|---|
| **MVP** | Implementar patrón de cuadrícula rectangular simple (líneas horizontales/verticales) como fullscreen shader. |
| **Completo** | 5 patrones de grid renderizados como geometría de líneas instanciada. Controles de estilo en tab Style. |

---

## 6. Coordinates

### Qué hace en Azgaar
Renderiza líneas de graticule (latitud/longitud) con etiquetas de coordenadas (ej: 10°N, 20°E). Usa `d3.geoGraticule()` para generar las líneas y `d3.geoEquirectangular()` para proyectarlas al SVG. Adapta automáticamente el step de la grilla basado en el zoom. Las etiquetas se colocan en los bordes del mapa.

### Código fuente
| Archivo | Líneas | Rol |
|---|---|---|
| `layers.js` | 673-731 | `drawCoordinates()` — todo: graticule + labels |
| `main.js` | 225 | Redraw en pan/zoom |

### Implementación exacta (`layers.js:673-731`)
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

### Datos que consume
- `mapCoordinates`: { lonW, lonE, latN, latS, lonT } — límites geográficos del mundo
- `graphWidth`, `graphHeight`: tamaño del viewbox SVG
- `scale`: factor de zoom actual

### Portabilidad a Voronia
| Fase | Descripción |
|---|---|
| **Implementación** | Voronia no tiene sistema de coordenadas geográficas (lat/lon) todavía. Depende de la Fase 0/geography que defina la proyección del mundo. |
| **MVP** | Graticule hardcodeado (step fijo, sin proyección real). |
| **Completo** | Graticule con proyección configurable, step adaptable al zoom, labels N/S/E/W. |

---

## Resumen de estado de portabilidad

| Layer | Voronia actual | Dependencias | Prioridad |
|---|---|---|---|
| texture | ❌ No implementado | Cargar textura PNG, fullscreen quad | Baja |
| heightmap | ✅ Mesh base OK. Contours: ❌ | Esquemas de color, isolíneas | Media |
| relief | ❌ No implementado | Biomas, Poisson-disc, sprite rendering | Baja |
| cells | ❌ No toggle. Borders OK | Geometría Voronoi/Delaunay | Media |
| grid | ❌ No implementado | Patrones de grid, shader líneas | Baja |
| coordinates | ❌ No implementado | Sistema de coordenadas del mundo | Alta (dependencia) |

### Dependencias cruzadas
- **coordinates** necesita que el World Data Model tenga coordenadas geográficas (`mapCoordinates` en Azgaar). Esto viene del header del `.map` (pack → `cells.coordinates`?).
- **relief** necesita biomas implementados (✅) + Poisson-disc sampling (nuevo).
- **texture** es independiente, solo necesita carga de imagen.

### Próximo paso sugerido
Implementar **cells** como capa toggle — es la más simple (wireframe de la malla Voronoi ya existente) y es necesaria para los editores de Fase 6 (rivers, routes). Usar el pipeline de líneas wgpu existente.
