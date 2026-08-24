# Windows/Linux in-app menu: wrong shortcut labels for Disable/Enable Filters, and missing items that exist in the macOS menus

**Severity:** low-medium (Windows is a primary target platform)
**Type:** CODE bug (wrong labels; feature-parity gap)
**Where:** `src/components/menu_bar.rs:155-164` (labels via `format_shortcut("1"/"2", ...)` render “Ctrl+1”/“Ctrl+2”; the real bindings are bare `1`/`2` — `src/app_keybindings.rs:79-80`); missing items: `menu_bar.rs:86-201` has no Delete File / Permanently Delete File (File), no GPU Pipeline / Reset GPU Pipeline / Toggle Zoom Indicator / Toggle Background (View), no Sort by Type toggle (Navigate) — all present in the macOS native menus (`src/app_keybindings.rs:174-252`)
**Verified:** menu definitions read directly (quoted in review)

## Description

Two defects in the Win/Linux in-app menu bar:

1. **Actively wrong labels:** `format_shortcut` always prepends the platform modifier, so “Disable Filters” shows **Ctrl+1** — and `ctrl-3…9` is the slot-_store_ family, so the displayed convention points users at the wrong key family entirely.  The bindings are bare `1`/`2`.
2. **Missing entries:** delete (both variants), the GPU Pipeline pair, zoom-indicator and background toggles, and the type-sort toggle are reachable on macOS via menus but absent from the only menu Win/Linux users have.  (The GPU pair is additionally unreachable by keyboard there — see `gpu-pipeline-unreachable-windows-linux`.)

## Reproduction

Read the menu on a non-macOS build (or force-compile `menu_bar.rs`): View → “Disable Filters Ctrl+1”.  Pressing Ctrl+1 does nothing; pressing 1 works.

## Suggested Fix

1. Labels: `Some("1")` / `Some("2")` literals, like the neighboring `Some("H")` / `Some("0")`.
2. Add the missing items mirroring `app_keybindings.rs`’s macOS menus: File → Delete File… (`Ctrl+Backspace` label per the binding; see `delete-shortcut-backspace-vs-delete` for the label/binding ruling), Permanently Delete File…; View → GPU Pipeline… (`Shift+Ctrl+G` once bound), Reset GPU Pipeline, Toggle Zoom Indicator (`T`), Toggle Background (`B`); Navigate → Sort by Type (`Shift+Ctrl+T`).

## Why This Fix

Menu labels become statements of the real keymap, and the two platforms expose the same feature set through their respective menus — restoring the parity discipline the rest of the keybinding file follows.
