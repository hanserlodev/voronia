//! Capa de heightmap: triangula las celdas del `Grid` con `lyon` y arma
//! vertex/index buffers para wgpu.
//!
//! Cada celda `p` del grid es el poligono cerrado formado por los circumcentros
//! de los triangulos de Delaunay adyacentes (los IDs en `grid.vertices.cell_rings[p]`
//! -> la posicion esta en `grid.vertices.positions[t]`). En Azgaar este orden viene
//! dado por `edgesAroundPoint` (CCW). Lyon recorre los puntos en el orden dado.
//!
//! Color por altura: usa una rampa estilo Azgaar ("Heightmap show"):
//! height < 20 -> azul (mar, mas oscuro cuanto menor); 20 = costa; > 20 -> verde
//! a marron/blanco segun altura. Es solo una rampa visual para el visor de Fase 2,
//! no sustituye el sistema de biomas (Fase 3).

use bytemuck::{Pod, Zeroable};
use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertexConstructor, VertexBuffers,
};
use vor_core::Grid;

/// Un vertice del buffer de terreno: posicion (pixeles de mundo) + color (linear RGBA).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct HeightmapVertex {
    pub pos: [f32; 2],
    /// Color RGBA lineal (el shader no aplica gamma).
    pub color: [f32; 4],
}

/// Malla triangulada lista para subir a GPU.
#[derive(Clone, Debug)]
pub struct HeightmapMesh {
    pub vertices: Vec<HeightmapVertex>,
    pub indices: Vec<u32>,
    /// Bounding box de mundo (min/max en pixels) para encuadre inicial.
    pub bounds_min: [f32; 2],
    pub bounds_max: [f32; 2],
}

/// Constructor de vertices de lyon: devuelve un `HeightmapVertex` por cada
/// vertice generado por el tessellator (con el color fijo de la celda).
pub(crate) struct ColorCtor(pub(crate) [f32; 4]);

impl FillVertexConstructor<HeightmapVertex> for ColorCtor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::FillVertex<'_>) -> HeightmapVertex {
        let p = vertex.position();
        HeightmapVertex {
            pos: [p.x, p.y],
            color: self.0,
        }
    }
}

/// Construye la `HeightmapMesh` a partir del `Grid`.
///
/// - Recorre las `grid.points.len()` celdas reales (sin boundary).
/// - Para cada una, construye el path del poligono cerrado con los vertices de
///   Voronoi listados en `grid.vertices.cell_rings[p]`, lo tessellate con `lyon`,
///   vuelca al mesh global offseteando indices.
/// - Calcula el bounding box final.
///
/// Celdas con `cell_rings[p]` vacio (boundary mal formado) se saltan.
pub fn build_mesh(grid: &Grid) -> HeightmapMesh {
    let n_cells = grid.points_n();
    let mut vertices: Vec<HeightmapVertex> = Vec::with_capacity(n_cells * 6);
    let mut indices: Vec<u32> = Vec::with_capacity(n_cells * 9);
    let mut bounds_min = [f32::INFINITY, f32::INFINITY];
    let mut bounds_max = [f32::NEG_INFINITY, f32::NEG_INFINITY];

    let mut tess = FillTessellator::new();

    for p in 0..n_cells {
        let ann = match grid.vertices.cell_rings.get(p) {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        let h = grid.cells.height.get(p).copied().unwrap_or(0);
        let color = height_color(h);

        // Path del poligono cerrado en orden CCW (el orden dado por `edgesAroundPoint`).
        let first_t = ann[0] as usize;
        let first_pos = grid
            .vertices
            .positions
            .get(first_t)
            .copied()
            .unwrap_or([0.0, 0.0]);
        let mut builder = Path::builder();
        builder.begin(point(first_pos[0], first_pos[1]));
        for &t in ann.iter().skip(1) {
            let ti = t as usize;
            let pos = grid
                .vertices
                .positions
                .get(ti)
                .copied()
                .unwrap_or([0.0, 0.0]);
            builder.line_to(point(pos[0], pos[1]));
        }
        builder.end(true);
        let path = builder.build();

        let mut mesh: VertexBuffers<HeightmapVertex, u32> = VertexBuffers::new();
        let mut buffer_builder = BuffersBuilder::new(&mut mesh, ColorCtor(color));
        let opts = FillOptions::default().with_fill_rule(lyon::tessellation::FillRule::EvenOdd);

        if tess
            .tessellate_path(&path, &opts, &mut buffer_builder)
            .is_err()
        {
            continue; // Skip degenerate cells.
        }

        let base = vertices.len() as u32;
        vertices.extend_from_slice(&mesh.vertices);
        indices.extend(mesh.indices.iter().map(|i| i + base));

        for v in &mesh.vertices {
            bounds_min[0] = bounds_min[0].min(v.pos[0]);
            bounds_min[1] = bounds_min[1].min(v.pos[1]);
            bounds_max[0] = bounds_max[0].max(v.pos[0]);
            bounds_max[1] = bounds_max[1].max(v.pos[1]);
        }
    }

    if !bounds_min[0].is_finite() {
        bounds_min = [0.0, 0.0];
        bounds_max = [grid.width, grid.height];
    }

    HeightmapMesh {
        vertices,
        indices,
        bounds_min,
        bounds_max,
    }
}

/// Rampa de color por altura (estilo Heightmap show de Azgaar).
pub fn height_color(h: u8) -> [f32; 4] {
    let h = h.min(100) as f32;
    let rgb = if h < 20.0 {
        let t = h / 20.0;
        [
            lerp(0.04, 0.39, t),
            lerp(0.13, 0.55, t),
            lerp(0.27, 0.73, t),
        ]
    } else {
        // Tierra: stops en h -> RGB.
        let stops: [(f32, [f32; 3]); 5] = [
            (20.0, [0.42, 0.61, 0.34]),
            (40.0, [0.29, 0.45, 0.21]),
            (60.0, [0.46, 0.39, 0.18]),
            (80.0, [0.51, 0.43, 0.31]),
            (100.0, [0.91, 0.91, 0.91]),
        ];
        let mut prev = stops[0];
        for s in &stops[1..] {
            if h <= s.0 {
                let span = (s.0 - prev.0).max(1e-6);
                let t = (h - prev.0) / span;
                return [
                    lerp(prev.1[0], s.1[0], t),
                    lerp(prev.1[1], s.1[1], t),
                    lerp(prev.1[2], s.1[2], t),
                    1.0,
                ];
            }
            prev = *s;
        }
        let last = stops[stops.len() - 1].1;
        [last[0], last[1], last[2]]
    };
    [rgb[0], rgb[1], rgb[2], 1.0]
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_color_sea_is_blue() {
        let c = height_color(0);
        assert!(
            c[2] > c[0] && c[2] > c[1],
            "deep sea should be bluish, got {c:?}"
        );
        assert_eq!(c[3], 1.0);
    }

    #[test]
    fn height_color_summit_is_bright() {
        let c = height_color(100);
        assert!(
            c[0] > 0.85 && c[1] > 0.85 && c[2] > 0.85,
            "summit got {c:?}"
        );
    }

    #[test]
    fn height_color_clamp_above_100() {
        let _ = height_color(255);
    }
}
