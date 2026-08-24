# Some settings changes only take effect after restart, inconsistently — cache size never resized, font scale applied to some UI but not floating panels

**Severity:** low-medium
**Type:** CODE bug (partial application; docs present the settings window as live)
**Where:** `src/app_handlers.rs:122-144` (`handle_close_settings` — saves and assigns but applies nothing); `src/main.rs:214` (`state_cache_size` → `AppState::max_cache_size`, startup only), `:305,:328` (`font_size_scale` captured at `FilterControls`/`GpuPipelineControls` construction, never updated); contrast: help/debug overlays get live values every render (`src/app_render.rs:432-433,456-457`)
**Verified:** `handle_close_settings` read in full; construction sites read

## Description

`handle_close_settings` persists the new settings and replaces `self.settings`, but performs no application pass:

- `viewer_behavior.state_cache_size` — `app_state.max_cache_size` keeps its launch value; changing 1000 → 50 in the UI does nothing until relaunch (TODO.md Phase 16.4 claims “Resize cache when state_cache_size changes” as done).
- `appearance.font_size_scale` — help and debug overlays pick up the new scale immediately (live reads at render), but the floating Filter and GPU Pipeline panels keep their construction-time scale until relaunch.  A user changing the scale sees half the UI obey — worse than all-or-nothing because it looks like a rendering glitch.

## Reproduction

Manual: `Cmd+,` → set Font Size Scale 1.0 → 2.0 → Close.  Help overlay text doubles; the Filter window’s text does not.  Test-shaped: after factoring an `apply_settings(&mut self, cx)` (below), assert `app_state.max_cache_size` equals the new value after close.

## Suggested Fix

Add an `apply_settings` step inside `handle_close_settings` after `self.settings = new_settings`:

- `self.app_state.max_cache_size = new.viewer_behavior.state_cache_size;` (evict down to the new cap if smaller — loop `evict_oldest_state` while over).
- Push `font_size_scale` into the two controls entities (add a `set_font_scale` method that stores and `cx.notify()`s).

## Why This Fix

It closes the gap between “saved” and “applied” at the single choke point every settings change already flows through, making the settings window behave as the docs and TODO claim.
