# Fase 4 — Formato `.vorn` (Vorn World File)

> Salida consolidada de la Fase 4 del plan maestro (§23). Cierre: commit `ee3ec2e` (27 jul 2026). Formato binario nativo de Voronia: reemplaza al placeholder `.gmap` de fases previas, implementado con serde + bincode v1 en `vor-format`, integrado con autosave en vor-app y benchmark validado contra Sorvik.

## 0. Nombre del formato

- **Nombre**: Vorn World File
- **Extensión**: `.vorn`
- **Etimología**: de "Voronoi" — el corazón geométrico del motor
- **Verificación de colisiones**: se buscó en fileinfo.com, file-extensions.org, justsolve.archiveteam.org — sin colisiones con formatos existentes. Descartado `.gmap` (colisión con Garmin GPS + Google My Maps), `.vor` (StarOffice/OpenOffice templates), `.vmap` (Valve Source 2), `.vwm` (Line 6 Variax + VisualWatermark).

---

## 1. Cronología de la sesión (commits por fecha)

| Commit | Fecha | Título | Qué hizo |
|---|---|---|---|
| `ee3ec2e` | 27 jul 2026 | `Fase 4 — formato .vorn: save/load binario, benchmark, autosave` | 23 archivos, +541 líneas: rename `.gmap`→`.vorn` en docs+skill+código, `vor-format` con header/magic+metadata+payload bincode, 5 tests, benchmark criterion, fix `serde_json::Value`→string para bincode, autosave integrado en vor-app, CLI `--export-vorn` |

---

## 2. Arquitectura del formato `.vorn`

### 2.1 Layout en disco

```
+0..4    magic: b"VORN"                    — identificación del formato
+4..6    format_version: u16 LE            — 1 para v1
+6..10   metadata_len: u32 LE              — bytes del VornMetadata serializado
+10      compression: u8                   — 0 = none (reservado: 1 = gzip)
+11..16  _reserved: [u8; 5]                — padding a 16 bytes
+16..    metadata: VornMetadata (bincode)  — map_name, seed, date, voronia_version, azgaar_version
+16+meta.. payload: World (bincode)        — World Data Model serializado
```

Header fijo: **16 bytes**. Permite forward/backward compat via `format_version` + `compression` flag. Metadata permite identificar un .vorn sin deserializar el World completo.

### 2.2 `VornMetadata`

| Campo | Tipo | Descripción |
|---|---|---|
| `map_name` | `String` | Nombre del mapa |
| `seed` | `String` | Semilla procedural |
| `date` | `String` | Fecha de creación/exportación |
| `voronia_version` | `String` | Versión de Voronia que generó el archivo |
| `azgaar_version` | `Option<String>` | Versión de Azgaar de la que se importó |

### 2.3 API pública (`vor-format`)

```rust
// Guardar
save::save(path, &world, &metadata) -> Result<(), FormatError>
save::save_world(path, &world) -> Result<(), FormatError>   // metadata inferida del World

// Cargar
load::load(path) -> Result<(World, VornMetadata), FormatError>
load::load_metadata(path) -> Result<VornMetadata, FormatError>  // sin deserializar World

// Tipos
VornHeader::new(metadata_len) -> VornHeader
VornHeader::encode(&self) -> [u8; 16]
VornHeader::decode(&[u8; 16]) -> Result<VornHeader, FormatError>

VornMetadata::new(map_name, seed, date, voronia_version, azgaar_version)
```

### 2.4 Errores

`FormatError` enum con 5 variantes: `Io`, `Bincode`, `InvalidMagic`, `UnsupportedVersion`, `ChecksumMismatch`.

---

## 3. Integración en vor-app

### 3.1 Autosave periódico

- **Toggle** en SidePanel izquierdo (sección "autosave"), default ON
- **Intervalo**: cada 60 segundos (fijo, configurable en código)
- **Destino**: `<ruta-del-.map>.vorn` (mismo directorio, extensión cambiada)
- **Mecanismo**: chequeo post-redraw en el event loop, save no-bloqueante (~18ms no justifica thread separado)

### 3.2 Save manual

Botón **"save .vorn ahora"** en el panel lateral → guarda inmediatamente.

### 3.3 CLI `--export-vorn`

```
cargo run --bin vor-app -- /ruta/mapa.map --export-vorn
```

Carga el .map, regenera geometría, exporta .vorn, sale sin abrir ventana.

---

## 4. Fix para bincode + `serde_json::Value`

**Problema**: bincode 1.x no soporta `deserialize_any()`, que es lo que `serde_json::Value` usa internamente en su `Deserialize` impl. Al intentar deserializar un `World` con bincode, falla con `DeserializeAnyNotSupported`.

**Solución**: módulo `vor_core::serde_json_string` con serializador/deserializador custom que convierte `Value`↔`String` (JSON textual). Aplicado con `#[serde(with = "crate::serde_json_string")]` en todos los campos `serde_json::Value` del modelo:

- `World::fonts`, `custom_good_icons`, `goods`, `markets`, `deals`
- `MapCoordinates::extras`
- `Settings::options`, `rescale_labels`, `urban_density`, `growth_rate`
- `State::diplomacy`, `campaigns`, `military`
- `CoatOfArms::payload`

---

## 5. Benchmark

**Setup**: Criterion, Sorvik (5MB .map, 10000 grid cells, 7268 pack cells), sample size 10, medición 10s.

| Operación | Tiempo | vs import |
|---|---|---|
| Import completo `.map` (parse + geom regenerate) | **90.3 ms** | 1× |
| Save `.vorn` | **17.9 ms** | 5× más rápido |
| Load `.vorn` | **37.3 ms** | **2.4× más rápido** |

La carga de `.vorn` es 2.4× más rápida que importar desde `.map` porque salta el parsing de slots, regeneración de Delaunay/Voronoi y repacking. El save es aún más rápido (solo serialización bincode en memoria + escritura).

---

## 6. Inventario de tests

| Archivo | Tests | Qué valida |
|---|---|---|
| `vor-format/src/lib.rs` | 5 | roundtrip default World, roundtrip file, invalid magic, unsupported version, save_world convenience |
| `vor-format/benches/vorn_benchmark.rs` | 2 benchmarks | full_import (90ms), save_vorn (18ms), load_vorn (37ms) |

### Todos los tests del workspace (53 tests)

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

## 7. Archivos creados/modificados

### Creados (6 archivos)

| Archivo | Líneas | Propósito |
|---|---|---|
| `crates/vor-format/src/header.rs` | 94 | VornHeader + VornMetadata, encode/decode |
| `crates/vor-format/src/save.rs` | 42 | save() + save_world() |
| `crates/vor-format/src/load.rs` | 45 | load() + load_metadata() |
| `crates/vor-format/src/error.rs` | 19 | FormatError enum |
| `crates/vor-format/src/lib.rs` | 145 | re-exports + 5 tests |
| `crates/vor-format/benches/vorn_benchmark.rs` | 62 | benchmark criterion |
| `crates/vor-core/src/serde_json_string.rs` | 20 | serde helper para bincode compat |

### Modificados (16 archivos)

| Archivo | Cambio |
|---|---|
| `.opencode/skills/voronia-dev/SKILL.md` | `.gmap` → `.vorn` en descripción y tabla stack |
| `references/architecture.md` | `.gmap` → `.vorn`, descripción actualizada |
| `references/conventions.md` | `.gmap` → `.vorn` en benchmark ref |
| `references/status.md` | Fase 4 marcada COMPLETADA, nombre del formato, resultados benchmark |
| `README.md` | `.gmap` → `.vorn` en árbol de crates |
| `voronia-plan-proyecto.md` | 12 ocurrencias `.gmap`→`.vorn`, §8.2 actualizado con decisión naming, item naming tildado, riesgo colisión resuelto |
| `docs/fase-1.md` | `.gmap` → `.vorn` en tabla de fases futuras |
| `crates/vor-format/Cargo.toml` | descripción, deps serde/bincode/thiserror, dev-deps criterion+vor-import, [[bench]] |
| `crates/vor-core/src/lib.rs` | `pub mod serde_json_string` |
| `crates/vor-core/src/world.rs` | `#[serde(with = "...")]` en 5 campos Value |
| `crates/vor-core/src/settings.rs` | `#[serde(with = "...")]` en 4 campos Value |
| `crates/vor-core/src/coordinates.rs` | `#[serde(with = "...")]` en extras |
| `crates/vor-core/src/entities/state.rs` | `#[serde(with = "...")]` en diplomacy/campaigns/military |
| `crates/vor-core/src/entities/coat_of_arms.rs` | `#[serde(with = "...")]` en payload |
| `crates/vor-app/Cargo.toml` | + `vor-format` dep |
| `crates/vor-app/src/lib.rs` | autosave fields + toggle + botón save + `--export-vorn` CLI |

---

## 8. Estado final

- **Fase 4**: ✓ COMPLETADA (plan maestro §23, checkbox tildado)
- **Working tree**: limpio (commit `ee3ec2e` pusheado a main)
- **Tests**: 53 verdes
- **clippy**: 0 errors
- **fmt**: clean
- **Benchmarks**: 2 groups (import_map, vorn_save_load)

---

## 9. Comandos de verificación

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
cargo bench --package vor-format
cargo run --bin vor-app -- /ruta/mapa.map --export-vorn
```

---

*Fin del registro Fase 4. Congelado en commit `ee3ec2e` (27 jul 2026). Próxima actualización: `docs/fase-5.md` al cerrar Fase 5.*
