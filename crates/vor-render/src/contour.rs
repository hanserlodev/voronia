use vor_core::Grid;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};

fn crossing(ha: f32, hb: f32, level: f32) -> bool {
    (ha < level) != (hb < level)
}

fn interp(pa: [f32; 2], pb: [f32; 2], ha: f32, hb: f32, level: f32) -> [f32; 2] {
    let t = if (hb - ha).abs() < 1e-8 {
        0.5
    } else {
        (level - ha) / (hb - ha)
    };
    [pa[0] + t * (pb[0] - pa[0]), pa[1] + t * (pb[1] - pa[1])]
}

pub fn build_contour_lines(grid: &Grid) -> HeightmapMesh {
    let cx = grid.cells_x as usize;
    let cy = grid.cells_y as usize;
    if cx < 2 || cy < 2 {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [0.0, 0.0],
            bounds_max: [0.0, 0.0],
        };
    }

    let levels: [f32; 9] = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0];
    let color = [0.4, 0.28, 0.14, 0.55];

    let mut verts: Vec<HeightmapVertex> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();

    for j in 0..(cy - 1) {
        for i in 0..(cx - 1) {
            let tl = i + j * cx;
            let tr = (i + 1) + j * cx;
            let br = (i + 1) + (j + 1) * cx;
            let bl = i + (j + 1) * cx;

            let h_tl = grid.cells.height.get(tl).copied().unwrap_or(0) as f32;
            let h_tr = grid.cells.height.get(tr).copied().unwrap_or(0) as f32;
            let h_br = grid.cells.height.get(br).copied().unwrap_or(0) as f32;
            let h_bl = grid.cells.height.get(bl).copied().unwrap_or(0) as f32;

            let p_tl = grid.points.get(tl).copied().unwrap_or([0.0; 2]);
            let p_tr = grid.points.get(tr).copied().unwrap_or([0.0; 2]);
            let p_br = grid.points.get(br).copied().unwrap_or([0.0; 2]);
            let p_bl = grid.points.get(bl).copied().unwrap_or([0.0; 2]);

            for &level in &levels {
                let mut pts: [[f32; 2]; 4] = [[0.0; 2]; 4];
                let mut n = 0usize;

                if crossing(h_tl, h_tr, level) && n < 4 {
                    pts[n] = interp(p_tl, p_tr, h_tl, h_tr, level);
                    n += 1;
                }
                if crossing(h_tr, h_br, level) && n < 4 {
                    pts[n] = interp(p_tr, p_br, h_tr, h_br, level);
                    n += 1;
                }
                if crossing(h_br, h_bl, level) && n < 4 {
                    pts[n] = interp(p_br, p_bl, h_br, h_bl, level);
                    n += 1;
                }
                if crossing(h_bl, h_tl, level) && n < 4 {
                    pts[n] = interp(p_bl, p_tl, h_bl, h_tl, level);
                    n += 1;
                }

                if n < 2 || !n.is_multiple_of(2) {
                    continue;
                }

                if n == 2 {
                    let base = verts.len() as u32;
                    verts.push(HeightmapVertex { pos: pts[0], color });
                    verts.push(HeightmapVertex { pos: pts[1], color });
                    idx.push(base);
                    idx.push(base + 1);
                } else {
                    let base = verts.len() as u32;
                    verts.push(HeightmapVertex { pos: pts[0], color });
                    verts.push(HeightmapVertex { pos: pts[1], color });
                    verts.push(HeightmapVertex { pos: pts[2], color });
                    verts.push(HeightmapVertex { pos: pts[3], color });
                    idx.push(base);
                    idx.push(base + 1);
                    idx.push(base + 2);
                    idx.push(base + 3);
                }
            }
        }
    }

    HeightmapMesh {
        vertices: verts,
        indices: idx,
        bounds_min: [0.0, 0.0],
        bounds_max: [0.0, 0.0],
    }
}
