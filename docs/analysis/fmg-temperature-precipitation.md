# FMG Temperature & Precipitation — análisis a fondo y plan de port

> **Categoría**: Water & Climate
> **Layers**: temperature, precipitation (en la misma categoría ya están lakes ✅ y rivers ✅ 90%)
> **Referencias azgaar-fmg**: `src/renderers/draw-temperature.ts`, `public/modules/ui/layers.js` (`togglePrecipitation`/`drawPrecipitation`), `public/main.js` (`generatePrecipitation`), `public/styles/default.json`
> **Fecha**: 8 ago 2026

---

## 1. Temperature

### 1.1 Qué hace en Azgaar

Dibuja **isotermas rellenas** (bandas de temperatura) sobre la grilla **grid** (no pack). Cada banda es un `<path>` cerrado y relleno con color del esquema Spectral (azul = frío, rojo = caliente), dibujadas de la más fría a la más caliente para que cada una cubra a las anteriores (facetado, como el heightmap). Las cadenas de vértices se obtienen caminando el **grafo de Voronoi** de celda a celda, filtrando celdas vecinas con `temp >= t` (mismo mecanismo `connectVertices` del heightmap).

Además dibuja **labels de temperatura** (p.ej. `-10°C`) en el centro superior e inferior de cada isoterma — y opcionalmente en el selector de `°C/°F/K/°R/De/N/Ré/Rø`.

### 1.2 Referencias exactas

| Archivo | Líneas | Rol |
|---|---|---|
| `src/renderers/draw-temperature.ts` | 1-138 | Todo: isolines + labels |
| `public/modules/ui/layers.js` | `toggleTemperature` + grupo `#temperature` | Toggle + re-draw |
| `public/styles/default.json` | `#temperature` | `fill:#000000, fill-opacity:0.3, stroke-width:1.8, font-size:8px` |

### 1.3 Algoritmo paso a paso (`draw-temperature.ts`)

Datos: `grid.cells.temp` (`Int8` °C), `grid.vertices` (topología Voronoi), `grid.points`.

1. **Extremos**:
   ```
   tMin = -50, tMax = +50           // extremos soportados
   delta = 100
   minTemp = min(cells.temp)        // datos reales
   maxTemp = max(cells.temp)
   step = max(round(|maxTemp - minTemp| / 5), 1)   // ~5 bandas
   isolines = [minTemp+step, minTemp+2*step, ..., < maxTemp]
   ```
   El `step` se calcula del **rango real de datos** — el render no sabe cuántas bandas habrá hasta leer `min/max`.

2. **Construir cadenas de vértices** (una por nivel):
   - Por cada celda `i` (no visitada, y cuya `temp` esté en `isolines`):
     - `startVertex = findStart(i, t)`:
       - si celda costera (`cells.b[i]`): un vértice con algún vecino fuera del mapa (`c >= n`).
       - si no: el vértice `cells.v[i]` asociado al vecino `cells.c[i]` con `temp < t || !temp` (primer índice).
     - `ofSameType = c => cells.temp[c] >= t`  (NOTA: invertido vs heightmap, que usa `h < h_ref`)
     - `chain = connectVertices({vertices, startingVertex, ofSameType, addToChecked})`
     - **Relax**: `relaxed = chain.filter((v,i) => i % 4 === 0 || neighborVertexOut(v))` — conserva 1 de cada 4 + todos los vértices cuyo vecino está fuera del mapa.
     - si `relaxed.length < 6` → descartar.
     - `points = relaxed.map(v => vertices.p[v])` → coordenadas.
     - `chains.push([t, points])`; registrar label (abajo).

3. **Render**:
   - **Base**: rectángulo `M0,0 h{graphWidth} v{graphHeight} h-{graphWidth} Z` con `fill = scheme(1 - (minTemp-tMin)/delta)` — el color de la temp mínima cubre todo el mapa (fondo).
   - Por cada `t` en `isolines`: `path` = `line().curve(basisClosed)` sobre los points, `fill = scheme(1 - (t-tMin)/delta)`, `stroke = color(fill).darker(0.2)`.
   - Esquema: `scaleSequential(interpolateSpectral)` de d3 — `scheme(x)`, azul @0 → rojo @1.

4. **Datos clave**: el algoritmo opera sobre la **grilla grid**, NO sobre pack. `cells.temp` es el `Int8Array` del `.map` (slot `[11]` del header grid → `grid.cells.temperature` en Voronia).

### 1.4 Detalles de paridad críticos (bit-exact)

- `connectVertices` aquí se **invierte**: `cells.temp[c] >= t` (en heightmap es `h[n] < h`). Además camina tomando el vecino del "otro lado" del contorno, marcando celdas "used".
- **Relax quita 3 de 4 puntos** → resultado mucho más simple que la cadena cruda del heightmap; después se suaviza con `curveBasisClosed`.
- `findStart` prioriza una salida al borde del mapa (`c >= n`) para garantizar cadena que toca el extremo.
- `interpolateSpectral` es **invertido** en la fórmula: para `t` frío, `1-(t-tMin)/delta ≈ 1` → rojo; para `t` cálido, → 0 → azul. Es decir, en Azgaar las isotermas **frías son rojas y las cálidas azules** con este esquema si `temp` está en negativo/positivo... De hecho: Azgaar normaliza `scheme(1 - (t - tMin)/delta)` donde frío = rojo, cálido = azul — **no asumas "rojo = calor"**; replicar la fórmula tal cual, sin «corregir» la semántica.

### 1.5 Requisitos en Voronia (veredicto de portabilidad)

Datos Ya disponibles:
- `world.grid.cells.temperature` → `Vec<i8>` ✅
- `world.grid.vertices` (Voronoi con `positions`, `adjacent_cells`, `cell_rings` repoblados por import) ✅
- `world.grid.points` ✅
- `connectVertices` → ya portado como `vor-render::isoline::connect_vertices` (misma firma: starting vertex, `same_type` callback, `addToChecked` callback, `close_ring`).

El 90% de la maquinaria existe en `vor-render`: `connect_vertices` + `get_isolines`, el bucle de bandas de `build_heightmap_band_mesh`, el esquema espectral ya en `heightmap.rs`. Lo que falta es **específico de temperature**:

1. Fuente = `grid` (no `pack`).
2. El relax `i % 4 === 0 || vertexBorder`.
3. La base `M0,0 hW vH h-W Z` con color `minTemp`.
4. Stroke `.darker(0.2)` en cada banda.
5. Labels de isoterma con `convertTemperature` (escala configurable °C/°F/K/…).

---

## 2. Precipitation

### 2.1 Qué hace en Azgaar

Pinta **círculos azules** en el centro de cada celda de **tierra seca** (`h >= 20`) con precipitación > 0, sobre la grilla **grid**. Radio `r = round(sqrt(prec/4) / cellsNumberModifier, 2)`. Un solo `<circle>` por celda con `fill: #003dff` (azul intenso, sin stroke). Se actualiza con transiciones de 800ms (entrada) y 1000ms (salida).

### 2.2 Referencias exactas

| Área | Líneas | Rol |
|---|---|---|
| `public/modules/ui/layers.js` | 318-358 | `togglePrecipitation` + `drawPrecipitation` |
| `public/styles/default.json` | `#prec` → `fill:#003dff, stroke-width:0` | color círculos |
| `public/index.css` | `#prec text` | `font-size:32px` (label) |
| `public/main.js` | 993+ | `generatePrecipitation` (simulación — origen de `cells.prec`) |

### 2.3 Algoritmo de `drawPrecipitation` (`layers.js`)

```
cellsNumberModifier = (pointsInput.dataset.cells / 10000) ** 0.25
   // 10k celdas → 1.0; 40k → sqrt(2)≈1.414; 2.5k → 0.71
data = cells.i.filter(i => cells.h[i] >= 20 && cells.prec[i])   // solo tierra con prec>0
getRadius = prec => rn(Math.sqrt(prec / 4) / cellsNumberModifier, 2)
circle { cx: grid.points[i][0], cy: grid.points[i][1], r: getRadius(cells.prec[i]) }
```

Notas:
- **No marca negativos ni extremes**: valores `prec` en `[0,255]` (`UInt8Array`).
- El `cellsNumberModifier` escala el radio según la densidad de celdas (celdas / 10000)^0.25 — a más celdas, círculos más pequeños.
- No hay threshold de visibilidad en la simulación; es puramente visual sobre `grid`.
- El ancho del texto del label `#prec text` es de 32px (raro — es la leyenda del control).

### 2.4 Datos requeridos en Voronia

- `world.grid.cells.precipitation` — ya en `GridCells` (`Vec<u8>` en Azgaar tipo `Uint8`; en Voronia `Vec<u16>`) ✅
- `world.grid.cells.height` (para filtrar `h >= 20`) ✅
- `world.grid.points` ✅
- `world.grid.width/height` ✅

**Veredicto port**: es la capa más fácil de todas. Se reduce a:
1. Instanciar círculos GPU (lo más simple: triángulos de unidad escalados) o sprites.
2. Filtrar celdas `h >= 20 && prec > 0`.
3. `r = sqrt(prec/4)/cellsMod` con `cellsMod = (nCells/10000)^0.25`.
4. Color sólido `#003dff`.

---

## 3. Resumen de dependencias y plan

| Capa | Fuente de datos | Ya existe (Voronia) | Falta escribir |
|---|---|---|---|
| temperature | `grid.cells.temp`, `grid.vertices`, `grid.points`, connectVertices | `connect_vertices` reutilizable, `get_isolines`, esquema Spectral `heightmap.rs`, `TextSystem` | bucle grid-iso con relax+base+stroke+darker+labels |
| precipitation | `grid.cells.prec`, `grid.cells.h`, `grid.points` | nada reutilizable (instancing nuevo) | mesh de círculos instancing + filter + radius |

Opciones de pipeline:
- **temperature**: TriangleList (bandas rellenas, como el heightmap) + TextSystem para labels.
- **precipitation**: circles instanced (1 quad por celda escalado) o un pass de sprites; escalar con el radio por celda.

---

## 4. Estado de implementación (verificado contra fuentes reales, 10 ago 2026)

Port **completo y compilando** de la capa de temperatura (`crates/vor-render/src/temperature.rs`, 388 líneas) +
`build_curve_basis_closed` en `crates/vor-render/src/isoline.rs`, cableado en `vor-app` vía `add_layer_mesh_blended`.

### 4.1 Fuentes verificadas (sin asumir)

- `src/renderers/draw-temperature.ts` (master): `minTemp/maxTemp` via `min/max(cells.temp)` con `|| 0`; `step = max(round(|min-max|/5),1)`; `isolines = range(min+step, max, step)`; `connectVertices` **sin** `closeRing`; `relaxed = filter((v,i)=> i%4===0 || vertices.c[v].some(c=>c>=n))`; base `M0,0 hW vH h-W Z` con `scheme(1-(minTemp-tMin)/delta)`; por banda `fill=scheme(1-(t-tMin)/delta)`, `stroke=color(fill).darker(0.2)`.
- `src/utils/pathUtils.ts`: `connectVertices` con `MAX_ITERATIONS`, breaks de seguridad (`next >= vertices.c.length`, `next === current`), `closeRing` **solo** apenda el starting vertex al final; `addToChecked` marca todas las celdas adyacentes `ofSameType`.
- d3-shape `basis.js` + `basisClosed.js`: la curva cerrada emite **n** beziers, uno por triple `(p_j, p_{j+1}, p_{j+2})` para `j = 1..n` (mod n), empezando en `moveTo((p0+4p1+p2)/6)`; el último bezier termina exactamente en el start (sin `Z` explícito en `lineEnd`, pero cierre geométrico exacto).
- `public/styles/default.json` → `#temperature`: `fill:#000000, fill-opacity:0.3, stroke-width:1.8, font-size:8px`. `fill-opacity` afecta solo al fill, no al stroke.

### 4.2 Divergencias detectadas y corregidas (causa del «cerca, aún no exacto»)

1. **Bucle de `build_curve_basis_closed` mal ordenado**: iteraba `j = 0..n` con `(A,B,C)=(p_j,p_{j+1},p_{j+2})` → bezier inicial degenerado (start == end) y el último bezier acababa en `(p_{n-1}+4p0+p1)/6 ≠ start`, cerrando con **línea recta visible**. Corregido a `j = 1..=n` (mod n): el último bezier termina en `(p0+4p1+p2)/6` = start, cierre suave exacto (paridad `basisClosed.js`).
2. **CSS `fill-opacity: 0.3`** no se aplicaba: fills (base + bandas) iban con alpha 1. Corregido con `CSS_FILL_OPACITY = 0.3` aplicado al canal alpha del fill.
3. **`stroke-width`**: el port asumía el default SVG `1`; la CSS `#temperature` define `1.8`. Corregido (`CSS_STROKE_WIDTH = 1.8`) y el **stroke mantiene alpha 1.0** (el `fill-opacity` de la CSS no le afecta; antes heredaba el alpha del fill).
4. **`d3_range`**: la implementación acumulaba `v += step` (drift de coma flotante); d3 usa `k = ceil((stop-start)/step)` y `start + i*step`. Corregido.
5. **`FillRule::EvenOdd`**: SVG usa `nonzero` por defecto. Corregido a `NonZero` en base + bandas (visualmente raro que difiera en estas figuras, pero parity).
6. **Pipeline**: la capa se añadía con `add_layer_mesh` (pipeline opaco REPLACE) → el alpha 0.3 se perdía por completo. Cambiado a `add_layer_mesh_blended` en `vor-app/src/lib.rs`.

### 4.3 Paridad confirmada (no requirió cambio)

- `connect_vertices` (isoline.rs) ya coincidía: `close_ring=false` para temperatura, `previous = u32::MAX` ≈ `undefined` de JS en la 1ª iteración, salida del loop cuando `next == starting_vertex`.
- `find_start`: `cells.c` filtrado (`c < pointsN`) en `vor-import` coincide con Azgaar; para celdas interiores `cell_rings` y `cell_neighbors` están alineados (mismo orden `edgesAroundPoint`), para celdas de borde se usa la rama `cells.b[i]` (no depende de la alineación). `!cells.temp[c]` ⇔ `temp == 0.0` preservado.
- `minTemp/maxTemp` con `|| 0`, `n = cells.i.length = points_n()`, loop sobre `0..n` (equivalente a `for cellId of cells.i`, que es `[0..n)`).
- `is_border_vertex` usa `vertices.c[v].some(c >= n)` con ids de borde conservados en `adjacent_cells` (verificado: `regraph.rs` los copia tal cual).
- `temp_color(t) = spectral_linear(1-(t-T_MIN)/DELTA)` con clamp `[0,1]` de `spectral` (dominio de `scaleSequential`; fuera de rango es extrapolación en d3, inalcanzable en datos reales).
- `darker(0.2)` aplicado en espacio sRGB con round-trip a linear para los vértices.

### 4.4 Pendiente (fuera de alcance de esta sesión)

- **Labels de temperatura**: `addLabel`/`pushLabel` (font-size 8px, `leastIndex` top/bottom center) + `convertTemperature` (escala °C/°F/K/…) — requieren `TextSystem`.
- Redondeo `round(path,1)` vía `rn = Math.round(v*10)/10`: en Rust se usa `f32::round` (half-away-from-zero); `Math.round` es half-up → divergencia solo en `.5` exacto negativo, despreciable.