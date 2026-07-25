# Contributing to Voronia

Thanks for your interest in contributing. Voronia is a single-maintainer project right now, so the bar is small, but a few conventions keep things clean.

## Before you start

- Read [`voronia-plan-proyecto.md`](voronia-plan-proyecto.md) (Spanish, the master plan) for architecture, data model, and phase roadmap.
- Read [`docs/fase-0-investigacion.md`](docs/fase-0-investigacion.md) — it's the authoritative reference for how Azgaar works internally (PRNG, Delaunay/Voronoi, repack, `.map` format) and informs most Phase 1+ decisions.
- The repository is currently a one-person effort; expect rough edges in `CONTRIBUTING.md` itself as things stabilize.

## Architecture rules (do not break)

1. `vor-render` **never** mutates the World Data Model (`vor-core`). It only reads. If you need to write something while rendering, that logic belongs in `vor-edit`, not `vor-render`.
2. `vor-render` **never** depends on `vor-import`. If a render path seems to need a piece of import logic, that logic belongs in `vor-core` or `vor-import` exposing it cleanly.
3. Nothing depends "upward" from `vor-app` — `vor-app` is the orchestration layer, not the foundation.
4. **Everything procedural is deterministic.** Same seed + same parameters = same result, always. If you write a generator without an explicit seeded RNG, it's wrong.
5. **Structure-of-Arrays** for cell attributes (`Vec<u8>`, `Vec<u16>`, ...), not array-of-structs. This mirrors Azgaar and matters for cache locality on large maps.
6. Errors: `thiserror` for library error types inside each crate, `anyhow` in the binaries (`vor-app`, `vor-cli`).
7. Logging: `tracing` structured logs, never `println!`/`eprintln!`.
8. Tests with a fixed seed for any generator — same seed must give byte-identical output across runs.

## Bit-exactness against Azgaar (Phase 1+ only)

Any code that touches geometry regeneration (`getJitteredGrid`, `Delaunator.from`, the `Voronoi` class, `reGraph`, or anything derived from these) MUST produce output bit-identical to Azgaar's TypeScript/JavaScript for the same seed and parameters. Tests for those modules must validate against fixtures produced by Azgaar itself, not by Voronia's own generators — otherwise the test is circular.

If you find a divergence and Azgaar's output looks "wrong" (e.g. integer truncation of circumcenters via `Math.floor` — yes, this is real), still match it. Any Voronia-specific divergence silently corrupts attribute→cell mapping on imported `.map` files.

## Commit & PR style

- Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`, `perf:`). Scope optional but encouraged (`feat(vor-import): ...`).
- No emoji unless it actually carries meaning.
- Squash-merge on PR by default; rebase if you have a strong reason.
- Don't push force to `main`.
- Sign-off not required; the maintainer is currently the sole author.

## Tests

- `cargo test` at the workspace root should pass on every commit.
- `cargo build --workspace` should pass on every commit.
- `cargo clippy --workspace -- -D warnings` is a stretch goal; aim for it but don't block trivial PRs on it.

## Code style

- Follow `rustfmt` defaults. Don't reformat the whole file you're touching — only the lines you edited.
- No emojis in code or comments unless explicitly requested.
- Comments in English inside source files. Spanish is reserved for the plan and docs that are user-facing.
- Avoid `unwrap()` in library code unless the unwrap is provably impossible AND documented; prefer `expect("...")` with an explanation if you must.

## Reporting issues

Open an issue at https://github.com/hanserlodev/voronia/issues. Include:
- Rust version, OS, GPU.
- If the bug is in `.map` import: the seed, the size, the version of Azgaar that produced the file, and ideally a minimal `.map` that reproduces.
- If the bug is in rendering: a screenshot, the active layers, and camera position if known.

## License

By contributing you agree that your contributions are licensed MIT under the same terms as the rest of the project.
