# Deleting the last image leaves a blank window instead of the “no images” notice

**Severity:** low — inconsistent empty state after the toast fades
**Type:** CODE bug
**Where:** `src/components/image_viewer.rs:1656-1662` (`clear()` sets `no_images_path = None`); startup path sets the friendly notice (`src/main.rs:284-289`); the delete flow reaches `clear()` via `update_viewer` when the list empties
**Verified:** `clear()` read directly (quoted in review); startup contrast read

## Description

Launching into an empty directory shows a friendly “no images found in <dir>” notice (`no_images_path`).  Deleting the last image empties the list through `viewer.clear()`, which nulls that field — after the 2.5 s deletion toast, the user faces a blank window titled “rpview” with no explanation.  The two paths to the same state (no images) render differently.

## Reproduction

Directory with one image → open it → `Cmd+Backspace` → confirm → after the toast: blank window.  Test-shaped: drive `handle_confirm_delete` on a single-image list (with the FS delete stubbed/using a temp file) and assert `viewer.no_images_path.is_some()` afterward (fails today).

## Suggested Fix

In the post-delete path (where `remove_current_image` leaves an empty list, before/instead of plain `clear()`), set `viewer.no_images_path = Some(parent_dir_of_deleted)` — the same presentation the startup path uses.

## Why This Fix

Both routes into “no images” converge on the same explanatory screen; no new UI needed.
