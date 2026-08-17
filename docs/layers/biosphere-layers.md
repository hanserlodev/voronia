# Azgaar Biosphere Layers — Documentation and porting plan

> **Category**: Biosphere
> **Layers**: biomes (fill + water gap), relief icons (trees/mountains)
> **Source**: Azgaar's FMP v1.138.0 (checkout local `51d8e3e`); docs anteriores referencian v1.135.2 — los archivos citados no han cambiado de comportamiento en estas versiones
> **Draw order Azgaar** (z-order real, `public/main.js`): `capa 6` = `#biomes`, `capa 12` = `#terrain`

La categoría Biosphere cubre las dos capas "vivas" del mundo: **biomes** (el relleno de vegetación por celda, basado en temperatura+humedad) y **relief icons** (los iconos de árboles/montañas que sobreimprimen la vegetación).

---

## 1. Biomes

### Qué hace en Azgaar

Rellena cada región de bioma con su color, agrupando las celdas de un mismo bioma en **isolines** (contornos polígonales) en vez de pintar celda por celda, igual que el heightmap y la temperatura. El color sale del catálogo `biomesData.color[biome]`; además dibuja un **water gap** (trazado de borde del mismo color, `stroke-width:3`) para que el color no sangre al océano.

### Data consumed

| Slot | Voronia Field | Type | Descripción |
|------|---------------|------|-------------|
| `[3]` | `world.biomes` | `Vec<Biome>` | Catálogo: `color|habitability[name]` pipe-CSV (ver §1.3) |
| `[16]` | `cells.biome` | `Vec<u8>` | biome id por celda (0 = Marine) |
| — | `biomesData.biomesMatrix` | `[[u8;26];5]` | matriz `[humedad band][temperatura band] → biome` |
| — | `biomesData.color` | `String[]` | color hex por biome (default: en el código, no en el `.map`) |
| — | `biomesData.habitability` | `number[]` | habitabilidad (burg scoring) |
| — | `biomesData.iconsDensity` | `number[]` | densidad de iconos (relief layer) |
| — | `biomesData.icons` | `string[][]` | pool de iconos por biome (relief layer) |
| — | `biomesData.cost` | `number[]` | coste de movimiento (culture/state expansion) |

### Implementación en Azgaar

`drawBiomes()` (`public/modules/ui/layers.js:302`):

```js
const isolines = getIsolines(pack, cellId => cells.biome[cellId], { fill: true, waterGap: true });
Object.entries(isolines).forEach(([index, { fill, waterGap }]) => {
  const color = biomesData.color[index];
  bodyPaths.push(getGappedFillPaths("biome", fill, waterGap, color, index));
});
```

1. `getIsolines` (`src/utils/pathUtils.ts:84`): para cada celda no visitada de biome ≠ 0, camina el anillo de vértices de borde (células vecinas de otro biome) con `connectVertices` (mismo motor que temperature/heightmap), y acumula un `fill` por biome. Skip de lagos internos (feature lake con shoreline toda del mismo biome).
2. `getGappedFillPaths` (`layers.js:1018`):
   - `<path d="${fill}" fill="${color}" id="biome${index}" />`
   - `<path d="${waterGap}" fill="none" stroke="${color}" stroke-width="3" id="biome-gap${index}" />`
3. El `waterGap` es `getBorderPath(vertices, chain, vertex => todos los vecinos del vértice son tierra)` — el trazado se corta en vértices 100% tierra, dejando solo los bordes que dan a agua.

### Color scheme (CSS `#biomes`)

| Preset | Fill | Mask | Opacity |
|---|---|---|---|
| default.json | (inline por biome) | `url(#land)` | — |
| watercolor.json | (inline) | `url(#land)` | 0.6 |

El **fill no viene del CSS**: va inline en cada `<path>` desde `biomesData.color`. La máscara `#land` (blanco=tierra, negro=lago, `draw-features.ts:39-47`) recorta el relleno a la tierra.

### Paleta default de biomas (`src/generators/biomes.ts:11`)

| id | name | color | habitability | iconsDensity | cost |
|----|------|-------|--------------|--------------|------|
| 0 | Marine | `#466eab` | 0 | 0 | 10 |
| 1 | Hot desert | `#fbe79f` | 4 | 3 | 200 |
| 2 | Cold desert | `#b5b887` | 10 | 2 | 150 |
| 3 | Savanna | `#d2d082` | 22 | 120 | 60 |
| 4 | Grassland | `#c8d68f` | 30 | 120 | 50 |
| 5 | Tropical seasonal forest | `#b6d95d` | 50 | 120 | 70 |
| 6 | Temperate deciduous forest | `#29bc56` | 100 | 120 | 70 |
| 7 | Tropical rainforest | `#7dcb35` | 80 | 150 | 80 |
| 8 | Temperate rainforest | `#409c43` | 90 | 150 | 90 |
| 9 | Taiga | `#4b6b32` | 12 | 100 | 200 |
| 10 | Tundra | `#96784b` | 4 | 5 | 1000 |
| 11 | Glacier | `#d5e7eb` | 0 | 0 | 5000 |
| 12 | Wetland | `#0b9131` | 12 | 250 | 150 |

### Matriz de biomas (`biomesMatrix`)

`Uint8Array[5]`, fila = banda de humedad `moisture/5 | 0` (0-4, húmedo→seco), columna = banda de temperatura `20 - temp` clampado a 0-25 (cálido→frío):

```
dry  [ 1,1,1,1,1,1,1,1, 2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,10 ]   // hot↔cold
     [ 3,3,3,4,4, ..., 9,9,9,9,9,9,10,10,10 ]                       // banda humedad 1
     [ 5,6,6, ..., 9,9,9,9,9, 10,10,10 ]                            // banda 2
     [ 5,6,6, ..., 8,8,8, ..., 9,9,9,9,9,9, 10,10,10 ]             // banda 3
wet  [ 7,8,8, ..., 9,9,9,9,9,9,9, 10,10 ]                          // banda 4
```

### Asignación de bioma por celda (`Biomes.define()`, `getId`)

```
moisture = prec[cell]
        + (cell tiene río ? max(flux/10, 2) : 0)               // flux del pack
moisture = 4 + mean([moisture] + prec de vecinos tierra)        // rn() → redondeo JS
biome:
  height < 20            → 0  (Marine)
  temp < -5              → 11 (Glacier)
  temp >= 25 && !río && moisture < 8 → 1  (Hot desert)
  isWetland ?            → 12 (Wetland)
    temp <= -2 → no
    moisture > 40 && height < 25   → sí  (nearly coast)
    moisture > 24 && 24 < height < 60 → sí (off coast)
  else → biomesMatrix[ min((moisture/5)|0, 4) ][ min(max(20-temp,0), 25) ]
```

Nota: `Biomes.getDefault()` devuelve la paleta hardcodeada; el `.map` solo guarda `color|habitability|name` en el slot `[3]`. `biomesMatrix`, `icons`, `iconsDensity` y `cost` **no viajan en el `.map`** — son parte del código/preset de Azgaar. Para regenerar biomes nativamente hacen falta.

### Portabilidad a Voronia

- **Pipeline**: TriangleList, isolines rellenos por biome + water gap, à la heightmap/temperature.
- **Estado actual (Voronia)**: `vor-render/src/biome.rs` pinta **celda por celda** con `build_pack_mesh(cells.biome → color)` + `append_water_gap`. Geométricamente equivalente al fill de isolines (contornos = límites celda↔celda), pero **sin usar el motor de isolines** y **sin el gap exacto de Azgaar**.
- **Water gap (corregido 10 ago 2026)**: `vor-render/src/water_gap.rs` dibuja ahora los quads sobre la **arista compartida del Voronoi** (los 2 circumcenters de los triángulos que contienen ambas celdas tierra/agua), con `GAP_HALF_WIDTH=1.5` (= `stroke-width:3` de Azgaar) y sobre los **vértices suavizados** (mismo `laplacian_smooth_vertices(0.2, 2)` que el fill), para que no queden líneas desplazadas. Antes dibujaba sobre el segmento **centro-centro** (`pack.points[p]→[nb]`), que es perpendicular a la arista real del Voronoi y producía líneas visibles saliendo de la costa hacia el mar.
- **Catalog**: `vor-core::entities::biome::Biome` ya trae `color/habitability/move_cost/name`; la paleta default hardcodeada y la matriz no están — hay que portar `Biomes.getDefault()` a `vor-sim` (generación) y a `vor-render` (paleta para textos/densidades).
- **Generación nativa**: `vor-sim` aún **no tiene** `define()` ni `getId()` ni `biomesMatrix` — anotado como pendiente.

### Estado actual Voronia
- **Biomes**: ✅ capa portada y cableada (`build_biome_mesh` + `water_gap` en `vor-app/src/lib.rs:286-300`). Water gap **corregido (10 ago 2026)**: ya se dibuja sobre la arista real del Voronoi (2 circumcenters compartidos) con ancho 3 y vértices suavizados — desaparecen las líneas que salían de la costa. **Coast fill (13 ago 2026)**: `build_biome_coast_fill` elimina el halo blanco (ver §landmask/coast fill). ⚠️ Pendientes: usar isolines para paridad exacta del contour, y la generación nativa de biomas en `vor-sim`.
- **Goods/Trade (13 ago 2026)**: 🟡 **~70%** vía **Routes** (z=19): modelo `Route`/`RouteGroup` completo en `vor-core`, render de líneas por grupo en `route_layer.rs::build_route_mesh`, cableado como line layer con flag en la UI. La capa **Goods** propiamente (z=23) sigue 🔴 sin modelo/render (`world.goods` re-exportado opaco). Detalle en `docs/layers/human-geography-layers.md` §8.

### Landmask (stencil buffer) — 13 ago 2026
El equivalente wgpu de `mask: url(#land)` de Azgaar se implementó con un **stencil buffer** en el pass del mapa:
- `vor-render/src/renderer.rs`: buffer depth-stencil (`Depth24PlusStencil8`, MSAA-matched) + tres pipelines de máscara:
  - `mask_write_pipeline`: lo usa la **layer 0** (el landmass fractal de `build_fractal_landmass_mesh`, que salta lagos) y **escribe `stencil = 1`** donde hay tierra.
  - `mask_test_pipeline` / `mask_test_blend_pipeline`: los usan las capas thematic mas reales (biomes, states, provinces, cultures, religions) y **solo pinta donde `stencil == 1`** (`pass.set_stencil_reference(1)`).
- Toda capa registrada con **`renderer.add_layer_mesh_masked(mesh, blend)`** queda recortada a la costa: el fill de biomes deja de sangrar al océano y el water gap se recorta a la línea de costa (sin líneas saliendo al mar).
- FMG también enmascara a `#land` los fills climáticos: en Voronia temperature (blended) y precipitation se registran masked.
- Requisito wgpu: cualquier pipeline dibujado en el pass del mapa (heightmap, ocean, line, texture overlay, glyphon MSAA) declara un `depth_stencil` passthrough (`stencil_passthrough()`) con el mismatch del `stencil_view()` del renderer.

### Coast fill (halo blanco) — 13 ago 2026

La costa fractal de `build_fractal_landmass_mesh` (que alimenta el stencil) sobresale de las celdas de tierra por el desplazamiento fractal: **41% de las muestras de costa caen fuera de las celdas de tierra, hasta ~47 u**. Donde el stencil pinta pero el fill de celdas no llega, el fondo blanco del landmass se veía como un **halo blanco alrededor de la costa**.

Fix en `vor-render/src/biome.rs::build_biome_coast_fill`:
- Recolorea **cada triángulo del mesh fractal** con el biome de la **celda de tierra más cercana** (búsqueda via `CentroidGrid`, grilla uniforme de centroides con anillos expansivos; excluye océano/lagos).
- En `vor-app/src/lib.rs` se fusiona **al inicio** de `biome_mesh` (que ya lleva el fill de celdas + water gap) en la misma layer masked: como el pipeline usa `depth_compare: Always` sin culling, el orden dentro del mesh importa — el coast fill se dibuja **primero** y las celdas lo tapan, así el biome llega exactamente hasta la costa fractal.

### Mask unión (islas pequeñas) — 13 ago 2026

La fractalización **encoge los polígonos de islas pequeñas** (cobertura del mask de 68-89%): parte de sus celdas de tierra quedaba **fuera** del landmask y sin pintar (huecos que "pelean con el mar"). Fix: la capa 0 (fuente del stencil) ahora es la **unión** de la costa fractal y un mesh blanco de celdas de tierra con vértices **crudos**:

- `vor-render/src/mesh.rs::build_land_cells_mask_mesh(vertices, points_n, is_land)` — tesela cada celda de tierra con sus **vértices originales** (sin Laplacian smoothing, que las encogería) en blanco.
- En `vor-app/src/lib.rs` se fusiona **prepended** a `cfg.mesh` (layer 0) antes de `set_mesh`, garantizando que **toda celda de tierra queda dentro del mask** (paint-bucket del landmass).

Combinado con `build_biome_coast_fill`, el mask queda completamente pintado: islas de 1 celda pasan de 68% a 100% de cobertura, y el área "mask only" (sin biome) se mantiene en 0.0%.

---

## 2. Relief icons (relief / terrain)

### Qué hace en Azgaar

Genera iconos SVG (árboles, dunas, montañas, colinas) por celda, colocados con **Poisson-disc sampling** dentro del polígono de cada celda, y los sobreimprime sobre los biomas. En cada celda decide entre **iconos de bioma** (height < 50, árboles/vegetación según `icons[biome]`) o **iconos de relieve** (height ≥ 50, montañas/colinas según altura y temperatura).

### Data consumed

| Slot | Voronia Field | Type | Descripción |
|------|---------------|------|-------------|
| — | `pack.cells.biome` | `Vec<u8>` | biome por celda (define el pool de iconos y la densidad) |
| — | `pack.cells.h` | `Vec<u8>` | height (mountain/hill vs vegetación) |
| — | `pack.cells.r` | `Vec<u16>` | river id (skip si > 0) |
| — | `grid.cells.temp` | `Int8` | temperatura (montaña nevada si temp < 0) |
| — | `biomesData.icons` | `string[][]` | pool por biome, p.ej. `[1] = [dune,cactus,deadTree]` |
| — | `biomesData.iconsDensity` | `number[]` | densidad por biome |
| CSS `#terrain` | `set`/`size`/`density` | `string/number` | estilo, escala y densidad global |

### Implementación en Azgaar

`drawReliefIcons()` (`src/renderers/draw-relief-icons.ts:21`):

1. Por cada celda (`cells.i`): skip si `h < 20` (agua), skip si `r[i]` (río), skip si `h < 50 && iconsDensity[biome] == 0`.
2. Bounding box del polígono de la celda (`extent` de los vértices).
3. **height < 50 → `placeBiomeIcons()`**:
   - `iconsDensity = iconsDensity[biome]/100`; `radius = 2 / iconsDensity / density`
   - `if (Math.random() > iconsDensity*10) return` — celda parcialmente vacía
   - `poissonDiscSampler(minX,minY,maxX,maxY,radius)` (mbostock, `graphUtils.ts:408`): puntos con distancia mínima `radius`
   - `if (!polygonContains(polygon, p)) continue` — descarta puntos fuera de la celda
   - `h = (4 + Math.random())*size`; icon random de `biomesData.icons[biome]`; si `grass` → `h *= 1.2`
   - `relief.push({ i: icon, x: rn(cx-h,2), y: rn(cy-h,2), s: rn(h*2,2) })`
4. **height ≥ 50 → `placeReliefIcons()`**: `radius = 2/density`; `getReliefIcon(i,height)`:
   - `type = temp < 0 ? "mountSnow" : "mount"` si `h > 70`; si no `"hill"`
   - `iconSize = h > 70 ? (h-45)*mod : minmax((h-40)*mod, 3, 6)` con `mod = 0.2*size`
5. Ordena todos los iconos por `y+s` (paint order correcto en superposición).
6. Emite `<use href="#relief-..." x y width height>` (31 símbolos en `#defs-relief`, `src/index.html`): sets `simple` (`-1`), `colored` (variantes `-2..-7`), `gray` (`-bw`).

### CSS `#terrain`

| Preset | set | size | density | mask |
|---|---|---|---|---|
| default.json | `simple` | 1 | 0.4 | — |
| watercolor.json | `gray` | 1 | 0.4 | — |

### Sprites (`defs-relief`, set simple)

`relief-mount-1`, `relief-hill-1`, `relief-deciduous-1`, `relief-conifer-1`, `relief-acacia-1`, `relief-palm-1`, `relief-grass-1`, `relief-swamp-1`, `relief-dune-1` (+ variantes `-bw` para gray y `-2..-7` para colored). Cada `<symbol viewBox="0 0 100 100">` traza el icono con paths; el `use` los coloca con `width`/`height` = `size`. Los sprites mount/hill/árbol tienen paths con fill/stroke **en negro/blanco** o color según set — el color final lo da el symbol, no el `use`.

### Portabilidad a Voronia

- **Poisson-disc**: port limpio de `poissonDiscSampler` (mbostock, MIT) con `rng: &mut impl Rng` para determinismo (Azgaar usa `Math.random()` — no determinista; Voronia debe fijar semilla por consistencia).
- **polygonContains**: punto-en-polígono (ray casting) sobre el polígono de la celda.
- **Render**: Voronia no tiene `<use>`/symbols SVG — hay que "bakear" cada icono a geometría (paths de `defs-relief` → triangulación lyon) o dibujar primitivas GL (árboles/montañas simplificados). Para paridad visual real se requiere portar los 31 symbols a paths/normales.
- **Estado actual (Voronia)**: `vor-render/src/relief.rs` dibuja **triángulos montaña/colina por celda** con altura ≥ 40 (forma genérica, 3 colores fijos, sin Poisson, sin biomes, sin sprites). Funciona pero está muy lejos de la paridad con Azgaar (⚠️ pendiente real).

---

## 3. Z-order (Biosphere dentro del mapa)

Orden real de grupos en Azgaar (`public/main.js`, de abajo a arriba; el resto marcado como referencia de las categorías vecinas):

```
1.  #ocean
2.  #landmass (features)
3.  #texture
4.  #terrs  (heightmap)
5.  #lakes
6.  #biomes           ← BIOMES
7.  #cells
8.  #gridOverlay
9.  #coordinates
10. #compass
11. #rivers
12. #terrain           ← RELIEF ICONS
13. #relig
14. #cults
15. #regions
16. #provs
17. #zones
18. #borders
19. #routes
20. #temperature
21. #coastline
22. #ice
23. #goods
24. #markets
25. #tradeAnimation
26. #prec
27. #population
28. #emblems
29. #icons          (burg icons)
30. #labels
31. #armies
32. #markers
33. #ruler
34. #debug
```

Nota: **biomes va debajo de rivers/lakes** y **relief va encima de ríos pero debajo de layers humanas**. En Voronia el draw order definido en `docs/plans/master-plan.md` §9.2 coincide (3. biomes, 12. relief icons).

---

## 4. Referencias

- `public/modules/ui/layers.js` → `drawBiomes()` (`:302`), `getGappedFillPaths()` (`:1018`)
- `src/utils/pathUtils.ts` → `getIsolines()` (`:84`), `getBorderPath()`, `connectVertices()` (`:261`)
- `src/renderers/draw-relief-icons.ts` → `drawReliefIcons()` completo
- `src/utils/graphUtils.ts` → `poissonDiscSampler()` (`:408`, mbostock)
- `src/generators/biomes.ts` → `getDefault()`, `define()`, `getId()`, `isWetland()`
- `src/renderers/draw-features.ts` → construcción del mask `#land`
- `src/index.html` → `defs-relief` (símbolos de iconos), `mask id="land"`
- `public/styles/default.json` → CSS `#biomes`, `#terrain`

## 5. Open Questions / pendientes

- **biomes**: ¿mantener fill celda↔celda (equivalente) o migrar a isolines+waterGap exacto de Azgaar? (paridad visual parece suficiente con el actual; el gap es lo que difiere).
- **biomes**: falta `Biomes.getDefault()` (paleta+matriz+densidades+cost) en Voronia — ¿en `vor-sim::biomes`? El `.map` no la trae.
- **relief**: portar `poissonDiscSampler` con `rng` determinista (semilla: `header.seed`, igual que costas).
- **relief**: sprites — portar los 31 symbols del set simple a paths → triangulación lyon, o partir de primitivas GL simplificadas (decisión de fidelidad).
- **relief**: el orden de pintado `y+s` exige depth-sort dentro de la capa.