//! Preset save/load for the unified GPU pipeline.
//!
//! Presets are individual JSON files in `{settings_dir}/gpu-presets/`.  The
//! serialised payload is a `GpuPreset` capturing the full panel state
//! (per-stage enable flags + every slider value + resize factor) so a load
//! restores the user's exact UI configuration — including slider values
//! for stages that happen to be disabled at save time.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Sentinel label displayed in the preset dropdown when the current
/// parameter set doesn't match any saved preset.  Reserved — the user
/// can't save a preset under this name.
pub const CUSTOM_PRESET_LABEL: &str = "(Custom)";

/// Full GPU pipeline panel state captured for save/load.  Slider values
/// follow the on-panel scale (`lc_radius_t` is the 0..1 normalized knob
/// position, `hue_value` is 0..1 with 0.5 = no rotation, etc.) — same
/// units the controls already use, so apply is a direct write back.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpuPreset {
    pub resize_factor: f32,

    pub lc_enabled: bool,
    pub lc_radius_t: f32,
    pub lc_strength: f32,
    pub lc_midpoint: f32,

    pub bc_enabled: bool,
    pub bc_brightness: f32,
    pub bc_contrast: f32,

    pub vibrance_enabled: bool,
    pub vibrance_amount: f32,
    pub vibrance_saturation: f32,

    pub hue_enabled: bool,
    pub hue_value: f32,

    /// Equalize stage was added in v0.22.9.  `#[serde(default)]` lets older
    /// preset JSON (without these keys) load cleanly — the stage simply
    /// stays disabled at its default Amount.
    #[serde(default)]
    pub equalize_enabled: bool,
    #[serde(default = "default_equalize_amount")]
    pub equalize_amount: f32,
    /// Tonally-weighted equalization (v0.24.0).  Default 0 so older presets
    /// load unchanged.
    #[serde(default)]
    pub equalize_shadows: f32,
    #[serde(default)]
    pub equalize_highlights: f32,
}

fn default_equalize_amount() -> f32 {
    0.5
}

fn presets_dir() -> PathBuf {
    let settings_path = crate::utils::settings_io::get_settings_path();
    settings_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("gpu-presets")
}

fn preset_path(name: &str) -> PathBuf {
    let safe_name: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    presets_dir().join(format!("{}.json", safe_name))
}

/// Return a sorted list of all saved preset names (without the `.json` extension).
pub fn list_preset_names() -> Vec<String> {
    let dir = presets_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| {
            let e = e.ok()?;
            let path = e.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names
}

/// Load a preset by name, returning `None` if the file doesn't exist or can't be parsed.
pub fn load_preset(name: &str) -> Option<GpuPreset> {
    let path = preset_path(name);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Save a preset to disk.  Creates the presets directory if needed.
pub fn save_preset(name: &str, preset: &GpuPreset) -> Result<(), String> {
    let dir = presets_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    let json = serde_json::to_string_pretty(preset).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(preset_path(name), json).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

/// Delete a preset file.  Succeeds silently if the preset doesn't exist.
pub fn delete_preset(name: &str) -> Result<(), String> {
    let path = preset_path(name);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("delete: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique test preset prefix to avoid collisions with user data.
    const TEST_PREFIX: &str = "__rpview_test_gpu_preset_";

    fn test_name(suffix: &str) -> String {
        format!("{TEST_PREFIX}{suffix}")
    }

    fn cleanup(name: &str) {
        let _ = delete_preset(name);
    }

    fn sample_preset() -> GpuPreset {
        GpuPreset {
            resize_factor: 0.5,
            lc_enabled: true,
            lc_radius_t: 0.42,
            lc_strength: 0.7,
            lc_midpoint: 0.5,
            bc_enabled: false,
            bc_brightness: 0.0,
            bc_contrast: 0.0,
            vibrance_enabled: true,
            vibrance_amount: 0.3,
            vibrance_saturation: 0.0,
            hue_enabled: false,
            hue_value: 0.5,
            equalize_enabled: false,
            equalize_amount: 0.5,
            equalize_shadows: 0.0,
            equalize_highlights: 0.0,
        }
    }

    #[test]
    fn sanitizes_special_characters_in_name() {
        let name = test_name("special!@#chars");
        save_preset(&name, &sample_preset()).unwrap();
        let loaded = load_preset(&name);
        assert!(loaded.is_some(), "should load preset with sanitized name");
        cleanup(&name);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let name = test_name("roundtrip");
        let preset = sample_preset();
        save_preset(&name, &preset).unwrap();
        let loaded = load_preset(&name).expect("should load saved preset");
        assert_eq!(loaded, preset);
        cleanup(&name);
    }

    #[test]
    fn save_overwrites_existing() {
        let name = test_name("overwrite");
        let mut v1 = sample_preset();
        v1.resize_factor = 0.25;
        let mut v2 = sample_preset();
        v2.resize_factor = 2.0;
        save_preset(&name, &v1).unwrap();
        save_preset(&name, &v2).unwrap();
        let loaded = load_preset(&name).unwrap();
        assert_eq!(loaded.resize_factor, 2.0);
        cleanup(&name);
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let result = load_preset("__rpview_does_not_exist_ever__");
        assert!(result.is_none());
    }

    #[test]
    fn list_includes_saved_preset() {
        let name = test_name("list_check");
        save_preset(&name, &sample_preset()).unwrap();
        let names = list_preset_names();
        assert!(
            names.iter().any(|n| n == &name),
            "expected {name} in {names:?}"
        );
        cleanup(&name);
    }

    #[test]
    fn list_returns_sorted_names() {
        let a = test_name("aaa_sort");
        let b = test_name("zzz_sort");
        save_preset(&b, &sample_preset()).unwrap();
        save_preset(&a, &sample_preset()).unwrap();
        let names = list_preset_names();
        let idx_a = names.iter().position(|n| n == &a);
        let idx_b = names.iter().position(|n| n == &b);
        assert!(idx_a < idx_b, "expected {a} before {b} in sorted list");
        cleanup(&a);
        cleanup(&b);
    }

    #[test]
    fn delete_removes_preset() {
        let name = test_name("delete_me");
        save_preset(&name, &sample_preset()).unwrap();
        assert!(load_preset(&name).is_some());
        delete_preset(&name).unwrap();
        assert!(load_preset(&name).is_none());
    }

    #[test]
    fn delete_nonexistent_succeeds() {
        let result = delete_preset("__rpview_never_existed__");
        assert!(result.is_ok());
    }

    #[test]
    fn custom_preset_label_is_reserved_name() {
        assert_eq!(CUSTOM_PRESET_LABEL, "(Custom)");
    }

    /// JSON written before v0.22.9 (no `equalize_*` keys) must still
    /// deserialise — the `#[serde(default)]` attributes carry the load.
    #[test]
    fn legacy_preset_without_equalize_fields_loads() {
        let legacy_json = r#"{
            "resize_factor": 1.0,
            "lc_enabled": false,
            "lc_radius_t": 0.5,
            "lc_strength": 0.5,
            "lc_midpoint": 0.5,
            "bc_enabled": false,
            "bc_brightness": 0.0,
            "bc_contrast": 0.0,
            "vibrance_enabled": false,
            "vibrance_amount": 0.5,
            "vibrance_saturation": 0.0,
            "hue_enabled": false,
            "hue_value": 0.5
        }"#;
        let parsed: GpuPreset =
            serde_json::from_str(legacy_json).expect("legacy preset must parse");
        assert!(!parsed.equalize_enabled);
        assert!((parsed.equalize_amount - 0.5).abs() < f32::EPSILON);
        // The tonally-weighted equalize fields are absent from legacy JSON and
        // default to 0.
        assert_eq!(parsed.equalize_shadows, 0.0);
        assert_eq!(parsed.equalize_highlights, 0.0);
    }

    #[test]
    fn alphanumeric_hyphens_underscores_spaces_preserved() {
        let name = test_name("My Preset-Name_v2");
        save_preset(&name, &sample_preset()).unwrap();
        let loaded = load_preset(&name);
        assert!(loaded.is_some());
        cleanup(&name);
    }
}
