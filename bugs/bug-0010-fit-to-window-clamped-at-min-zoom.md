# Fit-to-window is clamped at the 10% zoom floor — images more than 10× the viewport don’t actually fit at “Fit”

**Severity:** low
**Type:** NEEDS-DECISION — deliberate, tested code vs an undocumented spec gap; Chris must rule whether fit may go below MIN_ZOOM
**Where:** `src/utils/zoom.rs:32-47` (`calculate_fit_to_window` ends with `clamp_zoom(...)`; the clamp is pinned by `test_calculate_fit_to_window_very_large_image`); `DESIGN.md:118-119` (documents both “Zoom range: 10% – 2,000%” and fit-to-window, without noting they conflict)
**Verified:** function and its tests read; behavior is deliberate (explicitly tested), but the interaction is documented nowhere

## Description

`calculate_fit_to_window` clamps its result to `MIN_ZOOM = 0.1`.  An image more than 10× the viewport in either dimension therefore overflows the viewport in “Fit” mode: a 10,000-px-wide photo in an 800-px window needs zoom 0.08 but gets 0.1.  With `max_image_dimension` defaulting to 17,000 px, images that trigger this load without any oversize warning.  The zoom indicator shows “Fit (10%)” while the image visibly doesn’t fit.

## Reproduction

`calculate_fit_to_window(10_000, 10_000, 800.0, 600.0)` returns `0.1` (needs 0.06).  Manual: open any panorama > 10× the window width, note “Fit” with horizontal overflow.

## Resolution options (Chris to pick)

- **A:** let fit-to-window bypass the floor (`width_ratio.min(height_ratio)` un-clamped below, still clamped above at MAX) — “Fit” always fits; manual zoom-out still stops at 10% (or at fit, whichever is smaller).
- **B:** keep the clamp, document it in DESIGN.md (“Fit is bounded by the 10% floor; extremely large images overflow at Fit”), and optionally show a hint in the indicator.

Recommendation: A — “fit” failing to fit contradicts the feature’s name; the floor exists to stop runaway zoom-out, which fit-to-window is not.

## Why This Fix

Either option removes the silent contradiction between the two documented behaviors; A does it by making the invariant “Fit fits” unconditionally true, B by making the exception explicit.
