use vor_core::voronoi::VoronoiVertices;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};

pub fn build_cell_wireframe(vertices: &VoronoiVertices, points_n: usize) -> HeightmapMesh {
    let mut verts: Vec<HeightmapVertex> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    let color = [0.7, 0.7, 0.7, 0.25];

    for p in 0..points_n {
        let ring = match vertices.cell_rings.get(p) {
            Some(v) if v.len() >= 3 => v,
            _ => continue,
        };
        let first = ring.len() - 1;
        for w in ring.windows(2) {
            let a = vertices
                .positions
                .get(w[0] as usize)
                .copied()
                .unwrap_or_default();
            let b = vertices
                .positions
                .get(w[1] as usize)
                .copied()
                .unwrap_or_default();
            let base = verts.len() as u32;
            verts.push(HeightmapVertex { pos: a, color });
            verts.push(HeightmapVertex { pos: b, color });
            idx.push(base);
            idx.push(base + 1);
        }
        if ring.len() >= 2 {
            let a = vertices
                .positions
                .get(ring[first] as usize)
                .copied()
                .unwrap_or_default();
            let b = vertices
                .positions
                .get(ring[0] as usize)
                .copied()
                .unwrap_or_default();
            let base = verts.len() as u32;
            verts.push(HeightmapVertex { pos: a, color });
            verts.push(HeightmapVertex { pos: b, color });
            idx.push(base);
            idx.push(base + 1);
        }
    }

    HeightmapMesh {
        vertices: verts,
        indices: idx,
        bounds_min: [0.0, 0.0],
        bounds_max: [0.0, 0.0],
    }
}
