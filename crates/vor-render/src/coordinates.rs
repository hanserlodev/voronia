use crate::heightmap::{HeightmapMesh, HeightmapVertex};

pub fn build_coordinate_lines(bounds_min: [f32; 2], bounds_max: [f32; 2]) -> HeightmapMesh {
    let dx = bounds_max[0] - bounds_min[0];
    let dy = bounds_max[1] - bounds_min[1];
    if dx <= 0.0 || dy <= 0.0 {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min,
            bounds_max,
        };
    }

    let spacing = {
        let base = dx.min(dy) / 8.0;
        let mag = 10.0_f32.powf(base.log10().floor());
        let norm = base / mag;
        let nice = if norm < 1.5 {
            1.0
        } else if norm < 3.5 {
            2.0
        } else if norm < 7.5 {
            5.0
        } else {
            10.0
        };
        nice * mag
    }
    .max(1.0);

    let color = [0.5, 0.6, 0.7, 0.12];
    let mut verts: Vec<HeightmapVertex> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();

    let x_start = (bounds_min[0] / spacing).ceil() * spacing;
    let y_start = (bounds_min[1] / spacing).ceil() * spacing;

    let mut x = x_start;
    while x <= bounds_max[0] {
        let base = verts.len() as u32;
        verts.push(HeightmapVertex {
            pos: [x, bounds_min[1]],
            color,
        });
        verts.push(HeightmapVertex {
            pos: [x, bounds_max[1]],
            color,
        });
        idx.push(base);
        idx.push(base + 1);
        x += spacing;
    }

    let mut y = y_start;
    while y <= bounds_max[1] {
        let base = verts.len() as u32;
        verts.push(HeightmapVertex {
            pos: [bounds_min[0], y],
            color,
        });
        verts.push(HeightmapVertex {
            pos: [bounds_max[0], y],
            color,
        });
        idx.push(base);
        idx.push(base + 1);
        y += spacing;
    }

    HeightmapMesh {
        vertices: verts,
        indices: idx,
        bounds_min,
        bounds_max,
    }
}
