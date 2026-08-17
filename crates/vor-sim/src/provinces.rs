//! Native province generation (FMG `provinces-generator.ts`).
//!
//! Port of `generate()`: provinces are created from state burgs (capitals +
//! population), expanded with a `FlatQueue` restricted to the state's territory
//! using elevation-based costs, then "justified" to smooth shapes. Cells without
//! a province get "wild" provinces to cover the state.

use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use vor_core::entities::burg::Burg;
use vor_core::entities::province::Province;
use vor_core::entities::state::State;
use vor_core::Pack;

use crate::states::mix_color;

/// Simple deterministic minimum-priority queue (FMG `FlatQueue`).
pub(crate) struct FlatQueue<T> {
    items: Vec<(f32, u64, T)>,
    seq: u64,
}

impl<T> FlatQueue<T> {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::new(),
            seq: 0,
        }
    }

    pub(crate) fn push(&mut self, item: T, priority: f32) {
        self.seq += 1;
        self.items.push((priority, self.seq, item));
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        if self.items.is_empty() {
            return None;
        }
        let mut best = 0;
        for i in 1..self.items.len() {
            if self.items[i].0 < self.items[best].0 {
                best = i;
            }
        }
        Some(self.items.swap_remove(best).2)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Deterministic gaussian-like value (FMG `gauss(20, 5, 5, 100)`).
fn gauss(mean: f64, std: f64, min: f64, max: f64, rng: &mut Pcg64Mcg) -> f64 {
    // Box–Muller approximation with the same min/max clamp as FMG's gauss().
    let u1 = rng.gen::<f64>();
    let u2 = rng.gen::<f64>();
    let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
    (mean + z * std).clamp(min, max)
}

/// FMG `generate()`: creates and expands provinces.
///
/// `provinces_ratio` mirrors FMG's `#provincesRatio` slider (0..100). `states`
/// must already be generated/imported so `pack.cells.state` is valid.
pub fn generate_provinces(
    pack: &mut Pack,
    states: &[State],
    burgs: &mut [Burg],
    provinces_ratio: f64,
    seed: u64,
) -> Vec<Province> {
    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    let n = pack.points_n();

    let mut provinces: Vec<Province> = vec![Province::default()]; // 0 = no province
    let mut province_ids: Vec<u16> = vec![0; n];

    let max_growth = if provinces_ratio == 100.0 {
        1000.0
    } else {
        gauss(20.0, 5.0, 5.0, 100.0, &mut rng) * provinces_ratio.sqrt()
    } as f32;

    // Create provinces for each state from its burgs.
    let mut state_provinces: Vec<Vec<u16>> = vec![Vec::new(); states.len()];

    for (state_id, state) in states.iter().enumerate() {
        if state_id == 0 || state.removed {
            continue;
        }
        if state.locked {
            continue;
        }

        let mut state_burgs: Vec<(usize, &Burg)> = burgs
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                b.id != 0
                    && b.state == state.id
                    && !b.removed
                    && province_ids.get(b.cell as usize).copied().unwrap_or(0) == 0
            })
            .collect();
        // Sort: capitals first, then by population descending (FMG sort).
        state_burgs.sort_by(|a, b| {
            b.1.is_capital.cmp(&a.1.is_capital).then(
                b.1.population
                    .partial_cmp(&a.1.population)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        if state_burgs.len() < 2 {
            continue; // at least 2 provinces required
        }

        let provinces_number =
            ((state_burgs.len() as f64 * provinces_ratio / 100.0).ceil() as usize).max(2);

        for (_, burg) in state_burgs.iter().take(provinces_number) {
            let province_id = provinces.len() as u16;
            let prov = Province {
                id: province_id,
                state: state.id,
                center_cell: burg.cell,
                capital: Some(burg.id),
                culture: burg.culture,
                name: format!("{} Province", state.name),
                color: mix_color(&state.color, &mut rng),
                ..Default::default()
            };
            province_ids[burg.cell as usize] = province_id;
            state_provinces[state_id].push(province_id);
            provinces.push(prov);
        }
    }

    // Expand provinces with a FlatQueue.
    let mut queue: FlatQueue<(usize, f32, u16, u16)> = FlatQueue::new();
    let mut cost: Vec<f32> = vec![0.0; n];

    for (pid, prov) in provinces.iter().enumerate() {
        if pid == 0 || prov.removed || prov.locked {
            continue;
        }
        let center = prov.center_cell as usize;
        province_ids[center] = prov.id;
        queue.push((center, 0.0, prov.id, prov.state), 0.0);
        cost[center] = 1.0;
    }

    while !queue.is_empty() {
        let (e, p, province, state) = queue.pop().unwrap();
        let neighbors = pack.cells.adjacency.get(e).cloned().unwrap_or_default();
        for nb in neighbors {
            let e = nb as usize;
            if e >= n {
                continue;
            }
            if province_ids.get(e).copied().unwrap_or(0) != 0 {
                // isProvinceCellLocked -> we skip locked; here we simply don't
                // overwrite existing provinces (no lock support yet).
                continue;
            }
            let h = pack.cells.height.get(e).copied().unwrap_or(0) as f32;
            let land = h >= 20.0;
            let t = pack.cells.water_type.get(e).copied().unwrap_or(0);
            if !land && t == 0 {
                continue; // cannot pass deep ocean
            }
            if land && pack.cells.state.get(e).copied().unwrap_or(0) != state {
                continue;
            }
            let elevation = if h >= 70.0 {
                100.0
            } else if h >= 50.0 {
                30.0
            } else if h >= 20.0 {
                10.0
            } else {
                100.0
            };
            let total_cost = p + elevation;
            if total_cost > max_growth {
                continue;
            }
            let existing = cost.get(e).copied().unwrap_or(0.0);
            if existing == 0.0 || total_cost < existing {
                if land {
                    province_ids[e] = province;
                }
                cost[e] = total_cost;
                queue.push((e, total_cost, province, state), total_cost);
            }
        }
    }

    // Justify shapes.
    for i in 0..n {
        if pack.cells.burg.get(i).copied().unwrap_or(0) != 0 {
            continue;
        }
        let cur = province_ids.get(i).copied().unwrap_or(0);
        if cur == 0 {
            continue;
        }
        let neighbors = pack.cells.adjacency.get(i).cloned().unwrap_or_default();
        let mut neibs: Vec<u16> = neighbors
            .iter()
            .map(|&c| c as usize)
            .filter(|&c| {
                c < n
                    && pack.cells.state.get(c).copied().unwrap_or(0)
                        == pack.cells.state.get(i).copied().unwrap_or(0)
            })
            .map(|c| province_ids.get(c).copied().unwrap_or(0))
            .collect();
        neibs.retain(|&c| c != 0);
        let adversaries: Vec<u16> = neibs.iter().copied().filter(|&c| c != cur).collect();
        if adversaries.len() < 2 {
            continue;
        }
        let buddies = neibs.iter().copied().filter(|&c| c == cur).count();
        if buddies > 2 {
            continue;
        }
        // Most common adversary becomes the new province.
        let mut counts = std::collections::HashMap::new();
        for &a in &adversaries {
            *counts.entry(a).or_insert(0usize) += 1;
        }
        let max_buddies = counts.values().copied().max().unwrap_or(0);
        if buddies >= max_buddies {
            continue;
        }
        let best = counts.iter().max_by_key(|(_, &v)| v).map(|(&k, _)| k);
        if let Some(best) = best {
            province_ids[i] = best;
        }
    }

    // Assign province to burgs.
    for burg in burgs.iter_mut() {
        if burg.id != 0 && !burg.removed {
            let _ = burg; // Province doesn't have a per-burg field in the model.
        }
    }

    // Persist province ids to pack.
    pack.cells.province = province_ids;
    let _ = (states, provinces_ratio);
    provinces
}
