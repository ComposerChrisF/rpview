# Manual frame stepping (`[` / `]`) reaches uncached frames and shows “Failed to load image frame”

**Severity:** medium — a documented feature errors on first use with a cold cache
**Type:** CODE bug (violates DESIGN.md:211 “`[` / `]` — step frame by frame”)
**Where:** `src/app_handlers.rs:1207-1244` (`handle_next_frame` / `handle_previous_frame` — call `set_current_frame` but never `cache_frame`); `src/components/image_viewer.rs:1940-1961` (render early-returns to `ErrorDisplay("Failed to load image frame")` when the frame’s cached PNG is missing); the only `cache_frame` call sites are `src/app_render.rs:44-52` (restored frame) and `:182-193` (Phase-2 look-ahead — runs **only while `is_playing`**)
**Verified:** both handlers read in full (quoted in review); `cache_frame` call sites enumerated

## Description

The loader pre-caches frames 0..3; look-ahead caching runs only during playback.  The manual step handlers pause playback and jump `current_frame` — `]` three times reaches frame 3 (uncached on a cold first visit), and `[` from frame 0 wraps to the **last** frame (never cached).  The render path then finds no cached PNG for the frame and replaces the whole view with the error display.

## Reproduction

Settings → animation auto-play OFF (or press `O` right after load).  Open a ≥5-frame GIF on its first visit (cold cache — clear the frame cache in Settings first).  Press `[` once → jumps to the last frame → full-screen “Failed to load image frame”.  Test-shaped: drive `handle_previous_frame` on a freshly loaded animated image with only frames 0..2 cached; assert the target frame’s cache file exists after the handler (fails today).

## Suggested Fix

Add `self.viewer.cache_frame(target)` in both step handlers after computing the target index (mirroring what `app_render.rs:44-52` already does for a restored out-of-range frame).  `cache_frame` is synchronous per-frame PNG encode — acceptable for a manual step; optionally also cache `target±1` for smoother repeated stepping.

## Why This Fix

The render path’s contract is “current frame is cached”; the two manual mutators are the only writers of `current_frame` that don’t uphold it.  Adding the same call the restore path uses closes the contract for all writers.
