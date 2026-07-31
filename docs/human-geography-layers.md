# Azgaar Human Geography Layers — Documentation and porting plan

> **Category**: Human Geography
> **Layers**: states, provinces, zones, cultures, religions, population, burgs, markets, trade
> **Source**: Azgaar's FMP v1.135.2 — `/home/hans/Proyectos/azgaar-fmg/`

## 1. States (fill + borders)

### What it does in Azgaar
Fills each Voronoi cell with the state's color (from `state.color`). Draws borders between states with a darker stroke.

### Data
| Slot | Voronia Field | Type | Description |
|------|---------------|------|-------------|
| `[14]` | `pack.states` | `Vec<State>` | State catalog |
| `[25]` | `pack.cells.state` | `Vec<u16>` | state_id per pack cell |
| — | `state.color` | `String` | Hex `#rrggbb` |

### Voronia implementation
- **Fill**: `build_pack_mesh()` with `pack.cells.state[p] → states[id].color`
- **Borders**: `build_border_mesh(BorderKind::State)` already exists (but with fixed red color)

---

## 2. Provinces (fill + borders)

Identical to States but with `pack.cells.province` and `provinces[].color`.

---

## 3. Cultures (fill + borders)

Identical to States but with `pack.cells.culture` and `cultures[].color`.

---

## 4. Religions (fill)

Identical to States but with `pack.cells.religion` and `religions[].color`. No borders (Azgaar does not draw religious borders).

---

## 5. Population

### What it does in Azgaar
There is no explicit population layer in Azgaar — it is shown as burg size. Voronia will implement a per-cell heatmap.

### Data
| Slot | Voronia Field | Type |
|------|---------------|------|
| `[21]` | `pack.cells.population` | `Vec<f32>` |

### Implementation
Heatmap: transparent → yellow → orange → red depending on density.

---

## 6. Burgs

### What it does in Azgaar
Circular markers at each burg's position, colored by state.

### Voronia implementation
`build_burg_mesh()` already exists (red triangles). Improvement: color by `burgs[].state → states[].color`.

---

## 7. Zones

### Data
| Slot | Voronia Field | Type |
|------|---------------|------|
| `[38]` | `world.zones` | `Vec<Zone>` |
| — | `zone.cells` | `Vec<u32>` |
| — | `zone.color` | `String` |

### Implementation
Color the pack cells that belong to zones.

---

## 8. Routes/Trade

### Data
| Slot | Voronia Field | Type |
|------|---------------|------|
| `[37]` | `world.routes` | `Vec<Route>` |
| — | `route.points` | `Vec<[f32;3]>` |
| — | `route.group` | RouteGroup |

### Implementation
Lines between route points. Color by group: roads=brown, trails=tan, searoutes=blue.

---

## Summary

| Layer | Pipeline | Method | Source |
|------|----------|--------|--------|
| States fill | TriangleList | `build_pack_mesh` + entity color | `state[]` |
| Provinces fill | TriangleList | `build_pack_mesh` + entity color | `province[]` |
| Cultures fill | TriangleList | `build_pack_mesh` + entity color | `culture[]` |
| Religions fill | TriangleList | `build_pack_mesh` + entity color | `religion[]` |
| Population | TriangleList | `build_pack_mesh` + heatmap | `population[]` |
| Burgs | TriangleList | existing + color by state | `burg[]` → `state[]` |
| Zones | TriangleList | `build_pack_mesh` + lookup | `zone[].cells` |
| Routes | LineList | line segments | `route[].points` |
