//! Verificación A — cross-consistency de atributos con producción FMG (mapa real de Sorvik,
//! azgaar.github.io v1.138.0, exportado el 24 Jul 2026).
//!
//! El `.map` de FMG NO serializa la geometría (Delaunay/Voronoi/reGraph/pack-mesh). La única
//! geometría real anclada a producción es `slot[6].points` (verificado bit-exacto en
//! `sorvik_handshake.rs`). Este archivo cierra lo que sí se puede verificar de forma objetiva:
//! **consistencia cruzada** entre los atributos indexados por pack-id y la geometría que
//! regenera Voronia, usando dos señales independientes del archivo:
//!
//! 1. `pack.cells.grid_id` (re_graph) → cada pack cell debe apuntar a un grid cell válido,
//!    sin duplicados, y `pack.cells.height[p]` debe coincidir con `grid.cells.height[grid_id[p]]`
//!    (la altura proviene de `slot[7]`, es decir FMG la escribió indexada por grid cell).
//! 2. `pack.cells.feature_id` (derivada vía grid_id → grid.feature_id) debe ser coherente con
//!    `pack.features` (slot `[12]`): el conteo de celdas por feature debe coincidir con el
//!    `cell_count`/`firstCell` que FMG serializó.
//! 3. Catálogos (slots `[14]`/`[15]`...) → los ids que referencia cada pack cell
//!    (`state`, `burg`, `culture`, `religion`, `province`) deben existir en los catálogos.
//!
//! Qué NO puede probar este archivo: que el orden de pack ids de Voronia sea idéntico al de FMG.
//! Eso solo es comprobable visualmente (verificación B) o contra la réplica JS self-referencial.

use vor_import::mapfile::{raw, Loader};

const SORVIK_MAP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/reference/Sorvik-2026-07-24-23-39.map"
);

fn load_sorvik() -> vor_import::mapfile::LoadResult {
    let bytes = std::fs::read(SORVIK_MAP_PATH)
        .expect("Sorvik.map must exist in crates/vor-import/tests/reference/");
    let raw = raw::parse(&bytes).expect("raw parse");
    Loader::load(&raw).expect("loader succeeds on Sorvik")
}

/// 1) `pack.cells.grid_id` es un mapeo válido pack→grid: todos los ids en rango, y la altura
/// replica exactamente la del grid cell de origen (slot `[7]`). Nótese que `re_graph` añade
/// puntos intermedios en costas que comparten el mismo `grid_id` del cell de origen — por eso
/// NO se exige unicidad del grid_id (cada pack cell apunta al grid cell que la originó).
#[test]
fn pack_grid_id_is_valid_and_height_replicates() {
    let w = load_sorvik();
    let n_grid = w.world.grid.cells.height.len();
    let n_pack = w.world.pack.cells.grid_id.len();
    assert_eq!(n_pack, 7268, "pack count");
    assert_eq!(n_grid, 10000, "grid count");

    for p in 0..n_pack {
        let gid = w.world.pack.cells.grid_id[p] as usize;
        assert!(gid < n_grid, "pack {p} → grid_id {gid} out of range");
        assert_eq!(
            w.world.pack.cells.height[p],
            w.world.grid.cells.height[gid],
            "pack {p} height must replicate grid cell {gid} height"
        );
    }
}

/// 2) `pack.cells.feature_id` (derivada vía grid_id → grid.feature_id, slot `[9]`) es coherente
/// con el catálogo de `grid.features` (slot `[6]`): todo id presente en pack debe existir como
/// feature real en grid, y ningún pack cell puede apuntar a un id inexistente. (Los conteos
/// `cell_count` que FMG serializa en `grid.features` son 0 en Sorvik — no son una señal útil —
/// y los puntos extra de costa de `re_graph` inflan el pack, por eso no se comparan conteos.)
#[test]
fn pack_feature_ids_resolve_to_grid_feature_catalog() {
    let w = load_sorvik();
    let grid = &w.world.grid;
    let pack = &w.world.pack;

    let known_ids: std::collections::HashSet<u32> =
        grid.features.iter().map(|f| f.id).collect();
    assert!(!known_ids.is_empty(), "grid.features catalog must be populated");

    for p in 0..pack.cells.feature_id.len() {
        let fid = pack.cells.feature_id[p] as u32;
        assert!(
            known_ids.contains(&fid),
            "pack {p}: feature_id {fid} not in grid.features catalog"
        );
    }
}

/// 3) Referencias de catálogo de cada pack cell son válidas (existen en los catálogos de FMG):
/// state, burg, culture, religion, province.
#[test]
fn pack_cell_catalog_references_resolve() {
    let w = load_sorvik();
    let pack = &w.world.pack;

    let state_ids: std::collections::HashSet<u16> =
        w.world.states.iter().map(|s| s.id).collect();
    let culture_ids: std::collections::HashSet<u16> = w.world.cultures.iter().map(|c| c.id).collect();
    let religion_ids: std::collections::HashSet<u16> =
        w.world.religions.iter().map(|r| r.id).collect();
    let province_ids: std::collections::HashSet<u16> =
        w.world.provinces.iter().map(|p| p.id).collect();

    for p in 0..pack.cells.state.len() {
        let s = pack.cells.state[p];
        if s != 0 {
            assert!(state_ids.contains(&s), "pack {p}: unknown state id {s}");
        }
        let c = pack.cells.culture[p];
        if c != 0 {
            assert!(culture_ids.contains(&c), "pack {p}: unknown culture id {c}");
        }
        let r = pack.cells.religion[p];
        if r != 0 {
            assert!(religion_ids.contains(&r), "pack {p}: unknown religion id {r}");
        }
        let pr = pack.cells.province[p];
        if pr != 0 {
            assert!(province_ids.contains(&pr), "pack {p}: unknown province id {pr}");
        }
    }
}

/// 4) Todo burg (slot `[15]`) referencia un pack cell existente, y ese pack cell tiene asignado
/// ese burg en `pack.cells.burg` (slot `[17]`).
#[test]
fn burg_cell_references_resolve_to_assigned_burg() {
    let w = load_sorvik();
    let pack = &w.world.pack;
    let n_pack = pack.cells.burg.len();

    for b in &w.world.burgs {
        let cell = b.cell as usize;
        assert!(cell < n_pack, "burg {} (id {}) references cell {cell} out of range", b.name, b.id);
        assert_eq!(
            pack.cells.burg[cell], b.id,
            "burg {} (id {}) expects cell {cell} assigned to itself",
            b.name, b.id
        );
    }
}
