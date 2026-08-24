# Newly viewed images get hardcoded filter values 0/0/1.0 while reset and state-restore honor the configurable defaults

**Severity:** low — inconsistency visible only to users with non-neutral `filters.default_*` settings
**Type:** BOTH — code inconsistency, plus the two settings docs define the setting differently (needs a one-line ruling)
**Where:** `src/app_render.rs:74-83` (fresh-image path hardcodes `brightness: 0.0, contrast: 0.0, gamma: 1.0`) vs `src/app_handlers.rs:463-469` (`handle_reset_filters` uses `settings.filters.default_*`) and `:1450-1456` (`load_current_image_state` passes settings defaults)
**Verified:** all three sites read; independently flagged by two review passes

## Description

Three code paths initialize/reset filter values and they disagree.  A user who sets `default_brightness: 20` gets 20 from `Shift+Cmd/Ctrl+R` (reset) and from cached-state defaults, but 0 on every newly viewed image (the fresh-image branch in the render loop).  The docs disagree with each other about intent: `docs/SETTINGS.md` describes `default_*` as the values used _when resetting_; `docs/SETTINGS_DESIGN.md` calls them “Starting brightness/contrast/gamma”.

## Decision required (Chris)

Are `filters.default_*` (a) starting values for any image with no saved state, or (b) only the target of the Reset action?  Recommendation: (a) — it matches `load_current_image_state`’s existing behavior and makes all three paths agree; (b) would instead require changing `load_current_image_state` to pass neutral values.

## Reproduction (Rust test, after choosing (a))

Set `settings.filters.default_brightness = 20.0`; drive the fresh-image path (or factor the reset-values selection into a helper `fn initial_filters(settings) -> FilterSettings` used by all three sites); assert the fresh-image filters equal the settings defaults, not 0/0/1.0.

## Suggested Fix

Extract one helper (e.g. `App::default_filter_settings()`) returning `FilterSettings` built from `self.settings.filters`, and call it from all three sites.  Update whichever doc lost the ruling.

## Why This Fix

A single source for “what are default filters” makes divergence impossible; the bug is precisely that the value was constructed in three places.
