# Análisis completo: cómo Azgaar dibuja masa de tierra, líneas y suavizado

> **Fecha**: 30 jul 2026
> **Fuente**: Azgaar's FMP — `/home/hans/Proyectos/azgaar-fmg/` (commit local)
> **Propósito**: Referencia exacta para portar a Voronia el pipeline completo de tierra, líneas y suavizado
> **Cubre**: feature polygons → simplify → clip → fractalize → B-spline/Catmull-Rom → coastline stroke → isoline engine (connectVertices + getIsolines) → human geography fills + water gaps + halos → borders → heightmap contours → ocean bathymetry → river curves → SVG output

> ⚠️ **ALCANCE DE IMPLEMENTACIÓN INMEDIATA**: Solo puntos **1–5** (pipeline feature → SVG path, fractalización, path builder híbrido, coastline stroke). El resto (puntos 6–16: isoline engine, human geography fills, halos, borders, heightmap contours, ocean bathymetry, etc.) queda documentado para implementación futura.

---

## Índice

1. [Pipeline completo de tierra (feature → SVG path)](#1-pipeline-completo-de-tierra)
2. [Fractalización de costa (midpoint displacement + roughness profile)](#2-fractalización-de-costa)
3. [Path builder híbrido: B-spline + Catmull-Rom](#3-path-builder-híbrido)
4. [Coastline stroke (línea de costa)](#4-coastline-stroke)
5. [Isoline engine: connectVertices](#5-isoline-engine)
6. [Isoline engine: getIsolines](#6-getisolines)
7. [Human geography fills (estados, culturas, religiones, provincias)](#7-human-geography-fills)
8. [Water gap technique](#8-water-gap)
9. [State halos](#9-state-halos)
10. [Borders (fronteras)](#10-borders)
11. [Heightmap contours](#11-heightmap-contours)
12. [Ocean bathymetric layers](#12-ocean-bathymetric-layers)
13. [River smoothing (Catmull-Rom + meander)](#13-river-smoothing)
14. [Curvas disponibles en Azgaar (catálogo D3 + custom)](#14-curvas-disponibles)
15. [Resumen estilos SVG](#15-resumen-estilos-svg)
16. [Equivalencias Voronia](#16-equivalencias-voronia)

---

## 1. Pipeline completo de tierra

**Archivo**: `src/renderers/draw-features.ts:76-87` — `featurePathRenderer()`

Cada feature de tierra (continente, isla, lago, isla-en-lago) pasa por este pipeline exacto:

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

**Archivo**: `public/libs/simplify.js`

Librería de Vladimir Agafonkin. Combina dos algoritmos:
1. **Radial distance**: elimina puntos consecutivos dentro de la tolerancia cuadrática
2. **Ramer-Douglas-Peucker**: recursivo, encuentra el punto más alejado de la línea base; si supera la tolerancia, divide y repite

Llamada con `simplify(points, 0.3)` — tolerancia 0.3 píxeles, sin `highestQuality` → usa ambos algoritmos.

### 1.2 clipPoly con secure=1

**Archivo**: `src/utils/commonUtils.ts:16-37`

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

Sin `secure`, el B-spline de D3 (`curveBasisClosed`) se arquea lejos del borde del mapa, dejando un gap. Al duplicar los puntos de borde, la curva pasa exactamente por ellos.

**Solo `draw-features.ts` usa `secure=1`**. Ocean-layers y otros no lo necesitan porque sus paths no se cierran con `curveBasisClosed`.

---

## 2. Fractalización de costa

**Archivo**: `src/renderers/coastline-fractal.ts`

### 2.1 Roughness profile (per-feature)

Cada feature recibe un perfil de rugosidad único vía PRNG determinista (Alea con semilla `${seed}_c${featureIndex}`).

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

- `PROFILE_SIZE = 256` muestras alrededor del perímetro
- `numHarmonics = 4` → ~4 zonas rugosas alrededor del perímetro
- `contrast = 1.5` → acentúa diferencia entre calma y rugosidad

### 2.2 Midpoint displacement recursivo

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

Parámetros default:
- `maxDepth = 4` → hasta 16 segmentos por arista original
- `baseAmplitude = 1.5` → pico de desplazamiento
- `amplitudeDecay = 0.9` → 90% de amplitud por nivel
- `minEdge = 1` → aristas < 1px no se subdividen
- `smoothThreshold = 0.25` → zonas con rugosidad < 0.25 no se subdividen
- `lakeSmoothThreshMult = 2.0` → lagos tienen threshold 0.5 (más calmados)

### 2.3 Skip en bordes del mapa

Aristas donde AMBOS vértices están en el borde del mapa (`x===0 || x===graphWidth || y===0 || y===graphHeight`) no se fractalizan — mantienen la línea recta original.

---

## 3. Path builder híbrido

**Archivo**: `src/renderers/coastline-fractal.ts:194-252` — `buildCoastlinePath()`

### 3.1 Clasificación smooth/jagged

```typescript
// smooth[i] = true si el span original i→i+1 NO tiene subdivisión fractal
smooth[i] = (b > a ? b - a : b + N - a) === 1;
// Si solo hay 1 vértice entre orig[i] y orig[i+1], es smooth.
// Si hay más (fractal sub-points), es jagged.
```

### 3.2 Smooth spans: Q midpoint B-spline

Equivalente exacto a `curveBasisClosed` de D3:

```
M→(midpoint del último→primer span)   # arranque seamless
Q cpx,cpy mx,my                        # Q = quadratic bezier
```

Donde `mx = (cpx + npx) / 2`, `my = (cpy + npy) / 2`. Esto produce arcos suaves que ocultan la angularidad de Voronoi.

### 3.3 Jagged spans: centripetal Catmull-Rom

Para spans con subdivisión fractal, Catmull-Rom centrípeto (α~0.5 aunque la fórmula usada es equivalente a tensión ~0.25):

```
for each sub-segment j:
  cp1x = a.x + (b.x - prev.x) / 8
  cp1y = a.y + (b.y - prev.y) / 8
  cp2x = b.x - (nnext.x - a.x) / 8
  cp2y = b.y - (nnext.y - a.y) / 8
  C cp1x,cp1y cp2x,cp2y bx,by
```

La división por 8 produce tangentes = 1/4 de la diferencia, equivalente a tensión Catmull-Rom τ=0.25.

### 3.4 Transición seamless smooth↔jagged

El path arranca en el midpoint del último span (si es smooth) para que el loop cerrado no tenga costura. La variable `atMid` trackea si el cursor está en un midpoint (B-spline) o en un vértice original. Cuando se pasa de smooth a jagged, se emite un `L` al vértice original primero.

---

## 4. Coastline stroke

**Archivo**: `src/services/io/auto-update.ts:191-204`

La línea de costa NO es un stroke sobre el relleno de tierra — es un `<use>` que reutiliza el mismo path del feature pero renderizado como **línea** sin relleno.

### 4.1 Estructura DOM

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

### 4.2 Estilos

| Grupo | opacity | stroke | stroke-width | filter |
|-------|---------|--------|-------------|--------|
| `#sea_island` | 0.5 | `#1f3846` | 0.7 | `url(#dropShadow)` |
| `#lake_island` | 1 | `#7c8eaf` | 0.35 | none |

### 4.3 Diferencia con fill

El fill de tierra NO se renderiza directamente — la tierra se ve a través de las capas temáticas (heightmap, biomas, estados, etc.). El stroke de costa es sutil (~0.5px), semi-transparente, y en islas oceánicas lleva un drop shadow para dar profundidad.

---

## 5. Isoline engine: connectVertices

**Archivo**: `src/utils/pathUtils.ts:261-311`

Es el algoritmo fundamental que camina el grafo de Voronoi para trazar el contorno de una región de celdas del mismo tipo.

### 5.1 Algoritmo

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

### 5.2 Lógica de decisión

Para cada vértice actual, examina sus 3 celdas adyacentes (`vertices.c[vertex]` = normalmente 3 celdas contiguas). Si dos celdas consecutivas son de distinto tipo, el vértice está en la frontera y el vecino que conecta esas dos celdas es el siguiente en el chain.

```
   c1    c2
     \  /
      v   →  si c1 != c2, la arista v→v1 cruza la frontera
     /  \
   c3    (implícito)
```

### 5.3 Variantes

| Variante | Archivo | Diferencia |
|----------|---------|------------|
| General (`pathUtils.ts`) | `src/utils/pathUtils.ts:261` | Toma callback `ofSameType`, `addToChecked`, `closeRing` |
| Heightmap (`draw-heightmap.ts`) | `src/renderers/draw-heightmap.ts:162` | Especializado para altura: compara `cells.h[c] < h` |
| Ocean (`ocean-layers.ts`) | `src/renderers/ocean-layers.ts:35` | Especializado para capas de temperatura oceánica |

---

## 6. Isoline engine: getIsolines

**Archivo**: `src/utils/pathUtils.ts:84-177`

### 6.1 Flujo

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

Según `options`, genera:

| Option | Output | Descripción |
|--------|--------|-------------|
| `polygons` | `isolines[type].polygons` | `vertexChain.map(v => vertices.p[v])` — arrays de puntos |
| `fill` | `isolines[type].fill` | String SVG con paths de relleno (vía `getFillPath`) |
| `waterGap` | `isolines[type].waterGap` | Paths discontinúos en tierra (vía `getBorderPath`) |
| `halo` | `isolines[type].halo` | Paths discontinúos en borde de mapa (vía `getBorderPath`) |

### 6.3 `getFillPath`

**Archivo**: `src/utils/pathUtils.ts:49-82`

```typescript
function getFillPath(vertices, vertexChain): string {
  return vertexChain.map(v => vertices.p[v]).join(" ");
  // Pasado por lineGen().curve(curveBasisClosed) en el llamante
}
```

### 6.4 `getBorderPath`

**Archivo**: `src/utils/pathUtils.ts:25-47`

Genera SVG path con comandos M/L, rompiendo el path donde `discontinue(vertex)` es true. Esto produce múltiples sub-paths en lugar de uno continuo.

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

Todas las capas de relleno humano siguen el mismo patrón:

```
getIsolines(pack, cellId => cells.{type}[cellId], { fill: true, waterGap: true [, halo: bool] })
→ por cada tipo:
    <path d="{fill}" fill="{color}" id="{type}{index}" />
    <path d="{waterGap}" fill="none" stroke="{color}" stroke-width="3" id="{type}-gap{index}" />
    [<clipPath id="state-clip{index}"><use href="#state{index}"/></clipPath>]
    [<path d="{halo}" stroke="{darkerColor}" clip-path="url(#state-clip{index})" />]
```

### 7.1 Patrón por capa

| Capa | Archivo (layers.js) | getType | options | Extra |
|------|--------------------|---------|---------|-------|
| Culturas | `drawCultures():480` | `cells.culture` | `{fill, waterGap}` | — |
| Religiones | `drawReligions():509` | `cells.religion` | `{fill, waterGap}` | — |
| Estados | `drawStates():537` | `cells.state` | `{fill, waterGap, halo}` | Halo solo si `shapeRendering==="geometricPrecision"` |
| Provincias | `drawProvinces():592` | `cells.province` | `{fill, waterGap}` | — |
| Zonas | `drawZones():978` | `cells.zone` | `{fill}` | Sin water gap |

### 7.2 Diferencia clave: isolines vs borders

**getIsolines** → genera paths de **relleno** + water gaps + halos. Los paths son **curvas cerradas** (B-spline via D3).

**drawBorders** → genera paths de **línea** (stroke) entre celdas de distinto estado/provincia. Los paths son **segmentos rectos** (join de vértices con `M...L...L...`). No hay suavizado en borders.

---

## 8. Water gap technique

### 8.1 Propósito

Evitar que el color de una región "sangre" visualmente al océano/lago en los bordes. Azgaar dibuja un stroke fino del mismo color del relleno en los bordes que tocan agua.

### 8.2 Implementación

`getBorderPath(vertices, vertexChain, isLandVertex)` donde:
```typescript
const isLandVertex = (vertexId) => vertices.c[vertexId].every(i => cells.h[i] >= 20);
```

Cuando el vértice está rodeado SOLO de celdas de tierra (height ≥ 20), el path se rompe. Cuando está en costa (mezcla tierra/agua), el path continúa. El resultado es un path que solo dibuja en bordes contra agua.

### 8.3 Estilo

```html
<path d="{waterGap}" fill="none" stroke="{color}" stroke-width="3" ... />
```

Stroke-width 3 es deliberadamente grueso para cubrir cualquier anti-aliasing o gap. Como es del mismo color del relleno, se funde visualmente.

---

## 9. State halos

### 9.1 Propósito

Cuando `shapeRendering === "geometricPrecision"`, los estados tienen un halo (sombra interior) que los separa visualmente.

### 9.2 Implementación

```typescript
// 1. Path del halo: solo vértices en borde de mapa
const isBorderVertex = (vertexId) => vertices.c[vertexId].some(i => cells.b[i]);
// 2. Color más oscuro
const haloColor = d3.color(color).darker().hex();
// 3. Render con clip-path para que quede DENTRO del estado
<clipPath id="state-clip{index}"><use href="#state{index}"/></clipPath>
<path d="{halo}" stroke="{haloColor}" clip-path="url(#state-clip{index})" />
```

El halo solo se dibuja donde el path del estado toca el borde del mapa.

---

## 10. Borders

**Archivo**: `src/renderers/draw-borders.ts`

### 10.1 Diferencia fundamental con getIsolines

`drawBorders` NO usa `getIsolines`. Usa un algoritmo separado que:
1. Itera por celdas buscando pares (cellA, cellB) de distinto estado/provincia
2. Para cada par, encuentra un vértice inicial en la frontera
3. Camina el grafo de vértices con `getVerticesLine()` (similar a `connectVertices` pero local)
4. Output: `M x0,y0 x1,y1 x2,y2 ...` (segmentos rectos, sin suavizado)
5. Marca pares celda-estado como checked para no duplicar

### 10.2 Output

```typescript
select("#stateBorders").append("path").attr("d", statePath.join(" "));
select("#provinceBorders").append("path").attr("d", provincePath.join(" "));
```

Sin atributos stroke explícitos en el renderer — se heredan de CSS `#borders { stroke-linejoin: round; fill: none; }` y defaults de SVG. El color/width se setea dinámicamente desde el editor.

### 10.3 Estilo visual (desde editor)

- State borders: stroke `#000`, stroke-width `1.2` (cuando se resalta)
- Province borders: stroke `#999`, stroke-width `0.5-1.0` (defaults SVG)
- No hay suavizado — las líneas siguen las aristas de Voronoi directamente

---

## 11. Heightmap contours

**Archivo**: `src/renderers/draw-heightmap.ts`

### 11.1 Algoritmo

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

### 11.2 Configuración

Dos grupos SVG separados:
- `#oceanHeights`: alturas < 20 (océano), render condicional
- `#landHeights`: alturas >= 20 (tierra), siempre renderiza

Atributos configurables vía DOM:
- `skip`: cada N niveles de altura (default 1)
- `relax`: simplificación (stride, default 0)
- `curve`: tipo de curva D3 (default `curveBasisClosed`)
- `scheme`: esquema de color
- `terracing`: sombra de terracing (desplazamiento + darker)

### 11.3 Renderizado

```typescript
// Paths se agrupan por altura y se renderizan como rect base + paths coloreados
if (terracing) {
  group.append("path").attr("d", path)
    .attr("transform", "translate(.7,1.4)")
    .attr("fill", color(fillColor).darker(terracing));
}
group.append("path").attr("d", path).attr("fill", fillColor);
```

El terracing da efecto 3D desplazando una copia más oscura 0.7px X, 1.4px Y.

---

## 12. Ocean bathymetric layers

**Archivo**: `src/renderers/ocean-layers.ts`

### 12.1 Algoritmo

Similar a heightmap contours pero para capas de temperatura oceánica (`cells.t`):

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

### 12.2 Diferencias con heightmap

- layers.js llama `getIsolines(pack, cellId => cells.t[cellId], { polygons: true })` para obtener los polígonos
- Usa `curveBasisClosed` por defecto (vía `lineGen`)
- Opacidad total `0.4 / num_limits` distribuida entre las capas
- No tiene water gap ni halos

---

## 13. River smoothing

**Archivo**: `src/generators/river-generator.ts:426-455`

### 13.1 Catmull-Rom para banks

```typescript
this.lineGen.curve(curveCatmullRom.alpha(0.1));
// Right bank: lineGen(riverPointsRight.reverse())
// Left bank: reverse of lineGen(riverPointsLeft)
```

`alpha=0.1` → muy cercano a uniform Catmull-Rom (α=0), produce curvas más suaves que centrípeto (α=0.5).

### 13.2 Meander + relaxAcuteAngles

**Archivo**: `src/utils/pathUtils.ts:370-506`

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

`relaxAcuteAngles` itera sobre los puntos de control y refleja aquellos que forman ángulos agudos (< 60°) a través de la línea base anchor-to-anchor.

### 13.3 Diferencia con coastline path

Mientras el coastline usa **centripetal Catmull-Rom** (α=0.5, implícito en la fórmula de Hermite con división por 8), los ríos usan **uniform Catmull-Rom** (α=0.1 ~ α=0). El coastline prioriza que la curva pase cerca de los puntos fractalizados; los ríos priorizan suavidad general.

---

## 14. Curvas disponibles en Azgaar

### 14.1 Catálogo D3 completo

Desde `draw-heightmap.ts:27-45`, Azgaar expone TODAS las curvas D3:

| Curva | Tipo | Uso en Azgaar |
|-------|------|---------------|
| `curveBasis` | B-spline abierto | — |
| `curveBasisClosed` | B-spline cerrado | **Default** para heightmap, temperature, markets, ocean, isofeatures |
| `curveBasisOpen` | B-spline abierto | — |
| `curveCardinal` | Cardinal abierto | — |
| `curveCardinalClosed` | Cardinal cerrado | — |
| `curveCardinalOpen` | Cardinal abierto | — |
| `curveCatmullRom` | Catmull-Rom abierto | Rivers (α=0.1) |
| `curveCatmullRomClosed` | Catmull-Rom cerrado | — |
| `curveCatmullRomOpen` | Catmull-Rom abierto | — |
| `curveLinear` | Segmentos rectos | User selectable |
| `curveLinearClosed` | Segmentos rectos cerrado | — |
| `curveMonotoneX` | Monotono X | — |
| `curveMonotoneY` | Monotono Y | — |
| `curveNatural` | Spline natural | — |
| `curveStep` | Escalonada | — |
| `curveStepAfter` | Escalonada post | — |
| `curveStepBefore` | Escalonada pre | — |

### 14.2 Curva custom: coastline hybrid

La `buildCoastlinePath` es una curva **híbrida custom** que NO existe en D3: combina B-spline (spans suaves) con Catmull-Rom centrípeto (spans fractalizados) según si el span original fue subdividido o no.

---

## 15. Resumen estilos SVG

| Elemento | Fill | Stroke | Stroke-width | Opacidad | Filter |
|----------|------|--------|-------------|----------|--------|
| Coastline (sea island) | none | `#1f3846` | 0.7 | 0.5 | `url(#dropShadow)` |
| Coastline (lake island) | none | `#7c8eaf` | 0.35 | 1 | none |
| Lakes | color grupo | — | — | 0.5-1 | — |
| Heightmap contours | color scheme | — | — | 1 | terracing desplazado |
| State fill | `states[index].color` | — | — | 1 | — |
| State water gap | none | `states[index].color` | 3 | 1 | — |
| State halo | — | `color.darker()` | SVG default | — | clip-path al estado |
| Culture fill | `cultures[index].color` | — | — | 1 | — |
| Religion fill | `religions[index].color` | — | — | 1 | — |
| Province fill | `provinces[index].color` | — | — | 1 | — |
| State borders | none | `#000` (default) | 1.2 (hover) | 1 | — |
| Province borders | none | `#999` (default) | 0.5-1 | 1 | — |
| Ocean layers | `#ecf2f9` | — | — | `0.4/num` | — |
| Rivers left bank | `#6f9db3` (default) | — | — | 1 | — |
| Rivers right bank | `#0f2631` (default) | — | — | 1 | — |

---

## 16. Equivalencias Voronia

| Sistema Azgaar | Estado Voronia | Archivo Voronia |
|----------------|---------------|-----------------|
| `simplify(points, 0.3)` | No implementado (features se usan raw) | `mesh.rs` / nuevo |
| `clipPoly(points, W, H, 1)` | Implementado en `mesh.rs` (clip a bbox) | `mesh.rs` |
| `fractalizeCoastline()` | **Parcial**: fractal midpoint displacement sí, roughness profile sí | `coastline.rs` |
| `buildCoastlinePath()` hybrid | **Parcial**: Catmull-Rom sí, faltan spans smooth + B-spline | `coastline.rs` |
| Coastline stroke (línea) | **No implementado**: solo hay relleno | nuevo `coastline_stroke.rs` |
| `connectVertices()` | **No implementado**: núcleo del isoline engine | nuevo `isoline.rs` |
| `getIsolines()` | **No implementado**: envoltura que itera celdas | nuevo `isoline.rs` |
| `getBorderPath()` water gap | **Parcial**: `water_gap.rs` implementa detección de celdas costa | `water_gap.rs` |
| `getFillPath()` | No implementado (para relleno B-spline suavizado) | nuevo |
| State/culture/religion fills + water gap | **Hecho**: `state_layer.rs` etc. con `append_water_gap` | varios |
| State halos | **No implementado** | nuevo |
| Borders cell-to-cell | **Hecho**: `border.rs` con segmentos rectos | `border.rs` |
| Heightmap contours | **Parcial**: `contour.rs` usa marching squares sobre grid, no sobre Voronoi | `contour.rs` |
| Ocean bathymetric rings | **No implementado** | nuevo |
| River Catmull-Rom α=0.1 | **Hecho**: `river.rs` usa `build_river_mesh` con meander + ancho | `river.rs` |
| River meander + relax | **Hecho**: port exacto en `vor-sim` | `vor-sim/src/river/meander.rs` |

### 16.1 Lo que más impacto visual daría (orden recomendado)

1. **connectVertices + getIsolines** — destraba borders suaves, heightmap contours correctos, ocean layers, y reemplaza el marching squares actual
2. **buildCoastlinePath híbrido** — reemplaza el Catmull-Rom simple por B-spline + Catmull-Rom según span
3. **Coastline stroke** — la línea de costa sobre el relleno de tierra
4. **State halos** — separación visual entre estados
5. **Ocean bathymetric rings** — profundidad oceánica

### 16.2 Nota sobre B-spline en wgpu

D3 genera curvas B-spline como paths SVG (comandos Q/C). En wgpu no hay path renderer nativo. Opciones:
- **Opción A (CPU)**: Teselar los paths a triángulos en CPU antes de subir a GPU
- **Opción B (GPU)**: Shader geometry que evalúa B-spline/Catmull-Rom en vertex shader
- **Opción C (lyon)**: Lyon tiene `tessellate_path` que soporta curvas Bezier → triángulos

Voronia ya usa lyon para teselación, así que la Opción C es la más natural: generar paths SVG (comandos M/Q/C) y teselarlos con lyon.

---

*Fin del análisis — 30 jul 2026*
