# Keyboard zoom and the `0` toggle constrain the new pan against the OLD zoom — center-anchoring breaks near image edges

**Severity:** medium-high — the “viewport-center preserved” promise breaks by hundreds of pixels
**Type:** CODE bug
**Where:** `src/components/image_viewer.rs:402-431` (`zoom_in`/`zoom_out`: `adjust_pan_for_zoom` called **before** `self.image_state.zoom = new_zoom`), `:523-538` (`reset_zoom`, same order), `:435-457` (`adjust_pan_for_zoom` ends with `constrain_pan`), `:579-613` (`constrain_pan` computes bounds from `self.image_state.zoom` — still the old value).  Contrast: `zoom_toward_point` (`:639-640`) assigns the new zoom **before** constraining — correct.
**Verified:** all four functions read directly; arithmetic checked with a worked example

## Description

`constrain_pan` derives the allowed pan range from `zoomed_width = eff_w * self.image_state.zoom`.  The keyboard-zoom paths compute the anchoring pan for the **new** zoom but clamp it against limits belonging to the **old** zoom, because the zoom field is updated after the pan adjustment.

Worked example: 4000×3000 image, 1000×800 viewport.  At fit (zoom 0.25, pan ≈ (0, 25)), press `0` to go to 100%: the anchor pixel at the viewport center is image x = 2000, so the correct new pan_x is 500 − 2000·1.0 = −1500.  But `constrain_pan` runs with zoom 0.25 → zoomed_width 1000 → min_pan_x = −950 → pan clamps to −950.  The viewport shows image x ∈ [950, 1950] instead of [1500, 2500] — off by 550 px.  The same stale clamp makes `+` zoom-in jump when panned near an image edge.

## Reproduction (Rust test)

```rust
#[test]
fn reset_zoom_keeps_viewport_center_pixel() {
    let mut v = ImageViewer::new(fh);
    v.update_viewport_size(size(px(1000.0), px(800.0)));
    /* install 4000x3000 image */
    v.fit_to_window();                       // zoom 0.25, pan (0, 25)
    v.reset_zoom();                          // toggle to 100%
    assert!((v.image_state.pan.0 - (-1500.0)).abs() < 1.0);  // FAILS today: -950
}
```

## Suggested Fix

Reorder in `zoom_in`, `zoom_out`, and `reset_zoom`: set `self.image_state.zoom = new_zoom` (and `is_fit_to_window = false`) **before** calling `adjust_pan_for_zoom(…, old_zoom, new_zoom)` — exactly the order `zoom_toward_point` already uses.  `adjust_pan_for_zoom` takes old/new zoom explicitly, so its math is unaffected; only the `constrain_pan` call inside it starts seeing the correct new zoom.

## Why This Fix

The constraint is meant to bound the pan for the state being entered, not the state being left; matching `zoom_toward_point`’s proven ordering makes all zoom paths consistent and the test’s anchor arithmetic exact.
