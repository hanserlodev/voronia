//! Coordinate (lat/lon graticule) overlay layer.
//!
//! Replicates Azgaar FMG's `drawCoordinates()` (`layers.js:671`): a geographic
//! **graticule** of latitude/longitude lines projected with an equirectangular
//! mapper, plus one text label per line (`"23°N"`, `"45°W"`, `"0"`). This
//! replaces the previous plain rectangular grid.
//!
//! Projection is affine: `x ↦ lonL + (x/w)·(lonR−lonL)`,
//! `y ↦ latN − (y/h)·latT` (Azgaar `getLongitude`/`getLatitude`).
//!
//! The step between lines is a "round" value picked from Azgaar's `[0.5,1,2,5,10,15,30]`
//! as the closest to `lonT/10`.

use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use vor_core::MapCoordinates;

/// A world-pixel graticule label: anchor position (in canvas px) + text.
#[derive(Debug, Clone)]
pub struct GraticuleLabel {
    /// For a meridian, the world X of the line (canvas px). For a parallel it is
    /// ignored (the label rides the viewport's left edge).
    pub world_x: f32,
    /// For a parallel, the world Y of the line (canvas px). For a meridian it is
    /// ignored (the label rides the viewport's top edge).
    pub world_y: f32,
    /// `true` for a latitude line (label pinned to the viewport's left edge),
    /// `false` for a longitude line (label pinned to the viewport's top edge).
    pub is_latitude: bool,
    /// Text of the label (e.g. `"23°N"`, `"45°W"`, `"0"`).
    pub text: String,
}

/// Possible graticule steps (Azgaar `steps` array in `drawCoordinates`).
const STEPS: [f32; 6] = [0.5, 1.0, 2.0, 5.0, 10.0, 15.0];
/// Optional major step (Azgaar `stepMajor([400, 400])`; irrelevant here).
const _MAJOR: f32 = 400.0;
/// Line color for the graticule (Azgaar `#coordinateGrid` stroke).
const GRID_COLOR: [f32; 4] = [0.5, 0.6, 0.7, 0.3];

/// Picks the "round" step Azgaar uses: closest of `STEPS` to `lonT/10`.
pub fn pick_step(lon_t: f32) -> f32 {
    let goal = (lon_t / 10.0).abs().max(1e-6);
    STEPS
        .iter()
        .map(|&s| (s - goal).abs())
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| STEPS[i])
        .unwrap_or(2.0)
}

/// Converts a world canvas x (`0..width`) to longitude (Azgaar `getLongitude`).
pub fn lon_at_x(x: f32, coords: &MapCoordinates, width: f32) -> f32 {
    coords.lon_l + (x / width.max(1e-6)) * (coords.lon_r - coords.lon_l)
}

/// Converts a world canvas y (`0..height`) to latitude (Azgaar `getLatitude`).
pub fn lat_at_y(y: f32, coords: &MapCoordinates, height: f32) -> f32 {
    coords.lat_n - (y / height.max(1e-6)) * coords.lat_t
}

/// Formats a degree value like Azgaar: `value°N/°S/°E/°W`, or `"0"` at equator.
fn fmt_degree(value: f32, is_latitude: bool) -> String {
    if value.abs() < 1e-6 {
        return "0".to_string();
    }
    let letter = if is_latitude {
        if value > 0.0 {
            "°N"
        } else {
            "°S"
        }
    } else if value < 0.0 {
        "°W"
    } else {
        "°E"
    };
    format!("{}{}", value.abs(), letter)
}

/// Result of building the graticule.
pub struct GraticuleMesh {
    /// Line mesh of the graticule (to draw with a line layer).
    pub lines: HeightmapMesh,
    /// Labels (one per longitude/latitude line of the graticule).
    pub labels: Vec<GraticuleLabel>,
}

/// Builds the geographic graticule over `[0,0]`..`[width,height]` given the map's
/// lat/lon coordinates, mirroring Azgaar FMG.
pub fn build_coordinate_graticule(
    coords: &MapCoordinates,
    width: f32,
    height: f32,
) -> GraticuleMesh {
    let empty = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [0.0, 0.0],
        bounds_max: [width, height],
    };
    let result = GraticuleMesh {
        lines: empty,
        labels: Vec::new(),
    };
    if width <= 0.0 || height <= 0.0 {
        return result;
    }

    let lon_l = coords.lon_l;
    let lon_r = coords.lon_r;
    let lat_n = coords.lat_n;
    let lat_t = coords.lat_t;
    let lat_s = lat_n - lat_t;

    let step = pick_step(lon_r - lon_l);

    let mut verts: Vec<HeightmapVertex> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    let mut labels: Vec<GraticuleLabel> = Vec::new();

    // Meridians (constant longitude). x for a given lon: inverse of lon_at_x.
    // lon = lon_l + (x/width)*(lonR-lonL) -> x = (lon-lonL)/(lonR-lonL)*width.
    let mut lon = (lon_l / step).ceil() * step;
    while lon <= lon_r {
        let x = (lon - lon_l) / (lon_r - lon_l).max(1e-6) * width;
        if x >= -1.0 && x <= width + 1.0 {
            let base = verts.len() as u32;
            verts.push(HeightmapVertex {
                pos: [x, 0.0],
                color: GRID_COLOR,
            });
            verts.push(HeightmapVertex {
                pos: [x, height],
                color: GRID_COLOR,
            });
            idx.push(base);
            idx.push(base + 1);
            labels.push(GraticuleLabel {
                world_x: x,
                world_y: 0.0,
                is_latitude: false,
                text: fmt_degree(lon, false),
            });
        }
        lon += step;
    }

    // Parallels (constant latitude). Same inverse mapping on y.
    let mut lat = (lat_s / step).ceil() * step;
    while lat <= lat_n {
        let y = (lat_n - lat) / lat_t.max(1e-6) * height;
        if y >= -1.0 && y <= height + 1.0 {
            let base = verts.len() as u32;
            verts.push(HeightmapVertex {
                pos: [0.0, y],
                color: GRID_COLOR,
            });
            verts.push(HeightmapVertex {
                pos: [width, y],
                color: GRID_COLOR,
            });
            idx.push(base);
            idx.push(base + 1);
            labels.push(GraticuleLabel {
                world_x: 0.0,
                world_y: y,
                is_latitude: true,
                text: fmt_degree(lat, true),
            });
        }
        lat += step;
    }

    GraticuleMesh {
        lines: HeightmapMesh {
            vertices: verts,
            indices: idx,
            bounds_min: [0.0, 0.0],
            bounds_max: [width, height],
        },
        labels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coords() -> MapCoordinates {
        MapCoordinates {
            lat_t: 54.0,
            lat_n: 44.6,
            lat_s: -9.4,
            lon_l: -9.0,
            lon_r: 26.0,
            ..Default::default()
        }
    }

    #[test]
    fn pick_step_closest_to_goal() {
        // lonT = 20 -> goal 2.0 -> exactly 2.
        assert_eq!(pick_step(20.0), 2.0);
        // lonT = 60 -> goal 6.0 -> closest is 5 (|5-6|=1 < |10-6|=4).
        assert_eq!(pick_step(60.0), 5.0);
        // Ties resolve to the earlier (smaller) step, like JS `reduce` (< strict).
        assert_eq!(pick_step(35.0), 2.0); // goal 3.5: 2 and 5 both at 1.5
        assert_eq!(pick_step(0.0), 0.5); // degenerately small range -> smallest step
    }

    #[test]
    fn graticule_produces_lines_and_labels() {
        let c = coords();
        let g = build_coordinate_graticule(&c, 1000.0, 1000.0);
        assert!(!g.lines.vertices.is_empty());
        assert!(!g.labels.is_empty());
        // Every label has text.
        assert!(g.labels.iter().all(|l| !l.text.is_empty()));
        // Even count of vertices for segments; indices pair up.
        assert_eq!(g.lines.indices.len() % 2, 0);
    }

    #[test]
    fn label_text_has_direction() {
        assert_eq!(fmt_degree(0.0, true), "0");
        assert_eq!(fmt_degree(23.0, true), "23°N");
        assert_eq!(fmt_degree(-45.0, false), "45°W");
    }

    #[test]
    fn zero_bounds_empty() {
        let g = build_coordinate_graticule(&coords(), 0.0, 100.0);
        assert!(g.lines.vertices.is_empty());
    }
}
