//! Preset save/load for local-contrast parameters.
//!
//! Presets are individual JSON files in `{settings_dir}/lc-presets/`.

use crate::utils::local_contrast::Parameters;
use std::path::PathBuf;

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
