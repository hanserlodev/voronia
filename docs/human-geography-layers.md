# Azgaar Human Geography Layers — Documentación y plan de portabilidad

> **Categoría**: Human Geography
> **Capas**: states, provinces, zones, cultures, religions, population, burgs, markets, trade
> **Fuente**: Azgaar's FMP v1.135.2 — `/home/hans/Proyectos/azgaar-fmg/`

## 1. States (fill + borders)

### Qué hace en Azgaar
Rellena cada celda Voronoi con el color del estado (de `state.color`). Dibuja bordes entre estados con trazo más oscuro.

### Datos
| Slot | Campo Voronia | Tipo | Descripción |
|------|---------------|------|-------------|
| `[14]` | `pack.states` | `Vec<State>` | Catálogo de estados |
| `[25]` | `pack.cells.state` | `Vec<u16>` | state_id por celda pack |
| — | `state.color` | `String` | Hex `#rrggbb` |

### Implementación Voronia
- **Fill**: `build_pack_mesh()` con `pack.cells.state[p] → states[id].color`
- **Borders**: ya existe `build_border_mesh(BorderKind::State)` (pero color fijo rojo)

---

## 2. Provinces (fill + borders)

Idéntico a States pero con `pack.cells.province` y `provinces[].color`.

---

## 3. Cultures (fill + borders)

Idéntico a States pero con `pack.cells.culture` y `cultures[].color`.

---

## 4. Religions (fill)

Idéntico a States pero con `pack.cells.religion` y `religions[].color`. Sin borders (Azgaar no dibuja bordes religiosos).

---

## 5. Population

### Qué hace en Azgaar
No hay capa de población explícita en Azgaar — se muestra como tamaño de burg. Voronia implementará un heatmap por celda.

### Datos
| Slot | Campo Voronia | Tipo |
|------|---------------|------|
| `[21]` | `pack.cells.population` | `Vec<f32>` |

### Implementación
Heatmap: transparente → amarillo → naranja → rojo según densidad.

---

## 6. Burgs

### Qué hace en Azgaar
Marcadores circulares en la posición de cada burgo, color según el estado.

### Implementación Voronia
Ya existe `build_burg_mesh()` (triángulos rojos). Mejora: color por `burgs[].state → states[].color`.

---

## 7. Zones

### Datos
| Slot | Campo Voronia | Tipo |
|------|---------------|------|
| `[38]` | `world.zones` | `Vec<Zone>` |
| — | `zone.cells` | `Vec<u32>` |
| — | `zone.color` | `String` |

### Implementación
Colorear celdas del pack que pertenecen a zonas.

---

## 8. Routes/Trade

### Datos
| Slot | Campo Voronia | Tipo |
|------|---------------|------|
| `[37]` | `world.routes` | `Vec<Route>` |
| — | `route.points` | `Vec<[f32;3]>` |
| — | `route.group` | RouteGroup |

### Implementación
Líneas entre puntos de ruta. Color según grupo: roads=marrón, trails=tan, searoutes=azul.

---

## Resumen

| Capa | Pipeline | Método | Source |
|------|----------|--------|--------|
| States fill | TriangleList | `build_pack_mesh` + entity color | `state[]` |
| Provinces fill | TriangleList | `build_pack_mesh` + entity color | `province[]` |
| Cultures fill | TriangleList | `build_pack_mesh` + entity color | `culture[]` |
| Religions fill | TriangleList | `build_pack_mesh` + entity color | `religion[]` |
| Population | TriangleList | `build_pack_mesh` + heatmap | `population[]` |
| Burgs | TriangleList | existing + color por estado | `burg[]` → `state[]` |
| Zones | TriangleList | `build_pack_mesh` + lookup | `zone[].cells` |
| Routes | LineList | segmentos de línea | `route[].points` |
