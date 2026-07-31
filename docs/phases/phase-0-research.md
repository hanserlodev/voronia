# Phase 0 — Research and laying the foundations

> Consolidated output of Phase 0 of the master plan (§23). Everything that follows was obtained by reading the source code and documentation of the locally cloned Azgaar Fantasy Map Generator repo. When Phase 0 closes, the checkboxes of §23 of the master plan are checked off and this file remains as a frozen reference for Phase 1+.

## 0. Azgaar reference (frozen reference)

- **Repo**: https://github.com/Azgaar/fantasy-map-generator (case-insensitive on GitHub).
- **Cloned ref**: `origin/main`, shallow (`--depth 1`).
- **Local path**: `/home/hans/Proyectos/azgaar-fmg/` (sister folder of `voronia`, outside the Voronia repo so as not to interfere with git status or the workspace build).
- **Commit**: `51d8e3e487a28995aac2304af57ad1ac4fbe3789` (2026-07-21).
- **Version declared in package.json**: `1.135.2`.
- **Version of the latest (bump) commit**: `1.138.0` → there is a lag between package.json and the bump commit, normal in main. For comparison purposes, the effective reference version is **1.138.0**.
- **Master plan §25 cited `v1.119` as the latest detected release** → outdated: the current reality of main is ~20 versions newer. This does not invalidate the plan, but it is good to know before comparing hal rounding.
- **Azgaar license**: MIT (identical to Voronia's, no legal friction for deriving).

## 1. Confirmed JS stack (from `package.json` v1.135.2)

### Runtime deps (relevant for the Rust port)

| JS dep | Version | What it does in Azgaar | Planned Rust equivalent |
|---|---|---|---|
| `alea` | ^1.0.1 | **Seedable PRNG** — critical per §3 | `rand_pcg` (to confirm that the `alea` algorithm matches some concrete PCG) |
| `delaunator` | ^5.0.1 | Delaunay triangulation | `delaunator` crate (bit-exactness to be validated) |
| `d3` | ^7.9.0 | Various helpers (scale, paths, geo, voronoi-delaunay in d3-delaunay) | n/a — reimplement the specific parts |
| `three` | ^0.184.0 | Optional 3D renderer (3d view) | wgpu |
| `polylabel` | ^2.0.1 | Find the "pole of inaccessibility" for labels | reimplement (García/Castro algorithm) |
| `lineclip` | ^2.0.0 | Line clipping (Cohen–Sutherland) | reimplement (Lyon lines also work) |
| `driver.js` | ^1.4.0 | UI tour (not related to the engine) | — |

### Relevant dev deps

- **Gradual TypeScript** (`tsc && vite build`); legacy `.js` and `.ts` still coexist.
- **Vite** (dev/build), **Vitest** (tests), **Playwright** (e2e), **Biome** (lint/format), `simple-git-hooks` (pre-commit = `biome check --write`).
- `engines.node` `>=24.0.0`.

## 2. Architecture declared by Azgaar itself (from `README.md`)

Literal from the README (lines 35-41):

> The expected **future** architecture is based on a separation between **world data**, **procedural generation**, **interactive editing**, and **rendering**. [...] The application is conceptually divided into four main layers: world data and styles (state), generators (model), editors (controllers), renderers (view).
>
> Flow:
> - `settings → generators → world data → renderer`
> - `UI → editors → world data → renderer`
>
> The data layer must contain no logic and no rendering code. Generators implement the procedural world simulation. Editors implement interactive editing tools used by the user. [...] The renderer converts the world state into SVG or WebGl graphics. Renderer must be pure visualization step and not modify world data.

**This directly validates the Voronia architecture** (plan §5: `vor-core` = world data without logic, `vor-sim` = generators, `vor-edit` = editors, `vor-render` = renderer that never mutates world). It is the same separation, exposed by Azgaar itself as "future architecture" — interesting: it is not yet fully applied in their codebase, but it is the official direction.

`main.js` at the root is still the legacy entry point; TS migrates gradually.

## 3. Structure of Azgaar's `docs/` (to read in this phase 0)

```
docs/
├── architecture/   — code internals
│   ├── architecture.md          (28KB)
│   ├── data_model.md             (33KB) ← key: per-cell data schema
│   ├── future_data_model.md      ( 4KB)
│   ├── lazy_loading.md           ( 5KB)
│   └── migration_guide.md        (13KB)
├── domain/         — domain models
│   ├── generation_pipeline.md   (13KB) ← key: generation order and RNG consumption order
│   ├── glossary.md
│   ├── goods_schema.md          (11KB)
│   ├── production_schema.md     ( 9KB)
│   ├── trade_schema.md          (13KB)
│   ├── taxes.md
│   └── 3d-view.md
├── prd/            — specs for specific features
│   ├── controller-service-registry.md (13KB)
│   ├── good-multipliers.md
│   ├── meandering-river-routes.md     (22KB)
│   ├── navigable-river-routes.md      (20KB)
│   └── state-taxes-and-treasury.md
└── updates/
    ├── v1.123.0/
    └── v1.124.0/
```

## 4. Key wikis (summary of each)

Links:
- Heightmap customization: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Heightmap-customization
- Heightmap template editor: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Heightmap-template-editor
- Culture types: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Culture-types
- Military Forces: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Military-Forces
- Goods: spread functions: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Goods:-spread-functions

### 4.1 Heightmap customization (UI/UX)

Documents the **Heightmap Editor** as a user feature (not engine). Points relevant to Voronia:

- Three modes when editing an existing heightmap: **Erase** (regenerates everything), **Keep** (keeps data, does not touch coastline), **Risk** (keeps data but allows changing coastline — can break things).
- Brushes: Raise / Elevate / Lower / Depress / Align / Smooth / Disrupt (the last four operate against neighbors). Line tool for linear mountains/valleys. "change only land cells" checkbox protects the coastline.
- Editor footer: Undo/redo (heightmap only), Rescaler (rescales all heights), Condition (conditional rescaler), Smooth global, Disrupt global, Clear (= all to 0).
- **Image Converter**: uploads an image and maps colors→heights by (luminosity | hue | Azgaar's own scheme). Set max amount of colors (3–255). Unassigned colors fall to ocean.
- The wiki does not document the exact brush formulas — those live in the code (see §6/§7). For Voronia: **brush editing** is part of `vor-edit` (plan §11); image→heightmap conversion can be deferred to Phase 6 or later.

### 4.2 Heightmap template editor ( Heightmap templates )

Documents the **template editor** — the textual DSL that describes how to generate a heightmap step by step. **Very relevant for `vor-sim`** (Phase 7).

- A *template* is a sequence of **actions** applied in order on a freshly cleared heightmap:
  - `Hill n:.. h:.. x:.. y:..` — hill (raises the surroundings).
  - `Pit n:.. h:.. x:.. y:..` — pit (sinks the surroundings).
  - `Range n:.. h:.. x:.. y:..` — mountain range (elongated elevation).
  - `Trough ...` — elongated depression.
  - `Strait w:.. d:{vertical|horizontal}` — strait (lowers a line, splits land). Location cannot be chosen.
  - `Mask <fraction>` — lowers heights gradually toward the edge; negative fraction inverts (lowers the center).
  - `Invert` — mirrors over X / Y / both, with probability.
  - `Add V:<n> to all cells` — global offset (negatives allowed).
  - `Multiply V:<n>` — global multiplier (typically decimal).
  - `Smooth` — averages against neighbors.
- Each step has: `n` (how many times, can be a random range), `h` (height range 1–100; 20 = coastline, 100 = peak), `x` and `y` (percentage ranges 0–100 of the map). A step can be skipped with a checkbox.
- For bit-exact reproduction: if the seed is locked, the result is identical between runs. **Implies a determined RNG consumption per step** → see §5 (RNG consumption order).

### 4.3 Culture types (cultures + spread)

Documents how the culture type is assigned at generation time and how each culture expands competitively. Relevant for `vor-sim` (Phase 7).

- **7 types**: Nomadic / Highland / Lake / Naval / River / Hunting / Generic. The type is assigned at generation time based on the geography of the spawn (first rule that matches, except Naval which also does a probability check).
- Rules per type (with high `h` = height):
  - Nomadic: desert or grassland with height < 70.
  - Highland: height > 50.
  - Lake: next to lakes of > 5 cells.
  - Naval: coastal cells (more chance if oceanic coast, even more if island; extra random roll).
  - River: cells with a river of flux > 100.
  - Hunting: >2 cells from the coast AND biome ∈ {Savanna, Rain forest, Taiga, Tundra, Wetland}.
  - Generic: fallback.
- **Culture spread** is a competitive Dijkstra-like cost: each culture competes for cells according to type-dependent costs.
  - **Expansionism** base × multiplier per type: Generic 1, Lake 0.8, Naval 1.5, River 0.9, Nomadic 1.5, Hunting 0.7, Highland 1.2. Higher expansionism → lower cost → more territory.
  - **Biome cost** base: Hot desert 200, Cold desert 150, Savanna 60, Grassland 50, Trop seasonal forest 70, Temp deciduous forest 70, Trop rainforest 80, Temp rainforest 90, Taiga 200, Tundra 1000, Glacier 5000, Wetland 150. If it is the native biome → cost 10. Hunting × 5 in non-natives. Nomadic × 10 in the 5 forested biomes. Rest × 2. +20 if the biome differs from the origin.
  - **Water crossing cost**: Lake 10 when crossing its own lake. Naval = 2 × cell size(pixel) when crossing water. Nomadic = 50 × size(pixel). Rest (incl. Lake crossing sea) = 6 × size(pixel).
  - **River crossing cost**: non-River +20..100 depending on flux. River: river is free, but non-river +100.
  - **Distance to coast cost**: Nomadic +60 on coast; other non-Naval non-River +20. Naval and Nomadic +30 on cells adjacent to coast. Naval and River +100 far from coast. Costs are summed and then divided by expansionism. When the total cost > budget, expansion stops.

### 4.4 Military Forces (militia)

Documents the military regiment system — determined consumption of population and cells. Relevant for `vor-sim` (Phase 7); master plan §7.6 covers the entities.

- **War Alert** per state = `Expansion_fulfillment (expansionism/area) + Diplomatic_alert`. Relations: Ally −0.2, Friendly −0.1, Neutral 0, Suspicion +0.1, Enemy +1, Unknown 0, Rival +0.5, Vassal +0.5, Suzerain −0.5.
- **State-type modifier** × unit-type (8 types: Melee, Ranged, Mounted, Machinery, Naval, Armored, Aviation, Magical) — 7×8 matrix in the wiki (rows: Generic, Nomadic, Highland, Lake, Naval, Hunting, River). Hordes × 2 in mounted; Republics × 1.2 in naval.
- **Cell-unit modifier** × biome/height of the cell: Nomadic, Wetland, Highland (height > 70) — three separate matrices. Same for burgs (Nomadic/Wetland/Highland). See the wiki for the exact values (I don't duplicate them here to avoid breaking maintenance).
- **Troop formula per cell/burg**:
  `Troops = Population_points / 100 × Possession_divider × Unit_percentage × State_mod × Unit_mod × Population_rate`
- **Possession_divider** applies if: cell culture ≠ dominant culture of the state (Unions 1.2, others 2); cell religion ≠ dominant religion (Theocracies 2.2, others 1.4); cell on an island different from the state center (Naval 1.2, others 1.8).
- **Naval units** only in port-burgs.
- Platoons (1 per cell+burg) are added to **regiments**. Regiment expected size = `3 × Population_rate`. When the regiment reaches the expected size, a new one is created. Naming and details are generated later, positioned over the map.

### 4.5 Goods: spread functions (goods DSL)

Documents the *spread models* DSL — boolean expressions evaluated per cell to decide whether a good can appear. **Relevant for `vor-sim`** (Phase 7 / economy §10.8) and eventually for plugins (plan §21.4).

- Goods are generated **before** states and cultures — the models rely only on geography (biome, height, temp, coast, rivers). Changing a model requires regenerating all goods.
- Technically, each spread model is a **valid JS expression** evaluated per cell → `true`/`false`. JS logical operators allowed (`!`, `||`, `&&`).
- Built-in DSL functions:
  - `random(n)`: true with probability `n%`.
  - `nth(n)`: true every n-th cell (`nth(2)` = 50%, `nth(5)` = 20%).
  - `habitable()`: biome with habitability > 0.
  - `habitability()`: checks against the biome's habitability (≥100 always, 0 never, 50 → 50%).
  - `elevation()`: against the cell's height (higher height → higher chance). Negatable with `!`.
  - `biome(id,...)`: biome ids.
  - `minHeight(n)`, `maxHeight(n)`: range 0–100, 20 = coastline.
  - `minTemp(n)`, `maxTemp(n)`: in Celsius.
  - `shore(ring,...)`: distance to shoreline. `1` = land coast, `2` = next land ring, `-1` = shallow water, `-2 -3...` = deeper.
  - `type(str,...)`: ocean | freshwater | salt | sinkhole | frozen | lava | dry | continent | island | isle | lake_island. The wiki discourages `type` for land.
  - `river()`: there is a river in the cell.
- **Biome ids** (default, order matters):
  `0 Marine, 1 Hot desert, 2 Cold desert, 3 Savanna, 4 Grassland, 5 Tropical seasonal forest, 6 Temperate deciduous forest, 7 Tropical rainforest, 8 Temperate rainforest, 9 Taiga, 10 Tundra, 11 Glacier, 12 Wetland`. (To get the real ids at runtime: `biomesData.name.map((n,i)=>i+". "+n)` in the browser console.)
- **Built-in models** (quick reference): Deciduous_forests `biome(6,7,8)`, Hills `minHeight(40) || (minHeight(30) && nth(10))`, Mountains `minHeight(60) || (minHeight(40) && nth(10))`, Headwaters `river() && minHeight(40)`, Marine_and_rivers `type("ocean","freshwater","salt") || (river() && shore(1,2))`, Tropical_waters `shore(-1) && minTemp(18)`, Arctic_waters `biome(0) && maxTemp(7)`. (Full list in the wiki.)
- Consequence for Voronia: this DSL is an attack surface and a natural extension point. In Rust, one option is an interpreter of the subset + a mini-parser (or a predicate enum); a longer-term alternative is to allow WASM plugins (plan §21.4).
- ⚠️ The spread model is evaluated per cell; since goods are generated before states/cultures, anything Voronia does that changes the generation order or the cell id changes the outcome —_remember §3 and §6/§7_.

---

## 5. Structure of `src/` (module map)

Top-level tree (10 entries):

```
src/
├── components/    (3 files)   Custom elements (fill-box, slider-input) + barrel.
├── controllers/  (~60 files) UI editors/overlays, lazy-load via window.Controllers.
├── data/          (4 files)   Static data (heightmap-templates, precreated-heightmaps, supporters, view-3d-options).
├── generators/   (~25 files) Procedural pipeline (voronoi, heightmap, features, lakes, rivers, biomes, cultures, states, religions, provinces, routes, zones, ice, military, markers, measurers, goods, production, markets, draw-goods, resample) + emblems/ subtree.
├── renderers/    (~25 files) SVG drawing + emblems/ subtree. Side-effect barrel.
├── services/      (4 files + io/) High-level services (Cloud, ExportJson, ExportMap, Installation, Load, Save, UiTour) + io/ subtree (save.ts, load.ts, export.ts, export-json.ts, cloud.ts, auto-update.ts).
├── types/         (2 files)   global.ts (declares legacy globals) + PackedGraph.ts (canonical type of the pack).
├── utils/         (18 files)  Pure helpers (graphUtils, probabilityUtils, pathUtils, arrayUtils, numberUtils, colorUtils, commonUtils, functionUtils, languageUtils, nodeUtils, stringUtils, unitUtils, debugUtils, registry, polyfills).
└── index.html                 DOM entry-point (loads alea.min.js and delaunator.min.js as <script>).
```

### Critical files for geometry/PRNG (with exact path)

| Purpose | Path |
|---|---|
| Jittered grid | `src/utils/graphUtils.ts:getJitteredGrid` (lines 46-61) + `placePoints` (69-98) |
| Delaunay + Voronoi assembly | `src/utils/graphUtils.ts:calculateVoronoi` (159-177), imports `Delaunator` and `Voronoi` |
| Voronoi class (derives cells/vertices) | `src/generators/voronoi.ts` (155 lines, `class Voronoi`) |
| Repack grid→pack | `public/main.js:reGraph` (1157-1209) — **legacy JS** |
| PRNG wrapper | No dedicated wrapper: monkey-patching of `Math.random`. Helpers in `src/utils/probabilityUtils.ts` |
| 2nd version of alea (legacy) | `public/libs/alea.min.js` (`aleaPRNG 1.1.0` by macmcmeans) |
| Canonical pipeline | `public/main.js:generate()` (lines 650-680 approx) — **legacy JS**, ~16 phases |
| First-use driver of the RNG | `public/modules/ui/options.js:randomizeOptions` (607) — **legacy JS** |
| .map parser | `src/services/io/load.ts:parseLoadedResult/parseLoadedData` + `src/services/io/save.ts:prepareMapData` |
| JSON export | `src/services/io/export-json.ts` (Full/Minimal/PackCells/GridCells modes) |
| .map version migrations | `src/services/io/auto-update.ts` (1243 lines) |
| Canonical `pack` type | `src/types/PackedGraph.ts` (70 lines) — complete list of `pack.cells.*` and `pack.{rivers,features,burgs,states,cultures,routes,religions,zones,markers,ice,provinces,goods,markets,deals,measurers}` |
| Utils barrel (injects into `window.*`) | `src/utils/index.ts` (269 lines) — legacy TS↔JS bridge |

### JS→TS migration status (summary)

- **Geometry and PRNG: ~95% migrated to TS** (`graphUtils.ts`, `voronoi.ts`, `probabilityUtils.ts` are complete and typed).
- **The pipeline driver and the first RNG consumption remain in legacy JS** (`public/main.js:generate/setSeed/reGraph/...`, `public/modules/ui/options.js:applyGraphSize/randomizeOptions`). Any Rust port has to read the legacy JS as the source of truth for that part.
- **The runtime loads two different alea versions**: `public/libs/alea.min.js` as a `<script>` (in `src/index.html`) and `alea@1.0.1` from npm (in `graphUtils.ts:1`). Both must be kept at hand to reproduce bit-exact (see §7).

## 6. Delaunay/Voronoi — exact algorithm

Primary source: `src/generators/voronoi.ts` (155 lines) — `Voronoi` class with docstrings.

### 6.1 Delaunay

- Library: **`delaunator@5.0.1`** (Mapbox, npm). Pure, no variants.
- Main usage point: `src/utils/graphUtils.ts:162` → `Delaunator.from(allPoints)` where `allPoints = points.concat(boundary)`.
- Secondary usage point: `src/generators/routes-generator.ts:241` → `Delaunator.from(points)` for road adjacencies (not of the grid). Does not affect Azgaar's mesh, only routes.
- The legacy bundle `public/libs/delaunator.min.js` also loads as a global for the legacy JS modules, but the TS code already uses the npm version directly.

### 6.2 Voronoi — derivation from Delaunay

Constructor `new Voronoi(delaunay, allPoints, pointsN)` in `voronoi.ts:25`. It iterates over each half-edge `e ∈ [0, delaunay.triangles.length)` and derives two structures:

- **`cells`** (entities per point `p ∈ [0, pointsN)`):
  - `cells.v[p]` — IDs of vertices (Voronoi points) surrounding point `p`. Computed with `edgesAroundPoint(e)` which walks half-edges via `nextHalfedge(e)` + `delaunay.halfedges[]`. Explicit cap of **20 half-edges** (`result.length < 20`) to avoid infinite loops.
  - `cells.c[p]` — IDs of neighboring cells, filtered `c < pointsN` (discards boundary points).
  - `cells.b[p]` — flag `1` if `edges.length > cells.c[p].length` (i.e. the point touches the boundary: more half-edges than valid neighbors within the real points range).
  - `cells.i` — populated externally, in `calculateVoronoi:169-172`: `createTypedArray({maxValue: points.length, length: points.length}).map((_, i) => i)`. That is, `cells.i[k] === k` for all `k ∈ [0, pointsN)`. Therefore **the final cell id is k = point index in `placePoints`** — this is what makes the correspondence between RNG output (jitter in order) and per-cell attributes deterministic.

- **`vertices`** (entities per triangle `t = floor(e/3)`):
  - `vertices.p[t]` — **circumcenter coordinates** of triangle `t` (`triangleCenter(t)`, which calls `circumcenter(a,b,c)`).
  - `vertices.v[t]` — IDs of neighboring triangles (via opposite `halfedges[]`).
  - `vertices.c[t]` — the 3 point IDs that make up triangle `t`.

### 6.3 ⚠️ The `circumcenter` trap (critical for bit-exactness)

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

Critical observations:

1. **`Math.floor` truncates to integer**, so the Voronoi vertex coordinates are **integers** (deliberate precision loss, inherited from Azgaar's original JS version). Any Rust port that uses raw `f32` or `f64` for `vertices.p` will not match: if Voronia computes the circumcenter in pure floats, the `vertices.p[t]` will have decimals where Azgaar has integers, and that changes the sub-pixel order of subsequent operations (raster, picking, polygon area in `reGraph`, etc.).
2. The coordinates of the **Voronoi points depend on** the coordinates of the input points; hence they also depend on `rn(x + jitter(), 2)` and `rn(y + jitter(), 2)` — the jitters are rounded to 2 decimals (see §6.5). And those `f64` values feed into the circumcenter; multiplications **are not float32**: JS always uses doubles.
3. **`D = 2*(ax*(by-cy) + bx*(cy-ay) + cx*(ay-by))`** can be 0 (degenerate triangle) — the code does not guard against it. It is an edge case to test, although Delaunator should avoid strictly collinear points in practice.

### 6.4 Helper macros (`triangleOfEdge`, `nextHalfedge`, `edgesOfTriangle`)

Standard Delaunator helpers (taken verbatim from Mapbox docs):

- `triangleOfEdge(e) = Math.floor(e / 3)`
- `nextHalfedge(e) = (e % 3 === 2) ? e - 2 : e + 1`
- `edgesOfTriangle(t) = [3t, 3t+1, 3t+2]`
- `pointsOfTriangle(t) = edgesOfTriangle(t).map(e => delaunay.triangles[e])`
- `trianglesAdjacentToTriangle(t) = edgesOfTriangle(t).map(e => triangleOfEdge(halfedges[e]))`

### 6.5 Jitter-exactness (pre-Delaunay)

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

Details critical for bit-exactness:

1. **Row-major iteration**: `y` is the outer loop, `x` the inner. The RNG consumption order is `(y0,x0), (y0,x1), ..., (y1,x0), ...`. And per cell it draws **a single random** (`xj` consumes one, `yj` consumes another) x-first.
2. `rn(x + jitter(), 2)` rounds to 2 decimals — `rn` comes from `numberUtils.ts`.
3. `Math.min(..., width)`/`(..., height)` clamps to the size; clamping happens **after** `rn(.,2)`. No lower clamp (the initial `radius` values guarantee `x,y >= radius > 0`).
4. When `generateGrid(seed, w, h)` is called, the **first line** of the body is `Math.random = Alea(seed);` (line 137). This **resets the PRNG** — the first `Math.random` that `getJitteredGrid` consumes is the first call to `Alea(seed)` post-construction.

### 6.6 Boundary points

`getBoundaryPoints` (`graphUtils.ts:17-37`): generates points on the canvas border to avoid infinite cells in the Voronoi. Apart from the computation (`offset = rn(-1 * spacing)`, `bSpacing = spacing * 2`), the critical detail is that it **does not consume RNG** — the boundary is purely deterministic as a function of `graphWidth, graphHeight, spacing`.

## 7. PRNG — exact algorithm

### 7.1 Two different `alea` versions in a single run

This is the finding that complicates the Rust port (reinforces §3 of the master plan):

| Wrapper | Version | Source | Where it is seeded | Coverage |
|---|---|---|---|---|
| `Alea` (npm, ^1.0.1) | 0.1 of Johannes Baagøe's original `seedrandom` | `import Alea from "alea"` in `src/utils/graphUtils.ts:1` and ~9 TS generators | `Math.random = Alea(seed)` in `graphUtils.ts:137` (inside `generateGrid`) | The whole TS procedural pipeline from `generateGrid` onward |
| `aleaPRNG` (vendored) | 1.1.0 by macmcmeans (includes `Mash` and `.int`, `.uint` methods) | `public/libs/alea.min.js` loaded via `<script>` | `Math.random = aleaPRNG(seed)` in `public/main.js:762` (inside `setSeed`) | The `setSeed → generateGrid` stretch, including `randomizeOptions()` (the first generative consumption) |

Both versions are **Alea algorithms by Johannes Baagøe** — same base algorithm (with `mash()`), but different wrapper. **They are not bit-exact compatible with each other** because the instance returns the same numeric stream for the same seed, but the API changes (`aleaPRNG.seed('x').random()` vs `Alea('x')()`), and the wrappers can initialize the internal state differently.

### 7.2 Careful: the RNG stream is seeded twice

Exact temporal order inside `generate()` (main pipeline, `main.js:650-680`):

1. `setSeed()` (line ~660): `Math.random = aleaPRNG(seed)` (with the vendored `public/libs/alea.min.js` version).
2. `applyGraphSize()` + `randomizeOptions()`: **consumes the PRNG** `aleaPRNG`. This is the first generative use of the pipeline. It calls `gauss(...)` several times (which calls `randomNormal.source(() => Math.random())`, i.e. uses `aleaPRNG`).
3. `shouldRegenerateGrid`, `generateGrid`: **resets the PRNG** with `Math.random = Alea(seed)` (npm version). Here the stream of the seed starts over from the beginning.
4. From this point onward — `getJitteredGrid`, `HeightmapGenerator.generate`, `Features.markupGrid`, `Rivers.generate`, etc. — everything consumes `Alea(seed)` (npm).

**Consequence**: to reproduce bit-exact, both alea versions must be ported. The `setSeed → generateGrid` stretch consumes `aleaPRNG`, not npm's `Alea`. What gets stored in Voronia's World Data Model includes results generated in both stretches (e.g. options like `eraInverseOption`, `statesNumber` and a bunch of randomized settings). Reproducing the full stream means: `aleaPRNG(seed)` for the randomized options stretch, and from `generateGrid` cut the race and start over with `Alea(seed)` (same seed, different lib).

### 7.3 RNG helpers (`src/utils/probabilityUtils.ts`)

All consume `Math.random` (the monkey-patched one):

- `rand(min, max)` → integer in `[min, max]`: `Math.floor(Math.random() * (max - min + 1)) + min`. Without args: raw `Math.random()`.
- `P(prob)` → bool: `Math.random() < prob` (with `>=1 → true`, `<=0 → false`).
- `gauss(exp, dev, min, max, round)` → uses `d3.randomNormal.source(() => Math.random())(exp, dev)` clamped to `[min, max]` and rounded to `round` decimals.
- `Pint(float)` → `~~float + P(float % 1)` (truncates + 1 with the probability of the decimal).
- `ra(arr)` → `arr[Math.floor(Math.random() * arr.length)]`.
- `rw(obj)` → array of keys (with value as weight) → random selection.
- `biased(min, max, ex)` → `Math.round(min + (max-min) * Math.random() ** ex)`.
- `getNumberInRange("a-b")` → range parsing with `rand(a, b)`.
- `generateSeed()` → `String(Math.floor(Math.random() * 1e9))`.

### 7.4 The PRNG wrapper in legacy code

There is none. The "wrapper" is `Math.random = <fn seedable>`. That is why the temporal order of steps 2-3 above is so critical.

**For Voronia (Phase 1)**:
- Decide: do we reproduce the `setSeed → generateGrid` stretch (i.e. the `aleaPRNG` consumption done by `randomizeOptions`)? If we only export "full options" (i.e. compute the options at runtime in the user's Azgaar, serialize them, and import them into Voronia), then **only `Alea@1.0.1`** + geometry need to be ported. If we want to reproduce the pipeline from the pure seed, we must also port `aleaPRNG 1.1.0` (the vendored version) and replicate `randomizeOptions`'s consumption order.
- Estimate: porting both is ~200 lines of Rust + byte-exact tests against the output of the JS versions.

## 8. Repacking grid→pack — exact algorithm

Source: `public/main.js:reGraph` (1157-1209), still **legacy JS**.

### 8.1 Purpose

The "grid" (what Azgaar's `pack` expects) has ~N×cellsDesired points (one per jittered cell). The "pack" discards unnecessary cells (deep ocean, non-coastal lakes) and regenerates density only on coastlines. This lowers the number of cells processed by the rest of the pipeline (cultures, states, routes, military) without losing detail on land.

### 8.2 Algorithm

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

### 8.3 Key points (bit-exactness for Voronia)

1. **It does not consume RNG**. It is purely deterministic as a function of the already computed `grid`.
2. **Iterates in the order of `gridCells.i`**, which is `[0, 1, 2, ..., pointsN-1]` exactly as `calculateVoronoi` populates it (`createTypedArray(...).map((_, i) => i)`). Therefore the "pack id" assigned when a point is added to `newCells.p` matches the push index, and `pack.cells.g` maps to the original grid id.
3. **Cell types (`gridCells.t`)**:
   - `-2` = lake (non-coastal if `i % 4 !== 0`).
   - `-1` = coastal water (near land).
   - `1` = coastal land (near water).
   - other = interior land / deep ocean.
4. **Discards**:
   - `(height < 20 AND type NOT IN {-1, -2})` = deep ocean, drop.
   - `(type === -2 AND (i % 4 === 0 OR features[f].type === 'lake'))` = **non-coastal** lakes, drop (one out of every 4 ids; `i%4` determines the density of surviving lakes; if the feature is lake, discard).
5. **Extra coastal points** (`type ∈ {1, -1}`):
   - Only if the cell is not near-border (`!gridCells.b[i]`).
   - Only for neighbors of the same type (`gridCells.t[e] === type`).
   - Only if `i > e` (avoids dup — it only adds when it first sees the lower id).
   - Only if the distance to the neighbor >= `spacing` (dist2 < spacing2 → skip).
   - Position = midpoint (`rn((x+ex)/2, 1)`, rounded to 1 decimal).
6. **After the repack**: `calculateVoronoi(newCells.p, grid.boundary)` recomputes Delaunay + Voronoi with the new points. Here too, the circumcenters are truncated to integer (see §6.3). The resulting `pack.cells.i` is the range `[0, pack.pointsN-1]`, where `pack.cells.i[k] === k`.
7. **`pack.cells.area`** = `Math.abs(d3.polygonArea(getPackPolygon(cellId)))`, capped at `UINT16_MAX`. `getPackPolygon` builds the cell's polygon using the `pack.cells.v`. This already involves floating point precision — but `Math.abs` and `Math.min` are stable.
8. **Other call sites of `reGraph` in the codebase**: `services/io/load.ts:414` (when loading a map), `controllers/heightmap-editor.ts:501,628` (when editing the heightmap). The repack happens **after any change to the grid**.

### 8.4 Grid and pack memory slot

- `grid.cells.*` are TypedArrays indexed by **grid id** (`i ∈ [0, grid.pointsN-1]`).
- `pack.cells.*` are TypedArrays indexed by **pack id** (`i ∈ [0, pack.pointsN-1]`), different from the grid id.
- `pack.cells.g[packId]` holds the original grid id (pack→grid mapping).
- Therefore **the attributes Azgaar serializes in the `.map` ARE associated with the pack id**, NOT the grid id. This is what makes bit-exact reproduction of the repack crucial: two engines that diverge in how many cells survive or in the order of `newCells.p` will have a different `pack.cells.g[k]` mapping, and the .map attributes end up misapplied.

## 9. Canonical pipeline (from `docs/domain/generation_pipeline.md` + `main.js`)

The canonical routine is `generate()` in `public/main.js` (lines ~650-680). 16 phases, in order:

1. **Seed & sizing** — `setSeed`, `applyGraphSize`, `randomizeOptions` → seed, dimensions, randomized options (first RNG consumption: `aleaPRNG`).
2. **Grid + heightmap** — `shouldRegenerateGrid`, `generateGrid` (re-seeds with npm `Alea`), `HeightmapGenerator.generate` → `grid.cells.h`.
3. **Base hydrology (grid)** — `Features.markupGrid`, `addLakesInDeepDepressions`, `openNearSeaLakes` → grid lake/ocean topology.
4. **World position & climate** — `OceanLayers`, `defineMapSize`, `calculateMapCoordinates`, `calculateTemperatures`, `generatePrecipitation` → `mapCoordinates`, `cells.temp`, `cells.prec`.
5. **Repack** — `reGraph`, `Features.markupPack`, `Measurers.createDefaultRuler` → `pack.cells.*` and default ruler.
6. **Rivers & biomes** — `Rivers.generate`, `Biomes.define`, `Features.defineGroups` → `pack.rivers`, `cells.biome`, feature groups.
7. **Ice** — `Ice.generate` → ice layer.
8. **Goods catalog** — `Goods.generate` → `pack.goods` (idempotent, called once).
9. **Cell ranking & cultures** — `rankCells`, `Cultures.generate`, `Cultures.expand` → `cells.s`, `cells.pop`, `pack.cultures`.
10. **Settlements & politics** — `Burgs.generate`, `States.generate`, `Routes.generate`, `Religions.generate` → `pack.burgs`, `pack.states`, `pack.routes`, `pack.religions`.
11. **Political specification** — `Burgs.specify`, `States.collectStatistics`, `States.defineStateForms` → burg types, stats, state forms.
12. **Provinces** — `Provinces.generate`, `Provinces.getPoles` → `pack.provinces`.
13. **Names (polish)** — `Rivers.specify`, `Lakes.defineNames` → river/lake names.
14. **Economy** — `Markets.generate`, `Production.produce`, `States.collectTaxes` → `pack.markets`, `cells.market`, `pack.deals`, `burg.production`, treasuries.
15. **Military & overlays** — `Military.generate`, `Markers.generate`, `Zones.generate` → regiments, markers, zones.
16. **Finalise** — `drawScaleBar`, `Names.getMapName`, `showStatistics` → scale bar, name, stats.

Other places that replicate sub-pipelines:
- `heightmap-editor.js:regenerateErasedData` repeats phases 3→15.
- `heightmap-editor.js:restoreRiskedData` repeats phases 3→7 + remapping of preserved entities.
- `src/generators/resample.ts:Resampler.process` repeats phases 3→7 + cell-data remap + regen-economy.

## 10. `.map` parser/serializer (binary-text format)

Primary source: `src/services/io/save.ts:prepareMapData` (44-187) and `src/services/io/load.ts:parseLoadedResult/parseLoadedData` (167-197, 400+).

### 10.1 Format

The `.map` file is an **array of strings joined by `\r\n`**. Some elements are serialized JSON, others are `.toString()` (`.join(",")`) of TypedArrays. The version sits in slot `[0]` as a pipe-delimited field. The indexed structure:

| Slot | Content | Encoding |
|---|---|---|
| `[0]` | `VERSION\|license\|date\|seed\|graphWidth\|graphHeight\|mapId` | pipe-delimited |
| `[1]` | settings joined by `\|` (distanceUnit, distanceScale, options, mapName, ...) | pipe-delimited |
| `[2]` | `mapCoordinates` | JSON |
| `[3]` | biomes (color\|habitability\|name) | pipe-delimited |
| `[4]` | notes | JSON |
| `[5]` | serialized SVG | XML string |
| `[6]` | `gridGeneral` (spacing, cellsX, cellsY, boundary, points, features, cellsDesired) | **JSON — only grid blob** |
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
| `[23]` | deprecated `pack.cells.road` (empty) | — |
| `[24]` | `pack.cells.s` | Uint16 csv |
| `[25]` | `pack.cells.state` | Uint16 csv |
| `[26]` | `pack.cells.religion` | Uint16 / JSON |
| `[27]` | `pack.cells.province` | Uint16 / JSON |
| `[28]` | deprecated `pack.cells.crossroad` (empty) | — |
| `[29]` | `pack.religions` | JSON |
| `[30]` | `pack.provinces` | JSON |
| `[31]` | `namesData` (name\|min\|max\|d\|m\|b) | `/`-delimited |
| `[32]` | `pack.rivers` | JSON |
| `[33]` | deprecated rulers (empty) | — |
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

### 10.2 Observations for the port

1. Slots `[40]`..`[46]` are **recent additions** (v1.124+, goods/markets/production/taxes). Older rows may have versions without them; `auto-update.ts` re-expands the missing fields with defaults.
2. **There is NO serialized "pack object"** — the pack is distributed across multiple slots. The "pack object" is reconstructed by `parseLoadedData` by calling `calculateVoronoi` + `reGraph` on the `grid` slot `[6]` and reapplying slot-by-slot.
3. **The `gridGeneral` slot `[6]` is the only grid blob**. However, it only stores `points, spacing, cellsX, cellsY, boundary, features` — **it does not store `cells.h`, `cells.t`, `cells.f`, `cells.i`, `cells.v`, `cells.c`, etc.** Those are reconstructed:
   - `grid.cells` is reconstructed by calling `calculateVoronoi(grid.points, grid.boundary)` → recomputes `cells.i/v/c/b` from the points. (See §11 below.)
   - `grid.cells.h/t/f/prec/temp` are reapplied from slots `[7]`, `[10]`, `[9]`, `[8]`, `[11]` by grid id.
   - `pack.cells` is reconstructed from scratch by calling `reGraph` with the already reconstructed grid. The pack attributes are reapplied from slots `[16]`..`[29]`, `[40]`, `[44]` by pack id.
4. **Slots `[13]` (cultures), `[14]` (states), `[15]` (burgs), `[29]` (religions), `[30]` (provinces), `[32]` (rivers), `[35]` (markers), `[38]` (zones), `[39]` (ice), `[41]` (goods), `[42]` (markets), `[43]` (deals), `[46]` (measurers)** are whole JSONs (entities, not cell-arrays).
5. **Empty/migrated slots**: `[23]`, `[28]`, `[33]` are deprecated; the parser ignores their content.
6. **CRLF mandatory**: the parser detects `\r\n` as delimiter. There is a `scripts/repair-map-line-endings.py` that fixes .map files with broken CRLFs (it always trusts that slot `[6]` starts with `{"spacing"`). Encoding handling to keep in mind in Rust: use `String::from_utf8` over the content, split by `"\r\n"`.
7. **OPTIONAL compression**: if `parseLoadedResult` fails to detect `|` between slot `[0]` fields, it attempts `uncompress` (gzip via `DecompressionStream`). That is, `.map` files may or may not be gzipped. Detector: try first without compression, and if it does not parse, try gzip.

### 10.3 JSON export (alternative format)

`src/services/io/export-json.ts:exportToJson` with 4 payload variants:

- **Full**: everything — cells, vertices, entities, settings. It is a more readable dump, but it is not what Azgaar loads back; it is for inspection / interoperability.
- **Minimal**: settings + entities (without cell-level attributes).
- **PackCells**: only `pack.cells.*` and the pack vertices.
- **GridCells**: only `grid.cells.*` and the grid vertices.

The "full JSON export" that master plan §23 asks to dissect is the **Full** mode. It will be inspected with a real exported test map (see §11).

## 11. Pending items of this phase

- [x] §12 Dissection of a real test `.map`.
- [x] §13 Empirical validation of the §3 finding.

## 12. Dissection of a real `.map` (Brample)

File: `/home/hans/Descargas/Brample 2026-07-22-21-24.map` (~11.7 MB, `XD.map` is already out of scope at the user's request).

### 12.1 Header (slot `[0]`)

```
1.138.0|File can be loaded in azgaar.github.io/Fantasy-Map-Generator|2026-7-22|861039636|2000|2000|1784767061245
```

Parse:
- `version` = `1.138.0` (matches repo main).
- `license` = `"File can be loaded in azgaar.github.io/Fantasy-Map-Generator"`.
- `date` = `2026-7-22` (year-month-day format without zero-padding).
- `seed` = `861039636` (integer as string — Azgaar treats it as a string for `Alea(seed)`; it uses `generateSeed()` which produces `String(Math.floor(Math.random()*1e9))`, so it is always 1–10 numeric digits).
- `graphWidth` = `2000`, `graphHeight` = `2000` (2000×2000 canvas).
- `mapId` = `1784767061245` (Date.now() timestamp at creation time).

### 12.2 Settings (slot `[1]`)

Begins:
```
km|1|square|m|2|°C|||||||1000|1||||||{"pinNotes":false,"winds":[225,45,225,315,135,315],"t...
```

Parse complementary to the `load.ts:254-295` code:
- `[0] distanceUnit="km"`
- `[1] distanceScale=1`
- `[2] areaUnit="square"`
- `[3] heightUnit="m"`
- `[4] heightExponent=2`
- `[5] temperatureScale="°C"`
- `[6]–[11]` empty (old scales, week, etc., now live in style).
- `[12] populationRate=1000`
- `[13] urbanization=1`
- `[14]–[18]` empty (old settings migrated to `options`, slot [19]).
- `[19]` = full `options` JSON (`{"pinNotes":false,"winds":[225,45,225,315,135,315],...}`).
- `[20] mapName`
- `[21] hideLabels`
- `[22] stylePreset`
- `[23] rescaleLabels`
- `[24] urbanDensity`
- `[25]` = longitude decimal (legacy).
- `[26] growthRate` setting.

### 12.3 Mapping of all confirmed slots

I reconstructed the 47 expected slots in the Brample file. The §10.1 table above **is confirmed** by a real case. Some relevant points of the file:

| Slot | Size (chars) | Content (Brample) |
|---|---|---|
| `[0]` | 112 | pipe-delimited header |
| `[1]` | 1627 | settings (with options embedded as JSON in `[19]`) |
| `[2]` | 65 | `mapCoordinates` JSON (`{"latT":180,"latN":90,"latS":-90,...}`) |
| `[3]` | 309 | biomes: `colors,habitability,fields` pipe-delimited (12 default biomes). |
| `[4]` | 44954 | notes (JSON of regiments/markers legend). |
| `[5]` | 3,344,024 (~3.2 MB) | **serialized SVG** (the whole visual DOM, ~28% of the file). |
| `[6]` | 169,634 (~170 KB) | **`gridGeneral` JSON** — the only grid blob seen as `{spacing,cellsX,cellsY,boundary,points,features,cellsDesired}`. Confirmed: **no `cells.*` here.** |
| `[7]` | 25,756 | `grid.cells.h` (Uint8 csv). Pattern `0,0,0,0,...,1,2,2,1,1,1,...,3,6,7,6,11,8,9,9,...` |
| `[8]` | 20,645 | `grid.cells.prec` (csv). |
| `[9]` | 20,044 | `grid.cells.f` (Uint16 csv, almost all `1` — 1 single feature). |
| `[10]` | 23,725 | `grid.cells.t` (Int8 csv, values 0/-1/-2/...). |
| `[11]` | 30,280 | `grid.cells.temp` (Int8 csv `-27,-27,...`). |
| `[12]` | 12,027 | `pack.features` JSON `[0,{i:1,type:"ocean",land:false,...}]`. The `[0]` is the reserved "null"/placeholder. |
| `[13]` | 2,258 | `pack.cultures` JSON `[{name:"Wildlands",i:0,...},...]`. |
| `[14]` | 57,438 | `pack.states` JSON. |
| `[15]` | 3,286,398 (~3.1 MB) | `pack.burgs` JSON `[{},{"cell":1133,"x":1468.66,"y":567.28,"state":1,"name":"Tal",...}]`. Another 28% of the file. |
| `[16]` | 11,441 | `pack.cells.biome` (Uint8 csv). |
| `[17]` | 12,299 | `pack.cells.burg` (Uint16 csv). |
| `[18]` | 11,454 | `pack.cells.conf` (csv). |
| `[19]` | 11,441 | `pack.cells.culture` (Uint16 csv). |
| `[20]` | 12,299 | `pack.cells.fl` (Uint16 csv). |
| `[21]` | 33,089 | `pack.cells.pop` (Float32 csv `rn(p,4)` — `0,0,...,53.9622,28.4531,50.2828,...`). |
| `[22]` | 11,921 | `pack.cells.r` (Uint16 csv). |
| `[23]` | 0 (empty) | deprecated `road`. |
| `[24]` | 13,008 | `pack.cells.s` (Uint16 csv). |
| `[25]` | 13,856 | `pack.cells.state` (Uint16 csv). |
| `[26]` | 11,594 | `pack.cells.religion` (Uint16 csv). |
| `[27]` | 17,146 | `pack.cells.province` (Uint16 csv). |
| `[28]` | 0 (empty) | deprecated `crossroad`. |
| `[29]` | 3,733 | `pack.religions` JSON. |
| `[30]` | 35,207 | `pack.provinces` JSON. |
| `[31]` | 884 | `namesData` `German|5|12|lt|0|/English|6|11|...`. |
| `[32]` | 22,600 | `pack.rivers` JSON. |
| `[33]` | 0 (empty) | deprecated `rulers`. |
| `[34]` | 299 | `fonts` JSON (`[{family:"Georgia"},{family:"Underdog",src:"url(...)",...}]`). |
| `[35]` | 4,953 | `pack.markers` JSON (`[{icon:"🌋",type:"volcanoes",dx:52,px:13,x:...,y:...,cell:...,i:0},...]`). |
| `[36]` | 74,398 | `pack.cells.routes` JSON (`{"6":{"7":359,"39":359},"7":{...}}` — adjacency map). |
| `[37]` | 103,538 | `pack.routes` JSON (`[{i:0,group:"roads",feature:2,points:[[758.56,351.83,325],...]}]`). |
| `[38]` | 2 (`[]`) | `pack.zones` (empty in this map). |
| `[39]` | 1,162 | `pack.ice` JSON. |
| `[40]` | 12,290 | `pack.cells.good` (Uint16 csv). |
| `[41]` | 17,591 | `pack.goods` JSON `[{i:1,name:"Wood",tags:["construction","fuel"],icon:"good-wood",color:"#966F33",value:1,...}]`. |
| `[42]` | 64,236 | `pack.markets` JSON. |
| `[43]` | 4,100,313 (~3.9 MB!) | `pack.deals` JSON (`[{i:0,seller:18,sellerType:"market",buyer:332,buyerType:"burg",good:21,units:1,price...}]`). **Largest individual slot of the file** (33% of the total). |
| `[44]` | 14,675 | `pack.cells.market` (Uint16 csv). |
| `[45]` | — (I did not capture the size, should be the HTML outer of custom icons) | `customGoodIcons`. |
| `[46]` | — | `pack.measurers` JSON. |

**Σ size**: ~11.7 MB, mostly split between SVG (3.2 MB), burgs (3.1 MB) and deals (3.9 MB) — features the most deal-heavy.

### 12.4 View of slot `[6]` (gridGeneral) — literal confirmation

```json
{"spacing":20,"cellsX":100,"cellsY":100,"boundary":[[1,-20],[1,2020],[42,-20],[42,2020],[82,-20],[82,2020],...],"points":[[10.12,10.34],[30.88,10.56],...],"features":[...],"cellsDesired":10000}
```

- `spacing` = 20 → spacing between points (in canvas units; 2000×2000 canvas, cellsX/Y=100 → expected 100×100 = 10000 points).
- `cellsDesired` = 10000 — the user asked for "10k cells" in the UI (it is the default).
- `points` is the `[x,y]` array with jitter (e.g. `[10.12, 10.34]`, `rn(x + jitter(), 2)`).
- **`boundary`** is the virtual border outside the canvas for edge cells; it consists of `[x, -20]` and `[x, 2020]` pairs (and similar for the y axis).
- Important: `cells` is not in the JSON. Confirmed in §13 below.

### 12.5 Validation of the parsing algorithm

The real parser (`load.ts:178-186`) replaces `\r\n` with `\n` inside the `<svg id="map" ...</svg>` block before doing the split. The split is always by `\r\n`, **not** by a loose `\n`.

⚠️ **Edge case not covered by Azgaar's parser**: files saved with a loose `\n` (like Brample, which was normalized by the OS or the browser download) **CANNOT be loaded by the current main version of Azgaar**. Its parser splits by `\r\n` and since there are none, only 1 slot remains; `JSON.parse(data[6])` blows up trying to parse the whole file. The gzip fallback tries to decompress, also fails. For Voronia: the parser must handle both separators (\r\n or \n with SVG-rescuing) — do not rely on the file preserving CRLF.

## 13. Empirical validation of the §3 finding (JSON does not store geometry)

**Confirmed**: the master plan §3 hypothesis is correct, strengthened on two points.

### 13.1 Slot `[6]` does NOT persist the mesh

The slot `[6]` JSON (Brample) contains exactly:

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

It does NOT contain `cells`, `vertices`, nor `seed` (the latter is in the header slot `[0]`, field `params[3]`). `cells.i/v/c/b/h/t/f/prec/temp` are all derived or stored in subsequent slots `[7]-[11]`.

### 13.2 The `load.ts` code reconstructs geometry from `points`

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

That is: the parser **does not read `cells.v/c/i` from the JSON** — it **reconstructs** them by invoking `calculateVoronoi(grid.points, grid.boundary)` (which in turn runs `Delaunator.from(...)` + `new Voronoi(...)` and populates `cells.i = 0..pointsN-1`).

### 13.3 The full pack (cells) is also reconstructed

Immediately after `parseLoadedData`, the loader executes:
- `reGraph()` over the already reconstructed grid → recomputes `pack.cells.i/v/c/b/p/g/h/area` from scratch.

And the pack attributes (biome, burg, culture, state, religion, province, pop, fl, r, s, conf, haven, harbor, good, market, routes) — slots `[16]`, `[17]`, `[18]`, `[19]`, `[20]`, `[21]`, `[22]`, `[24]`, `[25]`, `[26]`, `[27]`, `[40]`, `[44]` — are reapplied indexed by the reconstructed pack id.

### 13.4 Reinforced consequences (vs master plan §3)

1. **Plan §3 already said this**: the JSON does not store geometry; you must reproduce bit-exact `placePoints → getJitteredGrid → Delaunator.from → Voronoi → reGraph`. Confirmed in-vivo.
2. **New consequence**: the insertion order of the points in `points` must also be reproduced (`getJitteredGrid` iterates row-major, x inside y), because `Delaunator.from` uses the point index as `pointId`, and the reconstructed `cells.i` are `0..pointsN-1` in that order. If Voronia iterates in another direction, the ids are permuted.
3. **New consequence**: the pack is reconstructed by calling `reGraph()` with the reconstructed grid. If Voronia has any bug in reGraph (e.g. omits the `i > e` check or the `i % 4 === 0` check of the lake step), the `pack.cells.g[k] = gridId` mapping comes out different, and then all the pack attributes (biome, state, burg, ...) that arrive indexed by `k` get applied to the wrong cell. **Silent bug with no runtime error.**
4. **Confirmed**: the Prample file still comes with settings/`options` serialized in slot `[1]`, which includes the result of `randomizeOptions()` (where the first PRNG consumption happens with `aleaPRNG`). If Voronia only wants to load already generated maps, **it does not need to port `aleaPRNG`/`randomizeOptions`** — it can import the serialized options from slot `[1]`. This trims Phase 1 substantially compared to what §7.4 suggested.
5. **Confirmed**: the dimensions (`graphWidth`, `graphHeight`) come in the header slot `[0]`, fields `[4]` and `[5]`. The seed is in the header slot `[0]` field `[3]`. This is the first thing Voronia parses to reconstruct `placePoints` → deterministic chain.

### 13.5 Conclusion for Phase 1 (plan §23)

For the shortest path "import a `.map` and show it in the viewer" (Phase 1+2):

- **Port**:
  1. `alea@1.0.1` (npm version) → `Math.random` patchable in Rust (at least a seedable `dyn Rng` that produces floats with the same Baagøe algebra).
  2. `getJitteredGrid` + `getBoundaryPoints` + `placePoints` (`src/utils/graphUtils.ts:17-98`) — the grid geometry.
  3. `delaunator@5.0.1` (Mapbox). Validate bit-exact (the Rust `delaunator` crate is a port of the same lib, it should match).
  4. `Voronoi` class, **including the `Math.floor` trap in `circumcenter`** (`src/generators/voronoi.ts:142-154`).
  5. `reGraph()`, **including the `i % 4 === 0` for discarding non-coastal lakes** and the 20 half-edge cap in `edgesAroundPoint` (`public/main.js:1157-1209`).
- **NO need to port** (for import only):
  1. `aleaPRNG 1.1.0` (a.k.a. `public/libs/alea.min.js`).
  2. `randomizeOptions()` and `applyGraphSize()` (the setSeed → generateGrid stretch of the pipeline). Only the already serialized options are imported.
  3. Any procedural generator (heightmap, rivers, biomes, cultures, ...) — these produce the attributes that are already serialized in the `.map`.
- **In the future (Phase 7)**: if Voronia wants to generate maps from scratch with its own seed (not just import), only then must everything else be ported, including `aleaPRNG` + `randomizeOptions`.
