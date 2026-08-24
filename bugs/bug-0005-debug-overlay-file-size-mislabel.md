# Debug overlay labels the image’s pixel dimensions as “File Size”

**Severity:** low (trivial mislabel)
**Type:** CODE bug
**Where:** `src/components/debug_overlay.rs:124-128` (`image_dims_str = format!("{}x{}", w, h)`), `:184` (`render_info_line("File Size", image_dims_str)`)
**Verified:** both lines read directly (quoted in review)

## Description

F12’s overlay shows e.g. “File Size: 4000x3000” — the value is pixel dimensions, not a file size.  (Secondary note for the DESIGN.md pass: DESIGN.md:244 promises the debug overlay shows “performance” info; it renders filename/folder/index/dims/sort/zoom/pan/viewport only — fold the doc correction into the DESIGN.md rewrite tracked by `spec-docs-describe-removed-local-contrast`.)

## Reproduction

Press F12 with any image loaded; read the fourth info line.

## Suggested Fix

Rename the label to “Dimensions” (or add a real file-size line via `std::fs::metadata(path).len()`, formatted in KB/MB, if that was the original intent — Chris’s call, label rename is the minimum).

## Why This Fix

The label matches the value; optional metadata lookup restores what the label originally promised.
