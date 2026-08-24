# Preset name sanitization silently collides distinct names, and the dropdown shows the sanitized filename instead of the saved name

**Severity:** low — silent overwrite of one preset by another with a different name
**Type:** CODE bug
**Where:** `src/utils/gpu_presets.rs:120-132` (`preset_path` maps every char outside `[alnum]-_ ` to `_`); `:135-155` (`list_preset_names` returns sanitized file stems); `src/components/gpu_pipeline_controls.rs:435-447,457-463` (dropdown selection compares the raw typed name against sanitized list entries)
**Verified:** both functions read; collision is mechanical

## Description

“Sunset: Warm” and “Sunset?  Warm” both sanitize to `Sunset_ Warm.json` — saving the second silently overwrites the first, with no warning (distinct-name collision, unlike the deliberate same-name overwrite).  Separately, the raw name is not stored in the JSON payload and `list_preset_names` returns the sanitized stem, so after saving “Café” the dropdown lists “Caf_”, and `with_selected_value("Café")` can never match a list entry — the just-saved preset can’t show as selected.

## Reproduction (Rust test)

```rust
#[test]
fn distinct_names_must_not_collide() {
    let p1 = sample_preset_with(0.25);
    let p2 = sample_preset_with(2.0);
    save_preset("a:b", &p1).unwrap();
    save_preset("a?b", &p2).unwrap();
    // Today this passes — proving the silent overwrite:
    assert_ne!(load_preset("a:b").unwrap().resize_factor, p2.resize_factor);  // FAILS today
}
```

## Suggested Fix

Store the raw name inside the JSON (`name: String` field on `GpuPreset`, `#[serde(default)]` for legacy files) and have `list_preset_names` read it (falling back to the stem for legacy files).  For the file path, keep sanitizing but disambiguate collisions — e.g. append a short FNV hash of the raw name when sanitization changed it (`Sunset_ Warm-3fa2.json`).  The dropdown then round-trips exactly what the user typed.

## Why This Fix

Identity moves from the lossy filename to a lossless stored field; the filename becomes a storage detail that no longer needs to be unique per raw name on its own.
