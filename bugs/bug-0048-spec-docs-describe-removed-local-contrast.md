# Documentation still describes the removed CPU Local Contrast feature; GPU pipeline undocumented

**Severity:** high (documentation) — the primary spec no longer matches the shipped product
**Type:** SPEC bug — the code is correct; do NOT change code to match these docs
**Where:** `DESIGN.md`, `README.md`, `docs/COMPONENTS.md`, `docs/TESTING.md`, `docs/TROUBLESHOOTING.md`, `docs/local-contrast-spec.md`, `TODO.md` (Phase 17)
**Verified:** files named by the docs do not exist in the tree; git history confirms removal

## Description

Commit `247605f` (“lc: remove CPU DSP (Local Contrast) feature entirely — 0.28.0”) deleted 4,722 lines — `src/utils/local_contrast.rs`, `float_map.rs`, `color.rs`, `lc_presets.rs`, `src/components/local_contrast_controls.rs`, `local_contrast_window.rs` — and touched **no documentation file except CHANGELOG.md**.  Its replacement, the GPU pixel-shader pipeline (shipped v0.22.0, commit `4e5c68b`, expanded through v0.27.0), likewise never received documentation.  The result is that every major doc now describes a feature that does not exist, and does not describe the one that replaced it:

- `DESIGN.md` module layout lists all six deleted files (lines ~43–44, ~63–66).
- `DESIGN.md` “Local Contrast” section describes the removed `Shift+Cmd/Ctrl+L` floating dialog, rayon thread pool, OkLCh CPU pipeline, LC presets.  The actual binding today is `shift-cmd-g` → `ToggleGpuPipeline` (`src/app_keybindings.rs:17`).
- `DESIGN.md` “Performance” bullets: “Rayon-parallelized LC”, “Cached FloatMap” — both removed.
- `DESIGN.md` “Exit Handling”: “ESC closes Filter or LC window first” — the second floating window is now the GPU Pipeline window.
- `DESIGN.md` “Future Considerations” still lists “GPU-based filter pipeline (wgpu + naga)” as _future_ — it shipped 6 versions ago and is the app’s flagship feature.
- `README.md` §Local Contrast (~lines 124–135, 231–240) documents `Shift+Cmd+L` and the OkLCh/rayon design.
- `docs/COMPONENTS.md` documents `LocalContrastControls` and `LocalContrastWindow` (lines ~123, ~138).
- `docs/TESTING.md` counts tests for `float_map` (17), `local_contrast` (8), `lc_presets` (10) — none exist.
- `docs/TROUBLESHOOTING.md` §“Filter & Local Contrast Issues” and the ~144 MB float-map memory note.
- `TODO.md` Phase 17 says the cache-purge buttons live “in LC controls”; the surviving “Clear All Cached Frames” button now lives in the settings window (`src/components/settings_window.rs:1364`).

## Reproduction

No Rust test — documentation inspection:

1. `grep -rn "local_contrast\|Shift+Cmd+L\|float_map\|lc_presets" DESIGN.md README.md docs/` — many hits.
2. `ls src/utils/local_contrast.rs` — no such file.
3. `grep -n "shift-cmd-l" src/app_keybindings.rs` — no such binding; `shift-cmd-g` → `ToggleGpuPipeline` exists instead.

## Suggested Fix (docs only)

1. Rewrite `DESIGN.md`: drop the six deleted files from the module layout; add `src/gpu/` (mod, device, pipeline, unified, cache, readback) and `src/utils/gpu_presets.rs`, `src/utils/frame_cache.rs`, `src/components/gpu_pipeline_controls.rs`, `gpu_pipeline_window.rs`.  Replace the “Local Contrast” section with a “GPU Pipeline” section describing the shipped stages (LC, Document Contrast, Brightness/Contrast, Vibrance, Hue, Equalize — see `src/gpu/unified.rs` and `GpuPreset` in `src/utils/gpu_presets.rs` for the authoritative stage/parameter list), the `Shift+Cmd/Ctrl+G` binding, presets, per-frame animation processing, and the worker thread.  Fix the ESC and Performance sections.  Remove “GPU-based filter pipeline” from Future Considerations.
2. `README.md`: same replacement at user-documentation level.
3. `docs/COMPONENTS.md`: replace the two LC component sections with `GpuPipelineControls` / `GpuPipelineWindowView`.
4. `docs/TESTING.md`: regenerate the test-count tree from `cargo test` output.
5. `docs/TROUBLESHOOTING.md`: drop or rewrite the LC sections for the GPU pipeline.
6. `docs/local-contrast-spec.md`: do not delete (it documents the algorithm the GPU LC stage was ported from); add a banner at the top: superseded by the GPU pipeline as of 0.28.0, kept as historical design record.
7. `TODO.md` Phase 17: reword the two button locations to “Settings window” and see the separate per-image-purge bug (the “Clear Cache for This Image” feature no longer exists at all).

## Why This Fix

The code is the current truth: the CPU LC removal (0.28.0) and the GPU pipeline (0.22.0–0.27.0) were deliberate, versioned product decisions recorded in CHANGELOG.md.  The docs simply were not updated in those commits.  Any agent “fixing the code to match DESIGN.md” here would be re-adding a deliberately deleted feature — this bug exists to prevent exactly that mistake.

## Git Evidence

- `247605f` — removal commit; `--stat` shows 21 files changed, only CHANGELOG.md among docs.
- `4e5c68b` (v0.22.0) through `ab0fad3` (v0.27.0) — GPU pipeline feature commits; none touch DESIGN.md/README.md.
