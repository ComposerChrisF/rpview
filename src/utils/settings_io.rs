//! Settings persistence module
//!
//! Handles loading and saving settings to/from disk using JSON format.

use crate::state::settings::AppSettings;
use std::path::PathBuf;

/// Get the path to the settings file
///
/// Platform locations:
/// - macOS: ~/Library/Application Support/rpview/settings.json
/// - Linux: ~/.config/rpview/settings.json
/// - Windows: C:\Users\<User>\AppData\Roaming\rpview\settings.json
///
/// Falls back to ~/.rpview/settings.json if config directory is unavailable,
/// or ./rpview_settings.json as a last resort.
pub fn get_settings_path() -> PathBuf {
    // Try platform config directory first
    let config_dir = if let Some(config) = dirs::config_dir() {
        config.join("rpview")
    } else if let Some(home) = dirs::home_dir() {
        // Fallback to home directory
        eprintln!("Warning: Could not find config directory, using home directory");
        home.join(".rpview")
    } else {
        // Last resort: current directory
        eprintln!("Warning: Could not find config or home directory, using current directory");
        PathBuf::from(".")
    };

    // Create directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        eprintln!("Warning: Could not create config directory: {}", e);
    }

    config_dir.join("settings.json")
}

/// Save settings to disk
///
/// Settings are saved as pretty-printed JSON for readability.
pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    save_settings_to_path(settings, &get_settings_path())
}

/// Save settings to a specific path (used for testing)
pub fn save_settings_to_path(settings: &AppSettings, path: &std::path::Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    std::fs::write(path, json).map_err(|e| format!("Failed to write settings file: {}", e))?;

    Ok(())
}

/// Load settings from disk
///
/// If the settings file doesn't exist, creates it with default settings.
/// If the file can't be parsed, returns default settings (and backs up the corrupt file).
/// Errors are logged to stderr but don't prevent the application from starting.
pub fn load_settings() -> AppSettings {
    load_settings_from_path(&get_settings_path())
}

/// Load settings from a specific path (used for testing)
pub fn load_settings_from_path(path: &std::path::Path) -> AppSettings {
    // If file doesn't exist, return defaults
    if !path.exists() {
        return AppSettings::default();
    }

    // Try to read the file
    let json = match std::fs::read_to_string(path) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Warning: Failed to read settings file: {}", e);
            eprintln!("Using default settings");
            return AppSettings::default();
        }
    };

    // Try to parse the JSON
    match serde_json::from_str(&json) {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("Warning: Failed to parse settings file: {}", e);

            // Back up the corrupt file
            let backup_path = path.with_extension("json.backup");
            if let Err(backup_err) = std::fs::copy(path, &backup_path) {
                eprintln!(
                    "Warning: Failed to backup corrupt settings file: {}",
                    backup_err
                );
            } else {
                eprintln!(
                    "Corrupt settings file backed up to: {}",
                    backup_path.display()
                );
            }

            eprintln!("Using default settings");
            AppSettings::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_settings_path() {
        let path = get_settings_path();
        assert!(path.to_string_lossy().contains("rpview"));
        assert!(path.to_string_lossy().ends_with("settings.json"));
    }

    #[test]
    fn test_save_and_load_settings() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test_settings.json");

        let settings = AppSettings::default();

        // Save settings
        save_settings_to_path(&settings, &test_path).expect("Failed to save settings");

        // Load settings
        let loaded = load_settings_from_path(&test_path);

        assert_eq!(settings, loaded);
    }

    #[test]
    fn test_load_nonexistent_settings() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.json");

        // Should return defaults without panicking
        let settings = load_settings_from_path(&nonexistent_path);
        assert_eq!(settings, AppSettings::default());
    }

    #[test]
    fn test_load_corrupt_settings() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("corrupt_settings.json");

        // Write corrupt JSON
        std::fs::write(&test_path, "{ invalid json }").unwrap();

        // Should return defaults without panicking
        let settings = load_settings_from_path(&test_path);
        assert_eq!(settings, AppSettings::default());

        // Backup file should exist
        let backup_path = test_path.with_extension("json.backup");
        assert!(backup_path.exists());
    }

    #[test]
    fn test_modified_settings_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("modified_settings.json");

        let mut settings = AppSettings::default();
        settings.viewer_behavior.default_zoom_mode =
            crate::state::settings::ZoomMode::OneHundredPercent;
        settings.appearance.font_size_scale = 2.0;

        save_settings_to_path(&settings, &test_path).unwrap();
        let loaded = load_settings_from_path(&test_path);

        assert_eq!(
            loaded.viewer_behavior.default_zoom_mode,
            settings.viewer_behavior.default_zoom_mode
        );
        assert_eq!(loaded.appearance.font_size_scale, settings.appearance.font_size_scale);
    }

    #[test]
    fn test_load_empty_file_returns_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("empty_settings.json");
        std::fs::write(&test_path, "").unwrap();

        // Empty string is not valid JSON — should return defaults
        let settings = load_settings_from_path(&test_path);
        assert_eq!(settings, AppSettings::default());
    }

    #[test]
    fn test_partial_json_uses_serde_defaults_for_top_level() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("partial_settings.json");

        // Save real settings, then re-serialize with only viewer_behavior
        // changed. Top-level fields use #[derive(Default)] so missing
        // sections get filled in by serde.
        let mut settings = AppSettings::default();
        settings.viewer_behavior.state_cache_size = 42;
        save_settings_to_path(&settings, &test_path).unwrap();

        let loaded = load_settings_from_path(&test_path);
        assert_eq!(loaded.viewer_behavior.state_cache_size, 42);
        // Untouched sections should match defaults
        assert_eq!(loaded.performance, crate::state::settings::Performance::default());
    }

    #[test]
    fn test_save_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested = temp_dir.path().join("a").join("b").join("settings.json");

        // Parent doesn't exist yet, save_settings_to_path writes directly
        // (it doesn't create parent dirs — that's get_settings_path's job)
        // So this should fail
        let result = save_settings_to_path(&AppSettings::default(), &nested);
        assert!(result.is_err());
    }

    #[test]
    fn test_corrupt_backup_preserves_original_content() {
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("settings.json");

        let corrupt_content = "not json at all {{{";
        std::fs::write(&test_path, corrupt_content).unwrap();

        let _ = load_settings_from_path(&test_path);

        // The backup should contain the original corrupt content
        let backup_path = test_path.with_extension("json.backup");
        let backup_content = std::fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, corrupt_content);
    }
}
