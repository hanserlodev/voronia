**Última actualización**: 30 julio 2026 — Port del sistema de ríos de Azgaar COMPLETO

## Fase actual

**Fase 7 — Motor de generación procedural nativo**: ⏳ **EN PROGRESO** (30 jul 2026). Hidrología (ríos) completa.

## Port de ríos — Estado final

### vor-core
- **River model**: `width_factor`, `source_width_km`, `type_name`, `meandered_points` añadidos
- **PackCells**: `feature_id: Vec<u16>` añadido
- **Feature**: `shoreline`, `lake_height`, `inlets`, `outlet_river`, `entering_flux`, `closed`, `out_cell` — todos los campos de lago necesarios para hidrología

### vor-import
- **RiverRaw**: parsea `widthFactor`, `sourceWidth`, `type`, `cells` (con `-1`→`u32::MAX`), `points`
- **FeatureRaw**: `shoreline` y `height` mapeados a Feature
- **PackCells feature_id**: poblado desde grid cells via `grid_id` mapping

### vor-sim (motor de simulación procedural)

| Módulo | Azgaar | Voronia |
|--------|--------|---------|
| hydrology | `alterHeights()` | ✅ |
| | `resolveDepressions()` | ✅ (Priority-Flood) |
| | `Lakes.defineClimateData()` | ✅ (Penman evaporation) |
| | `Lakes.detectCloseLakes()` | ✅ (BFS desde shoreline) |
| | `drainWater()` + lake outlets | ✅ |
| | `flowDown()` + confluencias | ✅ |
| width | `getOffset()`, `getSourceWidth()`, `getWidth()` | ✅ fórmulas exactas |
| meander | `meander()`, `relaxAcuteAngles()`, `addMeandering()` | ✅ |
| river_def | `defineRivers()`, `calculateConfluenceFlux()` | ✅ |
| | `downcutRivers()` | ✅ |
| specify | `specify()`, `getParent()`, `getBasin()`, `getName()`, `getType()` | ✅ simplificado |
| | `remove()`, `getNextId()`, `getApproximateLength()` | ✅ |
| resolve | `resolveLakeDrainFeature()`, `resolveDrainFeature()` | ✅ |
| | `isNavigable()` | ✅ |

### Constantes de Azgaar replicadas
`MIN_FLUX_TO_FORM_RIVER=30`, `MIN_NAVIGABLE_FLUX=100`, `FLUX_FACTOR=500`, `MAX_FLUX_WIDTH=1`, `LENGTH_FACTOR=200`, `MAX_DOWNCUT=5`, `WATER_MEANDER_SCALE=0.25`

### Tests
**79 tests total** (67 existentes + 12 nuevos de vor-sim).
- width: fórmulas getOffset/getSourceWidth/getWidth
- meander: 2 puntos, 1 punto sin cambios
- specify: getApproximateLength, getNextId
- rn: redondeo estilo JS Math.round
- Todos los tests de importación (Sorvik) verdes

## Pendiente (post-Fase 7)
- Integrar vor-sim::generate() con vor-app (generación nativa vs solo import)
- Unificar meander (vor-render tiene copia, vor-sim tiene original)
- Tests end-to-end de generate() con un mundo real
- Optimizar: todo el pipeline es O(n²) en el peor caso (resolveDepressions)
