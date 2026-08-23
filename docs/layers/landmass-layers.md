# Azgaar Landmass Layers — estado de paridad (22 ago 2026)

> **Category**: Landmass · **Layers**: texture, heightmap, relief, cells, grid, coordinates + coastline strokes + ocean
> **Source**: azgaar-fmg checkout local, **v1.138.0** (commit 51d8e3e). Referencia z-order: `public/main.js:33-90`.
> Plan de ejecución: `.opencode/plans/landmass-parity-a-b.md`. Esta página reemplaza la versión anterior (basada en v1.135 y desactualizada respecto al código).

## Z-order implementado (`LayerFlags::draw_sequence`, fuente única)

`ocean[bathymetry] → landmass → texture(mask) → terrs(heightmap) → lakes → biomes → cells → grid → coordinates → rivers → relief → religions → cultures → states → provinces → zones → borders(s/p/c) → routes → temperature → coastline[shadow?, stroke] → ice → goods → markets → trade → prec → population → burgs`

El océano base (`#466eab` opaco) y el patrón PNG tileable (alpha 0.2) se dibujan como quads fijos antes de la secuencia; la batimetría es un mesh layer dinámico al inicio de esta.

## Estado por capa

| Layer | Estado | Detalles |
|---|---|---|
| Ocean | ✅ | Base `#466eab` opaca; batimetría `#oceanLayers` portada literal (`ocean_layers.rs`: water_type slot 10, `findStart`/`connectVertices` con outside={0,t−1}, relax `1+t·−2`, curveBasisClosed round 1, fill `#ecf2f9` op `0.4/n`, presets default `-6,-3,-1`); patrón `pattern1.png` tile 100×100 alpha 0.2 (`OceanPatternOverlay`). Pendiente Fase C: selector de patrón/opacidad |
| Texture | ✅ | Capa sobre landmass enmascarada a tierra por stencil (`mask:url(#land)` FMG); shift X/Y world-units vía uniform + sliders en Style tab; default marble-big. PNG export aún no la dibuja (Fase C) |
| Heightmap | ✅ núcleo | Bandas por nivel con `skip+1` step (default skip=5), contornos curvados `curveBasisClosed`, 7 esquemas bit-fieles (`HeightmapScheme`: Spectral/RdYlGn/Greens/Greys + rgbBasis natural/olive/livid), terracing (copia `darken(ter)` offset `.7,1.4`), stride `relax` (`simplifyLine`) — todo vía `HeightmapBandOptions`. Pendiente: render ocean-heights (off en FMG), UI de opciones (Fase C) |
| Coastline | ✅ | Siempre visible (sin toggle en FMG). `build_coastline_meshes`: sea_island `#1f3846` w0.5 op0.5 / lake_island `#7c8eaf` w0.35 op1 sobre el path híbrido Q/Catmull-Rom; sombra = copia offset (1,2) op 0.3 solo mar; auto-filtro: sombra solo con zoom_scale ≤ 1.5 (`SHADOW_MAX_SCALE`). Divergencia: sin blur real del `dropShadow` SVG ni el blur 0.2 de zoom >2.6 |
| Cells | ✅ | Wireframe pack, `#808080` op1. Divergencia: LineList 1 px vs stroke-width 0.1 SVG. Pendiente Fase C: modo grid en editores + auto-toggle |
| Grid | ✅ | PointyHex con la tabla de 7 segmentos original (validada visualmente); estilo `#777777` op 0.8; bounds = canvas completo. **Revertido** el intento de portar los 10 patterns con parser M/L/H/V/Z: renderizaba mal (sospecha: la arista coincidente que duplica el `Z` tras `V 7.2` se sobre-dibuja con alpha 0.8 y oscurece la línea central de cada hex — a revisar si se retoman los patterns) |
| Coordinates | ✅ | Step dinámico por zoom (`goal = lonT/scale/10`, rebuild solo al cambiar step elegido), steps `[0.5,1,2,5,10,15,30]`; labels `#333333` con font `desired/scale^0.8` (`label_font_px`); líneas `#d4d4d4`. Divergencias: sin dasharray 5, ancho fijo 1 px, sin halo blanco de labels |
| Relief | ✅ | Poisson-disc determinista (`poisson_disc`, k=3, stream Alea `{seed}_relief` — divergencia vs Math.random global FMG documentada); tablas exactas `iconsDensity` + picks uniformes por bioma (`biomes.ts`), `cactus`/`deadTree`→dune en set simple; radios `2/density` (h≥50) y `2/(iconsDensity/100)/density` con rechazo `rand > d·10` (h<50); tamaños `(h−45)·mod` mount (nevado colapsa a mount en set simple) / `clamp((h−40)·mod,3,6)` hill; grass ×1.2; skip celdas con río; sort por `y+s`; **9 símbolos** del set simple (incluye acacia) rasterizados de `index.html` a `assets/textures/relief/atlas.png` 768² (3×3×256px) y renderizados como quads con alpha blending (`ReliefIconsOverlay`, `DrawItem::Relief` en la posición exacta de `#terrain`). Slot `LAYER_RELIEF=5` ocupado con mesh vacío para mantener contigüidad de constantes. Sets colored/gray (77 símbolos, variantes + temp) en Fase C |

## Notas de implementación

- `DynamicLayerIds` concentra todos los índices runtime (líneas + economía + costa + batimetría); `DrawOptions.coastline_shadow` depende del zoom.
- La textura usa el stencil existente (`stencil_mask_test()` nuevo helper en renderer.rs).
- El graticule se reconstruye SOLO cuando cambia el step elegido tras un zoom (no cada frame).
- Tests nuevos: secuencia FMG completa, sombra costa según opción, split lake/shadow costa, 8 segmentos pointyHex (Z duplica arista coincidente — fiel a SVG), esquemas válidos + darken(1)=0.7, patterns ×10 parsean, pick_step incluye 30, label_font_px contra fórmula.

## Pendientes Fase C (fuera de esta ronda)
Presets panel = `getDefaultPresets()` exactos (political default SIN provinces/population como tenemos hoy); panel Style por capa (esquemas heightmap, tipo de grid, densidad relief, patrón océano); PNG export con textura/patrón/líneas/texto; limpieza dead code (`heightmap.rs::build_mesh`, `build_landmass_mesh_legacy`, `tessellate_fill`); golden-image testing headless.
