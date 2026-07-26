// Generator de fixture para el test bit-exacto de `voronoi` Rust vs `voronoi.ts` de Azgaar.
//
// Reproduce `placePoints(2000, 2000, cellsDesired=10000, seed="861039636")` (mismo input que
// `delaunay_grid_2000x2000_c10k_seed_861039636_selfref.json`), corre `delaunator@5.1.0(JS)`
// sobre `allPoints = points.concat(boundary)` (igual que `calculateVoronoi` de Azgaar),
// y luego corre la `Voronoi` class de `voronoi.ts:18-155` (replicada vanilla abajo, sin TS)
// para producir `cells.v/c/b/i` y `vertices.p/v/c`.
//
// Serializa:
//   - cells.v_bits   : u32 (triangle id) decimal strings, agrupados por cell, longitudes variables.
//                       Encoded como array de arrays. Entero `[p]` = lista de vértices Voronoi.
//   - cells.c_bits   : u32 (cell id adyacente) decimal strings, agrupados por cell.
//   - cells.b_bits   : u8 (0/1) por cell.
//   - vertices.p_bits : f64 bits (BigInt64Array) — coords [x,y] por triángulo (2 × nTriangles entradas).
//   - vertices.v_bits : i32 (-1=EMPTY, o triangle id) × 3 por triángulo.
//   - vertices.c_bits : u32 (cell id) × 3 por triángulo.
//
// El test Rust (`voronoi_bit_exact.rs`) levanta este fixture y compara con `calculate_voronoi`
// bit-a-bit. Si diverge — bug silencioso en el porte del circumcenter (fase-0 §6.3).

const fs = require("fs");
const path = require("path");

const Alea = require(path.join(__dirname, "alea-1.0.1.original.js"));
const Delaunator = require(path.join(__dirname, "delaunator-5.1.0.js"));

const rn = (v, d = 0) => {
  const m = 10 ** d;
  return Math.round(v * m) / m;
};

const getBoundaryPoints = (width, height, spacing) => {
  const offset = rn(-1 * spacing);
  const bSpacing = spacing * 2;
  const w = width - offset * 2;
  const h = height - offset * 2;
  const numberX = Math.ceil(w / bSpacing) - 1;
  const numberY = Math.ceil(h / bSpacing) - 1;
  const points = [];
  for (let i = 0.5; i < numberX; i++) {
    const x = Math.ceil((w * i) / numberX + offset);
    points.push([x, offset], [x, h + offset]);
  }
  for (let i = 0.5; i < numberY; i++) {
    const y = Math.ceil((h * i) / numberY + offset);
    points.push([offset, y], [w + offset, y]);
  }
  return points;
};

const getJitteredGrid = (width, height, spacing) => {
  const radius = spacing / 2;
  const jittering = radius * 0.9;
  const doubleJittering = jittering * 2;
  const jitter = () => Math.random() * doubleJittering - jittering;
  const points = [];
  for (let y = radius; y < height; y += spacing) {
    for (let x = radius; x < width; x += spacing) {
      const xj = Math.min(rn(x + jitter(), 2), width);
      const yj = Math.min(rn(y + jitter(), 2), height);
      points.push([xj, yj]);
    }
  }
  return points;
};

const placePoints = (W, H, cellsDesired) => {
  const spacing = rn(Math.sqrt((W * H) / cellsDesired), 2);
  const boundary = getBoundaryPoints(W, H, spacing);
  const points = getJitteredGrid(W, H, spacing);
  const cellsX = Math.floor((W + 0.5 * spacing - 1e-10) / spacing);
  const cellsY = Math.floor((H + 0.5 * spacing - 1e-10) / spacing);
  return { spacing, cellsDesired, boundary, points, cellsX, cellsY };
};

// --- Réplica bit-exacta de `Voronoi` class (voronoi.ts:18-155). Vanilla JS (sin TS).
const Voronoi = class {
  constructor(delaunay, points, pointsN) {
    this.delaunay = delaunay;
    this.points = points;
    this.pointsN = pointsN;
    this.cells = { v: [], c: [], b: [], i: new Uint32Array() };
    this.vertices = { p: [], v: [], c: [] };

    for (let e = 0; e < this.delaunay.triangles.length; e++) {
      const p = this.delaunay.triangles[this.nextHalfedge(e)];
      if (p < this.pointsN && !this.cells.c[p]) {
        const edges = this.edgesAroundPoint(e);
        this.cells.v[p] = edges.map((ee) => this.triangleOfEdge(ee));
        this.cells.c[p] = edges
          .map((ee) => this.delaunay.triangles[ee])
          .filter((c) => c < this.pointsN);
        this.cells.b[p] = edges.length > this.cells.c[p].length ? 1 : 0;
      }
      const t = this.triangleOfEdge(e);
      if (!this.vertices.p[t]) {
        this.vertices.p[t] = this.triangleCenter(t);
        this.vertices.v[t] = this.trianglesAdjacentToTriangle(t);
        this.vertices.c[t] = this.pointsOfTriangle(t);
      }
    }
  }
  pointsOfTriangle(t) {
    return this.edgesOfTriangle(t).map((e) => this.delaunay.triangles[e]);
  }
  trianglesAdjacentToTriangle(t) {
    const triangles = [];
    for (const e of this.edgesOfTriangle(t)) {
      const op = this.delaunay.halfedges[e];
      triangles.push(this.triangleOfEdge(op));
    }
    return triangles;
  }
  edgesAroundPoint(start) {
    const result = [];
    let incoming = start;
    do {
      result.push(incoming);
      const outgoing = this.nextHalfedge(incoming);
      incoming = this.delaunay.halfedges[outgoing];
    } while (incoming !== -1 && incoming !== start && result.length < 20);
    return result;
  }
  triangleCenter(t) {
    const vertices = this.pointsOfTriangle(t).map((p) => this.points[p]);
    return this.circumcenter(vertices[0], vertices[1], vertices[2]);
  }
  edgesOfTriangle(t) {
    return [3 * t, 3 * t + 1, 3 * t + 2];
  }
  triangleOfEdge(e) {
    return Math.floor(e / 3);
  }
  nextHalfedge(e) {
    return e % 3 === 2 ? e - 2 : e + 1;
  }
  circumcenter(a, b, c) {
    const [ax, ay] = a, [bx, by] = b, [cx, cy] = c;
    const ad = ax * ax + ay * ay;
    const bd = bx * bx + by * by;
    const cd = cx * cx + cy * cy;
    const D = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    return [
      Math.floor((1 / D) * (ad * (by - cy) + bd * (cy - ay) + cd * (ay - by))),
      Math.floor((1 / D) * (ad * (cx - bx) + bd * (ax - cx) + cd * (bx - ax))),
    ];
  }
};

// --- Main.
(() => {
  const W = 2000, H = 2000, CELLS = 10000, SEED = "861039636";

  const random = new Alea(SEED);
  Math.random = random;

  const g = placePoints(W, H, CELLS);
  const allPoints = g.points.concat(g.boundary);
  const pointsN = g.points.length; // 10000

  console.log("[done] points:", pointsN, "boundary:", g.boundary.length);
  const delaunay = Delaunator.from(allPoints);
  console.log("[delaunay] triangles:", delaunay.triangles.length, "hull:", delaunay.hull.length);

  const voronoi = new Voronoi(delaunay, allPoints, pointsN);
  console.log("[voronoi] cells populated:", voronoi.cells.v.filter((v) => v).length, "vertices:", voronoi.vertices.p.filter((p) => p).length);

  // --- Serializar.
  // cells.v — array de arrays. Cada entrada es un array de u32 (triangle ids). Algunas entries son undefined.
  const cellsV = voronoi.cells.v.map((arr) => arr ? arr.map((t) => t.toString(10)) : null);
  const cellsC = voronoi.cells.c.map((arr) => arr ? arr.map((c) => c.toString(10)) : null);
  const cellsB = voronoi.cells.b.map((b) => b === undefined || b === null ? null : b.toString(10));

  // vertices.p — f64 bits. `vertices.p[t]` es [x,y] o undefined. Serializamos como BigInt64Array.
  // nTriangles = delaunay.triangles.length / 3.
  const nTriangles = delaunay.triangles.length / 3;
  // Creamos flat f64 array de tamaño nTriangles * 2. Para undefined, dejamos 0 (el test debe detectar ambos).
  const vFlat = new Float64Array(nTriangles * 2);
  for (let t = 0; t < nTriangles; t++) {
    const p = voronoi.vertices.p[t];
    if (p) {
      vFlat[2*t] = p[0];
      vFlat[2*t+1] = p[1];
    }
  }
  const vBits = new BigInt64Array(vFlat.buffer);
  const verticesPBits = Array.from(vBits, (b) => b.toString(10));

  // vertices.v[t] — array de 3 triangle ids (algunos pueden ser -1 = EMPTY si el half-edge es de borde).
  // JS: triangleOfEdge(-1) = Math.floor(-1/3) = -1 (truncation hacia cero... error? Math.floor(-1/3) = -1).
  // En azgaar.ts no tiran null check — `if (opposite === -1) triangles.push(-1)`. Eso da `triangleOfEdge(-1) = -1`.
  // Rust: si halfedges[e] == EMPTY = usize::MAX, `triangle_of_edge(usize::MAX) = usize::MAX/3 = impar`. Distinto!
  // → En Rust necesitamos replicar: `triangles_adjacent_to_triangle` ya hace `if opposite != EMPTY`. Por eso
  // guardamos vbits como i32 con -1 para EMPTY.
  const verticesV = []; // array of [t1, t2, t3] with -1 for EMPTY.
  for (let t = 0; t < nTriangles; t++) {
    const v = voronoi.vertices.v[t] || [0, 0, 0];
    verticesV.push(v.map((x) => x.toString(10)));
  }

  // vertices.c[t] — 3 u32 (cell ids).
  const verticesC = [];
  for (let t = 0; t < nTriangles; t++) {
    const c = voronoi.vertices.c[t] || [0, 0, 0];
    verticesC.push(c.map((x) => x.toString(10)));
  }

  const out = {
    description: "Fixture self-reference: placePoints(2000,2000,10000,seed=861039636) + delaunator@5.1.0 + Voronoi class (voronoi.ts) JS. Test Rust debe reproducir cells.v/c/b y vertices.p/v/c bit-exacto.",
    seed: SEED, width: W, height: H, cellsDesired: CELLS,
    spacing: g.spacing, cellsX: g.cellsX, cellsY: g.cellsY,
    nPoints: pointsN, nBoundary: g.boundary.length,
    nAllPoints: allPoints.length, nTriangles,
    cells_v: cellsV,
    cells_c: cellsC,
    cells_b: cellsB,
    vertices_p_bits: verticesPBits,
    vertices_v: verticesV,
    vertices_c: verticesC,
  };

  const outPath = path.join(__dirname, "voronoi_grid_2000x2000_c10k_seed_861039636_selfref.json");
  fs.writeFileSync(outPath, JSON.stringify(out, null, 2));
  console.log("[wrote]", outPath, `(${(fs.statSync(outPath).size / 1024).toFixed(1)} KB)`);
})();
