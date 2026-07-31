---
name: "🐛 Bug report"
about: "Report a bug in the Voronia engine"
title: "bug: [crate or module] brief description"
labels: ["bug"]
assignees: []
---

<!-- Before opening: search for an existing similar issue and read the master plan (voronia-plan-proyecto.md §22) and docs/phases/phase-*.md -->

## Description

What does the bug do? (one or two sentences)

## Steps to Reproduce

1. Open/import: `...` (which map? if it is an Azgaar `.map`, attach it or state the name + seed)
2. Click on / run: `...`
3. Observe: `...`

## Expected Behavior

What should happen?

## Actual Behavior

What actually happens? Attach a screenshot if possible (especially for rendering bugs).

## Context

- **Crate / module**: e.g. `vor-import`, `vor-render/src/coastline.rs`
- **OS**: Linux/macOS/Windows
- **GPU** (if it is a rendering bug): e.g. Intel UHD / NVIDIA 3050
- **Commit**: `git log -1` or version
- **Command used**: e.g. `cargo run -p vor-app -- path/to/map.map`

## Additional Information

<!-- Logs, stack traces, anything that helps. -->

## Checklist

- [ ] I could reproduce it always / sometimes (explain)
- [ ] The same map and seed in Azgaar does not show the problem (if applicable)
