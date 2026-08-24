# Every SVG (re-)rasterization persists a temp PNG that is never deleted — unbounded temp-dir growth

**Severity:** medium — multi-MB file per zoom gesture; on Windows the temp dir is never auto-cleaned
**Type:** CODE bug (resource leak)
**Where:** `src/utils/svg.rs:96-113` (`rerasterize_svg_full`: `into_temp_path().keep()`), `:166-173` (`rerasterize_svg_viewport`, same); no `remove_file` for these paths exists anywhere in `src/` (grep verified); producers: initial load (`src/utils/image_loader.rs:107-123`) and every debounced zoom/pan re-raster (`src/components/image_viewer.rs` SVG re-raster path)
**Verified:** grep for deletion sites; `.keep()` semantics (persist the file, disown cleanup)

## Description

Each rasterization writes `rpview_svg_reraster_*.png` / `rpview_svg_viewport_*.png` into the OS temp dir and `keep()`s it so GPUI can load it by path.  When a newer raster replaces an older one, the older file is simply forgotten.  A session of zooming around a large SVG produces dozens to hundreds of PNGs (viewport rasters at high zoom are several MB each).  macOS clears /tmp periodically (≈3 days), bounding the damage; **Windows never auto-cleans %TEMP%**, so on the other primary platform this accumulates indefinitely.

## Reproduction

Open a large SVG; zoom in/out ~20 times with pauses (letting the debounce fire); `ls $TMPDIR/rpview_svg_*` — one file per re-raster, none removed, including after quitting.  Test-shaped: track the previous raster path in the viewer; after installing a replacement, assert the previous file no longer exists.

## Suggested Fix

Track and reap the previous raster:

1. In the viewer, when a new raster path is installed (both the full and viewport variants, and the `rasterized_path` from a new image load replacing an old SVG’s), delete the outgoing path — but **one generation late**: GPUI may still be presenting the old texture this frame, so keep `previous_raster: Option<PathBuf>` and delete it when it is displaced a second time (or after the new image is confirmed rendered).  Deleting a file whose texture GPUI already uploaded is safe on Unix; on Windows delete may fail while mapped — treat failure as “try again next displacement”.
2. Best-effort sweep at startup: remove `rpview_svg_*` files older than a day from the temp dir (guarded to the exact prefix), catching files leaked by crashes.

## Why This Fix

The generation-delayed reap bounds live temp files to ~2 per SVG regardless of session length, and the prefix sweep cleans historical and crash leakage; neither changes rendering behavior.
