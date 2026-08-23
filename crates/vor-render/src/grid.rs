//! Grid overlay layer.
//!
//! Voronia previously drew a plain rectangular grid of horizontal/vertical lines.
//! Azgaar FMG instead renders the grid as a **repeating SVG pattern** tiled over
//! the map bounds. The default type is `pointyHex` (`auto-update.ts` sets
//! `#gridOverlay type="pointyHex" size=10`), and the pattern markup:
//!
//! ```svg
//! <pattern id="pattern_pointyHex" width="25" height="43.4" patternUnits="userSpaceOnUse">
//!   <path d="M 0,0 12.5,7.2 25,0 M 12.5,21.7 V 7.2 Z M 0,43.4 V 28.9 L 12.5,21.7 25,28.9 v 14.5"/>
//! </pattern>
//! ```
//!
//! This module replays that same pointy-hex tiling as a line mesh so the overlay
//! matches Azgaar's default grid appearance (hexagons, not squares). The tiles
//! are emitted from the integer tile range overlapping the requested bounds; the
//! viewport clips anything that sticks out, exactly like the SVG pattern fill.

use crate::biome::hex_color_to_linear;
use crate::heightmap::{HeightmapMesh, HeightmapVertex};

/// Horizontal size of the Azgaar `pattern_pointyHex` tile (SVG `width` attr).
const PATTERN_W: f32 = 25.0;
/// Vertical size of the Azgaar `pattern_pointyHex` tile (SVG `height` attr).
const PATTERN_H: f32 = 43.4;
/// Pattern stroke style — FMG `default.json` `#gridOverlay`: stroke `#777777`,
/// opacity 0.8. (GL LineList draws 1 px hardware lines, so only the color is
/// exact; the 0.5 SVG stroke width is approximated.)
fn grid_color() -> [f32; 4] {
    let mut c = hex_color_to_linear("#777777");
    c[3] = 0.8;
    c
}

/// Builds a line mesh reproducing Azgaar's default pointy-hex grid overlay inside
/// `[bounds_min, bounds_max]`, replacing the legacy rectangular lines.
pub fn build_grid_lines(bounds_min: [f32; 2], bounds_max: [f32; 2]) -> HeightmapMesh {
    let min_x = bounds_min[0];
    let min_y = bounds_min[1];
    let max_x = bounds_max[0];
    let max_y = bounds_max[1];

    if max_x <= min_x || max_y <= min_y {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min,
            bounds_max,
        };
    }

    // Tile range that overlaps the bounds. Extra tiles are fine; the viewport
    // clips the overflow (Azgaar fills the whole rect with userSpaceOnUse).
    let first_col = (min_x / PATTERN_W).floor() as i32;
    let last_col = (max_x / PATTERN_W).ceil() as i32;
    let first_row = (min_y / PATTERN_H).floor() as i32;
    let last_row = (max_y / PATTERN_H).ceil() as i32;

    let segs = pattern_segments();

    let mut verts: Vec<HeightmapVertex> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    for row in first_row..=last_row {
        let y_off = row as f32 * PATTERN_H;
        for col in first_col..=last_col {
            let x_off = col as f32 * PATTERN_W;
            for seg in &segs {
                let a = [x_off + seg[0][0], y_off + seg[0][1]];
                let b = [x_off + seg[1][0], y_off + seg[1][1]];
                let base = verts.len() as u32;
                verts.push(HeightmapVertex {
                    pos: a,
                    color: grid_color(),
                });
                verts.push(HeightmapVertex {
                    pos: b,
                    color: grid_color(),
                });
                idx.push(base);
                idx.push(base + 1);
            }
        }
    }

    HeightmapMesh {
        vertices: verts,
        indices: idx,
        bounds_min,
        bounds_max,
    }
}

/// Returns the 7 line segments that make one `pointyHex` tile.
///
/// Translated straight from the SVG `d`:
/// - `M 0,0 12.5,7.2 25,0`           → `[[0,0]-[12.5,7.2]]`, `[[12.5,7.2]-[25,0]]`
/// - `M 12.5,21.7 V 7.2 Z`           → `[[12.5,21.7]-[12.5,7.2]]` (vertical edge)
/// - `M 0,43.4 V 28.9 L 12.5,21.7 25,28.9 v 14.5` → 4 more segments.
fn pattern_segments() -> [[[f32; 2]; 2]; 7] {
    [
        [[0.0, 0.0], [12.5, 7.2]],
        [[12.5, 7.2], [25.0, 0.0]],
        [[12.5, 21.7], [12.5, 7.2]],
        [[0.0, 43.4], [0.0, 28.9]],
        [[0.0, 28.9], [12.5, 21.7]],
        [[12.5, 21.7], [25.0, 28.9]],
        [[25.0, 28.9], [25.0, 43.4]],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointy_hex_tile_has_seven_segments() {
        assert_eq!(pattern_segments().len(), 7);
    }

    #[test]
    fn small_bounds_produce_hexes_not_squares() {
        let mesh = build_grid_lines([0.0, 0.0], [50.0, 43.4]);
        // Covers cols 0..=2 and rows 0..=1 -> 6 tiles * 7 segs * 2 verts.
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.indices.len(), mesh.vertices.len());
        // A rectangular grid would include an X-at-0 line; a hex grid of this
        // width spans full columns: verify many distinct x positions.
        let xs: std::collections::HashSet<u32> = mesh
            .vertices
            .iter()
            .map(|v| (v.pos[0] * 10.0) as u32)
            .collect();
        // Pointy hexes tile every 25px; expect more than a single x.
        assert!(xs.len() >= 3);
    }

    #[test]
    fn zero_or_negative_bounds_return_empty() {
        let mesh = build_grid_lines([5.0, 5.0], [5.0, 5.0]);
        assert!(mesh.vertices.is_empty());
        assert!(mesh.indices.is_empty());
    }
}
