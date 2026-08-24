# GPU pipeline errors are compiled out of release builds; the Auto resize button is never gated by GPU limits; `OutputTooLarge` reports the wrong dimension

**Severity:** medium — in release, an oversized image makes the pipeline a silent no-op with the stage showing “enabled”
**Type:** CODE bug (three related reporting defects; the limits guard itself is correct)
**Where:** `src/components/image_viewer.rs:1210-1213` (worker error → `debug_eprintln!`, which is `#[cfg(debug_assertions)]` — `src/utils/mod.rs:14-19`); `src/components/gpu_pipeline_controls.rs:283-289` (Auto fallback `best = 0.25` even when 0.25 still violates the target) and `:792-813` (Auto button lacks the `factor_exceeds_gpu_limits` gating the discrete buttons have at `:300-311`); `src/gpu/unified.rs:680-688` (`OutputTooLarge { width: out_w, height: out_h }` even when the **source** dimension tripped the guard)
**Verified:** macro definition, error arm, and both button paths read directly

## Description

1. **Silent no-op in release.**  When the unified pipeline returns `Err` (dimension over `max_texture_dimension_2d`, or any GPU failure), the only report is a `debug_eprintln!`.  Release users click a stage on, nothing changes, no toast, no log — the panel shows the stage enabled while the display shows the unprocessed source.
2. **Auto not gated.**  The 1/4×…4× buttons are disabled when the resulting texture would exceed GPU limits; the Auto button is not, and Auto’s fallback hard-codes 0.25 even when 0.25 also exceeds the ≤4096 target its doc comment promises (“without exceeding it”) — for a >65k-px panorama this lands exactly in failure 1.
3. **Misleading error text.**  `OutputTooLarge` always carries the _output_ dims, so a 20000-px source at ¼× produces “output 5000×3750 exceeds GPU max texture dimension 16384” — self-contradictory.

## Reproduction

Release build + image with a side longer than `max_texture_dimension_2d` (16384 on most Metal devices; synthesize a 20000×100 PNG): enable Vibrance → nothing happens, nothing is reported.  For (3): debug build, same image at ¼×, read the error text.

## Suggested Fix

1. Surface worker `Err` to the user: pass the error string through the result channel (it already flows to `check_gpu_processing`) and set the app toast (`ToastState { is_error: true, .. }`), plus an unconditional `eprintln!`.
2. Gate the Auto button with the same `factor_exceeds_gpu_limits` check; make Auto’s search return an `Err`/`None` when even the smallest factor violates the limit, and show that state in the panel instead of silently picking 0.25.
3. Include both source and output dims in `GpuError::OutputTooLarge` (or name which one exceeded).

## Why This Fix

The guard already prevents the crash; these changes make it _fail loudly_ (portfolio posture: a tool must not present “enabled” UI over an operation that silently did not run).  Routing the error through the existing channel and toast reuses infrastructure verified elsewhere in the app.
