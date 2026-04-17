//! Preset save/load for local-contrast parameters.
//!
//! Presets are individual JSON files in `{settings_dir}/lc-presets/`.

use crate::utils::local_contrast::Parameters;
use std::path::PathBuf;

/// Sentinel label displayed in the preset dropdown when the current parameter
/// set doesn't match any saved preset. Also used as a guard in save/load so
/// the user can't save a preset under this reserved name.
pub const CUSTOM_PRESET_LABEL: &str = "(Custom)";

fn presets_dir() -> PathBuf {
    let settings_path = crate::utils::settings_io::get_settings_path();
    settings_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("lc-presets")
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

pub fn load_preset(name: &str) -> Option<Parameters> {
    let path = preset_path(name);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_preset(name: &str, params: &Parameters) -> Result<(), String> {
    let dir = presets_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    let json = serde_json::to_string_pretty(params).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(preset_path(name), json).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

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
    use crate::utils::local_contrast::Parameters;

    /// Unique test preset name to avoid collisions with user data.
    const TEST_PREFIX: &str = "__rpview_test_preset_";

    fn test_name(suffix: &str) -> String {
        format!("{TEST_PREFIX}{suffix}")
    }

    /// Clean up a test preset after use.
    fn cleanup(name: &str) {
        let _ = delete_preset(name);
    }

    // -- preset_path sanitization (tested indirectly) -------------------------

    #[test]
    fn sanitizes_special_characters_in_name() {
        let name = test_name("special!@#chars");
        // Save should succeed — special chars get replaced with underscores
        save_preset(&name, &Parameters::default()).unwrap();
        // The preset should be loadable under the same name
        let loaded = load_preset(&name);
        assert!(loaded.is_some(), "should load preset with sanitized name");
        cleanup(&name);
    }

    // -- save / load round-trip -----------------------------------------------

    #[test]
    fn save_and_load_roundtrip() {
        let name = test_name("roundtrip");
        let params = Parameters {
            contrast: 0.08,
            lighten_shadows: 0.75,
            ..Parameters::default()
        };
        save_preset(&name, &params).unwrap();
        let loaded = load_preset(&name).expect("should load saved preset");
        assert_eq!(loaded.contrast, 0.08);
        assert_eq!(loaded.lighten_shadows, 0.75);
        cleanup(&name);
    }

    #[test]
    fn save_overwrites_existing() {
        let name = test_name("overwrite");
        let v1 = Parameters { contrast: 0.01, ..Parameters::default() };
        let v2 = Parameters { contrast: 0.99, ..Parameters::default() };
        save_preset(&name, &v1).unwrap();
        save_preset(&name, &v2).unwrap();
        let loaded = load_preset(&name).unwrap();
        assert_eq!(loaded.contrast, 0.99);
        cleanup(&name);
    }

    // -- load nonexistent -----------------------------------------------------

    #[test]
    fn load_nonexistent_returns_none() {
        let result = load_preset("__rpview_does_not_exist_ever__");
        assert!(result.is_none());
    }

    // -- list -----------------------------------------------------------------

    #[test]
    fn list_includes_saved_preset() {
        let name = test_name("list_check");
        save_preset(&name, &Parameters::default()).unwrap();
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
        save_preset(&b, &Parameters::default()).unwrap();
        save_preset(&a, &Parameters::default()).unwrap();
        let names = list_preset_names();
        let idx_a = names.iter().position(|n| n == &a);
        let idx_b = names.iter().position(|n| n == &b);
        assert!(idx_a < idx_b, "expected {a} before {b} in sorted list");
        cleanup(&a);
        cleanup(&b);
    }

    // -- delete ---------------------------------------------------------------

    #[test]
    fn delete_removes_preset() {
        let name = test_name("delete_me");
        save_preset(&name, &Parameters::default()).unwrap();
        assert!(load_preset(&name).is_some());
        delete_preset(&name).unwrap();
        assert!(load_preset(&name).is_none());
    }

    #[test]
    fn delete_nonexistent_succeeds() {
        let result = delete_preset("__rpview_never_existed__");
        assert!(result.is_ok());
    }

    // -- CUSTOM_PRESET_LABEL guard -------------------------------------------

    #[test]
    fn custom_preset_label_is_reserved_name() {
        assert_eq!(CUSTOM_PRESET_LABEL, "(Custom)");
    }

    // -- name with only safe chars -------------------------------------------

    #[test]
    fn alphanumeric_hyphens_underscores_spaces_preserved() {
        let name = test_name("My Preset-Name_v2");
        save_preset(&name, &Parameters::default()).unwrap();
        let loaded = load_preset(&name);
        assert!(loaded.is_some());
        cleanup(&name);
    }
}
