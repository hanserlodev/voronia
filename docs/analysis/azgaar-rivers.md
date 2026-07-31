# Exhaustive analysis of Azgaar FMG's river system

> Date: 29 jul 2026
> Source: azgaar-fmg TypeScript source code (HEAD commit in `/home/hans/Proyectos/azgaar-fmg/`)
> Purpose: reference for the native port in Voronia

## Files analyzed

| # | File | Lines | Role |
|---|---|---|---|
| 1 | `src/generators/river-generator.ts` | 589 | Main generation engine and width calculus |
| 2 | `src/utils/pathUtils.ts` | 526 | Meandering + SVG banking utilities |
| 3 | `src/generators/lakes.ts` | 129 | Lake climate, closed-lake detection, cleanup |
| 4 | `src/generators/features.ts` | 386 | `Feature` type with `outlet`/`outCell`/`inlets`/`shoreline` |
| 5 | `src/generators/resample.ts` | 471 | Re-sampling with `restoreRivers` |
| 6 | `src/types/PackedGraph.ts` | 70 | `PackedGraph` interface with `cells.r`, `cells.conf`, `rivers` |
| 7 | `src/generators/voronoi.ts` | 155 | Generates adjacency `cells.c`, `cells.v`, vertices |
| 8 | `src/utils/graphUtils.ts` | 554 | `calculateVoronoi`, `findClosestCell`, `getPackPolygon` |
| 9 | `src/controllers/river-editor.ts` | 350 | Visual editor for existing rivers |
| 10 | `src/controllers/river-creator.ts` | 172 | Manual river creation |
| 11 | `src/controllers/rivers-overview.ts` | 258 | River table + CSV export + basin highlight |
| 12 | `src/services/io/save.ts` | 215 | .map serialization (slots 18, 22, 32) |
| 13 | `src/services/io/load.ts` | 847 | .map parsing + integrity validation |
| 14 | `src/services/io/export-json.ts` | 233 | JSON export (minimal + full pack) |
| 15 | `src/services/io/export.ts` | 807 | River GeoJSON export |
| 16 | `src/services/io/auto-update.ts` | — | v1.21, v1.6, v1.65 migrations |

---

## 1. General architecture

Rivers in Azgaar are modeled as:

1. **`pack.rivers`**: array of `River` objects, one per river. Each river has a unique id (1-based), source, mouth, the cells it traverses, anchor points, and width parameters.
2. **`pack.cells.r`**: `Uint16Array` indexed by pack cell — the id of the river passing through that cell (0 = no river).
3. **`pack.cells.fl`**: `Uint16Array` — water flow (flux) at each cell, in m³/s.
4. **`pack.cells.conf`**: `Uint8Array` — flux accumulated at confluences (all tributaries except the main one).

### 1.1 River interface (`river-generator.ts:9-24`)

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

### 1.2 Relevant PackedGraph interface (`PackedGraph.ts:18-70`)

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

## 2. Generation pipeline (`river-generator.ts:47-294`)

### 2.1 Execution order

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

### 2.2 `alterHeights()` (lines 296-306)

```ts
alterHeights(): number[] {
  return Array.from(h).map((h, i) => {
    if (h < 20 || t[i] < 1) return h;    // agua o sin tipo: sin cambios
    return h + t[i]/100 + mean(c[i].map(c => t[c]))/10000;
  });
}
```

- Returns `Array<number>` (not TypedArray) — allows fractions
- Land cells are raised according to their terrain type and the average of their neighbors
- The result is used for flow calculations; the original `cells.h` is preserved

### 2.3 `resolveDepressions(h)` (lines 309-367)

Priority-Flood variant (Barnes):
- Processes cells from lowest to highest height
- Raises depressed cells to `min(neighbor height) + 0.1`
- Lakes: attempts to raise `l.height` to `min(shoreline) + 0.2` (first 75% iterations)
- After 75% of iterations: resistant lakes are marked `closed = true`
- If progression worsens (>0 delta over 5 iterations), aborts and restarts h
- Maximum iterations from UI (`lakeElevationLimitOutput`)

### 2.4 `drainWater()` (lines 60-150)

Main drainage algorithm:

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

Key constants:
- `MIN_FLUX_TO_FORM_RIVER = 30`
- `cellsNumberModifier = (cells/10000)^0.25`

### 2.5 `flowDown(toCell, fromFlux, river)` (lines 152-186)

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

### 2.6 `defineRivers()` (lines 188-241)

Builds the `pack.rivers` array:

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

#### Mouth: the penultimate cell

```ts
const mouth = riverCells[riverCells.length - 2];
```

The last cell of the `riverCells` array is:
- `-1` if the river exits the map (case `cells.b[i] && cells.r[i]`)
- A water/lake cell where it discharges

### 2.7 `calculateConfluenceFlux()` (lines 259-272)

```ts
for each cell with conf[i]:
    influx = vecinos con río y más altos (entrantes) -> sus flujos
    sort influx descending
    cells.conf[i] = sum(influx[1:])   — suma todos EXCEPTO el mayor
```

The flux of the main river at the confluence is NOT counted as "confluence flux".

### 2.8 `downcutRivers()` (lines 243-257)

```ts
MAX_DOWNCUT = 5
for each cell:
    if h[i] < 35 || !fl[i]: continue    — no erosiona tierras bajas
    higherFlux = mean(fl de vecinos más altos)
    if !higherFlux: continue
    downcut = floor(fl[i] / higherFlux)
    if downcut: h[i] -= min(downcut, MAX_DOWNCUT)
```

- Erodes only if `h >= 35`
- The `fl[i]/higherFlux` ratio determines how much to erode
- Maximum of 5 height units per cell

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

Characteristics:
- `meanderVal` decreases naturally: `1/step` shrinks, `max(0.5-step/100,0)` becomes 0 when `step > 50`
- In water cells: meander × 0.25 (almost straight)
- `startStep` = `h[riverCells[0]] < 20 ? 1 : 10` (if source is in a lake, starts with step 1 = heavy meandering until it exits)
- Short segments (dist² ≤ 25, ~5px) are skipped if there are ≥6 cells

### 3.2 `relaxAcuteAngles(points, anchorIndices)` (lines 453-506)

Relaxation of acute angles via reflection across the base line between anchors:

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

- `acuteCost(i)` = `max(cos(angle at vertex i), 0)` — penalizes angles < 90°
- `reflectAcrossLine(m, P, Q)` reflects `m` across the line `PQ`
- Only moves non-anchor points; anchors are fixed

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

The resulting points have the form `[x, y, flux]`. Intermediate (non-anchor) points have flux=0.

---

## 4. River width

### 4.1 `getOffset({flux, pointIndex, widthFactor, startingWidth})` (lines 400-418)

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

Constants: `FLUX_FACTOR=500`, `MAX_FLUX_WIDTH=1`, `LENGTH_FACTOR=200`, `LENGTH_STEP_WIDTH=1/200`.

### 4.2 `getSourceWidth(flux)` (lines 420-422)

```ts
return rn(min(flux^0.9 / FLUX_FACTOR, MAX_FLUX_WIDTH), 2)
       = rn(min(flux^0.9 / 500, 1.0), 2)
```

Exponent 0.9 vs 0.7 of fluxWidth — the source grows almost linearly with flow.

### 4.3 `getWidth(offset)` (lines 492-494)

```ts
return rn((offset / 1.5)^1.8, 2);  // mouth width in km
```

Converts visual offset to km. Real data (commented in code): Amazon/Volga ~6km, Dniepr 3km, Mississippi 1.3km, Danube 0.8km, Nile 0.45km.

### 4.4 Final calculation in defineRivers (lines 218-226)

```ts
sourceWidth = getSourceWidth(cells.fl[source])
width = getWidth(getOffset({
    flux: discharge,                     // = cells.fl[mouth]
    pointIndex: meanderedPoints.length,  // número de puntos meandeados
    widthFactor,
    startingWidth: sourceWidth
}))
```

### 4.5 Width progression along the river

In `getRiverPath`, the flux is updated as follows (line 435):
```ts
if (pointFlux > flux) flux = pointFlux;
```
`flux` only grows — it never decreases. Intermediate points have flux=0, so they keep the last non-zero flux encountered.

---

## 5. River path SVG (`getRiverPath`, lines 425-456)

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

The river is a **closed SVG polygon**:
- `lineGen` uses `d3.line().curve(curveCatmullRom.alpha(0.1))` — centripetal Catmull-Rom
- Right bank in reverse order (mouth→source), left bank in forward order
- `left` starts with `M(x,y)`, truncated to just `C...` to join with the end of `right`
- The result is an SVG path that closes automatically

---

## 6. Lakes and their interaction with rivers (`lakes.ts`)

### 6.1 `Lakes.defineClimateData(heights)` (lines 49-84)

```ts
for each lake:
    lake.flux = sum(shoreline prec)              — precipitación total
    lake.temp = mean(shoreline temp)
    evaporation = Penman(temp, height_m) * cells
    if NOT closed:
        lake.outCell = lowestShoreCell
        lakeOutCells[outCell] = lake.i
```

Penman formula: `evaporation = ((700*(temp+0.006*height_m))/50+75)/(80 - temp)`.
Multiplied by `lake.cells`. Typical result: 1-11.

### 6.2 `Lakes.detectCloseLakes(h)` (lines 87-126)

BFS from `lowestShorelineCell` outward, expanding neighbors with `h[n] < lake.height + ELEVATION_LIMIT`:
- If it reaches ocean or a lower lake → open (`closed = false`)
- If not → closed (`closed = true`)

### 6.3 `Lakes.cleanupLakeData()` (lines 31-47)

Cleans up temporary fields: `river`, `enteringFlux`, `outCell`, `closed`.
Filters `inlets` that no longer exist in `pack.rivers`.

### 6.4 Lake outlets in `drainWater()` (lines 72-105)

When a cell is the `outCell` of an open lake:
1. Finds the neighboring lake cell (`h[lakeCell] < 20 && cells.f[lakeCell] === lake.i`)
2. Transfers `lake.flux - lake.evaporation` to that cell
3. Assigns the river id to the lake cell (respecting chains of lakes)
4. Sets `lake.outlet = cells.r[lakeCell]`
5. Calls `flowDown(i, cells.fl[lakeCell], lake.outlet)`
6. Assigns the parents of the inlets to the outlet

### 6.5 `resolveLakeDrainFeature` (river-generator.ts:535-557)

Walk through the chain of outlets from the lake to the ocean:
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

Same but starts from a **cell** (not a feature):
```
startRiver = cells.r[cellId]
// Misma lógica que resolveLakeDrainFeature desde ahí
```

Used in `burgs-generator.ts` and `burg-editor.ts` to assign ports.

---

## 7. Data format in the .map

### 7.1 Serialization (`save.ts:93-187`)

| Slot | Field | Format |
|---|---|---|
| data[18] | `pack.cells.conf` | CSV of Uint8Array |
| data[22] | `pack.cells.r` | CSV of Uint16Array |
| data[32] | `pack.rivers` | JSON.stringify(full array) |

### 7.2 Reverse parsing (`load.ts`)

| Slot | Code |
|---|---|
| data[18] | `Uint8Array.from(data[18].split(","), Number)` |
| data[22] | `Uint16Array.from(data[22].split(","), Number)` |
| data[32] | `JSON.parse(data[32])` |

Validation (lines 600-632): detects river ids in `cells.r` that do not exist in `pack.rivers`. Cleans them (`cells.r[i] = 0`) and deletes invalid SVG paths.

### 7.3 JSON export (`export-json.ts`)

- `getMinimalDataJson`: rivers directly
- `getPackCellsData`: `r` and `conf` arrays per cell, full rivers array

### 7.4 GeoJSON (`export.ts:622-639`)

Re-meanders with `Rivers.addMeandering(cells, points)`. Each river = a Feature LineString with properties: id, source, mouth, parent, basin, widthFactor, sourceWidth, discharge, name, type.

---

## 8. Resample (`resample.ts`)

### 8.1 `saveRiversData` (lines 34-39)

Saves rivers with pre-computed meanderedPoints before destroying the grid.

### 8.2 `restoreRivers` (lines 142-189)

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

## 9. Editors

### 9.1 River Editor (`river-editor.ts`)

Operations:
- Drag control points (swap of river/flux between cells)
- Add/remove control points
- Change name (culture/random/TTS)
- Change parent (mainstem) and basin (derived)
- Change `sourceWidth` and `widthFactor` (recalculates width)
- View elevation profile
- Delete river + tributaries

### 9.2 River Creator (`river-creator.ts`)

Manual creation:
- Click cells to add them to the river
- Edit flux per cell
- Complete: builds River and SVG path with `addMeandering` + `getRiverPath`
- Opens RiverEditor automatically

### 9.3 Rivers Overview (`rivers-overview.ts`)

Table with: Name, Type, Discharge, Length, Width, Basin.
Operations: sort, search, zoom, edit, remove, add auto, create manual, basin highlight, CSV export, remove all.

---

## 10. Historical migrations (`auto-update.ts`)

- **v1.21**: "added rivers data to pack" — builds River objects from SVG paths
- **v1.6**: adds `widthFactor`, `discharge`, `width`, `sourceWidth`
- **v1.65**: "changed rivers data" — parses `d` of SVG paths to rebuild cells/points

---

## 11. Constants and critical semantics

| Constant | Value | Meaning |
|---|---|---|
| `MIN_FLUX_TO_FORM_RIVER` | 30 | Minimum flux to declare a new river |
| `MIN_NAVIGABLE_FLUX` | 100 | Minimum flux for a navigable cell |
| `FLUX_FACTOR` | 500 | Denominator in the fluxWidth calculation |
| `MAX_FLUX_WIDTH` | 1 | Cap of the flux component of width |
| `LENGTH_FACTOR` | 200 | Denominator in the length component of width |
| `MAX_DOWNCUT` | 5 | Maximum erosion per cell |
| `WATER_MEANDER_SCALE` | 0.25 | Meander reduction factor in water |
| `RELAX_ITERATIONS` | 4 | Angle relaxation iterations |
| `river_id = 0` | — | "No river" |
| `river_id ≥ 1` | — | Real rivers |
| `mouth = cells[-2]` | — | Penultimate cell of the array |
| `cell = -1` | — | Off-map (river exits the canvas) |
| `h < 20` | — | Water |
| `h ≥ 20` | — | Land |
| `t[i] < 1` | — | No terrain type |

---

## 12. Critical findings for the Voronia port

1. **The `mouth` is always `riverCells[length-2]`** — the penultimate cell. The last one is `-1` (off-map) or a water/lake cell.

2. **`riverNext` starts at 1** (0 = "no river").

3. **Width in 3 components**: flux (`flux^0.7/500`), length (Fibonacci/200 capped), startingWidth (`sourceFlux^0.9/500`). Result → `(offset/1.5)^1.8` for km.

4. **Downcut** capped at 5 and only if `h >= 35`.

5. **Depression filling** Priority-Flood with offset `+0.1` (land) and `+0.2` (lakes).

6. **`circumcenter` uses `Math.floor`** — critical for bit-exactness of the mesh.

7. **`alterHeights` produces fractional values**; only at the end is it quantized to `Uint8Array`.

8. **Penman formula** for lake evaporation.

9. **`cells.conf` is really confluence flux**, not "confidence" (erroneous comment in PackedGraph).

10. **The river SVG path is a closed polygon** (ribbon of 2 banks), not a stroke.

11. **`river.points` is optional** and is preserved in resample/export only if the user edited the river.

12. **The `.map` does not persist geometry** — load.ts recomputes Voronoi from `grid.points + boundary`. Bit-exactness is critical.

13. **`startStep = h[source] < 20 ? 1 : 10`** — if the source is in a lake, heavy meandering at the start.

14. **Fibonacci progression** `[1,1,2,3,5,8,13,21,34]/200` for the length component of width — the first steps grow little, then accelerate.

15. **`riverTypes` and `specify()`** — the types (`River`, `Creek`, `Fork`, `Branch`...) are assigned after `generate`, not inside it. Weights weighted with `rw`. `smallLength` = 15th percentile of the length ordering.

16. **`remove(id)` deletes tributaries in cascade** — filters by `r.i === id || r.parent === id || r.basin === id`, restores `fl` to the original precipitation of the affected cells.

17. **`isNavigable(cell)`** = `r[cell] && fl[cell] >= 100` — used to assign ports.

---

## 13. Auxiliary river-generator functions (not covered before)

### 13.1 `riverTypes` (lines 34-43)

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

Weights for `rw` (weighted random selection). `Creek` has weight 9 (most common), `Stream` weight 1 (rare).

### 13.2 `specify()` (lines 458-468)

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

Called **outside** `generate()` (typically after `generate` and before render). Sets `parent`, `basin`, `name`, `type` on each river.

### 13.3 `getType({i, length, parent})` (lines 474-483)

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

- `isFork` requires: `i % 3 === 0` (every 3rd river) + valid parent + parent ≠ self
- `isSmall` based on the 15th percentile of lengths
- Catalog: `main.big`, `main.small`, `fork.big`, `fork.small`

### 13.4 `getName(cell)` (lines 470-472)

```ts
getName(cell) { return Names.getCulture(pack.cells.culture[cell]); }
```

Name based on the **culture of the mouth cell** (not the source).

### 13.5 `getRiverPoints(riverCells, riverPoints = null)` (lines 390-398)

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

Returns points of cell centers, projecting `-1` to the nearest edge. Used by controllers (not by `addMeandering` directly).

### 13.6 `remove(id)` (lines 497-510)

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

**Deletes in cascade**: the river + all its direct tributaries (parent === id) + the whole basin (basin === id). Restores `fl` to the original precipitation (important so the water flows correctly again if rivers are regenerated).

### 13.7 `getParent(r)` / `getBasin(r)` (lines 512-523)

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

`getBasin` is recursive: it climbs up the parents until it reaches a river without a parent (main stem); that id is the basin.

### 13.8 `getNextId(rivers)` (lines 525-527)

```ts
getNextId(rivers) {
  const maxId = rivers.reduce((max, r) => Math.max(max, r.i), 0);
  return maxId + 1;
}
```

### 13.9 `isNavigable(cellId)` (lines 529-532)

```ts
isNavigable(cellId) {
  return Boolean(cells.r[cellId]) && cells.fl[cellId] >= MIN_NAVIGABLE_FLUX;
}
```

`MIN_NAVIGABLE_FLUX = 100`. Used in `burgs-generator.ts` to assign ports.

---

## 14. Rivers Overview — `src/controllers/rivers-overview.ts`

Management table of all the world's rivers. 258 lines.

### 14.1 UI

- **Header**: 6 sortable columns — Name, Type, Discharge, Length, Width, Basin
- **Footer**: 4 aggregates — number of rivers, avg discharge, avg length, avg width
- **Buttons**: Refresh, Add River (auto), Create River (manual), Basin Highlight, Export CSV, Remove All
- **Search**: case-insensitive over name / type / basin

### 14.2 Operations

| Action | Detail |
|---|---|
| Hover | `riverHighlightOn/Off` — paints red stroke on the SVG path |
| Click target | `zoomToRiver` — `highlightElement(river, 3)` |
| Click pencil | `openRiverEditor` — opens RiverEditor on the river |
| Click trash | `triggerRiverRemove` — confirms and `Rivers.remove(id)` |
| Basin highlight | Colors each basin with one of 10 d3 colors (`#1f77b4`, `#ff7f0e`, ..., `#17becf`); toggleable |
| Export CSV | Headers `Id,River,Type,Discharge,Length,Width,Basin`; applies `distanceScale` to lengths/widths |
| Remove all | `pack.rivers = []`, `pack.cells.r = new Uint16Array(...)`, deletes SVG |

### 14.3 Search filtering

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

## 15. Coastline editors (do not touch rivers directly, but the sub-agent analyzed them)

### 15.1 `coastline-editor.ts` (485 lines)

**Does not touch rivers directly**. It is an editor of coastline fractalization settings:

| Lines | Section | Description |
|---|---|---|
| 12-20 | `SliderDef` | Defines fields: id, label, tip, min, max, step, key |
| 22-95 | `SLIDER_DEFS` | 9 sliders: `maxDepth`, `baseAmplitude`, `amplitudeDecay`, `minEdge`, `smoothThreshold`, `roughnessContrast`, `profileHarmonics`, `lakeSmoothThreshMult` |
| 97-141 | `COAST_PRESETS` | 5 presets: Default, Smooth, Rocky, Fjords, Archipelago — hardcoded values |
| 143 | `PREVIEW_SEED` | `"preview_coastline"` — deterministic seed for previews |
| 292-395 | `drawRoughnessGraph` | Canvas with roughness profile curve divided into ROUGH (orange) / CALM (green) by threshold |
| 397-483 | `drawShapePreview` | Canvas with preview of a fractalized 4-vertex polygon |

The fractalizer implementation lives in `src/renderers/coastline-fractal.ts` (not read by the sub-agent — referenced by the imports `buildCoastlinePath`, `fractalize`, `makeRoughnessProfile`). **Relevant for Voronia**: the port of the coastline fractalizer is already done in `vor-render/src/coastline.rs` (Phase 6, commits 7f0afbf and 9644bd1), but Azgaar exposes presets and sliders that are not yet in Voronia.

### 15.2 `coastline-vertex-editor.ts` (267 lines)

Editor for dragging individual vertices of a feature (landmass / lake):

| Lines | Section | Description |
|---|---|---|
| 5-24 | `open(element)` | Selects the SVG element; neighboring cells are shown as debug polygons |
| 58-88 | `drawCoastlineVertices()` | Draws vertices as draggable circles + neighbors as polygons |
| 90-122 | `handleVertexDrag` | On drag: updates `vertices.p[vertexId]`, recomputes the SVG path with `getFeaturePath(feature)`, recomputes `feature.area = abs(polygonArea(...))` |
| 124-131 | `handleVertexDragEnd` | Re-renders states/provinces/borders/biomes/religions/cultures (NOT rivers — rivers are not affected by coastline changes) |
| 133-259 | Group functions | Create/rename/delete coast groups; `sea_island`/`lake_island` are default and cannot be deleted |

---

## 16. Geometry — `voronoi.ts` + `graphUtils.ts`

### 16.1 `voronoi.ts` (155 lines)

Builds Voronoi via Bowyer-Watson with Delaunator.

| Lines | Function | Description |
|---|---|---|
| 18-50 | `class Voronoi` | Constructor: for each halfedge with `p < pointsN` (not boundary), builds `cells.v[p] = trianglesAround` and `cells.c[p] = adjacent valid cells`, marks `cells.b[p] = 1` if there are more edges than neighbors (border). For each triangle: `vertices.p[t] = triangleCenter`, `vertices.v[t] = adjacent triangles`, `vertices.c[t] = pointsOfTriangle` |
| 96-99 | `triangleCenter(t)` | Calls `circumcenter` of the 3 points — **this is the Voronoi computation** (the vertices are the circumcenters) |
| **142-154** | **`circumcenter(a, b, c)`** | Standard formula: `D = 2 * (ax*(by-cy) + bx*(cy-ay) + cx*(ay-by))`, returns `[(1/D)*(...), (1/D)*(...)]`. **NOTE: uses `Math.floor`** on the result (lines 151-152) — produces integer coords. **Critical to reproduce bit-exact in Voronia**: if the port does not floor, cell ids and neighbors map differently. |

### 16.2 `graphUtils.ts` (554 lines) — the part relevant to rivers

| Lines | Function | Description related to rivers |
|---|---|---|
| 17-37 | `getBoundaryPoints(w, h, spacing)` | Boundary points (jittered), concatenated with internal points before Delaunay |
| 46-61 | `getJitteredGrid(w, h, spacing)` | Square grid with jitter × 0.9 of the square radius — the base of the cell pack |
| 69-98 | `placePoints` | `spacing = sqrt(area / cellsDesired)`, `cellsX = floor((graphWidth + 0.5*spacing) / spacing)` |
| **136-151** | **`generateGrid(seed, w, h)`** | **`Math.random = Alea(seed)` (CRITICAL PRNG RESET)**, `placePoints` + `calculateVoronoi`. Returns `Grid` with the `seed` saved |
| **159-177** | **`calculateVoronoi(points, boundary)`** | Concatenates points + boundary, `Delaunator.from(allPoints)`, instantiates `Voronoi(delaunay, allPoints, points.length)` (internal points first, boundary last; `pointsLength` allows distinguishing them). Creates `cells.i = Uint32Array` |
| 186-191 | `findGridCell(x, y, grid)` | Direct lookup in the square grid: `floor(y/spacing) * cellsX + floor(x/spacing)` |
| **235-250** | **`findClosestCell(x, y, radius, pack)`** | **Cached quadtree** (`quadtreeCache` WeakMap keyed by `pack.cells.p`) — used in `resample.restoreRivers` (mapping points to new cells) and by `findCell` (alias). Creates the quadtree lazily if not cached |
| 261-361 | `findAllInQuadtree` | Manual implementation of radial search in the d3 quadtree |
| **384-386** | **`getPackPolygon(cellIndex, packedGraph)`** | `packedGraph.cells.v[cellIndex].map(v => packedGraph.vertices.p[v])` — a cell's polygon. **Used in all river controllers** (`river-editor.drawCells`, `river-creator.drawCells`) |
| **476-478** | **`isLand(i, packedGraph)`** | `h[i] >= 20` — **VERIFY THRESHOLD: land if h>=20, water if h<20**. Consistent with all uses in river-generator |
| 485-487 | `isWater(i, packedGraph)` | `h[i] < 20` (complement) |
| 536-553 | Global declaration | `findCell = findClosestCell` (global alias) |

**Data that `graphUtils` provides to rivers:**
- `cells.c[i]`: neighbors of each cell (for `drainWater`, `downcutRivers`, `calculateConfluenceFlux`)
- `cells.v[i]`: polygon vertices (for `getPackPolygon`)
- `cells.p[i]`: center position (for meander anchors)
- `cells.b[i]`: near-border (for the off-map case in `drainWater`)
- `vertices.p[v]`, `vertices.c[v]`, `vertices.v[v]`: vertex positions, cells it touches (3), neighboring vertices
- `cells.g[i]`: index into the original grid (to obtain precipitation/temp from the grid)

---

## 17. Tests in Azgaar — `river-generator.test.ts` (lines 4-160)

Tests of `resolveDrainFeature` and `resolveLakeDrainFeature` (gold for the Rust port):

### `isNavigable` tests
- `r[cell] && fl[cell] >= 100` → true
- No river → false
- With river but `fl < 100` → false

### `resolveDrainFeature` (7 cases)
1. River that reaches the ocean → returns ocean id
2. River that reaches a closed lake (no outlet) → returns lake id
3. River that reaches a lake with an outlet, which chains to the ocean → returns ocean id
4. River that exits the map (lastCell < 0) → returns `null`
5. Cell without a river (`cells.r[cell] === 0`) → returns `null`
6. If `cells.f[lastCell]` is neither lake nor ocean → returns `null`
7. Unknown river id → returns `null`

### `resolveLakeDrainFeature` (7 cases)
Same as above but starting from a lake feature instead of a cell.

**Recommendation**: port these tests byte-exact to Rust to validate `vor-sim` when resolveDrainFeature is implemented.

---

## 18. Useful cross-references (not read in depth)

| File | Relevance |
|---|---|
| `src/renderers/coastline-fractal.ts` | Implements `fractalize`, `buildCoastlinePath`, `makeRoughnessProfile`, `PROFILE_SIZE` (used by coastline-editor). Already partially ported in `vor-render/src/coastline.rs` |
| `src/generators/features.ts` `markupPack` + `defineLakeGroup` (lines 217+, 373) | Defines `shoreline` and the `freshwater`/`salt`/etc. classification for lakes |
| `src/generators/burgs-generator.ts` (lines 247, 254) | Port assignment based on `resolveLakeDrainFeature` and `resolveDrainFeature`. Tests in `burgs-generator.test.ts` cover burg promotion in open lakes |
| `src/controllers/burg-editor.ts` (lines 421, 424) | Same as burgs-generator but when editing burgs individually |
| `src/services/io/auto-update.ts` (lines 254-269, 395-406, 513-554) | Historical migrations of the River model: v1.21 (builds River from SVG paths), v1.6 (adds widthFactor/discharge/sourceWidth), v1.65 (parses `d` to rebuild cells/points) |
| `src/generators/river-generator.test.ts` (lines 4-160) | Tests of `isNavigable`, `resolveDrainFeature` (7 cases) and `resolveLakeDrainFeature` (7 cases) — gold for testing the Rust port |
