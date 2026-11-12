# Closing the Settings window reverts state the app saved since launch

**Severity:** medium — silently undoes user choices and persists the undo to `settings.json`.

## Symptom

1. Launch rpview with the dark background.
2. Press `B` (light background).  It is saved to `settings.json`.
3. Open Settings (`Cmd+,`) and close it with `Escape` or `Cmd+Enter`, changing nothing.

The background flips back to dark, and `settings.json` now says `use_light_background: false`.

The same happens to every setting the app writes outside the Settings window:

| Field | Written by |
|---|---|
| `appearance.use_light_background` | `B` (`handle_toggle_background`) |
| `viewer_behavior.nearest_neighbor` | `N` (`handle_toggle_nearest_neighbor`) |
| `appearance.filter_window_bounds` / `filter_window_open` | moving, opening, or closing the Filter window |
| `appearance.gpu_pipeline_window_bounds` / `gpu_pipeline_window_open` | moving, opening, or closing the GPU Pipeline window |

So closing Settings also forgets where the floating windows were and whether they were open.

## Cause

`SettingsWindow` is created once per main window (`src/main.rs`, `SettingsWindow::new(settings.clone(), cx)`) and keeps that snapshot as `working_settings`.  Nothing refreshes it.  `handle_close_settings` (`src/app_handlers.rs`) replaces `App::settings` wholesale with `working_settings` and saves it, so every field above goes back to its launch-time value.  It also happens if the field changed while Settings was open (a floating window dragged meanwhile).

## Expected

Closing Settings applies what the Settings window edits, and leaves alone the state that only the app writes.
