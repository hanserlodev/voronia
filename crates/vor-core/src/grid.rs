//! Malla del grid (grilla jitterizada). Únicamente datos; la generación (puntos
//! con jitter, Delaunay, Voronoi, repacking) vive en `vor-import`.

use crate::cells::GridCells;
use crate::feature::Feature;
use crate::voronoi::VoronoiVertices;

/// Malla del grid — etapa previa al repacking.
///
/// Los `points` son los puntos jitterizados en fila-mayor (`y` externo, `x` interno),
/// en el orden exacto que `getJitteredGrid` produce en Azgaar (`graphUtils.ts:46-98`).
/// El `id` de celda `k` corresponde al `points[k]` → es el detalle que vuelve
/// determinista el correspondence entre RNG (orden de consumo de jitter) y
/// atributos por celda (ver `docs/fase-0-investigacion.md` §6.5, §13.4).
/// Los `boundary` son puntos virtuales fuera del canvas para evitar celdas de
/// Voronoi infinitas en el borde — no consumen RNG (§6.6).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Grid {
    /// Cantidad de celdas deseada (input del usuario; default 10000 en Azgaar).
    pub cells_desired: u32,
    /// Espaciamiento entre puntos = `min(width, height) / sqrt(cellsDesired)`, redondeado abajo a int.
    pub spacing: f32,
    /// Cantidad de columnas = `floor(width / spacing)`.
    pub cells_x: u32,
    /// Cantidad de filas = `floor(height / spacing)`.
    pub cells_y: u32,
    /// Ancho del canvas (units de Azgaar — el `graphWidth` del header slot `[0]`).
    pub width: f32,
    /// Alto del canvas (units de Azgaar — el `graphHeight` del header slot `[0]`).
    pub height: f32,
    /// Puntos jitterizados (`points.length = cells_x * cells_y`), fila-mayor.
    pub points: Vec<[f32; 2]>,
    /// Puntos de borde virtuales (no consumen RNG). Se concatenan a `points` antes
    /// de Delaunay: `allPoints = points.concat(boundary)`.
    pub boundary: Vec<[f32; 2]>,
    /// Topología de Voronoi (regenerada por `vor-import`).
    pub cells: GridCells,
    /// Vértices de Voronoi (regenerados por `vor-import`).
    pub vertices: VoronoiVertices,
    /// Features del grid (`grid.features` — slot `[6]` las trae, son la versión pre-repack).
    pub features: Vec<Feature>,
}

impl Grid {
    /// Cantidad de puntos reales (sin `boundary`). Cada uno = 1 celda jitterizada.
    #[inline]
    pub fn points_n(&self) -> usize {
        self.points.len()
    }

    /// Cantidad total de puntos usados en Delaunay: reales + boundary.
    #[inline]
    pub fn all_points_n(&self) -> usize {
        self.points.len() + self.boundary.len()
    }
}
