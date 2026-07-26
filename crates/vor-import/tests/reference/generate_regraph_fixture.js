// Generator de fixture para el test bit-exacto de `re_graph` Rust vs `reGraph` JS.
//
// Setup synthetic:
//   - placePoints(2000, 2000, 10000, "861039636") → grid.points (10000) + grid.boundary (200).
//   - calculateVoronoi (Vor JS replicado, igual que generate_voronoi_fixture.js) sobre
//     allPoints = points.concat(boundary) → cells.c/b/i, vertices.p/v/c.
//   - Atributos synthetic para no depender de HeightmapGenerator/features:
//       grid.cells.h = Array filled 50 (interior land)
//       grid.cells.t = Array filled 2 (tierra interior, no costa)
//       grid.cells.f = Array filled 0 (feature_id 0)
//       grid.features = [{ type: "ocean" }] (placeholder — "ocean" para que el branch
//            de "lake" no dispare).
//   - Override del `d3.polygonArea` en el contexto: importamos d3 completo desde node_modules
//     de azgaar-fmg via un require relativo (alternativa: reimplementar shoelace in-line, ya lo
//     hicimos — usamos la versión local para asegurar bit-exactitud vs el JS fuente).
//
// Resultado del JS (`reGraph` vanilla replicado abajo):
//   - newCells.p: ids 0..nPoints (todos terrestres interiores, sin descarte, sin extras).
//   - newCells.g: [0,1,...,9999]
//   - newCells.h: [50,...,50]
//   - Compute second calculateVoronoi → pack.cells.v (topología) + vertices.
//   - pack.cells.area per cell via getPackPolygon(cellId) + shoelace, capped a 65535.
//
// Serializamos:
//   - pack_points_bits: f64 bits (BigInt64Array) — N*2 entradas.
//   - grid_id: u32 strings (nPoints entradas).
//   - pack_height: u8 strings (nPoints entradas).
//   - pack_area: u16 strings (nPoints entradas).
//   - vertices_p_bits: f64 bits (nTriangles * 2 entradas).
//   - vertices_v: array of [i32;3], -1=EMPTY.
//   - vertices_c: array of [u32;3].
//
// El test Rust reproduce inputData + corre `re_graph` y compara bit-a-bit.

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

// Réplica de Voronoi class (voronoi.ts:18-155).
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

const calculateVoronoi = (points, boundary) => {
  const allPoints = points.concat(boundary);
  const delaunay = Delaunator.from(allPoints);
  const voronoi = new Voronoi(delaunay, allPoints, points.length);
  const cells = voronoi.cells;
  cells.i = new Uint32Array(points.length).map((_, i) => i);
  return { cells, vertices: voronoi.vertices };
};

// d3.polygonArea — reimplementación local del shoelace (bit-exacta vs npm d3-polygon@3.0.1).
const polygonArea = (polygon) => {
  let i = -1, n = polygon.length, a, b = polygon[n - 1], area = 0;
  while (++i < n) {
    a = b;
    b = polygon[i];
    area += a[1] * b[0] - a[0] * b[1];
  }
  return area / 2;
};

const getPackPolygon = (cellId, pack) => {
  return pack.cells.v[cellId].map((v) => pack.vertices.p[v]);
};

// --- Réplica de `reGraph` (main.js:1157-1209).
const reGraph = (grid) => {
  const { cells: gridCells, points, features } = grid;
  const newCells = { p: [], g: [], h: [] };
  const spacing2 = grid.spacing ** 2;

  for (const i of gridCells.i) {
    const height = gridCells.h[i];
    const type = gridCells.t[i];
    if (height < 20 && type !== -1 && type !== -2) continue;
    if (type === -2 && (i % 4 === 0 || features[gridCells.f[i]].type === "lake")) continue;
    const [x, y] = points[i];
    addNewPoint(i, x, y, height);
    if (type === 1 || type === -1) {
      if (gridCells.b[i]) continue;
      gridCells.c[i].forEach(function (e) {
        if (i > e) return;
        if (gridCells.t[e] === type) {
          const dist2 = (y - points[e][1]) ** 2 + (x - points[e][0]) ** 2;
          if (dist2 < spacing2) return;
          const x1 = rn((x + points[e][0]) / 2, 1);
          const y1 = rn((y + points[e][1]) / 2, 1);
          addNewPoint(i, x1, y1, height);
        }
      });
    }
  }

  function addNewPoint(i, x, y, height) {
    newCells.p.push([x, y]);
    newCells.g.push(i);
    newCells.h.push(height);
  }

  const { cells: packCells, vertices } = calculateVoronoi(newCells.p, grid.boundary);
  const pack = { cells: packCells, vertices };
  pack.cells.p = newCells.p;
  pack.cells.g = newCells.g; // already regular array — typed array just for storage; we keep plain.
  pack.cells.h = newCells.h;
  // Replica `reGraph` JS (main.js:1157-1209):
  //   pack.cells.area = createTypedArray({ maxValue: TYPED_ARRAY_MAX.UINT16, length: packCells.i.length })
  //                       .map((_, cellId) => Math.min(area, TYPED_ARRAY_MAX.UINT16));
  // `createTypedArray(...)` retorna un Uint16Array (vía new Uint16Array(length)). El .map sobre
  // Uint16Array retorna otro Uint16Array con truncation ToUint32 (floor para valores >= 0).
  // Para bit-exactitud con el runtime real, hacemos lo mismo.
  const UINT16_MAX = 65535;
  const areaFloats = newCells.p.map((_, cellId) => {
    const area = Math.abs(polygonArea(getPackPolygon(cellId, pack)));
    return Math.min(area, UINT16_MAX);
  });
  // Cast to Uint16Array con truncation (repl ToUint32 + & 0xFFFF) — idéntico al Azgaar real.
  pack.cells.area = new Uint16Array(areaFloats);
  return { pack, newCells };
};

// === Main.
(() => {
  const W = 2000, H = 2000, CELLS = 10000, SEED = "861039636";
  Math.random = new Alea(SEED);
  const placed = placePoints(W, H, CELLS);

  // Build grid topologia via calculateVoronoi.
  const { cells, vertices } = calculateVoronoi(placed.points, placed.boundary);

  // Synthetic attributes: every cell = interior land (h=50, type=2).
  const grid = {
    spacing: placed.spacing,
    points: placed.points,
    boundary: placed.boundary,
    features: [{ type: "ocean" }], // placeholder feature id 0 = ocean
    cells: {
      i: cells.i,
      c: cells.c,
      b: cells.b,
      v: cells.v,
      f: new Uint16Array(placed.points.length),
      h: new Uint8Array(placed.points.length).fill(50),
      t: new Int8Array(placed.points.length).fill(2),
    },
    vertices,
  };

  const { pack } = reGraph(grid);
  console.log("[done] pack.cells.p:", pack.cells.p.length, "vertices:", pack.vertices.p.length);

  // Serializar.
  const pack_points_flat = new Float64Array(pack.cells.p.flat());
  const pointsBits = Array.from(new BigInt64Array(pack_points_flat.buffer), (b) => b.toString(10));

  const grid_id = pack.cells.g.map((g) => g.toString(10));
  const pack_height = pack.cells.h.map((h) => h.toString(10));
  // pack.cells.area es un Uint16Array (truncado "ToUint32" + bitand 0xFFFF, igual que el runtime Azgaar).
  // Serializamos directo el valor truncado u16 — sin Math.round.
  const pack_area = Array.from(pack.cells.area, (a) => a.toString(10));

  const verticesFlat = new Float64Array(pack.vertices.p.length * 2);
  for (let t = 0; t < pack.vertices.p.length; t++) {
    const p = pack.vertices.p[t];
    verticesFlat[2*t] = p[0];
    verticesFlat[2*t+1] = p[1];
  }
  const verticesPBits = Array.from(new BigInt64Array(verticesFlat.buffer), (b) => b.toString(10));

  const verticesV = pack.vertices.v.map((v) => v.map((x) => x.toString(10)));
  const verticesC = pack.vertices.c.map((c) => c.map((x) => x.toString(10)));

  const out = {
    description: "Fixture self-reference: reGraph (main.js) with synthetic h=50 t=2 for all 10000 grid cells. Test Rust debe reproducir pack.points (input al 2nd calculateVoronoi), cells.g/h/area y vertices bit-exactos.",
    seed: SEED, width: W, height: H, cellsDesired: CELLS,
    spacing: placed.spacing,
    nGridPoints: placed.points.length,
    nGridBoundary: placed.boundary.length,
    nPackPoints: pack.cells.p.length,
    nPackTriangles: pack.vertices.p.length,
    pack_points_bits: pointsBits,
    grid_id, pack_height, pack_area,
    vertices_p_bits: verticesPBits,
    vertices_v: verticesV,
    vertices_c: verticesC,
  };

  const outPath = path.join(__dirname, "regraph_h50_t2_grid_2000x2000_c10k_seed_861039636_selfref.json");
  fs.writeFileSync(outPath, JSON.stringify(out, null, 2));
  console.log("[wrote]", outPath, `(${(fs.statSync(outPath).size / 1024).toFixed(1)} KB)`);
})();
