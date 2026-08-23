//! Exports the map to SVG (vector, standalone).
//!
//! Generates an SVG with the map layers without the egui overlay: heightmap
//! (Voronoi polygons), rivers, state borders and burgs with labels.

use std::fmt::Write;
use std::path::Path;

use vor_core::world::World;
use vor_render::heightmap::height_color;

/// Generates an SVG of the full map and writes it to the given path.
///
/// Replicates the render logic of the map layers in SVG:
/// - Dark background (ocean)
/// - Pack cells colored by height (same ramp as `height_color`)
/// - Rivers as blue polylines
/// - State borders as red lines
/// - Burgs as white circles with labels
pub fn export_svg(world: &World, path: &Path) -> anyhow::Result<()> {
    let pack = &world.pack;
    let verts = &pack.vertices;
    let cells = &pack.cells;

    // Bounds
    let (min_x, max_x, min_y, max_y) = {
        let mut mn = [f32::INFINITY; 2];
        let mut mx = [f32::NEG_INFINITY; 2];
        for &p in &pack.points {
            mn[0] = mn[0].min(p[0]);
            mn[1] = mn[1].min(p[1]);
            mx[0] = mx[0].max(p[0]);
            mx[1] = mx[1].max(p[1]);
        }
        (mn[0], mx[0], mn[1], mx[1])
    };
    let w = (max_x - min_x).max(1.0);
    let h = (max_y - min_y).max(1.0);

    let mut svg = String::with_capacity(1024 * 1024); // ~1MB prealloc
    let _ = write!(
        svg,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="{min_x} {min_y} {w} {h}" width="{w}" height="{h}">
"#
    );

    // 1. Ocean background
    let _ = writeln!(
        svg,
        "<rect x=\"{min_x}\" y=\"{min_y}\" width=\"{w}\" height=\"{h}\" fill=\"#05050d\"/>"
    );

    // 2. Pack cells (Voronoi polygons colored by height)
    let n_pack = pack.points_n();
    for p in 0..n_pack {
        let ann = match verts.cell_rings.get(p) {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        let h = cells.height.get(p).copied().unwrap_or(0);
        let color = height_color(h);
        let hex = color_to_hex(color);

        let _ = write!(svg, "<polygon points=\"");
        for (i, &t) in ann.iter().enumerate() {
            let pos = verts.positions.get(t as usize).copied().unwrap_or([0.0; 2]);
            if i > 0 {
                svg.push(' ');
            }
            let _ = write!(svg, "{:.1},{:.1}", pos[0], pos[1]);
        }
        let _ = writeln!(svg, "\" fill=\"{hex}\" stroke=\"none\" opacity=\"0.95\"/>");
    }

    // 3. Rivers as polylines (approximation of the GPU ribbon: centerline
    // stroke using the same width model as river.rs — get_offset/get_width —
    // evaluated at the mouth, where the river is widest).
    for river in &world.rivers {
        if river.cell_path.is_empty() {
            continue;
        }
        let _ = write!(svg, "<polyline points=\"");
        for (i, &cid) in river.cell_path.iter().enumerate() {
            let pt = pack.points.get(cid as usize).copied().unwrap_or([0.0; 2]);
            if i > 0 {
                svg.push(' ');
            }
            let _ = write!(svg, "{:.1},{:.1}", pt[0], pt[1]);
        }
        // Same width model as the GPU mesh: get_offset at the last point
        // (full length progression + full flux), then get_width.
        let flux = river.discharge_m3s.max(1.0);
        let idx = river.cell_path.len().saturating_sub(1);
        let wf = river.width_factor.max(0.1);
        let sw = river.source_width_km.max(0.05);
        let offset = vor_render::river::get_offset(flux, idx, wf, sw);
        let width_px = (vor_render::river::get_width(offset)
            / world.settings.distance_scale.max(0.01))
        .clamp(0.1, 50.0);
        let _ = writeln!(
            svg,
            "\" fill=\"none\" stroke=\"#5d97bb\" stroke-width=\"{width_px:.2}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
        );
    }

    // 4. State borders
    {
        let _ = write!(svg, "<path d=\"");
        for p in 0..n_pack {
            let sid = cells.state.get(p).copied().unwrap_or(0);
            let neighbors = match cells.adjacency.get(p) {
                Some(v) => v,
                None => continue,
            };
            for &nb in neighbors {
                let nid = cells.state.get(nb as usize).copied().unwrap_or(0);
                if nid != sid && nb > p as u32 {
                    // Draw segment only once (p < nb)
                    let a = pack.points.get(p).copied().unwrap_or([0.0; 2]);
                    let b = pack.points.get(nb as usize).copied().unwrap_or([0.0; 2]);
                    let _ = write!(svg, "M {:.1},{:.1} L {:.1},{:.1} ", a[0], a[1], b[0], b[1]);
                }
            }
        }
        let _ = writeln!(
            svg,
            "\" fill=\"none\" stroke=\"#e63333\" stroke-width=\"1.5\" opacity=\"0.8\"/>"
        );
    }

    // 5. Burgs: circle + label
    for burg in &world.burgs {
        if burg.id == 0 || burg.removed {
            continue;
        }
        let pos = burg.position;
        let r = if burg.is_capital { 5 } else { 3 };
        let _ = writeln!(
            svg,
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{r}\" fill=\"white\" stroke=\"black\" stroke-width=\"0.5\"/>",
            pos[0], pos[1]
        );
        let _ = writeln!(
            svg,
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"8\" fill=\"white\" stroke=\"black\" stroke-width=\"0.3\" font-family=\"sans-serif\" text-anchor=\"start\" dy=\".3em\">{}</text>",
            pos[0] + r as f32 + 2.0, pos[1], escape_xml(&burg.name)
        );
    }

    svg.push_str("</svg>\n");
    std::fs::write(path, &svg)?;
    Ok(())
}

fn color_to_hex(c: [f32; 4]) -> String {
    let r = (c[0].clamp(0.0, 1.0) * 255.0) as u8;
    let g = (c[1].clamp(0.0, 1.0) * 255.0) as u8;
    let b = (c[2].clamp(0.0, 1.0) * 255.0) as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
