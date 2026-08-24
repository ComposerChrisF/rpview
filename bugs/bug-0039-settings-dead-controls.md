# Eight settings are documented and/or exposed as UI controls but read by no code — the toggles do nothing

**Severity:** medium — users flip switches that have no effect; two are presented as fully functional
**Type:** CODE bug for the fields presented as functional; NEEDS-DECISION per field (implement vs remove) — do not blindly “fix the code” for the placeholders, some may be wanted as future work
**Where:** `src/state/settings.rs` (field definitions); `src/components/settings_window.rs` (controls); zero read-sites confirmed by grep outside settings definition/UI for every field below
**Verified:** grep across `src/` per field (no consumers); preload render path read (`src/app_render.rs:160-169` fills `preload_paths` unconditionally)

## Description

| Field | UI control? | Docs claim | Reality |
|---|---|---|---|
| `performance.preload_adjacent_images` | toggle | SETTINGS.md + DESIGN.md: functional | preloading is unconditional; toggle ignored |
| `performance.filter_processing_threads` | stepper 1–32 | SETTINGS.md: functional (“1–16”) | never read; filters use rayon’s default pool |
| `filters.remember_filter_state` | toggle | “currently always enabled” | never read; per-image filter memory actually follows `remember_per_image_state` — the toggle is deceptive |
| `external_tools.enable_file_manager_integration` | toggle | placeholder | never read; `handle_reveal_in_finder` doesn’t check it |
| `keyboard_mouse.spacebar_pan_accelerated` | toggle | placeholder (doc admits) | never read; the spacebar-pan feature itself was removed (see `spec-space-drag-pan-removed`) |
| `file_operations.auto_save_filtered_cache` | toggle | placeholder (doc admits) | never read |
| `file_operations.remember_last_directory` | toggle | placeholder (doc admits) | never read |
| `filters.filter_presets` | none | placeholder (doc admits) | never read |

The first two are the serious ones: DESIGN.md’s Performance section advertises the “preload toggle”, and disabling it changes nothing (both neighbors still decode and upload to GPU textures every render).

## Reproduction

For `preload_adjacent_images`: disable in settings, open a folder of large images, observe `preload_paths` still populated each render (or memory/GPU load unchanged).  Test-shaped after fix: with the setting false, assert `viewer.preload_paths.is_empty()` after a render pass.

## Suggested Fix (per field — Chris to ratify each)

- `preload_adjacent_images`: **implement** — wrap `src/app_render.rs:163-169` in `if self.settings.performance.preload_adjacent_images { ... }`.  One-line, restores the documented contract.
- `filter_processing_threads`: **remove** (recommended) — the rayon default pool made it obsolete; delete field + UI + docs.  (Implementing would mean a custom rayon pool — not worth it.)
- `remember_filter_state`: **decide** — either implement (per-image filter memory gated separately from zoom/pan memory) or remove the field and its deceptive toggle.
- `enable_file_manager_integration`: **implement** (hide/disable Reveal menu item and no-op the handler when false) or remove.
- `spacebar_pan_accelerated`: **remove** — its feature no longer exists.
- `auto_save_filtered_cache`, `remember_last_directory`, `filter_presets`: **decide** — these are Phase 16.5-deferred features; either keep the fields but _hide the dead UI controls_ until implemented, or remove entirely.  A visible toggle that does nothing is the bug; a reserved JSON field is fine.

Removal is safe for serde (unknown fields are ignored on load), but note each removal changes `settings.json` schema — update docs/SETTINGS.md in the same pass (see `spec-settings-docs-stale`).

## Why This Fix

Every visible control either works or disappears; reserved-for-future fields stop masquerading as features.  The per-field decision is explicitly Chris’s because several are deliberate placeholders documented as such — deleting those wholesale would discard planned work, and implementing all of them is scope, not a bug fix.
