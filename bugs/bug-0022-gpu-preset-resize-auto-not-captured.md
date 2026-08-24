# Presets do not capture Auto-resize state — saving in Auto mode records the stale manual factor

**Severity:** low — round-trip fidelity gap vs the module’s own “restores the user’s exact UI configuration” claim
**Type:** BOTH (code omission; module doc overpromises)
**Where:** `src/utils/gpu_presets.rs:22-84` (`GpuPreset` — no `resize_auto` field); `src/components/gpu_pipeline_controls.rs:349-351` (`to_preset` stores `self.resize_factor`, the last manual choice, not the effective auto factor), `:385-387` (`apply_preset` sets `resize_auto = false`)
**Verified:** `to_preset` read in full (quoted above at :345-380); no `resize_auto` in the struct

## Description

With Auto on showing “Auto (0.50×)”, `to_preset` saves `resize_factor` = the last _manual_ value (possibly 1.0) — neither the effective factor nor the Auto flag.  Loading that preset turns Auto off with a factor the user never saw.

## Reproduction (Rust test)

Set `resize_auto = true`, `resize_factor = 1.0`, image dims large enough that the effective auto factor is 0.5; `let p = to_preset()`; assert `p` records either Auto or 0.5 — today it records 1.0 with no Auto flag.

## Suggested Fix

Add `#[serde(default)] pub resize_auto: bool` to `GpuPreset`; `to_preset` stores the flag; `apply_preset` restores it (legacy presets default to `false`, preserving current behavior).  Update the module doc if Chris instead prefers presets to always pin a concrete factor — in that case `to_preset` should store `effective_resize_factor()` and the doc’s “exact UI configuration” claim should be softened.

## Why This Fix

Round-trips what the user actually configured; the serde default keeps old preset files loading unchanged, following the struct’s existing pattern for late-added fields.
