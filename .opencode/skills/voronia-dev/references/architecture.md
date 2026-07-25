# Voronia — arquitectura (referencia)

Resumen accionable para cuando hay que diseñar o tocar algo estructural. El detalle completo, con la justificación de cada decisión, vive en el plan maestro (§4–§13). Esto es lo que necesitás tener a mano mientras codeás, no la exposición completa.

## Flujo de datos

```
Import:   .map/JSON (Azgaar) → vor-import (regenera geometría + mapea atributos) → World Data Model
Generate: parámetros/semilla   → vor-sim                                          → World Data Model
Edit:     input de usuario     → vor-edit (comandos undo/redo-able)               → World Data Model (mutación controlada)
Render:   World Data Model     → vor-render                                       → GPU → pantalla (solo lectura, nunca escribe)
Save:     World Data Model     → vor-format                                       → .gmap
```

## World Data Model (`vor-core`) — campos clave

Basado en el modelo real de Azgaar (no inventado — verificado contra su wiki oficial de data model). Traducido a tipos fuertes de Rust en vez de arrays paralelos sueltos y strings mágicos.

- **`Grid`**: malla inicial (grilla cuadrada jitterizada). `cells_desired` (default 10k, min 1k, max 100k en Azgaar — usar como referencia de objetivos de escala), `spacing`, `points`, adjacencia/vértices de Voronoi.
- **`Pack`**: malla derivada de `Grid` tras "repacking" (optimizada a la masa de tierra real). La mayoría de la simulación opera acá, no en `Grid`.
- **Ni `Grid` ni `Pack` se leen de archivo** — se regeneran siempre desde semilla + parámetros (ver el hallazgo crítico en `SKILL.md`).
- **`Cell`** (por celda, esto SÍ se persiste): `height` (u8, 0-100, 20=nivel mínimo de tierra), `feature_id`, `distance_field` (i8, distancia a costa), `score` (para ubicar burgos), `biome`, `burg/culture/state/province/religion` (ids opcionales), `population` (f32, en "puntos de población"), `river`, `flux`/`confluence_flux` (hidrología), `harbor_score`, `haven_cell`, `routes`.
- **`Feature`**: island/lake/ocean. `is_land` (height≥20), `touches_border` (distingue lago de océano), subtipos (`continent/island/isle/lake_island` para tierra; `freshwater/salt/dry/sinkhole/lava` para lagos).
- **`Culture`**: `origins` (árbol de evolución), `expansionism`, `type` (enum, no string), namebase asociado.
- **`Burg`**: incluye flags de MFCG (`has_citadel/plaza/shanty/temple/walls`) — integración con Medieval Fantasy City Generator de Watabou, fuera de alcance de Voronia v1 pero el campo existe para compatibilidad de import.
- **`State`**: `form` (enum: Monarchy/Republic/Theocracy/Union/Anarchy), `diplomacy`, `campaigns` (guerras), `military: Vec<Regiment>`, `pole_of_inaccessibility` (centro visual del polígono).
- **`Province`**, **`Religion`** (tipos Folk/Organized/Heresy/Cult), **`River`** (con unidades físicas reales: `discharge_m3s`, `length_km`, `width_km`), **`Marker`**, **`Route`** (grupos roads/trails/searoutes), **`Zone`**, **`Ice`** (glacier/iceberg).
- **Datos globales**: `Biomes` (matriz `[temperatura][humedad] → bioma` + costo de movimiento + habitabilidad), `NameBases` (generador fonético por cultura: longitud mín/máx, letras duplicables, probabilidad de nombre multi-palabra).

Antes de implementar cualquiera de estas entidades a fondo, confirmar el detalle exacto contra la wiki de Azgaar (`Data-model`, y las páginas específicas: `Culture-types`, `Military-Forces`, `Goods:-spread-functions`, `Heightmap-customization`) — están linkeadas en el plan maestro §29.

## Formatos de archivo

- **Entrada**: JSON export completo de Azgaar (prioridad 1, más limpio) o `.map` legacy (prioridad 2, incluye un bloque SVG embebido que no aporta datos, solo ruido). En ambos casos, la geometría no viene en el archivo — hay que regenerarla (ver hallazgo crítico).
- **`.gmap` (formato propio)**: binario, no texto. Cabecera con versión + metadata (nombre, semilla, fecha, versión de Voronia que lo generó) + World Data Model serializado + opcionalmente buffers pre-triangulados listos para GPU. Versionado desde el día 1 (`u16 format_version`) para poder migrar sin romper mapas guardados.
- **Salida**: PNG a resolución arbitraria, SVG, GeoJSON, y opcionalmente JSON compatible con Azgaar (ida y vuelta entre herramientas).

## Pipeline de render (`vor-render`)

Orden de capas, de atrás hacia adelante: heightmap → océanos → biomas → ríos → hielo → zonas custom → culturas → provincias → estados/fronteras → religiones (alternable) → rutas → íconos de relieve → burgos → regimientos (alternable) → marcadores → labels → UI de mapa (grid/regla/brújula/leyenda).

Cada capa debe ser activable/desactivable sin regenerar las demás. Triangulación de celdas Voronoi vía `lyon`, cacheada (no retesela en cada frame). Cámara ortográfica 2D — sin perspectiva 3D, sin rotación en v1. Picking vía lookup espacial (quadtree/grid hash) sobre coordenada de mundo.

**Qué se beneficia de GPU realmente**: renderizado de geometría, texto, pan/zoom/culling. **Qué NO**: Delaunay/Voronoi, hidrología, expansión de culturas/estados (flood-fill con costo), pathfinding — todo eso es CPU, paralelizable entre sí con `rayon` pero no internamente GPU-friendly. No sobre-invertir en compute shaders para generación antes de que el perfilado lo justifique.

## Motor de simulación (`vor-sim`) — sistemas a portar

Heightmap (templates + ruido) → hidrología (flujo, ríos, lagos) → clima (temperatura/precipitación → biomas vía matriz) → culturas (expansión flood-fill con costo + namebases) → estados/burgos/provincias (scoring, expansión, capitales) → religiones (árbol, expansión culture/global) → rutas + economía (pathfinding + bienes) → milicia/diplomacia (regimientos, guerras — sistema real de Azgaar, no una idea nueva, ver wiki `Military-Forces`/`Battle-Simulator`).

Orden de implementación sugerido: seguir el roadmap del plan maestro §23 (Fase 7), no reordenar sin razón — hay dependencias reales entre estos sistemas (biomas necesitan clima, culturas necesitan biomas para el costo de expansión, etc.).
