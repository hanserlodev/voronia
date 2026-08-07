# River mouth clipping at the coastline

> **Date**: post-render (Phase 8) session.
> **Reference source**: `azgaar/app/Fantasy-Map-Generator/src/generators/river-generator.ts`, `src/renderers/river-renderer.ts` (via `docs/analysis/azgaar-rivers.md`).
> **Related**: `docs/analysis/azgaar-rivers.md`, `docs/analysis/azgaar-landmass-lines-smoothing.md`.
> **Scope**: `vor-render/src/river.rs`, `vor-import/src/mapfile/loader.rs`, `vor-app/src/lib.rs`.

This document records how Voronia makes each river's **mouth end exactly at the coast line** instead of floating into the center of the (large) ocean cell. It is the record of the bug, the two failed approaches and the final fix — useful next time anyone touches `build_river_mesh`.

---

## The problem

Azgaar's `.map` serializes each river's `cells` path in slot `[32]` **including the final water cell** where the river pours into the sea (its `river_id` is `0`, so it is not rediscoverable by tracing). When a river reaches a water/lake cell, the flow should visibly stop where it meets the coastline — i.e. at the **edge shared with the water cell**, not at the water cell center (which for a big ocean cell can be many pixels offshore).

Voronia had two defects that made mouths look wrong:

1. **`trace_river_paths` dropped the mouth water cell.** It rebuilt each path by following `pack.cells.river` (searching ancestors from mouth), and since the final water cell has `river_id == 0` it could never be rediscovered — every river stopped one cell short, short of the sea.
2. **`build_river_mesh` drew through the water cell center.** Even when the mouth cell was present, the final segment ran to the center of that water cell, over-shooting the actual coast and visibly floating in the ocean. ~10–20 % of the 141 rivers of Sorvik showed this.

---

## Root cause analysis

I instrumented a regression test (`crates/vor-render/tests/river_mouth_diag.rs`) that loads the real Sorvik map and buckets each of the 141 rivers by which step of the mouth-geometry fails. Findings:

| Bucket | Rivers | Meaning |
|---|---|---|
| single_cell | 0 | path < 2 cells |
| no_transition | 15 | path never goes land→water (mouth cell is still `h>=20`; no coast to clip) — legitimate |
| no_shared_edge | 0 | every mouth cell has a Voronoi ring |
| no_intersection | 0 (after fix) | the cast ray crosses the ring polygon |
| clipped_ok | 126 | mouth lands on the coast ✓ |

The two numbered `no_*` buckets going to 0 is the whole point: **every river that actually reaches the sea now ends at the coast.**

Intermediate result that motivated the fix: an earlier algorithm that only intersected the **single Voronoi edge shared by the last land cell and the water cell** left 23 rivers with `no_intersection`, because the river's flow does not always cross that exact edge. Switching to the *whole ring polygon* of the water cell made the cast robust.

---

## The fix

### 1. Trust the serialized path (`vor-import/src/mapfile/loader.rs`)

`trace_river_paths` now **skips rivers whose `cell_path` is already populated**, trusting the `.map`'s ground truth (which includes the final water cell):

```rust
if !river.cell_path.is_empty() {
    continue;
}
```

### 2. Clip the mouth to the water cell's ring (`vor-render/src/river.rs`)

`build_river_mesh` now receives `&VoronoiVertices` and `&[bool]` `is_water`, and calls `clip_to_coast` for each river right after building its `raw` point list:

```rust
clip_to_coast(&mut raw, path, vertices, is_water);
```

`clip_to_coast`:

1. Finds the **last land→water transition** in the path (`li`, `wi`).
2. Takes `lpt` (center of last land cell) and `wpt` (center of the mouth water cell).
3. Casts the ray `lpt → wpt` against **every edge of the water cell's Voronoi ring** (`vertices.cell_rings[water_cell]`).
4. Keeps the intersection **closest to `lpt`**.
5. Truncates the path at `wi` and push the hit point → the river ends exactly on the coast.

`is_water` is the same predicate used to draw the coastline (`h < 20 || feature is lake`), so the clip lands on the visible sea boundary.

### 3. Regression guard

`crates/vor-render/tests/river_mouth_diag.rs` (dev-dep on `vor-import`) hits the real Sorvik `.map` and asserts:

```rust
assert_eq!(no_intersection, 0);
assert_eq!(no_shared_edge, 0);
assert_eq!(out_of_points, 0);
assert!(clipped_ok >= rivers.len() - 15);
```

This guarantees that adding/regressing this logic re-surfaces any drop in coverage immediately.

---

## Non-goals / caveats

- The 15 rivers that end on **land** (`h ≳ 20`) have no mouth to clip — they are excluded by `no_transition` and are expected.
- This only fixes the **rendering** seam. The underlying `cells.fl`, discharge and width data were already correct; we never modified river *data*, only the drawn endpoint.
- The clip lands on the **post-resampled Voronoi ring**, not the azimuth fractalized coastline stroke. For a coastal scene they coincide visually; if the coastline fractalizer drifts further from the raw cell ring in the future, revisit by clipping against the fractalized feature perimeter instead.

---

## Verification

```
cargo test -p vor-render          # 27 passed (incl. river_mouth_diag)
cargo test                         # workspace green
cargo clippy -p vor-render -p vor-app   # no new warnings (2 pre-existing in vor-sim/hydrology.rs)
```

Manual check: load Sorvik, confirm the 126 coast-reaching rivers now belly up to the shoreline and none float in the open ocean.