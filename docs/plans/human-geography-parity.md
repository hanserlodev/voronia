# Human Geography & Economy — Plan de Paridad Total con FMG

> **Estado**: EN CURSO — iniciado 13 ago 2026
> **Objetivo (opción 1)**: **paridad completa de generación + datos + render** con Azgaar's Fantasy Map Generator para los layers de Human Geography (states, provinces, zones, cultures, religions, population, burgs, markets, trade) y Economy (goods, routes, deals).
> **Referencia**: Azgaar FMG `v1.135.2` en `/home/hans/Proyectos/azgaar-fmg` (checkout local). No se copia código TypeScript; se reimplementa la lógica en Rust/wgpu respetando la arquitectura de Voronia y la licencia MIT.
> **Fase de roadmap**: Fase 7 — Motor de generación procedural nativo.

---

## ⚠️ REGLA DE SUPERVIVENCIA TRAS COMPACTACIÓN

Este documento es la **fuente de verdad** del plan. En cada compactación de contexto:

1. **Releer este archivo completo** antes de tocar nada.
2. Reconstruir el `todowrite` a partir de la sección **"Checklist de avance"** (los checkboxes reflejan el estado real).
3. Si quedó trabajo a medias, `git status`/`git diff` lo confirman; continuar desde ahí.
4. Actualizar `references/status.md` de la skill y **marcar aquí los ítems completados** antes de cerrar sesión.

**No se pierde nada**: el plan vive en git (`docs/plans/human-geography-parity.md`).

---

## 0. Visión general

La paridad se logra en **dos niveles** por cada layer:

1. **Generación y datos**: producir los mismos resultados, dependencias, costes, relaciones y campos que FMG.
2. **Render**: reproducir geometría, z-order, colores, opacidades, estilos, iconos, etiquetas y animaciones.

### Dependencias críticas (orden de generación FMG)

```text
Geometry / Isolines (motor común)
        ↓
States / Cultures / Religions / Provinces / Zones
        ↓
Burgs
        ↓
Routes
        ↓
Goods + Production
        ↓
Markets
        ↓
Deals + Trade Animation
```

Dependencias extra:

```text
Burgs + Routes + Goods + Production → Markets
Markets + Routes + Deals → Trade animation
```

### Orden de generación exacto de FMG (pipeline)

```text
Goods.generate() → Burgs.generate() → States.generate() → Routes.generate()
→ Religions.generate() → Markets.generate() → Production.produce() → States.collectTaxes()
```

Fuente: `/home/hans/Proyectos/azgaar-fmg/docs/domain/generation_pipeline.md:5-33`.

---

## 1. Estado actual de Voronia

| Layer | Estado actual | Diferencia principal con FMG |
|---|---|---|
| States | Render básico por celda (`state_layer.rs`) | Falta `getIsolines`+waterGap+halo+labels y generación nativa |
| Provinces | Render básico por celda (`province_layer.rs`) | Falta expansión exacta, isolines, labels, generación nativa |
| Cultures | Render básico por celda (`culture_layer.rs`) | Falta generación y expansión exacta |
| Religions | Render básico por celda (`religion_layer.rs`) | Falta tipos, centros, expansión, isolines exactas |
| Population | Heatmap por celda (`population_layer.rs`) | FMG usa barras rurales y urbanas animadas |
| Burgs | Triángulo por burg (`burg.rs`) | FMG usa iconos SVG, puertos, labels, grupos |
| Zones | Fill básico (`zone_layer.rs`) | Falta comportamiento completo y estilo |
| Routes | ~70% (`route_layer.rs` + modelo `route.rs`) | Falta generación nativa y geometría visual exacta |
| Goods | Sin modelo/render (`world.goods` = `serde_json::Value`) | Todo por hacer |
| Markets | Sin modelo/render (`world.markets` = `serde_json::Value`) | Todo por hacer |
| Trade | Sin modelo/render (`world.deals` = `serde_json::Value`) | Todo por hacer |

### Archivos actuales de Voronia

- Datos: `crates/vor-core/src/world.rs` (+ `entities/{state,province,culture,religion,burg,route}.rs`)
- Flags de capas: `crates/vor-render/src/layers.rs`
- Capas humanas: `crates/vor-render/src/{state_layer,province_layer,culture_layer,religion_layer,population_layer,burg,zone_layer}.rs`
- Motor de isolines (parcial): `crates/vor-render/src/isoline.rs` (`connect_vertices`, `get_isolines`, `get_water_gap_path`, `get_halo_path`, `get_fill_path`)
- Routes: `crates/vor-core/src/entities/route.rs`, `crates/vor-render/src/route_layer.rs`
- Integración: `crates/vor-app/src/lib.rs`
- Plan previo: `docs/plans/full-rendering.md` (fases J/M/... para goods/markets/trade)

---

## 2. Qué hace FMG exactamente (referencia)

### 2.1 States

- **Fuentes**: `src/generators/states-generator.ts`; `public/modules/ui/layers.js:537-563`.
- **Generación**: selección de capitales → expansión `FlatQueue` (Dijkstra) con costes de cultura, población/score, bioma, altura, ríos y tipo cultural; respeta estados bloqueados; limita coste total por `growthRate`.
- **Post-proceso**: normalización de formas, vecinos por celdas adyacentes, colores greedy evitando colores de vecinos, estadísticas (área, población rural/urbana), polos de inaccesibilidad.
- **Render**: `getIsolines` + `waterGap` + halo opcional (`geometricPrecision`) + clip paths + labels estatales.
- **Labels**: raycasting desde el polo, ángulos cada 9°, longitud 5-300, `<textPath>` curvo, fallback a nombre corto.

### 2.2 Provinces

- **Fuentes**: `src/generators/provinces-generator.ts:70-199`; `public/modules/ui/layers.js:592-616`.
- **Generación**: burgs como capitales → expansión `FlatQueue` con coste principalmente de elevación; no cruza fronteras estatales; permite pasos acuáticos; segunda pasada de "justificación" suaviza formas.
- **Render**: isolines + fill + waterGap; label en `province.pole` o centro; color derivado del color estatal.

### 2.3 Cultures

- **Fuentes**: `src/generators/cultures-generator.ts`; `public/modules/ui/layers.js:480-494`.
- **Generación**: consume `culturesInput`, celdas pobladas (`cells.s`), crea centros, expansión con costes de terreno/bioma/altura.
- **Render**: isolines por `cells.culture` + fill por `cultures[index].color` + waterGap.

### 2.4 Religions

- **Fuentes**: `src/generators/religions-generator.ts`; `public/modules/ui/layers.js:509-522`.
- **Tipos**: `Folk`, `Organized`, `Cult`, `Heresy`; expansión `culture`, `state` o global.
- **Generación**: folk por cultura; organized con centros en burgs/celdas pobladas, separación con quadtree; expansión `FlatQueue` con costes de cultura, estado, bioma, rutas y agua; BFS para orígenes en radio limitado.
- **Render**: isolines + fill + waterGap (sin bordes).

### 2.5 Population

- **Fuente**: `public/modules/ui/layers.js:394-432`.
- **NO es heatmap** — son barras verticales:
  - **Rural**: una línea por celda con `cells.pop > 0`, altura `pop/5`, animación D3 2s (`easeSinIn`).
  - **Urbana**: una línea por burg (`!removed`), altura `(population/5)*urbanization`, delay 500ms.
- **Voronia**: capa de líneas (no TriangleList), separar rural/urban, escalado dependiente de zoom, animación.

### 2.6 Burgs

- **Fuentes**: `src/generators/burgs-generator.ts`; `src/renderers/draw-burg-icons.ts`; `src/renderers/draw-burg-labels.ts`; `src/renderers/draw-state-labels.ts`.
- **Generación**: capitales por score poblacional + separación quadtree; towns con score aleatorizado y separación mínima variable; asigna `cells.burg`, estado, cultura, provincia, puerto, grupo, producción, mercado.
- **Render**: iconos SVG `<use>` por grupo, anclas de puerto, labels con offsets `data-dx/data-dy`.

### 2.7 Zones

- **Datos**: `world.zones`, `zone.cells`, `zone.color`.
- **Render**: agrupar celdas por zona → isolines + fill + borde + labels según configuración. Reutiliza el motor común.

### 2.8 Routes

- **Fuentes**: `src/generators/routes-generator.ts:192-207,647-709`; `public/modules/ui/layers.js:845-869`.
- **Generación**: 1) `generateSeaRoutes()` (usa ríos navegables), 2) `generateMainRoads()`, 3) `generateTrails()`, 4) preparar puntos, 5-7) insertar por grupo; construye `cells.routes`.
- **Render**: agrupa por `group` (roads/trails/searoutes), paths de `Routes.getPath(route)` (curvas, meandros). Voronia dibuja segmentos rectos — falta paridad de curvas/grosor/dash/linecap/alpha.
- **Estado**: ~70% (modelo + import + líneas básicas + flag UI).

### 2.9 Goods

- **Fuentes**: `src/generators/goods-generator.ts`; `src/generators/production-generator.ts`; `src/renderers/draw-goods.ts`; `docs/domain/goods_schema.md`.
- **Tipos**: raw (`distribution`), manufactured (`recipes`), hybrid (ambos).
- **Generación**: restaura catálogo por defecto → inicializa `cells.good` → calcula máx recursos → mezcla celdas/goods → evalúa `good.distribution` por celda (`new Function`) → máximo un bonus-good por celda.
- **Render (3 subcapas)**:
  1. **goodsCells**: producción por celda (`Production.getCellProduction`), filtro visibles, máx global, opacity `0.1+0.9*normalize(total,0,maxTotal)`, un polígono por good producido.
  2. **goodsIcons**: `cells.good`, círculo opcional + `<use>` icono, tamaño configurable (`data-size`, default 6), stroke = `Goods.getStroke(color)`.
  3. **goodsBurgs**: top-3 goods de mayor valor por burg, placa con fondo `#f5f5f5`, icono, valor, padding `1/0.6`, rx `1`.

### 2.10 Markets

- **Fuentes**: `src/generators/markets-generator.ts`; `src/renderers/draw-markets.ts`.
- **Selección**: score por población, capital ×2.5, puerto ×1.2, ruido; separación quadtree.
- **Territorios**: `FlatQueue` con costes: base 10, cambio de estado 100, agua 50, agua sin puerto compatible 50, cambio de isla 100. Escribe `cells.market` y `burg.market`.
- **Precios**: raw por demanda/stock; manufactured = coste ingredientes + valor añadido.
- **Render**: isolines por `cells.market`, fill-opacity baja (`fill-opacity:0.03`), borde oscurecido (stroke-width 0.7), círculo en burg central, icono `⚖️` configurable, hover highlight (transición 1s).

### 2.11 Trade / Trade Animation

- **Fuentes**: `src/generators/markets-generator.ts:402-534`; `src/renderers/trade-animation.ts`; `src/renderers/draw-trade-animation.ts`.
- **Producción** (`Production.produce()`): rural → precios iniciales → índice recetas/demanda → worker loop burgs → venta inventario → `runGlobalTrade()` → compra para demanda.
- **Deals**: seller/buyer, tipo entidad, good, unidades, precio, impuesto.
- **Comercio global**: demanda consumidores+industria, población por market, coste transporte por distancia, clasifica exporters/importers, oportunidades rentables, ordena por beneficio, ejecuta deals, actualiza stock/precios. Precio final = exportador + transporte + impuesto estado exportador.
- **Animación**: batches, max 30 concurrentes, Dijkstra sobre `cells.routes` (agua 1, tierra 5, cambio 20), wagon.svg tierra / ship.svg agua, Catmull-Rom alpha(0.1), rotación según dirección, duración configurable, pausa por segmento, click para detalle.

---

## 3. Plan de implementación por fases

### ✅ Fase 0 — Congelar referencia y contrato
- [x] Decisión opción 1 tomada y persistida en este plan (13 ago 2026).
- [x] Dump de catálogos de Sorvik → `/tmp/voronia_hg_catalogs.json` (test `crates/vor-app/tests/hg_catalogs_dump.rs`). Contrato de datos fijado:
  - **13 states, 225 provinces, 15 cultures, 24 religions, 1009 burgs, 815 routes, 13 zones, 71 goods, 44 markets, 15492 deals, 7268 cells**.
  - Campos observados: `State{i?,name,color,...}`, `Burg{id,name,cell,position,culture,state,feature,population,kind,coat_of_arms,is_capital,port_feature,has_citadel,has_plaza,has_shanty,has_temple,has_walls,locked,removed}`, `Route{id,group,feature,points,length}`, `Zone{id,name,color,cells,style,description}`.
  - Goods/markets/deals ya se parsean como `serde_json::Value` (arrays JSON) en `vor-import` — el dato existe en el `.map`; falta modelado tipado en `vor-core`.
- [ ] Criterio de aceptación por layer (validar contra los dumps de arriba + snapshots SVG de FMG).

### ✅ Fase 1 — Motor común de regiones (isolines)
- [x] Confirmado que `isoline.rs` ya porta `connect_vertices`, `get_isolines`, `get_water_gap_path`, `get_halo_path` (13 ago 2026).
- [x] **Corregido `get_fill_path`**: FMG usa `M…L…Z` recto (`pathUtils.ts:getFillPath`), no Bézier — el fill de regiones es la frontera Voronoi cruda (el suavizado `build_curve_basis_closed` solo aplica a clima).
- [x] **Nuevo `build_region_mesh`** (exportado): agrupa celdas por tipo, tesela fills rectos con lyon, salta tipo 0. El water gap se delega a `water_gap::append_water_gap` (mismo patrón que biome).
- [x] **Cableado**: `state_layer.rs`, `province_layer.rs`, `culture_layer.rs`, `religion_layer.rs` ahora usan `build_region_mesh` + `append_water_gap(is_water)`; caller en `vor-app` simplificado (sin gaps duplicados).
- [x] Test `region_mesh_fill_matches_isolines` en `isoline.rs` (verde).
- [x] **Paridad numérica en Sorvik** (test `hg_catalogs_dump`): state 21 polígonos/13 tipos, province 231/225, culture 49/15, religion 36/16. FMG agrupa todos los polígonos conexos por tipo → los counts > tipos son islas/enclaves, esperado.
- [x] **Zones**: `build_vertex_path_mesh` (porta `getVertexPath`) — fill del contorno exterior por lista de celdas; `zone_layer.rs` lo usa (color ~35% alpha). Verificado: zones mesh 802v/2277i para 13 zonas.

### ✅ Fase 1 — COMPLETA (13 ago 2026)

### ⬜ Fase 2 — States y Provinces
- [x] **`vor-sim::states` creado** (`states.rs`): port de `createStates`+`expandStates`+`normalize`+`findNeighbors`+`assignColors`+`collectStatistics` con FlatQueue determinista (FMG usa `Math.random`; Voronia usa `Pcg64Mcg` con seed). Costes exactos: cultureCost (-9/100), populationCost (score), biomeCost (move_cost), heightCost (feature kind + h), riverCost (flux), typeCost (water_type). Validado en Sorvik: **14 states (placeholder + 13 capitales), 5212 celdas asignadas, colores hex, determinista por seed** (test `hg_catalogs_dump`).
- [x] **`pack.cells.water_type`** (cells.t) propagado en el loader (necesario para typeCost).
- [x] **`vor-sim::provinces` creado** (`provinces.rs`): port de `generate()` (provinces desde burgs del estado ordenados capital→población, expansión FlatQueue con costes de elevación 100/30/10/100, restringida al estado, justificación de formas). Validado en Sorvik: **310 provincias, 5107 celdas asignadas, colores hex**.
- [ ] Pendiente: generación de nombres de estado/provincia (namebase, Phase 7), poles (polylabel), COA, wild provinces, locks.
- [ ] `vor-render`: halo + borders (getBorder FMG) + labels estatales/provinciales.

### ⬜ Fase 3 — Cultures y Religions
- [x] **`vor-sim::cultures` creado** (`cultures.rs`): port de `generate()`+`expandCultures()` — centros en celdas pobladas separadas por spacing, `defineCultureType` (Nomadic/Highland/Lake/River/Hunting/Generic), expansionism por tipo, expansión FlatQueue con costes de biome/biome-change/height/river/type. Validado en Sorvik: **11 culturas, 5026 celdas asignadas**.
- [x] **`vor-sim::religions` creado** (`religions.rs`): port de `generate()`+`expandReligions()` — folk por cultura + organized/cult/heresy en celdas pobladas, expansión FlatQueue restringida por `ReligionExpansion` (culture/state/global). Añadida variante `ReligionExpansion::State` y campo `expansionism` al modelo `Religion` (vor-core) + import. Validado en Sorvik: **28 religiones, 3361 celdas asignadas**.
- [ ] Pendiente: nombres de religión (getDeityName, generateReligionName), passageCost con routes (biome), spreadFolkReligions exacto, locks.
- [x] Render: isolines + waterGap ya cableados (Fase 1).

### ⬜ Fase 4 — Population y Burgs
- [x] **`vor-render::population_layer` → barras** (`build_population_bars_mesh`): reemplaza el heatmap por **barras verticales** rural (una por celda, `pop/5`) + urbanas (una por burg, `(population/5)*urbanization`), colores FMG (`#4d4d4d` rural, `#d0240f` urban). Validado: 24140v/36210i.
- [x] **`vor-render::route_layer` mejorada**: Catmull-Rom (searoutes 8 subdiv, roads/trails 6). Validado: 55064v.
- [x] **`vor-render::burg` → iconos círculo** (`build_burg_icons_mesh`): círculo por burg coloreado por estado (FMG `#icon-circle`). Validado: 13117v/36324i.
- [ ] `vor-core::burg` completo: production/market (Fase 6/7).
- [ ] `vor-render::burg_labels` + `state_labels` (polo, rayos, textPath, fallback) + anclas de puerto.

### ⬜ Fase 5 — Routes exactas
- [ ] `vor-sim::routes`: sea routes, main roads, trails, navegación fluvial, roads Delaunay, `cells.routes`.
- [ ] `vor-render::route_layer`: paths curvos (`Routes.getPath`), stroke-width/dash/linecap/alpha, estilos por grupo, endpoints de burg.
- [ ] Validación: misma cantidad/grupo/endpoints/longitud/geometría.

### ⬜ Fase 6 — Goods y producción
- [x] **`vor-core::Good` modelado** (`entities/good.rs`): id/name/color/icon/chance/distribution/biomeOutput/multipliers/tags/unit/recipes/demandCoverage, `rename_all="camelCase"` + alias `i→id`. `world.goods` pasa de `serde_json::Value` a `Vec<Good>`; el loader parsea slot `[41]` tipado. Validado: **71 goods** en Sorvik.
- [x] **`vor-render::goods`** (`goods.rs`): `cell_production` (biomeOutput×pop + bonus channel), `build_goods_cells_mesh` (opacity normalizada por producción, masked blended) y `build_goods_icons_mesh` (círculo por célula con bonus). Cableado en vor-app con flag `goods` (2 sub-layers). Validado: cells 43613v/87231i, icons 639v/1704i. **Fix off-by-one**: goods se indexan por posición 0-based pero `cells.good` usa id 1-based → búsqueda por `good.id`.
- [ ] `vor-sim::goods` + `vor-sim::production`: catálogo default (GOODS_DATA), distribución DSL, `cells.good`, recetas, producción de burgs, stock.
- [ ] `vor-render::goods` subcapa **goodsBurgs**: placas top-3 por burg (requiere producción de burg).

### ⬜ Fase 7 — Markets
- [x] **`vor-core::Market` modelado** (`entities/market.rs`): id/centerBurgId/color/name/goods{stock,price}, `rename_all="camelCase"`. `world.markets` pasa de `serde_json::Value` a `Vec<Market>`; loader parsea slot `[42]`. Validado: **44 markets** en Sorvik.
- [x] **`vor-render::market`** (`market.rs`): `build_market_fill_mesh` (isolines por `cells.market`, fill-opacity 0.03), `build_market_border_mesh` (borde oscuro), `build_market_center_mesh` (círculo en burg central). Cableado en vor-app con flag `markets`. Validado: fill 5155v, border 21592v, center 748v.
- [ ] `vor-sim::markets`: selección de centros, expansión territorial con costes FMG, stock/precios.
- [ ] Hover highlight (transición 1s).

### ⬜ Fase 8 — Trade y animación
- [x] **`vor-core::Deal` modelado** (`entities/deal.rs`): id/seller/buyer/sellerType/buyerType/good/units/price/tax, `rename_all="camelCase"` + `DealEntityType`. `world.deals` pasa de `serde_json::Value` a `Vec<Deal>`; loader parsea slot `[43]`. Validado: **15492 deals** en Sorvik.
- [x] **`vor-render::trade`** (`trade.rs`): `build_trade_routes_mesh` — línea entre seller→buyer de cada deal, coloreada por good. Cableado en vor-app con flag `trade`. Validado: 57664v/86496i.
- [ ] Animación completa: Dijkstra sobre `cells.routes` (agua 1/tierra 5/cambio 20), batches concurrentes (30), wagon/ship, Catmull-Rom, rotación, duración, pausa por segmento, click para detalle.

### ⬜ Fase 9 — Validación de paridad
- [ ] Por layer: test de datos, geometría, render, snapshot visual, comparación contra FMG.
- [ ] Casos: continente grande, isla pequeña, lago interior, enclave, provincia con agua, burg portuario, ruta marítima, mercado multi-good, deal tierra-mar, goods raw+manufactured.

---

## 4. Checklist de avance (marcar en cada sesión)

- [x] Fase 0 — Decisión: opción 1 (paridad completa). Plan persistido en este archivo. (13 ago 2026)
- [x] Fase 0 — Dump de catálogos de Sorvik (contrato de datos fijado: 13/225/15/24/1009/815/13/71/44/15492).
- [ ] Fase 0 — Criterio de aceptación por layer formalizado.
- [x] Fase 1 — Motor común `build_region_mesh` (fill recto M/L/Z + isolines) implementado y cableado en states/provinces/cultures/religions (13 ago 2026).
- [x] Fase 1 — Zones: `build_vertex_path_mesh` (getVertexPath, contorno exterior por lista de celdas) en `zone_layer.rs`. Fase 1 COMPLETA.
- [x] Fase 2 — `vor-sim::states` portado (expansión FlatQueue con costes FMG exactos, determinista) + `pack.cells.water_type` propagado. Validado en Sorvik (13 states nativos). (13 ago 2026)
- [x] Fase 2 — `vor-sim::provinces` portado (expansión con costes de elevación, restringida al estado). Validado en Sorvik (310 provincias). (13 ago 2026)
- [ ] Fase 2 — Nombres/poles/COA de states+provinces + render (halo/borders/labels).
- [x] Fase 3 — `vor-sim::cultures` portado (centros + expansión con costes FMG). Validado en Sorvik (11 culturas). (13 ago 2026)
- [x] Fase 3 — `vor-sim::religions` portado (folk + organized + expansión por culture/state/global). Validado en Sorvik (28 religiones). (13 ago 2026)
- [ ] Fase 3 — Nombres/passageCost exactos de cultures+religions.- [x] Fase 4 — Population como barras (rural+urbana) en `population_layer.rs`. (13 ago 2026)
- [x] Fase 4 — Route mesh con Catmull-Rom en `route_layer.rs`. (13 ago 2026)
- [x] Fase 4 — Burg icons (círculo por estado) en `burg.rs`. (13 ago 2026)
- [ ] Fase 4 — Burg labels + state labels + anclas de puerto + production/market.
- [ ] Fase 5 — Routes exactas (generación + geometría visual).
- [x] Fase 6 — `Good` modelado en vor-core + parseo tipado del slot [41] (71 goods). (13 ago 2026)
- [x] Fase 6 — `vor-render::goods`: goodsCells + goodsIcons cableadas (flag goods). (13 ago 2026)
- [ ] Fase 6 — `vor-sim::goods`/`production` (catálogo, distribución, recetas, stock) + goodsBurgs.
- [x] Fase 7 — `Market` modelado en vor-core + parseo tipado del slot [42] (44 markets). (13 ago 2026)
- [x] Fase 7 — `vor-render::market`: fill+border+center cableados (flag markets). (13 ago 2026)
- [ ] Fase 7 — `vor-sim::markets` (centros, expansión, stock/precios) + hover highlight.
- [x] Fase 8 — `Deal` modelado en vor-core + parseo tipado del slot [43] (15492 deals). (13 ago 2026)
- [x] Fase 8 — `vor-render::trade`: trade routes mesh cableado (flag trade). (13 ago 2026)
- [ ] Fase 8 — Animación completa (Dijkstra, batches, wagon/ship, Catmull-Rom).
- [ ] Fase 9 — Validación de paridad completa.

---

## 5. Archivos previstos

### `vor-core`
- [ ] `entities/good.rs`, `entities/market.rs`, `entities/deal.rs` (nuevos)
- [ ] ampliar `entities/burg.rs`, `entities/route.rs`
- [ ] reemplazar campos opacos económicos en `world.rs`

### `vor-import`
- [ ] parseo tipado de slots `[41]` (goods), `[42]` (markets), `[43]` (deals)
- [ ] compatibilidad mapas antiguos, validación, roundtrip lossless

### `vor-sim`
- [ ] `states.rs`, `provinces.rs`, `cultures.rs`, `religions.rs`, `burgs.rs`, `routes.rs`, `goods.rs`, `production.rs`, `markets.rs`, `trade.rs`

### `vor-render`
- [ ] completar `isolines.rs` (motor común)
- [ ] `population_bars.rs`, `burg_icons.rs`, `burg_labels.rs`, `goods.rs`, `markets.rs`, `trade.rs` (nuevos)
- [ ] mejorar `route_layer.rs`

### `vor-app`
- [ ] cableado real de flags goods/markets/trade
- [ ] orden de capas, input/hover, animación, estilos configurables

---

## 6. Decisiones tomadas

1. **Opción 1 confirmada (13 ago 2026)**: paridad completa de generación + datos + render, ejecutada en el orden de fases anterior. La generación nativa en `vor-sim` es parte del objetivo, no una segunda etapa.
2. El plan vive en `docs/plans/human-geography-parity.md` y es la fuente de verdad tras cada compactación.
3. No se copia código TypeScript de FMG; se reimplementa la lógica en Rust con la arquitectura propia de Voronia.
4. **Regla de registro de capas (13 ago 2026)**: las capas con índices constantes (`LAYER_*` en `layers.rs`, 0..18) deben registrarse en `vor-app/src/lib.rs` en el **mismo orden exacto** (heightmap→relief→biomes→temp→prec→ice→lakes→rivers→state→province→culture→religion→population→zones→borders→burgs). Las capas "extra" (goods, markets, trade) se registran **después** de burgs, con índices devueltos por `add_layer_mesh` guardados en `State` y dibujadas explícitamente por flag en el render loop. **Bug corregido**: al insertar goods/markets en medio se desplazaron los índices de las capas fijas y los toggles activaban la capa equivocada.
5. **Composición visual**: states/provinces/cultures/religions/zones y overlays semitransparentes usan pipelines blended; el `waterGap` de regiones usa vértices raw, mientras el gap de celdas/biomes conserva smoothing porque esos fills usan vértices suavizados.
6. **Correcciones de composición ejecutadas**: alpha FMG aplicado a regions/zones/population/borders/markets/trade; `append_water_gap_raw` para regions; borders sobre aristas compartidas de Voronoi sin duplicación; PNG export usa stencil MSAA y capas economy 19..24; goodsCells dibuja todos los goods positivos, no solo el dominante.
