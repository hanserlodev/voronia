pub mod hydrology;
pub mod meander;
pub mod resolve;
pub mod specify;
pub mod width;

pub use width::*;

pub const MIN_FLUX_TO_FORM_RIVER: u16 = 30;
pub const MIN_NAVIGABLE_FLUX: u16 = 100;
pub const FLUX_FACTOR: f32 = 500.0;
pub const MAX_FLUX_WIDTH: f32 = 1.0;
pub const LENGTH_FACTOR: f32 = 200.0;
pub const MAX_DOWNCUT: u16 = 5;
pub const LENGTH_STEP_WIDTH: f32 = 1.0 / 200.0;
pub const LENGTH_PROGRESSION: [f32; 9] = [
    1.0 / 200.0,
    1.0 / 200.0,
    2.0 / 200.0,
    3.0 / 200.0,
    5.0 / 200.0,
    8.0 / 200.0,
    13.0 / 200.0,
    21.0 / 200.0,
    34.0 / 200.0,
];

use vor_core::entities::river::River;
use vor_core::grid::Grid;
use vor_core::pack::Pack;

pub struct RiverGenerator;

impl RiverGenerator {
    /// Full pipeline: modifies PackCells, returns Vec<River>.
    /// Accepts an `rivers` slice to merge with existing (imported) rivers.
    pub fn generate(
        pack: &mut Pack,
        grid: &Grid,
        allow_erosion: bool,
        existing_rivers: &[River],
    ) -> Vec<River> {
        let mut h = hydrology::alter_heights(pack);
        hydrology::define_lake_climate(pack, &h, grid);
        hydrology::resolve_depressions(pack, &mut h);
        hydrology::drain_water(pack, &mut h, grid);

        let mut new_rivers = Self::define_rivers(pack, &h);
        Self::calculate_confluence_flux(pack, &h);

        if allow_erosion {
            pack.cells.height = h.iter().map(|&v| v as u8).collect();
            let mut h2: Vec<f32> = h.to_vec();
            Self::downcut_rivers(pack, &mut h2);
            pack.cells.height = h2.iter().map(|&v| v as u8).collect();
        }

        let mut all = existing_rivers.to_vec();
        all.append(&mut new_rivers);
        specify::specify_common(&mut all);
        all
    }

    fn define_rivers(pack: &Pack, _h: &[f32]) -> Vec<River> {
        let cells = &pack.cells;
        let n = cells.len();
        let mut river_cells: Vec<Vec<u32>> = Vec::new();
        for i in 0..n {
            let rid = cells.river[i];
            if rid == 0 {
                continue;
            }
            let idx = rid as usize;
            if idx >= river_cells.len() {
                river_cells.resize(idx + 1, Vec::new());
            }
            river_cells[idx].push(i as u32);
        }

        let _modifier = ((n as f32) / 10000.0).powf(0.25);
        let default_wf = width::rn(1.0 / ((n as f32) / 10000.0).powf(0.25), 2);
        let main_wf = default_wf * 1.2;

        let mut rivers: Vec<River> = Vec::new();
        for (rid, cells_list) in river_cells.iter().enumerate().skip(1) {
            if cells_list.len() < 3 {
                continue;
            }
            let source = cells_list[0];
            let mouth = cells_list[cells_list.len() - 2];
            let discharge = cells.flux[mouth as usize] as f32;
            let width_factor = main_wf;
            let source_width = width::get_source_width(cells.flux[source as usize] as f32);
            let point_count = cells_list.len();
            let offset = width::get_offset(discharge, point_count, width_factor, source_width);
            let width_km = width::get_width(offset);
            let length_km = 0.0;

            rivers.push(River {
                id: rid as u16,
                name: String::new(),
                source_cell: source,
                mouth_cell: mouth,
                parent_river: None,
                basin_id: rid as u16,
                discharge_m3s: discharge,
                length_km,
                width_km,
                width_factor,
                source_width_km: source_width,
                type_name: "River".into(),
                cell_path: cells_list.clone(),
                meandered_points: Vec::new(),
            });
        }
        rivers
    }

    fn calculate_confluence_flux(pack: &mut Pack, h: &[f32]) {
        let cell_count = pack.cells.len();
        for i in 0..cell_count {
            if pack.cells.confluence[i] == 0 {
                continue;
            }
            let mut influx: Vec<u16> = Vec::new();
            if let Some(neighbors) = pack.cells.adjacency.get(i) {
                for &n in neighbors {
                    let n = n as usize;
                    if n < cell_count && pack.cells.river[n] != 0 && h[n] > h[i] {
                        influx.push(pack.cells.flux[n]);
                    }
                }
            }
            influx.sort_by(|a, b| b.cmp(a));
            let sum: u16 = influx
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx > 0)
                .map(|(_, &v)| v)
                .sum();
            pack.cells.confluence[i] = sum;
        }
    }

    fn downcut_rivers(pack: &Pack, h: &mut [f32]) {
        let cell_count = pack.cells.len();
        for i in 0..cell_count {
            if pack.cells.height[i] < 35 || pack.cells.flux[i] == 0 {
                continue;
            }
            let mut higher: Vec<usize> = Vec::new();
            if let Some(neighbors) = pack.cells.adjacency.get(i) {
                for &n in neighbors {
                    let n = n as usize;
                    if n < h.len() && h[n] > h[i] {
                        higher.push(n);
                    }
                }
            }
            if higher.is_empty() {
                continue;
            }
            let higher_flux_sum: u16 = higher.iter().map(|&n| pack.cells.flux[n]).sum();
            let higher_flux = higher_flux_sum as f32 / higher.len() as f32;
            if higher_flux <= 0.0 {
                continue;
            }
            let downcut = (pack.cells.flux[i] as f32 / higher_flux).floor() as u16;
            if downcut > 0 {
                h[i] = (h[i] - (downcut.min(MAX_DOWNCUT) as f32)).max(0.0);
            }
        }
    }
}
