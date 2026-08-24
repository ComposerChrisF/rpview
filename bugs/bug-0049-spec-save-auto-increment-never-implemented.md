# DESIGN.md documents auto-incrementing save filenames (`image_filtered_2.png`) — never implemented in any version

**Severity:** low (documentation, or a small feature — needs a ruling)
**Type:** NEEDS-DECISION — aspirational spec vs code; either implement the increment or strike the claim
**Where:** `DESIGN.md:189-190` (“Auto-increment filenames on save: `image.png` → `image_filtered.png` → `image_filtered_2.png`”); code truth: `src/app_handlers.rs:641-646` suggests only `{stem}_filtered.{ext}` — no existence probe, no counter, in any historical version (`git log -S "_filtered_2"` matches only the DESIGN.md commit `cf977e9`; the original Phase 10 implementation `cdd0bd9` already lacked it)
**Verified:** current and original save handlers read; git pickaxe run

## Description

The `_filtered_2` escalation exists only in DESIGN.md.  In practice the OS save dialog’s overwrite confirmation covers the collision case, so the feature’s absence has a mild cost (an extra click and a decision) rather than data loss — the dialog never silently overwrites.

## Resolution options (Chris to pick)

- **A (fix spec):** reword DESIGN.md:189-190 to “Suggested filename gains a `_filtered` suffix when processing is active; the OS save dialog handles name collisions.”
- **B (implement):** before `set_file_name`, probe the resolved directory: while `dir.join(&suggested).exists()`, step `image_filtered.png → image_filtered_2.png → image_filtered_3.png …`.  Note: the probe is advisory (picks a free suggestion), not a gate on destruction — the dialog still confirms overwrites — so a bare `exists()` is acceptable here.  Implement in `handle_save_file_impl` where `directory` is resolved (`src/app_handlers.rs:648-651`); needs the directory known before the dialog, which it is.

Recommendation: B is cheap (~10 lines) and matches the written intent; but A is defensible since the dialog already prevents accidental overwrite.  Chris’s call.

## Reproduction

Save a filtered image twice into the same folder: the second save suggests the same `image_filtered.png` and relies on the overwrite prompt — DESIGN.md says it would suggest `image_filtered_2.png`.

## Why This Fix

Ends a spec claim that has never matched any shipped build; whichever direction is chosen, the spec and the dialog behavior finally agree.
