# `ConfirmDelete` action has been dead since birth — no keybinding, no menu item, nothing dispatches it (and there is no keyboard confirm)

**Severity:** low — dead code plus a small UX gap
**Type:** CODE bug (dead code); NEEDS-DECISION on whether a keyboard confirm is wanted (DESIGN.md only promises “ESC cancels”, so mouse-only may be intentional)
**Where:** `src/lib.rs:86` (action defined); `src/app_render.rs:889-891` (handler registered); no dispatch site exists — the confirmation card’s button calls `handle_confirm_delete` directly via a mouse listener (`src/app_render.rs:538-545`)
**Verified:** grep for `ConfirmDelete` across bindings, menus, and dispatch sites

## Description

The action was added with the delete feature (v0.5.0, commit `b54d1e8`) but never bound to a key or menu item, and the card’s button bypasses it.  Consequences: (a) dead action + dead handler registration; (b) the delete confirmation cannot be accepted from the keyboard at all — arm with `Cmd+Backspace`, then the only way forward is the mouse (Esc cancels).

## Reproduction

Arm a delete; press Enter/Return/any key — nothing confirms.  Grep: `ConfirmDelete` appears only in lib.rs, the import lists, and the handler registration.

## Suggested Fix

Chris to pick:

- **A (complete the feature):** bind `enter` → `ConfirmDelete` gated to the armed state (the handler already no-ops when `pending_delete` is `None`), and have the card’s button dispatch the action instead of calling the method — one path.  Update the card’s hint text and help overlay to mention Enter.
- **B (mouse-only is intended):** delete the action and its handler registration; keep the direct mouse call.

Recommendation: A — a keyboard-driven app (per DESIGN.md’s “keyboard-first workflow”) whose only destructive confirmation is mouse-only is an odd exception; the accidental-Enter risk is mitigated by the card being armed only after an explicit chord.

## Why This Fix

Either option removes the dead code; A additionally closes the keyboard-first gap using the plumbing that already exists.
