//! Precipitation layer: colored circles over wet land cells, ported from
//! Azgaar's `drawPrecipitation` (`public/modules/ui/layers.js:333`).
//!
//! Azgaar draws one `<circle>` per grid cell with `height >= 20` (land) and
//! `precipitation > 0`, centered on the cell center `grid.points[d]`, with radius
//! `rn(sqrt(prec/4) / cellsNumberModifier, 2)` where `cellsNumberModifier =
//! (cells/10000)^0.25`. The circle color is the CSS `#003dff` (blue).

use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, VertexBuffers};
use vor_core::Grid;

use crate::biome::hex_color_to_linear;
use crate::heightmap::{ColorCtor, HeightmapMesh};

/// Azgaar `rn(n, 2)` — round to 2 decimals.
fn rn(x: f32) -> f32 {
    (x * 100.0).round() / 100.0
}

/// Tessellated circle outline around a cell center (concentric polygon).
fn push_circle(
    mesh: &mut HeightmapMesh,
    tess: &mut FillTessellator,
    opts: &FillOptions,
    cx: f32,
    cy: f32,
    r: f32,
    color: [f32; 4],
) {
    const SEGMENTS: u32 = 24;
    if r <= 0.0 {
        return;
    }
    let mut builder = Path::builder();
    let step = std::f32::consts::TAU / SEGMENTS as f32;
    let first = point(cx + r, cy);
    builder.begin(first);
    for i in 1..SEGMENTS {
        let (s, c) = (step * i as f32).sin_cos();
        builder.line_to(point(cx + r * c, cy + r * s));
    }
    builder.close();
    let path = builder.build();

    let mut verts: VertexBuffers<crate::heightmap::HeightmapVertex, u32> = VertexBuffers::new();
    let mut buffer_builder = BuffersBuilder::new(&mut verts, ColorCtor(color));
    if tess
        .tessellate_path(&path, opts, &mut buffer_builder)
        .is_err()
    {
        return;
    }
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&verts.vertices);
    mesh.indices.extend(verts.indices.iter().map(|i| i + base));
}

/// Builds the precipitation mesh (Azgaar `drawPrecipitation`).
pub fn build_precipitation_mesh(grid: &Grid) -> HeightmapMesh {
    let mut mesh = HeightmapMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        bounds_min: [0.0, 0.0],
        bounds_max: [grid.width, grid.height],
    };

    let n = grid
        .points_n()
        .min(grid.cells.height.len())
        .min(grid.cells.precipitation.len());
    if n == 0 {
        return mesh;
    }

    let cells_number_modifier = (grid.cells_desired as f32 / 10_000.0).powf(0.25);
    let color = hex_color_to_linear("#003dff");
    let mut tess = FillTessellator::new();
    let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);

    for cell in 0..n {
        let h = grid.cells.height[cell];
        let prec = grid.cells.precipitation[cell];
        if h < 20 || prec == 0 {
            continue;
        }
        let r = rn(f32::sqrt(prec as f32 / 4.0) / cells_number_modifier);
        let p = grid.points[cell];
        push_circle(&mut mesh, &mut tess, &opts, p[0], p[1], r, color);
    }

    mesh
}

/// One wind-direction glyph of FMG's `g#wind` (inside `#prec`), in world px.
#[derive(Debug, Clone, PartialEq)]
pub struct WindGlyph {
    /// `⇉` (U+21C9) westerly, `⇇` (U+21C7) easterly, `⇊` (U+21CA) northerly,
    /// `⇈` (U+21C8) southerly.
    pub ch: char,
    pub x: f32,
    pub y: f32,
}

/// FMG `options.winds` (main.js:159) — wind angle per 30° tier, N to S.
const WINDS: [f32; 6] = [225.0, 45.0, 225.0, 315.0, 135.0, 315.0];

/// Port of `drawWindDirection` (main.js:1108-1151) + the wind-tier loop
/// (main.js:1022-1033): rows are classified by tier (`(|lat − 89| / 30)`,
/// 30° bands N→S) and each tier's angle selects westerly/easterly bands
/// (`isWest: 40..140`, `isEast: 220..320`); a `⇉`/`⇇` glyph is placed at the
/// vertical center of each band with >3 rows (`x=20` / `x=width−52`), plus
/// single `⇊`/`⇈` glyphs at `(width/2, 42)` / `(width/2, height−20)` when any
/// row is northerly/southerly (`isNorth: 100..260`, `isSouth: >280 || <80`).
pub fn wind_glyphs(
    map_coords: &vor_core::MapCoordinates,
    points: &[[f32; 2]],
    cells_x: usize,
    cells_y: usize,
    width: f32,
    height: f32,
) -> Vec<WindGlyph> {
    let tier_of = |row: usize| -> Option<usize> {
        let lat = map_coords.lat_n - (row as f32 / cells_y.max(1) as f32) * map_coords.lat_t;
        let tier = ((lat - 89.0).abs() / 30.0) as usize;
        if tier < 6 {
            Some(tier)
        } else {
            None
        }
    };
    // FMG averages the first/last cell points of the band; rows are uniform
    // so the row-start cell's y is the row's y.
    let row_y = |row: usize| -> f32 {
        points
            .get(row * cells_x.max(1))
            .map(|p| p[1])
            .unwrap_or(0.0)
    };

    let mut glyphs = Vec::new();
    for tier in 0..6usize {
        let west_rows: Vec<usize> = (0..cells_y).filter(|&r| tier_of(r) == Some(tier)).collect();
        let mut west = false;
        let mut east = false;
        if let Some(&angle) = WINDS.get(tier) {
            west = angle > 40.0 && angle < 140.0;
            east = angle > 220.0 && angle < 320.0;
        }
        if west && west_rows.len() > 3 {
            let y = (row_y(west_rows[0]) + row_y(west_rows[west_rows.len() - 1])) / 2.0;
            glyphs.push(WindGlyph {
                ch: '\u{21C9}',
                x: 20.0,
                y,
            });
        }
        if east && west_rows.len() > 3
            || east && (0..cells_y).filter(|&r| tier_of(r) == Some(tier)).count() > 3
        {
            let rows: Vec<usize> = (0..cells_y).filter(|&r| tier_of(r) == Some(tier)).collect();
            if rows.len() > 3 {
                let y = (row_y(rows[0]) + row_y(rows[rows.len() - 1])) / 2.0;
                glyphs.push(WindGlyph {
                    ch: '\u{21C7}',
                    x: width - 52.0,
                    y,
                });
            }
        }
    }
    // northerly/southerly: any row in a north/south tier (single center glyph).
    let any_north = (0..cells_y).any(|r| {
        tier_of(r)
            .map(|t| WINDS[t] > 100.0 && WINDS[t] < 260.0)
            .unwrap_or(false)
    });
    let any_south = (0..cells_y).any(|r| {
        tier_of(r)
            .map(|t| WINDS[t] > 280.0 || WINDS[t] < 80.0)
            .unwrap_or(false)
    });
    if any_north {
        glyphs.push(WindGlyph {
            ch: '\u{21CA}',
            x: width / 2.0,
            y: 42.0,
        });
    }
    if any_south {
        glyphs.push(WindGlyph {
            ch: '\u{21C8}',
            x: width / 2.0,
            y: height - 20.0,
        });
    }
    glyphs
}

#[cfg(test)]
mod wind_tests {
    use super::*;
    use vor_core::MapCoordinates;

    #[test]
    fn wind_glyphs_follow_fmg_tiers() {
        // Sorvik-like coordinates (lat 44.6N..-9.4S): tiers 0-2 only (N side).
        let coords = MapCoordinates {
            lat_t: 54.0,
            lat_n: 44.6,
            lat_s: -9.4,
            lon_l: -9.0,
            lon_r: 26.0,
            ..Default::default()
        };
        // 20 rows × 10 cols, evenly spread over 1000×1000.
        let points: Vec<[f32; 2]> = (0..200)
            .map(|i| {
                let row = i / 10;
                let col = i % 10;
                [50.0 + col as f32 * 100.0, 25.0 + row as f32 * 50.0]
            })
            .collect();
        let glyphs = wind_glyphs(&coords, &points, 10, 20, 1000.0, 1000.0);
        // Tier 1 (lat 59..29N) covers rows near the top → westerly ⇉ at x=20.
        assert!(
            glyphs.iter().any(|g| g.ch == '\u{21C9}' && g.x == 20.0),
            "{glyphs:?}"
        );
        // The map spans into tier 3 (lat −1..−31, angle 315 → isSouth), so
        // the ⇈ glyph must appear at bottom center.
        assert!(glyphs
            .iter()
            .any(|g| g.ch == '\u{21C8}' && g.x == 500.0 && g.y == 980.0));
        // Northerly tiers (225 in 100..260) exist → ⇊ at center top.
        assert!(glyphs
            .iter()
            .any(|g| g.ch == '\u{21CA}' && g.x == 500.0 && g.y == 42.0));
    }
}
