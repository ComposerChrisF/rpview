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
pub fn get_settings_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .expect("Could not find config directory")
        .join("rpview");
    
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
    let path = get_settings_path();
    
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;
    
    Ok(())
}

/// Load settings from disk
/// 
/// If the settings file doesn't exist, creates it with default settings.
/// If the file can't be parsed, returns default settings (and backs up the corrupt file).
/// Errors are logged to stderr but don't prevent the application from starting.
pub fn load_settings() -> AppSettings {
    let path = get_settings_path();
    
    // If file doesn't exist, create it with defaults
    if !path.exists() {
        let defaults = AppSettings::default();
        if let Err(e) = save_settings(&defaults) {
            eprintln!("Warning: Failed to save default settings: {}", e);
        }
        return defaults;
    }
    
    // Try to read the file
    let json = match std::fs::read_to_string(&path) {
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
            if let Err(backup_err) = std::fs::copy(&path, &backup_path) {
                eprintln!("Warning: Failed to backup corrupt settings file: {}", backup_err);
            } else {
                eprintln!("Corrupt settings file backed up to: {}", backup_path.display());
            }
            
            eprintln!("Using default settings");
            let defaults = AppSettings::default();
            
            // Overwrite corrupt file with defaults
            if let Err(e) = save_settings(&defaults) {
                eprintln!("Warning: Failed to save default settings: {}", e);
            }
            
            defaults
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_settings_path() {
        let path = get_settings_path();
        assert!(path.to_string_lossy().contains("rpview"));
        assert!(path.to_string_lossy().ends_with("settings.json"));
    }
    
    #[test]
    fn test_save_and_load_settings() {
        let settings = AppSettings::default();
        
        // Save settings
        save_settings(&settings).expect("Failed to save settings");
        
        // Load settings
        let loaded = load_settings();
        
        assert_eq!(settings, loaded);
    }
    
    #[test]
    fn test_load_nonexistent_settings() {
        // Should return defaults without panicking
        let settings = load_settings();
        assert_eq!(settings, AppSettings::default());
    }
}
