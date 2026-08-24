# DESIGN.md claims the zoom indicator displays the sort mode — it never has

**Severity:** low (documentation)
**Type:** SPEC bug (claim was never true in any version)
**Where:** `DESIGN.md:49` (“`zoom_indicator.rs` # Zoom %, resolution, sort mode display”); code truth: `src/components/zoom_indicator.rs:38-78` renders zoom text (“Fit (NN%)” / “NN%”) and `W×H` only; `git log -S "sort_mode" -- src/components/zoom_indicator.rs` returns nothing
**Verified:** render function read in full (quoted in review); git pickaxe run

## Description

Sort mode is shown in the window title (`{sm}` placeholder) and the debug overlay, not the zoom indicator.  The DESIGN.md module-layout annotation invents a third location.

## Reproduction

Press `T` to show the indicator; it contains zoom and dimensions only.

## Suggested Fix

Change the DESIGN.md annotation to “Zoom %, resolution display” (fold into the DESIGN.md rewrite tracked by `spec-docs-describe-removed-local-contrast` if convenient — same file, same pass).

## Why This Fix

Removes a never-true claim; alternatively, if Chris actually wants sort mode there, that is a feature request, not this bug.
