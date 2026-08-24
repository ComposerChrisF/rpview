# DESIGN.md says Cmd/Ctrl+0 “resets zoom and pan to centered fit-to-window” — it is actually a fit ↔ 100% toggle, deliberately

**Severity:** low (documentation)
**Type:** SPEC bug — the toggle is a deliberate, committed behavior; fix DESIGN.md, do NOT “fix” the code back to a reset
**Where:** `DESIGN.md:120`; code truth: `src/components/image_viewer.rs:559-567` (`reset_zoom_and_pan`: “Toggle between fit-to-window (centered) and 100% (centered)” — from the default fit state, Cmd+0 goes **to 100%**)
**Verified:** function read directly (quoted in review); git history below

## Description

DESIGN.md’s zoom section says `Cmd/Ctrl+0` resets to centered fit-to-window.  The implementation toggles: at fit → centered 100%; anywhere else → centered fit.  So from a fresh image (already at fit), Cmd+0 moves _away_ from fit — the opposite of a reset.  Commit `917f168` (v0.6.3, “Fix Cmd+0 and 0 zoom toggle to properly center and toggle fit-to-window”) made this deliberate; DESIGN.md was edited afterwards (`9a98177`) without correcting the line.  Plain `0` (`reset_zoom`) matches its spec line (viewport-center-preserving toggle).

## Reproduction

Open an image (fit), press Cmd+0 → zoom becomes 100%, not fit.

## Suggested Fix

Reword DESIGN.md:120 to: “`Cmd/Ctrl+0` toggles fit-to-window ↔ 100%, both fully centered (unlike `0`, which preserves the viewport-center anchor point)”.  Also add Cmd+0 to the help overlay (tracked in `help-overlay-stale-lc-and-missing-shortcuts`) with the same wording.

## Why This Fix

The behavior was chosen on purpose and shipped for 4+ months; the one stale sentence is the defect.  The reworded line also documents the real difference between `0` and `Cmd+0` (anchor-preserving vs fully-centered), which today is discoverable only by reading the source.
