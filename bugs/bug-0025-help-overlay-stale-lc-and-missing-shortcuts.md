# Help overlay advertises two removed Local Contrast shortcuts and omits two real ones (GPU Pipeline, Cmd+0)

**Severity:** medium — the in-app documentation ships dead keys and hides live ones
**Type:** CODE bug (help text) — companion to the DESIGN.md doc-drift bug, but this one ships inside the binary
**Where:** `src/components/help_overlay.rs:204-213` (“⇧⌘L Show/hide Local Contrast dialog”, “⌘P Apply Local Contrast (process to buffer)”); missing entries: `shift-cmd-g`/`shift-ctrl-g` → `ToggleGpuPipeline` (`src/app_keybindings.rs:17`) and `cmd-0`/`ctrl-0` → `ZoomResetAndCenter` (`:23`)
**Verified:** help entries read directly (quoted in review); full cross-check of every help entry against `app_keybindings.rs` found all other 40+ entries correct

## Description

Commit `247605f` (v0.28.0) deleted the `shift-cmd-l` and `cmd-p` bindings with the CPU Local Contrast feature but did not touch `help_overlay.rs` — the overlay has advertised dead shortcuts since.  Meanwhile the replacement feature’s shortcut (`Shift+Cmd/Ctrl+G`) and `Cmd/Ctrl+0` (reset zoom & center, documented in DESIGN.md:120) appear nowhere in the overlay, though DESIGN.md:243 promises it lists all shortcuts.  (The `shift-cmd-s` SaveFile alias is deliberately unadvertised per the comment at `app_keybindings.rs:99-104` — not part of this bug.)

## Reproduction

Press `H`; Filters section lists ⇧⌘L and ⌘P; pressing them does nothing.  Search the overlay for “GPU” — absent.

## Suggested Fix

In `help_overlay.rs`: delete the two LC entries; add `format_shortcut("G", true, false)` → “Show/hide GPU Pipeline window” (same section) and `format_shortcut("0", false, false)` → “Reset zoom and center (fit ↔ 100%)” in the Zoom section.  Wording for Cmd+0 should match whatever the `spec-cmd-0-toggle-not-reset` resolution decides.

## Why This Fix

Makes the in-app reference match the actual keymap; the cross-check confirmed these four lines are the only divergence, so the fix is complete, not just spot repair.

## Git Evidence

`247605f` removed the bindings without touching `help_overlay.rs` (last changed in `d5676d9`, v0.20.0).
