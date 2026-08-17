//! Native culture generation (FMG `cultures-generator.ts`).
//!
//! Port of `generate()` + `expandCultures()`: selects a set of culture
//! archetypes, places centers in populated cells (spread out), defines type /
//! expansionism, then expands territories with a `FlatQueue` using FMG's cost
//! functions (biome, biome-change, height, river, type).

use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use vor_core::entities::biome::Biome;
use vor_core::entities::culture::{Culture, CultureType};
use vor_core::feature::FeatureType;
use vor_core::Pack;

use crate::provinces::FlatQueue;

fn rn(value: f32, decimals: i32) -> f32 {
    let m = 10f32.powi(decimals);
    (value * m).round() / m
}

fn minmax(v: f32, min: f32, max: f32) -> f32 {
    v.clamp(min, max)
}

fn type_name(t: &CultureType) -> &'static str {
    match t {
        CultureType::Generic => "Generic",
        CultureType::River => "River",
        CultureType::Lake => "Lake",
        CultureType::Naval => "Naval",
        CultureType::Nomadic => "Nomadic",
        CultureType::Hunting => "Hunting",
        CultureType::Highland => "Highland",
    }
}

/// FMG `defineCultureType`: decides the culture type from its center cell.
fn define_culture_type(pack: &Pack, i: usize) -> CultureType {
    let h = pack.cells.height.get(i).copied().unwrap_or(0);
    let biome = pack.cells.biome.get(i).copied().unwrap_or(0);
    if (h as f32) < 70.0 && [1, 2, 4].contains(&biome) {
        return CultureType::Nomadic;
    }
    if h > 50 {
        return CultureType::Highland;
    }
    let fid = pack.cells.feature_id.get(i).copied().unwrap_or(0);
    let f_kind = pack
        .features
        .get(fid as usize)
        .map(|f| f.kind)
        .unwrap_or(FeatureType::Ocean);
    let f_cells = pack
        .features
        .get(fid as usize)
        .map(|f| f.cell_count)
        .unwrap_or(0);
    if f_kind == FeatureType::Lake && f_cells > 5 {
        return CultureType::Lake;
    }
    let _ = (biome, h);
    if pack.cells.river.get(i).copied().unwrap_or(0) != 0
        && pack.cells.flux.get(i).copied().unwrap_or(0) > 100
    {
        return CultureType::River;
    }
    let t = pack.cells.water_type.get(i).copied().unwrap_or(0);
    if t > 2 && [3, 7, 8, 9, 10, 12].contains(&biome) {
        return CultureType::Hunting;
    }
    CultureType::Generic
}

fn define_expansionism(kind: &CultureType, size_variety: f32, rng: &mut Pcg64Mcg) -> f32 {
    let base = match kind {
        CultureType::Generic => 1.0,
        CultureType::Lake => 0.8,
        CultureType::Naval => 1.5,
        CultureType::River => 0.9,
        CultureType::Nomadic => 1.5,
        CultureType::Hunting => 0.7,
        CultureType::Highland => 1.2,
    };
    let v = ((rng.gen::<f32>() * size_variety) / 2.0 + 1.0) * base;
    rn(v, 1)
}

/// FMG `generate()` + `expandCultures()` (deterministic with `seed`).
///
/// `culture_count` mirrors FMG's `#culturesInput`; `size_variety` mirrors
/// `#sizeVariety`; `neutral_rate` mirrors `#neutralRate`.
pub fn generate_cultures(
    pack: &mut Pack,
    biomes: &[Biome],
    culture_count: usize,
    size_variety: f32,
    neutral_rate: f32,
    seed: u64,
) -> Vec<Culture> {
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    let n = pack.points_n();

    // Only a subset of the imported default catalog is available; we generate a
    // deterministic set of archetype cultures with distinct colors.
    let mut cultures: Vec<Culture> = Vec::new();
    // Culture id 0 = Wildlands.
    cultures.push(Culture {
        id: 0,
        name: "Wildlands".to_string(),
        ..Default::default()
    });

    let base_colors = [
        "#66c2a5", "#fc8d62", "#8da0cb", "#e78ac3", "#a6d854", "#ffd92f", "#8dd3c7", "#ffffb3",
        "#bebada", "#fb8072", "#80b1d3", "#fdb462", "#b3de69", "#fccde5",
    ];

    // Place centers in populated cells, spread out.
    let populated: Vec<usize> = (0..n)
        .filter(|&i| pack.cells.score.get(i).copied().unwrap_or(0) != 0)
        .collect();

    let count = culture_count.min(base_colors.len()).min(populated.len());
    let mut culture_ids: Vec<u16> = vec![0; n];
    let mut centers: Vec<usize> = Vec::new();
    let spacing = (n as f32 / (count as f32).max(1.0)).sqrt();

    let mut sorted: Vec<usize> = populated.clone();
    // sort by score descending
    sorted.sort_by(|&a, &b| {
        pack.cells
            .score
            .get(b)
            .copied()
            .unwrap_or(0)
            .partial_cmp(&pack.cells.score.get(a).copied().unwrap_or(0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut chosen: Vec<usize> = Vec::new();
    for &c in sorted.iter() {
        if chosen.len() >= count {
            break;
        }
        if chosen.iter().any(|&o| {
            let dx = pack.points.get(c).map(|p| p[0]).unwrap_or(0.0)
                - pack.points.get(o).map(|p| p[0]).unwrap_or(0.0);
            let dy = pack.points.get(c).map(|p| p[1]).unwrap_or(0.0)
                - pack.points.get(o).map(|p| p[1]).unwrap_or(0.0);
            (dx * dx + dy * dy).sqrt() < spacing
        }) {
            continue;
        }
        chosen.push(c);
    }

    for (idx, &center) in chosen.iter().enumerate() {
        let kind = define_culture_type(pack, center);
        let expansionism = define_expansionism(&kind, size_variety, &mut rng);
        let name = format!("Culture {}", idx + 1);
        let color = base_colors[idx % base_colors.len()].to_string();
        culture_ids[center] = (idx + 1) as u16;
        centers.push(center);
        cultures.push(Culture {
            id: (idx + 1) as u16,
            name,
            center_cell: center as u32,
            color,
            expansionism,
            kind,
            ..Default::default()
        });
    }

    // --- expandCultures ---
    let max_expansion_cost = (n as f32) * 0.6 * neutral_rate;
    let mut queue = FlatQueue::new();
    let mut cost: Vec<f32> = vec![0.0; n];

    for (idx, &center) in centers.iter().enumerate() {
        let culture_id = (idx + 1) as u16;
        queue.push((center, 0.0, culture_id), 0.0);
        cost[center] = 0.0;
    }

    while !queue.is_empty() {
        let (cell_id, priority, culture_id) = queue.pop().unwrap();
        let culture = cultures.get(culture_id as usize);
        let (kind, expansionism) = match culture {
            Some(c) => (c.kind, c.expansionism),
            None => continue,
        };
        let type_name = type_name(&kind);
        let source_biome = pack.cells.biome.get(cell_id).copied().unwrap_or(0);

        let neighbors = pack
            .cells
            .adjacency
            .get(cell_id)
            .cloned()
            .unwrap_or_default();
        for nb in neighbors {
            let e = nb as usize;
            if e >= n {
                continue;
            }
            let target_biome = pack.cells.biome.get(e).copied().unwrap_or(0);
            let biome_cost = {
                let native = pack
                    .cells
                    .biome
                    .get(centers.get((culture_id - 1) as usize).copied().unwrap_or(0))
                    .copied()
                    .unwrap_or(0);
                if native == target_biome {
                    10.0
                } else if type_name == "Hunting" {
                    biomes
                        .get(target_biome as usize)
                        .map(|b| b.move_cost * 5.0)
                        .unwrap_or(500.0)
                } else if type_name == "Nomadic" && (target_biome > 4 && target_biome < 10) {
                    biomes
                        .get(target_biome as usize)
                        .map(|b| b.move_cost * 10.0)
                        .unwrap_or(1000.0)
                } else {
                    biomes
                        .get(target_biome as usize)
                        .map(|b| b.move_cost * 2.0)
                        .unwrap_or(200.0)
                }
            };
            let biome_change_cost = if source_biome == target_biome {
                0.0
            } else {
                20.0
            };

            let h = pack.cells.height.get(e).copied().unwrap_or(0) as f32;
            let area = pack.cells.area_px.get(e).copied().unwrap_or(0) as f32;
            let height_cost = {
                let fid = pack.cells.feature_id.get(e).copied().unwrap_or(0);
                let f_kind = pack
                    .features
                    .get(fid as usize)
                    .map(|f| f.kind)
                    .unwrap_or(FeatureType::Ocean);
                if type_name == "Lake" && f_kind == FeatureType::Lake {
                    10.0
                } else if type_name == "Naval" && h < 20.0 {
                    area * 2.0
                } else if type_name == "Nomadic" && h < 20.0 {
                    area * 50.0
                } else if h < 20.0 {
                    area * 6.0
                } else if type_name == "Highland" && h < 44.0 {
                    3000.0
                } else if type_name == "Highland" && h < 62.0 {
                    200.0
                } else if type_name == "Highland" {
                    0.0
                } else if h >= 67.0 {
                    200.0
                } else if h >= 44.0 {
                    30.0
                } else {
                    0.0
                }
            };

            let has_river = pack.cells.river.get(e).copied().unwrap_or(0) != 0;
            let flux = pack.cells.flux.get(e).copied().unwrap_or(0);
            let river_cost = if type_name == "River" {
                if has_river {
                    0.0
                } else {
                    100.0
                }
            } else if !has_river {
                0.0
            } else {
                minmax(flux as f32 / 10.0, 20.0, 100.0)
            };

            let t = pack.cells.water_type.get(e).copied().unwrap_or(0);
            let type_cost = if t == 1 {
                match type_name {
                    "Naval" | "Lake" => 0.0,
                    "Nomadic" => 60.0,
                    _ => 20.0,
                }
            } else if t == 2 {
                match type_name {
                    "Naval" | "Nomadic" => 30.0,
                    _ => 0.0,
                }
            } else if t != -1 {
                match type_name {
                    "Naval" | "Lake" => 100.0,
                    _ => 0.0,
                }
            } else {
                0.0
            };

            let cell_cost = (biome_cost + biome_change_cost + height_cost + river_cost + type_cost)
                / expansionism;
            let total_cost = priority + cell_cost;
            if total_cost > max_expansion_cost {
                continue;
            }
            let existing = cost.get(e).copied().unwrap_or(0.0);
            if existing == 0.0 || total_cost < existing {
                if pack.cells.population.get(e).copied().unwrap_or(0.0) > 0.0 {
                    culture_ids[e] = culture_id;
                }
                cost[e] = total_cost;
                queue.push((e, total_cost, culture_id), total_cost);
            }
        }
    }

    pack.cells.culture = culture_ids;
    cultures
}
