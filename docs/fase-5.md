# Fase 5 — UI de edición · M ✓ COMPLETADA (27 jul 2026)

## Referencia congelada de Azgaar

- Basado en el mismo `.map` de Sorvik usado en Fase 4.
- El modelo de datos de entidades (State, Burg, Province) se confirmó contra el .map real de Sorvik (slots 14/15/30).

## Arquitectura del código producido

### `vor-edit` (nuevo crate)

```
crates/vor-edit/src/
├── lib.rs       -- EditBuffer (dirty flag + buffers de edición temporales), SelectedEntity enum
├── error.rs     -- EditError (EntityNotFound, InvalidHexColor, EmptyName)
├── color.rs     -- normalize_hex() (valida y normaliza #rrggbb)
├── state.rs     -- rename_state, set_state_color, set_state_form (busca por `id` con find)
├── burg.rs      -- rename_burg, set_burg_population, toggle_burg_capital
└── province.rs  -- rename_province, set_province_color
```

Decisiones de API:
- Todas las funciones reciben `&mut World` + id + valor, retornan `Result<(), EditError>`.
- La búsqueda de entidad es por `id` (no por posición en el Vec) porque el loader de .map hace `skip(1)` sin placeholder en pos 0.
- `set_burg_population` actualiza tanto el campo del Burgo como `pack.cells.population[cell]` (consistencia).
- `toggle_burg_capital` quita capital de otros burgos del mismo estado.
- EditBuffer almacena strings temporales y selected_entity_id para binding egui.

### `vor-app` — extensiones de UI

- **Entity Inspector**: sección "editor" en el SidePanel, debajo del inspector de celda. Aparece solo cuando la celda tiene estado/burgo/provincia. Campos: nombre (texto), color (hex), botón aplicar.
- **Export panel**: tres secciones colapsables: save .vorn (con autosave toggle), PNG, SVG.
- `entity_from_cell(world, cell)` → determina `SelectedEntity` (prioridad: State > Burg > Province).

### Export PNG (`crates/vor-app/src/png_export.rs`)

Renderiza las capas activas a una textura offscreen (mismo formato que la surface) del tamaño especificado, lee los pixels vía `map_async`, convierte BGRA→RGBA si el formato de surface es BGRA, encodea con `image::RgbaImage::save()`.

### Export SVG (`crates/vor-app/src/svg_export.rs`)

Genera SVG autónomo desde World Data Model (sin GPU):
1. Fondo oscuro (`<rect>`)
2. Polígonos de Voronoi por celda pack coloreados por altura (`height_color` rampa)
3. Ríos como `<polyline>` con ancho según caudal
4. Fronteras de estados como `<path>` con segmentos entre celdas vecinas de distinto estado
5. Burgos como `<circle>` + `<text>` (capitales con radio 5, resto con radio 3)

## Fixes incorporados

### Lookup por id (no posición)

Las entidades State/Province en el inspector de celda y en el editor buscaban con `get(sid as usize)` — incorrecto porque el loader de .map hace `skip(1)` sin placeholder en pos 0. Ahora usan `iter().find(|s| s.id == sid)` consistente con burgos.

## Inventario de tests

| Archivo | Count | Qué valida |
|---|---|---|
| `crates/vor-edit/tests/edit_tests.rs` | 14 | rename_state/burg/province, set_state/province_color, set_burg_population, toggle_capital, normalize_hex, casos error |

Total workspace: 67 tests (14 nuevos en vor-edit).

## Estado final

- `cargo test --workspace`: 67 passed, 0 failed.
- `cargo clippy --workspace`: 0 errors (warnings pre-existentes).
- `cargo fmt --check`: clean.
- Working tree: limpio.

## Checklist plan maestro §23

- [x] Paneles egui: capas, inspector de entidad, opciones de exportación.
- [x] Selección y edición básica de atributos de una entidad (renombrar, recolorear).
