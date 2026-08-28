//! Scale bar (FMG `#scaleBar`, outside `#viewbox` — screen-space).
//!
//! Recalculated on every zoom change (`draw-scalebar.ts`):
//! `val = 100 * data-bar-size * distanceScale / scaleLevel` snapped to clean
//! orders (>900 -> thousands, >90 -> hundreds, >9 -> tens); the bar spans
//! that value, is divided into 5 labeled segments and drawn as a double
//! white/gray line over a translucent backing rect at x99%/y99%.

/// FMG `#scaleBar data-bar-size` default.
pub const BAR_SIZE: f32 = 2.0;

/// FMG `getLength` rounding: snap `val` to clean 1/10/100/1000 multiples so
/// subdivisions read as round numbers.
pub fn clean_value(val: f32) -> f32 {
    let v = val.max(0.0);
    if v > 900.0 {
        (v / 1000.0).floor() * 1000.0
    } else if v > 90.0 {
        (v / 100.0).floor() * 100.0
    } else if v > 9.0 {
        (v / 10.0).floor() * 10.0
    } else {
        v.floor().max(1.0)
    }
}

/// Computes scale-bar geometry for the current view.
///
/// * `surface_h_px`: viewport height in px.
/// * `extent_y`: visible world height (camera extent).
/// * `distance_scale`: km per map unit (`settings.distanceScale`).
///
/// Returns `(bar_width_px, subdivision_km)`: the bar covers
/// `5 * subdivision_km`, drawn at `bar_width_px` wide.
pub fn scale_bar_geometry(surface_h_px: f32, extent_y: f32, distance_scale: f32) -> (f32, f32) {
    let zoom_scale = surface_h_px / extent_y.max(1e-6);
    let raw = 100.0 * BAR_SIZE * distance_scale.max(1e-6) / zoom_scale.max(1e-6);
    let clean = clean_value(raw);
    let bar_width_px = clean * zoom_scale / distance_scale.max(1e-6);
    (bar_width_px, clean / 5.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_value_snaps_to_orders() {
        assert_eq!(clean_value(1234.0), 1000.0);
        assert_eq!(clean_value(456.0), 400.0);
        assert_eq!(clean_value(78.0), 70.0);
        assert_eq!(clean_value(8.4), 8.0);
        assert_eq!(clean_value(0.2), 1.0);
    }

    #[test]
    fn geometry_at_fit_is_reasonable() {
        // height 900px, extent_y 900 world units → zoom_scale 1; dS = 1 km/unit
        // raw = 200 → clean 200 → bar 200px wide, subdivisions of 40km.
        let (w, sub) = scale_bar_geometry(900.0, 900.0, 1.0);
        assert!((w - 200.0).abs() < 1e-4);
        assert!((sub - 40.0).abs() < 1e-4);
    }
}
