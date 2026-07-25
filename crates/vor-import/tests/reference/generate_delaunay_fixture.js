// Generator de fixture para el test bit-exacto de `delaunator` Rust vs `delaunator@5.1.0` JS.
//
// Reproduce `placePoints(2000, 2000, cellsDesired=10000, seed="861039636")` de Azgaar
// (graphUtils.ts:46-98) usando:
//  - `alea@1.0.1` (Johannes Baagøe, npm) ya descargado en `alea-1.0.1.original.js` (este dir).
//  - `delaunator@5.1.0` (npm) descargado en `/tmp/opencode/delaunator_pkg2/package/` y copiado
//    a este dir como `delaunator-5.1.0.js`.
//
// Genera `delaunay_grid_2000x2000_c10k_seed_861039636_selfref.json` con:
//   - `points_bits`     : 10000 puntos (x,y) → 20000 f64 bits como strings decimalizados de u64.
//   - `boundary_bits`   : 200 puntos boundary → 400 f64 bits.
//   - `triangles_bits`  : Uint32Array del Delaunator JS → BigInt64Array serializado como strings
//                         decimalizados de i64 (negativos como complemento a 2 → BigInt).
//   - `halfedges_bits`  : Int32Array (-1 = EMPTY) → BigInt64Array → strings decimalizados de i64.
//
// Nota: `triangles` y `halfedges` son producidos por el *JS* `delaunator@5.1.0`. El test Rust
// corre el crate `delaunator = "1.1"` sobre los mismos `points`+`boundary` (cargados desde
// este fixture) y compara bit-a-bit. Si divergen, el crate Rust NO es un porte bit-exacto y hay
// que portear manualmente desde `delaunator-5.1.0.js`.

const fs = require("fs");
const path = require("path");

// --- Cargar Alea@1.0.1 desde el fuente original ya descargado (this dir).
// API de Baagøe: `const rand = new Alea(seed)` retorna una *function*; llamar `rand()` o
// `rand.next()` consume un float del stream. No hay `.random()` en el objeto.
const Alea = require(path.join(__dirname, "alea-1.0.1.original.js"));

// --- Cargar Delaunator@5.1.0 (UMD). Bajo `require()`, `module.exports = factory()` y expone la
//   clase `Delaunator` con el método estático `Delaunator.from(points)` que toma un array de
//   `[x,y]` pares (o un flat array con `{x,y}`). Azgaar usa `Delaunator.from(points)` direct.
const Delaunator = require(path.join(__dirname, "delaunator-5.1.0.js"));
if (typeof Delaunator !== "function" || typeof Delaunator.from !== "function") {
  throw new Error("Delaunator no se cargó correctamente");
}

// --- Réplica de `rn` (numberUtils.ts:7-10): `Math.round(v * 10^d) / 10^d`.
const rn = (v, d = 0) => {
  const m = 10 ** d;
  return Math.round(v * m) / m;
};

// --- Réplica de `getBoundaryPoints` (graphUtils.ts:17-37) — no consume RNG.
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

// --- Réplica de `getJitteredGrid` (graphUtils.ts:46-61) — consume `Math.random`.
// `Math.random` fue monkey-patcheado arriba (línea ~) por `Alea(seed).random`.
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

// --- Réplica de `placePoints` (graphUtils.ts:69-98). `cellsDesired` se hardcodea (10000).
const placePoints = (graphWidth, graphHeight, cellsDesired) => {
  const spacing = rn(Math.sqrt((graphWidth * graphHeight) / cellsDesired), 2);
  const boundary = getBoundaryPoints(graphWidth, graphHeight, spacing);
  const points = getJitteredGrid(graphWidth, graphHeight, spacing);
  const cellCountX = Math.floor((graphWidth + 0.5 * spacing - 1e-10) / spacing);
  const cellCountY = Math.floor((graphHeight + 0.5 * spacing - 1e-10) / spacing);

  return { spacing, cellsDesired, boundary, points, cellsX: cellCountX, cellsY: cellCountY };
};

// --- Main.
(() => {
  const W = 2000, H = 2000, CELLS = 10000, SEED = "861039636";

  // Monkey-patchear Math.random con Alea(seed) _antes_ de getJitteredGrid.
  // Baagøe: `new Alea(seed)` retorna directamente una función callable.
  // NOTA: NO debemos consumir el stream de Alea entre el `new Alea(seed)` y el primer
  //   `Math.random()` (que ocurre dentro de `getJitteredGrid`). Si lo hiciéramos, el primer
  //   float de jitter sería el segundo del stream y los puntos divergirían.
  //   El sanity del primer float se hace en un Alea separado abajo para no contaminar.
  const random = new Alea(SEED);
  Math.random = random;

  // Sanity (Alea aparte, para no contaminar el stream): valida la implementación.
  const sanityAlea = new Alea(SEED);
  const firstFloat = sanityAlea();
  console.log("[sanity] first Alea float:", firstFloat, "(debe ser 0.7971209338866174)");

  const g = placePoints(W, H, CELLS);
  console.log("[done] points:", g.points.length, "boundary:", g.boundary.length, "spacing:", g.spacing, "cells:", g.cellsX + "x" + g.cellsY);

  // Delaunator JS sobre `allPoints = points.concat(boundary)` — exactamente como lo hace Azgaar
  // en `calculateVoronoi(graph, points, boundary)` (voronoi.ts).
  const allPoints = g.points.concat(g.boundary);
  console.log("[delaunay] triangulating", allPoints.length, "points...");

  const delaunay = Delaunator.from(allPoints);
  console.log("[delaunay] triangles:", delaunay.triangles.length, "halfedges:", delaunay.halfedges.length, "hull:", delaunay.hull.length);

  // --- Serializar a bits. Usamos `BigUint64Array` para u64 (Float64 / Uint32) y `BigInt64Array`
  //   para i64 (Int32 con -1 = EMPTY = 4294967295 como Uint32 sin signo). La serialización a
  //   JSON es strings decimalizadas (BigInt no es nativamente JSON-serializable).
  const F64_B = BigInt64Array; // f64 -> BigInt64Array vía Float64Array.buffer.
  const U32_B = BigUint64Array; // u32 -> BigUint64Array (cada u32 ocupa 8 bytes little-endian; nos quedamos con el bajo).

  // points bits (FlatPointArray, 10200 * 2 = 20400 f64).
  const f64points = new Float64Array(g.points.flat());
  const pointsBitsView = new BigInt64Array(f64points.buffer);
  const pointsBits = Array.from(pointsBitsView, (b) => b.toString(10));

  // boundary bits (200 * 2 = 400 f64).
  const f64boundary = new Float64Array(g.boundary.flat());
  const boundaryBitsView = new BigInt64Array(f64boundary.buffer);
  const boundaryBits = Array.from(boundaryBitsView, (b) => b.toString(10));

  // all_points bits (10200 * 2 = 20400 f64) — los puntos en el orden en que se le pasan a Delaunator:
  //   `points.concat(boundary)` (igual que `calculateVoronoi` de Azgaar).
  const f64allPoints = new Float64Array(allPoints.flat());
  const allPointsBitsView = new BigInt64Array(f64allPoints.buffer);
  const allPointsBits = Array.from(allPointsBitsView, (b) => b.toString(10));

  // triangles bits (Uint32Array → BigUint64View agrupa de a dos u32 en un u64 little-endian;
  //   para preservar cada valor por separado es mejor hacer loops explícitos).
  // Cada Uint32Array.entry es 0..2^32-1. Lo guardamos como strings decimalizados de u32 (plain JS number).
  const triangles = Array.from(delaunay.triangles, (t) => t.toString(10)); // strings decimal de u32 sin pérdida.
  const halfedges = Array.from(delaunay.halfedges, (h) => (h | 0).toString(10)); // Int32 (-1 = EMPTY).

  const out = {
    description: "Fixture self-reference: placePoints(2000,2000,10000,seed=861039636) + delaunator@5.1.0(JS) Triangulation. Test Rust debe reproducir points/boundary bit-exacto y producir triangles/halfedges idénticos con el crate `delaunator = \"1.1\"`.",
    seed: SEED,
    width: W,
    height: H,
    cellsDesired: CELLS,
    spacing: g.spacing,
    cellsX: g.cellsX,
    cellsY: g.cellsY,
    nPoints: g.points.length,
    nBoundary: g.boundary.length,
    nAllPoints: allPoints.length,
    nTriangles: delaunay.triangles.length / 3,
    points_bits: pointsBits,         // 20000 strings (10000 points × 2 f64 bits).
    boundary_bits: boundaryBits,     // 400 strings (200 boundary × 2 f64 bits).
    all_points_bits: allPointsBits,   // 20400 strings (10200 allPoints × 2 f64 bits) — input a Delaunator.
    triangles: triangles,            // N*3 strings (u32 decimal).
    halfedges: halfedges,            // N*3 strings (i32 decimal, -1 = EMPTY).
  };

  const outPath = path.join(__dirname, "delaunay_grid_2000x2000_c10k_seed_861039636_selfref.json");
  fs.writeFileSync(outPath, JSON.stringify(out, null, 2));
  console.log("[wrote]", outPath, `(${(fs.statSync(outPath).size / 1024).toFixed(1)} KB)`);
})();
