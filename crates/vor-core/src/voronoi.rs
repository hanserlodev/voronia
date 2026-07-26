//! Geometría de Voronoi derivada de Delaunay.
//!
//! Importante para bit-exactitud contra Azgaar (ver `docs/fase-0-investigacion.md` §6.3):
//! los `positions` de los vértices de Voronoi son **coordenadas enteras** — Azgaar
//! trunca el circumcenter con `Math.floor` deliberadamente, y Voronia reproduce eso.
//! El cálculo se hace en `vor-import` (es lógica, no dato); acá solo vivirá el layout
//! de geometría, "congelado".

/// Vértices de la malla de Voronoi. Estructura SoA indexada por triángulo `t = floor(e/3)`.
///
/// Cada triángulo de la triangulación de Delaunay tiene asociado un vértice de Voronoi
/// (el circumcenter de ese triángulo). `positions[t]`, `adjacent_cells[t]` y
/// `adjacent_vertices[t]` describen ese vértice. Los IDs `-1` en `adjacent_vertices`
/// marcan bordes (sin triángulo vecino del lado opuesto).
///
/// Además el campo `cell_rings` (las `cells.v` de Azgaar) guarda, por cada celda `p`
/// (= punto), la lista de IDs de triángulos cuyos circumcentros conforman el polígono
/// de la celda, en orden CCW. **No se persiste** (es derivable del Delaunay), pero
/// `vor-import` lo repuebla en runtime — el renderer lo necesita para triangular el
/// polígono de cada celda sin tener que recalcular geometría.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VoronoiVertices {
    /// Coordenadas `[x, y]` del vértice. **Enteros** en Azgaar (por `Math.floor` en `circumcenter`);
    /// acá guardamos `f32` para render pero el cap de precisión lo impone `vor-import` al construir.
    pub positions: Vec<[f32; 2]>,
    /// Los 3 IDs de celdas (`pointId`s de la triangulación) que conforman el triángulo `t`.
    pub adjacent_cells: Vec<[i32; 3]>,
    /// IDs de los 3 triángulos vecinos (uno por half-edge opuesto). `-1` si el half-edge
    /// es de borde (sin triángulo vecino).
    pub adjacent_vertices: Vec<[i32; 3]>,
    /// `cells.v[p]` de Azgaar: IDs de triángulos cuyos circumcentros conforman el polígono
    /// de la celda `p`, en orden CCW (vía `edgesAroundPoint`, cap 20). No se persiste
    /// (derivable del Delaunay); `vor-import` lo repuebla en runtime para que el renderer
    /// no recalcula mallas.
    #[serde(skip)]
    pub cell_rings: Vec<Vec<u32>>,
}

impl VoronoiVertices {
    /// Cantidad de vértices (triángulos) de la malla.
    #[inline]
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// `true` si la malla no tiene vértices.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}
