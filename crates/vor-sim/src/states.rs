//! Native state generation (FMG `states-generator.ts`).
//!
//! Port of `StatesModule.generate()`: creates one state per capital burg,
//! expands territories with a Dijkstra-like `FlatQueue` using FMG's cost
//! functions, normalizes, finds neighbors, assigns colors greedily and
//! collects statistics.
//!
//! Determinism: FMG uses `Math.random()`; Voronia requires a seeded RNG so the
//! same seed always yields the same states (see conventions.md). `expansionism`
//! and color mixing use the provided RNG.

use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use vor_core::entities::biome::Biome;
use vor_core::entities::culture::CultureType;
use vor_core::entities::state::State;
use vor_core::feature::FeatureType;
use vor_core::Pack;

/// Native biome cost table: `biomesData.cost`. Voronia reads it from the
/// `Biome::move_cost` field of the imported catalog.
pub(crate) fn rn(value: f32, decimals: i32) -> f32 {
    let m = 10f32.powi(decimals);
    (value * m).round() / m
}

fn minmax(v: f32, min: f32, max: f32) -> f32 {
    v.clamp(min, max)
}

/// Simple deterministic minimum-priority queue (FMG `FlatQueue`).
struct FlatQueue<T> {
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

fn culture_type_name(t: &CultureType) -> &'static str {
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

fn get_biome_cost(b: i32, biome: u8, type_name: &str, biomes: &[Biome]) -> f32 {
    let cost = biomes
        .get(biome as usize)
        .map(|x| x.move_cost)
        .unwrap_or(100.0);
    if b as u8 == biome {
        return 10.0; // tiny penalty for native biome
    }
    if type_name == "Hunting" {
        return cost * 2.0; // non-native biome penalty for hunters
    }
    if type_name == "Nomadic" && (biome > 4 && biome < 10) {
        return cost * 3.0; // forest biome penalty for nomads
    }
    cost
}

fn get_height_cost(kind: FeatureType, h: u8, type_name: &str) -> f32 {
    let h = h as f32;
    if type_name == "Lake" && kind == FeatureType::Lake {
        return 10.0; // low lake crossing penalty for Lake cultures
    }
    if type_name == "Naval" && h < 20.0 {
        return 300.0; // low sea crossing penalty for Navals
    }
    if type_name == "Nomadic" && h < 20.0 {
        return 10000.0; // giant sea crossing penalty for Nomads
    }
    if h < 20.0 {
        return 1000.0; // general sea crossing penalty
    }
    if type_name == "Highland" && h < 62.0 {
        return 1100.0; // penalty for highlanders on lowlands
    }
    if type_name == "Highland" {
        return 0.0;
    }
    if h >= 67.0 {
        return 2200.0; // general mountains crossing penalty
    }
    if h >= 44.0 {
        return 300.0; // general hills crossing penalty
    }
    0.0
}

fn get_river_cost(has_river: bool, flux: u16, type_name: &str) -> f32 {
    if type_name == "River" {
        return if has_river { 0.0 } else { 100.0 };
    }
    if !has_river {
        return 0.0;
    }
    minmax(flux as f32 / 10.0, 20.0, 100.0)
}

fn get_type_cost(t: i8, type_name: &str) -> f32 {
    if t == 1 {
        return match type_name {
            "Naval" | "Lake" => 0.0,
            "Nomadic" => 60.0,
            _ => 20.0,
        };
    }
    if t == 2 {
        return match type_name {
            "Naval" | "Nomadic" => 30.0,
            _ => 0.0,
        };
    }
    if t != -1 {
        return match type_name {
            "Naval" | "Lake" => 100.0,
            _ => 0.0,
        };
    }
    0.0
}

/// Full native state generation (FMG `StatesModule.generate()`).
///
/// Returns the new states catalog and mutates `pack.cells.state` (+ burg.state).
/// `burgs` is the imported/current burg catalog (its `is_capital` flags and
/// `cell`/`culture` fields drive state creation).
///
/// `size_variety` mirrors FMG's `#sizeVariety` slider. Growth-rate sliders
/// (`globalGrowthRate`, `statesGrowthRate`) are folded into the `growth_rate`
/// argument (default 1.0).
pub fn generate_states(
    pack: &mut Pack,
    burgs: &mut [vor_core::entities::burg::Burg],
    cultures: &[vor_core::entities::culture::Culture],
    biomes: &[Biome],
    size_variety: f32,
    growth_rate: f32,
    seed: u64,
) -> Vec<State> {
    let mut rng = Pcg64Mcg::seed_from_u64(seed);

    let states = expand_states(
        pack,
        burgs,
        cultures,
        biomes,
        size_variety,
        growth_rate,
        &mut rng,
    );
    let states = normalize_states(pack, burgs, states);
    let states = assign_state_neighbors(pack, states);
    let states = assign_state_colors(pack, states, &mut rng);
    collect_state_statistics(pack, burgs, states)
}

/// FMG `expandStates()`: Dijkstra-like expansion from each state's capital.
fn expand_states(
    pack: &mut Pack,
    burgs: &mut [vor_core::entities::burg::Burg],
    cultures: &[vor_core::entities::culture::Culture],
    biomes: &[Biome],
    size_variety: f32,
    growth_rate: f32,
    rng: &mut Pcg64Mcg,
) -> Vec<State> {
    let n = pack.points_n();
    let mut states: Vec<State> = Vec::new();
    states.push(State::placeholder());

    // Build the catalog from capital burgs.
    for burg in burgs.iter().filter(|b| b.id != 0 && b.is_capital) {
        let cell = burg.cell as usize;
        let culture = burg.culture as usize;
        let kind = cultures.get(culture).map(|c| c.kind).unwrap_or_default();
        let expansionism = rn(rng.gen::<f32>() * size_variety + 1.0, 1).max(0.1);
        let name = cultures
            .get(culture)
            .map(|c| format!("{} State", c.name))
            .unwrap_or_else(|| "New State".to_string());
        let mut st = State::placeholder();
        st.id = burg.id;
        st.name = name;
        st.kind = kind;
        st.expansionism = expansionism;
        st.center_cell = cell as u32;
        st.culture = burg.culture;
        states.push(st);
    }

    let growth_limit = (n as f32 / 2.0) * growth_rate;

    // Reset state on all cells except locked (locked states preserved).
    for cell in 0..n {
        let sid = pack.cells.state.get(cell).copied().unwrap_or(0) as usize;
        let state = states.get(sid);
        if state.map(|s| s.locked).unwrap_or(false) {
            continue;
        }
        if let Some(v) = pack.cells.state.get_mut(cell) {
            *v = 0;
        }
    }

    let mut queue: FlatQueue<(usize, f32, u16, i32)> = FlatQueue::new();
    let mut cost: Vec<f32> = vec![0.0; n];

    for (sid, state) in states.iter().enumerate() {
        if sid == 0 || state.removed {
            continue;
        }
        let capital_cell = state.center_cell as usize;
        pack.cells.state[capital_cell] = sid as u16;
        let culture_center = cultures
            .get(state.culture as usize)
            .map(|c| c.center_cell)
            .unwrap_or(0) as usize;
        let b = pack.cells.biome.get(culture_center).copied().unwrap_or(0);
        queue.push((capital_cell, 0.0, sid as u16, b as i32), 0.0);
        cost[capital_cell] = 1.0;
    }

    while !queue.is_empty() {
        let (e, p, s, b) = queue.pop().unwrap();
        let state = &states[s as usize];
        let type_name = culture_type_name(&state.kind);

        let neighbors = pack.cells.adjacency.get(e).cloned().unwrap_or_default();
        for nb in neighbors {
            let e = nb as usize;
            if e >= n {
                continue;
            }
            let cur_state = states
                .get(pack.cells.state.get(e).copied().unwrap_or(0) as usize)
                .cloned()
                .unwrap_or_else(State::placeholder);
            if cur_state.locked {
                continue; // do not overwrite cell of locked states
            }
            if pack.cells.state.get(e).copied().unwrap_or(0) != 0 && e == state.center_cell as usize
            {
                continue; // do not overwrite capital cells
            }

            let cell_culture = pack.cells.culture.get(e).copied().unwrap_or(0);
            let culture_cost = if state.culture == cell_culture {
                -9.0
            } else {
                100.0
            };
            let h = pack.cells.height.get(e).copied().unwrap_or(0);
            let score = pack.cells.score.get(e).copied().unwrap_or(0);
            let population_cost = if (h as f32) < 20.0 {
                0.0
            } else if score != 0 {
                (20.0 - score as f32).max(0.0)
            } else {
                5000.0
            };

            let biome = pack.cells.biome.get(e).copied().unwrap_or(0);
            let biome_cost = get_biome_cost(b, biome, type_name, biomes);

            let fid = pack.cells.feature_id.get(e).copied().unwrap_or(0);
            let kind = pack
                .features
                .get(fid as usize)
                .map(|f| f.kind)
                .unwrap_or(FeatureType::Ocean);
            let height_cost = get_height_cost(kind, h, type_name);

            let has_river = pack.cells.river.get(e).copied().unwrap_or(0) != 0;
            let flux = pack.cells.flux.get(e).copied().unwrap_or(0);
            let river_cost = get_river_cost(has_river, flux, type_name);

            let t = pack.cells.water_type.get(e).copied().unwrap_or(0);
            let type_cost = get_type_cost(t, type_name);

            let cell_cost = (culture_cost
                + population_cost
                + biome_cost
                + height_cost
                + river_cost
                + type_cost)
                .max(0.0);
            let total_cost = p + 10.0 + cell_cost / state.expansionism;

            if total_cost > growth_limit {
                continue;
            }

            let existing = cost.get(e).copied().unwrap_or(0.0);
            if existing == 0.0 || total_cost < existing {
                if (h as f32) >= 20.0 {
                    pack.cells.state[e] = s; // assign state to cell
                }
                cost[e] = total_cost;
                queue.push((e, total_cost, s, b), total_cost);
            }
        }
    }

    // Assign state to burgs.
    for burg in burgs.iter_mut() {
        if burg.id != 0 && !burg.removed {
            burg.state = pack
                .cells
                .state
                .get(burg.cell as usize)
                .copied()
                .unwrap_or(0);
        }
    }

    states
}

/// FMG `normalize()`: smooths out enclave/fringe cells.
fn normalize_states(
    pack: &mut Pack,
    burgs: &[vor_core::entities::burg::Burg],
    states: Vec<State>,
) -> Vec<State> {
    let n = pack.points_n();
    let new_states = states;

    for i in 0..n {
        let h = pack.cells.height.get(i).copied().unwrap_or(0);
        if (h as f32) < 20.0 {
            continue;
        }
        let has_burg = pack.cells.burg.get(i).copied().unwrap_or(0) != 0;
        if has_burg {
            continue; // do not overwrite burgs
        }
        let sid = pack.cells.state.get(i).copied().unwrap_or(0);
        let state = new_states.get(sid as usize);
        if state.map(|s| s.locked).unwrap_or(false) {
            continue;
        }

        // Skip cells adjacent to a capital burg.
        let neighbors = pack.cells.adjacency.get(i).cloned().unwrap_or_default();
        let near_capital = neighbors.iter().any(|&c| {
            let c = c as usize;
            c < n && {
                let bid = pack.cells.burg.get(c).copied().unwrap_or(0);
                bid != 0
                    && burgs
                        .get(bid as usize)
                        .map(|b| b.is_capital)
                        .unwrap_or(false)
            }
        });
        if near_capital {
            continue;
        }

        let land_neighbors: Vec<usize> = neighbors
            .iter()
            .map(|&c| c as usize)
            .filter(|&c| c < n && (pack.cells.height.get(c).copied().unwrap_or(0) as f32) >= 20.0)
            .collect();

        let adversaries: Vec<usize> = land_neighbors
            .iter()
            .copied()
            .filter(|&c| {
                let s = pack.cells.state.get(c).copied().unwrap_or(0);
                let st = new_states.get(s as usize);
                !st.map(|x| x.locked).unwrap_or(false) && s != sid
            })
            .collect();
        if adversaries.len() < 2 {
            continue;
        }
        let buddies = land_neighbors
            .iter()
            .copied()
            .filter(|&c| {
                let s = pack.cells.state.get(c).copied().unwrap_or(0);
                let st = new_states.get(s as usize);
                !st.map(|x| x.locked).unwrap_or(false) && s == sid
            })
            .count();
        if buddies > 2 {
            continue;
        }
        if adversaries.len() <= buddies {
            continue;
        }
        let adopt = pack.cells.state.get(adversaries[0]).copied().unwrap_or(0);
        pack.cells.state[i] = adopt;
    }

    new_states
}

/// FMG `findNeighbors()`: adjacent states per cell boundary.
fn assign_state_neighbors(pack: &Pack, mut states: Vec<State>) -> Vec<State> {
    let n = pack.points_n();
    for s in states.iter_mut() {
        s.neighbors.clear();
    }
    for i in 0..n {
        let h = pack.cells.height.get(i).copied().unwrap_or(0);
        if (h as f32) < 20.0 {
            continue;
        }
        let s = pack.cells.state.get(i).copied().unwrap_or(0);
        if s == 0 {
            continue;
        }
        let neighbors = pack.cells.adjacency.get(i).cloned().unwrap_or_default();
        for c in neighbors {
            let c = c as usize;
            if c >= n {
                continue;
            }
            let h2 = pack.cells.height.get(c).copied().unwrap_or(0);
            if (h2 as f32) < 20.0 {
                continue;
            }
            let s2 = pack.cells.state.get(c).copied().unwrap_or(0);
            if s2 != s && s2 != 0 {
                if let Some(st) = states.get_mut(s as usize) {
                    if !st.neighbors.contains(&s2) {
                        st.neighbors.push(s2);
                    }
                }
            }
        }
    }
    states
}

/// FMG `assignColors()`: greedy coloring + color mixing per neighbor.
fn assign_state_colors(pack: &Pack, mut states: Vec<State>, rng: &mut Pcg64Mcg) -> Vec<State> {
    let base_colors = [
        "#66c2a5", "#fc8d62", "#8da0cb", "#e78ac3", "#a6d854", "#ffd92f",
    ];
    let _ = pack;

    // Greedy: pick first color not used by a neighbor.
    // Precompute neighbor color usage to avoid borrowing `states` mutably and
    // immutably at the same time.
    let neighbor_colors: Vec<std::collections::HashSet<String>> = states
        .iter()
        .map(|s| {
            s.neighbors
                .iter()
                .filter_map(|&nb| states.get(nb as usize).map(|x| x.color.clone()))
                .collect()
        })
        .collect();

    for (idx, state) in states.iter_mut().enumerate() {
        if state.id == 0 || state.removed || state.locked {
            continue;
        }
        let used = &neighbor_colors[idx];
        let chosen: String = base_colors
            .iter()
            .find(|c| !used.contains(**c))
            .map(|s| s.to_string())
            .unwrap_or_else(|| random_hex(rng));
        state.color = chosen;
    }

    // Mix colors for states sharing a base color.
    for base in base_colors.iter() {
        let same_colored: Vec<u16> = states
            .iter()
            .filter(|s| s.id != 0 && !s.locked && &s.color == base)
            .map(|s| s.id)
            .collect();
        for (idx, &sid) in same_colored.iter().enumerate() {
            if idx == 0 {
                continue;
            }
            if let Some(st) = states.get_mut(sid as usize) {
                st.color = mix_color(&st.color, rng);
            }
        }
    }

    states
}

fn random_hex(rng: &mut Pcg64Mcg) -> String {
    let h = rng.gen_range(0..0xFFFFFFu32);
    format!("#{:06x}", h)
}

/// Approximates FMG `getMixedColor` (blend base with a random color, then
/// brighten). Uses a simple lab-free HSL-space brighten for determinism.
pub(crate) fn mix_color(base: &str, rng: &mut Pcg64Mcg) -> String {
    let base_rgb = hex_to_rgb(base);
    let mix = random_hex(rng);
    let mix_rgb = hex_to_rgb(&mix);
    let blended = [
        base_rgb[0] as f32 * 0.8 + mix_rgb[0] as f32 * 0.2,
        base_rgb[1] as f32 * 0.8 + mix_rgb[1] as f32 * 0.2,
        base_rgb[2] as f32 * 0.8 + mix_rgb[2] as f32 * 0.2,
    ];
    // brighten by 0.3 in linear-ish space
    let bright = blended.map(|v| (v + (255.0 - v) * 0.3) as u8);
    format!("#{:02x}{:02x}{:02x}", bright[0], bright[1], bright[2])
}

fn hex_to_rgb(hex: &str) -> [u8; 3] {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return [0, 0, 0];
    }
    let parse = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    [parse(0), parse(2), parse(4)]
}

/// FMG `collectStatistics()`: area, cells, rural/urban population per state.
fn collect_state_statistics(
    pack: &Pack,
    burgs: &[vor_core::entities::burg::Burg],
    mut states: Vec<State>,
) -> Vec<State> {
    let n = pack.points_n();
    let mut counters: Vec<(u32, u32, f32, f32, u32)> = vec![(0, 0, 0.0, 0.0, 0); states.len()];

    for i in 0..n {
        let h = pack.cells.height.get(i).copied().unwrap_or(0);
        if (h as f32) < 20.0 {
            continue;
        }
        let sid = pack.cells.state.get(i).copied().unwrap_or(0) as usize;
        if sid == 0 || sid >= states.len() {
            continue;
        }
        let cell = &mut counters[sid];
        cell.0 += 1;
        cell.1 = cell
            .1
            .saturating_add(pack.cells.area_px.get(i).copied().unwrap_or(0) as u32);
        cell.2 += pack.cells.population.get(i).copied().unwrap_or(0.0);
        let bid = pack.cells.burg.get(i).copied().unwrap_or(0);
        if bid != 0 {
            let pop = burgs.get(bid as usize).map(|b| b.population).unwrap_or(0.0);
            cell.3 += pop;
            cell.4 += 1;
        }
    }

    for (sid, state) in states.iter_mut().enumerate() {
        if sid == 0 {
            continue;
        }
        let (cells, area, rural, urban, burgs) = counters[sid];
        state.cell_count = cells;
        state.area_px = area;
        state.rural_pop = rural;
        state.urban_pop = urban;
        state.burg_count = burgs;
    }

    states
}
