# Análisis exhaustivo del sistema de ríos de Azgaar FMG

> Fecha: 29 jul 2026
> Fuente: código TypeScript de azgaar-fmg (commit HEAD en `/home/hans/Proyectos/azgaar-fmg/`)
> Propósito: referencia para el port nativo en Voronia

## Archivos analizados

| # | Archivo | Líneas | Rol |
|---|---|---|---|
| 1 | `src/generators/river-generator.ts` | 589 | Motor principal de generación y width calculus |
| 2 | `src/utils/pathUtils.ts` | 526 | Meandering + SVG banking utilities |
| 3 | `src/generators/lakes.ts` | 129 | Clima de lagos, detección de cerrados, cleanup |
| 4 | `src/generators/features.ts` | 386 | Tipo `Feature` con `outlet`/`outCell`/`inlets`/`shoreline` |
| 5 | `src/generators/resample.ts` | 471 | Re-muestreo con `restoreRivers` |
| 6 | `src/types/PackedGraph.ts` | 70 | Interfaz `PackedGraph` con `cells.r`, `cells.conf`, `rivers` |
| 7 | `src/generators/voronoi.ts` | 155 | Genera adyacencia `cells.c`, `cells.v`, vértices |
| 8 | `src/utils/graphUtils.ts` | 554 | `calculateVoronoi`, `findClosestCell`, `getPackPolygon` |
| 9 | `src/controllers/river-editor.ts` | 350 | Editor visual de río existente |
| 10 | `src/controllers/river-creator.ts` | 172 | Creación manual de río |
| 11 | `src/controllers/rivers-overview.ts` | 258 | Tabla de ríos + export CSV + basin highlight |
| 12 | `src/services/io/save.ts` | 215 | Serialización .map (campos 18, 22, 32) |
| 13 | `src/services/io/load.ts` | 847 | Parseo .map + validación de integridad |
| 14 | `src/services/io/export-json.ts` | 233 | Export JSON (mínimo + pack completo) |
| 15 | `src/services/io/export.ts` | 807 | Export GeoJSON de ríos |
| 16 | `src/services/io/auto-update.ts` | — | Migraciones v1.21, v1.6, v1.65 |

---

## 1. Arquitectura general

Los ríos en Azgaar se modelan como:

1. **`pack.rivers`**: array de objetos `River`, uno por río. Cada río tiene un id único (1-based), source, mouth, cells que recorre, puntos de ancla, y parámetros de ancho.
2. **`pack.cells.r`**: `Uint16Array` indexado por celda pack — el id del río que pasa por esa celda (0 = no río).
3. **`pack.cells.fl`**: `Uint16Array` — flujo de agua (flux) en cada celda, en m³/s.
4. **`pack.cells.conf`**: `Uint8Array` — flux acumulado en confluencias (todos los afluentes excepto el principal).

### 1.1 Interfaz River (`river-generator.ts:9-24`)

```ts
export interface River {
  i: number;           // river id (1-based, 0 = no river)
  source: number;      // source cell index (pack)
  mouth: number;       // mouth cell index (pack) — penúltima celda
  parent: number;      // parent river id (0 = main stem)
  basin: number;       // basin river id (main stem id)
  length: number;      // river length (km)
  discharge: number;   // discharge at mouth (m³/s)
  width: number;       // mouth width (km)
  widthFactor: number; // width scaling factor
  sourceWidth: number; // source width (km)
  name: string;        // river name
  type: string;        // river type (River, Creek, Fork, etc.)
  cells: number[];     // cell ids forming the river path
  points?: Point[];    // custom anchor points (optional, set on edit)
}
```

### 1.2 Interfaz PackedGraph relevante (`PackedGraph.ts:18-70`)

```ts
interface PackedGraph {
  cells: {
    i: number[];          // cell indices
    c: number[][];        // neighboring cells
    v: number[][];        // neighboring vertices
    p: [number, number][];// cell centers
    b: boolean[];         // cell is on border
    h: TypedArray;        // heights (u8)
    t: TypedArray;        // terrain type
    r: TypedArray;        // river id per cell
    f: TypedArray;        // feature id per cell
    fl: TypedArray;       // water flux per cell
    conf: TypedArray;     // confluence flux
    haven: TypedArray;    // depression haven
    g: number[];          // grid cell id mapping
    culture: TypedArray;  // culture id per cell
  };
  vertices: { p, c, v };  // Voronoi vertices
  rivers: River[];         // array de ríos
  features: Feature[];     // features (ocean, lake, island)
}
```

---

## 2. Pipeline de generación (`river-generator.ts:47-294`)

### 2.1 Orden de ejecución

```
generate(allowErosion):
  1. cells.fl = new Uint16Array(...)     — arrays de agua
  2. cells.r = new Uint16Array(...)       — arrays de ríos
  3. cells.conf = new Uint8Array(...)     — arrays de confluencia
  4. riverNext = 1                        — primer id de río
  5. h = alterHeights()                   — copia de alturas modificada
  6. Lakes.detectCloseLakes(h)            — lagos cerrados/abiertos
  7. resolveDepressions(h)                — relleno de depresiones
  8. drainWater()                         — drenaje principal
  9. defineRivers()                       — construye pack.rivers
  10. calculateConfluenceFlux()           — flux de confluencias
  11. Lakes.cleanupLakeData()            — limpia temporales de lagos
  12. if (allowErosion): cells.h = Uint8Array.from(h); downcutRivers()
```

### 2.2 `alterHeights()` (líneas 296-306)

```ts
alterHeights(): number[] {
  return Array.from(h).map((h, i) => {
    if (h < 20 || t[i] < 1) return h;    // agua o sin tipo: sin cambios
    return h + t[i]/100 + mean(c[i].map(c => t[c]))/10000;
  });
}
```

- Retorna `Array<number>` (no TypedArray) — permite fracciones
- Las celdas de tierra se elevan según su tipo de terreno y el promedio de sus vecinos
- El resultado se usa para cálculos de flujo; `cells.h` original se preserva

### 2.3 `resolveDepressions(h)` (líneas 309-367)

Priority-Flood variant (Barnes):
- Procesa celdas de menor a mayor altura
- Eleva deprimidas a `min(altura vecina) + 0.1`
- Lagos: intenta elevar `l.height` a `min(shoreline) + 0.2` (primeras 75% iteraciones)
- Pasadas las 75% iteraciones: lagos resistentes se marcan `closed = true`
- Si la progresión empeora (>0 delta por 5 iteraciones), aborta y reinicia h
- Máximo de iteraciones desde UI (`lakeElevationLimitOutput`)

### 2.4 `drainWater()` (líneas 60-150)

Algoritmo principal de drenaje:

```
land = cells.i donde h[i] >= 20, sorted desc (high to low)
for each cell i in land:
    1. cells.fl[i] += prec[g[i]] / cellsNumberModifier   — precipitación
    2. if i es outCell de lago abierto:
         - extrae lakeCell (vecino con h<20, mismo feature)
         - cells.fl[lakeCell] += lake.flux - lake.evaporation
         - asigna river a lakeCell (respetando chain lakes)
         - flowDown(i, cells.fl[lakeCell], lake.outlet)
    3. if cells.b[i] && cells.r[i]: addCellToRiver(-1, cells.r[i]); continue  — off-map
    4. downhill = min(h[neighbors])  — celda vecina más baja
    5. if h[i] <= h[downhill]: continue  — depresión, skip
    6. if cells.fl[i] < MIN_FLUX_TO_FORM_RIVER (30):
         cells.fl[downhill] += cells.fl[i]; continue  — flujo subterráneo
    7. if !cells.r[i]:
         cells.r[i] = riverNext; addCellToRiver(i, riverNext); riverNext++
    8. flowDown(downhill, cells.fl[i], cells.r[i])
```

Constantes clave:
- `MIN_FLUX_TO_FORM_RIVER = 30`
- `cellsNumberModifier = (cells/10000)^0.25`

### 2.5 `flowDown(toCell, fromFlux, river)` (líneas 152-186)

```
toFlux = cells.fl[toCell] - cells.conf[toCell]   — flux "limpio"
if cells.r[toCell] (ya tiene río):
    if fromFlux > toFlux:
        cells.conf[toCell] += cells.fl[toCell]   — el que pierde se marca como conf
        riverParents[toRiver] = river             — el viejo es tributario del nuevo
        cells.r[toCell] = river                   — reasigna
    else:
        cells.conf[toCell] += fromFlux
        riverParents[river] = toRiver              — el nuevo es tributario del viejo
else:
    cells.r[toCell] = river

if h[toCell] < 20:  — agua
    waterBody = features[cells.f[toCell]]
    if waterBody es lago:
        actualiza waterBody.river (el de mayor enteringFlux)
        waterBody.flux += fromFlux
        registra inlet
else:
    cells.fl[toCell] += fromFlux

addCellToRiver(toCell, river)
```

### 2.6 `defineRivers()` (líneas 188-241)

Construye `pack.rivers` array:

```ts
// Reinicializa
cells.r = new Uint16Array(cells.i.length);
cells.conf = new Uint8Array(cells.i.length);
pack.rivers = [];

defaultWidthFactor = rn(1 / ((cells/10000)^0.25), 2);
mainStemWidthFactor = defaultWidthFactor * 1.2;

for each river in riversData:
    if riverCells.length < 3: continue           — descarta ríos pequeños

    // Marca confluencias reales
    for cell in riverCells:
        if cells.r[cell]: cells.conf[cell] = 1   — confluye con otro río
        else: cells.r[cell] = riverId

    source = riverCells[0]
    mouth = riverCells[length-2]                  — PENÚLTIMA celda
    parent = riverParents[key] || 0

    widthFactor = (parent === 0 || parent === riverId)
        ? mainStemWidthFactor
        : defaultWidthFactor

    meanderedPoints = addMeandering(riverCells)
    discharge = cells.fl[mouth]
    length = getApproximateLength(meanderedPoints)
    sourceWidth = getSourceWidth(cells.fl[source])
    width = getWidth(getOffset({
        flux: discharge,
        pointIndex: meanderedPoints.length,
        widthFactor,
        startingWidth: sourceWidth
    }))

    push River { i, source, mouth, parent, discharge, length,
                 width, widthFactor, sourceWidth, cells: riverCells }
```

#### Mouth: la penúltima celda

```ts
const mouth = riverCells[riverCells.length - 2];
```

La última celda del array `riverCells` es:
- `-1` si el río sale del mapa (caso `cells.b[i] && cells.r[i]`)
- Una celda de agua/lago donde desemboca

### 2.7 `calculateConfluenceFlux()` (líneas 259-272)

```ts
for each cell with conf[i]:
    influx = vecinos con río y más altos (entrantes) -> sus flujos
    sort influx descending
    cells.conf[i] = sum(influx[1:])   — suma todos EXCEPTO el mayor
```

El flux del río principal en la confluencia NO se cuenta como "confluence flux".

### 2.8 `downcutRivers()` (líneas 243-257)

```ts
MAX_DOWNCUT = 5
for each cell:
    if h[i] < 35 || !fl[i]: continue    — no erosiona tierras bajas
    higherFlux = mean(fl de vecinos más altos)
    if !higherFlux: continue
    downcut = floor(fl[i] / higherFlux)
    if downcut: h[i] -= min(downcut, MAX_DOWNCUT)
```

- Solo erosiona si `h >= 35`
- El ratio `fl[i]/higherFlux` determina cuánto erosionar
- Máximo 5 unidades de altura por celda

---

## 3. Meandering (`pathUtils.ts:370-429`)

### 3.1 `meander(cells, cellPositions, options)`

```ts
meandering = options.meandering ?? 0.5
startStep = options.startStep ?? 10
cellCount = options.cellCount ?? cells.length

// Construir anchor points (posiciones de celdas)
anchorPoints = cells.map(cell -> posición del centro)

// Loop principal
points = []
anchorIndices = []
step = startStep

for each anchor i:
    anchorIndices.push(points.length)
    points.push(anchorPoints[i])

    if i == last: break
    [x2, y2] = anchorPoints[i+1]
    dist2 = (x2-x1)² + (y2-y1)²

    if dist2 <= 25 && cellCount >= 6: continue    — saltar segmentos cortos

    meanderVal = meandering + 1/step + max(meandering - step/100, 0)
    if waterCell: meanderVal *= 0.25               — menos meandro en agua

    angle = atan2(y2-y1, x2-x1)
    sinM = sin(angle) * meanderVal
    cosM = cos(angle) * meanderVal

    if step < 20 && (dist2 > 64 || (dist2 > 36 && cellCount < 5)):
        // 2 meander points: a 1/3 y 2/3 del segmento
        p1 = [ (2*x1+x2)/3 - sinM , (2*y1+y2)/3 + cosM ]
        p2 = [ (x1+2*x2)/3 + sinM/2 , (y1+2*y2)/3 - cosM/2 ]
        push p1, p2
    else if dist2 > 25 || cellCount < 6:
        // 1 meander point: midpoint con desplazamiento
        pm = [ (x1+x2)/2 - sinM , (y1+y2)/2 + cosM ]
        push pm

    step++

relaxAcuteAngles(points, anchorIndices)
return { points, anchorIndices }
```

Características:
- `meanderVal` decrece naturalmente: `1/step` se reduce, `max(0.5-step/100,0)` se vuelve 0 cuando `step > 50`
- En celdas de agua: meandro × 0.25 (casi recto)
- `startStep` = `h[riverCells[0]] < 20 ? 1 : 10` (si source en lago, arranca con step 1 = mucho meandro hasta salir)
- Los segmentos cortos (dist² ≤ 25, ~5px) se saltan si hay ≥6 celdas

### 3.2 `relaxAcuteAngles(points, anchorIndices)` (líneas 453-506)

Relajación de ángulos agudos mediante reflejo sobre la línea base entre anchors:

```
RELAX_ITERATIONS = 4
isAnchor[i] = true para anchor points

for 4 iteraciones:
    snapshot = copia de points
    for each non-anchor point i:
        p = anchor anterior, q = anchor siguiente
        flipped = reflectAcrossLine(point[i], snapshot[p], snapshot[q])
        before = acuteCost(i-1) + acuteCost(i) + acuteCost(i+1)
        after = acuteCost con flipped(i-1) + acuteCost con flipped(i) + acuteCost con flipped(i+1)
        if after < before - 1e-6:
            points[i] = flipped

    if no flips: break
```

- `acuteCost(i)` = `max(cos(angle at vertex i), 0)` — penaliza ángulos < 90°
- `reflectAcrossLine(m, P, Q)` refleja `m` sobre la línea `PQ`
- Solo mueve puntos no-anchor; los anchors son fijos

### 3.3 `addMeandering(riverCells, riverPoints)` (river-generator.ts:369-387)

```ts
addMeandering(riverCells, riverPoints = null) {
    result = meander(riverCells, p, {
        anchors: riverPoints,          — custom anchors si existen
        meandering: 0.5,
        startStep: h[riverCells[0]] < 20 ? 1 : 10,
        isWaterCell: cells.map(c => c !== -1 && h[c] < 20),
        bounds: { width: graphWidth, height: graphHeight }
    })

    // Interpolación de flux por punto
    flux = new Array(points.length).fill(0)
    anchorIndices.forEach((pointIndex, anchorIndex) => {
        cellId = riverCells[anchorIndex]
        fluxCell = cellId === -1 ? riverCells[anchorIndex-1] : cellId
        flux[pointIndex] = fl[fluxCell] || 0
    })

    return points.map(([x,y], idx) => [x, y, flux[idx]])
}
```

Los puntos resultantes tienen forma `[x, y, flux]`. Los puntos intermedios (no-anchor) tienen flux=0.

---

## 4. Ancho del río

### 4.1 `getOffset({flux, pointIndex, widthFactor, startingWidth})` (líneas 400-418)

```ts
if (pointIndex === 0) return startingWidth;

fluxWidth = min(flux^0.7 / FLUX_FACTOR, MAX_FLUX_WIDTH)
         = min(flux^0.7 / 500, 1.0)

lengthWidth = pointIndex * LENGTH_STEP_WIDTH
            + LENGTH_PROGRESSION[min(pointIndex, 8)]
            = pointIndex / 200 + progression[min(pointIndex, 8)]

LENGTH_PROGRESSION = [1, 1, 2, 3, 5, 8, 13, 21, 34].map(n => n/200)

return widthFactor * (lengthWidth + fluxWidth) + startingWidth
```

Constantes: `FLUX_FACTOR=500`, `MAX_FLUX_WIDTH=1`, `LENGTH_FACTOR=200`, `LENGTH_STEP_WIDTH=1/200`.

### 4.2 `getSourceWidth(flux)` (líneas 420-422)

```ts
return rn(min(flux^0.9 / FLUX_FACTOR, MAX_FLUX_WIDTH), 2)
       = rn(min(flux^0.9 / 500, 1.0), 2)
```

Exponente 0.9 vs 0.7 del fluxWidth — el source crece casi lineal con flujo.

### 4.3 `getWidth(offset)` (líneas 492-494)

```ts
return rn((offset / 1.5)^1.8, 2);  // mouth width in km
```

Convierte offset visual a km. Datos reales comentados: Amazon/Volga ~6km, Dniepr 3km, Mississippi 1.3km, Danube 0.8km, Nile 0.45km.

### 4.4 Cálculo final en defineRivers (líneas 218-226)

```ts
sourceWidth = getSourceWidth(cells.fl[source])
width = getWidth(getOffset({
    flux: discharge,                     // = cells.fl[mouth]
    pointIndex: meanderedPoints.length,  // número de puntos meandeados
    widthFactor,
    startingWidth: sourceWidth
}))
```

### 4.5 Progresión de ancho a lo largo del río

En `getRiverPath`, el flux se actualiza así (línea 435):
```ts
if (pointFlux > flux) flux = pointFlux;
```
`flux` solo crece — nunca decrece. Los puntos intermedios tienen flux=0, así que mantienen el último flux no-cero encontrado.

---

## 5. River path SVG (`getRiverPath`, líneas 425-456)

```ts
getRiverPath(points, widthFactor, startingWidth) {
    this.lineGen.curve(curveCatmullRom.alpha(0.1));   // Catmull-Rom centripetal

    riverPointsLeft = []
    riverPointsRight = []
    flux = 0

    for each point i:
        [x0, y0] = prev point (o self si i=0)
        [x1, y1, pointFlux] = current point
        [x2, y2] = next point (o self si i=last)
        if pointFlux > flux: flux = pointFlux

        offset = getOffset({flux, pointIndex: i, widthFactor, startingWidth})
        angle = atan2(y0 - y2, x0 - x2)
        sinO = sin(angle) * offset
        cosO = cos(angle) * offset

        riverPointsLeft.push([x1 - sinO, y1 + cosO])
        riverPointsRight.push([x1 + sinO, y1 - cosO])

    right = lineGen(riverPointsRight.reverse())    — SVG path reversed
    left = lineGen(riverPointsLeft)                 — SVG path forward
    left = left.substring(left.indexOf("C"))        — quitar "M..." inicial

    return round(right + left, 1)                   — polígono cerrado
}
```

El río es un **polígono cerrado SVG**:
- `lineGen` usa `d3.line().curve(curveCatmullRom.alpha(0.1))` — Catmull-Rom centripetal
- Margen derecha en sentido inverso (mouth→source), margen izquierda en sentido directo
- `left` arranca con `M(x,y)`, se trunca a solo `C...` para unir con el final del `right`
- El resultado es un path SVG que se cierra automáticamente

---

## 6. Lagos y su interacción con ríos (`lakes.ts`)

### 6.1 `Lakes.defineClimateData(heights)` (líneas 49-84)

```ts
for each lake:
    lake.flux = sum(shoreline prec)              — precipitación total
    lake.temp = mean(shoreline temp)
    evaporation = Penman(temp, height_m) * cells
    if NOT closed:
        lake.outCell = lowestShoreCell
        lakeOutCells[outCell] = lake.i
```

Fórmula de Penman: `evaporation = ((700*(temp+0.006*height_m))/50+75)/(80 - temp)`.
Multiplicado por `lake.cells`. Resultado típico: 1-11.

### 6.2 `Lakes.detectCloseLakes(h)` (líneas 87-126)

BFS desde `lowestShorelineCell` hacia afuera, expandiendo vecinos con `h[n] < lake.height + ELEVATION_LIMIT`:
- Si alcanza océano o lago más bajo → abierto (`closed = false`)
- Si no → cerrado (`closed = true`)

### 6.3 `Lakes.cleanupLakeData()` (líneas 31-47)

Limpia campos temporales: `river`, `enteringFlux`, `outCell`, `closed`.
Filtra `inlets` que ya no existen en `pack.rivers`.

### 6.4 Lake outlets en `drainWater()` (líneas 72-105)

Cuando una celda es `outCell` de un lago abierto:
1. Encuentra la celda de lago vecina (`h[lakeCell] < 20 && cells.f[lakeCell] === lake.i`)
2. Transfiere `lake.flux - lake.evaporation` a esa celda
3. Asigna river id a la celda de lago (respetando cadenas de lagos)
4. Pone `lake.outlet = cells.r[lakeCell]`
5. Llama `flowDown(i, cells.fl[lakeCell], lake.outlet)`
6. Asigna parents de inlets al outlet

### 6.5 `resolveLakeDrainFeature` (river-generator.ts:535-557)

Walk por cadena de outlets desde lago hasta océano:
```
start en lakeFeature
if closed → return feature id (terminal)
while river:
    lastCell = river.cells[last]
    if lastCell < 0 → off-map, return null
    feature = cells.f[lastCell]
    if feature === ocean → return ocean id
    if feature !== lake → return null
    if !feature.outlet → closed lake, return feature id
    river = riverById.get(feature.outlet)
```

### 6.6 `resolveDrainFeature` (river-generator.ts:560-582)

Igual pero arranca desde una **celda** (no feature):
```
startRiver = cells.r[cellId]
// Misma lógica que resolveLakeDrainFeature desde ahí
```

Usado en `burgs-generator.ts` y `burg-editor.ts` para asignar puertos.

---

## 7. Formato de datos en el .map

### 7.1 Serialización (`save.ts:93-187`)

| Slot | Campo | Formato |
|---|---|---|
| data[18] | `pack.cells.conf` | CSV de Uint8Array |
| data[22] | `pack.cells.r` | CSV de Uint16Array |
| data[32] | `pack.rivers` | JSON.stringify(array completo) |

### 7.2 Parseo inverso (`load.ts`)

| Slot | Código |
|---|---|
| data[18] | `Uint8Array.from(data[18].split(","), Number)` |
| data[22] | `Uint16Array.from(data[22].split(","), Number)` |
| data[32] | `JSON.parse(data[32])` |

Validación (líneas 600-632): detecta river ids en `cells.r` que no existen en `pack.rivers`. Los limpia (`cells.r[i] = 0`) y borra paths SVG inválidos.

### 7.3 Export JSON (`export-json.ts`)

- `getMinimalDataJson`: rivers directo
- `getPackCellsData`: arrays de `r` y `conf` por celda, rivers array completo

### 7.4 GeoJSON (`export.ts:622-639`)

Re-meadeniza con `Rivers.addMeandering(cells, points)`. Cada río = Feature LineString con propiedades: id, source, mouth, parent, basin, widthFactor, sourceWidth, discharge, name, type.

---

## 8. Resample (`resample.ts`)

### 8.1 `saveRiversData` (líneas 34-39)

Guarda ríos con meanderedPoints pre-calculados antes de destruir el grid.

### 8.2 `restoreRivers` (líneas 142-189)

```ts
restoreRivers(riversData, projection, scale):
    pack.cells.r = new Uint16Array(pack.cells.i.length)
    pack.cells.conf = new Uint8Array(pack.cells.i.length)

    for each river in riversData:
        points = project(meanderedPoints)       — proyecta al nuevo mapa
        cells = points.map(findClosestCell)     — reasigna celdas
        mark confluences                        — conf=1 si celda ya tenía río
        assign cells.r = river.i
        source = cells[0], mouth = cells[-2]
        widthFactor *= scale

    recalc parent, basin, length
```

---

## 9. Editores

### 9.1 River Editor (`river-editor.ts`)

Operaciones:
- Arrastrar control points (swap de río/flux entre celdas)
- Añadir/quitar control points
- Cambiar nombre (culture/random/TTS)
- Cambiar parent (mainstem) y basin (derivado)
- Cambiar `sourceWidth` y `widthFactor` (recalcula width)
- Ver elevation profile
- Borrar río + tributarios

### 9.2 River Creator (`river-creator.ts`)

Creación manual:
- Click cells para añadirlas al río
- Editar flux por celda
- Completar: construye River y SVG path con `addMeandering` + `getRiverPath`
- Abre RiverEditor automáticamente

### 9.3 Rivers Overview (`rivers-overview.ts`)

Tabla con: Name, Type, Discharge, Length, Width, Basin.
Operaciones: sort, search, zoom, edit, remove, add auto, create manual, basin highlight, CSV export, remove all.

---

## 10. Migraciones históricas (`auto-update.ts`)

- **v1.21**: "added rivers data to pack" — construye objetos River desde paths SVG
- **v1.6**: añade `widthFactor`, `discharge`, `width`, `sourceWidth`
- **v1.65**: "changed rivers data" — parsea `d` de paths SVG para reconstruir cells/points

---

## 11. Constantes y semántica crítica

| Constante | Valor | Significado |
|---|---|---|
| `MIN_FLUX_TO_FORM_RIVER` | 30 | Mínimo flujo para proclamar nuevo río |
| `MIN_NAVIGABLE_FLUX` | 100 | Mínimo flujo para celda navegable |
| `FLUX_FACTOR` | 500 | Denominador en cálculo de fluxWidth |
| `MAX_FLUX_WIDTH` | 1 | Tope de componente flux del ancho |
| `LENGTH_FACTOR` | 200 | Denominador en componente length del ancho |
| `MAX_DOWNCUT` | 5 | Máxima erosión por celda |
| `WATER_MEANDER_SCALE` | 0.25 | Factor de reducción de meandro en agua |
| `RELAX_ITERATIONS` | 4 | Iteraciones de relajación de ángulos |
| `river_id = 0` | — | "No river" |
| `river_id ≥ 1` | — | Ríos reales |
| `mouth = cells[-2]` | — | Penúltima celda del array |
| `cell = -1` | — | Off-map (río sale del canvas) |
| `h < 20` | — | Agua |
| `h ≥ 20` | — | Tierra |
| `t[i] < 1` | — | Sin tipo de terreno |

---

## 12. Hallazgos críticos para el port a Voronia

1. **El `mouth` siempre es `riverCells[length-2]`** — la penúltima celda. La última es `-1` (off-map) o celda de agua/lago.

2. **`riverNext` arranca en 1** (0 = "sin río").

3. **Width en 3 componentes**: flux (`flux^0.7/500`), length (Fibonacci/200 capped), startingWidth (`sourceFlux^0.9/500`). Resultado → `(offset/1.5)^1.8` para km.

4. **Downcut** con tope 5 y solo si `h >= 35`.

5. **Depression filling** Priority-Flood con offset `+0.1` (tierra) y `+0.2` (lagos).

6. **`circumcenter` usa `Math.floor`** — crítico para bit-exactitud de la malla.

7. **`alterHeights` produce valores fraccionales**; solo al final se cuantiza a `Uint8Array`.

8. **Fórmula de Penman** para evaporación de lagos.

9. **`cells.conf` realmente es flux de confluencias**, no "confidence" (comentario erróneo en PackedGraph).

10. **El SVG path del río es un polígono cerrado** (ribbon de 2 bancos), no una stroke.

11. **`river.points` es opcional** y se preserva en resample/export solo si el usuario editó el río.

12. **El `.map` no persiste geometría** — load.ts recalcula Voronoi desde `grid.points + boundary`. Bit-exactitud crítica.

13. **`startStep = h[source] < 20 ? 1 : 10`** — si el source está en un lago, mucho meandro al inicio.

14. **Progresión Fibonacci** `[1,1,2,3,5,8,13,21,34]/200` para el componente length del width — los primeros pasos crecen poco, luego aceleran.

15. **`riverTypes` y `specify()`** — los tipos (`River`, `Creek`, `Fork`, `Branch`...) se asignan después de `generate`, no dentro. Pesos ponderados con `rw`. `smallLength` = percentil 15 del orden de longitudes.

16. **`remove(id)` borra tributarios en cascada** — filtra por `r.i === id || r.parent === id || r.basin === id`, restaura `fl` a precipitación original de las celdas afectadas.

17. **`isNavigable(cell)`** = `r[cell] && fl[cell] >= 100` — usado para asignar puertos.

---

## 13. Funciones auxiliares de river-generator (no cubiertas antes)

### 13.1 `riverTypes` (líneas 34-43)

```ts
riverTypes = {
  main: {
    big:   { River: 1 },                      // ríos grandes principales
    small: { Creek: 9, River: 3, Brook: 3, Stream: 1 }  // ríos pequeños principales
  },
  fork: {
    big:   { Fork: 1 },                        // afluentes grandes
    small: { Branch: 1 }                       // afluentes pequeños
  }
}
```

Pesos para `rw` (selección aleatoria ponderada). `Creek` tiene peso 9 (más común), `Stream` peso 1 (raro).

### 13.2 `specify()` (líneas 458-468)

```ts
specify() {
  for (const river of pack.rivers) {
    river.parent = this.getParent(river.i);
    river.basin  = this.getBasin(river.i);
    river.name   = this.getName(river.mouth);
    river.type   = this.getType(river);
  }
}
```

Llamado **fuera** de `generate()` (típicamente tras `generate` y antes de render). Setea `parent`, `basin`, `name`, `type` en cada río.

### 13.3 `getType({i, length, parent})` (líneas 474-483)

```ts
getType(river) {
  if (!smallLength) smallLength = rivers
    .map(r => r.length).sort((a,b) => a-b)
    [Math.ceil(rivers.length * 0.15)];         // percentil 15
  const isSmall = river.length < smallLength;
  const isFork  = river.i % 3 === 0 && river.parent && river.parent !== river.i;
  const pool = riverTypes[isFork ? "fork" : "main"][isSmall ? "small" : "big"];
  return rw(pool);                              // weighted random pick
}
```

- `isFork` requiere: `i % 3 === 0` (cada 3er río) + parent válido + parent ≠ self
- `isSmall` según percentil 15 de longitudes
- Catálogo: `main.big`, `main.small`, `fork.big`, `fork.small`

### 13.4 `getName(cell)` (líneas 470-472)

```ts
getName(cell) { return Names.getCulture(pack.cells.culture[cell]); }
```

Nombre basado en la **cultura de la celda del mouth** (no del source).

### 13.5 `getRiverPoints(riverCells, riverPoints = null)` (líneas 390-398)

```ts
getRiverPoints(riverCells, riverPoints = null) {
  if (riverPoints) return riverPoints;                   // custom anchors (edited rivers)
  return riverCells.map(cell => {
    if (cell === -1) {
      const prev = p[/* cell anterior */];
      return projectToNearestEdge(prev, graphWidth, graphHeight);
    }
    return p[cell];
  });
}
```

Devuelve puntos de centros de celdas, proyectando `-1` al borde más cercano. Usado por controllers (no por `addMeandering` directamente).

### 13.6 `remove(id)` (líneas 497-510)

```ts
remove(id) {
  const removed = pack.rivers.filter(r => 
    r.i === id || r.parent === id || r.basin === id     // tributarios en cascada
  );
  for (const r of removed) {
    select(`#river${r.i}`).remove();                    // borra SVG
    for (const cell of r.cells) {
      if (cells.r[cell] === r.i) {
        cells.r[cell] = 0;
        cells.fl[cell] = grid.cells.prec[cells.g[cell]] / cellsNumberModifier;  // restaura precipitación
        cells.conf[cell] = 0;
      }
    }
  }
  pack.rivers = pack.rivers.filter(r => !removed.includes(r));
}
```

**Borra en cascada**: río + todos sus tributarios directos (parent === id) + toda la cuenca (basin === id). Restaura `fl` a precipitación original (importante para que el agua vuelve a fluir correctamente si se regeneran ríos).

### 13.7 `getParent(r)` / `getBasin(r)` (líneas 512-523)

```ts
getParent(r) {
  const parent = pack.rivers.find(rev => rev.i === r.parent);
  return parent ? parent.i : r.i;                        // self si no existe
}

getBasin(r) {
  if (r.parent && r.parent !== r.i) return getBasin(parent);
  return r.i;                                            // main stem = self basin
}
```

`getBasin` es recursivo: sube por parents hasta llegar a un río sin parent (main stem); ese id es el basin.

### 13.8 `getNextId(rivers)` (líneas 525-527)

```ts
getNextId(rivers) {
  const maxId = rivers.reduce((max, r) => Math.max(max, r.i), 0);
  return maxId + 1;
}
```

### 13.9 `isNavigable(cellId)` (líneas 529-532)

```ts
isNavigable(cellId) {
  return Boolean(cells.r[cellId]) && cells.fl[cellId] >= MIN_NAVIGABLE_FLUX;
}
```

`MIN_NAVIGABLE_FLUX = 100`. Usado en `burgs-generator.ts` para asignar puertos.

---

## 14. Rivers Overview — `src/controllers/rivers-overview.ts`

Tabla de gestión de todos los ríos del mundo. 258 líneas.

### 14.1 UI

- **Header**: 6 columnas sortables — Name, Type, Discharge, Length, Width, Basin
- **Footer**: 4 agregados — número de ríos, avg discharge, avg length, avg width
- **Botones**: Refresh, Add River (auto), Create River (manual), Basin Highlight, Export CSV, Remove All
- **Search**: case-insensitive sobre name / type / basin

### 14.2 Operaciones

| Acción | Detalle |
|---|---|
| Hover | `riverHighlightOn/Off` — pinta stroke red en el path SVG |
| Click target | `zoomToRiver` — `highlightElement(river, 3)` |
| Click pencil | `openRiverEditor` — abre RiverEditor sobre el río |
| Click trash | `triggerRiverRemove` — confirma y `Rivers.remove(id)` |
| Basin highlight | Colorea cada basin con uno de 10 colores d3 (`#1f77b4`, `#ff7f0e`, ..., `#17becf`); toggleable |
| Export CSV | Headers `Id,River,Type,Discharge,Length,Width,Basin`; aplica `distanceScale` a longitudes/anchos |
| Remove all | `pack.rivers = []`, `pack.cells.r = new Uint16Array(...)`, borra SVG |

### 14.3 Filtrado por búsqueda

```ts
riversOverviewAddLines() {
  const search = searchInput.value.toLowerCase();
  for (const river of pack.rivers) {
    const basinName = pack.rivers.find(r => r.i === river.basin)?.name || "";
    if (![river.name, river.type, basinName].some(s => s.toLowerCase().includes(search))) continue;
    // render row
  }
}
```

---

## 15. Coastline editors (no tocan ríos directamente, pero el sub-agente los analizó)

### 15.1 `coastline-editor.ts` (485 líneas)

**No toca ríos directamente**. Es un editor de settings de fractalización de costas:

| Líneas | Sección | Descripción |
|---|---|---|
| 12-20 | `SliderDef` | Define campos: id, label, tip, min, max, step, key |
| 22-95 | `SLIDER_DEFS` | 9 sliders: `maxDepth`, `baseAmplitude`, `amplitudeDecay`, `minEdge`, `smoothThreshold`, `roughnessContrast`, `profileHarmonics`, `lakeSmoothThreshMult` |
| 97-141 | `COAST_PRESETS` | 5 presets: Default, Smooth, Rocky, Fjords, Archipelago — valores hardcodeados |
| 143 | `PREVIEW_SEED` | `"preview_coastline"` — semilla determinista para previews |
| 292-395 | `drawRoughnessGraph` | Canvas con curva de roughness profile dividida en ROUGH (naranja) / CALM (verde) por threshold |
| 397-483 | `drawShapePreview` | Canvas con preview de polígono 4-vertex fractalizado |

La implementación del fractalizador vive en `src/renderers/coastline-fractal.ts` (no leído por el sub-agente — referenceda por imports `buildCoastlinePath`, `fractalize`, `makeRoughnessProfile`). **Relevante para Voronia**: el port del fractalizador de costas ya está hecho en `vor-render/src/coastline.rs` (Fase 6, commits 7f0afbf y 9644bd1), pero Azgaar expone presets y sliders que aún no están en Voronia.

### 15.2 `coastline-vertex-editor.ts` (267 líneas)

Editor de arrastre de vértices individuales de una feature (masa de tierra / lago):

| Líneas | Sección | Descripción |
|---|---|---|
| 5-24 | `open(element)` | Selecciona elemento SVG; celdas vecinas se visualizan como polígonos de debug |
| 58-88 | `drawCoastlineVertices()` | Dibuja vértices como círculos arrastrables + vecinos como polígonos |
| 90-122 | `handleVertexDrag` | En drag: actualiza `vertices.p[vertexId]`, recalcula path SVG con `getFeaturePath(feature)`, recalcula `feature.area = abs(polygonArea(...))` |
| 124-131 | `handleVertexDragEnd` | Re-renderiza states/provinces/borders/biomes/religions/cultures (NO ríos — los ríos no se ven afectados por cambio de coastline) |
| 133-259 | Funciones de grupo | Crear/renombrar/eliminar grupos de coast; `sea_island`/`lake_island` son default no eliminables |

---

## 16. Geometría — `voronoi.ts` + `graphUtils.ts`

### 16.1 `voronoi.ts` (155 líneas)

Construye Voronoi vía Bowyer-Watson con Delaunator.

| Líneas | Función | Descripción |
|---|---|---|
| 18-50 | `class Voronoi` | Constructor: para cada halfedge con `p < pointsN` (no boundary), construye `cells.v[p] = trianglesAround` y `cells.c[p] = adjacent valid cells`, marca `cells.b[p] = 1` si hay más edges que vecinos (border). Para cada triangle: `vertices.p[t] = triangleCenter`, `vertices.v[t] = adjacent triangles`, `vertices.c[t] = pointsOfTriangle` |
| 96-99 | `triangleCenter(t)` | Llama a `circumcenter` de los 3 puntos — **este es el cálculo de Voronoi** (los vértices son los circumcentros) |
| **142-154** | **`circumcenter(a, b, c)`** | Fórmula estándar: `D = 2 * (ax*(by-cy) + bx*(cy-ay) + cx*(ay-by))`, retorna `[(1/D)*(...), (1/D)*(...)]`. **OJO: usa `Math.floor`** en el resultado (líneas 151-152) — produce integer coords. **Crítico para reproducir bit-exacto en Voronia**: si el port no floor, los ids de celda y vecinos mapean distinto. |

### 16.2 `graphUtils.ts` (554 líneas) — lo relevante para ríos

| Líneas | Función | Descripción relacionada con ríos |
|---|---|---|
| 17-37 | `getBoundaryPoints(w, h, spacing)` | Puntos del borde (jittered), concatenados con points internos antes de Delaunay |
| 46-61 | `getJitteredGrid(w, h, spacing)` | Grid cuadrado con jitter × 0.9 del radio del cuadrado — base del pack de celdas |
| 69-98 | `placePoints` | `spacing = sqrt(area / cellsDesired)`, `cellsX = floor((graphWidth + 0.5*spacing) / spacing)` |
| **136-151** | **`generateGrid(seed, w, h)`** | **`Math.random = Alea(seed)` (RESET PRNG CRÍTICO)**, `placePoints` + `calculateVoronoi`. Retorna `Grid` con `seed` guardado |
| **159-177** | **`calculateVoronoi(points, boundary)`** | Une points + boundary, `Delaunator.from(allPoints)`, instancia `Voronoi(delaunay, allPoints, points.length)` (points internos primero, boundary al final; `pointsLength` permite distinguirlos). Crea `cells.i = Uint32Array` |
| 186-191 | `findGridCell(x, y, grid)` | Lookup directo en grid cuadrado: `floor(y/spacing) * cellsX + floor(x/spacing)` |
| **235-250** | **`findClosestCell(x, y, radius, pack)`** | **Quadtree cacheado** (`quadtreeCache` WeakMap por `pack.cells.p`) — usado en `resample.restoreRivers` (mapear points a nuevas cells) y por `findCell` (alias). Crea el quadtree lazy si no está en cache |
| 261-361 | `findAllInQuadtree` | Implementación manual de búsqueda radial en quadtree d3 |
| **384-386** | **`getPackPolygon(cellIndex, packedGraph)`** | `packedGraph.cells.v[cellIndex].map(v => packedGraph.vertices.p[v])` — polígono de una celda. **Usado en todos los controllers de río** (`river-editor.drawCells`, `river-creator.drawCells`) |
| **476-478** | **`isLand(i, packedGraph)`** | `h[i] >= 20` — **VERIFICAR UMBRAL: tierra si h>=20, agua si h<20**. Coherente con todos los usos en river-generator |
| 485-487 | `isWater(i, packedGraph)` | `h[i] < 20` (complemento) |
| 536-553 | Declaración global | `findCell = findClosestCell` (alias global) |

**Datos que `graphUtils` provee a ríos:**
- `cells.c[i]`: vecinos de cada celda (para `drainWater`, `downcutRivers`, `calculateConfluenceFlux`)
- `cells.v[i]`: vértices del polígono (para `getPackPolygon`)
- `cells.p[i]`: posición centro (para meander anchors)
- `cells.b[i]`: near-border (para caso off-map en `drainWater`)
- `vertices.p[v]`, `vertices.c[v]`, `vertices.v[v]`: posiciones de vértices, celdas que toca (3), vértices vecinos
- `cells.g[i]`: index al grid original (para obtener precipitación/temp del grid)

---

## 17. Tests en Azgaar — `river-generator.test.ts` (líneas 4-160)

Tests de `resolveDrainFeature` y `resolveLakeDrainFeature` (oro para el port a Rust):

### `isNavigable` tests
- `r[cell] && fl[cell] >= 100` → true
- Sin río → false
- Con río pero `fl < 100` → false

### `resolveDrainFeature` (7 casos)
1. Río que llega a océano → retorna ocean id
2. Río que llega a lago cerrado (sin outlet) → retorna lake id
3. Río que llega a lago con outlet, que chain-ea a océano → retorna ocean id
4. Río que sale del mapa (lastCell < 0) → retorna `null`
5. Celda sin río (`cells.r[cell] === 0`) → retorna `null`
6. Si `cells.f[lastCell]` no es lago ni océano → retorna `null`
7. River id desconocido → retorna `null`

### `resolveLakeDrainFeature` (7 casos)
Igual que arriba pero arrancando desde feature de lago en lugar de celda.

**Recomendación**: portar estos tests a Rust byte-exacto para validar `vor-sim` cuando se implemente resolveDrainFeature.

---

## 18. Referencias cruzadas útiles (no leídas en profundidad)

| Archivo | Relevancia |
|---|---|
| `src/renderers/coastline-fractal.ts` | Implementa `fractalize`, `buildCoastlinePath`, `makeRoughnessProfile`, `PROFILE_SIZE` (usado por coastline-editor). Ya porteado parcialmente en `vor-render/src/coastline.rs` |
| `src/generators/features.ts` `markupPack` + `defineLakeGroup` (líneas 217+, 373) | Define `shoreline` y clasificación `freshwater`/`salt`/etc. para lagos |
| `src/generators/burgs-generator.ts` (líneas 247, 254) | Asignación de puertos basada en `resolveLakeDrainFeature` y `resolveDrainFeature`. Tests en `burgs-generator.test.ts` cubren promoción de burgs en lagos abiertos |
| `src/controllers/burg-editor.ts` (líneas 421, 424) | Igual que burgs-generator pero al editar burgs individualmente |
| `src/services/io/auto-update.ts` (líneas 254-269, 395-406, 513-554) | Migraciones históricas del modelo River: v1.21 (construye River desde paths SVG), v1.6 (añade widthFactor/discharge/sourceWidth), v1.65 (parse `d` para reconstruir cells/points) |
| `src/generators/river-generator.test.ts` (líneas 4-160) | Tests de `isNavigable`, `resolveDrainFeature` (7 casos) y `resolveLakeDrainFeature` (7 casos) — oro para testear el port a Rust |