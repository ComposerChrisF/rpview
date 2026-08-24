# “Clear Cache for This Image” (per-image frame-cache purge) was removed with the LC window; TODO.md still claims it shipped

**Severity:** low — feature silently lost; only the all-or-nothing purge survives
**Type:** NEEDS-DECISION — restore the feature (where?) or strike the claim; do not treat as a pure doc fix without Chris’s ruling
**Where:** the function `purge_image(image_key)` no longer exists (`src/utils/frame_cache.rs` exports only `cache_root`, `image_key`, `raw_frame_path`, `purge_all`, `total_size`); the surviving “Clear All Cached Frames” button moved to the settings window (`src/components/settings_window.rs:1364`); TODO.md Phase 17 claims both buttons exist “in LC controls”
**Verified:** grep for `purge_image` (zero hits); `247605f` diff shows `frame_cache.rs` lost 84 lines in the LC removal

## Description

Phase 17 shipped two cache controls in the Local Contrast window: “Clear Cache for This Image” (`purge_image`) and “Clear All Cached Frames” (`purge_all`).  The v0.28.0 LC removal (`247605f`) deleted the LC window and took `purge_image` with it; `purge_all` was rehomed to the settings window.  Result: a user with one misbehaving or bloated animation can only nuke the entire cache.  TODO.md Phase 17 still lists both buttons as shipped, in a window that no longer exists.  (Also still open from Phase 17: no size-threshold warning; `total_size()` exists unused for that purpose.)

## Reproduction

Grep `purge_image` — absent.  Settings window shows only the purge-all button.

## Resolution options (Chris to pick)

- **A (restore):** re-add `purge_image(image_key) -> Result<u64, String>` to `frame_cache.rs` (delete `{key}_raw_*.png` + any per-frame GPU-era leftovers; recover the old implementation from `git show 247605f^:src/utils/frame_cache.rs`), and surface it — natural home is the GPU Pipeline window (the LC window’s successor) or a per-image entry in the settings cache section.
- **B (accept the loss):** update TODO.md Phase 17 to say the per-image purge was removed with LC in 0.28.0 and only purge-all remains (in Settings).

## Why This Fix

Either resolution ends the false claim; A restores a genuinely useful escape hatch (per-image invalidation without discarding every other animation’s cache) using code recoverable from git.
