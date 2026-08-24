# Incremental zoom (Shift+Cmd/Ctrl + ±) is not viewport-center anchored, unlike every other zoom speed

**Severity:** low — inconsistent anchoring; image drifts toward top-left during 1% zooming
**Type:** CODE bug (consistency; DESIGN.md’s table asserts centering only for `+`/`-`, so possibly intentional — flag for a quick ruling before fixing)
**Where:** `src/app_handlers.rs:1331-1363` (`handle_zoom_in_incremental` / `handle_zoom_out_incremental` mutate `image_state.zoom` directly — no `adjust_pan_for_zoom`); contrast `image_viewer.rs:402-431` (`zoom_in`/`zoom_out` anchor on the viewport center)
**Verified:** both handlers read directly (quoted in review)

## Description

The 1.2×, 1.5×, and 1.05× zoom speeds all keep the image pixel at the viewport center fixed.  The 1% incremental speed changes `zoom` without adjusting `pan`, which anchors the image’s top-left corner instead — during repeated incremental zooming the content visibly drifts.  No comment explains the divergence.

## Reproduction

Manual: zoom into an off-center detail, then press `Shift+Cmd+=` repeatedly — the detail migrates toward the bottom-right (image top-left is pinned).  Test: after `handle_zoom_in_incremental`, assert the image pixel previously at the viewport center is still there (same assertion style as the stale-zoom-constraint bug’s test) — fails today.

## Suggested Fix

Route the incremental handlers through the same anchored path: replace the direct mutation with `v.zoom_in(1.0 + ZOOM_STEP_INCREMENTAL)`-style additive variant — cleanest is a small `ImageViewer::zoom_to(new_zoom)` that does the `adjust_pan_for_zoom` + assignment (in the fixed order per `keyboard-zoom-constrains-pan-with-stale-zoom`), used by both incremental handlers.

## Why This Fix

One anchored implementation serves all four speeds, eliminating the odd-one-out; if Chris rules the top-left anchoring was intentional for pixel-precise work, the fix is instead a code comment and a DESIGN.md note saying so.
