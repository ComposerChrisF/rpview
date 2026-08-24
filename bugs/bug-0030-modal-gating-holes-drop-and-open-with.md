# Drag & drop, macOS “Open With”, and several handlers are not gated while the delete confirmation is armed

**Severity:** medium-low — an armed delete confirmation can be retargeted between the user’s decision and their click
**Type:** CODE bug (gating hole)
**Where:** ungated: `src/app_handlers.rs:1132-1140` (`handle_dropped_files`), `:1143-1166` (`process_pending_open_paths` — fed by the 250 ms poll loop in `src/main.rs:579-598`), `:1085-1130` (`import_image_paths`), `:186-193` (`handle_toggle_filters`), `:425-441` (`handle_disable_filters`/`handle_enable_filters`), `:443-461` (`handle_store_slot`/`handle_recall_slot`); contrast: navigation/zoom/pan/file handlers all early-return on `is_modal_open()` (`:8-10`)
**Verified:** each handler read; `is_modal_open` = `show_settings || pending_delete.is_some()`

## Description

With the delete confirmation card up, dropping files or receiving a Finder “Open With” event (which arrives asynchronously via the poll loop — the user needn’t do anything at that moment) replaces `image_paths` and `current_index`.  The still-armed confirmation then targets whatever became current.  Severity is bounded because the card re-renders the _current_ filename and `handle_confirm_delete` deletes the _current_ image — display and action stay consistent — but an async retarget can land between the user reading the filename and clicking Delete, deleting a file they never chose.  The filter/slot handlers have the same hole with milder consequences (state churn under a modal).

## Reproduction

`RequestDelete` on image X (card shows X) → deliver a pending open path (simulate by pushing into `PENDING_OPEN_PATHS` and calling `process_pending_open_paths`) → card now shows the imported image; Confirm deletes it.  Test-shaped: arm `pending_delete`, call `process_pending_open_paths` with a valid path, assert `image_paths` unchanged (fails today).

## Suggested Fix

Add the same early-return the other handlers use: `if self.is_modal_open() { return; }` at the top of `import_image_paths` (covering drop + Open With in one place) and of the filter/slot handlers.  For Open With specifically, don’t drop the request — leave the paths in `PENDING_OPEN_PATHS` (return before `mem::take`) so they process when the modal clears.

## Why This Fix

It extends the existing, deliberate gating pattern to the handlers that missed it; deferring (not discarding) pending Open With paths preserves the user’s Finder action instead of silently eating it.
