# The GPU Pipeline window is completely unreachable on Windows/Linux — no Ctrl keybinding, no menu item

**Severity:** high (on Windows/Linux; Windows is a primary target platform) — the app’s flagship processing feature cannot be opened at all
**Type:** CODE bug
**Where:** `src/app_keybindings.rs:17` (`shift-cmd-g` only; the `#[cfg(not(target_os = "macos"))]` block at `:115-169` duplicates every other shortcut with Ctrl but has **no `shift-ctrl-g`** and no `ResetGpuPipeline` twin); `src/components/menu_bar.rs:86-201` (the Win/Linux in-app menu has no GPU Pipeline entries); contrast: the macOS native menu has both (`src/app_keybindings.rs:223-224`)
**Verified:** grep for `shift-ctrl-g` / `ToggleGpuPipeline` across bindings and menu; the file’s own comment at `:115` explains GPUI 0.2.2 does not translate `cmd` to `ctrl` — which is why every other shortcut is duplicated

## Description

`ToggleGpuPipeline` and `ResetGpuPipeline` exist only as `shift-cmd-g` (etc.) bindings and macOS menu items.  On Windows/Linux, where `cmd` bindings don’t fire and the in-app menu bar is the only menu, there is no way to open the GPU Pipeline window.  The Local Contrast feature this replaced _did_ have `shift-ctrl-l`/`ctrl-p` twins — the platform-parity discipline was dropped for the replacement (v0.22.0, commit `4e5c68b`).

## Reproduction

Build on Windows (or temporarily compile the non-macOS keybinding block): press `Ctrl+Shift+G` — nothing.  Scan File/Edit/View/Navigate menus — no GPU Pipeline entry.

## Suggested Fix

1. Add to the non-macOS block: `KeyBinding::new("shift-ctrl-g", ToggleGpuPipeline, None)` (and a Ctrl twin for any other GPU action bound on macOS).
2. Add “GPU Pipeline...” (and “Reset GPU Pipeline”) to the in-app View menu in `menu_bar.rs`, mirroring the macOS menu (see the companion `menu-bar-drift-windows-linux` for the other missing items).

## Why This Fix

Restores the same reachability the feature has on macOS, following the file’s own established pattern for every other shortcut.
