//! Test de handshake con Sorvik .map real: regenera place_points con la semilla del
//! header y compara los puntos contra los serializados en slot [6]. Si bit-exacto,
//! el pipeline de regeneración de geometría está bien y el .map puede cargarse sobre
//! la malla nativa sin riesgo de aplicar atributos en celdas equivocadas.
use vor_import::geometry::place_points;
use vor_import::mapfile::raw;

const SORVIK_MAP_PATH: &str = "/home/hans/Descargas/Sorvik 2026-07-24-23-39.map";

#[test]
fn sorvik_grid_points_match_native_regeneration() {
    let bytes = std::fs::read(SORVIK_MAP_PATH).expect("Sorvik map path exists");
    let parsed = raw::parse(&bytes).expect("raw parse succeeds");
    let slots = parsed.slots;
    assert_eq!(slots.len(), 47, "Sorvik has 47 slots");

    let header = slots[0].split('|').collect::<Vec<_>>();
    assert_eq!(header.len(), 7, "header has 7 pipe-delimited fields");
    let version = header[0];
    assert_eq!(version, "1.138.0", "Sorvik version");
    let seed = header[3];
    let graph_width: f64 = header[4].parse().unwrap();
    let graph_height: f64 = header[5].parse().unwrap();

    let grid_json: serde_json::Value = serde_json::from_str(&slots[6]).expect("slot6 is JSON");
    let cells_desired: u32 = grid_json["cellsDesired"].as_u64().unwrap_or(10000) as u32;

    let expected_pts = grid_json["points"].as_array().expect("points array");
    let expected_pts: Vec<[f32; 2]> = expected_pts
        .iter()
        .map(|p| {
            let x = p[0].as_f64().unwrap() as f32;
            let y = p[1].as_f64().unwrap() as f32;
            [x, y]
        })
        .collect();

    let placed = place_points(graph_width, graph_height, cells_desired, seed);
    assert_eq!(
        placed.points.len(),
        expected_pts.len(),
        "place_points must yield same count as slot6"
    );

    let mismatches = placed
        .points
        .iter()
        .zip(expected_pts.iter())
        .enumerate()
        .filter(|(_, (p, q))| (p[0] as f32) != q[0] || (p[1] as f32) != q[1])
        .count();
    if mismatches > 0 {
        eprintln!(
            "First regenerated: {:?} / First expected: {:?}",
            &placed.points[..2],
            &expected_pts[..2]
        );
    }
    assert_eq!(
        mismatches,
        0,
        "Sorvik must match native regen bit-exact (got {} mismatch in {} points)",
        mismatches,
        expected_pts.len()
    );
}
