//! Settings module for rpview-gpui
//! 
//! This module defines all user-configurable settings for the application.
//! Settings are serialized to JSON and saved in the platform-appropriate config directory.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::app_state::SortMode;

/// Main application settings container
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AppSettings {
    pub viewer_behavior: ViewerBehavior,
    pub performance: Performance,
    pub keyboard_mouse: KeyboardMouse,
    pub file_operations: FileOperations,
    pub appearance: Appearance,
    pub filters: Filters,
    pub sort_navigation: SortNavigation,
    pub external_tools: ExternalTools,
}

/// Viewer behavior settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewerBehavior {
    /// Default zoom mode when loading images
    pub default_zoom_mode: ZoomMode,
    /// Whether to remember per-image state (zoom, pan, filters)
    pub remember_per_image_state: bool,
    /// Maximum number of images to cache state for
    pub state_cache_size: usize,
    /// Whether animated images auto-play when loaded
    pub animation_auto_play: bool,
}

impl Default for ViewerBehavior {
    fn default() -> Self {
        Self {
            default_zoom_mode: ZoomMode::FitToWindow,
            remember_per_image_state: true,
            state_cache_size: 1000,
            animation_auto_play: true,
        }
    }
}

/// Default zoom mode options
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ZoomMode {
    /// Fit image to window
    FitToWindow,
    /// Display at 100% (actual pixels)
    OneHundredPercent,
}

/// Performance settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Performance {
    /// Whether to preload adjacent images for fast navigation
    pub preload_adjacent_images: bool,
    /// Number of background threads for filter processing
    pub filter_processing_threads: usize,
    /// Maximum image dimension to load (neither width nor height can exceed this)
    pub max_image_dimension: u32,
}

impl Default for Performance {
    fn default() -> Self {
        Self {
            preload_adjacent_images: true,
            filter_processing_threads: 4,
            max_image_dimension: 17000,
        }
    }
}

/// Keyboard and mouse input settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyboardMouse {
    /// Pan speed for normal keyboard panning (pixels)
    pub pan_speed_normal: f32,
    /// Pan speed with Shift modifier (pixels)
    pub pan_speed_fast: f32,
    /// Pan speed with Cmd/Ctrl modifier (pixels)
    pub pan_speed_slow: f32,
    /// Scroll wheel zoom sensitivity (zoom factor per notch)
    pub scroll_wheel_sensitivity: f32,
    /// Z-drag zoom sensitivity (percentage per pixel)
    pub z_drag_sensitivity: f32,
    /// Whether spacebar+drag panning uses acceleration
    pub spacebar_pan_accelerated: bool,
}

impl Default for KeyboardMouse {
    fn default() -> Self {
        Self {
            pan_speed_normal: 10.0,
            pan_speed_fast: 30.0,
            pan_speed_slow: 3.0,
            scroll_wheel_sensitivity: 1.1,
            z_drag_sensitivity: 0.01,
            spacebar_pan_accelerated: false,
        }
    }
}

/// File operations settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileOperations {
    /// Default directory for save operations
    pub default_save_directory: Option<PathBuf>,
    /// Default image format when saving filtered images
    pub default_save_format: SaveFormat,
    /// Whether to permanently save filtered image cache
    pub auto_save_filtered_cache: bool,
    /// Whether to remember last used directory in file dialogs
    pub remember_last_directory: bool,
}

impl Default for FileOperations {
    fn default() -> Self {
        Self {
            default_save_directory: None,
            default_save_format: SaveFormat::SameAsLoaded,
            auto_save_filtered_cache: false,
            remember_last_directory: true,
        }
    }
}

/// Image save format options
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SaveFormat {
    /// Save in same format as loaded image
    SameAsLoaded,
    Png,
    Jpeg,
    Bmp,
    Tiff,
    Webp,
}

impl SaveFormat {
    /// Get display name for the save format
    pub fn display_name(&self) -> &'static str {
        match self {
            SaveFormat::SameAsLoaded => "Same as loaded image",
            SaveFormat::Png => "PNG",
            SaveFormat::Jpeg => "JPEG",
            SaveFormat::Bmp => "BMP",
            SaveFormat::Tiff => "TIFF",
            SaveFormat::Webp => "WEBP",
        }
    }

    /// Get all save format options
    pub fn all() -> Vec<Self> {
        vec![
            Self::SameAsLoaded,
            Self::Png,
            Self::Jpeg,
            Self::Bmp,
            Self::Tiff,
            Self::Webp,
        ]
    }
}

/// Appearance settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Appearance {
    /// Background color for image viewer (RGB)
    pub background_color: [u8; 3],
    /// Alpha value for overlay backgrounds (0-255)
    pub overlay_transparency: u8,
    /// Font size multiplier for overlays (0.5 - 8.0)
    pub font_size_scale: f32,
    /// Window title format template
    pub window_title_format: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            background_color: [0x1e, 0x1e, 0x1e], // #1e1e1e
            overlay_transparency: 204, // ~80% opacity
            font_size_scale: 1.0,
            window_title_format: "{filename} ({index}/{total})".to_string(),
        }
    }
}

/// Filter settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Filters {
    /// Default brightness value when resetting
    pub default_brightness: f32,
    /// Default contrast value when resetting
    pub default_contrast: f32,
    /// Default gamma value when resetting
    pub default_gamma: f32,
    /// Whether to remember filter state per-image
    pub remember_filter_state: bool,
    /// Saved filter presets
    pub filter_presets: Vec<FilterPreset>,
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            default_brightness: 0.0,
            default_contrast: 0.0,
            default_gamma: 1.0,
            remember_filter_state: true,
            filter_presets: Vec::new(),
        }
    }
}

/// Saved filter preset
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterPreset {
    pub name: String,
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
}

/// Sort and navigation settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SortNavigation {
    /// Default sort mode on startup
    pub default_sort_mode: SortModeWrapper,
    /// Whether navigation wraps around (last -> first)
    pub wrap_navigation: bool,
    /// Whether to show image counter in window title
    pub show_image_counter: bool,
}

impl Default for SortNavigation {
    fn default() -> Self {
        Self {
            default_sort_mode: SortModeWrapper::Alphabetical,
            wrap_navigation: true,
            show_image_counter: true,
        }
    }
}

/// Wrapper for SortMode to add Serialize/Deserialize
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SortModeWrapper {
    Alphabetical,
    ModifiedDate,
}

impl From<SortModeWrapper> for SortMode {
    fn from(wrapper: SortModeWrapper) -> Self {
        match wrapper {
            SortModeWrapper::Alphabetical => SortMode::Alphabetical,
            SortModeWrapper::ModifiedDate => SortMode::ModifiedDate,
        }
    }
}

impl From<SortMode> for SortModeWrapper {
    fn from(mode: SortMode) -> Self {
        match mode {
            SortMode::Alphabetical => SortModeWrapper::Alphabetical,
            SortMode::ModifiedDate => SortModeWrapper::ModifiedDate,
        }
    }
}

/// External tools settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalTools {
    /// List of external image viewers to try (in order)
    pub external_viewers: Vec<ViewerConfig>,
    /// External image editor configuration
    pub external_editor: Option<ViewerConfig>,
    /// Whether to show "Show in Finder/Explorer" in menu
    pub enable_file_manager_integration: bool,
}

impl Default for ExternalTools {
    fn default() -> Self {
        Self {
            external_viewers: Self::default_viewers(),
            external_editor: None,
            enable_file_manager_integration: true,
        }
    }
}

impl ExternalTools {
    /// Get platform-specific default external viewers
    fn default_viewers() -> Vec<ViewerConfig> {
        #[cfg(target_os = "macos")]
        {
            vec![ViewerConfig {
                name: "Preview".to_string(),
                command: "open".to_string(),
                args: vec!["-a".to_string(), "Preview".to_string(), "{path}".to_string()],
                enabled: true,
            }]
        }
        
        #[cfg(target_os = "windows")]
        {
            vec![
                ViewerConfig {
                    name: "Photos".to_string(),
                    command: "cmd".to_string(),
                    args: vec!["/C".to_string(), "start".to_string(), "ms-photos:".to_string(), "{path}".to_string()],
                    enabled: true,
                },
                ViewerConfig {
                    name: "Windows Photo Viewer".to_string(),
                    command: "rundll32.exe".to_string(),
                    args: vec![
                        "C:\\Program Files\\Windows Photo Viewer\\PhotoViewer.dll,ImageView_Fullscreen".to_string(),
                        "{path}".to_string()
                    ],
                    enabled: true,
                },
            ]
        }
        
        #[cfg(target_os = "linux")]
        {
            vec![
                ViewerConfig { name: "Eye of GNOME".to_string(), command: "eog".to_string(), args: vec!["{path}".to_string()], enabled: true },
                ViewerConfig { name: "Xviewer".to_string(), command: "xviewer".to_string(), args: vec!["{path}".to_string()], enabled: true },
                ViewerConfig { name: "Gwenview".to_string(), command: "gwenview".to_string(), args: vec!["{path}".to_string()], enabled: true },
                ViewerConfig { name: "feh".to_string(), command: "feh".to_string(), args: vec!["{path}".to_string()], enabled: true },
                ViewerConfig { name: "Default Viewer".to_string(), command: "xdg-open".to_string(), args: vec!["{path}".to_string()], enabled: true },
            ]
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            vec![]
        }
    }
}

/// External viewer configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewerConfig {
    /// Display name for the viewer
    pub name: String,
    /// Command to execute
    pub command: String,
    /// Arguments to pass (use {path} as placeholder for image path)
    pub args: Vec<String>,
    /// Whether this viewer is enabled
    pub enabled: bool,
}
