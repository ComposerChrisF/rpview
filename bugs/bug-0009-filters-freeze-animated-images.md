# Applying any B/C/G filter to an animated image freezes the display on a filtered copy of frame 0 while the frame counter keeps advancing

**Severity:** high — core features (filters + animation) are mutually broken
**Type:** CODE bug
**Where:** `src/components/image_viewer.rs:914-1005` (`update_filtered_cache` — SVG guard at :917-922, **no animation guard**; decodes `loaded.path` = frame 0 at :971); `:1998-2011` (render source priority: `filtered_render` beats the per-frame animation path)
**Verified:** function read in full; render priority chain read; no-op filters correctly skip (so unfiltered playback works, which is why this went unnoticed)

## Description

`update_filtered_cache` decodes `loaded.path` — for a GIF/WebP that is frame 0 — applies the LUT, and installs `filtered_render`.  The render path chooses the image source as slot → GPU render → **`filtered_render`** → per-frame cached path.  With any non-neutral filter active on an animated image, every rendered frame is the same filtered frame 0; the animation timer keeps advancing `current_frame`, so the `AnimationIndicator` counts while the picture stands still.

Note the GPU pipeline solved this same problem properly (per-frame renders keyed by frame index); the CPU filter path predates animation-awareness.

## Reproduction

Manual: open an animated GIF, let it play, `Cmd+F`, drag Brightness — motion stops on a brightened frame 0; the frame counter keeps counting.

Test-shaped (state-level): with an animated `LoadedImage`, set non-neutral filters, run `update_filtered_cache`, complete the processing channel, and assert that either (a) `filtered_render` is `None` (option 1 below), or (b) the render source for frame N ≠ frame-0 filtered render (option 2).

## Suggested Fix

Two options, in ascending effort:

1. **Guard (parity with SVG):** at the top of `update_filtered_cache`, if `self.image_state.animation.is_some()`, clear and skip — filters simply don’t apply to animated images, and the filter window could show a hint.  Honest, cheap, removes the freeze.
2. **Per-frame filtering (feature-correct):** key the filtered render by frame index (mirroring `gpu_frame_renders`): filter the _current frame’s_ decoded RGBA (the frame cache PNGs already exist on disk; decode the current frame instead of `loaded.path`) and invalidate on frame change.  Costs one LUT pass per frame (~5–15 ms per the DESIGN numbers) — viable at typical GIF frame rates.

Option 1 is the immediate bug fix; option 2 is the feature DESIGN.md implies (“Filter state is remembered per-image” makes no animated/static distinction).  Chris may want 1 now, 2 later.

## Why This Fix

Both options remove the wrong-frame render from the priority chain: option 1 by never producing a stale `filtered_render` for animations, option 2 by making the filtered render track the frame index.  Either way the displayed pixels and the frame counter agree again.
