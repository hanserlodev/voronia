# Fase 6 — Overhaul de visualización: landmass por features, ríos suavizados, splines

> Fecha de creación: 28 jul 2026
> Fecha de cierre: (en curso)
> Estado: en progreso

## 1. Referencia

N/A — trabajo de visualización nativa, sin código de Azgaar involucrado.

## 2. Cronología de commits

| Hash | Fecha | Título | Qué hizo |
|---|---|---|---|
| `f39c085` | 28 jul 2026 | feature-based landmass mesh with Catmull-Rom coastlines | Cambia heightmap de grilla regular a mesh basado en features: por cada feature de costa/tierra firme, construye un polígono con Catmull-Rom. Altura se obtiene de la grilla subyacente. Problema: islas quedan con huecos. |
| `ed8aef7` | 28 jul 2026 | smooth lake and river rendering with Catmull-Rom splines | Lake/lagoon borders con Catmull-Rom smooth en lake.rs. Ríos con Catmull-Rom en river.rs. |
| `67a44d0` | 28 jul 2026 | increase Catmull-Rom subdivisions (rivers 12, lakes 5) | Más subdivisiones para curvas más suaves. |
| `a47f468` | 28 jul 2026 | extend rivers into ocean, increase subdivisions (rivers 20, lakes 6) | Primer intento de extensión de desembocadura: extiende el path del río desde la última celda pack hacia la posición de mouth_cell (grid). |
| `80fdf32` | 28 jul 2026 | proportional river mouth extension (0.3x seg_len), 30 subdivisions | Extensión proporcional a la longitud del último segmento. |
| `0ee10b8` | 28 jul 2026 | simplify river path (RDP) then Catmull-Rom, find real mouth in pack | Ramer-Douglas-Peucker para simplificar el path antes del spline. Intenta convertir mouth_cell de grid a pack via grid_id. |
| `d7d52ce` | 28 jul 2026 | revert river to simple Catmull-Rom (4 subdiv), no RDP or mouth extension | Revert: RDP eliminaba curvas naturales, mouth extension no funcionaba correctamente. Vuelve a spline simple. |
| `c0c75a4` | 28 jul 2026 | fix: remove unused Pack import from river.rs, clean up | Limpieza de imports muertos. |
| `488accc` | 28 jul 2026 | rivers: use lyon StrokeTessellator with round caps/joins | Cambia de quads manuales a StrokeTessellator para mejor rendering de extremos y esquinas. |
| `027284f` | 28 jul 2026 | fix: call end(false) on open river path before build | Crash de lyon: path builder requiere `end(true/false)` antes de `build()`. |

## 3. Arquitectura del código producido

### crates/vor-render/src/river.rs

- `build_river_mesh(points, rivers) -> HeightmapMesh`: itera ríos, filtra paths <2 pts, Catmull-Rom con 4 subdiv, StrokeTessellator con grosor = `discharge_m3s / 3000.0` clamp(0.8, 5.0), color azul fijo.

### crates/vor-render/src/lake.rs (no tocado en esta sesión pero relacionado)

- Usa FillTessellator para polígonos cerrados, Catmull-Rom para suavizar contornos si tiene suficientes puntos.

### crates/vor-render/src/heightmap.rs - Feature-based landmass

- `build_mesh` con feature-iteration: por cada feature de tipo Landmass/Island, triangula los vértices Voronoi de las celdas del feature. La altura se obtiene muestreando la grilla (triángulo grande → ~5 puntos de muestra por arista).
- Altura de celda: usa `grid.height` con sample bi-lineal o nearest-neighbor.
- Altura de feature de costa: si algún sample del triángulo cae en mar (grid.height=0), asigna color de landscape, no de costa.

### crates/vor-render/src/mesh.rs

- `catmull_rom_open(points, subdivisions) -> Vec<[f32; 2]>`: spline Catmull-Rom abierto (no cierra el lazo). 4 subdivisiones default.

## 4. Hallazgos críticos y decisiones

### Feature-based landmass: coast sampling

El heightmap original usaba celdas individuales → costa irregular. Al cambiar a features con Catmull-Rom, la costa queda suave pero los triángulos grandes de Voronoi cruzan zonas de mar. Solución: samplear ~5 puntos equiespaciados en cada arista del triángulo; si alguno cae en mar (grid cell con height=0), ese triángulo pinta como landscape. Esto da la ilusión de costa suave sin cambiar la topología de la malla.

### StrokeTessellator para ríos

Reemplaza los quads manuales (que tenían bugs de culling, orientación, y se veían cortados en extremos) por `StrokeTessellator` de lyon. Esto da caps redondos y joins suaves automáticamente. Grosor variable por caudal. Color azul fijo (0.15, 0.45, 0.85).

### `builder.end()` obligatorio en lyon

El path builder de lyon requiere `end(true/false)` antes de `build()`. Con paths abiertos (ríos) se usa `end(false)`. Si se omite, lyon crashea con "build() called before end()". Los paths cerrados (lagos, features) usan `end(true)` o tessellator de fill.

## 5. Bug conocido: desembocaduras de río

**Síntoma**: los ríos no llegan al océano. Terminan en la última celda de tierra (costa), no se extienden al mar.

**Causa raíz**: en `trace_river_paths()` (loader.rs:318):
```rust
let mouth = river.mouth_cell as usize;
```
`mouth_cell` está en espacio **GRID** (índice en `grid.cells`, 0..N_grid). Pero `adjacency` está en espacio **PACK** (post-reGraph). El path sigue celdas pack con `river_id` decreciente en altura, pero `mouth` es un ID de grilla que no existe en pack. La condición `current == mouth` nunca se cumple. El loop termina cuando no encuentra más vecinos pack con ese river_id — en la última celda de tierra en la costa. La celda de desembocadura (que sería la primer celda de mar) no está en pack porque reGraph descarta celdas de mar salvo las costeras con altura >0.

**Intentos de fix previos**:
- `a47f468`: Extensión en el renderer desde último punto hacia posición de mouth_cell. Funcionaba a medias pero la longitud de extensión era fija, no proporcional al terreno.
- `80fdf32`: Extensión proporcional (0.3x largo del último segmento). Mejor pero la dirección a veces apuntaba mal.
- `0ee10b8`: Conversión mouth_cell grid→pack via grid_id lookup. Falla porque `mouth_cell` es una celda grid que NO tiene grid_id en pack (es mar, no pasó por reGraph).

**Fix planeado**: en `trace_river_paths`, al llegar al dead-end (última celda pack con river_id), mirar la `mouth_cell` original (grid) y si la celda grid tiene `height=0` (es mar), extender el path hasta la boca. O, más robusto: en el renderer, tomar el último punto del path + la posición de la celda mouth (en grid space) y extender el spline hacia esa dirección un 30% de la distancia al mouth.

## 6. Inventario de tests

| Archivo | Tests | Qué valida |
|---|---|---|
| `river.rs` | (en HeightmapMesh, indirectamente) | Render de ríos, grosor, Catmull-Rom |
| `mesh.rs` | `test_catmull_rom_open_basic` | Catmull-Rom produce puntos, no se traga paths cortos |

No se agregaron tests unitarios nuevos de render en esta sesión (validación visual).

## 7. Estado final del working tree

Working tree limpio al cierre de la sesión (commit `027284f`). Compila con `cargo build --workspace`. Tests verdes.
