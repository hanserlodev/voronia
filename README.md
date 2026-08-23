# Voronia

[![CI](https://github.com/hanserlodev/voronia/actions/workflows/ci.yml/badge.svg)](https://github.com/hanserlodev/voronia/actions/workflows/ci.yml)

*A native, GPU-accelerated engine for generating and editing fantasy worlds — written in Rust, rendered with wgpu.*

Voronia is a from-scratch reimplementation of the procedural pipeline behind [Azgaar's Fantasy Map Generator](https://github.com/Azgaar/fantasy-map-generator), built as a native desktop application instead of a web app. It imports existing `.map` files produced by Azgaar, regenerates the underlying geometry bit-for-bit so that every per-cell attribute lands on the exact right cell, and renders them at high framerates — on its way to reproducing and extending the full procedural pipeline natively.

## Status

Development version: **0.3.0**. See [`CHANGELOG.md`](CHANGELOG.md). The master plan — roadmap, data model, architectural decisions — lives in [`docs/plans/master-plan.md`](docs/plans/master-plan.md).

**Completed phases** (roadmap §22):

- **Phase 0 — Research & groundwork**: the full investigation of Azgaar's source (PRNG, Delaunay/Voronoi, grid→pack repacking, `.map` format, generation pipeline) lives in [`docs/phases/phase-0-research.md`](docs/phases/phase-0-research.md).
- **Phase 1 — Geometry regeneration + data parser**: bit-exact port of Azgaar's grid + Voronoi + repacking to Rust, plus the legacy `.map` parser → World Data Model. Validated handshake against a real Azgaar 1.138.0 map (`Sorvik`, committed as test fixture).
- **Phase 2 — Minimal GPU viewer**: winit + wgpu window, grid mesh, pan/zoom.
- **Phase 3 — Full render layers**: biomes, relief, rivers, states, burgs, labels, water gap, GPU text.
- **Phase 4 — `.vorn` format**: native binary serialization (bincode v1, versioned header) + autosave.
- **Phase 5 — Editing UI**: egui interface, layer toggles, entity inspector, basic attribute editing (rename, recolor), PNG/SVG export.

**In progress** — **Phase 7: native procedural generation**, tracked in detail in [`docs/plans/human-geography-parity.md`](docs/plans/human-geography-parity.md):

- **River hydrology (complete)** — native port of Azgaar's rivers in `vor-sim`: depression resolving, lakes, flux accumulation, width, meander, confluences and downcutting.
- **Bit-exact coastline rendering** — Azgaar's coastline pipeline (`simplify` → `clipPoly` → `fractalize` → `buildCoastlinePath`) reproduced point-exact, documented in [`docs/analysis/azgaar-coastline-parity.md`](docs/analysis/azgaar-coastline-parity.md).
- **Human Geography & Economy parity** — deterministic native generation of states, provinces, cultures and religions using FMG's exact expansion costs; typed data models + initial render for goods, markets and trade deals; region isolines with landmask stencil, population bars, burg icons and Catmull-Rom routes.

**Next up**: procedural names & labels (states/provinces/burgs), exact route generation, market/trade simulation with animation — then **Phase 6 — Advanced editing** (brushes, manual borders, river editor).

## Why

- **Native performance**: GPU-accelerated pan/zoom designed for maps of hundreds of thousands of cells (target: 100k cells at 60 FPS).
- **Bit-exact `.map` import**: regenerate Azgaar's grid + Voronoi topology deterministically from the seed embedded in a `.map` file, so all per-cell attributes line up with the correct geometry. No silent data loss.
- **Editing**: entity inspector, rename/recolor, layer toggles today — with undo/redo, brushes and river editing on the roadmap (Phase 6).
- **Beyond Azgaar**: no practical cell-count ceiling, optional spherical projection, batch/headless generation via CLI, and native procedural generation instead of a web app.

## Building & running

Requires a Rust toolchain (stable). The workspace builds with a single command:

```sh
cargo build --workspace
```

Run the desktop viewer, passing any Azgaar `.map` file (a reference one is committed as a test fixture):

```sh
cargo run -p vor-cli -- crates/vor-import/tests/reference/Sorvik-2026-07-24-23-39.map
```

Run the test suite (includes the bit-exact handshake tests against the real Azgaar map):

```sh
cargo test --workspace
```

## Architecture

```
crates/
├── vor-core/     World Data Model (pure data, no logic, no render)
├── vor-import/   Azgaar .map parser + bit-exact geometry regeneration
├── vor-format/   .vorn native serialization
├── vor-sim/      Procedural simulation engine (hydrology + states/provinces/cultures/religions)
├── vor-render/   wgpu pipeline, layers, camera (never mutates world data)
├── vor-edit/     Edit commands (the only other mutator of world data)
├── vor-app/      Desktop application: winit + egui + orchestration
└── vor-cli/      Headless tools + the `vor` binary
```

Hard rule: `vor-render` never depends on `vor-import`, and nothing depends upward from `vor-app`.

## Tech stack

Rust · wgpu · winit · egui · glam · lyon · glyphon · noise · petgraph · rand + rand_pcg · serde · bincode · rayon · tracing · anyhow/thiserror. The Delaunay/Voronoi geometry is a bit-exact port of `delaunator@5.1.0` and Azgaar's `Voronoi` class, not the upstream crates (bit-exactness requires an exact replica — see `docs/phases/phase-0-research.md`).

## Compatibility with Azgaar

Voronia imports the **legacy `.map` format** (the in-app binary text format, slot-by-slot) generated by Azgaar and reconstructs the exact geometry so that imported attributes (biome, culture, state, burg, ...) land on the right cells. The Full JSON export (`export-json.ts`) is **deferred** to a later phase. The reference Azgaar checkout used during Phase 0 was:

- Repo: https://github.com/Azgaar/fantasy-map-generator
- Commit: `51d8e3e487a28995aac2304af57ad1ac4fbe3789` (2026-07-21, version 1.138.0)

No line of Azgaar's source code is copied; only algorithms and data invariants are reproduced. The test fixture `Sorvik-2026-07-24-23-39.map` (Azgaar 1.138.0) lives in `crates/vor-import/tests/reference/` under the same MIT license.

## Credits and inspirations

Voronia would not exist without these prior works. All credit for the procedural generation concepts belongs to them; Voronia only ports and reorganizes the ideas into a native engine.

- **Azgaar (Max Haniyeu)** — *Fantasy Map Generator*, the direct inspiration and reference implementation for this project. https://github.com/Azgaar/fantasy-map-generator
- **Martin O'Leary** — *Generating fantasy maps*: https://mewo2.com/notes/terrain
- **Amit Patel** — *Polygonal Map Generation for Games*: http://www-cs-students.stanford.edu/~amitp/game-programming/polygon-map-generation
- **Scott Turner** — *Here Dragons Abound*: https://heredragonsabound.blogspot.com

## License

MIT — see [`LICENSE`](LICENSE).
