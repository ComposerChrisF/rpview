# Settings window: Esc closes WITHOUT saving and Cmd/Ctrl+Enter does nothing — the `SettingsWindow` key context is never declared

**Severity:** high — silent loss of settings edits, plus a deferred-apply trap
**Type:** CODE bug (dead bindings, wrong footer text) + SPEC conflict (docs disagree with each other on Esc semantics — Chris must rule; see “Decision required”)
**Where:** `src/app_keybindings.rs:75-76,145` (bindings gated on context `"SettingsWindow"`); no `key_context("SettingsWindow")` exists anywhere (repo grep: only `"ImageViewer"` in `src/app_render.rs:608`, `src/components/filter_window.rs:38`, `src/components/gpu_pipeline_window.rs:39`); `src/app_handlers.rs:49-54` (`handle_escape` hides settings without saving); `src/components/settings_window.rs:2344,2360` (footer claims “⌘Enter or Esc to close and save”)
**Verified:** grep for `key_context` across the repo; `handle_escape` and `handle_close_settings` read in full; independently confirmed by two review passes

## Description

The three `CloseSettings` keybindings all require a `"SettingsWindow"` key context that no element ever declares, so they can never fire:

- **Cmd/Ctrl+Enter does nothing at all** in the settings window.
- **Esc** falls through to the context-free `escape → EscapePressed` binding and lands in `handle_escape`’s settings branch (`app_handlers.rs:49-54`), which sets `show_settings = false` — **without** calling `handle_close_settings`.  Nothing is saved, nothing is applied.

The footer text (“⌘Enter or Esc to close and save”) is therefore false on both counts.  The only working save path is the mouse “Close” button (which dispatches `CloseSettings` directly).

**The trap:** the `SettingsWindow` entity keeps its edited `working_settings` for the app’s lifetime (there is no revert).  Edits “discarded” with Esc silently reappear when settings is reopened — and are applied and saved the next time the user clicks Close, possibly much later, possibly bundled with a forgotten “Reset all settings to Defaults” click.  Combined with the no-save-on-quit gap (separate bug), “edit → Esc → quit” loses the edits entirely.

## Reproduction

Manual: `Cmd+,` → change any toggle → press `Cmd+Enter` (nothing happens) → press Esc (window closes; `settings.json` unchanged; no “Settings saved successfully” on stdout) → `Cmd+,` again (edit still shown) → click Close (edit now applies and saves).

Test-shaped: after the fix, a GPUI integration test can assert that dispatching `CloseSettings` from the settings window’s dispatch path works via keyboard; pre-fix, a unit assertion can pin the contract by checking that some rendered element declares `key_context("SettingsWindow")` (or better, restructure so the binding uses the existing `"ImageViewer"`-style context that the window actually sets).

## Decision required (Chris)

The docs disagree about what Esc _should_ do:

- `DESIGN.md:232`: “Apply (`Cmd/Ctrl+Enter`) / **Cancel (`Esc`)**” — Esc reverts.
- `settings_window.rs` footer + module header (and TODO.md Phase 16.7 step 9): auto-save on close; “Esc … close **and save**”.
- There is no revert implementation at all today (`original_settings` from `docs/SETTINGS_DESIGN.md` was never built).

Option A (smaller): Esc = close-and-save, same as the Close button; fix DESIGN.md to say so.  Option B (matches DESIGN.md): implement a true working-copy revert — re-seed `working_settings` from `self.settings` on every open, Esc discards, Cmd+Enter/Close saves.  **Option B also fixes the deferred-apply trap by construction**; Option A must additionally re-seed on open (see companion bug on stale working-copy).

## Suggested Fix (code, once the decision is made)

1. Add `.key_context("SettingsWindow")` to the settings window’s root element in `SettingsWindow::render` (next to the existing `.track_focus(...)`), making the two `CloseSettings` bindings live.
2. Remove the `show_settings` branch from `handle_escape` (or make it dispatch the decided action) so Esc has exactly one meaning.
3. Update the footer string to match the decided semantics.

## Why This Fix

The bindings were written for a context that was never wired — adding the context makes the declared design (keyboard close) function.  Routing Esc through the same handler as the Close button (or through a real cancel) removes the second, divergent close path that silently skips persistence, which is the root of both the data-loss and the deferred-apply symptoms.

## Git Evidence

The bindings have carried the `SettingsWindow` context since the Phase 16.7 settings-UI work; no commit ever added a matching `key_context` — this never worked.
