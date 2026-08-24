# Floating-window bounds are restored verbatim with no on-screen validation — window can reopen fully offscreen and unreachable

**Severity:** low-medium — recoverable only by hand-editing settings.json
**Type:** CODE bug (SUSPECTED severity: confirm on-screen behavior live before fixing; the code path is certain)
**Where:** `src/app_handlers.rs:200-207` (`open_filter_window`: `filter_window_bounds.map(|b| b.to_bounds())` straight into `open_window`), same pattern for the GPU pipeline window (`:327-334`); `src/state/settings.rs:226-242` (`WindowBounds` — no validation on load)
**Verified:** restore path read (no clamp, no display intersection check); live offscreen behavior not yet exercised

## Description

Saved bounds are fed to `open_window` unmodified.  If the display arrangement changed since the save (external monitor unplugged, resolution lowered), the always-on-top panel is restored at coordinates on a display that no longer exists — open (its `*_window_open` flag says so, and it re-opens on every launch) but invisible and unfocusable.  Hand-edited or corrupted values (negative sizes, NaN) also pass straight through to the window system.

## Reproduction

Set in settings.json: `"filter_window_bounds": {"x": 20000.0, "y": 20000.0, "width": 360.0, "height": 320.0}` and `"filter_window_open": true`; launch.  Confirm whether macOS rescues the window (it may clamp); if it does not, the bug is user-visible.  On Windows (primary target platform) offscreen restore is the classic failure mode.  Test-shaped: unit-test the clamp function below with an offscreen rect and a display list.

## Suggested Fix

Add a `fn clamp_to_visible(bounds: Bounds<Pixels>, cx: &App) -> Bounds<Pixels>` used by both open paths: intersect the saved rect with the union of `cx.displays()` bounds; if the intersection is smaller than a minimum grab area (say 100×40 px visible), fall back to the centered default.  Also sanitize on deserialize (reject non-finite or non-positive sizes → `None`).

## Why This Fix

The saved value is a hint, not a contract — validating it against the _current_ display set at open time handles monitor changes, hand-edits, and corruption in one place, and the fallback is the same centered default already used for first open.
