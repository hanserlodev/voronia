//! Exporta el mapa a SVG (vectorial, autónomo).
//!
//! Genera un SVG con las capas de mapa sin overlay egui: heightmap (polígonos de
//! Voronoi), ríos, fronteras de estados y burgos con labels.

use std::fmt::Write;
use std::path::Path;

use vor_core::world::World;
use vor_render::heightmap::height_color;

/// Genera un SVG del mapa completo y lo escribe al path dado.
///
/// Replica la lógica de render de las capas de mapa en SVG:
/// - Fondo oscuro (océano)
/// - Celdas del pack coloreadas por altura (misma rampa que `height_color`)
/// - Ríos como polilíneas azules
/// - Fronteras de estados como líneas rojas
/// - Burgos como círculos blancos con label
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

    // 1. Fondo océano
    let _ = writeln!(
        svg,
        "<rect x=\"{min_x}\" y=\"{min_y}\" width=\"{w}\" height=\"{h}\" fill=\"#05050d\"/>"
    );

    // 2. Celdas del pack (polígonos Voronoi con color por altura)
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

    // 3. Ríos como polilíneas
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
        // Ancho según caudal (mapeado a 0.5~4px)
        let width_px = (river.discharge_m3s / 5000.0).clamp(0.5, 4.0);
        let _ = writeln!(
            svg,
            "\" fill=\"none\" stroke=\"#4488cc\" stroke-width=\"{width_px:.1}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
        );
    }

    // 4. Fronteras de estados
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
                    // Dibujar segmento solo una vez (p < nb)
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

    // 5. Burgos: círculo + label
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
