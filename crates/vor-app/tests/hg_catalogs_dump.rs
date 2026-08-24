//! Fase 0 — Matriz de paridad: dump de catálogos de Human Geography & Economy.
//!
//! Vuelca a `/tmp/voronia_hg_catalogs.json` los catálogos y arrays de celdas de
//! states, provinces, cultures, religions, burgs, routes, zones y los campos
//! económicos (goods/markets/deals, hoy opacos como `serde_json::Value`) del
//! mapa de referencia Sorvik. Sirve como contrato de datos congelado contra el
//! que validar la paridad con FMG.

use std::fmt::Write;

use vor_import::mapfile::{raw, Loader};

const SORVIK_MAP_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vor-import/tests/reference/Sorvik-2026-07-24-23-39.map"
);

#[test]
fn dump_human_geography_catalogs() {
    let bytes = std::fs::read(SORVIK_MAP_PATH).expect("Sorvik.map");
    let raw = raw::parse(&bytes).expect("raw parse");
    let loaded = Loader::load(&raw).expect("loader");
    let world = &loaded.world;
    let pack = &world.pack;

    let mut s = String::new();
    let _ = write!(
        s,
        "{{\n  \"states\": {},\n  \"provinces\": {},\n  \"cultures\": {},\n  \"religions\": {},\n",
        serde_json::to_string_pretty(&world.states).unwrap(),
        serde_json::to_string_pretty(&world.provinces).unwrap(),
        serde_json::to_string_pretty(&world.cultures).unwrap(),
        serde_json::to_string_pretty(&world.religions).unwrap(),
    );
    let _ = write!(
        s,
        "  \"burgs\": {},\n  \"routes\": {},\n  \"zones\": {},\n",
        serde_json::to_string_pretty(&world.burgs).unwrap(),
        serde_json::to_string_pretty(&world.routes).unwrap(),
        serde_json::to_string_pretty(&world.zones).unwrap(),
    );
    let _ = write!(
        s,
        "  \"cells\": {{\n    \"state\": {},\n    \"province\": {},\n    \"culture\": {},\n    \"religion\": {},\n    \"burg\": {},\n    \"population\": {},\n    \"good\": {},\n    \"market\": {}\n  }},\n",
        serde_json::to_string(&pack.cells.state).unwrap(),
        serde_json::to_string(&pack.cells.province).unwrap(),
        serde_json::to_string(&pack.cells.culture).unwrap(),
        serde_json::to_string(&pack.cells.religion).unwrap(),
        serde_json::to_string(&pack.cells.burg).unwrap(),
        serde_json::to_string(&pack.cells.population).unwrap(),
        serde_json::to_string(&pack.cells.good).unwrap(),
        serde_json::to_string(&pack.cells.market).unwrap(),
    );
    let _ = write!(
        s,
        "  \"goods\": {},\n  \"markets\": {},\n  \"deals\": {}\n}}\n",
        serde_json::to_string(&world.goods).unwrap(),
        serde_json::to_string(&world.markets).unwrap(),
        serde_json::to_string(&world.deals).unwrap(),
    );

    let out = "/tmp/voronia_hg_catalogs.json";
    std::fs::write(out, &s).expect("write dump");
    println!("wrote {out} ({} bytes)", s.len());

    println!(
        "summary: {} states, {} provinces, {} cultures, {} religions, {} burgs, {} routes, {} zones, {} cells",
        world.states.len(),
        world.provinces.len(),
        world.cultures.len(),
        world.religions.len(),
        world.burgs.len(),
        world.routes.len(),
        world.zones.len(),
        pack.points_n(),
    );

    // Isoline parity: number of closed boundary polygons the common engine
    // produces per region type vs the catalog count (FMG `getIsolines`).
    use vor_render::isoline::{get_isolines, IsolineOptions};
    let opts = IsolineOptions {
        polygons: true,
        ..Default::default()
    };
    for (name, ty) in [
        ("state", pack.cells.state.clone()),
        ("province", pack.cells.province.clone()),
        ("culture", pack.cells.culture.clone()),
        ("religion", pack.cells.religion.clone()),
    ] {
        let iso = get_isolines(pack, &|c| ty.get(c).copied().unwrap_or(0), &opts);
        let distinct: std::collections::BTreeSet<u16> =
            ty.iter().copied().filter(|&t| t != 0).collect();
        println!(
            "isolines[{name}]: {} polygons for {} distinct types",
            iso.len(),
            distinct.len()
        );
    }

    // Zone mesh: outer-boundary fill per zone (FMG `getVertexPath`).
    let zone_mesh = vor_render::zone_layer::build_zone_hatch_mesh(pack, &world.zones);
    println!(
        "zones mesh: {}v/{}i ({} zones)",
        zone_mesh.vertices.len(),
        zone_mesh.indices.len(),
        world.zones.len()
    );
    assert!(
        zone_mesh.vertices.iter().all(|v| v.pos[0].is_finite()),
        "zone mesh vertices must be finite"
    );

    // Native state generation (FMG states-generator). Requires a mutable pack and
    // burg catalog. Validate counts + determinism.
    let mut pack2 = world.pack.clone();
    let mut burgs2 = world.burgs.clone();
    let capitals = burgs2.iter().filter(|b| b.is_capital).count();
    let states_a = vor_sim::states::generate_states(
        &mut pack2,
        &mut burgs2,
        &world.cultures,
        &world.biomes,
        10.0,
        1.0,
        42,
    );
    let assigned: usize = pack2.cells.state.iter().filter(|&&s| s != 0).count();
    println!(
        "native states: {} ({} capital burgs), {} cells assigned",
        states_a.len(),
        capitals,
        assigned
    );
    for st in states_a.iter() {
        if st.id != 0 {
            println!(
                "  state {}: id={} center={} color={:?} expansion={}",
                st.name, st.id, st.center_cell, st.color, st.expansionism
            );
        }
    }
    assert!(
        states_a.len() >= 2,
        "expected at least placeholder + states"
    );
    assert!(assigned > 0, "state expansion must assign cells");
    assert!(
        states_a
            .iter()
            .filter(|s| s.id != 0)
            .all(|s| s.color.starts_with('#')),
        "real states must have hex colors"
    );

    // Determinism: same seed must give identical results.
    let mut pack3 = world.pack.clone();
    let mut burgs3 = world.burgs.clone();
    let states_b = vor_sim::states::generate_states(
        &mut pack3,
        &mut burgs3,
        &world.cultures,
        &world.biomes,
        10.0,
        1.0,
        42,
    );
    assert_eq!(
        states_a
            .iter()
            .map(|s| (s.id, s.name.clone(), s.color.clone(), s.expansionism))
            .collect::<Vec<_>>(),
        states_b
            .iter()
            .map(|s| (s.id, s.name.clone(), s.color.clone(), s.expansionism))
            .collect::<Vec<_>>(),
        "state generation must be deterministic for the same seed"
    );

    // Native province generation on the freshly generated states.
    let mut pack4 = world.pack.clone();
    let mut burgs4 = world.burgs.clone();
    let _states4 = vor_sim::states::generate_states(
        &mut pack4,
        &mut burgs4,
        &world.cultures,
        &world.biomes,
        10.0,
        1.0,
        7,
    );
    let provinces =
        vor_sim::provinces::generate_provinces(&mut pack4, &_states4, &mut burgs4, 30.0, 99);
    let prov_assigned: usize = pack4.cells.province.iter().filter(|&&p| p != 0).count();
    println!(
        "native provinces: {} ({} cells assigned)",
        provinces.len(),
        prov_assigned
    );
    assert!(
        provinces.len() >= 2,
        "expected placeholder + generated provinces"
    );
    assert!(prov_assigned > 0, "province expansion must assign cells");
    assert!(
        provinces
            .iter()
            .filter(|p| p.id != 0)
            .all(|p| p.color.starts_with('#')),
        "provinces must have hex colors"
    );

    // Native culture generation.
    let mut pack5 = world.pack.clone();
    let cultures =
        vor_sim::cultures::generate_cultures(&mut pack5, &world.biomes, 10, 10.0, 1.0, 123);
    let cult_assigned: usize = pack5.cells.culture.iter().filter(|&&c| c != 0).count();
    println!(
        "native cultures: {} ({} cells assigned)",
        cultures.len(),
        cult_assigned
    );
    assert!(
        cultures.len() >= 2,
        "expected Wildlands + generated cultures"
    );
    assert!(cult_assigned > 0, "culture expansion must assign cells");
    assert!(
        cultures
            .iter()
            .filter(|c| c.id != 0)
            .all(|c| c.color.starts_with('#')),
        "cultures must have hex colors"
    );

    // Native religion generation.
    let mut pack6 = world.pack.clone();
    let religions =
        vor_sim::religions::generate_religions(&mut pack6, &world.cultures, 12, 1.0, 10.0, 77);
    let relig_assigned: usize = pack6.cells.religion.iter().filter(|&&r| r != 0).count();
    println!(
        "native religions: {} ({} cells assigned)",
        religions.len(),
        relig_assigned
    );
    assert!(religions.len() >= 2, "expected placeholder + religions");
    assert!(relig_assigned > 0, "religion expansion must assign cells");
    assert!(
        religions
            .iter()
            .filter(|r| r.id != 0)
            .all(|r| r.color.starts_with('#')),
        "religions must have hex colors"
    );

    // Population bars (rural + urban vertical bars, FMG drawPopulation).
    let pop_mesh = vor_render::population_layer::build_population_bars_mesh(
        &pack.vertices,
        pack,
        &world.burgs,
        1.0,
    );
    println!(
        "population bars: {}v/{}i",
        pop_mesh.vertices.len(),
        pop_mesh.indices.len()
    );
    assert!(!pop_mesh.vertices.is_empty(), "population bars must draw");

    // Routes: tessellated strokes per subgroup (roads/trails/searoutes).
    let route_meshes = vor_render::route_layer::build_route_group_meshes(&world.routes);
    println!(
        "routes: roads {}v, trails {}v, searoutes {}v",
        route_meshes.roads.vertices.len(),
        route_meshes.trails.vertices.len(),
        route_meshes.searoutes.vertices.len()
    );
    let route_total = route_meshes.roads.vertices.len()
        + route_meshes.trails.vertices.len()
        + route_meshes.searoutes.vertices.len();
    assert!(route_total > 0, "routes must draw");

    // Burg icons (circles per burg, colored by state).
    let burg_mesh = vor_render::burg::build_burg_icons_mesh(&world.burgs);
    println!(
        "burg icons: {}v/{}i",
        burg_mesh.vertices.len(),
        burg_mesh.indices.len()
    );
    assert!(!burg_mesh.vertices.is_empty(), "burg icons must draw");

    // Goods: typed catalog + cells/icons sub-layers.
    println!("goods catalog: {} goods", world.goods.len());
    assert!(world.goods.len() > 1, "goods catalog must be parsed");
    let goods_cells = vor_render::goods::build_goods_cells_mesh(pack, &world.goods);
    let goods_icons = vor_render::goods::build_goods_icons_mesh(pack, &world.goods);
    println!(
        "goods cells mesh: {}v/{}i, icons: {}v/{}i",
        goods_cells.vertices.len(),
        goods_cells.indices.len(),
        goods_icons.vertices.len(),
        goods_icons.indices.len()
    );
    assert!(!goods_icons.vertices.is_empty(), "goods icons must draw");

    // Markets: typed catalog + render (fill/border/center).
    println!("markets catalog: {}", world.markets.len());
    assert!(world.markets.len() > 1, "markets catalog must be parsed");
    let m_fill = vor_render::market::build_market_fill_mesh(pack, &world.markets);
    let m_border = vor_render::market::build_market_border_mesh(pack, &world.markets);
    let m_center = vor_render::market::build_market_center_mesh(&world.markets, &world.burgs);
    println!(
        "markets mesh: fill {}v/{}i, border {}v/{}i, center {}v/{}i",
        m_fill.vertices.len(),
        m_fill.indices.len(),
        m_border.vertices.len(),
        m_border.indices.len(),
        m_center.vertices.len(),
        m_center.indices.len()
    );
    assert!(!m_center.vertices.is_empty(), "market centers must draw");

    // Trade: typed deals + routes mesh.
    println!("deals catalog: {}", world.deals.len());
    assert!(world.deals.len() > 1, "deals catalog must be parsed");
    let trade_mesh = vor_render::trade::build_trade_routes_mesh(
        &world.deals,
        &world.burgs,
        &world.markets,
        &world.goods,
    );
    println!(
        "trade routes mesh: {}v/{}i",
        trade_mesh.vertices.len(),
        trade_mesh.indices.len()
    );
    assert!(!trade_mesh.vertices.is_empty(), "trade routes must draw");
}
