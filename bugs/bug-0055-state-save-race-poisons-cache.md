# Input during an in-flight image load saves the previous image’s state under the new image’s path — permanently poisoning the per-image state cache

**Severity:** high — persistent wrong zoom/pan/filters/animation on the destination image
**Type:** CODE bug
**Where:** `src/components/image_viewer.rs:779-807` (`load_image_async` clears `current_image` but not `image_state`); `src/app_handlers.rs:1374-1382` (`do_pan` → `save_current_image_state`), same pattern in `do_zoom`, `adjust_filter`, and the filter-controls subscription (`src/main.rs:311-323`); `src/app_handlers.rs:1439-1448` (`save_current_image_state` keys on `app_state.current_image()`); `src/app_render.rs:28-37` (restore-on-load-completion)
**Verified:** all four code sites read directly; no ownership check exists anywhere on the save path

## Description

Navigation immediately updates `app_state.current_index` and kicks off an async load, but the viewer’s `image_state` (zoom, pan, filters, animation) still belongs to the image being navigated _away from_ — `load_image_async` clears `current_image`, `error_message`, and SVG state, but not `image_state`.

`save_current_image_state()` snapshots `viewer.get_image_state()` and stores it under `app_state.current_image()` — the _destination_ path.  So any handler that mutates state and saves during the in-flight window (WASD pan, `+`/`-` zoom, a filter slider tick) writes image A’s state into `image_states[B]`.

When the load completes, `app_render.rs:33-37` sees `image_states.contains_key(&B)` and restores the poisoned entry instead of applying the default zoom mode — B opens with A’s zoom/pan/filters, and even A’s `AnimationState` (a frame index B may not have; `app_render.rs:44-52` then calls `cache_frame` with it).  Because `remember_per_image_state` defaults to true, the corrupted entry persists in the LRU: **every future visit to B restores the wrong state** until eviction.

Holding a pan key while arrow-keying through a directory of large images reproduces this reliably; a single pan keystroke during one slow load is enough.

## Reproduction (Rust test)

The pieces are testable without GPUI: the poisoning is pure state manipulation.

```rust
#[test]
fn state_saved_during_load_must_not_poison_destination() {
    // A at zoom 4.0; navigate to B; viewer still holds A's state (load in flight).
    let mut app_state = AppState::new(vec![a.clone(), b.clone()]);
    app_state.current_index = 0;
    let mut a_state = ImageState::new();
    a_state.zoom = 4.0;
    app_state.save_current_state(a_state.clone());

    app_state.current_index = 1;            // handle_next_image
    // viewer.image_state still == A's state; simulate do_pan's save:
    app_state.save_current_state(a_state);  // ← what save_current_image_state does today

    // B was never displayed, yet it now has a cached state with A's zoom:
    assert!(!app_state.image_states.contains_key(&b));  // FAILS today
}
```

(An integration-shaped test would drive `handle_next_image` + `handle_pan_up` with a stubbed slow loader and assert `image_states` has no B entry before B’s load completes.)

## Suggested Fix

Make the save conditional on the viewer actually owning the current image.  Two options:

- **Robust (preferred):** have `ImageViewer` record which path its `image_state` belongs to (e.g. set `state_owner: Option<PathBuf>` in `load_image_async`’s completion path, not at kickoff).  `save_current_image_state` compares `viewer.state_owner` against `app_state.current_image()` and skips the write on mismatch.
- **Minimal:** early-return from `save_current_image_state` when `self.viewer.is_loading` is true.  (Slightly lossier: state mutations made mid-load are not persisted — acceptable, since they are being applied to the wrong image’s state anyway.)

Either way, add the regression test above.

## Why This Fix

The defect is a key/value mismatch: the value (viewer state) and the key (current path) come from two structs that go out of sync during every async load.  Tying the save to an ownership check removes the mismatch at its root instead of patching individual handlers (pan, zoom, filters, and any future mutator all flow through `save_current_image_state`).
