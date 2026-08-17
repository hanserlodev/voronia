//! Native religion generation (FMG `religions-generator.ts`).
//!
//! Port of `generate()`: folk religions (one per culture) + organized religions
//! (placed in populated cells, spread out), then expansion via `FlatQueue`
//! restricted by `expansion` mode (culture/state/global).

use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use vor_core::entities::culture::Culture;
use vor_core::entities::religion::{Religion, ReligionExpansion, ReligionType};
use vor_core::Pack;

use crate::provinces::FlatQueue;

fn rn(value: f32, decimals: i32) -> f32 {
    let m = 10f32.powi(decimals);
    (value * m).round() / m
}

/// FMG `generate()` + `expandReligions()` (deterministic with `seed`).
///
/// `religions_number` mirrors FMG's `#religionsNumber`; `growth_rate` mirrors
/// `#growthRate`; `size_variety` is used for expansionism scaling.
pub fn generate_religions(
    pack: &mut Pack,
    cultures: &[Culture],
    religions_number: usize,
    growth_rate: f32,
    size_variety: f32,
    seed: u64,
) -> Vec<Religion> {
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    let n = pack.points_n();

    // Folk religions: one per culture (id 0 = Wildlands placeholder skipped).
    let mut religions: Vec<Religion> = vec![Religion::default()];
    let mut religion_ids: Vec<u16> = vec![0; n];

    let mut folk: Vec<(u16, usize)> = Vec::new(); // (culture_id, center)
    for c in cultures.iter().filter(|c| c.id != 0 && !c.removed) {
        folk.push((c.id, c.center_cell as usize));
    }

    // Assign folk religion cells: all cells of that culture.
    for &(culture_id, center) in &folk {
        let rid = religions.len() as u16;
        let color = format!("#{:06x}", rng.gen_range(0..0xFFFFFFu32));
        religions.push(Religion {
            id: rid,
            name: format!("Culture {} Faith", culture_id),
            color,
            kind: ReligionType::Folk,
            expansion: ReligionExpansion::Culture,
            center_cell: center as u32,
            ..Default::default()
        });
    }

    // Organized religions: placed in populated cells, spread out.
    let base_colors = [
        "#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00", "#ffff33", "#a65628", "#f781bf",
    ];
    let mut sorted: Vec<usize> = (0..n)
        .filter(|&i| pack.cells.score.get(i).copied().unwrap_or(0) > 2)
        .collect();
    sorted.sort_by(|&a, &b| {
        pack.cells
            .score
            .get(b)
            .copied()
            .unwrap_or(0)
            .partial_cmp(&pack.cells.score.get(a).copied().unwrap_or(0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let spacing = (n as f32 / (religions_number as f32).max(1.0)).sqrt();
    let mut placed: Vec<usize> = Vec::new();
    for &c in &sorted {
        if placed.len() >= religions_number {
            break;
        }
        if placed.iter().any(|&o| {
            let dx = pack.points.get(c).map(|p| p[0]).unwrap_or(0.0)
                - pack.points.get(o).map(|p| p[0]).unwrap_or(0.0);
            let dy = pack.points.get(c).map(|p| p[1]).unwrap_or(0.0)
                - pack.points.get(o).map(|p| p[1]).unwrap_or(0.0);
            (dx * dx + dy * dy).sqrt() < spacing
        }) {
            continue;
        }
        placed.push(c);
    }

    for (idx, &center) in placed.iter().enumerate() {
        let rid = religions.len() as u16;
        let kind = if idx % 10 < 1 {
            ReligionType::Cult
        } else if idx % 10 < 4 {
            ReligionType::Heresy
        } else {
            ReligionType::Organized
        };
        let expansion = match rng.gen_range(0..3) {
            0 => ReligionExpansion::Culture,
            1 => ReligionExpansion::State,
            _ => ReligionExpansion::Global,
        };
        let expansionism = rn(rng.gen::<f32>() * size_variety / 2.0 + 1.0, 1).max(0.1);
        let color = base_colors[idx % base_colors.len()].to_string();
        religion_ids[center] = rid;
        religions.push(Religion {
            id: rid,
            name: format!("{} Religion", idx + 1),
            color,
            kind,
            expansion,
            expansionism,
            center_cell: center as u32,
            ..Default::default()
        });
    }

    // --- expandReligions ---
    let max_expansion_cost = (n as f32) / 20.0 * growth_rate;
    let mut queue = FlatQueue::new();
    let mut cost: Vec<f32> = vec![0.0; n];

    for r in religions
        .iter()
        .filter(|r| r.id != 0 && r.kind != ReligionType::Folk)
    {
        let center = r.center_cell as usize;
        religion_ids[center] = r.id;
        let state = pack.cells.state.get(center).copied().unwrap_or(0);
        queue.push((center, 0.0, r.id, state), 0.0);
        cost[center] = 1.0;
    }

    while !queue.is_empty() {
        let (cell_id, p, r, state) = queue.pop().unwrap();
        let religion = religions.get(r as usize).cloned().unwrap_or_default();
        let culture = pack.cells.culture.get(cell_id).copied().unwrap_or(0);

        let neighbors = pack
            .cells
            .adjacency
            .get(cell_id)
            .cloned()
            .unwrap_or_default();
        for nb in neighbors {
            let next = nb as usize;
            if next >= n {
                continue;
            }
            match religion.expansion {
                ReligionExpansion::Culture => {
                    if culture != pack.cells.culture.get(next).copied().unwrap_or(0) {
                        continue;
                    }
                }
                ReligionExpansion::State => {
                    if state != pack.cells.state.get(next).copied().unwrap_or(0) {
                        continue;
                    }
                }
                ReligionExpansion::Global => {}
            }

            let culture_cost = if culture != pack.cells.culture.get(next).copied().unwrap_or(0) {
                10.0
            } else {
                0.0
            };
            let state_cost = if state != pack.cells.state.get(next).copied().unwrap_or(0) {
                10.0
            } else {
                0.0
            };
            let passage_cost = 10.0; // simplified biome passage cost
            let cell_cost = culture_cost + state_cost + passage_cost;
            let total_cost = p + 10.0 + cell_cost / religion.expansionism;

            if total_cost > max_expansion_cost {
                continue;
            }
            let existing = cost.get(next).copied().unwrap_or(0.0);
            if existing == 0.0 || total_cost < existing {
                if pack.cells.culture.get(next).copied().unwrap_or(0) != 0 {
                    religion_ids[next] = r;
                }
                cost[next] = total_cost;
                queue.push((next, total_cost, r, state), total_cost);
            }
        }
    }

    pack.cells.religion = religion_ids;
    religions
}
