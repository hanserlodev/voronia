# Phase 5 — Editing UI · M ✓ COMPLETED (Jul 27, 2026)

## Frozen Azgaar reference

- Based on the same Sorvik `.map` used in Phase 4.
- The entity data model (State, Burg, Province) was confirmed against the real Sorvik `.map` (slots 14/15/30).

## Architecture of the code produced

### `vor-edit` (new crate)

```
crates/vor-edit/src/
├── lib.rs       -- EditBuffer (dirty flag + temporary editing buffers), SelectedEntity enum
├── error.rs     -- EditError (EntityNotFound, InvalidHexColor, EmptyName)
├── color.rs     -- normalize_hex() (validates and normalizes #rrggbb)
├── state.rs     -- rename_state, set_state_color, set_state_form (searches by `id` with find)
├── burg.rs      -- rename_burg, set_burg_population, toggle_burg_capital
└── province.rs  -- rename_province, set_province_color
```

API decisions:
- All functions receive `&mut World` + id + value, return `Result<(), EditError>`.
- Entity lookup is by `id` (not by position in the Vec) because the .map loader does `skip(1)` without a placeholder at position 0.
- `set_burg_population` updates both the Burg field and `pack.cells.population[cell]` (consistency).
- `toggle_burg_capital` removes the capital from other burgs of the same state.
- EditBuffer stores temporary strings and selected_entity_id for egui binding.

### `vor-app` — UI extensions

- **Entity Inspector**: "editor" section in the SidePanel, below the cell inspector. It appears only when the cell has a state/burg/province. Fields: name (text), color (hex), apply button.
- **Export panel**: three collapsible sections: save .vorn (with autosave toggle), PNG, SVG.
- `entity_from_cell(world, cell)` → determines `SelectedEntity` (priority: State > Burg > Province).

### PNG export (`crates/vor-app/src/png_export.rs`)

Renders the active layers to an offscreen texture (same format as the surface) of the specified size, reads the pixels via `map_async`, converts BGRA→RGBA if the surface format is BGRA, encodes with `image::RgbaImage::save()`.

### SVG export (`crates/vor-app/src/svg_export.rs`)

Generates a standalone SVG from the World Data Model (no GPU):
1. Dark background (`<rect>`)
2. Voronoi polygons per pack cell colored by height (`height_color` ramp)
3. Rivers as `<polyline>` with width according to discharge
4. State borders as `<path>` with segments between neighboring cells of different states
5. Burgs as `<circle>` + `<text>` (capitals radius 5, the rest radius 3)

## Fixes incorporated

### Lookup by id (not position)

The State/Province entities in the cell inspector and editor used `get(sid as usize)` — incorrect because the .map loader does `skip(1)` without a placeholder at position 0. They now use `iter().find(|s| s.id == sid)`, consistent with burgs.

## Test inventory

| File | Count | What it validates |
|---|---|---|
| `crates/vor-edit/tests/edit_tests.rs` | 14 | rename_state/burg/province, set_state/province_color, set_burg_population, toggle_capital, normalize_hex, error cases |

Workspace total: 67 tests (14 new in vor-edit).

## Final state

- `cargo test --workspace`: 67 passed, 0 failed.
- `cargo clippy --workspace`: 0 errors (pre-existing warnings).
- `cargo fmt --check`: clean.
- Working tree: clean.

## Master plan §23 checklist

- [x] egui panels: layers, entity inspector, export options.
- [x] Basic selection and editing of an entity's attributes (rename, recolor).
