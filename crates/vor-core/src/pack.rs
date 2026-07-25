//! Malla del pack — resultado del repacking grid→pack.
//!
//! El repacking (algoritmo `reGraph` en `public/main.js:1157-1209` de Azgaar) descarta
//! celdas innecesarias (océano profundo no-costero, ciertos lagos no-costeros) y agrega
//! puntos extra en costas (punto medio entre celdas vecinas del mismo tipo a ≥ spacing
//! de distancia). Después recalcula Delaunay/Voronoi sobre esos puntos (`calculateVoronoi`).
//!
//! Critically: si Voronia produce un malla de pack distinta (aunque sea por 1 celda
//! o por orden distinto en `newCells.p`), el mapeo `pack.cells.g[packId] → gridId` queda
//! distinto, y todos los atributos que Azgaar serializa indexados por id de pack
//! (bioma, state, burg, ...) se aplican a celdas equivocadas — **bug silencioso sin
//! error en runtime**. Por eso el repacking en `vor-import` tiene que ser bit-exacto.

use crate::cells::PackCells;
use crate::feature::Feature;
use crate::voronoi::VoronoiVertices;

/// Malla del pack — sobre la que opera la mayoría de la simulación.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Pack {
    /// Puntos del pack (post-repack; distintos de `grid.points`).
    pub points: Vec<[f32; 2]>,
    /// Boundary reutilizado del grid (Azgaar pasa `grid.boundary` a la segunda `calculateVoronoi`).
    pub boundary: Vec<[f32; 2]>,
    /// Topología de celdas (atributos serializados + `grid_id` mapping).
    pub cells: PackCells,
    /// Vértices de Voronoi (regenerados por `vor-import` tras `reGraph`).
    pub vertices: VoronoiVertices,
    /// Features del pack (`pack.features`, slot `[12]`).
    pub features: Vec<Feature>,
}

impl Pack {
    /// Cantidad de celdas del pack.
    #[inline]
    pub fn points_n(&self) -> usize {
        self.points.len()
    }
}
