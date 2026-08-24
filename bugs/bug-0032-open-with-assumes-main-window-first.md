# “Open With” delivery assumes the main window is first in `cx.windows()` — a silent no-op if a floating window ever comes first

**Severity:** low (latent — ordering currently holds)
**Type:** CODE bug (fragile assumption, silent failure mode)
**Where:** `src/main.rs:603-613` (`check_and_process_pending_paths`: `cx.windows().first()` + `downcast::<App>()`; on downcast failure it does nothing and the poll loop retries forever)
**Verified:** function read in full

## Description

Pending “Open With” paths are delivered by downcasting the **first** window to `App`.  The main window is created first today, so it works — but nothing guarantees GPUI’s `windows()` ordering, and if a floating Filter/GPU window ever sorts first (creation-order change, platform difference, future GPUI update), the downcast fails silently and pending paths are re-polled every 250 ms forever, never delivered.  No error, no log.

## Reproduction

Latent; demonstrable by reordering (open a floating window first in a test harness) or by unit-testing the fixed version’s window-search logic.

## Suggested Fix

Iterate all windows and use the first successful downcast:

```rust
for window in cx.windows() {
    if window.update(cx, |view, window, cx| {
        if let Ok(app) = view.downcast::<App>() {
            app.update(cx, |app, cx| app.process_pending_open_paths(window, cx));
            true
        } else { false }
    }).unwrap_or(false) { break; }
}
```

Log a warning if no `App` window was found.

## Why This Fix

Removes the ordering assumption entirely for a few lines; the warning converts the residual impossible-case from silent to loud.
