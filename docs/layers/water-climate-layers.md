# Water & Climate — estado de paridad (22 ago 2026)

> **Category**: Water & Climate · **Layers**: lakes, rivers, temperature, precipitation, ice
> **Source**: azgaar-fmg checkout local, **v1.138.0** (commit 51d8e3e). Plan: `.opencode/plans/water-climate-parity.md`.
> Reemplaza la versión anterior (v1.135, previa a la ronda de paridad W1–W6).

## Z-order (ya global desde la ronda Landmass)

`#viewbox` hijos: … lakes(5) … rivers(11) … temperature(19) → coastline(20) → ice(21) … prec(25/26) …
Slots Voronia: `LAYER_LAKES=2` (+`dyn_ids.lake_stroke`), `LAYER_RIVERS=4` (masked), `LAYER_TEMPERATURE=14` (blended, sin mask), `LAYER_ICE=15` (blended, +`dyn_ids.ice_shadow/ice_stroke`), `LAYER_PRECIPITATION=16`.

## Estado por capa

| Layer | Estado | Detalles |
|---|---|---|
| Lakes | ✅ | Pipeline fractal de costa ya portado (simplify→clip→fractalize→hybrid path). **6 subgrupos con estilos exactos** de `default.json:122-163` vía `Feature.lake_group` (se añadió la variante `Frozen`, temp<−3): freshwater `#a6c1fd`/`#5f799d` w0.7 op0.5 · salt `#409b8a` · sinkhole `#5bc9fd` op1 · frozen `#cdd4e7` op0.95 sin stroke · lava `#90270d`/`#f93e0c` w2 op0.7 · dry `#c9bfa7`. Fill blended NonZero + stroke tessellado por lago (Round), dibujado justo encima del fill (`dyn_ids.lake_stroke`). Divergencia: filtro SVG `crumpled` de lava omitido |
| Rivers | ✅ | Ribbon con `get_offset/get_width` exactos (flux^0.7/500, Fibonacci/200 — ya portados); el `width_factor` por río ya trae el main-stem ×1.2 desde el .map. Curva **`curveCatmullRom.alpha(0.1)`** portada de d3 (`catmull_rom_open_alpha`, empieza en el 2º punto como d3). Color `#5d97bb`. **Mask `#land`** (stencil del landmass fractal corta la boca en la costa, como FMG) — el clip manual `clip_to_coast` fue eliminado. NonZero. `svg_export` unificado al mismo modelo de ancho. Divergencia: ancho 1:1 solo en el mesh GPU; el export SVG aproxima con stroke central |
| Temperature | ✅ | Bandas con paridad fuente verificada (0.3/1.8/darker(0.2)/basisClosed/d3.range/NonZero) desde ago 2026. **Sin mask** (FMG pinta sobre océano también — se quitó la nuestra). **Labels de isotermas** portados (`addLabel`: punto superior-centro `min(y−\|x−xc\|/2)`, 2º label abajo si >20 pts y dist²>100, descarte a <20px del borde), `convert_temperature` con las **8 escalas** de `unitUtils.ts` (°C/°F/K/°R/°De/°N/°Ré/°Rø), font 8px mundo (escala con zoom), fill #000 op1. Selector de unidad en Style tab. Divergencia: sin text-shadow blanco (glyphon no hace outline), font weight normal |
| Precipitation | ✅ | Círculos `r=rn(sqrt(prec/4)/cellsNumberModifier)` (ya portado), **sin mask** (se quitó — cortaba círculos costeros). **Flechas de viento** `g#wind` portadas (`wind_glyphs`): tiers de 30° con `options.winds=[225,45,225,315,135,315]`, ⇉ x=20 / ⇇ x=W−52 por banda con >3 filas, ⇊ (W/2,42) / ⇈ (W/2,H−20), font 32px, fill `#003dff`. Divergencia: sin animación de entrada/salida (800/1000ms), sin text-shadow |
| Ice | ✅ | **3 meshes blended** en orden FMG: sombra `dropShadow01` (copia negra offset (0.2,0.3), blur 0.1 omitido) → fill `#f1f8fe` op0.9 → stroke `#e8f0f6` w0.5. Polígonos crudos sin curvar (fiel). `IceKind` glacier/iceberg comparten estilo (como FMG); **drift de icebergs**: campos `offset`/`size` añadidos a `vor-core::Ice` y parseados del slot 39, offset aplicado como translate |

## Tests

- `lakes.rs`: estilos por subgrupo vs defaults FMG (alpha, hue, widths).
- `river.rs` + `river_mouth_diag.rs`: mesh de ríos en Sorvik — no vacío, en-bounds con holgura, determinista.
- `temperature.rs`: `d3_range`, colores Spectral, `convert_temperature` (5 fórmulas), labels de unidades.
- `precipitation.rs::wind_tests`: tiers de viento → glifos ⇉/⇊/⇈ con posiciones exactas.
- `ice_layer.rs`: 3 meshes, offset aplicado, alpha 0.9.
- `layers.rs`: orden FMG con los nuevos items (lake_stroke, ice shadow/fill/stroke).

## Divergencias aceptadas

1. Sin animaciones de FMG (fade in/out de lakes/prec/ice, transición de círculos de prec).
2. Filtros SVG omitidos: `crumpled` (lava), blur de `dropShadow01` (offset exacto sí).
3. Labels sin halo/text-shadow (glyphon sin outline) y sin bold.
4. `svg_export` de ríos = aproximación con stroke central del mismo modelo de ancho.
5. `Math.round` half-up vs `f32::round` half-away en conversiones (despreciable).
6. Generación nativa de temp/prec/vientos NO porteada (los datos vienen del .map; solo el render es 1:1).

## Fuera de alcance (Fase C)

Panel Style por capa (opacidad/fill/stroke editables por subgrupo de lagos), labels de lagos y ríos (categoría Labels), simulación climática nativa (calculateTemperatures/generatePrecipitation) para mapas generados in-app.
