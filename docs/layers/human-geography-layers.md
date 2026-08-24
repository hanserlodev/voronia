# Human Geography + Borders — estado de paridad (22 ago 2026)

> **Category**: Human Geography · **Layers**: states, provinces, zones, cultures, religions, population, burgs, markets, trade (+ Borders)
> **Source**: azgaar-fmg checkout local, **v1.138.0** (commit 51d8e3e). Plan: `.opencode/plans/human-geography-parity.md`.
> Reemplaza la versión anterior (v1.135.2 — obsoleta: hablaba de heatmap para population, burgs "triángulos rojos" y routes LineList).

## Z-order (ya global)

`#relig(13) < #cults(14) < #regions states(15) [statesBody+statesHalo] < #provs(16) < #zones(17) < #borders(18) < #routes(19) < … < #markets(24) < #tradeAnimation(25) < #population(27) < #icons(29) [burgIcons→anchors]`

Slots Voronia: `LAYER_RELIGION_FILL=6, CULTURE_FILL=7 (+dyn culture_stroke), STATE_FILL=8, PROVINCE_FILL=9, ZONES=10, BORDERS=11-13, POPULATION=17, BURGS=18` + `dyn_ids.goods_burgs/market_*/trade`.

## Estado por capa

| Layer | Estado | Detalles |
|---|---|---|
| States | ✅ | `build_region_mesh` (getIsolines, anillos rectos evenodd) + water gap; alpha **0.4**, masked `url(#land)`; color del catálogo slot `[14]`. Divergencias: `statesHalo` NO implementado (en FMG solo renderiza con `shapeRendering=geometricPrecision`, no es default); labels de estado → categoría Labels |
| Provinces | ✅ | Mismo motor; alpha **0.7**. Labels → Labels |
| Religions | ✅ | Alpha 0.7, sin stroke (FMG w0). Pendiente menor: filtrar religiones `removed` |
| Cultures | ✅ | Alpha 0.6 + **stroke de fill `#777777` w0.5** portado (`build_region_stroke_mesh` → `dyn_ids.culture_stroke`) |
| Zones | ✅ | **Hatching por tipo** (`build_zone_hatch_mesh`): líneas negras clippeadas al contorno `getVertexPath` a escala tile 1:1, grupo op 0.6. Mapeo: invasion→hatch1 (45° sp4 w2), flood→hatch2 (horizontal), rebels→hatch3 (−45°), eruption/fault→hatch5 (grid 45°/135° w1.5), proselytism/crusade→hatch6 (**puntos r1 grid 5**), avalanche→hatch7 (−45° sp3 w1.5), disease→hatch12 (dos familias offset), tsunami→hatch13. Kinds desconocidos sin geometría |
| Population | ✅ | Barras `pop/5` rurales y `(pop/5)·urbanization` urbanas ✅; colores FMG **rural `#0000ff` / urbano `#ff0000`**, ancho total **1.6** |
| Burgs | ✅ | Iconos blancos **por tipo** (fill `#ffffff` op0.7, stroke `#3e3e4b`, centrados en el burg): capital=cuadrado fs2 sw1 · city=círculo 1.5 sw1 · town=círculo 1 sw1.2 · village 0.7 · hamlet 0.5 · fort=cuadrado 0.7 · monastery=cruz · caravanserai/trading_post=triángulo. **Anchors de puerto** (opacos, sw1.2) tras todos los iconos. Requiere `burg.group` (parseado del slot `[15]`). Pipeline blended. Divergencia: anchor = aproximación geométrica del path curvo de FMG; labels de burg → Labels |
| Markets | ✅ | Territorio = get_isolines por mercado con **curveBasisClosed** (curvo), fill `market.color` op **0.03**; border = stroke **`darken(fill)` w0.7 op0.8** sobre el anillo curvo (FMG lo clipea al propio fill — nosotros trazamos el anillo completo, documentado); centro círculo **r4** (`max(rn(3+1/scale,2),2)`). Divergencia: emoji ⚖️ no renderizado (glyphon sin soporte emoji fiable) |
| Trade | ✅* | **Extensión documentada**: quads estáticos por deal coloreados por bien. FMG no dibuja nada estático (`#tradeAnimation` runtime-only con marcadores ship/wagon + Dijkstra — Fase 8). El `.map` guarda el grupo vacío |
| Borders | ✅ | Estilos exactos por kind: state `#56566d` **w1 dash [2,2]** butt · province `#56566d` **w0.5 dots "0 2" round** · culture = extensión propia ámbar sólida (`#cultureBorders` no existe en FMG). Geometría por aristas Voronoi compartidas (el port a cadenas `getBorderPath` queda pendiente — los strokes son equivalentes por arista) |

## Tests

- `burg.rs`: shapes por grupo (capital cuadrado/town círculo), anchors solo en puertos, removed skipped, fallback town.
- `zone_layer.rs`: mapeo kind→patrón (dots para crusade, familias/ángulos), spans inside-polygon.
- `border.rs`: estilos por kind vía tabla (colores/dash).
- `hg_catalogs_dump.rs`: paridad isolines regionales vs distinct types, zone hatch finito, population bars, burg icons, markets centers, deals+trade mesh.
- `isoline.rs::region_mesh_fill_matches_isolines`: fills regionales.

## Divergencias aceptadas

1. `statesHalo` omitido (no-default en FMG; requiere blur GPU).
2. Labels (states/provinces/burgs/population values) → categoría Labels / Fase C.
3. Anchor de puerto geométrico (aproximación del path curvo).
4. Emoji ⚖️ del centro de mercados no renderizado (glyphon).
5. Trade estático = extensión Voronia (animación real → Fase 8).
6. Borders por aristas individuales (no cadenas getBorderPath continuas).
7. Generación nativa: markets/trade/zones siguen import-only (states/provinces/cultures/religions ya generan en vor-sim).

## Fuera de alcance (Fase C / Fase 8)
Style editor por capa, tooltips, trade animation completa, generación nativa de markets/zones/trade, emblems/armies/markers/rulers (grupo Overlay).
