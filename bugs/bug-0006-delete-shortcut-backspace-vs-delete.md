# Windows/Linux delete is bound to Ctrl+Backspace but documented as Ctrl+Delete (help overlay and DESIGN.md)

**Severity:** low-medium — on Win/Linux, following the app’s own help produces no action
**Type:** BOTH — the non-mac help text is a CODE bug; DESIGN.md’s platform-generic “Cmd/Ctrl+Delete” is a SPEC imprecision; the resolution needs one small ruling (bind both vs relabel)
**Where:** `src/app_keybindings.rs:166-169` (`ctrl-backspace` / `shift-ctrl-backspace`); `src/components/help_overlay.rs:252-260` (renders `format_shortcut("Delete", …)` → “Ctrl+Delete” / “Ctrl+Shift+Delete” on Win/Linux); `DESIGN.md:185-186` (“Cmd/Ctrl+Delete”); TODO.md:335-336 says Cmd+Backspace (matches the code)
**Verified:** bindings and help entries read directly

## Description

On macOS, “⌘Delete” and `cmd-backspace` are the same physical key, so mac users see no discrepancy.  On Windows/Linux, Backspace and Delete are different keys: the help overlay and DESIGN.md tell users Ctrl+Delete, which does nothing; the working chord (Ctrl+Backspace) is documented nowhere.

## Reproduction

Non-macOS build: press Ctrl+Delete (per help overlay) — nothing.  Press Ctrl+Backspace — delete confirmation appears.

## Suggested Fix

Recommended: **bind both** on non-macOS — add `KeyBinding::new("ctrl-delete", RequestDelete, None)` and `shift-ctrl-delete` alongside the backspace bindings (Windows convention favors the Delete key for file deletion anyway) — then the existing help/DESIGN text becomes true, and the backspace chord remains for muscle-memory parity with macOS.  Alternative (if double-binding is unwanted): change the help overlay’s non-mac label to “Backspace” and adjust DESIGN.md:185-186 to name the per-platform keys.

## Why This Fix

Binding both makes every existing piece of documentation correct with two added lines and no behavioral downside; the alternative fixes the docs at the cost of a nonstandard Windows key choice.
