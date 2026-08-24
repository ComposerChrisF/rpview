# SVG re-rasterization ignores the display scale factor — SVGs are soft on Retina above ~110% zoom

**Severity:** medium — contradicts the headline “always-crisp vector display” claim on Chris’s primary hardware
**Type:** CODE bug (DESIGN.md:22-23’s promise is the intended behavior)
**Where:** `src/components/image_viewer.rs:1459-1474` (sharpness threshold `current_zoom <= base_scale * 1.1`), `:1531-1545` (re-raster spawned at `scale = zoom`, logical px); `src/utils/svg.rs:77-114,122-195` (rasterizers take the logical scale); no file in the crate consults `window.scale_factor()` (grep: the only `scale_factor` hits are an unrelated parameter name in svg.rs)
**Verified:** grep for scale-factor usage; threshold and spawn sites read

## Description

The initial raster is rendered at 2× (`rerasterize_svg_full(&tree, 2.0)`) — exactly right for a 2× (Retina) display at zoom ≤ 100%.  But dynamic re-rasters render at `scale = current_zoom` in **logical** pixels: on a 2× display that is 0.5 raster px per device px — permanently under-resolved.  And the threshold treats the 2× base as “sharp enough” up to 220% zoom, though on Retina it is under-resolved from 100% up.  Net: between ~110% and 220% zoom the SVG blurs progressively; above 220% the “fresh” re-raster is still 2× below device resolution.  1× displays are unaffected.

## Reproduction

On a Retina Mac: open a text-heavy SVG, zoom to 400%, wait for the re-raster to land, compare with Preview.app at 400% — rpview’s glyph edges are visibly 2×-upscaled.  Test-shaped: after the fix, assert the pixmap dimensions produced for zoom Z on a scale-factor-S window are `intrinsic × Z × S` (today: `intrinsic × Z`).

## Suggested Fix

Thread the window’s scale factor into the SVG re-raster decision and render:

1. Capture `window.scale_factor()` in the render loop (it is available on `Window`) and store it on the viewer (it can change when the window moves between displays — update per frame like `viewport_size`).
2. Render re-rasters at `zoom × scale_factor` (`rerasterize_svg_full/viewport(tree, …, zoom * sf)`), and compare sharpness as `zoom * sf <= base_scale * 1.1` (the initial `2.0` base is then just “sf-appropriate for zoom ≤ 1 on a 2× display” and could itself become `sf.max(1.0) × 1.0`).
3. The display math already divides by the raster scale to get layout size, so no other changes — verify the viewport-region variant applies the same factor.

## Why This Fix

Rasterizing at device resolution is the definition of “crisp” on a scaled display; making the threshold compare in device px stops the system from declaring an under-resolved raster sharp.
