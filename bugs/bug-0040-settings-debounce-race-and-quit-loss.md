# Debounced settings writer can clobber a newer immediate save; pending writes are dropped at quit; no save-on-quit exists

**Severity:** medium — stale window bounds/open flags persisted; occasional lost edits
**Type:** CODE bug (plus divergence from docs/SETTINGS_DESIGN.md, which specifies save-on-quit)
**Where:** `src/utils/settings_io.rs:97-138` (debounce worker); mixed callers in `src/app_handlers.rs` (bounds observers use `save_settings_debounced` at :256, :387; close/open handlers use immediate `save_settings` at :268, :282, :295, :399, :411, :421); `src/main.rs:260-262` (`Quit` → bare `cx.quit()`, no flush)
**Verified:** worker logic and all call sites read; no flush or synchronization exists

## Description

Three related loss modes in one subsystem:

1. **Stale-over-fresh race.**  Bounds observers enqueue snapshots to a worker thread (up to 250 ms latency); every other save path writes immediately on the main thread.  The two paths are unsynchronized.  Drag the filter window (snapshot A queued, `filter_window_open: true`) → close it within 250 ms (immediate save B, `filter_window_open: false`) → the worker wakes and writes stale A over B.  Next launch reopens a window the user closed.
2. **Pending write dropped at quit.**  The worker is a detached thread; a queued snapshot dies with the process.  Drag a window, `Cmd+Q` within 250 ms → final position lost.
3. **No save-on-quit at all.**  `docs/SETTINGS_DESIGN.md` §Auto-save Strategy specifies “Save on quit: In `Quit` action handler as safety measure”; the handler is a bare `cx.quit()`.

## Reproduction

Test-shaped for mode 1 (timing-based, mark `#[ignore]` if flaky in CI): send snapshot A to the debounced path, sleep 50 ms, write B via `save_settings_to_path`, sleep 300 ms, read the file — today it contains A.  Mode 2/3 are best pinned after the fix by unit-testing the flush function directly.

## Suggested Fix

1. Add a monotonically increasing sequence number (AtomicU64) stamped when a save is _requested_, checked by the worker before writing: if a newer immediate save has already been performed (`last_written_seq >= my_seq`), skip.  Simplest correct form: route the **immediate** saves through the same worker with a “write now” message so all writes are serialized in request order on one thread.
2. Add `settings_io::flush()` — send a flush message, wait (bounded, e.g. 500 ms) for the worker to drain — and call it from the `Quit` handler and the main-window-closed → quit path (`src/main.rs:421-434`), followed by nothing else.  This also implements the SETTINGS_DESIGN save-on-quit clause.

## Why This Fix

Serializing all writes through one ordered channel removes the interleaving that lets an older snapshot land last; a bounded flush at the two quit paths turns “whatever happened to be on disk” into “the last state the user saw”.  Both are contained in `settings_io.rs` plus two call sites.
