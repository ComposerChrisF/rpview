# Z-drag zoom writes pan directly, bypassing `constrain_pan` — the image can be zoomed completely off-screen

**Severity:** medium — violates the app’s own “never lose the image off-screen” invariant
**Type:** CODE bug
**Where:** `src/app_render.rs:373-381` (Z-drag handler: `this.viewer.image_state.pan = (new_pan_x, new_pan_y);` — direct assignment); every other pan writer clamps (keyboard `viewer.pan()`, drag-pan, `zoom_toward_point`, `adjust_pan_for_zoom` all route through `constrain_pan`, `src/components/image_viewer.rs:579-613`)
**Verified:** handler read directly (quoted); constraint sites enumerated

## Description

Z+drag zoom anchors on the initial click position.  If that anchor is on empty background far from a small image, each zoom step scales the image away from the anchor; with no constraint the image leaves the viewport entirely and the min-visible-50 px guarantee of `constrain_pan` never applies.  Release the mouse and no part of the image is visible; recovery requires knowing to press `0`.

## Reproduction

Manual: open a small image (fit leaves large margins) → hold `Z`, press the mouse in a far corner of the empty background, drag to zoom in vigorously → image exits the viewport.

Test-shaped: simulate the handler’s math (or expose a `zoom_about_point` method): with viewport 1000×800, image 100×100 at center, anchor (10, 10), apply repeated zoom steps; assert `constrain_pan(pan) == pan` afterward (fails today — the resulting pan is outside the allowed range).

## Suggested Fix

Route the result through the existing constraint: `this.viewer.image_state.pan = this.viewer.constrain_pan_public(new_pan_x, new_pan_y);` — i.e. add a thin `pub(crate) fn set_pan_constrained(&mut self, x: f32, y: f32)` on `ImageViewer` (constraining with the **new** zoom already assigned, mirroring `zoom_toward_point`’s order) and use it here.

## Why This Fix

It applies the same invariant every other pan writer already honors, through the same function, so the Z-drag path can no longer produce states the rest of the app considers impossible.
