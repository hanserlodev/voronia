# Voronia — Plan de Proyecto

> Motor nativo de generación y edición de mundos de fantasía, acelerado por GPU. Reescrito desde cero como producto original y de código abierto, basado en la lógica procedural de **Azgaar's Fantasy Map Generator** (MIT) pero con arquitectura, renderer y formato de datos propios.

| | |
|---|---|
| **Estado** | Planificación inicial |
| **Versión del documento** | 1.0 |
| **Fecha** | 23 julio 2026 |
| **Nombre de proyecto** | `Voronia` (ver §1.3 para justificación y verificación) |
| **Basado en investigación de** | repo, wiki y código público de Azgaar/Fantasy-Map-Generator (verificado jul 2026) |

---

## Resumen ejecutivo

Azgaar's Fantasy Map Generator es una app web (JS/TS) que genera y edita mapas de fantasía de forma procedural: terreno, ríos, clima, biomas, culturas, estados, religiones, rutas, economía y milicia, todo sobre una malla de Voronoi. Es open source (MIT), pero está limitada por su renderer SVG+DOM, que no escala bien en mapas grandes (confirmado por la propia comunidad: hubo un intento de versión Electron que se descartó porque no resolvía el cuello de botella real).

Este proyecto **no es un port ni un wrapper de Azgaar**. Es un motor nativo (Rust + wgpu) que:
1. Puede **importar** datos generados por Azgaar (`.map` / JSON export).
2. Convierte esos datos a un **formato binario propio optimizado para GPU** (`.gmap`).
3. Renderiza con aceleración por hardware real (no SVG/DOM).
4. Eventualmente **reimplementa y expande** toda la lógica procedural de Azgaar de forma nativa, para dejar de depender de Azgaar por completo.
5. Se mantiene **de código abierto**, con atribución correcta a Azgaar por la lógica/algoritmos de referencia.

Un hallazgo clave de esta investigación (§3) cambia el enfoque técnico del parser: **el `.map`/JSON de Azgaar no guarda la geometría del mapa** (posiciones de celdas, vecinos, vértices) — la regenera en cada carga a partir de una semilla. Esto significa que "leer un mapa de Azgaar" no es un parseo pasivo: requiere **reimplementar con precisión bit-exacta el algoritmo de generación de la grilla y el diagrama de Voronoi/Delaunay** para que los datos guardados (altura, bioma, cultura, etc., indexados por celda) calcen con la geometría correcta.

---

## Índice

1. Visión y objetivos
2. Relación con Azgaar's Fantasy Map Generator
3. ⚠️ Hallazgo crítico: cómo funciona realmente el formato de Azgaar
4. Principios de diseño
5. Arquitectura general del sistema
6. Stack tecnológico
7. Modelo de datos del mundo (World Data Model)
8. Formatos de archivo
9. Motor de renderizado GPU
10. Motor de simulación procedural
11. Edición de mapas
12. UI nativa
13. Estructura del repositorio (Cargo workspace)
14. Gestión de estado, rendimiento y concurrencia
15. Persistencia, autosave y recuperación
16. Testing y QA
17. CI/CD y builds multiplataforma
18. Empaquetado y distribución
19. Documentación
20. Código abierto: licencia, gobernanza y comunidad
21. Extensiones más allá de Azgaar
22. Integración futura con Atenea
23. Roadmap por fases (con checklists)
24. Métricas de éxito / objetivos de rendimiento
25. Riesgos y mitigaciones
26. Decisiones pendientes
27. Flujo de trabajo con Claude Code
28. Glosario
29. Referencias

---

## 1. Visión y objetivos

### 1.1 Resumen

Construir un motor de mundos de fantasía nativo, rápido y extensible — inspirado en Azgaar pero no limitado por sus decisiones técnicas de 2017 (SVG, DOM, single-thread JS) — que además pueda crecer hacia un **World Engine** completo integrable con Atenea (asistente IA local de Hans).

### 1.2 Objetivos

- [ ] Renderizar mapas de decenas/cientos de miles de celdas a 60 FPS con pan/zoom fluido.
- [ ] Leer mapas existentes de Azgaar (`.map` y JSON export) con fidelidad geométrica.
- [ ] Definir un formato binario propio (`.gmap`) como fuente de verdad interna, rápido de cargar/guardar.
- [ ] Reimplementar (progresivamente) toda la lógica procedural de Azgaar en Rust.
- [ ] Ofrecer edición interactiva (terreno, entidades, fronteras) con undo/redo real.
- [ ] Expandir capacidades más allá de lo que Azgaar permite hoy (ver §21).
- [ ] Mantener el proyecto open source, con atribución correcta.
- [ ] Sentar las bases para integrarse como "motor de mundo" de Atenea.

### 1.3 Nombre del proyecto: Voronia

Los candidatos evaluados en la primera versión de este documento (`worldforge`, `terraforge`, `mapsmith`, etc.) se descartaron tras verificar colisiones reales:

| Candidato descartado | Por qué |
|---|---|
| `Worldforge` | Colisiona fuerte: un framework MMORPG open source activo desde 1998 (worldforge.org, motor Ogre3D), una wiki de worldbuilding para escritores (worldforge.me) y una herramienta de worldbuilding en itch.io — los tres usan el nombre casi literal. |
| `Terraforge` | Colisiona con **TerraForge3D**, una herramienta open source real de generación procedural de terreno (alternativa a World Machine/Gaea). Justo el mismo espacio de producto. |
| `Tessera` (evaluado como alternativa) | Colisiona con un asset de Unity muy usado (Tessera Procedural Tile Generator), un paper académico de WFC llamado igual, y un motor de juego llamado "Tessera Engine" lanzado en 2026. |
| `Graticule` (evaluado como alternativa) | Es un término cartográfico real y además ya lo usan media docena de productos de software (geocoding, GIS, una empresa de datos de salud). Demasiado saturado. |

**Nombre elegido: `Voronia`.**

- Verificado sin colisiones directas como nombre de software/proyecto (búsquedas específicas no arrojaron ningún producto, librería o repo con ese nombre exacto).
- Es un nombre genuinamente propio, no un compuesto obvio en inglés (evita el problema de fondo: "World/Terra/Map + Forge/Craft/Smith" es la primera idea que se le ocurre a todo el mundo en este rubro, por eso choca tanto).
- Doble lectura intencional: para cualquiera que conozca el dominio técnico, remite directo a **Voronoi**, el diagrama geométrico que es el corazón de toda la malla del mundo (§7.1) — coherente con el nombre desde el nivel más profundo de la arquitectura. Para cualquier otra persona, simplemente suena a topónimo de fantasía (mismo patrón que Narnia, Sonoria, etc.), que encaja con el dominio de mapas de fantasía.
- Corto, fácil de pronunciar en español e inglés, buen candidato de crate (`voronia`) y de nombre de repo.

Alternativa de respaldo si en algún momento no convence: `Mundrift` (nombre inventado desde cero, sin ningún hit en las búsquedas realizadas — más neutro, menos cargado de significado técnico).

> Nota honesta: ninguna búsqueda web es 100% exhaustiva como chequeo de trademark. Antes de registrar el repo en GitHub y publicar el primer crate en crates.io, vale la pena una verificación final de disponibilidad exacta en esas dos plataformas puntuales (dos minutos, cero fricción, y cierra cualquier duda residual).

### 1.4 No-objetivos (fuera de alcance v1)

- No es un clon 1:1 pixel-perfect de la UI de Azgaar.
- No se persigue compatibilidad binaria retroactiva con **todas** las versiones históricas de `.map` (solo el formato JSON/`.map` actual; versiones muy antiguas quedan fuera salvo que se pida explícitamente).
- No se implementa multiplayer/colaboración en tiempo real en v1 (queda como extensión, §21).
- No se apunta a mobile/tablet en v1 (desktop Linux/Windows/Mac).
- No se reimplementa Armoria (generador de escudos) ni el Medieval Fantasy City Generator de Watabou — se listan como integraciones externas opcionales (§21).

### 1.5 Público objetivo y casos de uso

- Uso personal de Hans para worldbuilding (ligado a Atenea).
- Comunidad de worldbuilders, DMs de TTRPG, escritores de fantasía — mismo público que Azgaar.
- Potencial público técnico: gente que quiera un motor de mundos embebible en sus propios juegos/herramientas.

---

## 2. Relación con Azgaar's Fantasy Map Generator

### 2.1 Qué se reutiliza

- **Los conceptos y algoritmos de referencia** que el propio Azgaar cita como inspiración (ver §29): generación de terreno de Martin O'Leary, generación de mapas poligonales (Voronoi/Delaunay) de Amit Patel, y el enfoque de Scott Turner ("Here Dragons Abound").
- **El modelo conceptual de datos** (grid/pack, cells, features, culturas, estados, burgos, religiones, ríos, rutas, zonas, biomas, namebases) — descrito con precisión en §7, tomado de la wiki oficial de Azgaar.
- **La compatibilidad de entrada**: poder importar mapas ya generados en Azgaar.

### 2.2 Qué se reescribe por completo

- Todo el código: 0% JS/TS de Azgaar se reutiliza literalmente. Se reimplementa en Rust desde cero.
- El renderer: de SVG/DOM a pipeline GPU (wgpu).
- El formato de persistencia interno: de `.map` (texto custom + SVG embebido) a `.gmap` (binario propio).
- La UI: de HTML/CSS a UI nativa inmediata (egui, ver §12).

### 2.3 Licencia y atribución

- Azgaar's Fantasy Map Generator está bajo **licencia MIT** (copyright Max Haniyeu / Azgaar, 2017–2024). El propio proyecto aclara que los mapas, capturas y obras derivadas generadas *con* la herramienta no están restringidos por la licencia y pueden usarse comercialmente — pero eso aplica a los *mapas generados*, no al código fuente en sí.
- MIT permite reimplementar, adaptar y redistribuir con la única condición de mantener el aviso de copyright y de licencia si se reutiliza código literal. Como aquí **no se copia código**, sino que se reimplementan algoritmos/lógica desde cero, no hay obligación legal de licencia — pero **sí es correcto y honesto** dar atribución explícita.
- Acción concreta: el `README.md` de Voronia debe incluir una sección "Créditos e inspiración" mencionando a Azgaar, con enlace al repo original y a los tres artículos de referencia (§29). Esto también es simplemente buena práctica de comunidad open source.
- Voronia se licenciará también bajo **MIT** (recomendado): máxima compatibilidad, cero fricción para que otros lo adopten, coherente con el ecosistema del que parte.

### 2.4 Compatibilidad de datos

- El propio equipo de Azgaar declara en su guía de contribución que la arquitectura **futura** hacia la que están migrando separa: **datos del mundo**, **generación procedural**, **edición interactiva** y **renderizado**, en 4 capas: `world data (state)` → `generators (model)` → `editors (controllers)` → `renderer (view)`, con flujo `settings → generators → world data → renderer` y `UI → editors → world data → renderer`.
- Esto **valida directamente** la arquitectura de Voronia (§5): no estamos inventando una separación arbitraria, estamos yendo un paso más allá de hacia dónde el propio Azgaar quiere evolucionar, pero en una plataforma nativa.

---

## 3. ⚠️ Hallazgo crítico: cómo funciona realmente el formato de Azgaar

Esto es lo más importante que hay que internalizar antes de escribir una sola línea de parser.

Según la documentación oficial del modelo de datos de Azgaar:

- Azgaar mantiene dos objetos principales: **`grid`** (malla base, antes de "repacking") y **`pack`** (malla final, optimizada para la masa de tierra actual, después de "repacking").
- Ambos contienen **datos de Voronoi**: posiciones de puntos/celdas, vecinos, vértices.
- **Estos datos geométricos NO se guardan en el `.map` ni en el JSON export.** Se guardan solo en memoria durante la sesión y **se recalculan cada vez que se carga un mapa.**
- Lo que sí se guarda son los **atributos por celda** (altura, bioma, cultura, estado, población, río, flujo, etc.), indexados por un **número de índice de celda** — no por coordenadas.

### 3.1 Implicación técnica directa

Para que un array de atributos guardado (p. ej. `pack.cells.h` = alturas) tenga sentido al cargarlo, **la celda con índice `N` debe caer exactamente en la misma posición geográfica que tenía cuando Azgaar generó y guardó ese mapa.** Eso solo es posible si:

1. Se conoce la **semilla** (seed) y los parámetros (`cellsDesired`, dimensiones del canvas, `spacing`) usados en la generación original.
2. Se reimplementa, **bit-exacto**, el mismo algoritmo de:
   - Generador pseudoaleatorio con semilla (a determinar en Fase 0 cuál usa Azgaar internamente).
   - Colocación de puntos en grilla cuadrada "jitterizada".
   - Triangulación de Delaunay sobre esos puntos.
   - Derivación del diagrama de Voronoi (dual del Delaunay).
   - El proceso de "repacking" (grid → pack): cómo se recorta/optimiza la malla a la masa de tierra real.

Si cualquiera de estos pasos difiere aunque sea ligeramente del original, **los índices de celda no van a coincidir**, y cargar un `.map` real producirá un mapa con los atributos mal ubicados (montañas donde debería haber mar, etc.).

### 3.2 Qué significa esto para el roadmap

- **Fase 1 no es "escribir un parser JSON".** Es "portar el generador de grilla + Delaunay/Voronoi + repacking de Azgaar a Rust, con el mismo comportamiento determinista", y **recién después** parsear/aplicar los arrays de atributos sobre esa geometría regenerada.
- Esto se investiga a fondo en **Fase 0** (revisar el código fuente real de Azgaar — carpeta `src/` del repo — para identificar el PRNG exacto y el algoritmo de repacking).
- Buena noticia: esto es exactamente el tipo de lógica que además **queremos** reimplementar nativamente para poder generar mapas nuevos (no solo importar existentes), así que no es trabajo "desperdiciado" — es el corazón del motor de generación (§10).
- Mitigación de riesgo: si en Fase 0 se determina que la réplica bit-exacta es inviable o demasiado frágil (por ejemplo si cambia entre versiones de Azgaar), el plan B es soportar el **JSON export completo** (que incluye más contexto que el `.map` legacy) como única vía de importación soportada oficialmente, documentando claramente que la fidelidad geométrica depende de la versión de Azgaar usada para exportar.

---

## 4. Principios de diseño

1. **El dato manda, el render obedece.** El renderer nunca modifica el estado del mundo; solo lo visualiza (mismo principio que ya persigue el propio Azgaar en su arquitectura futura).
2. **Todo es determinista y reproducible.** Misma semilla + mismos parámetros = mismo mundo, siempre. Esto habilita testing, comparación de versiones, y compatibilidad de import.
3. **CPU para lógica, GPU para render masivo.** No todo se beneficia de GPU — ver §5.3 para el desglose explícito.
4. **Incremental, no big-bang.** Cada fase entrega algo usable y demostrable (ver §23).
5. **El `.map` de Azgaar es un formato de intercambio, no el modelo interno.** Voronia nunca piensa "en términos de Azgaar" puertas adentro — solo en el import/export.
6. **Extensible desde el día uno.** El sistema procedural se diseña para poder agregar nuevos generadores sin tocar el core (ver §21.4, sistema de plugins).
7. **Todo dato serializable y versionado.** El formato `.gmap` lleva número de versión desde la v1 para poder evolucionar sin romper mapas guardados.

---

## 5. Arquitectura general del sistema

### 5.1 Vista de alto nivel

```text
                     AZGAAR .map / JSON export
                              │
                              ▼
                 ┌─────────────────────────┐
                 │   IMPORT / COMPAT LAYER  │
                 │  (regenera geometría +   │
                 │   mapea atributos, §3)   │
                 └────────────┬─────────────┘
                              │
                              ▼
                 ┌─────────────────────────┐
                 │    WORLD DATA MODEL      │  (§7)
                 │  grid / pack / cells /   │
                 │  features / rivers /     │
                 │  cultures / states /     │
                 │  burgs / religions /     │
                 │  routes / zones / ice    │
                 └────────────┬─────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
     SIMULATION ENGINE   EDIT ENGINE     RENDER ENGINE
        (§10)               (§11)           (§9)
              │               │               │
              └───────┬───────┴───────┬───────┘
                      ▼               ▼
                 .gmap (save)   GPU Framebuffer
                      │               │
                      ▼               ▼
                   Disco          Pantalla (winit/egui)
```

### 5.2 Flujo de datos (mirror del principio de Azgaar, adaptado)

```text
Import:   .map/JSON  → import layer → World Data Model
Generate: settings    → simulation engine → World Data Model
Edit:     UI input    → edit engine → World Data Model (mutación controlada)
Render:   World Data Model → render engine → GPU → pantalla
Save:     World Data Model → serializer → .gmap
Export:   World Data Model → exporters → PNG/SVG/GeoJSON/.map-compatible
```

El render **nunca** escribe de vuelta al World Data Model. Los editores sí, pero de forma controlada (comandos undo/redo-able).

### 5.3 Qué se beneficia de GPU y qué no (honestidad técnica)

| Sistema | Naturaleza | Se beneficia de GPU |
|---|---|---|
| Renderizado de geometría (celdas, ríos, fronteras) | Masivamente paralelo | ✅ Sí, muchísimo |
| Texto/labels | Paralelo con más trabajo | ✅ Parcial (glyph atlas) |
| Pan/zoom/culling | Paralelo | ✅ Sí |
| Generación de heightmap (ruido) | Paralelizable | ✅ Sí (compute shaders, fase avanzada) |
| Delaunay/Voronoi | Secuencial/grafos | ❌ Mayormente CPU |
| Hidrología (flujo de ríos) | Secuencial, depende de orden topológico | ❌ CPU |
| Expansión de culturas/estados/religiones | Tipo flood-fill con costos, iterativo | ❌ CPU (paralelizable entre sí con `rayon`, no dentro de un solo flood-fill) |
| Pathfinding (rutas) | Grafos (Dijkstra/A*) | ❌ CPU |
| Edición interactiva | Baja carga, interactiva | N/A |

Conclusión: la ganancia de GPU es principalmente en **renderizado**, no en generación procedural. Para generación, la ganancia real viene de **paralelismo multi-core en CPU** (`rayon`) aprovechando el i7-14650HX, no de la GPU.

---

## 6. Stack tecnológico

| Categoría | Elección | Por qué |
|---|---|---|
| Lenguaje | **Rust** | Seguridad de memoria sin GC, rendimiento nativo, gran ecosistema gráfico |
| Renderer GPU | **wgpu** | Abstrae Vulkan/Metal/DX12, portable, acceso directo a Vulkan en Linux/CachyOS |
| Ventana/input | **winit** | Estándar de facto junto a wgpu |
| UI inmediata | **egui** (+ `egui-wgpu`, `egui-winit`) | Se integra directo en el render pass de wgpu, ideal para paneles/inspectores sobre el canvas del mapa |
| Matemática gráfica | **glam** | Vectores/matrices rápidas, estándar en Rust gamedev |
| Triangulación de polígonos | **lyon** | Tesselation de paths/polígonos irregulares (celdas de Voronoi) para poder dibujarlos como triángulos |
| Delaunay/Voronoi | **delaunator** (puerto Rust de Delaunator.js) + derivación de Voronoi vía circuncentros | Es la misma familia de algoritmo que usan la mayoría de generadores de este estilo (a confirmar exactitud vs Azgaar en Fase 0) |
| Ruido procedural | **noise** | Perlin/Simplex/Worley para heightmap |
| Grafos | **petgraph** | Redes de ríos, adyacencia de celdas, pathfinding de rutas |
| RNG determinista | **rand + rand_pcg** (o similar seedable) | Reproducibilidad total con semilla |
| Serialización (import) | **serde + serde_json** | Parseo del JSON export de Azgaar |
| Serialización (formato propio) | **bincode** (v1) → evaluar **rkyv** (v2, zero-copy) | bincode es simple y maduro; rkyv da cargas casi instantáneas pero es más complejo — se arranca simple y se optimiza si hace falta |
| Texto en GPU | **glyphon** | Rendering de texto moderno sobre wgpu (labels de burgos, estados, ríos) |
| Paralelismo CPU | **rayon** | Paraleliza generación (múltiples culturas/estados creciendo a la vez, cálculo de clima por celda, etc.) |
| Imágenes/texturas | **image** | Carga de heightmaps externos, exportación a PNG |
| Errores | **anyhow + thiserror** | Manejo de errores idiomático |
| Logging | **tracing + tracing-subscriber** | Equivalente a SLF4J/Logback — logging estructurado |
| CLI | **clap** | Herramientas headless (generación batch, conversión de formatos sin UI) |
| Benchmarking | **criterion** | Detectar regresiones de rendimiento en CI |
| Async mínimo | **pollster** | Solo para bloquear en la inicialización async de wgpu |

> Nota: no se pinnean versiones exactas de crates en este documento — se usan las últimas estables al momento de iniciar cada fase (`cargo add` resuelve esto).

### 6.1 Decisión: ¿motor de juego (Bevy) o wgpu crudo?

**Recomendación: wgpu crudo, no Bevy**, al menos para v1. Bevy trae su propio ECS, su propio renderer abstraído y muchas convenciones que compiten con el control fino que este proyecto necesita (renderizado 2D masivo de geometría irregular, no es un juego 3D convencional). Queda como posible reevaluación en fases avanzadas si se necesita tooling de editor más sofisticado.

---

## 7. Modelo de datos del mundo (World Data Model)

Basado directamente en la documentación oficial del modelo de datos de Azgaar (wiki `Data-model`), reorganizado en structs Rust idiomáticos. Azgaar mismo describe su modelo actual como "pobremente definido e inconsistente" — acá se aprovecha para limpiar el diseño (usar `enum` en vez de strings mágicos, tipos fuertes en vez de números crudos) manteniendo la misma cobertura funcional.

### 7.1 Grid vs Pack

- **`Grid`**: malla inicial, puntos en grilla cuadrada "jitterizada", cantidad configurable (`cells_desired`: default 10.000, mínimo 1.000, máximo 100.000 en Azgaar — ver §24 para cómo esto informa nuestros objetivos de rendimiento).
- **`Pack`**: malla derivada de `Grid` tras el "repacking" — optimizada para la masa de tierra real (más densidad de celdas en zonas relevantes). La mayoría de la simulación (culturas, estados, ríos, etc.) opera sobre `Pack`, no sobre `Grid`.
- Ambas mallas se **regeneran siempre a partir de semilla + parámetros**, nunca se leen directamente del archivo (ver §3).

```rust
struct Grid {
    cells_desired: u32,
    spacing: f32,
    cells_x: u32,
    cells_y: u32,
    points: Vec<[f32; 2]>,
    cells: GridCells,
    vertices: VoronoiVertices,
    features: Vec<Feature>,
}

struct Pack {
    cells: PackCells,
    vertices: VoronoiVertices,
    features: Vec<Feature>,
}

struct VoronoiVertices {
    positions: Vec<[f32; 2]>,
    adjacent_cells: Vec<[i32; 3]>,
    adjacent_vertices: Vec<[i32; 3]>, // -1 si no tiene vecino (borde)
}
```

### 7.2 Cells (atributos por celda — esto SÍ se persiste)

Campos confirmados en el modelo real de Azgaar (`pack.cells.*`), traducidos a un struct de tipos fuertes en vez de arrays paralelos sueltos:

```rust
struct Cell {
    height: u8,            // 0-100, 20 = nivel mínimo de tierra
    feature_id: u32,       // isla/lago/océano al que pertenece
    distance_field: i8,    // distancia a costa: +N tierra, -N agua
    score: u16,            // qué tan buena es la celda para fundar un burgo
    biome: u8,
    burg: Option<u16>,
    culture: Option<u16>,
    state: Option<u16>,
    province: Option<u16>,
    religion: Option<u16>,
    area_px: u16,
    population: f32,       // en "puntos de población" (1 pt = 1000 hab. por defecto)
    river: Option<u16>,
    flux: u16,              // caudal de agua que pasa por la celda
    confluence_flux: u16,   // caudal en puntos donde confluyen ríos
    harbor_score: u8,       // cuántas celdas de agua son adyacentes
    haven_cell: Option<u32>,// celda de "puerto" para rutas
    routes: Vec<(u32, u32)>,// (celda destino, id de ruta)
}
```

### 7.3 Features (islas, lagos, océanos)

```rust
enum FeatureType { Ocean, Island, Lake }
enum LandGroup { Continent, Island, Isle, LakeIsland }
enum LakeGroup { Freshwater, Salt, Dry, Sinkhole, Lava }

struct Feature {
    id: u32,
    is_land: bool,       // height >= 20
    touches_border: bool,// distingue lago de océano
    kind: FeatureType,
    cell_count: u32,
    first_cell: u32,
    perimeter_vertices: Vec<u32>,
    name: Option<String>, // solo lagos
}
```

### 7.4 Cultures

```rust
enum CultureType { /* Generic, River, Lake, Naval, Nomadic, Hunting, Highland — confirmar set exacto en Fase 0 vía wiki "Culture types" */ }

struct Culture {
    id: u16,
    namebase_id: u16,
    name: String,
    origins: Vec<u16>,      // culturas de origen, para árbol evolutivo
    shield: String,
    center_cell: u32,
    code: String,           // abreviación
    color: Color,
    expansionism: f32,      // multiplicador de crecimiento
    kind: CultureType,
    area_px: u32,
    cells: u32,
    rural_pop: f32,
    urban_pop: f32,
    locked: bool,
    removed: bool,
}
```

### 7.5 Burgs (asentamientos)

```rust
struct Burg {
    id: u16,
    name: String,
    cell: u32,
    position: [f32; 2],
    culture: u16,
    state: u16,
    feature: u32,
    population: f32,
    kind: CultureType, // mismo enum que Culture.type
    coat_of_arms: Option<CoatOfArms>, // compatible con formato "Armoria" de Azgaar
    is_capital: bool,
    port_feature: Option<u32>,
    has_citadel: bool,
    has_plaza: bool,
    has_shanty: bool,
    has_temple: bool,
    has_walls: bool,
    locked: bool,
    removed: bool,
}
```

> Azgaar integra los burgos con **MFCG** (Medieval Fantasy City Generator de Watabou) vía una semilla derivada — generación de la ciudad a nivel calle/edificio. Esto queda fuera del alcance de Voronia v1, pero se documenta como posible integración externa (§21.5).

### 7.6 States (estados/países)

```rust
enum GovernmentForm { Monarchy, Republic, Theocracy, Union, Anarchy }

struct State {
    id: u16,
    name: String,
    form: GovernmentForm,
    full_name: String,
    color: Color,
    center_cell: u32,
    pole_of_inaccessibility: [f32; 2], // "centro visual" del polígono, técnica de Mapbox
    culture: u16,
    kind: CultureType,
    expansionism: f32,
    area_px: u32,
    burg_count: u32,
    cell_count: u32,
    rural_pop: f32,
    urban_pop: f32,
    neighbors: Vec<u16>,
    provinces: Vec<u16>,
    diplomacy: Vec<DiplomaticStatus>,
    campaigns: Vec<War>,
    war_alert: f32,
    military: Vec<Regiment>,
    coat_of_arms: Option<CoatOfArms>,
    locked: bool,
    removed: bool,
}

struct War { start_year: i32, end_year: Option<i32>, name: String }
struct Regiment {
    id: u16, position: [f32; 2], base_position: [f32; 2],
    angle_deg: f32, icon: char, origin_cell: u32, state: u16,
    name: String, is_separate_unit: bool, // navales, etc.
    composition: std::collections::HashMap<String, u32>,
}
```

### 7.7 Provinces, Religions, Rivers, Markers, Routes, Zones, Ice

Todos confirmados en el modelo real de Azgaar; se listan de forma compacta (estructura completa análoga a los ejemplos anteriores — detallar 1:1 en Fase 0/1):

- **Province**: subdivisión de un estado; tiene capital opcional, lista de burgos, área, población.
- **Religion**: `Folk | Organized | Heresy | Cult`; expansión `culture` (solo dentro de su cultura) o `global`; árbol de religiones (origins).
- **River**: `source_cell`, `mouth_cell`, `parent_river`, `basin_id`, `discharge_m3s`, `length_km`, `width_km` — unidades físicas reales, no arbitrarias.
- **Marker**: pin de punto de interés, ícono (emoji/unicode), tipo, estilo.
- **Route**: grupo `roads | trails | searoutes`, lista de puntos de control, longitud.
- **Zone**: overlay de color custom sobre un set de celdas (para marcar territorios especiales, zonas de peligro, etc.).
- **Ice**: elementos `glacier | iceberg`, con posición y vértices — sistema de hielo dedicado que Azgaar mantiene separado del heightmap normal.

### 7.8 Datos globales secundarios

- **Biomes**: matriz 2D `[temperatura][humedad] → bioma`, más costo de movimiento por bioma (usado en la expansión de culturas/estados) y habitabilidad (usado en el scoring de burgos).
- **NameBases**: generador de nombres por cultura — lista de nombres de entrenamiento, longitud mín/máx, letras duplicables, probabilidad de nombres multi-palabra. Es un sistema de generación fonética, no una lista fija.
- **Notes**: texto tipo leyenda asociado a cualquier entidad (burgo, estado, marcador, etc.) — relevante directamente para la integración con Atenea (§22).

---

## 8. Formatos de archivo

### 8.1 Entrada: compatibilidad con Azgaar

| Formato | Descripción | Prioridad |
|---|---|---|
| **JSON export completo** (Full JSON) | Formato más limpio, ya usado por integraciones de terceros (p. ej. el módulo de importación a Foundry VTT). Sin el ruido del SVG embebido. | **Prioridad 1** — fuente de verdad para import |
| `.map` legacy | Formato de texto custom con secciones concatenadas + un bloque SVG embebido íntegro (confirmado por desarrolladores de la comunidad que intentaron parsearlo). Más complejo, parcialmente redundante. | Prioridad 2 — soporte si se justifica por casos reales de "no tengo el JSON, solo el `.map`" |

En ambos casos aplica el hallazgo de §3: la geometría no viene en el archivo, se regenera.

### 8.2 Formato interno propio: `.gmap`

- **Binario**, no texto — prioriza velocidad de carga sobre legibilidad humana.
- Contiene: cabecera con versión de formato + metadata (nombre del mundo, semilla, fecha, versión de Voronia que lo generó) + el `World Data Model` completo serializado + (opcionalmente) buffers ya pre-triangulados listos para subir a GPU, para saltarse la tesselation en cada carga.
- Serialización v1 con `bincode`; evaluar migración a `rkyv` (zero-copy) si los tiempos de carga en mapas grandes no son suficientes.
- Versionado desde el día 1 (`u16 format_version` en la cabecera) para poder migrar mapas guardados cuando el modelo de datos evolucione.
- **Decisión pendiente de naming exacto**: `.gmap` es la opción por defecto en este documento (users mismo lo propuso, "g" de GPU). Alternativa: `.mapg`. Antes de fijarlo, vale la pena una revisión rápida de que no colisione fuerte con extensiones ya establecidas en otro software (p. ej. algunas herramientas GPS/GIS usan nombres parecidos) — ver §26.

### 8.3 Exportación

- **Imagen** (PNG a resolución arbitraria — ventaja real de GPU: exportar renders en altísima resolución rápido, algo que Azgaar no puede hacer bien por estar atado al DOM/canvas del navegador).
- **SVG** (para compatibilidad con flujos existentes de la comunidad, p. ej. edición posterior en Illustrator/Inkscape).
- **GeoJSON** (para uso en herramientas GIS como QGIS, igual que ya ofrece Azgaar).
- **JSON compatible con Azgaar** (para que alguien pueda ir y volver entre ambas herramientas, si el roadmap lo justifica).

---

## 9. Motor de renderizado GPU

### 9.1 Pipeline gráfico

```text
World Data Model
      │
      ▼
Geometry Builder (CPU)
  - Triangulación de celdas Voronoi (lyon)
  - Construcción de vertex/index buffers por capa
      │
      ▼
Upload a GPU (wgpu buffers)
      │
      ▼
Render Passes (uno o más, según capas activas)
      │
      ▼
Framebuffer → Surface → Pantalla
```

### 9.2 Sistema de capas (orden de dibujo, de atrás hacia adelante)

1. Heightmap / relieve base
2. Océanos y masas de agua
3. Biomas (color por bioma)
4. Ríos
5. Hielo (glaciares/icebergs)
6. Zonas custom (overlays)
7. Culturas (color/hatching)
8. Provincias
9. Estados/fronteras políticas
10. Religiones (modo alternable)
11. Rutas (caminos, senderos, rutas marítimas)
12. Iconos de relieve (bosques, montañas — patrón denso por bioma)
13. Burgos (íconos de ciudad/pueblo/capital)
14. Regimientos militares (modo alternable)
15. Marcadores (POIs)
16. Etiquetas/labels (nombres de estados, burgos, ríos, océanos)
17. Grid/regla, barra de escala, brújula, leyenda (UI de mapa)

Cada capa es activable/desactivable individualmente sin regenerar las demás — requisito explícito para no repetir el problema de rendimiento de Azgaar donde togglear capas puede ser costoso.

### 9.3 Cámara

- Proyección **ortográfica 2D** (no hay perspectiva 3D — el mapa es una "hoja", igual que en Azgaar).
- Pan/zoom = transformar la matriz de proyección (traslación + escala). Sin rotación en v1 (Azgaar tampoco la tiene).
- Zoom con límites sensatos y proyección esférica opcional queda en extensiones (§21.6).

### 9.4 Triangulación

Las celdas de Voronoi son polígonos irregulares, potencialmente no convexos. La GPU solo dibuja triángulos, así que se usa `lyon` para tesselation de cada polígono de celda. Se cachea el resultado (no se retesela en cada frame, solo cuando cambia geometría/edición).

### 9.5 Picking / selección

Click en el mapa → convertir coordenada de pantalla a coordenada de mundo (inversa de la matriz de cámara) → lookup espacial (quadtree o grid hash, igual que el `q` que ya usa Azgaar internamente para esto) → id de celda → resolver a burgo/estado/cultura si aplica.

### 9.6 Texto y labels

`glyphon` para texto en GPU (atlas de glyphs). Los nombres de estados curvados a lo largo de su forma (como hace Azgaar con sus "state labels") es una función avanzada, no bloqueante para v1 — texto horizontal simple primero.

### 9.7 LOD y culling

- Frustum culling: no procesar/dibujar celdas fuera de la vista.
- LOD por nivel de zoom: a zoom muy alejado, agrupar/simplificar geometría (no dibujar cada celda individual en un mapa de 100k celdas visto desde "todo el continente").

### 9.8 Temas y paletas

- Paleta de altura configurable (Azgaar permite esquemas de color custom para el heightmap).
- Paletas accesibles para daltonismo como opción, especialmente en el mapa de biomas.

---

## 10. Motor de simulación procedural

Cobertura completa de los sistemas confirmados que tiene Azgaar hoy (no listas hipotéticas — todos documentados en su wiki oficial), a reimplementar de forma nativa. El orden sugerido de implementación va en §23; acá se documenta el **alcance funcional** de cada uno.

### 10.1 Heightmap

- Sistema de **templates** (recetas predefinidas: continentes, archipiélago, atolón, mediterráneo, península, pangea, etc. — confirmar el set exacto y los parámetros de cada uno en Fase 0, wiki "Heightmap customization" y "Heightmap template editor").
- Combinación de ruido (Perlin/Simplex vía crate `noise`) + operaciones manuales de "blob placement" (montañas, depresiones) según la receta activa.
- Soporte para heightmap importado desde imagen externa (Azgaar lo permite — "Heightmap image overlay").

### 10.2 Hidrología

- Precipitación depositada por celda según el sistema de clima (10.3).
- Flujo de agua siguiendo el gradiente de altura, acumulando caudal (`flux`) hacia el mar — determina qué celdas tienen ríos y su ancho/caudal.
- Formación de lagos en depresiones sin salida.
- Confluencias donde se juntan ríos (`confluence_flux`).

### 10.3 Clima

- Temperatura por celda (banda de latitud + altura).
- Precipitación (viento prevaleciente + efecto orográfico — montañas bloquean humedad).
- Ambos alimentan la matriz de biomas (10.4).

### 10.4 Biomas

- Determinados por la matriz `[temperatura][humedad] → bioma` (confirmada en el modelo real de Azgaar, `biomesMatrix`).
- Cada bioma tiene costo de movimiento (afecta expansión de culturas/estados) y habitabilidad (afecta población/score de burgos).

### 10.5 Culturas

- Expansión tipo "flood-fill con costo" desde celdas centro, usando `expansionism` y el costo de bioma como resistencia.
- Sistema de herencia/evolución (árbol de culturas vía `origins`).
- Namebases fonéticos por cultura (10.9).

### 10.6 Estados y burgos

- Colocación de burgos según `score` de celda (harbor, biomas habitables, cercanía a ríos/costa).
- Expansión de estados análoga a culturas, pero considerando fronteras de estados vecinos y capitales.
- Provincias como subdivisión administrativa dentro de un estado.
- Sistema diplomático (`diplomacy`, `campaigns`/guerras) y militar (`Regiment`, alerta de guerra) — confirmado como sistema real y documentado ("Military Forces", "Battle Simulator" en la wiki de Azgaar), no una idea nueva.

### 10.7 Religiones

- Tipos: Folk, Organized, Heresy, Cult.
- Expansión limitada a la cultura de origen o global, según tipo.
- Árbol de religiones (herejías derivadas de una religión madre).

### 10.8 Rutas y economía

- Rutas (caminos, senderos, rutas marítimas) vía pathfinding sobre el grafo de celdas (Dijkstra/A* con `petgraph`), conectando burgos.
- Sistema de bienes/economía con "funciones de propagación" (confirmado — Azgaar documenta esto en su wiki "Goods: spread functions") — bienes que se producen en un lugar y se distribuyen por las rutas comerciales.

### 10.9 Nombres (namebases)

- Generador fonético entrenado con listas de nombres reales/fantásticos por cultura.
- Reglas: longitud mín/máx, letras duplicables permitidas, probabilidad de colapsar nombres multi-palabra en uno solo.
- Esto es un sistema relativamente autocontenido y es candidato ideal para portar temprano y testear de forma aislada (ver Fase 0/1).

---

## 11. Edición de mapas

- **Editar terreno**: pincel de altura, con tamaño/intensidad configurable.
- **Editar entidades**: mover/renombrar/recolorear burgos, estados, culturas, religiones; fusionar o dividir estados; bloquear (`locked`) entidades para que no se vean afectadas por una regeneración parcial (Azgaar ya soporta esto vía el flag `lock` en casi todas las entidades — se replica igual).
- **Editar fronteras manualmente**: reasignar celdas a otro estado/cultura/provincia a mano.
- **Editor de ríos**: mover el curso, cambiar ancho/caudal manualmente (Azgaar tiene un "River Editor" dedicado).
- **Undo/redo real**: patrón de comandos (`Command` trait con `apply`/`undo`), pila de historial persistente durante la sesión — mejora explícita sobre las limitaciones de undo que tiene Azgaar hoy.
- **Herramientas de dibujo**: tamaño de pincel, modo de aplicación (reemplazar/sumar/suavizar).

---

## 12. UI nativa

- **egui** sobre wgpu: paneles flotantes/dockeables sobre el canvas del mapa.
- Paneles principales: generación (parámetros/semilla/template), capas (toggles), inspector de entidad seleccionada (burgo/estado/cultura/río), leyenda, opciones de exportación.
- Atajos de teclado configurables (Azgaar tiene una página de "Hotkeys" dedicada — usar como checklist de paridad mínima).
- Tema oscuro por defecto (coherente con el setup de Hans en Hyprland), tema claro opcional.
- Escalado correcto en pantallas HiDPI (`winit` expone el factor de escala; hay que propagarlo a `egui` vía `pixels_per_point`).

---

## 13. Estructura del repositorio (Cargo workspace)

```text
voronia/
├── Cargo.toml                 # workspace root
├── LICENSE                    # MIT
├── README.md                  # incluye créditos a Azgaar
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── crates/
│   ├── vor-core/                # World Data Model (§7), sin lógica ni render
│   ├── vor-import/               # parsers .map / JSON de Azgaar + regeneración de geometría (§3, §8.1)
│   ├── vor-format/                # serialización .gmap (§8.2)
│   ├── vor-sim/                    # motor de simulación procedural (§10)
│   ├── vor-render/                 # pipeline wgpu, capas, cámara (§9)
│   ├── vor-edit/                    # comandos de edición + undo/redo (§11)
│   ├── vor-app/                      # binario final: winit + egui + orquestación
│   └── vor-cli/                       # herramientas headless (batch gen, conversión de formatos)
├── docs/
│   └── (specs por fase, ver §27)
└── tests/
    └── fixtures/               # mapas de ejemplo (propios y, si la licencia lo permite, de Azgaar) para tests de regresión
```

Separar en crates desde el inicio fuerza los límites de dependencia correctos (p. ej. `vor-render` nunca debería depender de `vor-import`), y permite compilar `vor-cli` sin toda la maquinaria gráfica.

---

## 14. Gestión de estado, rendimiento y concurrencia

- **Structure-of-Arrays (SoA)** para los atributos de celda (`Vec<u8>`, `Vec<u16>`, etc. indexados por id de celda) — mismo patrón que ya usa Azgaar internamente con `TypedArray`s de JS (`Uint8Array`, `Uint16Array`...), por buenas razones de cache-locality. Se mantiene el mismo patrón en Rust.
- **`rayon`** para paralelizar pasos independientes de la simulación (cálculo de clima por celda, expansión simultánea de múltiples culturas antes de resolver conflictos, etc.).
- El render corre en su propio ciclo (frame loop) desacoplado del tiempo que toma la simulación — la generación de un mundo nuevo no debe congelar la UI (considerar correrla en un thread aparte con progreso reportado a la UI).

---

## 15. Persistencia, autosave y recuperación

Azgaar mismo advierte a sus usuarios que **si se pierde el archivo `.map`, no hay forma de recuperar el progreso** — esto es una limitación conocida y real, no una suposición. Voronia debe mejorar esto explícitamente:

- Autosave periódico a `.gmap` en una ubicación de datos local (no depende de que el usuario recuerde exportar).
- Guardado versionado / historial de snapshots (al menos los últimos N autosaves, configurable).
- Recuperación ante cierre inesperado (detectar sesión anterior no cerrada correctamente al iniciar).

---

## 16. Testing y QA

- **Tests unitarios** de cada generador con semilla fija — dado el mismo seed, el output debe ser byte-idéntico entre corridas (regresión).
- **Tests de import**: cargar mapas reales exportados de Azgaar (varios tamaños/templates) y verificar que la geometría regenerada coincide con lo esperado.
- **Snapshot testing visual**: renders de referencia (PNG) comparados por diff perceptual, para detectar regresiones visuales en el renderer.
- **Benchmarks** (`criterion`) sobre: tiempo de generación por tamaño de mapa, tiempo de carga de `.gmap`, FPS de pan/zoom en mapas de referencia (10k/50k/100k celdas).
- CI corre benchmarks y falla si hay una regresión de rendimiento mayor a un umbral definido.

---

## 17. CI/CD y builds multiplataforma

- GitHub Actions: build + test + clippy + fmt en cada PR, para Linux (prioridad, target de desarrollo de Hans), Windows y macOS.
- Cross-compilation matrix desde el día 1 para no descubrir problemas de portabilidad tarde.
- Cache de dependencias de Cargo para acelerar CI.

---

## 18. Empaquetado y distribución

| Plataforma | Formato | Prioridad |
|---|---|---|
| Linux | AppImage y/o Flatpak | Alta (plataforma principal de desarrollo) |
| Windows | Instalador (`.msi` vía `cargo-wix` o similar) | Media |
| macOS | `.app` + notarización | Baja/opcional inicialmente |

CLI headless (`vor-cli`) se distribuye como binario suelto, sin empaquetado gráfico.

---

## 19. Documentación

- `README.md`: qué es, screenshots, quickstart, créditos a Azgaar y a los tres artículos de referencia.
- `docs/architecture.md`: versión resumida de este documento, mantenida viva.
- `docs/data-model.md`: el modelo de datos completo (expansión de §7), como referencia para contribuidores — mismo espíritu que la wiki de Azgaar, pero como referencia consistente desde el día 1 (no "documentado post-hoc" como reconoce el propio Azgaar que le pasó a él).
- Comentarios de código: doc-comments de Rust (`///`) en toda API pública, para poder generar `cargo doc`.

---

## 20. Código abierto: licencia, gobernanza y comunidad

- **Licencia: MIT** (ver §2.3).
- Repositorio en GitHub bajo la cuenta `hanserlodev` (identidad ya establecida en el flujo de git de Hans).
- `CONTRIBUTING.md`: cómo levantar el entorno, convención de commits, cómo correr tests.
- `CODE_OF_CONDUCT.md`: estándar (Contributor Covenant o similar).
- Templates de Issues/PRs.
- Roadmap público (este documento, resumido, o un GitHub Project board).

---

## 21. Extensiones más allá de Azgaar

Esto es explícitamente lo que el usuario pidió no dejar fuera: no solo igualar Azgaar, sino **superar sus límites actuales**.

### 21.1 Escala sin techo práctico
Azgaar limita `cellsDesired` a 100.000 por restricciones de rendimiento del navegador. Con GPU nativa, el objetivo es soportar cómodamente ese máximo y explorar mapas de 500k–1M+ celdas.

### 21.2 Simulación temporal / historia
Avanzar el mundo por años/siglos generando eventos históricos de forma procedural (guerras, sucesiones, migraciones, cambios climáticos), con una línea de tiempo navegable ("ver el mapa en el año X"). Esto conecta directamente con Atenea (§22).

### 21.3 Proyección esférica opcional
Además del modo "hoja plana" (como Azgaar), soportar opcionalmente un modo de planeta completo con proyección esférica, para mundos que no son solo un continente recortado.

### 21.4 Sistema de plugins/scripting
API de extensión (vía WASM o un lenguaje embebido tipo Lua/Rhai) para que la comunidad agregue generadores custom (nuevos templates de heightmap, nuevos tipos de cultura, etc.) sin tocar el core — algo que el propio Azgaar no ofrece hoy (es monolítico).

### 21.5 Exportación a motores de juego
Exportar heightmap + datos de entidades en formatos consumibles por Godot/Unity/Unreal — diferenciador fuerte frente a Azgaar, que es puramente cartográfico.

### 21.6 Integración con generación asistida por LLM
Nombres, descripciones y lore generados/enriquecidos vía LLM local — conecta directo con Atenea. Dato interesante encontrado en la investigación: el propio Azgaar ya tiene una página de wiki dedicada a integración con **Ollama** para generación de texto, que vale la pena revisar como referencia de lo que ya intentaron (ver §29).

### 21.7 API / CLI headless para generación batch
Generar N mundos en batch sin UI (`vor-cli`), útil para testing, para crear datasets, o para que Atenea pida "generame un mundo nuevo" sin abrir la app gráfica.

### 21.8 Colaboración en tiempo real (largo plazo, opcional)
Edición multi-usuario del mismo mundo — ambicioso, se deja fuera de v1 explícitamente (§1.4) pero documentado como visión de largo plazo.

---

## 22. Integración futura con Atenea

```text
                  ATENEA
                     │
                     ▼
              WORLD ENGINE (voronia)
                     │
        ┌────────────┼────────────┐
        ▼            ▼            ▼
      MAPA        HISTORIA      LORE
   (§9 render)   (§21.2 sim)  (Notes, §7.8)
        │            │            │
        └────────────┼────────────┘
                     ▼
                  IA LOCAL (Atenea)
```

Idea concreta de uso: Hans le pregunta a Atenea "¿qué pasó en el Reino de Valdoria en los últimos 200 años?", y Atenea consulta el mismo `World Data Model` estructurado que usa el mapa (estado, cultura, religión, guerras vía `campaigns`, relaciones diplomáticas) en vez de inventar la respuesta sin contexto. Esto requiere que `vor-core` exponga una API de consulta limpia (o un modo de exportar un resumen estructurado en JSON/texto) que Atenea pueda leer — a diseñar cuando ambos proyectos estén lo bastante maduros para integrarse. No es un requisito de v1 de Voronia, pero **sí debe influir en el diseño del modelo de datos desde ahora** (que sea fácil de consultar/serializar para consumo externo).

---

## 23. Roadmap por fases

Estimaciones de esfuerzo relativas (S/M/L/XL), no calendario fijo — depende de horas disponibles reales.

### Fase 0 — Investigación y sentado de bases · `M`
- [ ] Clonar el repo de Azgaar y revisar `src/` para identificar el PRNG exacto usado (crítico por §3).
- [ ] Identificar el algoritmo exacto de Delaunay/Voronoi y de "repacking" grid→pack en el código fuente real.
- [ ] Confirmar estructura exacta del JSON export completo (exportar un mapa real de prueba y diseccionarlo).
- [ ] Revisar wikis clave: Heightmap customization, Heightmap template editor, Culture types, Military Forces, Goods spread functions.
- [ ] Crear el repo `voronia` bajo `hanserlodev`, licencia MIT, README inicial con créditos.
- [ ] Setup del Cargo workspace (estructura de §13, vacía pero compilando).

### Fase 1 — Regeneración de geometría + parser de datos · `L` · ✓ COMPLETADA (commit `7907084`, 25 jul 2026)
 - [x] Portar el generador de puntos en grilla jitterizada con semilla.
 - [x] Portar Delaunay (`delaunator`) + derivación de Voronoi.
 - [x] Portar el algoritmo de repacking grid→pack.
 - [x] Validar contra mapas reales exportados de Azgaar (mismo seed → mismos índices de celda) — handshake Sorvik `.map` confirmado bit-exacto en `place_points` (10000 pts) y end-to-end loader (47 slots, 7268 pack cells, todos los counts de catálogos).
 - [x] Parser del `.map` Azgaar → poblar `World Data Model` (`vor-import::mapfile::Loader::load`). Parser JSON export Full DIFERIDO a Fase 2+ (decisión Hans: fase 0 §13.4 — si solo se importan mapas ya generados, no hace falta portear `aleaPRNG`/`randomizeOptions`).


### Fase 2 — Visor GPU mínimo · `M`
- [ ] Ventana (winit) + inicialización de wgpu.
- [ ] Cámara ortográfica con pan/zoom.
- [ ] Render de una sola capa: terreno (color por altura), vía triangulación con `lyon`.
- [ ] Esto ya es demostrable: "cargar un mapa real de Azgaar y verlo en un visor nativo GPU".

### Fase 3 — Capas completas de renderizado · `L`
- [ ] Ríos, fronteras de estados/provincias/culturas, biomas, burgos, labels básicos.
- [ ] Sistema de toggles de capas.
- [ ] Picking (click → info de celda/entidad).

### Fase 4 — Formato `.gmap` · `M`
- [ ] Definir el esquema completo con `serde` + `bincode`, versión 1.
- [ ] Save/load, con benchmark de velocidad vs re-importar desde JSON.
- [ ] Autosave básico (§15).

### Fase 5 — UI de edición · `M`
- [ ] Paneles egui: capas, inspector de entidad, opciones de exportación.
- [ ] Selección y edición básica de atributos de una entidad (renombrar, recolorear).

### Fase 6 — Edición avanzada · `L`
- [ ] Undo/redo (patrón Command).
- [ ] Pincel de edición de terreno.
- [ ] Edición manual de fronteras (reasignar celdas).
- [ ] Editor de ríos.

### Fase 7 — Motor de generación procedural nativo · `XL`
- [ ] Heightmap (templates + ruido).
- [ ] Hidrología (ríos, lagos).
- [ ] Clima (temperatura, precipitación) → biomas.
- [ ] Culturas (expansión + namebases).
- [ ] Estados/burgos/provincias (expansión, scoring, capitales).
- [ ] Religiones.
- [ ] Rutas + economía (bienes).
- [ ] Milicia/diplomacia (opcional dentro de esta fase, puede diferirse).

### Fase 8 — Extensiones más allá de Azgaar · `XL` (selección progresiva, no todo a la vez)
- [ ] Elegir 1-2 ítems de §21 como siguiente objetivo concreto (recomendado: empezar por 21.7 CLI headless, es la base para varias otras).

### Fase 9 — Integración con Atenea · `M`
- [ ] API de consulta estructurada del `World Data Model`.
- [ ] Prototipo de pregunta-respuesta usando Atenea + un mundo generado.

### Fase 10 — Empaquetado, distribución y v1.0 pública · `M`
- [ ] AppImage/Flatpak para Linux.
- [ ] Documentación pública completa.
- [ ] Anuncio en Discord/Reddit de Azgaar (con crédito claro) y donde sea relevante para la comunidad de worldbuilding.

---

## 24. Métricas de éxito / objetivos de rendimiento

Objetivos iniciales a validar con benchmarks reales desde la Fase 2 — no son promesas duras, son metas de diseño:

- Mapas de hasta **100.000 celdas** (el máximo actual que permite Azgaar) renderizando a **60 FPS** de pan/zoom en una RTX 4060 Mobile (8GB) o equivalente.
- Carga de un `.gmap` de un mapa grande en **menos de 1 segundo**.
- Import/conversión desde JSON de Azgaar de un mapa típico (10k–30k celdas) en **menos de 5 segundos** (incluye recomputar geometría, §3).
- Togglear cualquier capa de renderizado sin frame drop perceptible.

---

## 25. Riesgos y mitigaciones

| Riesgo | Impacto | Mitigación |
|---|---|---|
| No lograr reproducir bit-exacto la geometría de Azgaar (§3) | Alto — rompe compatibilidad de import | Plan B: soportar solo JSON export reciente, documentar dependencia de versión; invertir tiempo extra en Fase 0/1 antes de seguir |
| Alcance del motor de generación nativo (Fase 7) subestimado | Alto — es la fase más grande con diferencia | Fases 0-6 ya entregan valor real (visor + editor) sin depender de tener Fase 7 completa; se puede vivir un buen tiempo importando mapas ya generados en Azgaar |
| Fatiga de proyecto (solo dev, muchas fases) | Medio | Cada fase entrega algo demostrable/usable por sí sola (principio de diseño §4.4); usar Claude Code agresivamente para reducir fricción de escritura de código |
| Colisión de nombre `.gmap` con formato existente | Bajo | Revisar antes de fijar el nombre definitivo (§8.2, §26) |
| Deriva de la lógica de Azgaar (el proyecto original sigue evolucionando activamente, última release detectada: v1.119) | Bajo-Medio | Fijar la investigación de Fase 0 a una versión/commit específico de Azgaar como referencia, documentarlo, no perseguir cada cambio upstream |

---

## 26. Decisiones pendientes

- [x] Nombre del proyecto: `Voronia` (§1.3) — falta solo la verificación final de registro exacto en GitHub/crates.io al crear el repo.
- [ ] `.gmap` vs `.mapg` vs otro — y confirmar que no colisiona con un formato ya establecido.
- [ ] ¿`bincode` es suficiente para siempre, o se planifica desde ya la migración a `rkyv`?
- [x] ¿Soporte a `.map` legacy es un requisito real de v1, o alcanza con JSON export? (afecta el esfuerzo de Fase 1). → **Resuelto (25 jul 2026, Hans)**: Fase 1 scopea solo `.map` legacy (slot-by-slot), JSON export Full DIFERIDO. Validado handshake vs `.map` real Azgaar 1.138.0 (Sorvik). El `.map` legacy queda como vía de import soportada oficial en v1.
- [ ] Alcance exacto de Fase 8 (qué extensión de §21 se prioriza primero).
- [ ] ¿Se apunta a publicar en crates.io los crates internos (`vor-core`, etc.) por separado, o el workspace se mantiene monolítico?

---

## 27. Flujo de trabajo con Claude Code

- Este documento es la referencia macro. Para cada fase (§23), generar un **sub-spec enfocado** (`docs/specs/fase-N.md`) con el detalle accionable de esa fase específica antes de pedirle a Claude Code que implemente — evita saturar el contexto con las 29 secciones completas en cada sesión (coherente con el enfoque de optimización de tokens que Hans ya usa en otros proyectos).
- Convención de commits y de identidad de git: usar `hanserlodev` como identidad por defecto, igual que en el resto de los proyectos de Hans.
- Sugerido: una rama por fase (`fase-1-parser`, `fase-2-visor`, etc.), PR a `master` al cerrar cada fase con checklist de §23 como descripción.
- Los tests de regresión (§16) son especialmente importantes al trabajar con un agente: dan una señal objetiva de "esto no rompió nada" sin tener que revisar cada línea generada a mano.

---

## 28. Glosario

- **Voronoi**: partición del plano en regiones según el punto "semilla" más cercano.
- **Delaunay**: triangulación dual del diagrama de Voronoi; cada triángulo conecta 3 puntos semilla vecinos.
- **Heightmap**: mapa de elevación del terreno.
- **Bioma**: clasificación de terreno según temperatura/humedad (bosque, desierto, tundra, etc.).
- **Burg**: término de Azgaar para asentamiento (ciudad/pueblo/capital).
- **Namebase**: conjunto de reglas + datos de entrenamiento para generar nombres fonéticamente coherentes por cultura.
- **Flux**: caudal de agua acumulado en una celda (usado para determinar ríos).
- **LOD** (Level of Detail): reducir el detalle de geometría/render según la distancia/zoom.
- **Tessellation/Triangulación**: descomponer un polígono en triángulos para que la GPU pueda dibujarlo.
- **Bind group**: en wgpu, el mecanismo para pasarle datos (texturas, matrices) a un shader.
- **Render pass**: una pasada de dibujo sobre un framebuffer.
- **Shader (vertex/fragment)**: programa que corre en GPU; el de vértice transforma posiciones, el de fragmento calcula el color final de cada píxel.
- **WGSL**: lenguaje de shaders nativo de wgpu.
- **SoA** (Structure of Arrays): guardar cada atributo en su propio array contiguo (en vez de un array de structs) — mejor uso de cache de CPU.
- **Zero-copy deserialization**: leer datos de disco/memoria sin copiarlos a una nueva estructura (lo que ofrece `rkyv`).
- **Compute shader**: programa GPU de propósito general (no gráfico), para cálculos masivamente paralelos.
- **Pole of inaccessibility**: punto "más adentro" de un polígono, usado por Azgaar como centro visual para ubicar el nombre de un estado/provincia.

---

## 29. Referencias

**Proyecto original:**
- Repositorio: https://github.com/Azgaar/Fantasy-Map-Generator
- Aplicación: https://azgaar.github.io/Fantasy-Map-Generator/
- Licencia (MIT): https://github.com/Azgaar/Fantasy-Map-Generator/blob/master/LICENSE
- Modelo de datos (wiki oficial): https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Data-model
- Modelo de datos "en progreso": https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Data-model-(in-progress)
- Heightmap customization: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Heightmap-customization
- Heightmap template editor: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Heightmap-template-editor
- Culture types: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Culture-types
- Military Forces: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Military-Forces
- Battle Simulator: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Battle-Simulator
- Goods: spread functions: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Goods:-spread-functions
- GIS data export: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/GIS-data-export
- Run FMG locally: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Run-FMG-locally
- Working offline: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Working-offline
- Ollama text generation: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Ollama-text-generation
- Performance tips: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Tips#performance-tips
- Discusión de la comunidad sobre el formato `.map`: https://github.com/Azgaar/Fantasy-Map-Generator/discussions/1046
- Ejemplo de importador comunitario basado en JSON export: https://github.com/Ethck/azgaar-foundry

**Inspiración algorítmica citada por el propio Azgaar:**
- Martin O'Leary — *Generating fantasy maps*: https://mewo2.com/notes/terrain
- Amit Patel — *Polygonal Map Generation for Games*: http://www-cs-students.stanford.edu/~amitp/game-programming/polygon-map-generation
- Scott Turner — *Here Dragons Abound*: https://heredragonsabound.blogspot.com

**Proyectos hermanos de Azgaar (integraciones opcionales futuras, §21.5):**
- Armoria (generador de escudos/emblemas): https://github.com/Azgaar/Armoria
- Medieval Fantasy City Generator de Watabou: https://watabou.github.io/city-generator

**Recursos técnicos para el stack elegido:**
- Tutorial learn-wgpu: https://sotrh.github.io/learn-wgpu
