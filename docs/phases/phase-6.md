# Phase 6 — Visualization overhaul: feature-based landmass, smoothed rivers, splines

> Creation date: Jul 28, 2026
> Closing date: (in progress)
> Status: in progress

## 1. Reference

N/A — native visualization work, no Azgaar code involved.

## 2. Commit chronology

| Hash | Date | Title | What it did |
|---|---|---|---|
| `f39c085` | Jul 28, 2026 | feature-based landmass mesh with Catmull-Rom coastlines | Changes the heightmap from a regular grid to a feature-based mesh: for each coastline/land feature, builds a polygon with Catmull-Rom. Height is taken from the underlying grid. Problem: islands end up with holes. |
| `ed8aef7` | Jul 28, 2026 | smooth lake and river rendering with Catmull-Rom splines | Lake/lagoon borders with Catmull-Rom smoothing in lake.rs. Rivers with Catmull-Rom in river.rs. |
| `67a44d0` | Jul 28, 2026 | increase Catmull-Rom subdivisions (rivers 12, lakes 5) | More subdivisions for smoother curves. |
| `a47f468` | Jul 28, 2026 | extend rivers into ocean, increase subdivisions (rivers 20, lakes 6) | First attempt at mouth extension: extends the river path from the last pack cell toward the position of mouth_cell (grid). |
| `80fdf32` | Jul 28, 2026 | proportional river mouth extension (0.3x seg_len), 30 subdivisions | Extension proportional to the length of the last segment. |
| `0ee10b8` | Jul 28, 2026 | simplify river path (RDP) then Catmull-Rom, find real mouth in pack | Ramer-Douglas-Peucker to simplify the path before the spline. Tries to convert mouth_cell from grid to pack via grid_id. |
| `d7d52ce` | Jul 28, 2026 | revert river to simple Catmull-Rom (4 subdiv), no RDP or mouth extension | Revert: RDP removed natural curves, mouth extension did not work correctly. Returns to simple spline. |
| `c0c75a4` | Jul 28, 2026 | fix: remove unused Pack import from river.rs, clean up | Dead import cleanup. |
| `488accc` | Jul 28, 2026 | rivers: use lyon StrokeTessellator with round caps/joins | Switches from manual quads to StrokeTessellator for better rendering of ends and corners. |
| `027284f` | Jul 28, 2026 | fix: call end(false) on open river path before build | lyon crash: the path builder requires `end(true/false)` before `build()`. |

## 3. Architecture of the code produced

### crates/vor-render/src/river.rs

- `build_river_mesh(points, rivers) -> HeightmapMesh`: iterates rivers, filters paths <2 pts, Catmull-Rom with 4 subdivisions, StrokeTessellator with thickness = `discharge_m3s / 3000.0` clamp(0.8, 5.0), fixed blue color.

### crates/vor-render/src/lake.rs (not touched in this session but related)

- Uses FillTessellator for closed polygons, Catmull-Rom to smooth contours if it has enough points.

### crates/vor-render/src/heightmap.rs - Feature-based landmass

- `build_mesh` with feature-iteration: for each Landmass/Island feature, triangulates the Voronoi vertices of the feature's cells. Height is obtained by sampling the grid (large triangle → ~5 sample points per edge).
- Cell height: uses `grid.height` with bilinear or nearest-neighbor sampling.
- Coastline feature height: if any triangle sample falls on the sea (grid.height=0), it assigns the landscape color, not the coast color.

### crates/vor-render/src/mesh.rs

- `catmull_rom_open(points, subdivisions) -> Vec<[f32; 2]>`: open Catmull-Rom spline (does not close the loop). 4 subdivisions default.

## 4. Critical findings and decisions

### Feature-based landmass: coast sampling

The original heightmap used individual cells → irregular coastlines. When switching to features with Catmull-Rom, the coastlines become smooth but the large Voronoi triangles cross sea areas. Solution: sample ~5 evenly spaced points on each triangle edge; if any falls on the sea (grid cell with height=0), that triangle is painted as landscape. This gives the illusion of a smooth coastline without changing the mesh topology.

### StrokeTessellator for rivers

Replaces the manual quads (which had culling and orientation bugs, and looked cut off at the ends) with lyon's `StrokeTessellator`. This automatically provides round caps and smooth joins. Thickness varies by discharge. Fixed blue color (0.15, 0.45, 0.85).

### `builder.end()` mandatory in lyon

lyon's path builder requires `end(true/false)` before `build()`. For open paths (rivers), `end(false)` is used. If omitted, lyon crashes with "build() called before end()". Closed paths (lakes, features) use `end(true)` or a fill tessellator.

## 5. Known bug: river mouths

**Symptom**: rivers do not reach the ocean. They end at the last land cell (coastline) and do not extend into the sea.

**Root cause**: in `trace_river_paths()` (loader.rs:318):
```rust
let mouth = river.mouth_cell as usize;
```
`mouth_cell` is in **GRID** space (index in `grid.cells`, 0..N_grid). But `adjacency` is in **PACK** space (post-reGraph). The path follows pack cells with `river_id` decreasing in height, but `mouth` is a grid id that does not exist in pack. The `current == mouth` condition is never satisfied. The loop ends when no more pack neighbors with that `river_id` are found — at the last land cell on the coastline. The mouth cell (which would be the first sea cell) is not in pack because reGraph discards sea cells except coastal ones with height >0.

**Previous fix attempts**:
- `a47f468`: Extension in the renderer from the last point toward the mouth_cell position. Worked partially, but the extension length was fixed, not proportional to the terrain.
- `80fdf32`: Proportional extension (0.3x the length of the last segment). Better, but the direction sometimes pointed wrong.
- `0ee10b8`: mouth_cell grid→pack conversion via grid_id lookup. Fails because `mouth_cell` is a grid cell that does NOT have a grid_id in pack (it is sea, it did not go through reGraph).

**Planned fix**: in `trace_river_paths`, when reaching the dead-end (last pack cell with river_id), look at the original `mouth_cell` (grid) and if the grid cell has `height=0` (it is sea), extend the path up to the mouth. Or, more robustly: in the renderer, take the last path point + the position of the mouth cell (in grid space) and extend the spline in that direction 30% of the distance to the mouth.

## 6. Test inventory

| File | Tests | What it validates |
|---|---|---|
| `river.rs` | (in HeightmapMesh, indirectly) | River rendering, thickness, Catmull-Rom |
| `mesh.rs` | `test_catmull_rom_open_basic` | Catmull-Rom produces points, does not choke on short paths |

No new render unit tests were added in this session (visual validation).

## 7. Final working tree state

Clean working tree at the end of the session (commit `027284f`). Compiles with `cargo build --workspace`. Green tests.
