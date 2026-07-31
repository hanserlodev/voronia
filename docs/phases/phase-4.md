# Phase 4 — `.vorn` format (Vorn World File)

> Consolidated output of Phase 4 of the master plan (§22). Closing: commit `ee3ec2e` (Jul 27, 2026). Native binary format of Voronia: replaces the `.gmap` placeholder from previous phases, implemented with serde + bincode v1 in `vor-format`, integrated with autosave in vor-app and benchmark validated against Sorvik.

## 0. Format name

- **Name**: Vorn World File
- **Extension**: `.vorn`
- **Etymology**: from "Voronoi" — the geometric heart of the engine
- **Collision check**: searched on fileinfo.com, file-extensions.org, justsolve.archiveteam.org — no collisions with existing formats. Discarded `.gmap` (collision with Garmin GPS + Google My Maps), `.vor` (StarOffice/OpenOffice templates), `.vmap` (Valve Source 2), `.vwm` (Line 6 Variax + VisualWatermark).

---

## 1. Session chronology (commits by date)

| Commit | Date | Title | What it did |
|---|---|---|---|
| `ee3ec2e` | Jul 27, 2026 | `Fase 4 — formato .vorn: save/load binario, benchmark, autosave` | 23 files, +541 lines: `.gmap`→`.vorn` rename in docs+skill+code, `vor-format` with header/magic+metadata+payload bincode, 5 tests, criterion benchmark, `serde_json::Value`→string fix for bincode, autosave integrated in vor-app, `--export-vorn` CLI |

---

## 2. `.vorn` format architecture

### 2.1 On-disk layout

```
+0..4    magic: b"VORN"                    — format identification
+4..6    format_version: u16 LE            — 1 for v1
+6..10   metadata_len: u32 LE              — bytes of the serialized VornMetadata
+10      compression: u8                   — 0 = none (reserved: 1 = gzip)
+11..16  _reserved: [u8; 5]                — padding to 16 bytes
+16..    metadata: VornMetadata (bincode)  — map_name, seed, date, voronia_version, azgaar_version
+16+meta.. payload: World (bincode)        — serialized World Data Model
```

Fixed header: **16 bytes**. Enables forward/backward compatibility via the `format_version` + `compression` flag. Metadata allows identifying a `.vorn` without deserializing the full World.

### 2.2 `VornMetadata`

| Field | Type | Description |
|---|---|---|
| `map_name` | `String` | Map name |
| `seed` | `String` | Procedural seed |
| `date` | `String` | Creation/export date |
| `voronia_version` | `String` | Voronia version that generated the file |
| `azgaar_version` | `Option<String>` | Azgaar version it was imported from |

### 2.3 Public API (`vor-format`)

```rust
// Save
save::save(path, &world, &metadata) -> Result<(), FormatError>
save::save_world(path, &world) -> Result<(), FormatError>   // metadata inferred from the World

// Load
load::load(path) -> Result<(World, VornMetadata), FormatError>
load::load_metadata(path) -> Result<VornMetadata, FormatError>  // without deserializing the World

// Types
VornHeader::new(metadata_len) -> VornHeader
VornHeader::encode(&self) -> [u8; 16]
VornHeader::decode(&[u8; 16]) -> Result<VornHeader, FormatError>

VornMetadata::new(map_name, seed, date, voronia_version, azgaar_version)
```

### 2.4 Errors

`FormatError` enum with 5 variants: `Io`, `Bincode`, `InvalidMagic`, `UnsupportedVersion`, `ChecksumMismatch`.

---

## 3. Integration in vor-app

### 3.1 Periodic autosave

- **Toggle** in the left SidePanel ("autosave" section), default ON
- **Interval**: every 60 seconds (fixed, configurable in code)
- **Destination**: `<path-of-the-.map>.vorn` (same directory, changed extension)
- **Mechanism**: post-redraw check in the event loop, non-blocking save (~18ms does not justify a separate thread)

### 3.2 Manual save

**"save .vorn ahora"** button in the side panel → saves immediately.

### 3.3 CLI `--export-vorn`

```
cargo run --bin vor-app -- /path/mapa.map --export-vorn
```

Loads the .map, regenerates geometry, exports .vorn, exits without opening a window.

---

## 4. Fix for bincode + `serde_json::Value`

**Problem**: bincode 1.x does not support `deserialize_any()`, which is what `serde_json::Value` uses internally in its `Deserialize` impl. When trying to deserialize a `World` with bincode, it fails with `DeserializeAnyNotSupported`.

**Solution**: `vor_core::serde_json_string` module with a custom serializer/deserializer that converts `Value`↔`String` (textual JSON). Applied with `#[serde(with = "crate::serde_json_string")]` on all `serde_json::Value` fields of the model:

- `World::fonts`, `custom_good_icons`, `goods`, `markets`, `deals`
- `MapCoordinates::extras`
- `Settings::options`, `rescale_labels`, `urban_density`, `growth_rate`
- `State::diplomacy`, `campaigns`, `military`
- `CoatOfArms::payload`

---

## 5. Benchmark

**Setup**: Criterion, Sorvik (5MB .map, 10000 grid cells, 7268 pack cells), sample size 10, 10s measurement.

| Operation | Time | vs import |
|---|---|---|
| Full `.map` import (parse + geom regenerate) | **90.3 ms** | 1× |
| Save `.vorn` | **17.9 ms** | 5× faster |
| Load `.vorn` | **37.3 ms** | **2.4× faster** |

Loading a `.vorn` is 2.4× faster than importing from `.map` because it skips slot parsing, Delaunay/Voronoi regeneration and repacking. Saving is even faster (only in-memory bincode serialization + disk write).

---

## 6. Test inventory

| File | Tests | What it validates |
|---|---|---|
| `vor-format/src/lib.rs` | 5 | default World roundtrip, file roundtrip, invalid magic, unsupported version, save_world convenience |
| `vor-format/benches/vorn_benchmark.rs` | 2 benchmarks | full_import (90ms), save_vorn (18ms), load_vorn (37ms) |

### All workspace tests (53 tests)

```
vor-format: 5 passed
vor-import: 27 passed
vor-render: 7 passed
alea_bit_exact: 2 passed
delaunay_bit_exact: 1 passed
grid_bit_exact: 1 passed
regraph_bit_exact: 1 passed
sorvik_full_load: 8 passed
sorvik_handshake: 1 passed
voronoi_bit_exact: 1 passed
```

clippy 0 errors, fmt clean.

---

## 7. Created/modified files

### Created (6 files)

| File | Lines | Purpose |
|---|---|---|
| `crates/vor-format/src/header.rs` | 94 | VornHeader + VornMetadata, encode/decode |
| `crates/vor-format/src/save.rs` | 42 | save() + save_world() |
| `crates/vor-format/src/load.rs` | 45 | load() + load_metadata() |
| `crates/vor-format/src/error.rs` | 19 | FormatError enum |
| `crates/vor-format/src/lib.rs` | 145 | re-exports + 5 tests |
| `crates/vor-format/benches/vorn_benchmark.rs` | 62 | criterion benchmark |
| `crates/vor-core/src/serde_json_string.rs` | 20 | serde helper for bincode compat |

### Modified (16 files)

| File | Change |
|---|---|
| `README.md` | `.gmap` → `.vorn` in crates tree |
| `docs/plans/master-plan.md` | 12 `.gmap`→`.vorn` occurrences, §8.2 updated with the naming decision, naming item checked, collision risk resolved |
| `docs/phases/phase-1.md` | `.gmap` → `.vorn` in the future phases table |
| `crates/vor-format/Cargo.toml` | description, serde/bincode/thiserror deps, criterion+vor-import dev-deps, [[bench]] |
| `crates/vor-core/src/lib.rs` | `pub mod serde_json_string` |
| `crates/vor-core/src/world.rs` | `#[serde(with = "...")]` on 5 Value fields |
| `crates/vor-core/src/settings.rs` | `#[serde(with = "...")]` on 4 Value fields |
| `crates/vor-core/src/coordinates.rs` | `#[serde(with = "...")]` on extras |
| `crates/vor-core/src/entities/state.rs` | `#[serde(with = "...")]` on diplomacy/campaigns/military |
| `crates/vor-core/src/entities/coat_of_arms.rs` | `#[serde(with = "...")]` on payload |
| `crates/vor-app/Cargo.toml` | + `vor-format` dep |
| `crates/vor-app/src/lib.rs` | autosave fields + toggle + save button + `--export-vorn` CLI |

---

## 8. Final state

- **Phase 4**: ✓ COMPLETED (master plan §22, checkbox checked)
- **Working tree**: clean (commit `ee3ec2e` pushed to main)
- **Tests**: 53 green
- **clippy**: 0 errors
- **fmt**: clean
- **Benchmarks**: 2 groups (import_map, vorn_save_load)

---

## 9. Verification commands

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
cargo bench --package vor-format
cargo run --bin vor-app -- /ruta/mapa.map --export-vorn
```

---

*End of Phase 4 record. Frozen at commit `ee3ec2e` (Jul 27, 2026). Next update: `docs/phases/phase-5.md` when Phase 5 is closed.*
