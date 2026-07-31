<!-- Thank you very much for contributing to Voronia. Complete this template to speed up the review. -->

## Summary

What does this PR do? (one or two sentences)

Related to: #issue (if applicable)

## Changes

- [ ] `vor-core` (World Data Model)
- [ ] `vor-import` (.map/JSON parser)
- [ ] `vor-format` (.vorn serialization)
- [ ] `vor-sim` (simulation engine)
- [ ] `vor-render` (wgpu pipeline)
- [ ] `vor-edit` (commands + undo/redo)
- [ ] `vor-app` / `vor-cli`
- [ ] Docs (`docs/`, `SECURITY.md`, `.github/`)

## Verification

<!-- Check whatever applies. EVERYTHING must be green before the merge. -->

- [ ] `cargo test --workspace` green
- [ ] `cargo clippy --workspace` no new warnings
- [ ] `cargo fmt --check` clean
- [ ] Tests with a **fixed seed** if I touched generative code (same seed = byte-identical output)
- [ ] I did not add `println!` (use `tracing`)
- [ ] `vor-render` does not write to the World Data Model (read-only)
- [ ] No secrets or keys in the diff
- [ ] I updated `docs/` and `.opencode/skills/voronia-dev/references/status.md` if applicable

## Screenshots / evidence

<!-- For visual rendering changes, include a before/after screenshot or a comparison with Azgaar. -->

## Notes for the reviewer

<!-- Architectural decisions, divergences from Azgaar, manual testing steps. -->
