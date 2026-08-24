# GPU-pipeline resize can push zoom outside the documented 10%–2000% range (apparent-size preservation vs spec invariant)

**Severity:** low
**Type:** NEEDS-DECISION — code and spec disagree; which side is wrong is a design call (clamp vs document the exception)
**Where:** `src/components/image_viewer.rs:474-484` (`rescale_for_size_change`: `self.image_state.zoom *= scale;` — no `clamp_zoom`); `DESIGN.md:118` (“Zoom range: 10% – 2,000%”); every other zoom writer clamps via `utils::zoom::clamp_zoom`
**Verified:** function read directly (quoted in review); mechanism certain

## Description

`rescale_for_size_change` deliberately preserves apparent size when the effective pixel grid changes (GPU resize factor, frame-size transitions): `apparent = width × zoom` stays constant by scaling `zoom` inversely.  With resize 4× at 15% zoom the stored zoom becomes 3.75% (indicator shows “4%”); with 0.25× at 1500% it becomes 6000%.  Both violate the documented range that every other writer enforces, and subsequent zoom-out keys behave oddly at the out-of-range end (already at 3.75%, `-` clamps back up to 10%, jumping apparent size).

## Reproduction

Enable GPU resize 4× on any image, set zoom to minimum (10%) first, toggle resize — indicator reads below 10%.  Test-shaped: `rescale_for_size_change((1000,1000),(4000,4000))` with `zoom = 0.15` → assert on the chosen policy.

## Resolution options (Chris to pick)

- **A (document):** the invariant becomes “10%–2000% for user-initiated zoom; size transitions may exceed it to preserve apparent size”.  Fix DESIGN.md; optionally clamp only user zoom _steps_ from an out-of-range start so `-`/`+` don’t jump.
- **B (clamp):** apply `clamp_zoom` in `rescale_for_size_change`, accepting an apparent-size jump at the extremes.

Recommendation: A — the apparent-size preservation was built deliberately (v0.21.x series, commits `ac9f98d`/`bc7c197`) and jumping content on resize toggles is worse than a temporarily out-of-range percentage.

## Why This Fix

Either option removes the silent contradiction; A preserves the deliberate UX behavior and costs only a spec sentence plus a small step-clamp guard.
