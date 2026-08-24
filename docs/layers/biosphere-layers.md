# Biosphere — estado de paridad (22 ago 2026)

> **Category**: Biosphere (panel) · **Layers**: biomes, goods, routes
> **Source**: azgaar-fmg checkout local, **v1.138.0** (commit 51d8e3e). Plan: `.opencode/plans/biosphere-parity.md`.
> Relief icons (el tercer miembro histórico de esta categoría) se completó en la ronda Landmass — ver `docs/layers/landmass-layers.md` §Relief.

## Z-order (ya global desde la ronda Landmass)

`#biomes` hijo 6 (slot `LAYER_BIOMES=3` + `dyn_ids.biome_gap`) · `#routes` hijo 19 (`dyn_ids.routes_roads/trails/searoutes`) · `#goods` hijo 23 (`dyn_ids.goods_cells` + `DrawItem::GoodsIcons` + `dyn_ids.goods_burgs`).

## Estado por capa

| Layer | Estado | Detalles |
|---|---|---|
| Biomes | ✅ | **Motor de isolines FMG** (`build_biome_isolines_meshes`): una pasada de `get_isolines` por biome id (marine=0 skipped como en FMG), anillos rectos cerrados, **evenodd**. Colores del slot `[3]` del .map (paleta exacta). **WaterGap fiel**: port de `getBorderPath` — stroke w3 Round solo entre vértices no-all-land (`vertices.c[v].every(h≥20)`), color del bioma, mesh blended sobre el fill (`dyn_ids.biome_gap`). Mask `url(#land)` ✅. **`coast_fill` retenido** como anti-halo de seguridad (prepended al fill; candidato a eliminación tras verificación visual — FMG no lo necesita). Divergencia menor: `rx` de placas y `crumpled`-style filtros no aplican aquí; `move_cost` hardcodeado 50 en el catálogo (el .map no lo trae) |
| Goods | ✅ núcleo | `goodsCells`: producción rural por celda con opacidad normalizada `0.1+0.9·norm` (fiel, ya existente). **`goodsIcons` real**: círculo r3 (data-size 6) fill `good.color` + stroke `darker(2)` w0.3 (`build_goods_icon_circles_mesh`) + **símbolo** vía atlas `assets/textures/goods/atlas.png` (71 símbolos `good-*` rasterizados de `index.html`, 8×9 celdas de 64px; `GOOD_SYMBOL_IDS` orden canónico) — `GoodsIconsOverlay` (quads UV). **`goodsBurgs`**: placas top-3 por valor bajo burgs con producción (`build_goods_burg_plates`): rect `#f5f5f5` + stroke `#41414f` w0.2 + círculo r1.5 + símbolo size 3 + **texto valor** (glyphon, 3.5px mundo, `#28282f`); constantes PLATE_* de `draw-goods.ts:8-16`; `Burg.production` parseado del slot burgs (entradas `{goodId,units}`, `dealId` descartadas). Divergencias: sombra dropShadow01 de icons omitida, `rx=1` de placa aproximado a rectángulo, iconos custom (`good-custom-*`) sin símbolo (quad skipped) |
| Routes | ✅ | **Strokes tessellados por subgrupo** (`build_route_group_meshes`), fuera LineList: roads `#d06324` w0.7 dash `2` · trails `#d06324` w0.25 dash `.8 1.6` · searoutes `#ffffff` w0.35 dash `1 2` round — todo op 0.9, blended. Curvas `catmull_rom_open_alpha`: **0.1** roads/trails, **0.5** searoutes (port exacto de d3 hecho en la ronda Water & Climate). **Dash generator propio** (`dash_segments`: recorre la polyline muestreada y emite runs `on` según el patrón — lyon no tiene dasharray). Coordenadas de muestra redondeadas a 1 decimal (fiel a `round(lineGen,1)`). Skip rutas <2 puntos ✅ |

## Tests

- `biome.rs::isoline_tests`: pack vacío → meshes vacíos.
- `route_layer.rs`: dash splitter (runs exactos en línea de longitud 10, dash [2,2]) + estilos por subgrupo vs `default.json`.
- `goods.rs::plate_tests`: top-3 por valor acumulado con cutoff FMG, formato de valor (`String(rn(v,1))` sin `.0`).
- `biosphere_diag.rs` (integración Sorvik): fill de biomas no vacío y opaco, gap costero presente, burgs con producción > 0, placas top-3 con labels numéricos, 3 meshes de rutas con colores correctos (roads naranja, searoutes blanco).
- `hg_catalogs_dump.rs`: smoke de catálogos (preexistente).

## Divergencias aceptadas

1. Sombra `dropShadow01` de goodsIcons omitida (offset 0.2/0.3 — imperceptible a size 6).
2. `rx=1` de las placas aproximado a rectángulo recto.
3. Iconos custom de bienes (añadidos por el usuario en FMG) sin símbolo horneado.
4. `coast_fill` de biomas retenido (no existe en FMG; nuestro seguro anti-halo del fractal).
5. Generación nativa: biomas (`getId`/biomesMatrix), rutas (grafo de Urquhart) y bienes (production-generator) siguen import-only — el render es 1:1 con datos del `.map`.

## Fuera de alcance (Fase C)

Style editor por capa (opacity/stroke/dash editables, group selector de routes), tooltips de goods, editor de bienes, generación nativa (Fase 7).
