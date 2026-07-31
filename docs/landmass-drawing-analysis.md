# Análisis exhaustivo: cómo Azgaar dibuja toda la masa de tierra

> **Fecha**: 30 jul 2026
> **Fuente**: Azgaar's FMP — `/home/hans/Proyectos/azgaar-fmg/`
> **Propósito**: Referencia completa para el port nativo de renderizado de tierra en Voronia
> **Cubre**: TODO el pipeline de dibujo de landmass — desde la geometría base hasta las capas temáticas

---

## Índice

1. [Arquitectura de capas (Z-order)](#1-arquitectura-de-capas-z-order)
2. [Base: Features (tierra, océano, costa, lagos)](#2-base-features-tierra-océano-costa-lagos)
3. [Fractalización de costa](#3-fractalización-de-costa)
4. [Océano: base y batimétrica](#4-océano-base-y-batimétrica)
5. [Motor de isolíneas compartido](#5-motor-de-isolíneas-compartido)
6. [Heightmap (contornos de elevación)](#6-heightmap-contornos-de-elevación)
7. [Biomas](#7-biomas)
8. [Culturas](#8-culturas)
9. [Religiones](#9-religiones)
10. [Estados](#10-estados)
11. [Provincias](#11-provincias)
12. [Bordes (fronteras)](#12-bordes-fronteras)
13. [Ríos](#13-ríos)
14. [Rutas (caminos, senderos, rutas marítimas)](#14-rutas-caminos-senderos-rutas-marítimas)
15. [Íconos de relieve](#15-íconos-de-relieve)
16. [Burgos (asentamientos)](#16-burgos-asentamientos)
17. [Etiquetas de estado (texto curvo)](#17-etiquetas-de-estado-texto-curvo)
18. [Etiquetas de burgo](#18-etiquetas-de-burgo)
19. [Temperatura (isotermas)](#19-temperatura-isotermas)
20. [Precipitación](#20-precipitación)
21. [Población](#21-población)
22. [Capa de hielo](#22-capa-de-hielo)
23. [Bienes (goods)](#23-bienes-goods)
24. [Mercados](#24-mercados)
25. [Emblemas (escudos)](#25-emblemas-escudos)
26. [Militar](#26-militar)
27. [Textura satelital (3D)](#27-textura-satelital-3d)
28. [Overlays: textura, grilla, coordenadas](#28-overlays-textura-grilla-coordenadas)
29. [Patrones arquitectónicos clave](#29-patrones-arquitectónicos-clave)
30. [Estado de portabilidad a Voronia](#30-estado-de-portabilidad-a-voronia)

---

## 1. Arquitectura de capas (Z-order)

**Archivo**: `public/modules/ui/layers.js:225-261` — `drawLayers()`

Es la función maestra de dibujo, llamada en cada generación/refresco del mapa. El orden visual (Z-order) lo determina el orden de los grupos `<g>` en el DOM del SVG.

| Z | Grupo SVG | Renderizador | Qué dibuja |
|---|-----------|-------------|------------|
| 1 | `#ocean` | template | Rectángulo base de océano + patrón |
| 2 | `#oceanLayers` | `OceanLayers` | Anillos batimétricos de contorno |
| 3 | `#oceanPattern` | — | Imagen de textura oceánica |
| 4 | `#coastline` | `drawFeatures` | Líneas de costa (mar + lagos) |
| 5 | `#lakes` | `drawFeatures` | Relleno de lagos (fresh/salt/sinkhole...) |
| 6 | `#landmass` | (máscara) | Paths de tierra para reúso |
| 7 | `#terrs` | `drawHeightmap` | Contornos de altura (océano + tierra) |
| 8 | `#texture` | `drawTexture` | Textura de fondo (papel, pergamino...) |
| 9 | `#biomes` | `drawBiomes` | Relleno de biomas por celda |
| 10 | `#cells` | `drawCells` | Wireframe de celdas Voronoi (debug) |
| 11 | `#gridOverlay` | `drawGrid` | Grilla hex/cuadrada |
| 12 | `#coordinates` | `drawCoordinates` | Graticule lat/lon + etiquetas |
| 13 | `#compass` | — | Rosa de los vientos |
| 14 | `#rivers` | `drawRivers` | Polígonos de río |
| 15 | `#terrain` | `drawReliefIcons` | Íconos de relieve (montañas, árboles) |
| 16 | `#relig` | `drawReligions` | Relleno de religiones |
| 17 | `#cults` | `drawCultures` | Relleno de culturas |
| 18 | `#regions` | `drawStates` | Relleno de estados + halos |
| 19 | `#provs` | `drawProvinces` | Relleno de provincias + etiquetas |
| 20 | `#zones` | `drawZones` | Relleno de zonas |
| 21 | `#borders` | `drawBorders` | Líneas de borde estado + provincia |
| 22 | `#routes` | `drawRoutes` | Caminos, senderos, rutas marítimas |
| 23 | `#temperature` | `drawTemperature` | Isotermas coloreadas + etiquetas |
| 24 | `#prec` | `drawPrecipitation` | Círculos de precipitación |
| 25 | `#population` | `drawPopulation` | Barras de población |
| 26 | `#ice` | `drawIce` | Glaciares + icebergs |
| 27 | `#goods` | `drawGoods` | Producción de bienes |
| 28 | `#markets` | `drawMarketsLayer` | Zonas de influencia de mercado |
| 29 | `#tradeAnimation` | `TradeAnimation` | Animación de rutas comerciales |
| 30 | `#emblems` | `drawEmblems` | Escudos de burgo/provincia/estado |
| 31 | `#labels` | `drawLabels` | Etiquetas de estado (curvas) + burgo |
| 32 | `#icons` | `drawBurgIcons` | Puntos de asentamiento + anclas |
| 33 | `#armies` | `drawMilitary` | Rectángulos de regimiento |
| 34 | `#markers` | `drawMarkers` | Marcadores de usuario |
| 35 | `#ruler` | `drawMeasurers` | Medición de distancia/área |
| 36 | `#fogging` | — | Niebla de foco de estado |
| 37 | `#vignette` | template | Viñeta oscura en bordes |
| 38 | `#scaleBar` | `drawScalebar` | Barra de escala |
| 39 | `#legend` | — | Leyenda del mapa |

> **Nota**: No hay un "relleno de tierra" explícito. La tierra se rellena indirectamente a través de la capa activa (heightmap, biomas, estados, culturas, etc.).

---

## 2. Base: Features (tierra, océano, costa, lagos)

**Archivo**: `src/renderers/draw-features.ts:20-74` — `featuresRenderer()`  
**Archivo**: `src/renderers/draw-features.ts:76-87` — `featurePathRenderer(feature)`

### Datos que consume
- `pack.features[]` — array de Feature con `type` (landmass, sea_island, lake_island, lake), `vertices[]`, `group`, `i`
- `pack.vertices.p` — coordenadas de vértices

### Algoritmo
```
1. Para cada feature no-oceánico:
   a. Mapear vértices → coordenadas (pack.vertices.p[vertex])
   b. Simplificar con Ramer-Douglas-Peucker (tolerancia 0.3px)
   c. Recortar a límites del mapa (clipPoly)
   d. Fractalizar costa (midpoint displacement)
   e. Generar path SVG (B-spline + Catmull-Rom mixto)
   f. Guardar como <path id="feature_N">
2. Máscaras:
   - #deftemp > #land: relleno blanco para tierra, negro para lagos → <mask id="land">
   - #deftemp > #water: relleno negro para tierra, blanco para lagos → <mask id="water">
3. Línea de costa (#coastline):
   - <g id="sea_island">: features tipo sea_island
   - <g id="lake_island">: features tipo lake_island
4. Lagos (#lakes): agrupados por feature.group
```

### `buildCoastlinePath()` — Construcción del path (`coastline-fractal.ts:194-252`)
- **Tramos suaves** (sin subdivisión entre vértices originales): B-spline Q midpoint (`curveBasisClosed`)
- **Tramos dentados** (subdivididos por fractal): Catmull-Rum centrípeto (α=0.5) por cada sub-punto

---

## 3. Fractalización de costa

**Archivo**: `src/renderers/coastline-fractal.ts`

### Parámetros
```typescript
const defaultCoastSettings = {
  enabled: true,
  maxDepth: 4,
  baseAmplitude: 1.5,
  amplitudeDecay: 0.9,
  minEdge: 1,
  smoothThreshold: 0.25,
  roughnessContrast: 1.5,
  profileHarmonics: 4,
  lakeSmoothThreshMult: 2.0
};
```

### Algoritmo
1. **PRNG determinista**: `Alea(seed + "_c" + featureIndex)` para cada feature
2. **Perfil de rugosidad**: Suma de cosenos armónicos (seam-free, `PROFILE_SIZE=256`)
3. **Midpoint displacement** recursivo: `subdivideEdge()` desplaza puntos medios perpendicularmente por `(rand()-0.5) * sqrt(edgeLength) * amplitude * roughness`
4. Los lagos reciben perfil más suave (`smoothThreshold * 2.0`)
5. Bordes en el límite del mapa nunca se subdividen

### Efecto visual
La combinación de B-spline (tramos suaves) + Catmull-Rom (tramos fractales) produce costas fluidas que ocultan la angularidad del Voronoi, manteniendo detalle en zonas rugosas.

---

## 4. Océano: base y batimétrica

### 4a. Océano base
**Archivo**: `index.html:326-328` — SVG `<pattern id="oceanic">`

- Relleno base en SVG: `#466eab`
- Patrón opcional: imagen superpuesta (6 opciones + Kiwiroo)
- Controles de estilo: color hex configurable, opacidad de patrón (0-1)

### 4b. Capas batimétricas
**Archivo**: `src/renderers/ocean-layers.ts:67-109` — `OceanModule.draw()`

Dibuja anillos de contorno concéntricos en el océano a niveles de profundidad:
```typescript
const outline = this.oceanLayers.attr("layers");
// Valores: "none" | "random" | "-6,-3,-1" (default) | "-9,-6,-3,-1" | etc.
```

**Algoritmo**:
1. Parsear el atributo `layers` para límites de profundidad (ej. `-6, -3, -1`)
2. Para cada límite `t`, encontrar celdas donde `grid.cells.t[i] === t`
3. Caminar vértices con `connectVertices()` → cadenas de contorno cerradas
4. **Relajar**: filtrar cada N-ésimo punto (`N = 1 + abs(t) * -2`)
5. Recortar a límites del mapa
6. Renderizar como `<path>` SVG con fill `#ecf2f9` y `fill-opacity = 0.4 / numLimits`

**Presets**: "No outline", "Random", "Standard 3" (-6,-3,-1), "Indented 3" (-6,-4,-2), "Smooth 6", "Smooth 9"

---

## 5. Motor de isolíneas compartido

**Archivo**: `src/utils/pathUtils.ts:84-177` — `getIsolines()`

**Es el algoritmo de renderizado más crítico**. Genera paths SVG de relleno para cualquier atributo indexado por celda. Todas las capas de color regional (biomas, culturas, religiones, estados, provincias) comparten este mismo motor.

### Algoritmo
```
1. Iterar sobre todas las celdas; saltar celdas ya procesadas o tipo null
2. Para cada celda no procesada de tipo T:
   a. Encontrar un vecino de tipo diferente
   b. Saltar si es un lago interno (todos los vecinos de costa son mismo tipo)
   c. Encontrar vértice inicial en el borde entre T y no-T
   d. Caminar vértices con connectVertices() → polígono cerrado
3. Para cada cadena de vértices, generar hasta 4 formatos de path:
   - fill: "M x0,y0 L x1,y1 ... Z" — polígono de relleno sólido
   - waterGap: path con discontinuidades en vértices de costa → evita que el color se desborde al océano
   - halo: path con discontinuidades en bordes del mapa → para efectos de glow
   - polygons: arrays de coordenadas crudos (para cálculo de polo de inaccesibilidad)
```

### `connectVertices()` — El caminador de vértices fundamental
```typescript
function connectVertices({ vertices, startingVertex, ofSameType, addToChecked, closeRing }) {
  // Sigue la teselación Voronoi: cada vértice conecta 3 celdas
  // En cada vértice, revisa qué celdas adyacentes son "del mismo tipo"
  // Sigue la arista entre celdas del mismo tipo y diferente tipo
  // Termina cuando vuelve al startingVertex
}
```

**Este caminador es la base de**:
- `getIsolines()` → todos los rellenos regionales
- `drawBorders()` → bordes de estado/provincia
- `drawHeightmap()` → líneas de contorno
- `OceanLayers.draw()` → contornos batimétricos
- `drawTemperature()` → líneas de isoterma

---

## 6. Heightmap (contornos de elevación)

**Archivo**: `src/renderers/draw-heightmap.ts:51-196` — `heightmapRenderer()`

### Datos que consume
- `grid.cells.h` — alturas (0-100, ≥20 = tierra)
- `grid.vertices` — posiciones y conectividad de vértices
- Dos grupos SVG: `#oceanHeights` y `#landHeights` dentro de `#terrs`

### Algoritmo
```
1. Ordenar todas las celdas por altura ascendente
2. Contornos de océano (alturas 0-19):
   - Caminar líneas de contorno en cada nivel, saltando por skip
3. Contornos de tierra (alturas 20-100):
   - Caminar líneas de contorno desde altura 20
   - Saltar por skip configurable
4. Para cada banda de altura: usar connectVertices()
5. Simplificar saltando cada N-ésimo vértice
6. Renderizar:
   - height === 0: rect fill océano base, color = scheme(1.0)
   - height === 20: rect fill tierra base, color = scheme(0.8)
   - Para cada height con path: fill = scheme(1 - height/100)
   - Si terracing activo: translate(0.7, 1.4) + fill más oscuro
```

### Esquemas de color (`style.js`)
Presets: bright (Spectral), light (RdYlGn), natural, green, olive, livid, monochrome, o stops hex personalizados separados por coma.

### Controles de estilo
- Render ocean heights toggle
- Terracing power (0=off)
- Reduce layers (skip)
- Simplify line
- Line style (curveBasisClosed, linear, step)
- Color scheme

---

## 7. Biomas

**Archivo**: `public/modules/ui/layers.js:302-316` — `drawBiomes()`

### Datos que consume
- `pack.cells.biome[]` — biome ID por celda
- `biomesData.color[]` — color por biome ID
- `biomesData.i[]` — array de índices de bioma

### Algoritmo
```typescript
const isolines = getIsolines(pack, cellId => cells.biome[cellId], { fill: true, waterGap: true });
Object.entries(isolines).forEach(([index, { fill, waterGap }]) => {
  const color = biomesData.color[index];
  bodyPaths.push(getGappedFillPaths("biome", fill, waterGap, color, index));
});
```

Usa el motor de isolíneas con `{fill: true, waterGap: true}`. `getGappedFillPaths()` genera:
- `<path d="{fill}" fill="{color}" id="biome{index}">` — relleno
- `<path d="{waterGap}" fill="none" stroke="{color}" stroke-width="3" id="biome-gap{index}">` — borde contra agua

---

## 8. Culturas

**Archivo**: `public/modules/ui/layers.js:480-494` — `drawCultures()`

### Datos que consume
- `pack.cells.culture[]` — culture ID por celda
- `pack.cultures[].color` — color por cultura

### Algoritmo
Idéntico a biomas: `getIsolines(pack, cellId => cells.culture[cellId], { fill: true, waterGap: true })`, luego relleno con `cultures[index].color`.

---

## 9. Religiones

**Archivo**: `public/modules/ui/layers.js:509-523` — `drawReligions()`

### Datos que consume
- `pack.cells.religion[]` — religion ID por celda
- `pack.religions[].color` — color por religión

### Algoritmo
Idéntico a culturas: `getIsolines(pack, cellId => cells.religion[cellId], { fill: true, waterGap: true })`.

---

## 10. Estados

**Archivo**: `public/modules/ui/layers.js:537-566` — `drawStates()`

### Datos que consume
- `pack.cells.state[]` — state ID por celda
- `pack.states[].color` — color por estado

### Algoritmo
```typescript
const isolines = getIsolines(pack, cellId => cells.state[cellId], { fill: true, waterGap: true, halo: renderHalo });
Object.entries(isolines).forEach(([index, { fill, waterGap, halo }]) => {
  bodyPaths.push(getGappedFillPaths("state", fill, waterGap, color, index));
  if (renderHalo) {
    haloPaths.push(`<path id="state-border${index}" d="${halo}" clip-path="url(#state-clip${index})" stroke="${darkerColor}"/>`);
  }
});
```

- **Water gap**: stroke donde el borde del estado toca agua
- **Halo**: cuando shape-rendering es "geometricPrecision", renderiza un borde borroso debajo del estado (grupo `#statesHalo` con `filter="blur(5px)"`)

---

## 11. Provincias

**Archivo**: `public/modules/ui/layers.js:592-617` — `drawProvinces()`

### Datos que consume
- `pack.cells.province[]` — province ID por celda
- `pack.provinces[].color` — color
- `pack.provinces[].pole` o `pack.cells.p[province.center]` — posición de etiqueta

### Algoritmo
- Usa motor de isolíneas con `{fill: true, waterGap: true}`
- Renderiza etiquetas de provincia como SVG `<text>` en grupo `#provinceLabels`
- Las etiquetas se colocan en el `pole` (polo de inaccesibilidad) o en la celda central

---

## 12. Bordes (fronteras)

**Archivo**: `src/renderers/draw-borders.ts:7-165` — `bordersRenderer()`

### Datos que consume
- `pack.cells.state[]`, `pack.cells.province[]`
- `pack.cells.h[]` (≥20 = tierra)
- `pack.cells.v[]`, `pack.cells.c[]`
- `pack.vertices.c[]`, `pack.vertices.v[]`, `pack.vertices.p[]`

### Algoritmo (NO usa el motor de isolíneas — es un renderizador especializado de líneas)
```
1. Para cada celda con provincia:
   a. Si algún vecino tiene provincia diferente pero mismo estado → borde provincial
   b. Caminar vértices con getVerticesLine() → path de borde
2. Para cada celda con estado:
   a. Si algún vecino de tierra (h≥20) tiene estado diferente → borde estatal
3. getVerticesLine():
   - Dos pasadas: primera encuentra arista de borde, segunda traza el borde completo
   - Sigue aristas entre celdas de diferente tipo
   - Usa vertices.v[current] (3 vértices vecinos) para determinar el próximo paso
```

### Estilos (`auto-update.js:41-52`)
| Borde | Opacidad | Stroke | Width | Dasharray |
|-------|----------|--------|-------|-----------|
| Estado | 0.8 | `#56566d` | 1 | `"2"` |
| Provincia | 0.8 | `#56566d` | 0.5 | `"1"` |

---

## 13. Ríos

**Ver también**: `docs/analisis-rios-azgaar.md` (análisis exhaustivo de 977 líneas)

### Archivos
- `public/modules/ui/layers.js:810-831` — `drawRivers()`
- `src/generators/river-generator.ts:425-456` — `RiverModule.getRiverPath()`
- `src/generators/river-generator.ts:369-387` — `RiverModule.addMeandering()`
- `src/generators/river-generator.ts:400-418` — `RiverModule.getOffset()`

### Algoritmo de dibujo
1. **Meandering**: `addMeandering()` interpola entre centros de celda con meander factor 0.5 (más meandro upstream, menos en agua: `WATER_MEANDER_SCALE = 0.25`)
2. **Path generation**: Para cada punto, calcula offset (ancho) basado en flujo + posición → dos orillas (left bank, right bank)
3. **Offset/Width**: `FLUX_FACTOR = 500`, `MAX_FLUX_WIDTH = 1`, `LENGTH_STEP_WIDTH = 1/200`, progresión Fibonacci
4. **Renderizado**: polígono cerrado SVG con `curveCatmullRom.alpha(0.1)` en ambas orillas

---

## 14. Rutas (caminos, senderos, rutas marítimas)

**Archivo**: `public/modules/ui/layers.js:845-862` — `drawRoutes()`
**Archivo**: `src/generators/routes-generator.ts:867-873` — `RoutesModule.getPath()`

### Datos que consume
- `pack.routes[]` — array con `i`, `group` ("roads"|"trails"|"searoutes"), `points[]`

### Algoritmo
```typescript
getPath({ group, points }) {
  const curve = {
    roads: curveCatmullRom.alpha(0.1),
    trails: curveCatmullRom.alpha(0.1),
    searoutes: curveCatmullRom.alpha(0.5)
  };
  return round(lineGen(points.map(p => [p[0], p[1]])));
}
```

Renderizado como `<path>` SVG con `fill="none"`, agrupado por tipo en `#roads`, `#trails`, `#searoutes`.

---

## 15. Íconos de relieve

**Archivo**: `src/renderers/draw-relief-icons.ts:17-148` — `reliefIconsRenderer()`

### Datos que consume
- `pack.cells.h[]`, `pack.cells.r[]`, `pack.cells.biome[]`
- `grid.cells.temp[]` (para snowline)
- `biomesData.iconsDensity[]`, `biomesData.icons[][]`

### Algoritmo
```
1. Para cada celda de tierra (h≥20) sin río:
   a. Lowlands (h < 50): biome icons via poisson-disc sampling
      - Densidad según biomesData.iconsDensity[biome]
      - Tipo: conifer, coniferSnow, swamp, cactus, deadTree
   b. Highlands (h ≥ 50): relief icons
      - h > 70 + temp < 0: montañas con nieve
      - h > 70: montañas
      - otherwise: colinas
      - Size escala con altura
2. Poisson-disc sampling evita solapamiento
3. Íconos ordenados por y + size (painter's algorithm)
4. Render: <use href="#relief-{type}-{variant}" x y width height>
```

### Parámetros clave
```typescript
const density = terrain.attr("density") || 0.4; // 0.3-0.8
const size = 2 * (terrain.attr("size") || 1); // 0.2-4.0
```

### Sets de íconos: "simple" (minimal), "gray", "colored" (más detallado)

---

## 16. Burgos (asentamientos)

**Archivo**: `src/renderers/draw-burg-icons.ts:10-115` — `burgIconsRenderer()`

### Datos que consume
- `pack.burgs[]` — `i`, `x`, `y`, `group`, `port`, `removed`
- `options.burgs.groups[]` — grupos ordenados que definen jerarquía

### Algoritmo
1. Crear grupos de íconos ordenados por jerarquía
2. Para cada burgo: `<use href="#icon-{shape}" id="burg{i}" x="{x}" y="{y}">`
3. Si `burg.port`: también ícono de ancla en misma posición
4. Formas: circle, square, triangle, cross, star, circled, squared, star-circled, star-squared, y variantes Watabou (capital, city, town, village, hamlet, fort, monastery, caravanserai, trade post)

---

## 17. Etiquetas de estado (texto curvo)

**Archivo**: `src/renderers/draw-state-labels.ts:25-373` — `stateLabelsRenderer()`

### Algoritmo (raycasting complejo)
```
1. Desde el pole de cada estado, emitir rayos cada 9° hacia afuera
2. Para cada rayo, avanzar de 5px en 5px hasta salir del estado
3. Encontrar el mejor par de rayos (izquierdo + derecho) que:
   - Sean suficientemente largos para el nombre del estado
   - Preferir horizontales o casi-horizontales
   - Preferir ángulos obtusos (180° = recta = mejor)
   - Score = longitud × horizontalidad × curvatura
4. Conectar los dos endpoints a través del pole con curveNatural
5. SVG <text><textPath> a lo largo del path generado
6. Validación: bounding box dentro del estado (6 puntos de muestra rotados)
7. Fallback a nombre corto si no cabe el completo
```

### Constantes clave
```typescript
ANGLE_STEP = 9;        // grados entre rayos
LENGTH_START = 5;      // paso inicial
LENGTH_STEP = 5;       // incremento
LENGTH_MAX = 300;      // longitud máxima
```

---

## 18. Etiquetas de burgo

**Archivo**: `src/renderers/draw-burg-labels.ts:10-91` — `burgLabelsRenderer()`

SVG `<text>` simple en coordenadas del burgo con offset (`dx`, `dy` en em). Agrupado por grupo de burgo, cada uno con estilo independiente.

---

## 19. Temperatura (isotermas)

**Archivo**: `src/renderers/draw-temperature.ts:19-136` — `temperatureRenderer()`

### Algoritmo
1. Calcular min/max temperatura y auto-determinar step size
2. Para cada nivel de isoterma, caminar vértices → isolíneas
3. Relajar: mantener cada 4º vértice (o bordes)
4. Cada banda se rellena con escala Spectral: `fill = scheme(1 - (t - tMin) / delta)` donde scheme = `interpolateSpectral`
5. Stroke: `darker(fill, 0.2)`
6. Etiquetas en centro-superior e centro-inferior de cada banda

---

## 20. Precipitación

**Archivo**: `public/modules/ui/layers.js:333-359` — `drawPrecipitation()`

### Algoritmo
- Por cada celda de tierra con datos de precipitación: círculo centrado en la celda
- Radio = `sqrt(prec/4) / cellsModifier`
- Animación de aparición (800ms transition de r=0 al radio calculado)

---

## 21. Población

**Archivo**: `public/modules/ui/layers.js:394-432` — `drawPopulation()`

### Algoritmo
- **Rural**: Líneas verticales desde el centro de la celda hacia abajo, largo proporcional a la población
- **Urbano**: Líneas verticales desde el centro del burgo hacia abajo, largo proporcional a población × urbanización
- Animación 2000ms transition

---

## 22. Capa de hielo

**Archivo**: `src/renderers/draw-ice.ts:10-74` — `iceRenderer()`

### Datos que consume
- `pack.ice[]` — array de Ice con `type` ("glacier"|"iceberg"), `points`, `offset`

Renderizado como SVG `<polygon>` con los puntos almacenados. Glaciares/icebergs individuales redibujables via `redrawIceberg()` y `redrawGlacier()`.

---

## 23. Bienes (goods)

**Archivo**: `src/renderers/draw-goods.ts:31-168` — `drawGoods()`

Tres sub-capas:
- **`goodsCells`**: Polígonos de celda coloreados por tipo de producción, opacidad normalizada al máximo global
- **`goodsIcons`**: Íconos de bien a nivel de celda (símbolos SVG) con círculos opcionales
- **`goodsBurgs`**: Placas de burgo — rectángulos redondeados con top-3 bienes producidos + valores

---

## 24. Mercados

**Archivo**: `src/renderers/draw-markets.ts:17-118` — `drawMarketsLayer()`

- Cada mercado recibe su zona de influencia (isoline polygon) rellena con el color del mercado
- El burgo central recibe un círculo con emoji (default ⚖️)
- Animación hover highlight

---

## 25. Emblemas (escudos)

**Archivo**: `src/renderers/draw-emblems.ts:28-156` — `emblemsRenderer()`

- Escudos de burgo, provincia y estado renderizados como SVG `<use>`
- Tamaños auto-calculados según número de entidades y dimensiones del mapa
- Simulación D3 force (`forceCollide`) para separar emblemas solapados

---

## 26. Militar

**Archivo**: `src/renderers/draw-military.ts:13-169`

Regimientos como rectángulos coloreados con conteo de tropas, íconos de unidad e imágenes. Coloreados por estado con bordes más oscuros. Movimiento animado a lo largo de paths.

---

## 27. Textura satelital (3D)

**Archivo**: `src/renderers/draw-satellite-texture.ts:474-580` — `generateSatelliteTexture()`

**Ruta de renderizado completamente separada** para la vista 3D (Three.js/WebGL). Usa shaders GLSL personalizados en un fullscreen quad.

### Datos que consume
- Altura con erosión + datos de costa
- Datos climáticos (temp, prec, height)
- Biomas por celda

### Pipeline del fragment shader
1. **Texturas de entrada**: Clima (temp+128/R, prec/G, height/B), Bioma (albedo RGB, densidad A)
2. **Cálculos por píxel**:
   - **Slope** por diferencias centrales en height field
   - **Dithering noise** (FBM 5-octave value noise)
   - **Albedo de bioma** con UV wobble para variación natural de borde
   - **Roca** en pendientes pronunciadas, bandas de estrato, oscurecimiento de acantilados
   - **Arena/grava** en playas costeras
   - **Nieve** basada en temperatura + altitud
   - **Riparian darkening** cerca de drenajes
   - **Cavity shading** (gullies oscuro, crestas brillante)
   - **Hillshade** (estilo Swiss-relief: sol NW cálido, sombra azul fría)
   - **Perspectiva aérea** (tierras altas pálidas hacia el cielo)
   - **Agua**: bathymetric shelf-to-abyss, lagunas teñidas por clima, foam line
   - **Lagos**: por grupo (fresh/salt/sinkhole/dry/lava/frozen)
   - **Ríos**: canal teal profundo, bancos de sedimento, white water en pendientes, hielo en climas fríos

---

## 28. Overlays: textura, grilla, coordenadas

### 28a. Textura decorativa
**Archivo**: `public/modules/ui/layers.js:783-796` — `drawTexture()`

```javascript
function drawTexture() {
  texture.append("image")
    .attr("preserveAspectRatio", "xMidYMid slice")
    .attr("x", x).attr("y", y)
    .attr("width", graphWidth - x).attr("height", graphHeight - y)
    .attr("href", href);
}
```

### 28b. Grilla
**Archivo**: `public/modules/ui/layers.js:632-659` — `drawGrid()`

- Patrón SVG (`<pattern>`) para tiles: pointyHex, flatHex, square, square45deg, triangle
- Escala, stroke, dash, shift configurables

### 28c. Coordenadas (graticule)
**Archivo**: `public/modules/ui/layers.js:673-731` — `drawCoordinates()`

- Usa `d3.geoGraticule()` + `d3.geoEquirectangular()`
- Step adaptable al zoom (steps posibles: 0.5, 1, 2, 5, 10, 15, 30)
- Etiquetas N/S/E/W en bordes del mapa

---

## 29. Patrones arquitectónicos clave

### Patrón 1: Motor de isolíneas universal
`getIsolines()` + `connectVertices()` son el **motor universal de renderizado regional**. Cualquier atributo indexado por celda produce fills SVG con el mismo algoritmo: agrupar celdas → caminar bordes Voronoi → generar polígonos cerrados → water gaps opcionales.

### Patrón 2: Water Gap
Cada región se renderiza con un **stroke adicional** (`waterGap`) del mismo color que el fill. Donde el borde regional toca agua o el borde del mapa, el stroke se interrumpe — evita que los colores se desborden visualmente al océano.

### Patrón 3: Máscaras de tierra/agua
Se generan dos máscaras SVG en `#deftemp`:
- `<mask id="land">`: blanco para tierra, negro para lagos
- `<mask id="water">`: negro para tierra, blanco para lagos
Usadas para clipping de capas que no deben salir de la tierra o del agua.

### Patrón 4: Fractalización de costa
Pipeline: RDP simplificación (0.3px) → clip a bounds → midpoint displacement fractal (4 niveles, seed por feature) → path mixto B-spline/Catmull-Rom.

### Patrón 5: Sin relleno de tierra explícito
No hay un "fill tierra" global. En cambio:
- La capa base de tierra es el rect fill del heightmap en altura 20
- Las capas temáticas renderizan **polígonos regionales individuales** que cubren toda la tierra colectivamente
- La máscara `#land` existe para clipping, no como fill visual

---

## 30. Estado de portabilidad a Voronia

| Capa | Voronia actual | Dependencias | Prioridad |
|------|---------------|-------------|-----------|
| **Features** (tierra/océano/costa) | ❌ No implementado | Geometría Voronoi, masks | **Alta** |
| **Coastline fractal** | ❌ No implementado | Midpoint displacement, Catmull-Rom | **Alta** |
| **Ocean base** | ❌ Color fijo `[0.20, 0.45, 0.80]` | Color configurable, patrón | Media |
| **Ocean bathymetric** | ❌ No implementado | connectVertices(), contornos | Baja |
| **Heightmap contours** | ✅ Mesh base OK. Contornos ❌ | connectVertices(), esquemas de color | Media |
| **Biomes** | ❌ No implementado | getIsolines(), waterGap | **Alta** |
| **Cultures** | ❌ No implementado | getIsolines(), waterGap | **Alta** |
| **Religions** | ❌ No implementado | getIsolines(), waterGap | **Alta** |
| **States** | ❌ No implementado | getIsolines(), waterGap, halo | **Alta** |
| **Provinces** | ❌ No implementado | getIsolines(), waterGap, labels | **Alta** |
| **Borders** | ❌ No implementado | getVerticesLine(), dashed strokes | **Alta** |
| **Rivers** | ✅ Implementado en vor-sim + vor-render | — | ✅ Hecho |
| **Routes** | ❌ No implementado | Catmull-Rom paths | Media |
| **Relief icons** | ❌ No implementado | Poisson-disc, sprites, biomas | Baja |
| **Burg icons** | ❌ No implementado | Sprites, jerarquía | Media |
| **State labels** | ❌ No implementado | Raycasting, textPath curvo | Media |
| **Burg labels** | ❌ No implementado | SVG text, offset | Media |
| **Temperature** | ❌ No implementado | Isolineas, Spectral scale | Baja |
| **Precipitation** | ❌ No implementado | Círculos, radio por prec | Baja |
| **Population** | ❌ No implementado | Barras verticales | Baja |
| **Ice** | ❌ No implementado | Polígonos | Baja |
| **Goods** | ❌ No implementado | Múltiples sub-capas | Baja |
| **Markets** | ❌ No implementado | Isolineas, emoji | Baja |
| **Emblems** | ❌ No implementado | D3 force, SVG use | Baja |
| **Military** | ❌ No implementado | Rectángulos, animación | Baja |
| **Satellite texture** | ❌ No implementado | Shaders GLSL, full pipeline | Muy baja |
| **Texture overlay** | ❌ Parcial (carga textura OK, blend no) | Fullscreen quad, blend modes | Baja |
| **Grid overlay** | ❌ No implementado | Patrones, shader líneas | Baja |
| **Coordinates** | ❌ No implementado | Proyección geográfica | Media |

### Dependencias críticas (orden de implementación)
1. **`connectVertices()`** + **`getIsolines()`** — prerrequisito para TODAS las capas temáticas (biomas, culturas, religiones, estados, provincias, borders, heightmap contours)
2. Máscaras land/water — clipping de capas temáticas
3. Coastline fractal — calidad visual de la costa
4. Paleta de colores configurable por capa — esquemas estilo Azgaar
5. Water gap technique — para que las capas temáticas no sangren al océano

### Nota sobre el motor de isolíneas
El `connectVertices()` de Azgaar es un algoritmo que camina sobre la teselación Voronoi siguiendo aristas entre celdas de diferente tipo. **No requiere la geometría Delaunay** — solo necesita:
- `cells.c[i]` — vecinos de cada celda
- `cells.v[i]` — vértices de cada celda
- `vertices.c[v]` — celdas que comparten un vértice
- `vertices.v[v]` — vecinos de cada vértice (3 en Voronoi estándar)

Todo esto ya está disponible en `vor-core` (PackCells).

---

> **Documento generado a partir del análisis del código fuente de Azgaar FMG v1.135.2**.
> Para el análisis de ríos, ver `docs/analisis-rios-azgaar.md` (977 líneas).
> Para documentación previa de capas de tierra, ver `docs/landmass-layers.md` (análisis inicial, menos detallado).
