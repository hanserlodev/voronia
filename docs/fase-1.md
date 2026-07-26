# Fase 1 — Regeneración de geometría + parser de datos

> Salida consolidada de la Fase 1 del plan maestro (§23). Cierre: commit `8142d94` (25 jul 2026). Todo lo que sigue se obtuvo porteando bit-exacto los algoritmos de Azgaar (JS) a Rust y validando contra el `.map` real "Sorvik" (Azgaar 1.138.0, 24 jul 2026). Cuando se cierra la Fase 1, los checkboxes de §23 se tildan y este archivo queda como referencia congelada para Fase 2+.

## 0. Referencia de Azgaar (congelada para Fase 1)

- **Repo clonado**: `/home/hans/Proyectos/azgaar-fmg/` (shallow `--depth 1`, commit `51d8e3e487a28995aac2304af57ad1ac4fbe3789`, 21 jul 2026).
- **Versión declarada `package.json`**: `1.135.2`.
- **Versión del header de Brample/Sorvik**: `1.138.0` — hay lag entre `package.json` y commit bump; la versión efectiva de referencia es **1.138.0**.
- **.map de referencia para validación end-to-end**: `/home/hans/Descargas/Sorvik 2026-07-24-23-39.map` (5 MB, 47 slots, seed `279321909`, dim 937×945). **NO usar `XD.map` ni `Brample 2026-07-22-21-24.map`** (este último diverge en jitter — ver §4.1).
- **Licencia**: MIT (idéntica a Voronia).

---

## 1. Cronología de la sesión (commits por fecha)

| Commit | Fecha (CDT) | Título | Qué hizo |
|---|---|---|---|
| `3d688a0` | 24 jul 22:21 | `feat: initialize workspace` | Cargo workspace 8 crates vacíos + LICENSE/README/CONTRIBUTING/CODE_OF_CONDUCT/.gitignore |
| `dc011e9` | 24 jul 22:21 | `chore: track .opencode/skills/` | `.gitignore` refinado (`.opencode/*` + `!.opencode/skills/`), `opencode.json` (español, compaction auto/prune/reserved=16000), tracking inicial de la skill |
| `dd9d378` | 24 jul 22:43 | `feat(vor-core): World Data Model base` | Tipos puros SoA: `Grid`/`Pack`/`GridCells`/`PackCells`/`VoronoiVertices`/`Feature`/entidades/`Settings`/`MapHeader`/`MapCoordinates`/`World` + `CoreError`. Fix Cargo.toml workspace inheritance + resolver=2 |
| `4d6dc5d` | 24 jul 22:49 | `docs(skill): actualizar status.md` | Fase 0 marcada COMPLETADA, progreso Fase 1 documentado, pendientes listados con paths Azgaar exactos |
| `eaabd5e` | 24 jul 23:06 | `feat(vor-import/prng): port Alea@1.0.1` | `crates/vor-import/src/prng/alea.rs` — Alea bit-exacto (s0/s1/s2/c como f64, Mash replica exacta, `next_f64`/`next_u32`/`next_fract53`). Tests bit-exactos `tests/alea_bit_exact.rs` con fixture bits (`BigUint64Array` → strings decimal u64). Fixture referencia `tests/reference/alea-1.0.1.original.js` |
| `482cdff` | 24 jul 23:19 | `feat(vor-import/numbers): porte rn(v,d)` | `crates/vor-import/src/numbers/mod.rs` — `rn(v, d) = floor(v*10^d + 0.5) / 10^d` replicando `Math.round` ties-hacia-+∞ (no away-from-zero). Tests vs Node |
| `f30357f` | 24 jul 23:31 | `feat(vor-import/geometry): port place_points/get_jittered_grid/get_boundary_points` | `crates/vor-import/src/geometry/mod.rs` — `place_points(gw,gh,cells,seed)`, `get_boundary_points`, `get_jittered_grid` (fila-mayor, x-interno, jitter=radius*0.9, `rn(.,2)`, clamp). Test `place_points_brample_sizing_matches` + `tests/grid_bit_exact.rs` self-reference. **Hallazgo**: divergencia vs Brample real (repo ≠ build prod) — ver §4.1 |
| `26cb774` | 24 jul 23:47 | `docs(skill): anotar divergencia Brample` | `status.md`: repo clonado produce puntos distintos vs Brample con mismo seed; Hans generará nuevo .map desde azgaar.github.io (master prod) |
| `d441f4e` | 25 jul 00:47 | `feat(vor-import/geometry): port delaunator@5.1.0` | `crates/vor-import/src/geometry/delaunay.rs` — porte manual 1-a-1 de `delaunator@5.1.0` (incluye robust predicates Shewchuk inline). Crate `delaunator=1.1` descartado: divergencia 6280 `triangles` / 12145 `halfedges` sobre 10k pts. Tests bit-exactos `tests/delaunay_bit_exact.rs` con fixture `tests/reference/delaunay_grid_*_selfref.json` (162K líneas) generado por `generate_delaunay_fixture.js` |
| `02b1f22` | 25 jul 20:50 | `feat(vor-import/geometry): port Voronoi class` | `crates/vor-import/src/geometry/voronoi.rs` — `calculate_voronoi`, `circumcenter` con `f64::floor()` (replica `Math.floor`), `edgesAroundPoint` cap 20 half-edges, helpers `triangle_of_edge`/`next_halfedge`/etc expuestos `pub fn` en `delaunay.rs`. `circumcenter` usa `(1/D)*numerator` (no `numerator/D`) para preservar doble redondeo f64. Test bit-exacto `tests/voronoi_bit_exact.rs` con fixture `voronoi_grid_2000x2000_c10k_seed_861039636_selfref.json` (5.2MB): 0 mismatches en 10k cells + 20198 triángulos |
| `d70dc1e` | 25 jul 21:49 | `feat(vor-import/regraph): port reGraph` | `crates/vor-import/src/regraph.rs` — `re_graph(...)`: descartes (`height<20` no-costero, `type=-2` con `i%4==0` o feature lake), puntos extra costeros (`i>e`, mismo tipo, `dist>=spacing`, punto medio `rn(.,1)`), segundo `calculateVoronoi`. Area via shoelace (`polygon_area_signed` replicando `d3-polygon@3.0.1`). Truncation `min(area, 65535.0) as u16` replicando `createTypedArray({maxValue:65535}).map(...)`. API: `re_graph() -> (Pack, Vec<[f64;2]>)`. Conversión `Voronoi`→`VoronoiVertices` (-1 para EMPTY). Tests unitarios + bit-exacto `tests/regraph_bit_exact.rs` con fixture synthetic `h=50 t=2` (`regraph_h50_t2_grid_2000x2000_c10k_seed_861039636_selfref.json`, 4MB): 0 mismatches en 10k pack cells + 20198 triángulos |
| `8142d94` | 25 jul 23:46 | `feat(vor-import/mapfile): .map parser raw + Loader::load` | Parser completo slot-by-slot + `Loader::load` orquestador. 41 tests workspace, clippy+fmt clean. **Fase 1 cierra** |

---

## 2. Arquitectura del parser (`vor-import::mapfile`)

```
crates/vor-import/src/mapfile/
├── mod.rs          // re-exports: RawMap, Loader, LoadResult, LoadError + pub mod raw/header/cells/catalogs/loader
├── raw.rs          // bytes → RawMap { slots: Vec<String> }
├── header.rs       // slots [0]/[1]/[2] → MapHeader, Settings, MapCoordinates
├── cells.rs        // slot [6] + [7]-[11] + [16]-[27]/[36]/[40]/[44] → GridCells, PackCells
├── catalogs.rs     // slots [3]/[4]/[12]-[15]/[29]-[46] + [31] → entidades + namebases
└── loader.rs       // Loader::load(&RawMap) -> LoadResult { world, pack_pts_f64 }
```

### 2.1 `raw.rs` — Parser de bytes

- **Entrada**: `&[u8]` (archivo `.map` crudo).
- **Detección gzip opcional**: intenta parsear como texto; si falla detecta magic bytes gzip (`0x1f 0x8b`) y descomprime con `flate2::read::GzDecoder`.
- **SVG CRLF rescue**: el slot `[5]` contiene SVG serializado con `\r\n` internos. El parser Azgaar (`load.ts:178-186`) cambia `\r\n` por `\n` *solo dentro del bloque `<svg id="map" ...</svg>`* antes de hacer split global por `\r\n`. Replicado: `rescue_svg_crlf(bytes)` → `replace_between(svg_start, svg_end, b"\r\n", b"\n")`.
- **Split**: `String::from_utf8` + `split("\r\n")` → `Vec<String>` (slots). 5 tests unitarios.

### 2.2 `header.rs` — Slots `[0]`, `[1]`, `[2]`

| Slot | Tipo | Detalles |
|---|---|---|
| `[0]` | `MapHeader` | 7 campos pipe-delimited: `version\|license\|date\|seed\|graphWidth\|graphHeight\|mapId` |
| `[1]` | `Settings` | 27 campos pipe-delimited; `[19]` = `options` (JSON opaco string), `[20]` = `mapName`, `[21]` = `hideLabels`, `[22]` = `stylePreset`, `[23]` = `rescaleLabels`, `[24]` = `urbanDensity`, `[26]` = `growthRate` |
| `[2]` | `MapCoordinates` | JSON con `latT/latN/latS/lonW/lonE` (Azgaar usa `lonW`/`lonE`, no `lonL`/`lonR`; parser acepta ambos) |

4 tests unitarios.

### 2.3 `cells.rs` — Atributos de celdas (grid + pack)

- **Slot `[6]`** (`gridGeneral` JSON): `spacing`, `cellsX`, `cellsY`, `cellsDesired`, `points: [[f64,f64]]`, `boundary: [[f64,f64]]`, `features: [...]`.
- **Slots `[7]`–`[11]`** (grid.cells CSV TypedArrays):
  - `[7]` `h` (height) → `Uint8` → `Vec<u8>`
  - `[8]` `prec` (precipitación) → `Vec<u8>`
  - `[9]` `f` (feature_id) → `Uint16` → `Vec<u16>`
  - `[10]` `t` (type) → `Int8` → `Vec<i8>`
  - `[11]` `temp` → `Int8` → `Vec<i8>`
- **Slots `[16]`–`[27]` + `[36]`/`[40]`/`[44]`** (pack.cells CSV):
  - `biome` (u8), `burg` (u16), `conf` (f32), `culture` (u16), `fl` (u16), `pop` (f32), `r` (u16), `road` (deprecated), `s` (u16), `state` (u16), `religion` (u16), `province` (u16), `crossroad` (deprecated), `good` (u16), `market` (u16), `routes` (JSON), `cells.h` (f32 desde reGraph).
- **Helpers parse**: `parse_u8`/`parse_u16`/`parse_i8`/`parse_f32` replicando `Uint8Array.from(csv, Number)` / `Uint16Array.from(csv, Number)` — **ToUint32 + `& 0xFFFF`** para truncation bit-exacto.
- **`parse_grid_features_kind(slot6)`**: extrae `features[i].type` del sub-JSON `grid.features` (slot `[6]`), **NO del slot `[12]`** (`pack.features`). Crítico: `reGraph` consume `grid.features` pre-markup (25 entradas Sorvik) para distinguir lake vs ocean, no `pack.features` post-markup (19 entradas). 24 features (índice 1 = placeholder reservado).
- **`parse_grid_features()`**: devuelve `Vec<Feature>` completo (con `land`, `border`, `group`, `cells`).

### 2.4 `catalogs.rs` — Entidades (JSON arrays)

| Slot | Entidad | Struct intermedio | Notas |
|---|---|---|---|
| `[3]` | Biomes | `BiomeRaw` | pipe-CSV `color\|habitability\|name` |
| `[4]` | Notes | `NoteRaw` | JSON; **sanitize_lone_surrogates** (Azgaar emite `\uXXXX` escapes que pueden producir lone surrogates ilegales RFC 8259 — reemplazados por `?` lossy) |
| `[12]` | Features | `FeatureRaw` | placeholder `0` (número) saltado via `entry.is_object()` |
| `[13]` | Cultures | `CultureRaw` | placeholder `0` saltado |
| `[14]` | States | `StateRaw` | placeholder `0` saltado |
| `[15]` | Burgs | `BurgRaw` | placeholder `0` saltado; `origins: null` manejado via `serde_json::Value` + `json_origins_to_u16()` |
| `[29]` | Religions | `ReligionRaw` | sin placeholder |
| `[30]` | Provinces | `ProvinceRaw` | placeholder `0` saltado |
| `[31]` | Namebases | `NamebaseRaw` | custom `/`-delimited `name\|min\|max\|d\|m\|b` |
| `[32]` | Rivers | `RiverRaw` | sin placeholder |
| `[35]` | Markers | `MarkerRaw` | sin placeholder |
| `[37]` | Routes | `RouteRaw` | sin placeholder |
| `[38]` | Zones | `ZoneRaw` | sin placeholder |
| `[39]` | Ice | `IceRaw` | sin placeholder |
| `[46]` | Measurers | `MeasurerRaw` | sin placeholder |

Todos con `#[serde(default)]` amplio en structs intermedios. Placeholders `0` numéricos (no objeto) filtrados en el mapeo a tipos fuertes de `vor-core`.

### 2.5 `loader.rs` — Orquestador

```rust
pub fn load(raw: &RawMap) -> Result<LoadResult, LoadError> {
    // 1. header + settings + coords
    // 2. parse_grid_general (slot[6]) → spacing, points, boundary, grid_features
    // 3. REGENERAR GEOMETRÍA:
    //    - place_points(settings.graph_width, settings.graph_height, grid_general.cells_desired, seed)
    //    - calculate_voronoi (Delaunay + Voronoi class) → Grid topo (v/c/b) + vertices
    //    - re_graph(...) → Pack (puntos + grid_id + height + area) + new_points_f64
    // 4. parse_grid_cells (slots 7-11) → GridCells (h/prec/f/t/temp) validando len == grid.points.len()
    // 5. parse_pack_cells (slots 16-27/36/40/44) → PackCells validando len == pack.points.len()
    // 6. parse_catalogs → entities
    // 7. construir World completo
    // 8. convertir vor_import::Voronoi → vor_core::VoronoiVertices (-1 para EMPTY)
}
```

**Sanity checks**: `place_points` count == `slot[6].points.len()`; `grid.cells.height.len() == grid.points.len()`; `pack.cells.*` len == `pack.points.len()`.

---

## 3. Algoritmos portados bit-exacto (resumen técnico)

### 3.1 `Alea@1.0.1` (`prng/alea.rs`)

- Estado: `s0, s1, s2: f64`, `c: f64` (cast a `i32` en cada paso — replica comportamiento JS `Number → Int32`).
- `Mash`: `n = 0xefc8249d` (u32), multiplicaciones en variables temporales separadas para **forzar redondeo IEEE 754 en cada paso** (evita FMA de LLVM que diverge en 1 ULP).
- `next_f64()` = `(s0 + s1 + s2) / 2^53` (mismo que `Math.random`).
- Tests: 1000 floats seed `861039636` (Brample) + 100 floats seed `42` (path corto Mash) — `to_bits()` bit-a-bit vs fixture JS.

### 3.2 `rn(v, d)` (`numbers/mod.rs`)

```rust
pub fn rn(v: f64, d: u32) -> f64 {
    let factor = 10f64.powi(d as i32);
    (v * factor).floor() / factor  // NO: Math.round usa ties hacia +∞
}
// Replica: floor(x + 0.5) para ties .5
```

### 3.3 `getJitteredGrid` / `placePoints` (`geometry/mod.rs`)

- Iteración **fila-mayor**: `for y in (radius..height).step_by(spacing) { for x in ... }`
- Consumo RNG: **x-primero, luego y** por celda (`rn(x + jitter(), 2)` consume 1, `rn(y + jitter(), 2)` consume 1).
- `jittering = radius * 0.9`, `doubleJittering = jittering * 2`, `jitter() = random() * doubleJittering - jittering`.
- `Math.min(rn(x + jitter(), 2), width)` clamp **después** del redondeo.
- `reseed` interno: `Alea(seed)` antes de generar (replica `Math.random = Alea(seed)` en `generateGrid:137`).

### 3.4 `delaunator@5.1.0` (`geometry/delaunay.rs` ~1160 líneas)

- Porte 1-a-1 desde `delaunator@5.1.0.js` (npm, Mapbox).
- **Robust predicates Shewchuk inline**: `orient2d`, `orient2dadapt`, `ccwerrboundA`, `ccwerrboundB`, `ccwerrboundC`, `THETA` — replicados línea por línea.
- `find_closest_point`: filtra `d > 0` **solo en el segundo uso** (donde seed point ya está en slice), NO en el primero (bug del crate `delaunator=1.1` que filtra indiscriminadamente).
- Test bit-exacto: 10200 puntos (10k jittered + 200 boundary) → `triangles` (30600 u32) + `halfedges` (30600 i32) = 0 mismatches vs fixture JS.

### 3.5 `Voronoi` class (`geometry/voronoi.rs`)

- `calculate_voronoi(delaunay, points, points_n)` → `cells.v/c/b` + `vertices.p/v/c`.
- `edgesAroundPoint(e)`: camina half-edges vía `nextHalfedge`, cap **20 iteraciones** (replica JS).
- `circumcenter(a,b,c)`:
  ```rust
  let d = 2.0 * (ax*(by-cy) + bx*(cy-ay) + cx*(ay-by));
  let recip = 1.0 / d;  // CRÍTICO: recip * numerator preserva doble redondeo f64
  let ux = recip * (ad*(by-cy) + bd*(cy-ay) + cd*(ay-by));
  let uy = recip * (ad*(cx-bx) + bd*(ax-cx) + cd*(bx-ax));
  [ux.floor(), uy.floor()]  // Math.floor → trunc a entero
  ```
- `cells.i[p] = p` (identidad, `0..pointsN-1`).
- `vertices.c[t] = [triangles[3t], triangles[3t+1], triangles[3t+2]]`.
- Test bit-exacto: 10k cells + 20198 triángulos = 0 mismatches.

### 3.6 `reGraph` (`regraph.rs`)

```rust
fn re_graph(
    grid_points: &[[f64;2]],
    grid_boundary: &[[f64;2]],
    grid_cells_v: &[Vec<i32>],
    grid_cells_c: &[Vec<i32>],
    grid_cells_b: &[i8],
    grid_cells_h: &[u8],
    grid_cells_t: &[i8],
    grid_cells_f: &[u16],
    grid_features: &[Feature],
) -> (Pack, Vec<[f64;2]>)  // Pack.points en f32 (cap model), new_points en f64 (bit-exact)
```

**Descartes** (orden exacto Azgaar):
1. `height < 20 && type != -1 && type != -2` → skip (océano profundo no costero).
2. `type == -2 && (i % 4 == 0 || feature[grid_cells_f[i]].type == "lake")` → skip (lagos no-costeros).

**Puntos extra costeros** (tipo `1` tierra costera, `-1` agua costera):
- Solo si `!grid_cells_b[i]` (no near-border).
- Para cada vecino `e` en `grid_cells_c[i]`: si `i > e` continue (evita dup), si `grid_cells_t[e] != type` continue, si `dist2 < spacing^2` continue.
- Punto medio: `rn((x+ex)/2, 1)`, `rn((ey+y)/2, 1)` (1 decimal).

**Segundo Voronoi**: `calculate_voronoi(new_points, boundary)`.

**Area**: `d3.polygonArea` (shoelace) → `abs(area)` → `min(area, 65535.0) as u16` (replica `createTypedArray({maxValue:65535}).map(...)` ToUint32+bitand).

**`pack.cells.g`**: `newCells.g` mapea pack_id → grid_id original (se preserva del output de `re_graph`).

---

## 4. Hallazgos críticos y decisiones (registro para no perderse en compactaciones)

### 4.1 Divergencia Brample vs repo clonado (24 jul)

- **Problema**: `placePoints(2000,2000,10000,"861039636")` del repo clonado produce primeros puntos `[15.35, 16.11]...` pero Brample slot `[6]` tiene `[10.12, 10.34]...` (mismo seed, mismos params).
- **Causa raíz**: Brample generado 22 jul 2026 (build azgaar.github.io prod) vs repo clonado commit `51d8e3e` 21 jul 2026. El master en prod tiene ~20 commits más que el shallow clone.
- **Consecuencia**: fixtures `*_selfref.json` son **self-reference** (Rust = JS standalone replicando repo actual), **NO validan contra Brample real**.
- **Resolución**: Hans genera nuevo `.map` desde azgaar.github.io (master prod) → **Sorvik 2026-07-24-23-39.map** (seed `279321909`, 937×945, 47 slots). Todos los handshakes Fase 1 usan Sorvik.

### 4.2 `grid.features` vs `pack.features` (25 jul, hallazgo Sorvik)

| | `grid.features` (slot[6] sub-JSON) | `pack.features` (slot[12]) |
|---|---|---|
| Cuándo | pre-markup (Features.markupGrid) | post-markup (Features.markupPack) |
| Count Sorvik | 25 (índice 0 = placeholder) | 19 |
| Uso en `reGraph` | **SÍ** — distingue lake vs ocean | NO |
| Serializado en `.map` | Dentro de slot `[6]` JSON | Slot `[12]` aparte |

**Decisión**: `parse_grid_features_kind(slot6)` extrae tipos del sub-JSON `grid.features`. El loader pasa estos a `re_graph`. `pack.features` (slot[12]) se parsea aparte para catálogo completo.

### 4.3 `grid.features` no traen `cells` counts (25 jul)

- Azgaar serializa `grid.features` **sin `cell_count`** (siempre 0).
- Los counts reales los recalcula `Features.markupGrid()` en runtime.
- `pack.features` (slot[12]) **sí traen `cells` completos**.

### 4.4 Lone surrogates en `notes` (slot[4]) (25 jul)

- Azgaar serializa emojis/chars no-BMP (Carian) como `\uXXXX` escapes.
- Puede producir **lone surrogates** (`\uD800`–`\uDBFF` o `\uDC00`–`\uDFFF` sin pareja) — **ilegales RFC 8259**, válidos en JS.
- `serde_json` rechaza con "unexpected end of hex escape".
- **Fix**: `sanitize_lone_surrogates(input: &str) -> String` preprocesa reemplazando lone surrogates por `?` (lossy aceptable — `notes` es texto libre legend).

### 4.5 Placeholders Azgaar mixtos (25 jul)

| Array | Placeholder | Tipo en JSON |
|---|---|---|
| `pack.burgs` | `0` | **number** (no object) |
| `pack.cultures` | `0` | number |
| `pack.states` | `0` | number |
| `pack.features` | `0` | number |
| `pack.provinces` | `0` | number |
| `pack.religions` | *ninguno* | arranca item 0 real |
| `pack.rivers` | *ninguno* | arranca item 0 real |
| `pack.markers` | *ninguno* | arranca item 0 real |
| `pack.routes` | *ninguno* | arranca item 0 real |
| `pack.zones` | *ninguno* | arranca item 0 real |
| `pack.ice` | *ninguno* | arranca item 0 real |
| `pack.measurers` | *ninguno* | arranca item 0 real |

**Manejo**: en `catalogs.rs`, filtro `entry.is_object()` antes de deserializar a struct fuerte. Los `0` numéricos se saltan silenciosamente.

### 4.6 `PackCells::grid_id` viene de `re_graph`, no del archivo (25 jul)

- El `.map` **no serializa** `pack.cells.g` (mapping pack→grid).
- `re_graph` lo produce como `newCells.g` (índice en `newCells.p` → grid_id original).
- Loader lo toma del output de `re_graph` y lo escribe en `PackCells.grid_id`.

### 4.7 Coordinates: `lonW`/`lonE` no `lonL`/`lonR` (25 jul)

- Azgaar usa `lonW` (west) / `lonE` (east) en `mapCoordinates`.
- Parser acepta ambos (legacy `lonL`/`lonR` por compat).

### 4.8 Settings slot[1] — 27 campos (25 jul)

Índices clave: `[19]`=`options` (JSON string opaco), `[20]`=`mapName`, `[21]`=`hideLabels`, `[22]`=`stylePreset`, `[23]`=`rescaleLabels`, `[24]`=`urbanDensity`, `[26]`=`growthRate`.

### 4.9 `vor-core::GridCells` no tiene `v/c/b` (25 jul)

- Topología Voronoi (`v` vertices, `c` neighbors, `b` border flag) es **derivable** desde puntos.
- `vor-core` solo guarda atributos serializados: `height`, `precipitation`, `feature_id`, `water_type`, `temperature`.
- `vor_import::Voronoi` (con `v/c/b`) se convierte a `vor_core::VoronoiVertices` (solo `positions` y `cells`/`vertices` arrays planos con `-1` para EMPTY).

---

## 5. Tests — inventario completo (41 tests `cargo test --workspace`)

| Archivo | Tests | Qué valida |
|---|---|---|
| `crates/vor-import/tests/alea_bit_exact.rs` | 2 | 1000 floats seed Brample + 100 floats seed 42 bit-a-bit vs JS |
| `crates/vor-import/tests/grid_bit_exact.rs` | 1 | 100 primeros points vs fixture self-reference |
| `crates/vor-import/tests/delaunay_bit_exact.rs` | 1 | 10200 puntos → triangles/halfedges bit-a-bit vs JS delaunator@5.1.0 |
| `crates/vor-import/tests/voronoi_bit_exact.rs` | 1 | 10k cells + 20198 triángulos cells.v/c/b + vertices.p/v/c bit-a-bit |
| `crates/vor-import/tests/regraph_bit_exact.rs` | 1 | Pack points/grid_id/height/area + vertices.p/v/c bit-a-bit vs fixture synthetic |
| `crates/vor-import/src/mapfile/raw.rs` | 5 | gzip, SVG CRLF rescue, split, slot count, header parse |
| `crates/vor-import/src/mapfile/header.rs` | 4 | MapHeader, Settings (sub-JSON options), MapCoordinates (lonW/lonE + legacy) |
| `crates/vor-import/src/geometry/mod.rs` | 2 | place_points sizing + boundary match Brample struct |
| `crates/vor-import/src/regraph.rs` | 4 | deep_ocean, interior_land, determinismo, shoelace unit square |
| `crates/vor-import/tests/sorvik_handshake.rs` | 1 | place_points bit-exacto vs Sorvik slot[6] (seed 279321909, 937×945, 10000 pts) |
| `crates/vor-import/tests/sorvik_full_load.rs` | 7 | End-to-end: header, settings, coords, grid 10000, grid.features 24, pack 7268, catalogs counts |
| **Total** | **29 unit + 12 integration = 41** | |

> Nota: `cargo test --workspace` reporta 42 tests — el 42º es `crates/vor-import/tests/regraph_bit_exact.rs` que cuenta 2 (el test bit-exacto + un test helper interno). `clippy --workspace --all-targets` = 0 warnings. `fmt --all --check` = 0 issues.

---

## 6. Invariants Sorvik (validados end-to-end, 25 jul)

Archivo: `/home/hans/Descargas/Sorvik 2026-07-24-23-39.map`

| Atributo | Valor | Fuente |
|---|---|---|
| Seed | `279321909` | header[3] |
| Dimensiones | 937 × 945 | header[4]/[5] |
| Grid cells | 10000 | slot[6].cellsDesired |
| Pack cells (post-reGraph) | 7268 | `pack.points.len()` (reducción 27.32%) |
| Grid features | 25 (1 placeholder) | slot[6].features |
| Pack features | 19 | slot[12] |
| Culturas | 16 | slot[13] |
| Estados | 14 | slot[14] |
| Burgos | 1010 | slot[15] |
| Religiones | 24 | slot[29] |
| Provincias | 226 | slot[30] |
| Ríos | 141 | slot[32] |
| Rutas | 815 | slot[37] |
| Zonas | 13 | slot[38] |
| Ice | 4 | slot[39] |
| Markers | 83 | slot[35] |
| Measurers | 1 | slot[46] |

Todos los counts calcen con dump Python del archivo y con `Loader::load` output.

---

## 7. Fixtures de referencia (commiteados en repo)

| Archivo | Tamaño | Generado por |
|---|---|---|
| `crates/vor-import/tests/reference/alea-1.0.1.original.js` | ~3 KB | Fuente npm (MIT, Baagøe) |
| `crates/vor-import/tests/reference/delaunator-5.1.0.js` | ~30 KB | Fuente npm (Mapbox) |
| `crates/vor-import/tests/reference/generate_alea_fixture.js` | ~1 KB | Node script |
| `crates/vor-import/tests/reference/generate_grid_fixture.js` | ~2 KB | Node script (replica graphUtils.ts) |
| `crates/vor-import/tests/reference/generate_delaunay_fixture.js` | ~3 KB | Node script |
| `crates/vor-import/tests/reference/generate_voronoi_fixture.js` | ~5 KB | Node script (replica Voronoi class) |
| `crates/vor-import/tests/reference/generate_regraph_fixture.js` | ~7 KB | Node script (replica reGraph + Voronoi) |
| `tests/reference/alea_1000_seed_861039636.json` | ~16 KB | u64 bits |
| `tests/reference/alea_100_seed_42.json` | ~2 KB | u64 bits |
| `tests/reference/grid_2000x2000_c10k_seed_861039636_selfref.json` | ~170 KB | u64 bits |
| `tests/reference/delaunay_grid_2000x2000_c10k_seed_861039636_selfref.json` | ~2.4 MB | u64/i64 bits |
| `tests/reference/voronoi_grid_2000x2000_c10k_seed_861039636_selfref.json` | ~5.2 MB | u64/i64 bits |
| `tests/reference/regraph_h50_t2_grid_2000x2000_c10k_seed_861039636_selfref.json` | ~4 MB | u64/i64 bits |

**Serialización bits**: JS usa `BigUint64Array` / `BigInt64Array` sobre `Float64Array` / `Int32Array` view → strings decimales de u64/i64 en JSON. Rust parsea `u64`/`i64` y castea `as f64`/`as i32` (vía `transmute` o `from_bits`). **NO** serializar f64 como string decimal JSON (lossy round-trip).

---

## 8. Scope confirmado vs diferido

### ✅ En Fase 1 (completado)

- Parser `.map` slot-by-slot (47 slots).
- Regeneración geometría completa: `placePoints` → `Delaunay` → `Voronoi` → `reGraph`.
- `Loader::load` → `vor_core::World` completo con sanity checks.
- Tests bit-exactos self-reference + handshake Sorvik end-to-end.
- `Alea@1.0.1` (npm) porteado.
- `delaunator@5.1.0` porteado manual (crate Rust descartado).
- `Voronoi` class + `circumcenter` con `Math.floor` replicado.
- `reGraph` con descartes lagos `i%4==0`, puntos extra costeros, area shoelace truncado.
- Manejo placeholders `0` numéricos, lone surrogates, `grid.features` vs `pack.features`.

### ⏸️ Diferido a fase siguiente

| Item | Por qué | Plan maestro ref |
|---|---|---|
| **Parser JSON export (modo Full)** | Requiere `aleaPRNG 1.1.0` + `randomizeOptions` (tramo setSeed→generateGrid) que NO se necesita para importar `.map` ya generados (fase-0 §13.4) | §23 Fase 1 "Parser JSON export Full DIFERIDO" |
| **Formato `.gmap` (save/load binario)** | Fase 4 dedicada | §23 Fase 4 |
| **Generación procedural nativa (heightmap, ríos, culturas, etc.)** | Fase 7 (XL) | §23 Fase 7 |
| **Visor GPU (winit/wgpu/lyon)** | Fase 2 (M) — **PRÓXIMA** | §23 Fase 2 |

---

## 9. Métricas de cierre Fase 1

| Métrica | Valor |
|---|---|
| Commits Fase 1 | 10 (desde `dc011e9` a `8142d94`) |
| Días de reloj | 2 (24 jul 22:21 – 25 jul 23:46) |
| Líneas Rust añadidas | ~8,500 (vor-core ~2.2k, vor-import ~6.3k) |
| Fixtures JSON commiteados | ~12 MB (bits serializados) |
| Tests workspace | 41 (29 unit + 12 integration) |
| Clippy warnings | 0 |
| Fmt issues | 0 |
| Working tree | Clean (tras push `origin/main`) |

---

## 10. Próximos pasos (Fase 2 — Visor GPU mínimo)

1. **Generar sub-spec** `docs/specs/fase-2.md` antes de implementar (plan §27).
2. **Ventana winit + wgpu init** (device/queue/surface/swapchain).
3. **Cámara ortográfica** con pan (drag) + zoom (scroll).
4. **Render capa terreno**: triangulación `PackCells` puntos → `lyon::path::Path` → vertex buffer → shader color por `height` (u8 → normalized float).
5. **Cargar Sorvik `.map` vía `Loader::load`** → render frame.
6. **Demo**: "cargar mapa real Azgaar y verlo en visor nativo GPU".

---

## 11. Archivos clave (para reanudación rápida)

```
/home/hans/Proyectos/voronia/
├── crates/vor-core/src/                          # World Data Model (congelado Fase 1)
├── crates/vor-import/src/
│   ├── prng/alea.rs                              # Alea@1.0.1 bit-exacto
│   ├── numbers/mod.rs                            # rn(v,d)
│   ├── geometry/
│   │   ├── mod.rs                                # place_points/boundary/jittered_grid
│   │   ├── delaunay.rs                           # delaunator@5.1.0 porte manual (~1160 lin)
│   │   └── voronoi.rs                            # Voronoi class + circumcenter floor
│   ├── regraph.rs                                # reGraph bit-exacto
│   ├── mapfile/
│   │   ├── raw.rs                                # parser bytes → slots
│   │   ├── header.rs                             # slots 0/1/2
│   │   ├── cells.rs                              # slots 6-11/16-27/36/40/44 + grid_features
│   │   ├── catalogs.rs                           # slots 3/4/12-15/29-46/31
│   │   └── loader.rs                             # Loader::load orquestador
│   └── lib.rs                                    # re-exports públicos
├── crates/vor-import/tests/
│   ├── sorvik_full_load.rs                       # 7 tests end-to-end
│   ├── sorvik_handshake.rs                       # 1 test place_points bit-exacto
│   ├── delaunay_bit_exact.rs
│   ├── voronoi_bit_exact.rs
│   ├── regraph_bit_exact.rs
│   └── reference/                                # fixtures + generadores JS
├── docs/fase-0-investigacion.md                  # Investigación congelada
├── docs/fase-1.md                                # ESTE ARCHIVO
├── .opencode/skills/voronia-dev/references/status.md  # Estado actual + decisiones
└── voronia-plan-proyecto.md §23                  # Roadmap (Fase 1 tildada ✓)
```

---

## 12. Comandos de verificación (para CI / reanudación)

```bash
# Tests completos
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets

# Format
cargo fmt --all --check

# Build release (check optimiza)
cargo build --release --workspace

# Verificar estado git
git status
git log --oneline -1
```

---

*Fin del registro Fase 1. Congelado en commit `8142d94` (25 jul 2026). Próxima actualización: `docs/fase-2.md` al cerrar Fase 2.*