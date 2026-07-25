# Fase 0 — Investigación y sentado de bases

> Salida consolidada de la Fase 0 del plan maestro (§23). Todo lo que sigue se obtuvo leyendo el código fuente y la documentación del repo de Azgaar Fantasy Map Generator clonado localmente. Cuando se cierra la Fase 0, los checkboxes de §23 del plan maestro se tildan y este archivo queda como referencia congelada para Fase 1+.

## 0. Referencia de Azgaar (referencia congelada)

- **Repo**: https://github.com/Azgaar/fantasy-map-generator (mayúsculas/minúsculas indiferentes en GitHub).
- **Ref clonada**: `origin/main`, shallow (`--depth 1`).
- **Path local**: `/home/hans/Proyectos/azgaar-fmg/` (carpeta hermana de `voronia`, fuera del repo de Voronia para no interferir con git status ni con el build del workspace).
- **Commit**: `51d8e3e487a28995aac2304af57ad1ac4fbe3789` (2026-07-21).
- **Versión declarada en package.json**: `1.135.2`.
- **Versión del último commit (bump)**: `1.138.0` → hay un lag entre package.json y el commit bump, normal en main. A efectos de comparación, la versión efectiva de referencia es **1.138.0**.
- **Plan maestro §25 citaba `v1.119` como última release detectada** → desactualizado: realidad actual de main es ~20 versiones más nueva. Esto no invalida el plan pero conviene saberlo antes de comparar hal rounding.
- **Licencia Azgaar**: MIT (idéntica a la de Voronia, sin fricción legal para derivar).

## 1. Stack JS confirmado (de `package.json` v1.135.2)

### Runtime deps (relevantes para port a Rust)

| Dep JS | Versión | Para qué sirve en Azgaar | Equivalente Rust planeado |
|---|---|---|---|
| `alea` | ^1.0.1 | **PRNG** seedable — crítico por §3 | `rand_pcg` (a confirmar que el algoritmo de `alea` coincida con algún PCG concreto) |
| `delaunator` | ^5.0.1 | Triangulación de Delaunay | `delaunator` crate (a validar bit-exactitud) |
| `d3` | ^7.9.0 | Diversos helpers (escala, paths, geo, voronoi-delaunay en d3-delaunay) | n/a — reimplementar lo específico |
| `three` | ^0.184.0 | Renderer 3D opcional (vista 3d) | wgpu |
| `polylabel` | ^2.0.1 | Encontrar el "pole of inaccessibility" para labels | reimplementar (algoritmo de García/Castro) |
| `lineclip` | ^2.0.0 | Clipping de líneas (Cohen–Sutherland) | reimplementar (líneas Lyon también sirve) |
| `driver.js` | ^1.4.0 | Tour UI (no relacionado al motor) | — |

### Dev deps relevantes

- **TypeScript** gradual (`tsc && vite build`); todavía conviven `.js` legacy y `.ts`.
- **Vite** (dev/build), **Vitest** (tests), **Playwright** (e2e), **Biome** (lint/format), `simple-git-hooks` (pre-commit = `biome check --write`).
- `engines.node` `>=24.0.0`.

## 2. Arquitectura declarada por el propio Azgaar (de `README.md`)

Literal del README (líneas 35-41):

> The expected **future** architecture is based on a separation between **world data**, **procedural generation**, **interactive editing**, and **rendering**. [...] The application is conceptually divided into four main layers: world data and styles (state), generators (model), editors (controllers), renderers (view).
>
> Flow:
> - `settings → generators → world data → renderer`
> - `UI → editors → world data → renderer`
>
> The data layer must contain no logic and no rendering code. Generators implement the procedural world simulation. Editors implement interactive editing tools used by the user. [...] The renderer converts the world state into SVG or WebGl graphics. Renderer must be pure visualization step and not modify world data.

**Esto valida directamente la arquitectura de Voronia** (plan §5: `vor-core` = world data sin lógica, `vor-sim` = generators, `vor-edit` = editors, `vor-render` = renderer que jamás muta world). Es la misma separación, expuesta por el propio Azgaar como "future architecture" — interesante: todavía no está del todo aplicada en su codebase, pero es la dirección oficial.

`main.js` en la raíz sigue siendo el entry legacy; el TS migra gradualmente.

## 3. Estructura de `docs/` de Azgaar (a leer en este fase 0)

```
docs/
├── architecture/   — internals del código
│   ├── architecture.md          (28KB)
│   ├── data_model.md             (33KB) ← clave: esquema de datos por celda
│   ├── future_data_model.md      ( 4KB)
│   ├── lazy_loading.md           ( 5KB)
│   └── migration_guide.md        (13KB)
├── domain/         — modelos de dominio
│   ├── generation_pipeline.md   (13KB) ← clave: orden de generación y orden de consumo del RNG
│   ├── glossary.md
│   ├── goods_schema.md          (11KB)
│   ├── production_schema.md     ( 9KB)
│   ├── trade_schema.md          (13KB)
│   ├── taxes.md
│   └── 3d-view.md
├── prd/            — specs de features específicas
│   ├── controller-service-registry.md (13KB)
│   ├── good-multipliers.md
│   ├── meandering-river-routes.md     (22KB)
│   ├── navigable-river-routes.md      (20KB)
│   └── state-taxes-and-treasury.md
└── updates/
    ├── v1.123.0/
    └── v1.124.0/
```

## 4. Wikis clave (resumen de cada una)

Links:
- Heightmap customization: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Heightmap-customization
- Heightmap template editor: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Heightmap-template-editor
- Culture types: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Culture-types
- Military Forces: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Military-Forces
- Goods: spread functions: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Goods:-spread-functions

### 4.1 Heightmap customization (UI/UX)

Documenta el **Editor de Heightmap** como feature de usuario (no motor). Puntos relevantes para Voronia:

- Tres modos al editar heightmap existente: **Erase** (regenera todo), **Keep** (mantiene datos, no toca costa), **Risk** (mantiene datos pero permite cambiar costa — puede romper cosas).
- Brushes: Raise / Elevate / Lower / Depress / Align / Smooth / Disrupt (las cuatro últimas operan contra vecinos). Line tool para montañas/valles lineales. Checkbox "change only land cells" protege costa.
- Footer del editor: Undo/redo (solo de heightmap), Rescaler (rescala todas las alturas), Condition (rescaler condicional), Smooth global, Disrupt global, Clear (= todas a 0).
- **Image Converter**: sube una imagen y mapea colores→alturas por (luminosidad | hue | esquema propio de Azgaar). Set max amount of colors (3–255). Los colores no asignados se caen a océano.
- La wiki no documenta las fórmulas exactas de los brushes — esas viven en el código (ver §6/§7). Para Voronia: la **edición por pincel** es parte de `vor-edit` (plan §11); la conversión imagen→heightmap se puede deferir a Fase 6 o más adelante.

### 4.2 Heightmap template editor ( Heightmap templates )

Documenta el **editor de plantillas** — el DSL textual que describe cómo generar un heightmap paso a paso. **Muy relevante para `vor-sim`** (Fase 7).

- Una *template* es una secuencia de **acciones** aplicadas en orden sobre un heightmap recién cleared:
  - `Hill n:.. h:.. x:.. y:..` — colina (eleva entorno).
  - `Pit n:.. h:.. x:.. y:..` — pozo (hunde entorno).
  - `Range n:.. h:.. x:.. y:..` — cordillera (elevación alargada).
  - `Trough ...` — depresión alargada.
  - `Strait w:.. d:{vertical|horizontal}` — estrecho (baja línea, parte la tierra). No se puede elegir ubicación.
  - `Mask <fraction>` — baja las alturas gradualmente hacia el borde; fracción negativa invierte (baja el centro).
  - `Invert` — espeja por X / Y / ambos, con probabilidad.
  - `Add V:<n> to all cells` — offset global (admite negativos).
  - `Multiply V:<n>` — multiplicador global (típicamente decimal).
  - `Smooth` — promedia contra vecinos.
- Cada step tiene: `n` (cuántas veces, puede ser rango aleatorio), `h` (rango de altura 1–100; 20 = costa, 100 = pico), `x` e `y` (rangos porcentuales 0–100 del mapa). Step se puede skipping con checkbox.
- Para reproducción bit-exacta: si la seed está locked, el resultado es idéntico entre ejecuciones. **Implica consumo determinado de RNG por step** → ver §5 (orden de consumo del RNG).

### 4.3 Culture types (culturas + spread)

Documenta cómo se asigna el tipo de cultura al generarse y cómo se expande cada cultura competitivamente. Relevante para `vor-sim` (Fase 7).

- **7 tipos**: Nomadic / Highland / Lake / Naval / River / Hunting / Generic. El tipo se asigna al generarse por la geografía del spawn (primera regla que matchea, excepto Naval que además tira un probability check).
- Reglas por tipo (con alto `h` = height):
  - Nomadic: desierto o pastizal con height < 70.
  - Highland: height > 50.
  - Lake: junto a lagos de > 5 celdas.
  - Naval: celdas costeras (más chance si es costa oceánica, aún más si es isla; tirada random extra).
  - River: celdas con río de flux > 100.
  - Hunting: >2 celdas de la costa Y bioma ∈ {Savanna, Rain forest, Taiga, Tundra, Wetland}.
  - Generic: fallback.
- **Cultura spread** es un costo competitivo tipo Dijkstra: cada cultura compite por celdas según costos dependientes del tipo.
  - **Expansionism** base × multiplicador por tipo: Generic 1, Lake 0.8, Naval 1.5, River 0.9, Nomadic 1.5, Hunting 0.7, Highland 1.2. Mayor expansionism → menor costo → más territorio.
  - **Biome cost** base: Hot desert 200, Cold desert 150, Savanna 60, Grassland 50, Trop seasonal forest 70, Temp deciduous forest 70, Trop rainforest 80, Temp rainforest 90, Taiga 200, Tundra 1000, Glacier 5000, Wetland 150. Si es bioma nativo → cost 10. Hunting × 5 en no-nativos. Nomadic × 10 en los 5 biomas forestados. Resto × 2. +20 si el bioma difiere del de origen.
  - **Water crossing cost**: Lake 10 al cruzar su lago. Naval = 2 × size(pixel) del cell al cruzar agua. Nomadic = 50 × size(pixel). Resto (incl. Lake cruzando mar) = 6 × size(pixel).
  - **River crossing cost**: no-River +20..100 según flux. River: río free, pero no-río +100.
  - **Distance to coast cost**: Nomadic +60 en costa; otros no-Naval no-River +20. Naval y Nomadic +30 en celdas contiguas a costa. Naval y River +100 lejos de costa. Costos se suman y luego se dividen por expansionism. Cuando el costo total > budget, la expansión frena.

### 4.4 Military Forces (milicia)

Documenta el sistema de regimientos militares — consumo determinado de población y cells. Relevante para `vor-sim` (Fase 7); el plan maestro §7.6 cubre las entidades.

- **War Alert** por estado = `Expansion_fulfillment (expansionism/area) + Diplomatic_alert`. Relaciones: Ally −0.2, Friendly −0.1, Neutral 0, Suspicion +0.1, Enemy +1, Unknown 0, Rival +0.5, Vassal +0.5, Suzerain −0.5.
- **State-type modifier** × unit-type (8 tipos: Melee, Ranged, Mounted, Machinery, Naval, Armored, Aviation, Magical) — matriz 7×8 en el wiki (filas: Generic, Nomadic, Highland, Lake, Naval, Hunting, River). Hordes × 2 en mounted; Republics × 1.2 en naval.
- **Cell-unit modifier** × bioma/altura del cell: Nomadic, Wetland, Highland (height > 70) — tres matrices separadas. Idem para burgs (Nomadic/Wetland/Highland). Ver el wiki para los valores exactos (no los duplico acá para no romper mantenimiento).
- **Fórmula de tropas por cell/burg**:
  `Troops = Population_points / 100 × Possession_divider × Unit_percentage × State_mod × Unit_mod × Population_rate`
- **Possession_divider** se aplica si: cell culture ≠ culture dominante del state (Unions 1.2, otros 2); cell religion ≠ religión dominante (Theocracies 2.2, otros 1.4); cell en isla distinta al centro del state (Naval 1.2, otros 1.8).
- **Naval units** solo en port-burgs.
- Platoons (1 por cell+b¡rg) se agregan a **regiments**. Regiment expected size = `3 × Population_rate`. Cuando el regimiento llega al tamaño esperado, se crea uno nuevo. Naming y details generados después, posicionados sobre el mapa.

### 4.5 Goods: spread functions (DSL de bienes)

Documenta el DSL de *spread models* — expresiones booleanas evaluadas por cell para decidir si un bien puede aparecer. **Relevante para `vor-sim`** (Fase 7 / economía §10.8) y eventualmente para plugins (plan §21.4).

- Los bienes se generan **antes** que states y cultures — los modelos se basan solo en geografía (bioma, altura, temp, costa, ríos). Al cambiar un modelo hay que regenerar todos los bienes.
- Técnicamente, cada spread model es una **expresión JS válida** evaluada por cell → `true`/`false`. Operadores lógicos JS permitidos (`!`, `||`, `&&`).
- Funciones built-in del DSL:
  - `random(n)`: true con probabilidad `n%`.
  - `nth(n)`: true cada n-ésima cell (`nth(2)` = 50%, `nth(5)` = 20%).
  - `habitable()`: bioma con habitabilidad > 0.
  - `habitability()`: checkea contra la habitabilidad del bioma (≥100 siempre, 0 nunca, 50 → 50%).
  - `elevation()`: contra altura del cell (mayor altura → mayor chance). Negable con `!`.
  - `biome(id,...)`: immagbiome ids.
  - `minHeight(n)`, `maxHeight(n)`: rango 0–100, 20 = costa.
  - `minTemp(n)`, `maxTemp(n)`: en Celsius.
  - `shore(ring,...)`: distancia a shoreline. `1` = costa terrestre, `2` = siguiente anillo tierra, `-1` = agua poco profunda, `-2 -3...` = más profundo.
  - `type(str,...)`: ocean | freshwater | salt | sinkhole | frozen | lava | dry | continent | island | isle | lake_island. La wiki desaconseja `type` para tierra.
  - `river()`: hay río en el cell.
- **Biomes ids** (default, orden importa):
  `0 Marine, 1 Hot desert, 2 Cold desert, 3 Savanna, 4 Grassland, 5 Tropical seasonal forest, 6 Temperate deciduous forest, 7 Tropical rainforest, 8 Temperate rainforest, 9 Taiga, 10 Tundra, 11 Glacier, 12 Wetland`. (Para obtener ids reales en runtime: `biomesData.name.map((n,i)=>i+". "+n)` en la consola del navegador.)
- **Built-in models** (referencia rápida): Deciduous_forests `biome(6,7,8)`, Hills `minHeight(40) || (minHeight(30) && nth(10))`, Mountains `minHeight(60) || (minHeight(40) && nth(10))`, Headwaters `river() && minHeight(40)`, Marine_and_rivers `type("ocean","freshwater","salt") || (river() && shore(1,2))`, Tropical_waters `shore(-1) && minTemp(18)`, Arctic_waters `biome(0) && maxTemp(7)`. (Lista completa en la wiki.)
- Consecuencia para Voronia: este DSL es una superficie de ataque y un punto de extensión natural. En Rust una opción es un intérprete del subconjunto + un mini-parser (o un enum de predicados); alternativa a más largo plazo es permitir WASM plugins (plan §21.4).
- ⚠️ El spread model se evalúa por cell; dado que los bienes se generan antes de states/cultures, cualquier cosa que Voronia haga que cambie el orden de generación o el id de cell cambia el outcome —_recordar §3 y §6/§7_.

---

## 5. Estructura de `src/` (mapa de módulos)

Árbol top-level (10 entradas):

```
src/
├── components/    (3 files)   Custom elements (fill-box, slider-input) + barrel.
├── controllers/  (~60 files) Editores/overlays de UI, lazy-load vía window.Controllers.
├── data/          (4 files)   Datos estáticos (heightmap-templates, precreated-heightmaps, supporters, view-3d-options).
├── generators/   (~25 files) Pipeline procedural (voronoi, heightmap, features, lakes, rivers, biomes, cultures, states, religions, provinces, routes, zones, ice, military, markers, measurers, goods, production, markets, draw-goods, resample) + emblems/ subtree.
├── renderers/    (~25 files) Drawing SVG + emblems/ subtree. Barrel side-effect.
├── services/      (4 files + io/) Servicios de alto nivel (Cloud, ExportJson, ExportMap, Installation, Load, Save, UiTour) + io/ subtree (save.ts, load.ts, export.ts, export-json.ts, cloud.ts, auto-update.ts).
├── types/         (2 files)   global.ts (declares legacy globals) + PackedGraph.ts (canonical type del pack).
├── utils/         (18 files)  Helpers puros (graphUtils, probabilityUtils, pathUtils, arrayUtils, numberUtils, colorUtils, commonUtils, functionUtils, languageUtils, nodeUtils, stringUtils, unitUtils, debugUtils, registry, polyfills).
└── index.html                 DOM entry-point (carga alea.min.js y delaunator.min.js como <script>).
```

### Files críticos para geometría/PRNG (con path exacto)

| Propósito | Path |
|---|---|
| Grilla jitterizada | `src/utils/graphUtils.ts:getJitteredGrid` (líneas 46-61) + `placePoints` (69-98) |
| Delaunay + Voronoi assembly | `src/utils/graphUtils.ts:calculateVoronoi` (159-177), importa `Delaunator` y `Voronoi` |
| Clase Voronoi (deriva cells/vertices) | `src/generators/voronoi.ts` (155 líneas, `class Voronoi`) |
| Repack grid→pack | `public/main.js:reGraph` (1157-1209) — **legacy JS** |
| Wrapper del PRNG | No hay wrapper propio: monkey-patcheo de `Math.random`. Helpers en `src/utils/probabilityUtils.ts` |
| 2da versión de alea (legacy) | `public/libs/alea.min.js` (`aleaPRNG 1.1.0` por macmcmeans) |
| Pipeline canónico | `public/main.js:generate()` (líneas 650-680 aprox) — **legacy JS**, ~16 fases |
| Driver del primer uso del RNG | `public/modules/ui/options.js:randomizeOptions` (607) — **legacy JS** |
| Parser .map | `src/services/io/load.ts:parseLoadedResult/parseLoadedData` + `src/services/io/save.ts:prepareMapData` |
| Export JSON | `src/services/io/export-json.ts` (modos Full/Minimal/PackCells/GridCells) |
| Migraciones de versión de .map | `src/services/io/auto-update.ts` (1243 líneas) |
| Tipo canónico `pack` | `src/types/PackedGraph.ts` (70 líneas) — lista completa de `pack.cells.*` y `pack.{rivers,features,burgs,states,cultures,routes,religions,zones,markers,ice,provinces,goods,markets,deals,measurers}` |
| Barrel de utils (inyecta en `window.*`) | `src/utils/index.ts` (269 líneas) — puente legacy TS↔JS |

### Estado migración JS→TS (resumen)

- **Geometría y PRNG: ~95% migrado a TS** (`graphUtils.ts`, `voronoi.ts`, `probabilityUtils.ts` están completos y tipados).
- **El driver del pipeline y el primer consumo del RNG siguen en JS legacy** (`public/main.js:generate/setSeed/reGraph/...`, `public/modules/ui/options.js:applyGraphSize/randomizeOptions`). Cualquier port a Rust tiene que leer el JS legacy como fuente de verdad para esa parte.
- **El runtime carga dos alea distintas**: `public/libs/alea.min.js` como `<script>` (en `src/index.html`) y `alea@1.0.1` de npm (en `graphUtils.ts:1`). Hay que tener ambas a mano para reproducir bit-exacto (ver §7).

## 6. Delaunay/Voronoi — algoritmo exacto

Fuente primaria: `src/generators/voronoi.ts` (155 líneas) — clase `Voronoi` con docstrings.

### 6.1 Delaunay

- Librería: **`delaunator@5.0.1`** (Mapbox, npm). Pura, sin variantes.
- Punto de uso principal: `src/utils/graphUtils.ts:162` → `Delaunator.from(allPoints)` donde `allPoints = points.concat(boundary)`.
- Punto de uso secundario: `src/generators/routes-generator.ts:241` → `Delaunator.from(points)` para adyacencias de caminos (no del grid). No afecta a la malla de Azgaar, solo a rutas.
- El bundle legacy `public/libs/delaunator.min.js` también carga como global para que lo usen los módulos JS legacy, pero el código TS ya usa la versión npm directamente.

### 6.2 Voronoi — derivación desde Delaunay

Constructor `new Voronoi(delaunay, allPoints, pointsN)` en `voronoi.ts:25`. Recorre cada half-edge `e ∈ [0, delaunay.triangles.length)` y deriva dos estructuras:

- **`cells`** (entidades por punto `p ∈ [0, pointsN)`):
  - `cells.v[p]` — IDs de vértices (puntos de Voronoi) que rodean al punto `p`. Se calcula con `edgesAroundPoint(e)` que camina half-edges vía `nextHalfedge(e)` + `delaunay.halfedges[]`. Cap explícito de **20 half-edges** (`result.length < 20`) para evitar loops infinitos.
  - `cells.c[p]` — IDs de celdas vecinas, filtrado `c < pointsN` (descarta boundary points).
  - `cells.b[p]` — flag `1` si `edges.length > cells.c[p].length` (es decir, el punto toca el boundary: hay más half-edges que vecinos válidos dentro del rango de puntos reales).
  - `cells.i` — se pobla por fuera, en `calculateVoronoi:169-172`: `createTypedArray({maxValue: points.length, length: points.length}).map((_, i) => i)`. Es decir, `cells.i[k] === k` para todo `k ∈ [0, pointsN)`. Por eso **el id de celda final es k = índice de punto en `placePoints`** — esto es lo que vuelve determinista el correspondence entre RNG output (jitter en orden) y atributos por celda.

- **`vertices`** (entidades por triángulo `t = floor(e/3)`):
  - `vertices.p[t]` — **coordenadas del circumcenter** del triángulo `t` (`triangleCenter(t)`, que llama a `circumcenter(a,b,c)`).
  - `vertices.v[t]` — IDs de triángulos vecinos (via `halfedges[]` opuestos).
  - `vertices.c[t]` — los 3 IDs de puntos que conforman el triángulo `t`.

### 6.3 ⚠️ La trampa del `circumcenter` (crítico para bit-exactitud)

`voronoi.ts:142-154`:

```ts
const circumcenter = (a, b, c) => {
  const [ax, ay] = a, [bx, by] = b, [cx, cy] = c;
  const ad = ax * ax + ay * ay;
  const bd = bx * bx + by * by;
  const cd = cx * cx + cy * cy;
  const D = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
  return [
    Math.floor((1 / D) * (ad * (by - cy) + bd * (cy - ay) + cd * (ay - by))),
    Math.floor((1 / D) * (ad * (cx - bx) + bd * (ax - cx) + cd * (bx - ax)))
  ];
};
```

Observaciones críticas:

1. **`Math.floor` trunca a entero**, así que las coordenadas de los vértices de Voronoi son **enteros** (perdida de precisión deliberada, herencia de la versión original JS de Azgaar). Cualquier puerto a Rust que use `f32` o `f64` directo para `vertices.p` no va a calzar: si Voronia calcula el circumcenter en floats puros, los `vertices.p[t]` van a tener decimales donde Azgaar tiene enteros, y eso cambia el orden sub-pixel de operaciones posteriores (raster, picking, polygon area en `reGraph`, etc.).
2. Las coordenadas de los **puntos de Voronoi dependen de** las coordenadas de los puntos de entrada; por ende también dependen de `rn(x + jitter(), 2)` y `rn(y + jitter(), 2)` — los jitter son redondeados a 2 decimales (ver §6.5). Y esos valores `f64` entran al circumcenter; multiplications **no son float32**: JS usa siempre doubles.
3. **`D = 2*(ax*(by-cy) + bx*(cy-ay) + cx*(ay-by))`** puede ser 0 (triángulo degenerado) — el código no lo protege. Es un caso edge a probar, aunque Delaunator debería evitar collinear puntos estrictos en la práctica.

### 6.4 Helper macros (`triangleOfEdge`, `nextHalfedge`, `edgesOfTriangle`)

Standard Delaunator helpers (tomados textualmente de docs de Mapbox):

- `triangleOfEdge(e) = Math.floor(e / 3)`
- `nextHalfedge(e) = (e % 3 === 2) ? e - 2 : e + 1`
- `edgesOfTriangle(t) = [3t, 3t+1, 3t+2]`
- `pointsOfTriangle(t) = edgesOfTriangle(t).map(e => delaunay.triangles[e])`
- `trianglesAdjacentToTriangle(t) = edgesOfTriangle(t).map(e => triangleOfEdge(halfedges[e]))`

### 6.5 Jitter-exactitud (pre-Delaunay)

`getJitteredGrid` (`graphUtils.ts:46-61`):

```ts
const radius = spacing / 2;           // square radius
const jittering = radius * 0.9;       // max deviation
const doubleJittering = jittering * 2;
const jitter = () => Math.random() * doubleJittering - jittering;
for (let y = radius; y < height; y += spacing) {
  for (let x = radius; x < width; x += spacing) {
    const xj = Math.min(rn(x + jitter(), 2), width);
    const yj = Math.min(rn(y + jitter(), 2), height);
    points.push([xj, yj]);
  }
}
```

Detalles críticos para bit-exactitud:

1. **Iteración en fila-mayor**: `y` es el loop externo, `x` el interno. El orden de consumo del RNG es `(y0,x0), (y0,x1), ..., (y1,x0), ...`. Y por cada celda pide **un solo random** (`xj` consume uno, `yj` consume otro) en x-primero.
2. `rn(x + jitter(), 2)` redondea a 2 decimales — `rn` viene de `numberUtils.ts`.
3. `Math.min(..., width)`/`(..., height)` clampea al tamaño; el clamping sucede **después** del `rn(.,2)`. No se hace clamp por abajo (los `radius` iniciales garantizan `x,y >= radius > 0`).
4. Cuando se llama `generateGrid(seed, w, h)`, la **primera línea** del cuerpo es `Math.random = Alea(seed);` (línea 137). Esto **resetea el PRNG** — el primer `Math.random` que consume `getJitteredGrid` es la primera llamada a `Alea(seed)` post-construction.

### 6.6 Boundary points

`getBoundaryPoints` (`graphUtils.ts:17-37`): genera puntos en el borde del canvas para evitar celdas infinitas en el Voronoi. Aparte del cálculo (`offset = rn(-1 * spacing)`, `bSpacing = spacing * 2`), el detalle crítico es que **no consume RNG** — el boundary es puramente determinista en función de `graphWidth, graphHeight, spacing`.

## 7. PRNG — algoritmo exacto

### 7.1 Dos versiones de `alea` distintas en un mismo run

Este es el hallazgo que complica el puerto a Rust (refuerza el §3 del plan maestro):

| Wrapper | Versión | Source | Dónde se siembra | Cobertura |
|---|---|---|---|---|
| `Alea` (npm, ^1.0.1) | 0.1 de `seedrandom` original de Johannes Baagøe | `import Alea from "alea"` en `src/utils/graphUtils.ts:1` y ~9 generadores TS | `Math.random = Alea(seed)` en `graphUtils.ts:137` (dentro de `generateGrid`) | Todo el pipeline procedural TS desde `generateGrid` en adelante |
| `aleaPRNG` (vendored) | 1.1.0 de macmcmeans (incluye `Mash` y métodos `.int`, `.uint`fetc) | `public/libs/alea.min.js` cargado vía `<script>` | `Math.random = aleaPRNG(seed)` en `public/main.js:762` (dentro de `setSeed`) | El tramo `setSeed → generateGrid`, incluyendo `randomizeOptions()` (el primer consumo generativo) |

Ambas versiones son algoritmos de **Alea por Johannes Baagøe** — mismo algoritmo base (con `mash()`), pero wrapper distinto. **No son compatibles bit-exacto una con la otra** porque la instancia retorna el mismo stream numérico con la misma seed, pero la API cambia (`aleaPRNG.seed('x').random()` vs `Alea('x')()`), y los wrappers pueden inicializar el estado interno de forma distinta.

### 7.2 Mucho cuidado: el stream del RNG se siembra dos veces

Orden temporal exacto dentro de `generate()` (pipeline principal, `main.js:650-680`):

1. `setSeed()` (línea 660 aprox): `Math.random = aleaPRNG(seed)` (con versión vendored `public/libs/alea.min.js`).
2. `applyGraphSize()` + `randomizeOptions()`: **consume el PRNG** `aleaPRNG`. Este es el primer uso generativo del pipeline. Llama a `gauss(...)` varias veces (que llama a `randomNormal.source(() => Math.random())`, i.e. usa `aleaPRNG`).
3. `shouldRegenerateGrid`, `generateGrid`: **resetea el PRNG** con `Math.random = Alea(seed)` (versión npm). Acá se vuelve a empezar desde el inicio del stream de la seed.
4. Desde este punto en adelante — `getJitteredGrid`, `HeightmapGenerator.generate`, `Features.markupGrid`, `Rivers.generate`, etc. — todo consume `Alea(seed)` (npm).

**Consecuencia**: para reproducir bit-exacto, hay que portear ambas versiones de alea. El tramo `setSeed → generateGrid` consume `aleaPRNG`, no `Alea` de npm. Lo que se guarde en el World Data Model de Voronia incluye resultados generados en ambos tramos (e.g. options como `eraInverseOption`, `statesNumber` y un montón de settings randomizados). Reproducir el stream completo significa: `aleaPRNG(seed)` para el tramo de opciones aleatorizadas, y a partir de `generateGrid` cortar la carrera y empezar de nuevo con `Alea(seed)` (tanto mismo seed, distinta lib).

### 7.3 Helpers RNG (`src/utils/probabilityUtils.ts`)

Todos consumen `Math.random` (el monkey-patcheado):

- `rand(min, max)` → entero en `[min, max]`: `Math.floor(Math.random() * (max - min + 1)) + min`. Sin args: `Math.random()` puro.
- `P(prob)` → bool: `Math.random() < prob` (con `>=1 → true`, `<=0 → false`).
- `gauss(exp, dev, min, max, round)` → usa `d3.randomNormal.source(() => Math.random())(exp, dev)` clampeado a `[min, max]` y redondeado a `round` decimales.
- `Pint(float)` → `~~float + P(float % 1)` (trunca + 1 con probabilidad del decimal).
- `ra(arr)` → `arr[Math.floor(Math.random() * arr.length)]`.
- `rw(obj)` → array de keys (con peso = valor) → selección random.
- `biased(min, max, ex)` → `Math.round(min + (max-min) * Math.random() ** ex)`.
- `getNumberInRange("a-b")` → parseo de rango con `rand(a, b)`.
- `generateSeed()` → `String(Math.floor(Math.random() * 1e9))`.

### 7.4 El wrapper del PRNG en código legacy

No hay uno. El "wrapper" es `Math.random = <fn seedable>`. Por eso es tan crítico el orden temporal del paso 2-3 arriba.

**Para Voronia (Fase 1)**:
- Decidir: ¿reproducimos el tramo `setSeed → generateGrid` (i.e. el consumo `aleaPRNG` que hace `randomizeOptions`)? Si solo exportamos "full options" (i.e. calcular las options en runtime del usuario en Azgaar, y serializarlas, importerlas en Voronia), entonces **solo hace falta portear `Alea@1.0.1`** + la geometría. Si queremos reproducir el pipeline desde la seed pura, hay que portear también `aleaPRNG 1.1.0` (la versión vendored) y replicar el orden de consumo de `randomizeOptions`.
- Estimación: portear Ambas es ~200 líneas de Rust + tests byte-exactos contra la salida de las versiones JS.

## 8. Repacking grid→pack — algoritmo exacto

Fuente: `public/main.js:reGraph` (1157-1209), todavía **legacy JS**.

### 8.1 Propósito

El "grid" (`pack` de Azgaar espera) tiene ~N×cellsDesired puntos (uno por celda jittered). El "pack" descarta celdas innecesarias (océano profundo, lagos no-costeros) y reengendra densidad solo en costas. Esto baja el count de celdas a procesar por el resto del pipeline (culturas, estados, rutas, military) sin perder detalle en tierra.

### 8.2 Algoritmo

```js
const { cells: gridCells, points, features } = grid;
const newCells = { p: [], g: [], h: [] };
const spacing2 = grid.spacing ** 2;

for (const i of gridCells.i) {
  const height = gridCells.h[i];
  const type = gridCells.t[i];

  // descartes:
  if (height < 20 && type !== -1 && type !== -2) continue;  // océano profundo (no costero)
  if (type === -2 && (i % 4 === 0 || features[gridCells.f[i]].type === "lake"))
    continue;  // ciertos lagos no-costeros

  const [x, y] = points[i];
  addNewPoint(i, x, y, height);  // agrega p+[x,y], g+i, h+height

  // puntos extra en costas:
  if (type === 1 || type === -1) {  // tipo 1 = tierra costera, -1 = agua costera
    if (gridCells.b[i]) continue;   // no para near-border cells
    gridCells.c[i].forEach(function (e) {
      if (i > e) return;  // evita dup: solo procesa cuando i < e
      if (gridCells.t[e] === type) {
        const dist2 = (y - points[e][1])**2 + (x - points[e][0])**2;
        if (dist2 < spacing2) return;  // muy cerca
        const x1 = rn((x + points[e][0]) / 2, 1);
        const y1 = rn((y + points[e][1]) / 2, 1);
        addNewPoint(i, x1, y1, height);  // punto medio con el vecino
      }
    });
  }
}

const { cells: packCells, vertices } = calculateVoronoi(newCells.p, grid.boundary);
pack.cells = packCells;
pack.cells.p = newCells.p;
pack.cells.g = createTypedArray({ maxValue: grid.points.length, from: newCells.g });
pack.cells.h = createTypedArray({ maxValue: 100, from: newCells.h });
pack.cells.area = createTypedArray({maxValue: TYPED_ARRAY_MAX.UINT16, length: packCells.i.length})
  .map((_, cellId) => Math.min(Math.abs(d3.polygonArea(getPackPolygon(cellId))), TYPED_ARRAY_MAX.UINT16));
```

### 8.3 Puntos clave (bit-exactitud para Voronia)

1. **No consume RNG**. Es determinista puro en función del `grid` ya calculado.
2. **Itera en el orden de `gridCells.i`**, que es `[0, 1, 2, ..., pointsN-1]` tal como lo pobla `calculateVoronoi` (`createTypedArray(...).map((_, i) => i)`). Por eso el "id de pack" asignado cuando se agrega un punto a `newCells.p` coincide con el índice de push, y `pack.cells.g` mapea al id de grid original.
3. **Tipos de cell (`gridCells.t`)**:
   - `-2` = lago (no costero si `i % 4 !== 0`).
   - `-1` = agua costera (cercana a tierra).
   - `1` = tierra costera (cercana a agua).
   - otro = tierra interior / océano profundo.
4. **Descartes**:
   - `(height < 20 AND type NOT IN {-1, -2})` = océano profundo, eliminar.
   - `(type === -2 AND (i % 4 === 0 OR features[f].type === 'lake'))` = lagos **no-costeros**, eliminar (uno de cada 4 id; `i%4` determina densidad de lagos supervivientes; si la feature es lake, descarta).
5. **Puntos extra en costas** (`type ∈ {1, -1}`):
   - Solo si la celda no es near-border (`!gridCells.b[i]`).
   - Solo para vecinos del mismo tipo (`gridCells.t[e] === type`).
   - Solo si `i > e` (evita dup — lo agrega solo cuando ve primero al menor id).
   - Solo si distancia al vecino >= `spacing` (dist2 < spacing2 → skip).
   - Posición = punto medio (`rn((x+ex)/2, 1)`, redondeado a 1 decimal).
6. **После del repack**: `calculateVoronoi(newCells.p, grid.boundary)` recalcula Delaunay + Voronoi con los nuevos puntos. Acá también, los circumcenters se truncan a entero (ver §6.3). El `pack.cells.i` resultante es el rango `[0, pack.pointsN-1]`, donde `pack.cells.i[k] === k`.
7. **`pack.cells.area`** = `Math.abs(d3.polygonArea(getPackPolygon(cellId)))`, capped a `UINT16_MAX`. `getPackPolygon` arma el polígono de la celda usando los `pack.cells.v`. Esto ya involucra precisión de floats — pero `Math.abs` y `Math.min` son estables.
8. **Otros puntos de llamada a `reGraph` en el codebase**: `services/io/load.ts:414` (al cargar un mapa), `controllers/heightmap-editor.ts:501,628` (al editar heightmap). El repack ocurre **después de cualquier cambio en el grid**.

### 8.4 Slot de memoria del grid y del pack

- `grid.cells.*` son TypedArrays indexados por **id de grid** (`i ∈ [0, grid.pointsN-1]`).
- `pack.cells.*` son TypedArrays indexados por **id de pack** (`i ∈ [0, pack.pointsN-1]`), distinto del id de grid.
- `pack.cells.g[packId]` contiene el id de grid original (mapeo pack→grid).
- Por eso **los atributos que Azgaar serializa en el .map SÍ están asociados al id de pack**, NO al id de grid. Esto es lo que vuelve crucial la reprodución bit-exacta del repack: dos motores que diverjan en cuántas cells sobreviven o en el orden de `newCells.p` van a tener `pack.cells.g[k]` con mapping distinto, y los atributos del .map quedan mal aplicados.

## 9. Pipeline canónico (de `docs/domain/generation_pipeline.md` + `main.js`)

La rutina canónica es `generate()` en `public/main.js` (líneas ~650-680). 16 fases, en orden:

1. **Seed & sizing** — `setSeed`, `applyGraphSize`, `randomizeOptions` → seed, dimensiones, opciones randomizadas (primer consumo del RNG: `aleaPRNG`).
2. **Grid + heightmap** — `shouldRegenerateGrid`, `generateGrid` (re-siembra con `Alea` npm), `HeightmapGenerator.generate` → `grid.cells.h`.
3. **Hidrología base (grid)** — `Features.markupGrid`, `addLakesInDeepDepressions`, `openNearSeaLakes` → topología lake/ocean del grid.
4. **Posición mundial & clima** — `OceanLayers`, `defineMapSize`, `calculateMapCoordinates`, `calculateTemperatures`, `generatePrecipitation` → `mapCoordinates`, `cells.temp`, `cells.prec`.
5. **Repack** — `reGraph`, `Features.markupPack`, `Measurers.createDefaultRuler` → `pack.cells.*` y ruler default.
6. **Ríos & biomas** — `Rivers.generate`, `Biomes.define`, `Features.defineGroups` → `pack.rivers`, `cells.biome`, grupos de features.
7. **Hielo** — `Ice.generate` → capa de hielo.
8. **Catálogo de bienes** — `Goods.generate` → `pack.goods` (idempotente, se llama una vez).
9. **Ranking de celdas & culturas** — `rankCells`, `Cultures.generate`, `Cultures.expand` → `cells.s`, `cells.pop`, `pack.cultures`.
10. **Asentamientos & política** — `Burgs.generate`, `States.generate`, `Routes.generate`, `Religions.generate` → `pack.burgs`, `pack.states`, `pack.routes`, `pack.religions`.
11. **Especificación política** — `Burgs.specify`, `States.collectStatistics`, `States.defineStateForms` → tipos de burg, stats, formas de state.
12. **Provincias** — `Provinces.generate`, `Provinces.getPoles` → `pack.provinces`.
13. **Nombres (polish)** — `Rivers.specify`, `Lakes.defineNames` → nombres de ríos/lagos.
14. **Economía** — `Markets.generate`, `Production.produce`, `States.collectTaxes` → `pack.markets`, `cells.market`, `pack.deals`, `burg.production`, treasuries.
15. **Milicia & overlays** — `Military.generate`, `Markers.generate`, `Zones.generate` → regimientos, markers, zones.
16. **Finalise** — `drawScaleBar`, `Names.getMapName`, `showStatistics` → barra de escala, nombre, stats.

Otros lugares que replican sub-pipelines:
- `heightmap-editor.js:regenerateErasedData` repite fases 3→15.
- `heightmap-editor.js:restoreRiskedData` repite fases 3→7 + remapeo de entidades preservadas.
- `src/generators/resample.ts:Resampler.process` repite fases 3→7 + remap de cell-data + regen-economy.

## 10. Parser/serializer del `.map` (formato binario-texto)

Fuente primaria: `src/services/io/save.ts:prepareMapData` (44-187) y `src/services/io/load.ts:parseLoadedResult/parseLoadedData` (167-197, 400+).

### 10.1 Formato

El archivo `.map` es un **array de strings unidos por `\r\n`**. Algunos elementos son JSON serializado, otros son `.toString()` (`.join(",")`) de TypedArrays. La versión va en el slot `[0]` como una pipe-delimited field. La estructura indexada:

| Slot | Contenido | Encoding |
|---|---|---|
| `[0]` | `VERSION\|license\|date\|seed\|graphWidth\|graphHeight\|mapId` | pipe-delimited |
| `[1]` | settings join por `\|` (distanceUnit, distanceScale, options, mapName, ...) | pipe-delimited |
| `[2]` | `mapCoordinates` | JSON |
| `[3]` | biomes (color\|habitability\|name) | pipe-delimited |
| `[4]` | notes | JSON |
| `[5]` | serialized SVG | XML string |
| `[6]` | `gridGeneral` (spacing, cellsX, cellsY, boundary, points, features, cellsDesired) | **JSON — único blob del grid** |
| `[7]` | `grid.cells.h` | Uint8 csv |
| `[8]` | `grid.cells.prec` | csv |
| `[9]` | `grid.cells.f` | Uint16 csv |
| `[10]` | `grid.cells.t` | Int8 csv |
| `[11]` | `grid.cells.temp` | Int8 csv |
| `[12]` | `pack.features` | JSON |
| `[13]` | `pack.cultures` | JSON |
| `[14]` | `pack.states` | JSON |
| `[15]` | `pack.burgs` | JSON |
| `[16]` | `pack.cells.biome` | Uint8 csv |
| `[17]` | `pack.cells.burg` | Uint16 csv |
| `[18]` | `pack.cells.conf` | csv |
| `[19]` | `pack.cells.culture` | Uint16 csv |
| `[20]` | `pack.cells.fl` | Uint16 csv |
| `[21]` | `pack.cells.pop` | Float32 (`rn(p,4)` csv) |
| `[22]` | `pack.cells.r` | Uint16 csv |
| `[23]` | deprecated `pack.cells.road` (vacío) | — |
| `[24]` | `pack.cells.s` | Uint16 csv |
| `[25]` | `pack.cells.state` | Uint16 csv |
| `[26]` | `pack.cells.religion` | Uint16 / JSON |
| `[27]` | `pack.cells.province` | Uint16 / JSON |
| `[28]` | deprecated `pack.cells.crossroad` (vacío) | — |
| `[29]` | `pack.religions` | JSON |
| `[30]` | `pack.provinces` | JSON |
| `[31]` | `namesData` (name\|min\|max\|d\|m\|b) | `/`-delimited |
| `[32]` | `pack.rivers` | JSON |
| `[33]` | deprecated rulers (vacío) | — |
| `[34]` | fonts | JSON |
| `[35]` | `pack.markers` | JSON |
| `[36]` | `pack.cells.routes` | JSON |
| `[37]` | `pack.routes` | JSON |
| `[38]` | `pack.zones` | JSON |
| `[39]` | `pack.ice` | JSON |
| `[40]` | `pack.cells.good` | Uint16 csv |
| `[41]` | `pack.goods` | JSON |
| `[42]` | `pack.markets` | JSON |
| `[43]` | `pack.deals` | JSON |
| `[44]` | `pack.cells.market` | Uint16 csv |
| `[45]` | customGoodIcons (HTML outer join ` ` replace CRLF) | string |
| `[46]` | `pack.measurers` | JSON |

### 10.2 Observaciones para el puerto

1. Los slots `[40]`..`[46]` son **añadidos reciente** (v1.124+, goods/markets/production/taxes). Filas más antiguas pueden tener versiones sin ellos; `auto-update.ts` reexpande los campos faltantes con defaults.
2. **NO existe un "objeto pack" serializado** — el pack está distribuido entre múltiples slots. El "objeto pack" lo reconstruye `parseLoadedData` llamando `calculateVoronoi` + `reGraph` sobre `grid` slot `[6]` y reaplicando slot-by-slot.
3. **El `gridGeneral` slot `[6]` es el único blob del grid**. Sin embargo solo guarda `points, spacing, cellsX, cellsY, boundary, features` — **no guarda ni `cells.h`, ni `cells.t`, ni `cells.f`, ni `cells.i`, ni `cells.v`, ni `cells.c`, etc.** Esos se reconstruyen:
   - `grid.cells` se reconstruye llamando `calculateVoronoi(grid.points, grid.boundary)` → recalcula `cells.i/v/c/b` desde los puntos. (Ver §11 abajo.)
   - `grid.cells.h/t/f/prec/temp` se reaplican de los slots `[7]`, `[10]`, `[9]`, `[8]`, `[11]` por id de grid.
   - `pack.cells` se reconstruye desde cero llamando `reGraph` con el grid ya reconstruido. Los atributos de pack se reaplican desde slots `[16]`..`[29]`, `[40]`, `[44]` por id de pack.
4. **Los slots `[13]` (cultures), `[14]` (states), `[15]` (burgs), `[29]` (religions), `[30]` (provinces), `[32]` (rivers), `[35]` (markers), `[38]` (zones), `[39]` (ice), `[41]` (goods), `[42]` (markets), `[43]` (deals), `[46]` (measurers)** son JSONs enteros (entidades, no cell-arrays).
5. **Slots vacíos/migrados**: `[23]`, `[28]`, `[33]` son deprecated; el parser ignora su content.
6. **CRLF mandatory**: el parser detecta `\r\n` como delimiter. Existe un `scripts/repair-map-line-endings.py` que arregla .map con CRLF rotas (siempre confía en que slot `[6]` arranca con `{"spacing"`). Ziencode handling a tener en cuenta en Rust: usar `String::from_utf8` sobre el contenido, separar por `"\r\n"`.
7. **Compresión OPCIONAL**: si el `parseLoadedResult` falla en detectar `|` entre slot `[0]` fields, intenta `uncompress` (gzip via `DecompressionStream`). Es decir, los `.map` pueden estar o no gzipped. Detector: auto-probar primero sin comprimir, y si no parsea, intentar gzip.

### 10.3 Export JSON (formato alterno)

`src/services/io/export-json.ts:exportToJson` con 4 variantes de payload:

- **Full**: todo — cells, vertices, entidades, settings. Es un dump más legible, pero no es lo que Azgaar carga de vuelta; es para inspección / interoperabilidad.
- **Minimal**: settings + entidades (sin cell-level attributes).
- **PackCells**: solo `pack.cells.*` y vertices del pack.
- **GridCells**: solo `grid.cells.*` y vertices del grid.

El "JSON export completo" que el plan maestro §23 pide disectar es el modo **Full**. Se inspeccionará con un mapa real exportado de prueba (ver §11).

## 11. Pendientes de esta fase

- [x] §12 Disección de un `.map` real de prueba.
- [x] §13 Validación empírica del hallazgo §3.

## 12. Disección de un `.map` real (Brample)

Archivo: `/home/hans/Descargas/Brample 2026-07-22-21-24.map` (~11.7 MB, `<XD.map`§ ya está fuera de scope por pedido del usuario).

### 12.1 Header (slot `[0]`)

```
1.138.0|File can be loaded in azgaar.github.io/Fantasy-Map-Generator|2026-7-22|861039636|2000|2000|1784767061245
```

Parse:
- `version` = `1.138.0` (calza con main del repo).
- `license` = `"File can be loaded in azgaar.github.io/Fantasy-Map-Generator"`.
- `date` = `2026-7-22` (formato año-mes-día sin zero-pad).
- `seed` = `861039636` (entero como string — Azgaar lo trata como string para `Alea(seed)`; usa `generateSeed()` que produce `String(Math.floor(Math.random()*1e9))`, así que siempre 1–10 dígitos numéricos).
- `graphWidth` = `2000`, `graphHeight` = `2000` (canvas 2000×2000).
- `mapId` = `1784767061245` (timestamp Date.now() al momento de creación).

### 12.2 Settings (slot `[1]`)

Comienza:
```
km|1|square|m|2|°C|||||||1000|1||||||{"pinNotes":false,"winds":[225,45,225,315,135,315],"t...
```

Parse complementario al código de `load.ts:254-295`:
- `[0] distanceUnit="km"`
- `[1] distanceScale=1`
- `[2] areaUnit="square"`
- `[3] heightUnit="m"`
- `[4] heightExponent=2`
- `[5] temperatureScale="°C"`
- `[6]–[11]` vacíos (escalas antiguas, semana, etc., ahora viven en style).
- `[12] populationRate=1000`
- `[13] urbanization=1`
- `[14]–[18]` vacíos (settings antiguos migrados a `options`, slot [19]).
- `[19]` = JSON completo de `options` (`{"pinNotes":false,"winds":[225,45,225,315,135,315],...}`).
- `[20] mapName`
- `[21] hideLabels`
- `[22] stylePreset`
- `[23] rescaleLabels`
- `[24] urbanDensity`
- `[25]` = decimal de longitude (legacy).
- `[26] growthRate` setting.

### 12.3 Mapeo de todos los slots confirmados

Reconstruí los 47 slots esperados en el archivo Brample. La tabla §10.1 arriba **está confirmada** por caso real. Algunos puntos relevantes del archivo:

| Slot | Tamaño (chars) | Contenido (Brample) |
|---|---|---|
| `[0]` | 112 | header pipe-delimited |
| `[1]` | 1627 | settings (con options embebido como JSON en `[19]`) |
| `[2]` | 65 | `mapCoordinates` JSON (`{"latT":180,"latN":90,"latS":-90,...}`) |
| `[3]` | 309 | biomas: `colors,habitability,fields` pipe-delimited (12 biomas default). |
| `[4]` | 44954 | notes (JSON de regimientos/markers legend). |
| `[5]` | 3,344,024 (~3.2 MB) | **SVG serialized** (todo el DOM visual, ~28% del archivo). |
| `[6]` | 169,634 (~170 KB) | **`gridGeneral` JSON** — el único blob del grid visto como `{spacing,cellsX,cellsY,boundary,points,features,cellsDesired}`. Confirmado: **sin `cells.*` aquí.** |
| `[7]` | 25,756 | `grid.cells.h` (Uint8 csv). Pattern `0,0,0,0,...,1,2,2,1,1,1,...,3,6,7,6,11,8,9,9,...` |
| `[8]` | 20,645 | `grid.cells.prec` (csv). |
| `[9]` | 20,044 | `grid.cells.f` (Uint16 csv, casi todos `1` — 1 feature única). |
| `[10]` | 23,725 | `grid.cells.t` (Int8 csv, valores 0/-1/-2/...). |
| `[11]` | 30,280 | `grid.cells.temp` (Int8 csv `-27,-27,...`). |
| `[12]` | 12,027 | `pack.features` JSON `[0,{i:1,type:"ocean",land:false,...}]`. El `[0]` es el "null"/placeholder reservado. |
| `[13]` | 2,258 | `pack.cultures` JSON `[{name:"Wildlands",i:0,...},...]`. |
| `[14]` | 57,438 | `pack.states` JSON. |
| `[15]` | 3,286,398 (~3.1 MB) | `pack.burgs` JSON `[{},{"cell":1133,"x":1468.66,"y":567.28,"state":1,"name":"Tal",...}]`. Otro 28% del archivo. |
| `[16]` | 11,441 | `pack.cells.biome` (Uint8 csv). |
| `[17]` | 12,299 | `pack.cells.burg` (Uint16 csv). |
| `[18]` | 11,454 | `pack.cells.conf` (csv). |
| `[19]` | 11,441 | `pack.cells.culture` (Uint16 csv). |
| `[20]` | 12,299 | `pack.cells.fl` (Uint16 csv). |
| `[21]` | 33,089 | `pack.cells.pop` (Float32 csv `rn(p,4)` — `0,0,...,53.9622,28.4531,50.2828,...`). |
| `[22]` | 11,921 | `pack.cells.r` (Uint16 csv). |
| `[23]` | 0 (vacío) | deprecated `road`. |
| `[24]` | 13,008 | `pack.cells.s` (Uint16 csv). |
| `[25]` | 13,856 | `pack.cells.state` (Uint16 csv). |
| `[26]` | 11,594 | `pack.cells.religion` (Uint16 csv). |
| `[27]` | 17,146 | `pack.cells.province` (Uint16 csv). |
| `[28]` | 0 (vacío) | deprecated `crossroad`. |
| `[29]` | 3,733 | `pack.religions` JSON. |
| `[30]` | 35,207 | `pack.provinces` JSON. |
| `[31]` | 884 | `namesData` `German|5|12|lt|0|/English|6|11|...`. |
| `[32]` | 22,600 | `pack.rivers` JSON. |
| `[33]` | 0 (vacío) | deprecated `rulers`. |
| `[34]` | 299 | `fonts` JSON (`[{family:"Georgia"},{family:"Underdog",src:"url(...)",...}]`). |
| `[35]` | 4,953 | `pack.markers` JSON (`[{icon:"🌋",type:"volcanoes",dx:52,px:13,x:...,y:...,cell:...,i:0},...]`). |
| `[36]` | 74,398 | `pack.cells.routes` JSON (`{"6":{"7":359,"39":359},"7":{...}}` — adjacency map). |
| `[37]` | 103,538 | `pack.routes` JSON (`[{i:0,group:"roads",feature:2,points:[[758.56,351.83,325],...]}]`). |
| `[38]` | 2 (`[]`) | `pack.zones` (vacío en este mapa). |
| `[39]` | 1,162 | `pack.ice` JSON. |
| `[40]` | 12,290 | `pack.cells.good` (Uint16 csv). |
| `[41]` | 17,591 | `pack.goods` JSON `[{i:1,name:"Wood",tags:["construction","fuel"],icon:"good-wood",color:"#966F33",value:1,...}]`. |
| `[42]` | 64,236 | `pack.markets` JSON. |
| `[43]` | 4,100,313 (~3.9 MB!) | `pack.deals` JSON (`[{i:0,seller:18,sellerType:"market",buyer:332,buyerType:"burg",good:21,units:1,price...}]`). **Slot individual más grande del archivo** (33% del total). |
| `[44]` | 14,675 | `pack.cells.market` (Uint16 csv). |
| `[45]` | — (no capturé tamaño, debría ser HTML outer de custom iconos) | `customGoodIcons`. |
| `[46]` | — | `pack.measurers` JSON. |

**Σ tamaño**: ~11.7 MB, mayormente dividido entre SVG (3.2 MB), burgs (3.1 MB) y deals (3.9 MB) —eatures самойdeal-heavy.

### 12.4 Vista del slot `[6]` (gridGeneral) — confirmación literal

```json
{"spacing":20,"cellsX":100,"cellsY":100,"boundary":[[1,-20],[1,2020],[42,-20],[42,2020],[82,-20],[82,2020],...],"points":[[10.12,10.34],[30.88,10.56],...],"features":[...],"cellsDesired":10000}
```

- `spacing` = 20 → spacing entre puntos (en unidades de canvas; canvas 2000×2000, cellsX/Y=100 → esperados 100×100 = 10000 puntos).
- `cellsDesired` = 10000 — el usuario pidió "10k cells" en la UI (es el default).
- `points` es el arreglo de `[x,y]` con jitter (e.g. `[10.12, 10.34]`, `rn(x + jitter(), 2)`).
- **`boundary`** son los bordes virtuales fuera del canvas para cells edge; consta de duplas `[x, -20]` y `[x, 2020]` (y similares para eje y).
- Importante: `cells` no está en el JSON. Confirmado en §13 abajo.

### 12.5 Validación del algoritmo de parsing

El parser real (`load.ts:178-186`) cambia `\r\n` por `\n` dentro del bloque SVG `<svg id="map" ...</svg>` antes de hacer split. El split es siempre por `\r\n`, **no** por `\n` suelto.

⚠️ **Caso edge no cubierto por el parser de Azgaar**: archivos guardados con `\n` suelto (como Brample, que fue normalizado por el OS o el download del browser) **NO pueden ser loaded por la versión main actual de Azgaar**. Su parser divide por `\r\n` y como no hay ninguno, queda solo 1 slot; `JSON.parse(data[6])` revienta por parsear el todo-el-archivo. El fallback gzip intenta decomprimir, también falla. Para Voronia: el parser debe contemplar ambos separadores (\r\n o \n con SVG-rescuing) — no confiar en que el archivo preserve CRLF.

## 13. Validación empírica del hallazgo §3 (JSON no guarda geometría)

**Confirmado**: la hipótesis del plan maestro §3 es correcta, fortalecida en dos puntos.

### 13.1 El slot `[6]` NO persiste la malla

El JSON del slot `[6]` (Brample) contiene exactamente:

```json
{
  "spacing": 20,
  "cellsX": 100,
  "cellsY": 100,
  "boundary": [[...], ...],
  "points": [[...], ...],
  "features": [...],
  "cellsDesired": 10000
}
```

NO contiene `cells`, `vertices`, ni `seed` (este último se encuentra en el header slot `[0]`, campo `params[3]`). `cells.i/v/c/b/h/t/f/prec/temp` están todos derivados o guardados en slots posteriores `[7]-[11]`.

### 13.2 El código `load.ts` reconstruye geom desde `points`

`src/services/io/load.ts:404-409`:

```ts
grid = JSON.parse(data[6]);
const { cells, vertices } = calculateVoronoi(grid.points, grid.boundary);
grid.cells = cells;
grid.vertices = vertices;
grid.cells.h = Uint8Array.from(data[7].split(","), Number);
grid.cells.prec = Uint8Array.from(data[8].split(","), Number);
// ... y así con t, f, temp (slots 9, 10, 11)
```

Es decir: el parser **no lee `cells.v/c/i` desde el JSON** — los **reconstruye** invocando `calculateVoronoi(grid.points, grid.boundary)` (que a su vez corre `Delaunator.from(...)` + `new Voronoi(...)` y pobla `cells.i = 0..pointsN-1`).

### 13.3 El pack completo (cells) también se reconstruye

Inmediatamente después de `parseLoadedData`, el loader ejecuta:

- `reGraph()` sobre el grid ya reconstruido → recalcula `pack.cells.i/v/c/b/p/g/h/area` desde cero.

Y los atributos de pack (bioma, burg, culture, state, religion, province, pop, fl, r, s, conf, haven, harbor, good, market, routes) — slots `[16]`, `[17]`, `[18]`, `[19]`, `[20]`, `[21]`, `[22]`, `[24]`, `[25]`, `[26]`, `[27]`, `[40]`, `[44]` — se reaplican indexados por el id de pack reconstruido.

### 13.4 Consecuencias reforzadas (vs plan maestro §3)

1. **El plan §3 ya decía esto**: el JSON no guarda geometría; hay que reproducir bit-exacto `placePoints → getJitteredGrid → Delaunator.from → Voronoi → reGraph`. Confirmado in-vivo.
2. **Nueva consecuencia**: también hay que reproducir el orden de inserción de los puntos en `points` (`getJitteredGrid` itera fila-mayor, x dentro de y), porque `Delaunator.from` usa el índice de punto como `pointId`, y los `cells.i` reconstruidos son `0..pointsN-1` en ese orden. Si Voronia itera en otra dirección, los ids están permutados.
3. **Nueva consecuencia**: el pack se reconstruye llamando `reGraph()` con el grid reconstruido. Si Voronia tiene algún bug en reGraph (e.g. omite el `i > e` check o el `i % 4 === 0` check del paso de lagos), el mapeo `pack.cells.g[k] = gridId` queda distinto, y entonces todos los atributos del pack (bioma, state, burg, ...) que vienen indexados por `k` se aplican a la celda equivocada. **Bug silencioso sin error en runtime.**
4. **Confirmado**: el archivo Prample todavía vineene con settings/`options` serializado en slot `[1]`, que incluye el resultado del `randomizeOptions()` (donde se hace el primer consumo del PRNG con `aleaPRNG`). Si Voronia solo quiere cargar mapas ya generados, **no necesita portear `aleaPRNG`/`randomizeOptions`** — puede importar las options serializadas de slot `[1]`. Esto recorta la Fase 1 sustancialmente respecto a lo que parecía en §7.4.
5. **Confirmado**: las dimensiones (`graphWidth`, `graphHeight`) vienen en el header slot `[0]`, fields `[4]` y `[5]`. La seed está en el header slot `[0]` field `[3]`. Esto es lo primero que Voronia parsea para reconstruir `placePoints` → cadena determinista.

### 13.5 Conclusión para Fase 1 (plan §23)

Para el camino más corto "importar un `.map` y mostrarlo en el visor" (Fase 1+2):

- **Portear**:
  1. `alea@1.0.1` (npm versión) → `Math.random` patchable en Rust (por lo menos un `dyn Rng` seedable que produzca floats con la misma algebra de Baagøe).
  2. `getJitteredGrid` + `getBoundaryPoints` + `placePoints` (`src/utils/graphUtils.ts:17-98`) — la geometría del grid.
  3. `delaunator@5.0.1` (Mapbox). Validar bit-exacto (el crate `delaunator` de Rust es port de la misma lib, debería calzar).
  4. `Voronoi` class, **incluida la trampa de `Math.floor` en `circumcenter`** (`src/generators/voronoi.ts:142-154`).
  5. `reGraph()`, **incluida el `i % 4 === 0` para descarte de lagos no-costeros** y el cap de 20 half-edges en `edgesAroundPoint` (`public/main.js:1157-1209`).
- **NO need portear** (para solo import):
  1. `aleaPRNG 1.1.0` (a.k.a. `public/libs/alea.min.js`).
  2. `randomizeOptions()` y `applyGraphSize()` (el tramo setSeed → generateGrid del pipeline). Solo se importan las options ya serializadas.
  3. Cualquier generador procedural (heightmap, rivers, biomes, cultures, ...) — estos producen los atributos que ya están serializados en el `.map`.
- **A futuro (Fase 7)**: si Voronia quiere generar mapas desde cero con su propio seed (no solo importar), recién ahí hay que portear todo lo demás, incluyendo `aleaPRNG` + `randomizeOptions`.
