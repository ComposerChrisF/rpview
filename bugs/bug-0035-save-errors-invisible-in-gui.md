# Save success/failure is reported only to stdout/stderr — invisible when launched from Finder

**Severity:** medium — a failed save looks identical to a successful one in the GUI
**Type:** CODE bug (spec gap: DESIGN.md specifies toasts for delete but is silent for save)
**Where:** `src/app_handlers.rs:744-747` (save result match: `println!`/`eprintln!`), contrast with the delete flow which shows toasts (`src/app_handlers.rs:908-938`)
**Verified:** code path read in full

## Description

The async save task ends with:

```rust
match result {
    Ok(()) => println!("Image saved to: {}", save_path.display()),
    Err(e) => eprintln!("Failed to save image: {}", e),
}
```

An app launched from Finder / the Dock has no visible stdout or stderr, so a failed save (disk full, permission denied, unsupported format per the companion foreign-extension bug) produces **no user-visible feedback at all** — the dialog closes and nothing happens.  The app already has a toast mechanism used by the delete flow (`ToastState`, auto-dismiss ~2.5 s), so the infrastructure exists; the save path just predates it.

## Reproduction

Not unit-testable as-is (the outcome is a console side effect inside a detached task).  Manual: open an image, `Cmd+S`, choose a directory without write permission (e.g. `/`), confirm — no error appears anywhere in the UI.  After the fix, assert the handler sets `self.toast` — refactor the completion into a method on `App` (e.g. `fn on_save_completed(&mut self, result: Result<PathBuf, String>)`) and unit-test that both arms set an appropriate `ToastState` (`is_error` true/false).

## Suggested Fix

At the end of the spawned save future, re-enter the app entity (the `cx.spawn` closure receives a handle — use `_cx.update(...)` / the `WeakEntity<App>` pattern already used elsewhere in the file) and set `self.toast = Some(ToastState { message: "Saved" / "Save failed", detail: Some(path-or-error), is_error, created_at: Instant::now() })`, then `cx.notify()`.  Keep the stderr line for terminal users.

## Why This Fix

It routes the outcome through the same channel the delete flow already uses, so a GUI-launched user sees success and failure equally; nothing about the save logic changes, only its reporting.  The refactor into `on_save_completed` also makes the behavior unit-testable.
