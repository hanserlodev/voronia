# Height Unit Conversion & Cell Hover Tooltip — Azgaar parity

> **Date**: 2026-08-07
> **Scope**: `vor-core` + `vor-app` — height display units and hover pick
> **Target**: Azgaar FMG parity (the cell tooltip / Info tab show real-world heights, not the raw generator scale)
> **Status**: ⚠️ **PARTIAL** — conversion ports the FMG formula but is not 100% verified against a live map.

---

## Problem

Azgaar stores cell height on a generator scale `[0, 100]` (`cells.height`, `Uint8Array`), where
`20` is the sea–land threshold. When the map is inspected (hover tooltip, Info panel), FMG does **not**
show that raw value: it converts it to a real-world height with units via `getHeight()` in `unitUtils.ts`:

```ts
// getHeight(h, abs?) → string
let unitRatio = 3.281;            // default calculations are in feet
if (unit === "m") unitRatio = 1;  // meter
else if (unit === "f") unitRatio = 0.5468; // fathom

let height = -990;
if (h >= 20) height = (h - 18) ** +heightExponentInput.value;   // land
else if (h < 20 && h > 0) height = ((h - 20) / h) * 50;          // sea (negative)

return `${rn(height * unitRatio)}${unit}`;
```

- **Land** (`h >= 20`): `(h - 18) ^ exponent`, default exponent **`1.8`** (`units-editor.ts:80`); the
  `.map` header can override it (Azgaar commonly exports `2`).
- **Sea** (`h < 20`, `h > 0`): `((h - 20) / h) * 50`, which is **negative** (depth below sea level).
- `rn()` = JS-ish `Math.round`.

So a raw `h = 20` displays as `(20 - 18)^2 = 4` meters, and a raw `h = 14` sea cell is negative — the
previous Voronia code showed the raw `h` directly (he heightmap used `Height: {h}m`, which printed
`20m` and positive sea values). This mismatch is what this change fixes.

---

## What changed

### `vor-core/src/settings.rs`
Added `Settings::height_m(h: u8) -> f32` inside `impl Settings`, a faithful Rust port of `getHeight`:

```rust
pub fn height_m(&self, h: u8) -> f32 {
    let unit_ratio = match self.height_unit.as_str() {
        "ft" | "foot" | "feet" => 3.28084,
        _ => 1.0,
    };
    let height = if h >= 20 {
        (f32::from(h) - 18.0).powf(self.height_exponent as f32)
    } else if h > 0 && h < 20 {
        ((f32::from(h) - 20.0) / f32::from(h)) * 50.0
    } else {
        -990.0
    };
    height * unit_ratio
}
```

- Reads the already-parsed `Settings.height_exponent` and `height_unit` (from the `.map` `[1]` slot).
- `height_unit` mapping is simplified: only `"m"` (ratio `1.0`) and feet (`"ft"`/`"foot"`/`"feet"`,
  ratio `3.28084`) are handled; all other/unknown units fall back to meters.

### `vor-app/src/lib.rs` (hover tooltip)
- New `State.hover_cell: Option<usize>`.
- Updated on every `CursorMoved` (when not panning): `screen_to_world` → `pick_cell`.
- Tooltip (Azgaar-style, bottom–center of the window) built outside the egui closure and painted with
  `ctx.debug_painter()`:
  `Cell #{cid}  ·  Height: {height_m:0}{height_unit}  ·  Biome: {biome}`.
- Geometry: width `280`, height `24`, `x = (surface_w - 280)/2`, `y = surface_h - 44`, rounded rect
  `(20, 22, 26, 220)`, 13 px proportional font.

### `vor-app/src/lib.rs` (Info tab, `ui/info.rs`)
- The picked-cell Info panel now shows `Height: {height_m:.0}{height_unit}` (converted) instead of the
  raw `h`.

---

## Status: partial

- ✅ Height conversion for **land** (`h >= 20`) matches the formula and the `height_exponent` from the
  imported `.map`.
- ✅ **Sea** cells display negative values (depths) instead of the raw `h`.
- ⚠️ **Not 100% verified**: unit mapping is limited to `m`/feet; Azgaar also supports fathom (`f`,
  ratio `0.5468`) and custom units — not yet wired. The `height_unit` string match assumes Azgaar's
  value tokens (`"m"`, `"ft"`, ...); custom names would fall through to meters.
- ⚠️ **Not visually confirmed** against a live Azgaar map side by side; the heightmap texture change
  and the `.map`-sourced `height_exponent` interplay should be eyeballed.
- ⚠️ Default exponent when the `.map` carries none: falls back to the parsed value (may be `0`), which
  would render `(h-18)^0 == 1` for all land — a sensible default (`1.8`/`2`) should be applied.

## Next steps

- Wire fathom / custom height units.
- Set a sane default `height_exponent` (e.g. `1.8`) when absent.
- Visual parity check: same cell on a live Azgar map → Voronia equal height text.
- Prettier tooltip positing option (currently fixed bottom-center) and Formatter reuse for areas.