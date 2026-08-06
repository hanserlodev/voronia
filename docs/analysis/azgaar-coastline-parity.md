# Coastline parity with Azgaar — debugging process

> **Date**: post-Phase 7 (render) session.
> **Reference source**: `azgaar/app/Fantasy-Map-Generator/src/renderers/coastline-fractal.ts` (and `landmass/landmass.ts`).
> **Related**: `docs/analysis/azgaar-landmass-lines-smoothing.md`, `docs/plans/landmass-lines-points-1-5.md`, `docs/analysis/landmass-drawing.md`.

This document records the process of making Voronia's coastline rendering reproduce Azgaar's coastline geometry **bit-exact** (same seed = same coastline). It is not a user guide nor a replacement for the plan; it is the record of the bugs found, why they occurred and what was done to fix them.

---

## Why exact parity

Azgaar's `.map`/JSON **doesn't store the geometry** — only per-cell attributes. When loading a map, Azgaar recalculates the full mesh with its PRNG and its seed. If Voronia generates a slightly different mesh (even subtly different), the imported map's attributes end up placed on the wrong cells: **silent incorrect data, with no visible errors**.

Parity is required at two levels:

1. **Mesh / grid** (grid → pack → Delaunay/Voronoi): already solved in Phases 1–6 (see `docs/phases/phase-{1..6}.md`).
2. **Coastline rendering**: the fractal tracing of the coastline over the perimeter of each land feature. That's what this session covers.

## Implemented coastline pipeline

Azgaar, for each land feature, chains exactly this:

```
simplify(points, 0.3)                 → simplify-js (radial distance + RDP)
→ clipPoly(points, W, H, secure=1)   → Sutherland-Hodgman
→ fractalize(points, "seed_c{i}")     → Alea PRNG, roughness profile + subdivisión
→ buildCoastlinePath(...)             → Catmull-Rom centrípeta + B-spline midpoint
→ (lyon tessellation)                 → relleno EvenOdd
```

The equivalent in `vor-render` ended up distributed as follows:

| Azgaar step | Voronia module |
|---|---|
| `simplify` (simplify-js) | `vor-render/src/simplify.rs` |
| `clipPoly` (Sutherland-Hodgman, secure) | `vor-render/src/clip_poly.rs` |
| `fractalize` + `makeRoughnessProfile` + `subdivideEdge` | `vor-render/src/coastline.rs` |
| `buildCoastlinePath` (Catmull-Rom) | `vor-render/src/coastline_path.rs` |
| coastline stroke/shadow | `vor-render/src/coastline_stroke.rs` |
| isolines (get_isolines, connect_vertices, halos) | `vor-render/src/isoline.rs` |
| water mask (water gap) | `vor-render/src/water_gap.rs` |
| GPU text (glyphon) | `vor-render/src/text.rs` |
| PRNG `Alea@1.0.1` | `vor-render/src/prng/alea.rs` |

---

## PRNG: `Alea` (bit-exact)

Azgaar uses Johannes Baagøe's `alea` PRNG (npm `alea` 1.0.1) in almost everything generative, with string seeds. The `.map` stores the seeds in the header.

`Alea@1.0.1` was ported to `vor-render/src/prng/alea.rs` with these methods:

- `Alea::new(seed: &str)` — internal state `s0/s1/s2/c` initialized with the **mash** function (David Bau's algorithm, 32-bit).
- `next_f64() -> f64` — returns `(c() + s0()) * 2^-32` (an `f64` in `[0,1)`).
- `next_u32() -> u32`, `next_fract53() -> f64`.

**Verification**: the port was tested against the original source `vor-import/tests/reference/alea-1.0.1.original.js` (bit-exact). The port lives in `vor-render` (not in `vor-import`) because `vor-render` can't depend on `vor-import` due to the architecture rule — but the reference fixture remains in `vor-import/tests/reference/`.

---

## Bugs found and fixed

### Bug 1 — Wrong PRNG: `hash_f32` instead of `Alea`

**Symptom**: the coastlines were "similar but not identical" to Azgaar's. Same seed, same general shape, but the fractalization didn't match point by point.

**Cause**: `coastline.rs` used a hand-rolled hash (`hash64`/`hash_f32` based on multiplication by SplitMix64 constants) to generate pseudorandom numbers. Azgaar uses `Alea(format!("{}_c{}", seed, feature_index))`.

**Fix**: the hash was replaced with `Alea::new(&seed_str)`, and the roughness profile + edge subdivision now consume the **same** PRNG stream (`&mut dyn FnMut() -> f32`), as in the original JS where `rand` is a single shared reference.

### Bug 2 — Wrong seed: `map_id` instead of `header.seed`

**Symptom**: the coastlines didn't change when changing a map's seed (or changed in a way unrelated to the loaded map).

**Cause**: in `crates/vor-app/src/lib.rs` the seed was derived from `loaded.world.header.map_id.wrapping_add(2654435761)`. Azgaar uses the **`seed` field of the `.map` header** (string, e.g. `"123456"`), which is the seed with which the grid was generated — and therefore the one that lets the coastline match the geography.

**Fix**:
```rust
// antes
seed: loaded.world.header.map_id.wrapping_add(2654435761),
// después
seed: loaded.world.header.seed.parse::<u64>().unwrap_or(0),
```

**Note**: the `seed` field of the header is a numeric string; it's parsed into a `u64` and serialized as part of the seed string `"{seed}_c{featureIndex}"` that `Alea` consumes.

### Bug 3 — Next span index in `buildCoastlinePath`

**Symptom**: "smooth" spans (non-fractalized) drawn with rotated Catmull-Rom — curves that left the coastline, sharp spikes where there shouldn't be any.

**Cause**: in `coastline_path.rs`, inside the centripetal Catmull-Rom loop:

```rust
// ANTES (bug) — apuntaba al INICIO del siguiente span
let ni = spans[(i + 1) % m].end_idx;

// DESPUÉS (correcto) — apunta al FIN del span actual
let ni = spans[i].end_idx;
```

`spans[i].end_idx` is already the index of the next "original" point of the feature (the non-fractalized vertex), which is exactly the point where this span's interpolation ends. Using the next span mixed two contiguous spans and broke curve continuity.

**Fix side effect**: it also fixed the **midpoint B-spline** calculation of the smooth spans, which depended on the same index.

### Bug 4 — Hardcoded `roughness_contrast`

**Symptom**: the roughness profile didn't respect the `roughness_contrast` parameter of the header.

**Cause**: `make_roughness_profile` normalized with a fixed `.powf(1.5)`, ignoring the `roughnessContrast` that Azgaar reads from the header (default `1.5`).

**Fix**: the profile now receives the contrast as a parameter and applies `powf(contrast)`.

### Bug 5 — Shared PRNG stream (not discovered in this session, but relevant)

The roughness profile and edge subdivision share the **same** `Alea`. In the initial port two separate instances were created; that also broke parity (the displacements depend on the full sequence from the start). Now there is a single instance and a single `rand` closure.

### Bug 6 — `f32` arithmetic instead of `f64` (scale-dependent coastline divergence)

**Symptom**: small islands render **bit-identical** to Azgaar, but large landmasses show **considerable** coastline differences on the same seed/map.

**Cause**: JavaScript numbers are IEEE-754 `f64`. Azgaar's entire coastline pipeline — `simplify` (simplify-js), `clipPoly`, `fractalize`, `subdivideEdge` — is computed in `f64`. The initial Voronia port of those three modules (`vor-render/src/simplify.rs`, `clip_poly.rs`, `coastline.rs`) used `f32` throughout.

The failure only appears on large landmasses because of how the error accumulates:

- **Small islands** → short edges. The `subdivideEdge` stop criterion `len < min_edge` (1.0) cuts recursion almost immediately, so very few arithmetic steps run and the `f32` rounding error stays below the visible threshold.
- **Large landmasses** → long edges, recursion runs to `max_depth = 4`, and — the dominant effect — the **RDP decision in `simplify` is binary**: a midpoint is kept iff `max_sq_dist > sq_tolerance` (tolerance `0.3`). Squared distances on the order of `1e6` lose ~1e-3 of precision in `f32`. A point sitting *exactly* on the tolerance boundary in `f64` gets rounded to the *wrong side* of the decision in `f32`, flipping a point from **keep → discard**. Because simplification runs *before* fractalization, one kept/dropped point changes the polygon topology, cascading into a **considerably different** coastline for the entire feature.

The Voronoi vertex `positions` are **integers** (Azgaar `Math.floor`s each circumcenter — see `phase-0-research.md` §6.3), so casting them `f32 → f64` is **exact**; doing all internal arithmetic in `f64` then reproduces JS byte-for-byte, with `f32` only at the input/output boundary toward lyon.

**Fix**: internal arithmetic of `simplify`, `clip_poly` and `coastline` (fractalize/subdivide/profile) promoted to `f64`; public signatures keep taking/returning `f32` points, cast at the edge.

---

## Default parameters aligned with Azgaar

`FractalSettings::default()` now reflects the defaults of Azgaar's renderer:

| Parameter | Value | Source in Azgaar |
|---|---|---|
| `amplitude_decay` | `0.9` | slider default |
| `min_edge` | `1.0` | constant |
| `base_amplitude` | `1.5` | constant |
| `max_depth` | `4` | constant |
| `smooth_threshold` | `0.25` | constant |
| `roughness_contrast` | `1.5` | header `roughnessContrast` |
| `profile_harmonics` | `4` | constant |
| `lake_smooth_thresh_mult` | `2.0` | constant |
| `simplify_tolerance` | `0.3` | `simplify(pts, 0.3)` |
| `clip_secure` | `true` | `clipPoly(..., 1)` |

Details of the bit-exact ported algorithm:

- **Edge displacement**: `(rand() - 0.5) * sqrt(len) * amplitude * roughness` over the normal `(-dy/len, dx/len)` of the midpoint.
- **Roughness profile**: sum of `num_harmonics` cosines, each harmonic with `amp = rand()` and `phase = rand() * 2π`, normalized to `[0,1]` and raised to `roughness_contrast`. Fixed size 256. Sampled with linear interpolation using `t.rem_euclid(1.0)` (closed profile).
- **`mid_t`**: circular average of `t0/t1` (handles the wrap of the polygon closure).
- **Smooth threshold** (stop criterion): if `roughness(t_mid) < smooth_threshold`, don't subdivide (span stays smooth). For lakes, the threshold is multiplied by `lake_smooth_thresh_mult` (2.0).
- **Edges on the map border**: if both endpoints are on the border (`x<=0 || x>=W || y<=0 || y>=H`), they're not fractalized (avoids fractalizing the map border).
- **Spans**: one `CoastlineSpan` per original edge; `is_smooth` is `true` if the edge produced no new points (`num_points == 2`).

---

## `buildCoastlinePath` — Catmull-Rom + B-spline (point by point)

The path builder reproduces the JS `buildCoastlinePath`:

- If the span is **smooth**: the path traverses the edge with a **quadratic Bézier** from midpoint to midpoint (midpoint B-spline, `(a+b)/2`), or a `LineTo` to the midpoint when the previous span was jagged.
- If the span is **jagged** (fractalized): each pair of consecutive points is joined with a **cubic Bézier** using the previous and next neighbors with Catmull-Rom's `1/8` factor:
  ```
  cp1 = a + (b - prev) / 8
  cp2 = b - (nnext - a) / 8
  ```
- The start point is the midpoint of the last span if it's smooth, or `p0` if the last span is jagged (`at_mid`).
- `coastline_path_to_lyon` converts the `PathCommand`s into a `lyon::Path` for tessellation with `FillOptions::default().with_fill_rule(EvenOdd)`.

**Tests** (`coastline_path.rs`): `smooth_span_produces_quad_bezier`, `jagged_span_produces_cubic_bezier`, `start_point_jagged_last_span` — cover the two command emission cases and the start point.

---

## New modules from this batch

### `simplify.rs` — Ramer-Douglas-Peucker + radial distance
Port of `simplify-js` (Vladimir Agafonkin): first pass of radial distance with `sq_tolerance`, second of recursive RDP. Tolerance used: `0.3`. Public export: `simplify`.

### `clip_poly.rs` — Sutherland-Hodgman with "secure"
Port of Azgaar's `clipPoly`: clips the polygon to the map rectangle. With `secure=true` (1) it doesn't degenerate into rectangles/segments at the border — avoids tessellation artifacts when the feature touches the border. Public export: `clip_polygon`.

### `coastline_stroke.rs` — coastline stroke and shadow
Generates the contour (stroke) and the shadow of the coastlines (a thicker, darker line under the stroke), for Azgaar's look. Exports: `build_coastline_stroke_mesh`, `build_coastline_shadow_mesh`, `CoastlineStrokeSettings`.

### `isoline.rs` — isoline engine (connect_vertices)
Port of Azgaar's isoline engine (`connectVertices`, `getIsolines`, border/halo paths). Used for isoheight, isotherm, isobar, etc. Exports: `connect_vertices`, `get_isolines`, `get_border_path`, `get_fill_path`, `get_halo_path`, `get_water_gap_path`, `IsolineOptions`, `IsolineOutput`.

### `water_gap.rs` — water mask
So that the colors of human layers (states, provinces, cultures, religions, biomes) don't bleed into the ocean: generates a water gap over the water cells (`h < 20` or lake), painted with the ocean background color. `append_water_gap` mutates an existing mesh adding vertices/triangles; `build_water_gap_mesh` creates it from scratch.

### `text.rs` — TextSystem (glyphon)
GPU text system with `glyphon 0.6`: `FontSystem` + `SwashCache` + `TextAtlas` + `Viewport`. Two renderers (one MSAA for the map pass, one non-MSAA for debug). `prepare()` uploads glyphs outside the render pass; `render()` draws inside any pass; `render_debug_no_msaa()` on the resolved surface.

---

## Integration in vor-app

- `build_fractal_landmass_mesh` is called with `FractalSettings { seed: header.seed.parse::<u64>().unwrap_or(0), ..Default::default() }` (Bug 2).
- The human geography layers (states, provinces, cultures, religions) and biomes now carry `append_water_gap`, with the water color resolved per layer (catalog color for biomes, `hex_color_to_linear` for the rest).
- `TextSystem` is initialized in `init_state`, resized on resize, used inside the MSAA pass and `trim()`ed at the end of the frame.

## How it's verified

1. **Determinism**: same seed + same parameters = same mesh, always (tests with fixed seed).
2. **PRNG bit-exactness**: tests against the reference JS source (`alea-1.0.1.original.js`).
3. **Visual**: compare Voronia's render against the original Azgaar map with the same seed — coastlines and attributes must match.
4. **Tests**: `cargo test --workspace` green (99 tests, 1 ignored); `cargo test --package vor-render` (21 tests).

## Session checklist

- [x] Port of `Alea@1.0.1` to `vor-render/src/prng/alea.rs` + reference fixture
- [x] Replacement of the hash PRNG in `coastline.rs` with `Alea("seed_c{featureIndex}")`
- [x] Shared PRNG stream between profile and subdivision
- [x] `roughness_contrast` parameterized (Bug 4)
- [x] `simplify` + `clipPoly` in the pipeline (`simplify.rs`, `clip_poly.rs`)
- [x] Fix `ni = spans[i].end_idx` in `buildCoastlinePath` (Bug 3)
- [x] `FractalSettings` defaults aligned with Azgaar
- [x] Seed fix: `header.seed` instead of `map_id` (Bug 2)
- [x] `buildCoastlinePath` + `coastline_path_to_lyon` + 3 tests
- [x] Coastline stroke/shadow (`coastline_stroke.rs`)
- [x] Isoline engine (`isoline.rs`)
- [x] Water gap on human layers + biomes (`water_gap.rs`)
- [x] TextSystem glyphon (`text.rs`)
- [x] Tests green, clippy without new errors, clean fmt
