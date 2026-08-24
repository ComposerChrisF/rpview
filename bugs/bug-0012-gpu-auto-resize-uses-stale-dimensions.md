# Auto-resize computes the new image’s factor from the previous image’s dimensions (ordering bug in the render pass)

**Severity:** medium — wrong resize factor cached against the new image until the user touches the panel
**Type:** CODE bug
**Where:** `src/app_render.rs:5-16` (dims pushed to the controls at render start, from the **pre-load** `current_image`), `:28` (`check_async_load` installs the new image later in the same pass), `:58` (`reapply_gpu_pipeline_if_active` immediately after); `src/components/gpu_pipeline_controls.rs:273-290` (`effective_resize_factor` reads the stored dims)
**Verified:** ordering read directly in `render()`; `set_image_dimensions` stores silently and nothing re-triggers processing when correct dims arrive one frame later

## Description

Within a single `render()` pass the order is: push dims (still image A’s) → `check_async_load` installs image B → reapply → `get_params` → `effective_resize_factor` uses **A’s** dimensions.  B is processed and its result cached under `cached_gpu_pipeline_params` at A’s auto factor.  The correct dims are pushed on the next frame, but nothing fires `ParametersChanged`, so the wrong factor sticks until the user moves a slider.

Example: navigate from an 8192-px image (auto ½×) to a 1200-px image (auto should be 2× or 1×): the small image renders at ½× — visibly soft.

## Reproduction

Manual: enable Auto resize; folder with one very large and one small image adjacent; navigate; the small image’s processed output is at the large image’s factor (confirm via the panel’s “Auto (N×)” label after nudging a slider — the render sharpens).

Test-shaped: set dims (8192, 8192); read `effective_resize_factor`; set dims (1200, 1200); without any event, assert the factor used by the next `get_params` matches the 1200-px answer — then pin the fix by asserting reapply happens after `check_async_load` installs the image (or that dims are pushed post-install).

## Suggested Fix

Move the dims push to _after_ `check_async_load` in `render()` (immediately before `reapply_gpu_pipeline_if_active`), reading `self.viewer.current_image` at that point.  Alternatively (more robust to future reordering): have `reapply_gpu_pipeline_if_active` push current dims itself before calling `get_params`.

## Why This Fix

The factor is derived state of the _current_ image; computing it after the image installs makes the derivation use its own input.  Placing the push inside the reapply path guards every future caller, not just this render-pass ordering.
