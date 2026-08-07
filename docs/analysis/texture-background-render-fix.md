# Texture Layer Render Fix — background paper instead of post-filter

> **Date**: 2026-08-06
> **Scope**: `vor-render` + `vor-app` — texture layer render order
> **Target**: Azgaar FMG parity (texture reads as a *drawing* on the map, not a filter)

---

## Problem

In Azgaar FMP, toggling the texture layer produces a **paper/parchment background**
that the map is drawn on. In previous Voronia code the same toggle looked like a
*post-filter overlay* that darkened everything uniformly (land, ocean, borders)
almost exactly.

### Root cause
`TextureOverlay` was rendered as **Pass 1.5** — after Pass 1 had already resolved
the whole map (land, ocean, lines, text) to the surface — using a **`multiply`
blend**. `multiply` composites `src * dst`, so the texture darkened the existing
pixels everywhere rather than sitting underneath them. That is the classic "filter"
look: it tints the final frame rather than being a base the map is drawn on.

There was a **second, subtler root cause**: even after moving the quad to the
start of Pass 1, the quad was expressed in **clip space** (fixed to the screen,
ignoring the camera). When the user panned/zoomed, the paper stayed glued to the
viewport while the world geometry moved underneath — so it still read as a
"filter superimposed on the map". Azgaar's `#texture` is an `<image>` living
*inside* the world `#viewbox`, so it is **world-anchored** and moves with the map.

Azgaar instead:
- Inserts the `#texture` SVG group **before** `#landmass` in the z-order
  (`load.ts` `.insert("g", "#landmass")`), so it sits at the **bottom**.
- Draws the **ocean pages translucent** (`ocean-layers.ts` low `fill-opacity`) so
  the paper shows through the sea.
- Draws opaque continents above it.

## Fix

1. **`crates/vor-render/src/texture.rs`**
   - Quad rewritten to be **world-anchored**: vertices span the world rect and the
     vertex shader transforms them with the shared `camera` matrix (same uniform
     as the heightmap), so the paper pans/zooms *with* the map — no more
     screen-fixed "overlay filter".
   - Pipeline layout now includes the camera bind group (group 0) + texture
     (group 1); `draw()` takes the renderer's `camera_bind`.
   - Blend switched from `multiply` to opaque `BlendState::REPLACE`.
   - Sampler `Repeat` → `ClampToEdge` (slice-fit, matching FMG
     `preserveAspectRatio="xMidYMid slice"`).
   - Pipeline now MSAA-aware (`MultisampleState { count: msaa_count, .. }`) so it
     can render inside Pass 1 (4x MSAA target). `new()` takes `msaa_count`,
     `world_min`/`world_max`, and the camera layout.

2. **`crates/vor-render/src/renderer.rs`**
   - Added `ocean_pipeline`: a copy of the heightmap pipeline but with
     `ALPHA_BLENDING`, so the world-sized ocean quad is now translucent instead of
     opaque. `draw_ocean()` uses it.

3. **`crates/vor-app/src/lib.rs`**
   - The texture quad is now drawn at the **start of Pass 1**, *before*
     `draw_ocean()` and the land layers (`if self.layer_flags.texture`).
   - **Removed** the Pass 1.5 overlay pass entirely.
   - Ocean alpha set to `[0.16, 0.35, 0.66, 0.55]` so the paper canvas shows
     through the sea.

## Result

- The texture is the **base** the whole map is drawn over (true background).
- The ocean is **translucent**, so the chosen texture (paper/parchment) shows
  through the water, as in FMG.
- Opaque continents/landfill layers still hide the paper under them.
- The texture never darkens the map; it reads as a drawing of the world on canvas.

## Files

| File | Change |
|---|---|
| `crates/vor-render/src/texture.rs` | REPLACE blend, ClampToEdge sampler, MSAA-aware pipeline, `msaa_count` param |
| `crates/vor-render/src/renderer.rs` | new `ocean_pipeline` (alpha-blend); `draw_ocean` uses it |
| `crates/vor-app/src/lib.rs` | texture at start of Pass 1; Pass 1.5 removed; ocean alpha 0.55; `load_texture` passes `msaa_count` |
| `docs/layers/landmass-layers.md` | updated Texture section (approach + current phase) |

## Caveats / follow-ups

- **X/Y texture shift** controls (Azugar `data-x`/`data-y`) not ported yet.
- **Ocean alpha** is hardcoded; could become a UI slider in the Style tab.
- Visual regression must be confirmed via `cargo run` on Sorvik (GUI-only; not
  verifiable in a headless test).