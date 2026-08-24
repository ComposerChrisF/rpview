# DESIGN.md still documents “Space + drag” pan — replaced by plain click-and-drag in v0.20.0

**Severity:** low (documentation; plus one dead setting cross-referenced)
**Type:** SPEC bug — the replacement was deliberate; fix DESIGN.md (the dead `spacebar_pan_accelerated` setting is tracked in `settings-dead-controls`)
**Where:** `DESIGN.md:124-131` (pan table row “`Space` + drag | 1:1 mouse tracking”); code truth: `src/app_render.rs:244` comment “(Spacebar-drag removed: click-and-drag pans directly now.)”; commit `d5676d9` (v0.20.0) made the change; the help overlay already says “Click + Drag”
**Verified:** comment and help text read; git commit identified

## Description

Panning is plain left-click drag; the spacebar chord is gone.  Only DESIGN.md still describes it.  The related settings field `keyboard_mouse.spacebar_pan_accelerated` outlived its feature and still renders as a toggle in the settings window — that half is covered by `settings-dead-controls`.

## Reproduction

Click-drag pans; holding Space changes nothing.

## Suggested Fix

Replace the DESIGN.md pan-table row with “Click + drag | 1:1 mouse tracking”, and sweep the section for other spacebar references.

## Why This Fix

Matches the spec to a deliberate v0.20.0 UX change that the in-app help already reflects.
