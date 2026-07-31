use crate::river::{FLUX_FACTOR, LENGTH_PROGRESSION, LENGTH_STEP_WIDTH, MAX_FLUX_WIDTH};

/// Port of Azgaar's `getOffset()`.
pub fn get_offset(flux: f32, point_index: usize, width_factor: f32, starting_width: f32) -> f32 {
    if point_index == 0 {
        return starting_width;
    }
    let flux_width = (flux / FLUX_FACTOR).powf(0.7).min(MAX_FLUX_WIDTH);
    let prog_idx = point_index.min(LENGTH_PROGRESSION.len() - 1);
    let length_width = point_index as f32 * LENGTH_STEP_WIDTH + LENGTH_PROGRESSION[prog_idx];
    width_factor * (length_width + flux_width) + starting_width
}

/// Port of Azgaar's `getSourceWidth()`.
pub fn get_source_width(flux: f32) -> f32 {
    (flux / FLUX_FACTOR).powf(0.9).min(MAX_FLUX_WIDTH)
}

/// Port of Azgaar's `getWidth()`.
pub fn get_width(offset: f32) -> f32 {
    (offset / 1.5).powf(1.8)
}

/// Azgaar-style rounding (`rn()`).
pub fn rn(value: f32, decimals: i32) -> f32 {
    let factor = 10f32.powi(decimals);
    (value * factor + 0.5).floor() / factor
}
