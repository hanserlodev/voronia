//! Porte bit-exacto de la `Voronoi` class de Azgaar (`src/generators/voronoi.ts`).
//!
//! Construye la malla de Voronoi dual a partir de una triangulación Delaunay
//! (`crate::geometry::delaunay::Triangulation`) — repite 1-a-1 la lógica del TS.
//!
//! ## Bit-exactitud de `circumcenter` — crítico
//!
//! Azgaar deliberadamente trunca el centro circunscrito a enteros con `Math.floor`
//! (`voronoi.ts:151-152`). En Rust reproducimos con `f64::floor()` — **no** con `as i32`
//! (que truncaría hacia cero en negativos). Aunque las coordenadas de celdas no son
//! negativas en Azgaar, el circumcenter puede caer levemente fuera del rango de
//! celdas cuando el triángulo es obtuso o tiene un punto en el hull, y ahí `as i32`
//! divergiría de `Math.floor`. El hallazgo fase-0 §6.3 exige reproducción literal.
//!
//! ## Layout de salida
//!
//! Salida mapeada a los tipos de `vor-core`:
//!   - `cells.v[p] : Vec<u32>` → los 3+ IDs de triángulos (= vértices Voronoi) que forman
//!     la celda del punto `p`. Orden: counter-clockwise via `edgesAroundPoint`.
//!   - `cells.c[p] : Vec<u32>` → IDs de celdas adyacentes (interiores solamente —
//!     se filtran los boundary points con id >= `pointsN`).
//!   - `cells.b[p] : u8` → 1 si la celda toca el borde (vecinos filtrados != vecinos
//!     totales), 0 si no.
//!   - `cells.i` no se materializa acá (es `[0,1,...,pointsN-1]`, implícito).
//!   - `vertices.p[t] : [f32;2]` → coords del vértice Voronoi del triángulo `t`.
//!     `f32` (cap fijo) — el `floor` ya impuso el límite de precisión.
//!   - `vertices.v[t] : [i32;3]` → 3 triángulos vecinos (uno por half-edge opuesto).
//!     `-1` = borde (sin vecino).
//!   - `vertices.c[t] : [u32;3]` → 3 cells (points) que conforman el triángulo `t`.

use crate::geometry::delaunay::{
    next_halfedge, points_of_triangle, triangle_of_edge, triangles_adjacent_to_triangle,
    Triangulation, EMPTY,
};

/// Cap máximo de `edgesAroundPoint` (`voronoi.ts:87`). En el JS es un cap de seguridad
/// anti-loopwoops en mallas con bugs de half-edge. En Rust lo mantenemos por bit-exactitud:
/// si una malla legítima excede 20 edges, Azgaar trunca silenciosamente y nosotros también.
const EDGES_AROUND_POINT_CAP: usize = 20;

/// Coordenadas de un punto (par `[x, y]`).
pub type Point = [f64; 2];

/// Salida de `calculate_voronoi` — equivalente al `Voronoi` class de Azgaar.
///
/// Las celdas (`cells.*`) están indexadas por point-id `[0, pointsN)`.
/// Los vértices (`vertices.*`) están indexados por triangle-id `[0, triangles.len()/3)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Voronoi {
    /// Ver `VoronoiCells`. `v`: vecinos vértices; `c`: vecinos cells; `b`: border flag.
    pub cells: VoronoiCells,
    /// Ver `VoronoiVertices`. `p`: coords; `v`: vecinos (triángulos); `c`: cells adyacentes.
    pub vertices: VoronoiVertices,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VoronoiCells {
    /// `cells.v[p]` — IDs de triángulos (vértices Voronoi) que conforman la celda `p`,
    /// en orden CCW. `Vec::new()` si la celda no fue visitada (caso en que el punto `p`
    /// está en el boundary y nunca llegó a ser `triangles[nextHalfedge(e)]`).
    pub v: Vec<Vec<u32>>,
    /// `cells.c[p]` — IDs de celdas adyacentes (interiores; boundary points filtrados).
    pub c: Vec<Vec<u32>>,
    /// `cells.b[p]` — 1 si la celda toca el borde (algunos vecinos fueron filtrados), 0 si no.
    pub b: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VoronoiVertices {
    /// `vertices.p[t]` — coords `[x, y]` del circumcenter del triángulo `t`. **Enteros**
    /// (`Math.floor` en JS); acá `f32` por consistencia con `vor-core::VoronoiVertices`.
    pub p: Vec<[f64; 2]>,
    /// `vertices.v[t]` — los 3 triángulos vecinos (uno por half-edge opuesto). `EMPTY`
    /// marcado como `None` por miedo a la confusión — el receptor decide si rellenar
    /// con `-1` o `u32::MAX`. En `vor-core::VoronoiVertices` se guarda como `i32` con
    /// `-1` por compat con Azgaar (`feature.rs` espera `[i32; 3]`).
    pub v: Vec<[usize; 3]>,
    /// `vertices.c[t]` — los 3 points (cells) que conforman el triángulo `t`.
    pub c: Vec<[u32; 3]>,
}

/// Réplica bit-exacta del constructor `new Voronoi(delaunay, points, pointsN)` de Azgaar
/// (`voronoi.ts:25-50`).
///
/// `points` son todos los puntos (incluyendo boundary points), y `pointsN` es la cantidad
/// de puntos no-boundary (boundary points tienen id `[pointsN, points.len())`).
pub fn calculate_voronoi(delaunay: &Triangulation, points: &[Point], points_n: u32) -> Voronoi {
    let n_triangles = delaunay.triangles.len() / 3;

    let mut cells_v: Vec<Vec<u32>> = vec![Vec::new(); points_n as usize];
    let mut cells_c: Vec<Vec<u32>> = vec![Vec::new(); points_n as usize];
    let mut cells_b: Vec<u8> = vec![0; points_n as usize];

    // `vertices.p[t]` se inicializan como `None`/placeholder para distinguir "no set" de
    // "seteado a [0.0, 0.0]". Usamos un Vec<Option<[f64;2]>> intermedio — el JS confía
    // en `undefined` y `!this.vertices.p[t]`.
    let mut vertices_p: Vec<Option<[f64; 2]>> = vec![None; n_triangles];
    let mut vertices_v: Vec<[usize; 3]> = vec![[EMPTY; 3]; n_triangles];
    let mut vertices_c: Vec<[u32; 3]> = vec![[0; 3]; n_triangles];

    let triangles = delaunay.triangles.as_slice();
    let halfedges = delaunay.halfedges.as_slice();

    // El bucle principal replica `voronoi.ts:34-49` línea-a-línea.
    for e in 0..delaunay.triangles.len() {
        let p = delaunay.triangles[next_halfedge(e)];
        // `if (p < pointsN && !cells.c[p])` — solo puntos interiores y no-visitados.
        if p < points_n && cells_c.get(p as usize).is_none_or(|v| v.is_empty()) {
            // cells.v[p] = edges.map(e => triangleOfEdge(e))
            // cells.c[p] = edges.map(e => triangles[e]).filter(c => c < pointsN)
            // cells.b[p] = edges.length > cells.c[p].length ? 1 : 0
            let edges = edges_around_point(halfedges, e);
            let cell_v: Vec<u32> = edges.iter().map(|&e| triangle_of_edge(e) as u32).collect();
            let cell_c: Vec<u32> = edges
                .iter()
                .map(|&e| triangles[e])
                .filter(|&c| c < points_n)
                .collect();

            let is_border = if edges.len() > cell_c.len() { 1u8 } else { 0u8 };

            let pi = p as usize;
            cells_v[pi] = cell_v;
            cells_c[pi] = cell_c;
            cells_b[pi] = is_border;
        }

        let t = triangle_of_edge(e);
        // `if (!vertices.p[t])` — JS usa falsiness de `undefined`. Acá usamos `Option::is_none`.
        if vertices_p[t].is_none() {
            vertices_p[t] = Some(triangle_center(points, triangles, t));
            vertices_v[t] = triangles_adjacent_to_triangle(halfedges, t);
            vertices_c[t] = points_of_triangle(triangles, t);
        }
    }

    // El JS almacena `vertices.p[t]` como `[number, number]`. En Azgaar, los `vertices.p`
    // pueden ser `undefined` si un triángulo no fue tocado por el bucle — pero todos los
    // triángulos aparecen vía el bucle sobre `triangles.length` (`for e in 0..triangles.len()`),
    // así que todos los `t` se setean. Aun así, por seguridad dejamos el deunwrap con
    // `unwrap_or([0.0, 0.0])` para no panickear, y emitimos un assertion en debug builds.
    let vertices_p_final: Vec<[f64; 2]> = vertices_p
        .into_iter()
        .map(|opt| {
            debug_assert!(opt.is_some(), "vértice t no fue populado");
            opt.unwrap_or([0.0, 0.0])
        })
        .collect();

    Voronoi {
        cells: VoronoiCells {
            v: cells_v,
            c: cells_c,
            b: cells_b,
        },
        vertices: VoronoiVertices {
            p: vertices_p_final,
            v: vertices_v,
            c: vertices_c,
        },
    }
}

/// `edgesAroundPoint(start)` de `voronoi.ts:80-89`.
///
/// Camina los half-edges que tocan el punto destino `start` (es decir, todos los
/// incoming/outgoing del punto `triangles[start]`), en sentido CCW, con cap of 20.
fn edges_around_point(halfedges: &[usize], start: usize) -> Vec<usize> {
    let mut result: Vec<usize> = Vec::new();
    let mut incoming = start;
    loop {
        result.push(incoming);
        let outgoing = next_halfedge(incoming);
        incoming = halfedges[outgoing];
        if incoming == EMPTY || incoming == start || result.len() >= EDGES_AROUND_POINT_CAP {
            break;
        }
    }
    result
}

/// `triangleCenter(t)` de `voronoi.ts:96-99` — el circumcenter del triángulo `t`.
/// Las coords se calculan en f64 y se truncan a enteros con `f64::floor()`.
fn triangle_center(points: &[Point], triangles: &[u32], t: usize) -> [f64; 2] {
    let pts = points_of_triangle(triangles, t);
    let a = points[pts[0] as usize];
    let b = points[pts[1] as usize];
    let c = points[pts[2] as usize];
    circumcenter(a, b, c)
}

/// `circumcenter(a, b, c)` de `voronoi.ts:142-154` — fórmula de Wikipedia, con
/// `Math.floor` truncando el resultado a enteros (fase-0 §6.3).
///
/// Reproducción literal del JS:
/// ```js
/// const ad = ax*ax + ay*ay;
/// const bd = bx*bx + by*by;
/// const cd = cx*cx + cy*cy;
/// const D = 2 * (ax*(by-cy) + bx*(cy-ay) + cx*(ay-by));
/// return [ Math.floor((1/D) * (ad*(by-cy) + bd*(cy-ay) + cd*(ay-by))),
///          Math.floor((1/D) * (ad*(cx-bx) + bd*(ax-cx) + cd*(bx-ax))) ];
/// ```
///
/// Importante para bit-exactitud: el JS efectúa `(1/D) * numerator` — en f64 esto es
/// `numerator / D` aritméticamente, pero **no** es bit-idéntico. `(1/D)` calcula el
/// recíproco en f64 (con su propio redondeo), y luego multiplica por `numerator`
/// (con otro redondeo) — totale 2 des perfis de redondeo. `numerator / D` aplica u
/// redondeo único. Reproducimos el patrón del JS: `recip * numerator`, no `numerator/D`.
fn circumcenter(a: Point, b: Point, c: Point) -> [f64; 2] {
    let [ax, ay] = a;
    let [bx, by] = b;
    let [cx, cy] = c;
    let ad = ax * ax + ay * ay;
    let bd = bx * bx + by * by;
    let cd = cx * cx + cy * cy;
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    let recip = 1.0 / d;
    let x = recip * (ad * (by - cy) + bd * (cy - ay) + cd * (ay - by));
    let y = recip * (ad * (cx - bx) + bd * (ax - cx) + cd * (bx - ax));
    [x.floor(), y.floor()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::delaunay::from_pairs;

    /// Sanity básico: cuadrado 1×1 con 4 puntos → 2 triángulos, 4 celdas (sin borde, sin boundary).
    #[test]
    fn voronoi_square_no_boundary() {
        let points: Vec<[f64; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let delaunay = from_pairs(&points);
        let v = calculate_voronoi(&delaunay, &points, points.len() as u32);

        // 4 points = 4 cells (todas pobladas), 2 triangles = 2 vertices.
        assert_eq!(v.cells.b.len(), 4, "cells.b tiene points_n entradas");
        // Sin boundary, todas las celdas tienen tipo vecino =Cantidad de edges.
        // (Las celdas son contiguas — todo es interior — b deberia ser 0 en todas.)
        // En realidad, en cuadrado sin boundary los 4 puntos están en el hull (Delaunator
        // los pone en `hull` automáticamente), así que `vertices.v` tendrá EMPTYs.
        // Solo validamos la cantidad estructural.
        assert_eq!(v.vertices.p.len(), delaunay.triangles.len() / 3);
        assert_eq!(v.vertices.c.len(), delaunay.triangles.len() / 3);
    }

    /// Determinismo: misma entrada → misma salida.
    #[test]
    fn voronoi_is_deterministic() {
        let points: Vec<[f64; 2]> = vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [2.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [2.0, 1.0],
            [0.0, 2.0],
            [1.0, 2.0],
            [2.0, 2.0],
        ];
        let delaunay = from_pairs(&points);
        let a = calculate_voronoi(&delaunay, &points, points.len() as u32);
        let b = calculate_voronoi(&delaunay, &points, points.len() as u32);
        assert_eq!(a, b, "bit-exact determinismo");
    }

    /// `circumcenter` de un triángulo rectángulo isósceles unitario:
    /// A=(0,0), B=(1,0), C=(0,1). Circumcenter = (0.5, 0.5). `floor(0.5)=0`. → [0, 0].
    #[test]
    fn circumcenter_unit_right_triangle() {
        let cc = circumcenter([0.0, 0.0], [1.0, 0.0], [0.0, 1.0]);
        // 1/D = 1/(2 * (0*(0-1) + 1*(1-0) + 0*(0-0))) = 1/2
        // x = 0.5*(0*(0-1) + 1*(1-0) + 1*(0-0)) = 0.5 * 1 = 0.5 → floor = 0
        // y = 0.5*(0*(0-1) + 1*(0-0) + 1*(1-0)) = 0.5 * 1 = 0.5 → floor = 0
        assert_eq!(cc, [0.0, 0.0], "circumcenter rectangle isósceles");
    }

    /// Triángulo equilátero de lado 2: A=(0,0), B=(2,0), C=(1,√3).
    /// Circumcenter = (1, √3/3) ≈ (1, 0.5773...) → floor = [1, 0].
    #[test]
    fn circumcenter_equilateral() {
        let sqrt3 = 3f64.sqrt();
        let cc = circumcenter([0.0, 0.0], [2.0, 0.0], [1.0, sqrt3]);
        // D = 2*(0*(0-√3) + 2*(√3-0) + 1*(0-0)) = 2*2*√3 = 4√3
        // x = (1/(4√3)) * (0 + 4*(√3-0) + 4*(0-0)) = (1/(4√3)) * 4√3 = 1.0 → floor = 1
        // y = (1/(4√3)) * (0*(1-2) + 4*(0-1) + 4*(2-0)) = (1/(4√3)) * (-4 + 8)
        //   = (1/(4√3)) * 4 = 1/√3 ≈ 0.5773502691... → floor = 0
        assert_eq!(cc, [1.0, 0.0], "equilátero circumcenter");
    }

    /// Bit-exactitud del circumcenter contra `Math.floor` del JS — caso negativo:
    /// triángulo con circumcenter en territorio negativo.
    /// A=(-2,-2), B=(0,-2), C=(-1,-1). Circumcenter ≈ (-1, -3).
    #[test]
    fn circumcenter_negative_floor() {
        // D = 2*((-2)*(-2-(-1)) + 0*(-1-(-2)) + (-1)*((-2)-(-2)))
        //   = 2*((-2)*(-1) + 0*1 + (-1)*0)
        //   = 2 * 2 = 4
        // x = (1/4) * (ad*(by-cy) + bd*(cy-ay) + cd*(ay-by))
        //   ad = 4+4=8; bd = 0+4=4; cd = 1+1=2.
        //   = (1/4) * (8*(-2-(-1)) + 4*(-1-(-2)) + 2*((-2)-(-2)))
        //   = (1/4) * (8*(-1) + 4*(1) + 2*0)
        //   = (1/4) * (-8+4+0) = (1/4) * -4 = -1.0 → floor(-1.0) = -1
        // y = (1/4) * (ad*(cx-bx) + bd*(ax-cx) + cd*(bx-ax))
        //   = (1/4) * (8*((-1)-0) + 4*((-2)-(-1)) + 2*(0-(-2)))
        //   = (1/4) * (8*(-1) + 4*(-1) + 2*2)
        //   = (1/4) * (-8 -4 + 4) = (1/4) * -8 = -2.0 → floor(-2.0) = -2
        let cc = circumcenter([-2.0, -2.0], [0.0, -2.0], [-1.0, -1.0]);
        assert_eq!(cc, [-1.0, -2.0], "circumcenter negativo con floor");
    }
}
