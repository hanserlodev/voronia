# Overlay — estado de paridad (22 ago 2026)

> Último grupo del roadmap de layers. Fuente: azgaar-fmg v1.138.0 local.

| Flag | Estado | Detalles |
|---|---|---|
| markers | ✅ | Pins tessellados (13 formas, fill blanco stroke negro), anclaje centro-inferior (`x−size/2, y−size`), emoji vía glyphon centrado en el pin; slot `[35]` ya parseado (+ campo `size`). Rescale por zoom pendiente fina |
| wind rose | ✅ | Es la **brújula Compass** de FMG (hijo 10, antes de rivers): líneas radiales 8 ángulos ±20000 `#3f3f3f` w1.1 + núcleo rasterizado a textura desde `#defs-compass-rose` (`assets/textures/compass.png`, viewBox −220..220). Colocación `translate(80,80) scale(0.25)` op 0.8, **stencil agua** (solo visible sobre océano — `mask=url(#water)` FMG) |
| rulers | ✅ | Measurers persistidos slot `[46]` (+campo `type`): doble línea gris sólida + blanca dash10 w2, Opisometer catmull alpha 0.5, Planimeter lightblue op0.5; label distancia km en el punto medio (glyphon). Herramienta interactiva = fuera de alcance |
| labels | ✅ núcleo | Burg labels fieles por grupo (capital fs6 dy−0.5 / city 5 / town 4 / village 3 / fort-monastery 2, fill `#3e3e4b`) vía glyphon con rescale FMG `(desired+desired/scale)/2`. **State labels curvados**: raycasting desde el polo (9° step, LENGTH_MAX 300) + texto Almendra SC (TTF OFL embebida en `assets/fonts/`) rasterizado con fontdue y quads por carácter rotados según tangente; rebuild por bucket de zoom |
| icons | 🗑 eliminado | Duplicado del toggle burgs (en FMG "Icons" ES burgIcons) |
| vignette | 🗑 no portada | Decisión de hanserlodev — única pieza de FMG omitida deliberadamente |
| scale_bar | ✅ | Screen-space egui: fórmula FMG `val=100·barSize·distanceScale/scale` con snap a órdenes limpios, 5 subdivisiones etiquetadas, doble línea blanca/gris sobre fondo op0.2, esquina inferior derecha |
| emblems | ⏳ defer | Motor heráldico completo → Fase C (checkbox oculto) |

Divergencias: rescale fino de markers pendiente; emoji dependen de fuentes del sistema; halo de text-shadow ausente; state-label raycasting greedy simplificado vs scoring completo de FMG.
