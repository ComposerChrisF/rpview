# One missing/mistyped field in settings.json discards ALL user settings — no `#[serde(default)]` at the section level — and the test claiming otherwise is vacuous

**Severity:** high — all-or-nothing deserialization turns a one-key typo into a full reset
**Type:** CODE bug (also contradicts docs/SETTINGS_DESIGN.md, which requires per-field defaults), plus a lying test
**Where:** `src/state/settings.rs:11-21` (`AppSettings` — no `#[serde(default)]` on the container or its 8 section fields; most section structs likewise lack field-level defaults); `src/utils/settings_io.rs:282-300` (`test_partial_json_uses_serde_defaults_for_top_level` — vacuous)
**Verified:** struct definitions read; serde semantics are mechanical (missing field without `default` → error); the test provably serializes a complete struct before loading it

## Description

`AppSettings` and its sections derive `Deserialize` without `#[serde(default)]` (except for a handful of individually-annotated late additions like `pan_direction_mode` and the window-bounds fields).  Consequences:

- A `settings.json` missing any top-level section, or any non-defaulted field inside a section, fails to parse **in toto** → the corrupt-file path runs → the user gets full defaults.  Every other setting they had is gone (in memory immediately; on disk as soon as any save fires — the B key, moving a floating window, closing settings).
- `docs/SETTINGS.md` actively encourages hand-editing, and its own “Complete Example” doesn’t parse against the current schema (documented separately) — following the docs triggers exactly this reset.
- A settings file written by a _newer_ rpview (new enum variant, new section) read by an older binary resets everything rather than degrading gracefully.
- `docs/SETTINGS_DESIGN.md` explicitly requires the opposite: “All settings optional with sensible defaults”, “backwards-compatible (new fields added with defaults)”, “validated on load (use defaults for invalid values)”.

The unit test `test_partial_json_uses_serde_defaults_for_top_level` claims to cover this (“missing sections get filled in by serde”) but saves a **complete** serialized struct and reloads it — nothing is ever missing, so it passes vacuously.  Written honestly, it fails today.

## Reproduction (Rust test)

```rust
#[test]
fn partial_settings_json_keeps_present_values_and_defaults_the_rest() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, r#"{"viewer_behavior":{"default_zoom_mode":"FitToWindow",
        "remember_per_image_state":true,"state_cache_size":42,"animation_auto_play":true}}"#).unwrap();

    let loaded = load_settings_from_path(&path);
    assert_eq!(loaded.viewer_behavior.state_cache_size, 42);   // FAILS today: whole parse errors → defaults (1000)
    assert_eq!(loaded.performance, Performance::default());
}
```

## Suggested Fix

1. Add `#[serde(default)]` to every section field of `AppSettings` (or on the container) **and** to every section struct (`#[serde(default)]` on the struct makes each missing field fall back to that struct’s `Default`).  All sections already implement `Default`, so this is annotation-only.
2. Replace the vacuous test with the honest one above, plus a per-section variant (a section present but missing one field).
3. (Follow-on, per SETTINGS_DESIGN.md) clamp out-of-range numeric values on load — today `"font_size_scale": 0.0` loads unchecked; the UI clamps only stepper interaction.  This can be a small `fn validate(&mut self)` called from `load_settings_from_path`.

## Why This Fix

With per-field defaults, an unknown or missing key degrades to that one setting’s default instead of nuking the file — which is what SETTINGS_DESIGN.md specified from the start, and what makes hand-editing (a documented workflow) survivable.  The honest test then pins the contract so a future struct refactor can’t silently reintroduce all-or-nothing parsing.
