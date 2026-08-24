# DESIGN.md says “Cmd/Ctrl+F **or F**” toggles the filter window — bare F was never bound

**Severity:** low (documentation, or a two-line feature — needs a ruling)
**Type:** NEEDS-DECISION — spec and code have always disagreed; either bind F or strike it from the spec
**Where:** `DESIGN.md:141` (“`Cmd/Ctrl+F` or `F` — toggle floating filter window”); code truth: only `cmd-f` (`src/app_keybindings.rs:78`) and `ctrl-f` (non-mac, `:147`); `git log -S '"f", ToggleFilters'` across all history returns nothing — bare F never existed
**Verified:** bindings grepped; help overlay and menu bar correctly advertise Cmd/Ctrl+F only

## Description

The spec’s “or F” has never been true in any version.  The in-app help and menus never claimed it, so only DESIGN.md is wrong — but since this is an aspirational-spec case (like the save auto-increment), the resolution is a product decision, not automatically a doc edit.

## Resolution options (Chris to pick)

- **A (fix spec):** delete “or `F`” from DESIGN.md:141.  Zero risk.
- **B (implement):** add `KeyBinding::new("f", ToggleFilters, Some("ImageViewer"))` (+ non-mac twin).  Note the context matters: a context-free bare-letter binding would fire while typing in settings text inputs; gating on `"ImageViewer"` avoids that.  Update help overlay if adopted.

Recommendation: A — single-letter toggles are already crowded (`1 2 3-9 0 O T B H F12 [ ] Z WASD IJKL`), and `F` sits inside the WASD/IJKL pan cluster where an accidental window toggle mid-pan would be irritating.

## Reproduction

Press `F` with an image focused: nothing happens (pan cluster unaffected; no binding).

## Why This Fix

Either option ends a spec line that has misdocumented the product since day one; the recommendation preserves the existing keyboard ergonomics.
