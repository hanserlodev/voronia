//! Voronoi geometry derived from Delaunay.
//!
//! Important for bit-exactness against Azgaar (see `docs/phases/phase-0-research.md` §6.3):
//! the `positions` of the Voronoi vertices are **integer coordinates** — Azgaar
//! deliberately truncates the circumcenter with `Math.floor`, and Voronia reproduces that.
//! The computation happens in `vor-import` (it is logic, not data); only the geometry
//! layout lives here, "frozen".

/// Voronoi mesh vertices. SoA structure indexed by triangle `t = floor(e/3)`.
///
/// Each triangle of the Delaunay triangulation has an associated Voronoi vertex
/// (the circumcenter of that triangle). `positions[t]`, `adjacent_cells[t]` and
/// `adjacent_vertices[t]` describe that vertex. The `-1` IDs in `adjacent_vertices`
/// mark edges (no neighboring triangle on the opposite side).
///
/// Additionally the `cell_rings` field (Azgaar's `cells.v`) stores, for each cell `p`
/// (= point), the list of triangle IDs whose circumcenters form the cell's polygon,
/// in CCW order. **Not persisted** (derivable from the Delaunay), but
/// `vor-import` repopulates it at runtime — the renderer needs it to triangulate
/// each cell's polygon without recomputing geometry.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VoronoiVertices {
    /// Vertex coordinates `[x, y]`. **Integers** in Azgaar (due to `Math.floor` in `circumcenter`);
    /// here we store `f32` for rendering, but the precision cap is enforced by `vor-import` when building.
    pub positions: Vec<[f32; 2]>,
    /// The 3 cell IDs (triangulation `pointId`s) that form triangle `t`.
    pub adjacent_cells: Vec<[i32; 3]>,
    /// IDs of the 3 neighboring triangles (one per opposite half-edge). `-1` if the half-edge
    /// is on the border (no neighboring triangle).
    pub adjacent_vertices: Vec<[i32; 3]>,
    /// Azgaar's `cells.v[p]`: IDs of triangles whose circumcenters form cell `p`'s polygon,
    /// in CCW order (via `edgesAroundPoint`, cap 20). Not persisted (derivable from the Delaunay);
    /// `vor-import` repopulates it at runtime so the renderer does not recompute meshes.
    #[serde(skip)]
    pub cell_rings: Vec<Vec<u32>>,
}

impl VoronoiVertices {
    /// Number of mesh vertices (triangles).
    #[inline]
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// `true` if the mesh has no vertices.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}
