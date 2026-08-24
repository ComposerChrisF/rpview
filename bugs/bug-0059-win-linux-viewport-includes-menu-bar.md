# Windows/Linux: viewport size and mouse coordinates include the 28 px in-app menu bar — cursor-zoom anchor and fit/centering off by 28 px

**Severity:** medium (Windows is a primary target platform); macOS unaffected
**Type:** CODE bug (coordinate-space confusion: window-relative vs content-relative)
**Where:** `src/app_render.rs:156-158` (`window.viewport_size()` used raw for `update_viewport_size`), `:392-415` (scroll-zoom feeds `event.position` — window coordinates — to `zoom_toward_point`), `:602-624` + `src/components/menu_bar.rs:353` (on non-macOS the layout is a flex column: 28 px menu bar above the content, so the image container starts at y = 28)
**Verified:** layout and both coordinate uses read; GPUI mouse events are window-relative; cannot be exercised live on this macOS machine — confirm on Windows before/while fixing

## Description

On Windows/Linux the image content area is offset 28 px down by the in-app menu bar, but:

- `update_viewport_size` receives the full window size, so fit-to-window computes against a viewport 28 px taller than the visible content — a height-limited fit overflows/clips 28 px and vertical centering is off by 14 px.
- Ctrl+scroll cursor-centered zoom anchors at `event.position` in window coordinates, 28 px below the true content-relative cursor — the pixel under the cursor walks downward on every notch.
- The drag-pan and Z-drag anchors have the same 28 px skew (deltas are unaffected; anchors are).

## Reproduction

On Windows: place the cursor on a distinctive pixel, Ctrl+scroll repeatedly — the pixel drifts down out from under the cursor.  Test-shaped: with viewport 1000×828 (28 px bar), content-center in window coords is (500, 442); `zoom_toward_point(500, 442, …)` anchors an image pixel 28/zoom px below the content center.

## Suggested Fix

Introduce a single content-area offset: on non-macOS, subtract the menu-bar height (define one `MENU_BAR_HEIGHT: Pixels = px(28.0)` constant shared with `menu_bar.rs` instead of the current magic number) from `window.viewport_size().height` before `update_viewport_size`, and from `event.position.y` before every use as a content coordinate (scroll zoom, drag anchors).  Cleanest long-term: track the content element’s actual bounds via GPUI’s element-bounds callback and derive both from it, so a future toolbar doesn’t reintroduce the skew.

## Why This Fix

All the math already assumes content-relative coordinates (correct on macOS where the offset is 0); supplying the correct origin/size on the platforms with an in-app bar fixes fit, centering, and every anchor in one place.
