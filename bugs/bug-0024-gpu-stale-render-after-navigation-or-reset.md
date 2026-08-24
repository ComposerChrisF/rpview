# In-flight GPU job carries no image identity — its result installs onto whichever image is current (wrong image after navigation; resurrection after reset)

**Severity:** high — image B silently displays image A’s processed pixels
**Type:** CODE bug
**Where:** `src/components/image_viewer.rs:86-96` (`GpuJob` has `params` + `frame_idx` but no image identity); `:1228-1263` (`check_gpu_processing` installs into `self.current_image`, whatever it now is); the paths that fail to cancel: `:779-807` (`load_image_async` — cancels the loader, never `gpu_job`), `:1024-1031` (`update_gpu_pipeline` SVG early-return sits **above** the busy-cancel at `:1122-1126`), `:1273-1281` (`reset_gpu_pipeline` — clears caches, never cancels), `src/app_handlers.rs:1484-1487` (`reapply_gpu_pipeline_if_active` early-returns when `gpu_pipeline_enabled == false`, so it never reaches the cancel)
**Verified:** all sites read directly; render priority (`image_viewer.rs:1993-2004`) confirms the wrong render is what gets displayed

## Description

The GPU worker’s result is installed by `check_gpu_processing` into `self.current_image` with no check that the current image is still the one the job was started for.  Correctness depends on _every_ navigation path cancelling the in-flight job via `update_gpu_pipeline`’s busy branch — and three paths don’t:

1. **Pipeline display-disabled (`1` key):** `reapply_gpu_pipeline_if_active` returns immediately when `gpu_pipeline_enabled` is false, so navigating away never cancels.  Image A’s job completes after image B loads and installs A’s pixels, params, and per-frame slot into B’s `LoadedImage`.  Pressing `2` then displays **A’s picture while B is loaded**, and the poisoned `cached_gpu_pipeline_params` make the “already matches” early-return (`:1090-1095`) keep it indefinitely.
2. **Navigating to an SVG:** `update_gpu_pipeline` returns at the SVG guard _before_ the busy-cancel, so even with the pipeline enabled the stale A-job survives and installs onto the SVG’s `LoadedImage`.
3. **Reset:** `reset_gpu_pipeline` clears the cached renders but not the worker; with all sliders already at defaults, `reset_all` emits only `ResetRequested` (sliders that don’t change emit no `Change`), so nothing cancels — the “reset-away” render completes and reappears seconds after the user reset it.

## Reproduction

Manual (variant 1): open large image A → `Shift+Cmd+G`, enable an expensive stage (LC, big radius, 1×) → press `1` → drag any slider (a worker still spawns; `main.rs:334-336` calls `update_gpu_pipeline` unconditionally) → immediately `→` to image B → wait 1 s → press `2` → B displays A’s processed pixels.

Test-shaped: `update_gpu_pipeline(non_identity)` on image A → `set_gpu_pipeline_enabled(false)` → `load_image_async(B)` → complete B’s load → poll `check_gpu_processing()` until it returns true → assert `current_image.gpu_pipeline_render.is_none()` (fails today).

## Suggested Fix

1. Add the source image’s path (or a monotonically increasing generation counter bumped in `load_image_async`) to `GpuJob`; in `check_gpu_processing`, drop the result when it doesn’t match the current image.  This is the structural fix — it makes all cancel-gaps harmless.
2. Belt-and-braces (cheap, do both): `load_image_async` and `reset_gpu_pipeline` call `request_cancel()` on any live `gpu_job` and clear `pending_gpu_params`; hoist the busy-cancel in `update_gpu_pipeline` above the SVG early-return.

## Why This Fix

Identity-checking at install time converts “correct only if every current and future navigation path remembers to cancel” into “correct by construction” — the same fix shape as the per-image state race (`state-save-race-poisons-cache`): both bugs are a value produced for image A being keyed to image B because nothing ties the value to its source.
