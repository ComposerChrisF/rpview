# Closing the settings window reverts every setting changed outside it — background toggle and floating-window bounds/open flags, in memory and on disk

**Severity:** high — visible state reverts and silently clobbers persisted window bounds
**Type:** CODE bug (half-implemented working-copy pattern)
**Where:** `src/main.rs:348-349` (`SettingsWindow::new(settings.clone(), cx)` — seeded once at startup); `src/app_handlers.rs:105-120` (`handle_toggle_settings` never re-seeds); `src/app_handlers.rs:122-144` (`handle_close_settings` does wholesale `self.settings = new_settings` + save); out-of-band mutators: `src/app_handlers.rs:92-103` (B-key background toggle), `:251-296` and `:382-422` (filter/GPU window bounds + open-flag observers)
**Verified:** `working_settings` assigned only in `SettingsWindow::new` and `reset_to_defaults` (grep); close handler read in full

## Description

`SettingsWindow` snapshots the settings **once at app startup**.  Several settings are then legitimately mutated outside that window during the session — `use_light_background` (B key), `filter_window_bounds`/`filter_window_open`, `gpu_pipeline_window_bounds`/`gpu_pipeline_window_open` (drag/open/close observers) — each of which writes `self.settings` and saves to disk.

`handle_close_settings` replaces the whole struct: `self.settings = settings_window.get_settings()` and persists it.  Since the window’s copy still holds the startup values, **every Close reverts all out-of-band changes** — the background flips back, the floating windows’ saved positions are discarded, and a floating window the user opened has its `*_window_open` flag clobbered back to `false`, so restore-on-launch silently stops working.

## Reproduction

Manual (10 seconds): launch → press `B` (background goes light, saved) → `Cmd+,` → click Close → background flips back to dark and `settings.json` reads `"use_light_background": false`.

Test-shaped (pure state): construct `App`-level flow or factor the merge into a testable function —

```rust
// After: settings_window seeded with S0; app mutates S0→S1 (background=true);
// close_settings runs.  Assert the persisted settings retain background=true.
```

Pre-fix this fails because the close handler writes the window’s stale S0.

## Suggested Fix

Two complementary changes (do both):

1. **Re-seed on open:** in `handle_toggle_settings`, when opening, call a new `SettingsWindow::set_settings(self.settings.clone())` that refreshes `working_settings` (and the UI controls — the existing per-control `with_value` reads already pull from `working_settings` at render time; text inputs/steppers that cache state need their setters invoked, as `reset_to_defaults` already demonstrates at `settings_window.rs:847ff`).
2. **Merge, don’t replace, on close** — or, simpler and sufficient once (1) exists: keep the wholesale assignment, because the window’s copy is now fresh as of open, and the only out-of-band mutations that can occur _while the modal is open_ are the window-bounds observers (the B key is blocked by `is_modal_open`).  For those, either re-seed them into the working copy as they happen or copy the four bounds/open fields forward from `self.settings` into `new_settings` before assigning.

## Why This Fix

The root cause is one stale snapshot living for the entire app run.  Refreshing the snapshot at open bounds the staleness window to “while the settings UI is visible”, and carrying the bounds fields forward closes it entirely — no out-of-band setting can then be reverted by a Close.  This also interacts with the Esc bug (see companion report): today Esc leaves stale edits in `working_settings`; re-seeding on open erases them, fixing that trap as a side effect.
