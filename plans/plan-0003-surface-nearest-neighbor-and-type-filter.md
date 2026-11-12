# Plan: Surface Nearest-Neighbor Zoom in Settings and the Type Filter in the Debug Overlay

## Problem

Two recent toggles are visible only through their own key:

- **Nearest-neighbor zoom** (`N`, `viewer_behavior.nearest_neighbor`) is a saved setting, but the Settings window has no control for it.  It cannot be found by browsing Settings, nor reset with the other Viewer Behavior settings.
- **The type filter** (`Shift+Cmd+F`) shows only as the `F` after the sort label in the window title.  The debug overlay (`F12`) lists the sort mode, index, and total, but not the filter.  So its count can disagree with the folder’s image count and nothing on the overlay says why.

## Proposed Change

1. **Settings → Viewer Behavior** gains a toggle, “Nearest-neighbor zoom”, described as “Hard pixel edges when magnified (N)”.  It has a reset-to-default button like its neighbors, and Reset to Defaults clears it.
2. **The `N` key and the Settings toggle stay in step.**  Pressing `N` also updates the Settings window’s copy and its toggle.  Closing Settings then applies the toggle’s value and never reverts the key press.
3. **The debug overlay** gains a “Type Filter” row: `Off`, or the type it is limited to (`PNG`, `JPEG`, …).

## Implementation Notes

- `src/components/settings_window.rs`: a `nearest_neighbor_toggle` built with `create_toggle!`, rendered in `render_viewer_behavior`, reset in `reset_to_defaults`, plus a `set_nearest_neighbor(on, cx)` setter for the key handler.
- **Ownership moves.**  bug-0063’s fix carries `nearest_neighbor` across a Settings close as app-owned state.  Once the window edits it, it must leave that list, or the toggle would be ignored on close.  The setter in item 2 replaces that protection.
- `src/components/debug_overlay.rs`: `DebugOverlayConfig` gains `type_filter: Option<ImageTypeGroup>`, filled from `AppState::type_filter` where the config is built (`src/app_render.rs`, `src/main.rs`).  `ImageTypeGroup` gets a `label()` for the row.

## Why Not a Workaround

These are discoverability and diagnosis surfaces of the viewer itself.  The Settings window is where saved options are found and reset, and the debug overlay is where navigation state is inspected.  No outside script can add a row to either.
