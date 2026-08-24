# Animated playback starves the per-frame GPU cache: every completed render is discarded as “cancelled”, so slow stages never produce output during playback

**Severity:** medium — GPU runs at 100% producing nothing; processed playback never appears
**Type:** CODE bug
**Where:** `src/components/image_viewer.rs:1122-1126` (every new frame’s `update_gpu_pipeline` cancels the in-flight job), `:1246-1263` (`check_gpu_processing` discards a cancelled result entirely — `!cancelled &&` gates both the display install _and_ the `gpu_frame_renders[idx]` slot fill); `src/app_render.rs:224-229` (reapply fires on every frame advance)
**Verified:** install gate read directly (`let installed = if !cancelled && ...`); cancel-on-busy read; frame-advance reapply read

## Description

During playback, each frame advance requests GPU processing for the new frame, which cancels the in-flight job for the previous frame.  When a job takes longer than the frame interval, _every_ job is cancelled before completion is checked, and `check_gpu_processing` throws the whole result away — including the part that is still perfectly valid: the finished render belongs in `gpu_frame_renders[completed_job.frame_idx]` regardless of what frame is showing now.  The cancel flag exists to prevent a stale _display_ flash (per the `GpuJob` doc comment); it is being used to also discard durable cache fills.

Consequence: for any parameter set whose per-frame cost exceeds the frame delay, the per-frame cache (`42ed064`, v0.22.2) never fills, playback never shows processed frames, and the worker churns forever.

## Reproduction

Manual: animated GIF with short frame delays + expensive params (LC large radius, Equalize on) → play → frames stay unprocessed indefinitely; debug build shows continuous `[GPU_THREAD] start/done` churn with `produced` results never installed.

Test-shaped: start a job for frame 0, `request_cancel()`, complete the worker, call `check_gpu_processing()`, assert `current_image.gpu_frame_renders[0].is_some()` (fails today).

## Suggested Fix

In `check_gpu_processing`, split the two effects of `cancelled`:

- Always (when the result is `Some` **and** the job’s image identity matches — see `gpu-stale-render-after-navigation-or-reset`) store into `gpu_frame_renders[frame_idx]`.
- Only when `!cancelled` also install as the visible `gpu_pipeline_render` / `cached_gpu_pipeline_params`.

## Why This Fix

The cancel flag returns to meaning exactly what its comment says — “don’t flash this on screen” — while completed work is banked in the cache it was computed for.  On the second playback loop the cache hits and processed playback appears, converting the starvation into progressive warm-up.
