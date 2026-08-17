use vor_core::pack::Pack;

use crate::heightmap::{HeightmapMesh, HeightmapVertex};
use crate::mesh::build_pack_mesh;

/// Builds the biome mesh: each pack cell is colored according to its biome.
pub fn build_biome_mesh(pack: &Pack, biome_colors: &[[f32; 4]]) -> HeightmapMesh {
    let n_pack = pack.points_n();
    build_pack_mesh(&pack.vertices, n_pack, |p| {
        let bi = pack.cells.biome.get(p).copied().unwrap_or(0) as usize;
        biome_colors
            .get(bi)
            .copied()
            .unwrap_or([0.0, 0.0, 0.0, 1.0])
    })
}

/// Builds the biome "coast fill": recolors the fractal landmass mesh (the stencil
/// mask source) so that every triangle carries the biome color of the nearest
/// land cell.
///
/// The fractal coastline can protrude several cells beyond the outermost land
/// cells (fractal displacement), so the cell-based fill (`build_biome_mesh`)
/// leaves a white halo between the last cell and the fractal coast. Merging this
/// mesh BEFORE the cell fill into the same layer fills that halo with the
/// coastal cell's biome, extending the biome color exactly up to the fractal
/// coastline (Azgaar's isolines are clipped to `#land`, reaching the coast).
///
/// `is_water[i]` must be `true` for ocean and lake cells (they are excluded from
/// the nearest-cell search).
pub fn build_biome_coast_fill(
    landmass: &HeightmapMesh,
    pack: &Pack,
    is_water: &[bool],
    biome_colors: &[[f32; 4]],
) -> HeightmapMesh {
    let verts = &pack.vertices;
    let n = pack.points_n();

    // Land cell centroids (shoelace) + biome color index.
    let mut cells: Vec<(usize, [f32; 2], usize)> = Vec::new();
    for p in 0..n {
        if is_water.get(p).copied().unwrap_or(true) {
            continue;
        }
        let ring = match verts.cell_rings.get(p) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        let Some(c) = ring_centroid(verts, ring) else {
            continue;
        };
        let bi = pack.cells.biome.get(p).copied().unwrap_or(0) as usize;
        cells.push((p, c, bi));
    }
    if cells.is_empty() {
        return HeightmapMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds_min: [0.0, 0.0],
            bounds_max: [0.0, 0.0],
        };
    }

    let index = CentroidGrid::build(&cells, landmass.bounds_min, landmass.bounds_max);

    let mut result = HeightmapMesh {
        vertices: Vec::with_capacity(landmass.indices.len()),
        indices: Vec::with_capacity(landmass.indices.len()),
        bounds_min: landmass.bounds_min,
        bounds_max: landmass.bounds_max,
    };

    let tri_center = |a: [f32; 2], b: [f32; 2], c: [f32; 2]| {
        [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0]
    };

    for chunk in landmass.indices.chunks_exact(3) {
        let a = landmass.vertices[chunk[0] as usize].pos;
        let b = landmass.vertices[chunk[1] as usize].pos;
        let c = landmass.vertices[chunk[2] as usize].pos;
        let center = tri_center(a, b, c);
        let (_, _, bi) = index.nearest(center, &cells);
        let color = biome_colors
            .get(bi)
            .copied()
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let base = result.vertices.len() as u32;
        result.vertices.extend([
            HeightmapVertex { pos: a, color },
            HeightmapVertex { pos: b, color },
            HeightmapVertex { pos: c, color },
        ]);
        result.indices.extend([base, base + 1, base + 2]);
    }

    result
}

/// Small uniform-grid index over cell centroids for nearest-neighbor lookups.
struct CentroidGrid {
    cell_size: f32,
    nx: usize,
    ny: usize,
    min: [f32; 2],
    bins: Vec<Vec<usize>>,
}

impl CentroidGrid {
    fn build(cells: &[(usize, [f32; 2], usize)], min: [f32; 2], max: [f32; 2]) -> Self {
        let (w, h) = ((max[0] - min[0]).max(1.0), (max[1] - min[1]).max(1.0));
        let n = cells.len().max(1) as f32;
        // Target ~n/8 buckets so each bin holds ~8 centroids on average.
        let cell_size = (w * h / (n / 8.0)).sqrt().max(1.0);
        let nx = ((w / cell_size).ceil() as usize).max(1);
        let ny = ((h / cell_size).ceil() as usize).max(1);
        let mut bins = vec![Vec::new(); nx * ny];
        for (idx, &(_, c, _)) in cells.iter().enumerate() {
            let bx = (((c[0] - min[0]) / cell_size) as isize).clamp(0, nx as isize - 1) as usize;
            let by = (((c[1] - min[1]) / cell_size) as isize).clamp(0, ny as isize - 1) as usize;
            bins[by * nx + bx].push(idx);
        }
        CentroidGrid {
            cell_size,
            nx,
            ny,
            min,
            bins,
        }
    }

    /// Returns the index (into `cells`) of the nearest centroid to `p`.
    fn nearest(&self, p: [f32; 2], cells: &[(usize, [f32; 2], usize)]) -> (usize, [f32; 2], usize) {
        let cx = (((p[0] - self.min[0]) / self.cell_size) as isize).clamp(0, self.nx as isize - 1)
            as usize;
        let cy = (((p[1] - self.min[1]) / self.cell_size) as isize).clamp(0, self.ny as isize - 1)
            as usize;

        // Expanding ring search: grows the window until the current best distance
        // is smaller than the next ring's farthest reach.
        let mut best_d = f32::INFINITY;
        let mut best = (0usize, [0.0; 2], 0usize);
        let mut radius = 0isize;
        loop {
            let reach = (radius as f32 + 0.5) * self.cell_size * std::f32::consts::SQRT_2;
            if reach > best_d && best_d.is_finite() {
                break;
            }
            let mut found = false;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if radius > 0 && dx.abs() != radius && dy.abs() != radius {
                        continue; // only the ring, not the filled square
                    }
                    let bx = cx as isize + dx;
                    let by = cy as isize + dy;
                    if bx < 0 || by < 0 || bx >= self.nx as isize || by >= self.ny as isize {
                        continue;
                    }
                    let bin = &self.bins[by as usize * self.nx + bx as usize];
                    if bin.is_empty() {
                        continue;
                    }
                    found = true;
                    for &idx in bin {
                        let c = cells[idx].1;
                        let d = (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2);
                        if d < best_d {
                            best_d = d;
                            best = cells[idx];
                        }
                    }
                }
            }
            if !found && radius > (self.nx.max(self.ny) as isize) + 1 {
                break; // safety: should never happen with a non-empty grid
            }
            radius += 1;
        }
        best
    }
}

fn ring_centroid(vertices: &vor_core::voronoi::VoronoiVertices, ring: &[u32]) -> Option<[f32; 2]> {
    let pts: Vec<[f32; 2]> = ring
        .iter()
        .filter_map(|&t| vertices.positions.get(t as usize).copied())
        .collect();
    if pts.len() < 3 {
        return None;
    }
    let mut area = 0.0f32;
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    for i in 0..pts.len() {
        let j = (i + 1) % pts.len();
        let cross = pts[i][0] * pts[j][1] - pts[j][0] * pts[i][1];
        area += cross;
        cx += (pts[i][0] + pts[j][0]) * cross;
        cy += (pts[i][1] + pts[j][1]) * cross;
    }
    if area.abs() < 1e-9 {
        return None;
    }
    Some([cx / (3.0 * area), cy / (3.0 * area)])
}

/// Extracts colors from the biome list (hex string `#rrggbb` -> linear `[f32;4]`).
pub fn biome_colors_from_catalog(biomes: &[vor_core::entities::biome::Biome]) -> Vec<[f32; 4]> {
    biomes
        .iter()
        .map(|b| hex_color_to_linear(&b.color))
        .collect()
}

pub fn hex_color_to_linear(hex: &str) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(hex.get(0..2).unwrap_or("00"), 16).unwrap_or(0);
    let g = u8::from_str_radix(hex.get(2..4).unwrap_or("00"), 16).unwrap_or(0);
    let b = u8::from_str_radix(hex.get(4..6).unwrap_or("00"), 16).unwrap_or(0);
    // sRGB -> approximate linear (2.2 gamma)
    fn srgb_to_linear(c: u8) -> f32 {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0]
}
