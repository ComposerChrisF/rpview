# Preset dropdown never reverts to “(Custom)” — edited or reset parameters keep displaying the loaded preset’s name, leaving the Del button armed

**Severity:** low — misleading UI; Del can delete a preset whose values are no longer shown
**Type:** BOTH — code violates its own documented sentinel contract (`CUSTOM_PRESET_LABEL` doc)
**Where:** `src/utils/gpu_presets.rs:12-15` (sentinel contract: “displayed … when the current parameter set doesn’t match any saved preset”); `src/components/gpu_pipeline_controls.rs` — `current_preset` is only ever set (`:336,341,445`) or taken by delete (`:450`); no slider-change or `reset_all` path clears it (`:539-583` resets sliders but not `current_preset`)
**Verified:** every `current_preset` assignment site enumerated by grep

## Description

After loading a preset, any slider drag leaves the dropdown naming that preset over parameters that no longer match it.  `reset_all` resets every slider but leaves the selection and the armed Del button — so “Reset all” followed by Del deletes a preset whose contents the panel no longer displays.

## Reproduction

Manual: load preset → drag any slider → dropdown still names the preset.  Or: load preset → “Reset all” → Del is still enabled and deletes it.

## Suggested Fix

Clear `current_preset = None` (dropdown → `CUSTOM_PRESET_LABEL`) whenever a parameter change does not come from `apply_preset` — the natural choke point is the `ParametersChanged` emission (`emit_change`, or the shared slider-change handler) and `reset_all`.  A flag `applying_preset: bool` set around `apply_preset` avoids clearing during the load itself.

## Why This Fix

Restores the documented sentinel semantics: the dropdown names a preset exactly when the panel state equals it, and Del is armed only for a preset the user is actually looking at.
