# DESIGN.md’s zoom table omits the required Cmd/Ctrl modifier on scroll-wheel zoom

**Severity:** low (documentation)
**Type:** SPEC bug — the modifier requirement is deliberate (bare scroll is reserved); fix DESIGN.md
**Where:** `DESIGN.md:115` (“Scroll wheel | Zoom at cursor position…”); code truth: `src/app_render.rs:392-395` — zoom only fires `if event.modifiers.platform` (Cmd on macOS, Ctrl elsewhere); bare scroll does nothing
**Verified:** handler read directly; the help overlay already renders the shortcut correctly as “⌘Scroll”

## Description

The spec’s table row implies bare scrolling zooms.  The code requires the platform modifier, and the in-app help agrees with the code.  A DESIGN.md reader (or a test written from it) expects bare-scroll zoom that has never existed.

## Reproduction

Scroll over an image without a modifier: nothing.  With Cmd/Ctrl held: cursor-anchored zoom.

## Suggested Fix

Change the table row to “`Cmd/Ctrl` + scroll wheel | Zoom at cursor position (pixel under cursor stays fixed)”.

## Why This Fix

One-line spec correction matching both the code and the shipped help text.
