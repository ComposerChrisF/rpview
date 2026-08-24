# Lanczos resize filters straight (non-premultiplied) alpha — dark fringing at transparent edges when resize ≠ 1×

**Severity:** low — quality defect on transparent PNG/WebP content, resize path only
**Type:** CODE bug (mechanism confirmed; visual severity to be confirmed with a transparent test asset)
**Where:** `src/gpu/shaders/lanczos_h.wgsl:46-56`, `src/gpu/shaders/lanczos_v_oklab.wgsl:50-60` (`color = color + texel * w` accumulates RGBA with RGB not weighted by alpha)
**Verified:** shader accumulation read; standard resampling theory (premultiply → filter → unpremultiply)

## Description

The two-pass Lanczos resize accumulates neighbor RGBA with straight alpha.  Fully transparent neighbors (whose RGB is typically black) bleed their color into edge pixels during resampling, producing dark fringes on resized images with transparency.  Fully opaque images are unaffected (weights normalize, alpha stays 1); the non-resize path never mixes pixels.  Note the CPU path (`rgba_to_bgra_render_image`) also uses straight alpha, so the GPU output is at least consistent app-wide — the defect is visible specifically where the resize mixes transparent and opaque texels.

## Reproduction

Transparent PNG with saturated content over a transparent (black-RGB) background; GPU pipeline with resize ½×, any stage enabled; inspect edge pixels — darkened halo versus a premultiplied reference resample (e.g. `magick input.png -resize 50% reference.png` which premultiplies).  Test-shaped: run the pipeline on a synthetic 2-px opaque-red/transparent-black checker at ½× and assert edge texels retain red hue above a threshold.

## Suggested Fix

In `lanczos_h.wgsl`: multiply RGB by alpha when sampling (`let t = texel; accum += vec4(t.rgb * t.a, t.a) * w;`).  In `lanczos_v_oklab.wgsl` (the final resample stage before OKLab conversion): after accumulation, unpremultiply (`rgb / max(a, 1e-6)`) before the color-space transform.  Guard the divide for a = 0.

## Why This Fix

Premultiply-filter-unpremultiply is the standard correct pipeline for resampling with alpha: transparent texels then contribute no color, only coverage, eliminating the fringe at its source without affecting opaque images (a = 1 is a no-op through both steps).
