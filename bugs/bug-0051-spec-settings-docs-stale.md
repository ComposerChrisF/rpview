# Settings documentation describes a different schema and a different UI than what ships — following the docs can destroy a user’s settings

**Severity:** high (documentation) — the schema items are dangerous in combination with all-or-nothing deserialization
**Type:** SPEC bug — the code is (mostly) the truth; do NOT change code to match these docs, except where a companion code bug says otherwise
**Where:** `docs/SETTINGS.md`, `docs/SETTINGS_DESIGN.md`, `docs/SETTINGS_STATUS.md`, `DESIGN.md:232`
**Verified:** every item below checked against `src/state/settings.rs` and `src/components/settings_window.rs`

## Description

### A.  Schema drift in docs/SETTINGS.md (dangerous items first)

| Doc says | Code truth | Hazard |
|---|---|---|
| `max_image_dimensions`: array `[16384, 16384]` (lines ~116-124 and the “Complete Example” ~498) | `max_image_dimension: u32`, default `17000` (settings.rs:63,73; changed in commit `361ebf6`) | Pasting the doc’s Complete Example fails to parse → full settings reset (see `settings-missing-field-resets-all`) |
| `external_editor`: `String (optional)`, non-null example (~458-471) | `Option<ViewerConfig>` struct (settings.rs:343) | Same full-reset hazard |
| `background_color: [30,30,30]` (~249-257, ~515) | split into `background_color_dark` / `background_color_light` / `use_light_background` (commit `a234149`, v0.4.0) | Silently ignored key — edit does nothing |
| `default_save_format` default “Png”; values list omits “SameAsLoaded” | default is `SaveFormat::SameAsLoaded` (settings.rs:138) | Wrong expectations |
| `window_title_format` default `"{filename} ({index}/{total})"`; placeholder list omits `{sm}`/`{sortmode}` | default `"{filename} ({sm}, {index}/{total})"` (settings.rs:271) | Wrong expectations |
| `default_sort_mode` lists 2 values | 4 variants incl. `TypeAlpha`, `TypeModified` | Missing features |
| Missing fields entirely: `pan_direction_mode`, `use_light_background`, `background_color_dark/light`, `filter_window_bounds/_open`, `gpu_pipeline_window_bounds/_open` | exist since v0.6.0/v0.4.0/v0.11.0/v0.22.7 | Undocumented persisted state |

### B.  Range table drift (docs vs UI steppers in settings_window.rs)

`state_cache_size` doc “1–10000” vs UI 10–10000; `filter_processing_threads` doc “1–16” vs UI 1–32 (field is dead anyway — see `settings-dead-controls`); `pan_speed_fast` doc “1–100” vs UI 1–200; `pan_speed_slow` doc “0.1–100” vs UI 0.5–50; `font_size_scale` docs “0.5–2.0” vs UI 0.5–8.0; `max_image_dimension` UI 1000–100000 undocumented.

### C.  The Apply/Cancel UI that doesn’t exist

All four docs describe Apply/Cancel semantics: `docs/SETTINGS.md` (troubleshooting: “Make sure you clicked ‘Apply’”), `SETTINGS_DESIGN.md` (`original_settings`, `apply()`, `cancel()`, `[Reset to Defaults] [Cancel] [Apply]` layout), `SETTINGS_STATUS.md` (“Cmd+Enter to apply, Esc to cancel — Complete”), `DESIGN.md:232` (“Apply (`Cmd/Ctrl+Enter`) / Cancel (`Esc`)”).  The shipped UI has a single “Close” (auto-save) button; there is no cancel path at all, and the keyboard shortcuts are dead (see `settings-esc-dead-keybindings` — whose resolution decides what these docs should finally say).

### D.  Smaller items

- `docs/SETTINGS.md:559-563` corrupt-file behavior: step 4 (“overwrite with defaults”) currently false in code — but the companion bug `settings-file-not-created-on-first-run` restores it; sequence the doc edit after that decision.
- `settings.rs:94-95` doc comment and SETTINGS_DESIGN say slow pan is “Cmd/Ctrl”; actual binding is Alt (DESIGN.md and the UI label are correct).

## Suggested Fix (docs, after the companion code bugs are decided)

Regenerate `docs/SETTINGS.md`’s field reference and Complete Example **from the code** (`AppSettings::default()` serialized), fix the ranges from the stepper definitions, rewrite the window section for the real UI (as decided in `settings-esc-dead-keybindings`), and add the missing fields.  Mark `SETTINGS_DESIGN.md` and `SETTINGS_STATUS.md` as historical design/status docs (banner at top) rather than editing every stale claim.

## Why This Fix

docs/SETTINGS.md is user-facing and encourages hand-editing; a doc whose own example destroys the file it documents is worse than no doc.  Deriving the example from `AppSettings::default()` keeps it true mechanically.  The historical banner approach for the two design/status docs avoids maintaining three parallel descriptions of the same system (doc-placement: one authoritative copy).
