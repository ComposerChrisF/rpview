# Single-path drop / “Open With” failures are completely silent — not even the console log TODO.md claims

**Severity:** low — dropping an unsupported file or empty folder does nothing, with zero feedback
**Type:** CODE bug
**Where:** `src/app_handlers.rs:1094-1098` (`import_image_paths`: `if let Ok((images, index)) = process_dropped_path(&paths[0])` — the `Err` is discarded); contrast `handle_open_file`, which surfaces errors (`:558-562`); TODO.md Phase 11.5 claims “Handle error messages via eprintln (logged to console)”
**Verified:** handler read directly (quoted in review)

## Description

For a single dropped path (also the macOS “Open With” route), any error from `process_dropped_path` — unsupported format, empty directory, file vanished between event and processing — is swallowed by the `if let Ok`.  Nothing happens on screen, nothing is printed.  The multi-path branch silently skips invalid entries too; if all are invalid, the drop is a no-op with no message.  The app has both an error-message surface (`viewer.error_message`) and toasts; neither is used here.

## Reproduction

Drop a `.txt` file onto the window: nothing.  Test-shaped: call `import_image_paths(&[txt_path])` and assert some user-visible signal is set (`viewer.error_message.is_some()` after fix; fails today — all fields unchanged).

## Suggested Fix

Match on the result: on `Err(e)`, set `self.toast = Some(ToastState { message: "Could not open dropped file".into(), detail: Some(e.to_string()), is_error: true, created_at: Instant::now() })` (toast, not `error_message`, so the current image stays visible) and `cx.notify()`.  In the multi-path branch, if the loop ends with `all_images.is_empty()`, show the same toast with a “no supported images in drop” detail.

## Why This Fix

Puts drop-error handling at parity with `handle_open_file` using the existing toast machinery; the failure becomes observable exactly where the user is looking.
