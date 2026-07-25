# Voronia — estado actual

> Este archivo se actualiza en cada sesión de trabajo donde pase algo relevante (ver protocolo de mantenimiento en `SKILL.md`). Mantenelo corto — es para orientarse rápido al empezar una sesión, no para llevar el historial completo (eso vive en `git log` y en el plan maestro).

**Última actualización**: 25 julio 2026 (porte bit-exacto de `delaunator@5.1.0` en `vor-import`)

## Fase actual del roadmap

**Fase 1 — Regeneración de geometría + parser de datos**: en marcha.

**Fase 0 — Investigación y sentado de bases**: ✓ COMPLETADA. Documentación consolidada en `docs/fase-0-investigacion.md` (PRNG exacto, Delaunay/Voronoi con trampa de `Math.floor`, repacking grid→pack, parser `.map` slot-by-slot, validación empírica contra el archivo real "Brample"). Workspace de Cargo inicializado y los tipos base del World Data Model ya están commiteados.

## Progreso de la Fase 1 (plan maestro §23)

- [x] Clonar/revisar Azgaar para identificar algoritmos de geom + PRNG (hecho en Fase 0 — ver `docs/fase-0-investigacion.md`).
- [x] Diseccionar `.map` real ("Brample") — estructura de 47 slots confirmada (fase-0 §12.3).
- [x] **Setup Cargo workspace + crates vacíos** (commit `3d688a0`); fix `Cargo.toml` con `\n` literal roto + workspace inheritance (commit `dd9d378`).
- [x] **Trackear la skill en el repo** (commit `dc011e9`) — `.gitignore` con `.opencode/*` + `!.opencode/skills/` (resolución del "PENDIENTE DE CONFIRMACIÓN DE HANS" que vivía abajo).
- [x] **`vor-core` con tipos base del World Data Model** (commit `dd9d378`) — `Grid`/`Pack`/`GridCells`/`PackCells`/`VoronoiVertices`/`Feature`/entidades/`Settings`/`MapHeader`/`MapCoordinates`/`World` + `error::CoreError`. SoA estricto, enums fuertes, `serde_json::Value` opaco para subsistemas que no se modelan en Fase 1 (economía/milicia → Fase 7). `cargo check` + `clippy` + `fmt` limpios.
- [x] **Portear `Alea@1.0.1`** (npm, Johannes Baagøe) en `vor-import` (commit `eaabd5e`) — módulo `prng::alea`. 1100 floats validados bit-a-bit Rust↔JS. Fixtures como bits (vía `BigUint64Array`).
- [x] **Helpers numéricos de Azgaar** — `rn(v, decimals)` (commit `482cdff`) con `js_math_round = floor(x + 0.5)` para replicar ties-hacia-+∞ de `Math.round` JS.
- [x] **`getBoundaryPoints` + `getJitteredGrid`/`placePoints`** (`graphUtils.ts:17-98`) (commit `f30357f`) — fila-mayor, x-interno, jitter via `Alea` re-seedeada, `rn(.,2)`, `Math.min` clamp. Test bit-exacto contra fixture self-reference.
- [x] **Validar `delaunator` bit-exacto** contra `delaunator@5.1.0` (npm) — descartado el crate `delaunator = "1.1"` (Rust) por divergencia en casos degenerate (6280 entradas en `triangles` sobre los 10000 puntos jittered). Porte manual bit-exacto desde `delaunator-5.1.0.js` en `crates/vor-import/src/geometry/delaunay.rs` (incluye los robust predicates de Shewchuk inline). Test bit-exacto en `tests/delaunay_bit_exact.rs` con fixture self-reference.
- [ ] **Portear `Voronoi` class** (`voronoi.ts`) — `cells.v/c/b/i`, `vertices.p/v/c`, `edgesAroundPoint` con cap 20 half-edges, helpers `nextHalfedge`/`triangleOfEdge`/`edgesOfTriangle`. CRÍTICO: `circumcenter` con `Math.floor` truncado a entero (f64 interno → i32).
- [ ] **Portear `reGraph`** (`main.js:1157-1209`) — descartes (height<20 no-costero, type=-2 con `i%4==0` o feature lake), puntos extra costeros (`i>e`, mismo tipo, `dist>=spacing`, punto medio `rn(.,1)`), segundo `calculateVoronoi` sobre `newCells.p`, `pack.cells.g/h/area`.
- [ ] **Parser del `.map`** (slot-by-slot, `\r\n` o `\n` SVG-rescued, gzip opcional) — header `[0]`, settings `[1]`, gridGeneral `[6]` (JSON), grid.cells `[7]`-`[11]`, pack.cells `[16]`-`[44]`, entidades JSON `[12]`-`[46]`. Cubrir auto-update de slots deprecated `[23]/[28]/[33]`.
- [ ] **Poblar `World`** desde el parser — mapear slots del `.map` a structs de `vor-core`. Tipos fuertes según `references/architecture.md`.
- [ ] **BLOQUEADO**: Validación empírica contra `.map` Brample real — divergencia de versión de Azgaar (Brample 1.138.0 vs azgaar-fmg master). Hans generará un nuevo `.map` de referencia desde azgaar.github.io actual y lo dejará en `~/Descargas/`.
- [ ] **Test end-to-end**: cargar `.map` de referencia → `World` → verificar counts (10k grid cells, pack count, burgs/states/cultures) vs dump del archivo. Usa el nuevo `.map` que Hans generará.

**Scope de Fase 1 confirmado con Hans (24 jul 2026)**: **solo parser `.map`**. JSON export Full DIFERIDO a fase siguiente (aprovechando el hallazgo fase-0 §13.4: si solo se importan mapas ya generados, NO hace falta portear `aleaPRNG`/`randomizeOptions` — las options serializadas en slot `[1]` se importan como opaco).

## Decisiones tomadas (fuera de las que ya están en el plan maestro)

- **Nombre del proyecto**: Voronia (24 jul 2026, tras descartar `Worldforge`/`Terraforge` por colisión con proyectos reales — detalle en plan maestro §1.3).
- **Configuración de OpenCode** (24 jul 2026): `opencode.json` con `compaction.auto: true`, `compaction.prune: true`, `compaction.reserved: 16000`, `instructions` español por defecto + skill voronia-dev. Compactación automática porque GLM-5.2 vía NVIDIA tiene ventana nominal ~1M pero límite práctico ≈170K; umbral operativo 160K (ver `SKILL.md` "Límite de contexto y checkpoint de sesión").
- **Regla crítica de checkpoint de contexto** (24 jul 2026): al acercarse a 160K tokens el agente detiene generación, escribe `agent_state_checkpoint.md` en la raíz, avisa a Hans con mensaje literal (ver `SKILL.md`), dispara compactación. `agent_state_checkpoint.md` cubierto por `.gitignore` como ruta temporal no-commiteable.
- **Protocolo de reanudación** (24 jul 2026): al decir "continuar con el trabajo que se dejó / seguir donde lo dejamos / continuar con la Fase X", el agente lee `references/status.md` + `agent_state_checkpoint.md` si existe, **reconstruye el `todowrite`** ítem por ítem (mismo texto, orden, estado y prioridad), verifica con `git status` los archivos pendientes de commitear, confirma con Hans el punto exacto en 1-2 líneas, y ejecuta el próximo paso sugerido. Tras reanudar, borra el `agent_state_checkpoint.md`. Detalle en `SKILL.md` (subsección "Protocolo de reanudación").
- **Tracking de skill en repo** (24 jul 2026, commit `dc011e9`): `.gitignore` refina `.opencode/` como `.opencode/*` + `!.opencode/skills/` para trackear la skill (protocolo `SKILL.md` punto 4) sin trackear caches/node_modules/tools (estos últimos también cubiertos por `.opencode/.gitignore` interno con `node_modules`/`package.json`/`package-lock.json`/`bun.lock`). Resolución del "PENDIENTE DE CONFIRMACIÓN DE HANS" que vivía en ediciones anteriores de este archivo.
- **Parser `.map` primero, JSON export Full diferido** (24 jul 2026, decisión de Hans): Fase 1 scopea solo el parser `.map` (slot-by-slot). JSON export Full entra en una fase próxima.
- **`serde_json::Value` opaco para economía/milicia** (24 jul 2026): slots `[40]`-`[44]` (goods/markets/deals) y el subtree `military` dentro de `State` se preservan como JSON opaco en `vor-core`, no se modelan como tipos fuertes todavía — propio de Fase 7 (plan §10). Re-export sin pérdida hasta entonces.
- **Enums con `Default` + `#[default]`** (24 jul 2026): para que structs deriven `Default` felizmente y `#[serde(default)]` funcione en campos enum. Se eligió la variant "neutral/placeholder" como default en cada caso (`GovernmentForm::Anarchy`, `CultureType::Generic`, `ReligionType::Folk`, `ReligionExpansion::Culture`, `RouteGroup::Roads`, `IceKind::Glacier`, `FeatureType::Ocean`).
- **Porte manual de `delaunator@5.1.0` en vez del crate `delaunator` (Rust)** (25 jul 2026): el crate `delaunator = "1.1"` de crates.io NO es bit-exacto contra el JS `delaunator@5.1.0` (npm) que Azgaar usa según `azgaar-fmg/package-lock.json:1599`. Causa raíz: el crate `robust = "1.2"` reimplementa Shewchuk de forma distinta (signo del `orient2dadapt` no negado, constante `THETA` vs `ccwerrboundA`) y además `delaunator-rs` tiene un bug en `find_closest_point` (filtra `d > 0` indiscriminadamente en ambos usos). Resultado: divergencia de 6280 entradas en `triangles` y 12145 en `halfedges` sobre los 10000 puntos jittered `placePoints(2000,2000,10000,"861039636")`. Decisión: porte manual bit-exacto en `crates/vor-import/src/geometry/delaunay.rs`, replicando el fuente JS 1-a-1 (incluyendo los robust predicates inline de Shewchuk). Esto sigue el patrón del porte de `Alea@1.0.1` y garantiza bit-exactitud (hallazgo fase-0 §13.4 crítico para que atributos del `.map` queden en celdas correctas).

## Bloqueos / cosas pendientes de confirmar

- **`Cargo.lock` sigue ignorado** (decisión heredada del commit inicial — ver nota adjunta en `.gitignore`). Para un workspace con binarios normalmente se commitea; sin decisión final todavía.
- Ver plan maestro §26 (decisiones pendientes) para el resto: `.gmap` vs `.mapg`, alcance de soporte a `.map` legacy más allá del parser, prioridad Fase 8, etc.
- **Divergencia verificada entre `azgaar-fmg` (repo clonado v1.138.0 según header de Brample) y el `.map` "Brample" real (generado el 22 jul 2026, un día posterior al último commit del clon `51d8e3e`)**: el algoritmo `placePoints`/`getJitteredGrid` del repo produce puntos divergentes contra el slot `[6]` de Brample con el mismo seed `861039636` y `cellsDesired=10000`. Confirmado bit-exacto Rust↔JS-standalone-replicando-el-bg-master-actual, lo que implica que el Brample fue generado con **una build de azgaar.github.io más nueva que el último commit del repo clonado**. Por eso:
  * Mi fixture `crates/vor-import/tests/reference/grid_2000x2000_c10k_seed_861039636_selfref.json` es **self-reference** — valida Rust contra el álgoritmo actual del repo, no contra el Brample.
  * Hans generará un nuevo `.map` de referencia desde azgaar.github.io (master actual en producción) y lo dejará en `~/Descargas/`. El item "Test end-to-end: cargar .map de referencia → World Data Model" usará ese nuevo archivo.
  * Las pruebas `Alea` bit-exactas vs JS quedan intactas (no se ven afectadas por la divergencia — eso prueba Rust=JS standalone).
  * Las pruebas `place_points` structural (spacing=20, cellsX=cellsY=100, 10000 pts, boundary `[1,-20],[1,2020],[42,-20]...`) calzan bit-exacto con Brample slot `[6]` — no es afectada, esas derivaciones no usan RNG.
- **Variants de enums a confirmar contra wiki de Azgaar** antes del cierre de Fase 1: `CultureType`, `GovernmentForm`, `ReligionType` (variant `Organized` — el string exacto puede ser "Organized" o "Organized Religion"), subgrupos `LandGroup`/`LakeGroup`. Ver campos marcados `// TODO Fase 1: confirmar variants exactas` en `crates/vor-core/src/entities/*`.

## Historia corta de ediciones de este archivo

La edición previa (configuración OpenCode, commit previo DC011E9) decía "Fase 0: no iniciada" y marcaba como pendiente confirmar el tracking de la skill. **Eso estaba desactualizado**: la Fase 0 estaba completa (residía en `docs/fase-0-investigacion.md`), y el workspace ya existía. Se corrige acá. Si en una sesión futura este archivo dijese algo incompatible con `git log` o `docs/fase-0-investigacion.md`, dudá del archivo y confía en el log + la investigación.
