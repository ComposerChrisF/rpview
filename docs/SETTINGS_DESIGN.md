# Settings System Design

This document outlines the design and implementation plan for a comprehensive settings system in rpview-gpui.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Settings Categories](#settings-categories)
4. [UI Design](#ui-design)
5. [Persistence](#persistence)
6. [Implementation Steps](#implementation-steps)

---

## Overview

The settings system will provide users with fine-grained control over rpview's behavior, appearance, and performance. Settings will be accessible through a modal overlay window (similar to the existing Help/Debug overlays) and will persist across sessions.

### Key Goals

- **User-configurable behavior**: Allow customization of defaults and preferences
- **Platform-specific options**: Support external viewer configuration per platform
- **Persistent storage**: Save/load settings from disk automatically
- **Intuitive UI**: Use existing overlay patterns and components
- **Non-breaking changes**: All settings optional with sensible defaults

---

## Architecture

### Core Components

```
Settings System Architecture:
├── AppSettings struct (state/settings.rs)
│   ├── ViewerBehavior
│   ├── Performance
│   ├── KeyboardMouse
│   ├── FileOperations
│   ├── Appearance
│   ├── Filters
│   ├── SortNavigation
│   └── ExternalTools
├── SettingsWindow component (components/settings_window.rs)
│   ├── Tabbed/sectioned UI
│   ├── Various input widgets (sliders, checkboxes, text fields, dropdowns)
│   └── Apply/Cancel/Reset buttons
├── Settings persistence (utils/settings_io.rs)
│   ├── JSON serialization/deserialization
│   └── Config directory management
└── Integration points
    ├── App::render() - conditional rendering
    ├── ToggleSettings action
    └── Cmd+, keyboard binding
```

### Data Structures

```rust
// state/settings.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            viewer_behavior: ViewerBehavior::default(),
            performance: Performance::default(),
            keyboard_mouse: KeyboardMouse::default(),
            file_operations: FileOperations::default(),
            appearance: Appearance::default(),
            filters: Filters::default(),
            sort_navigation: SortNavigation::default(),
            external_tools: ExternalTools::default(),
        }
    }
}

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ZoomMode {
    FitToWindow,
    OneHundredPercent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Performance {
    /// Whether to preload adjacent images for fast navigation
    pub preload_adjacent_images: bool,
    /// Number of background threads for filter processing
    pub filter_processing_threads: usize,
    /// Maximum image dimensions to load (safety limit)
    pub max_image_dimensions: (u32, u32),
}

impl Default for Performance {
    fn default() -> Self {
        Self {
            preload_adjacent_images: true,
            filter_processing_threads: 4,
            max_image_dimensions: (16384, 16384), // 16K x 16K
        }
    }
}

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
            default_save_format: SaveFormat::Png,
            auto_save_filtered_cache: false,
            remember_last_directory: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SaveFormat {
    Png,
    Jpeg,
    Bmp,
    Tiff,
    Webp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Appearance {
    /// Background color for image viewer
    pub background_color: [u8; 3], // RGB
    /// Alpha value for overlay backgrounds (0-255)
    pub overlay_transparency: u8,
    /// Font size multiplier for overlays (0.5 - 2.0)
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterPreset {
    pub name: String,
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SortNavigation {
    /// Default sort mode on startup
    pub default_sort_mode: crate::state::SortMode,
    /// Whether navigation wraps around (last -> first)
    pub wrap_navigation: bool,
    /// Whether to show image counter in window title
    pub show_image_counter: bool,
}

impl Default for SortNavigation {
    fn default() -> Self {
        Self {
            default_sort_mode: crate::state::SortMode::Alphabetical,
            wrap_navigation: true,
            show_image_counter: true,
        }
    }
}

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
    }
}

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
```

---

## Settings Categories

### 1. Viewer Behavior

**Priority: High**

Controls how images are initially displayed and how state is managed.

- **Default zoom mode**: Fit-to-window or 100%
- **Remember per-image state**: Toggle the LRU cache on/off
- **State cache size**: Number of images to remember (current: 1000)
- **Animation auto-play**: Whether GIFs/WEBPs play automatically

**UI Elements:**
- Radio buttons for zoom mode
- Checkbox for state persistence
- Numeric input/slider for cache size (100-10000)
- Checkbox for animation auto-play

### 2. Performance

**Priority: Medium**

Performance tuning options for power users.

- **Preload adjacent images**: Toggle preloading (currently always on)
- **Filter processing threads**: Number of background threads
- **Max image dimensions**: Safety limit for loading huge images

**UI Elements:**
- Checkbox for preload toggle
- Numeric input for thread count (1-16)
- Two numeric inputs for width/height limits

### 3. Keyboard & Mouse

**Priority: High**

Customize input sensitivity and behavior.

- **Pan speeds**: Normal (10px), Fast/Shift (30px), Slow/Cmd (3px)
- **Scroll wheel sensitivity**: Zoom factor per notch (current: 1.1x)
- **Z-drag zoom sensitivity**: Percentage per pixel (current: 1%)
- **Spacebar-pan behavior**: 1:1 movement vs. accelerated

**UI Elements:**
- Three numeric inputs/sliders for pan speeds
- Slider for scroll sensitivity (1.01 - 2.0)
- Slider for Z-drag sensitivity (0.001 - 0.1)
- Checkbox for spacebar acceleration

### 4. File Operations

**Priority: Medium**

Configure save behavior and file dialog preferences.

- **Default save location**: Directory path or "Ask every time"
- **Default save format**: PNG, JPEG, BMP, TIFF, WEBP
- **Auto-save filtered cache**: Keep filtered versions permanently
- **Remember last directory**: For Open File dialog

**UI Elements:**
- File picker button with path display
- Dropdown for save format
- Checkbox for auto-save
- Checkbox for remember directory

### 5. Appearance

**Priority: Medium**

Visual customization options.

- **Background color**: RGB color picker
- **Overlay transparency**: Alpha value slider
- **Font size scale**: Multiplier for overlay text
- **Window title format**: Template string with variables

**UI Elements:**
- Color picker for background
- Slider for transparency (0-100%)
- Slider for font scale (50%-200%)
- Text input for title format with preview

### 6. Filters

**Priority: Low**

Filter-related preferences.

- **Default filter values**: Starting brightness/contrast/gamma
- **Remember filter state per-image**: Already implemented, make configurable
- **Filter presets**: Save named combinations (future enhancement)

**UI Elements:**
- Three sliders for default values (matching filter controls)
- Checkbox for per-image persistence
- List view for presets (add/remove/apply)

### 7. Sort & Navigation

**Priority: Low**

Navigation behavior settings.

- **Default sort mode**: Alphabetical or Modified Date
- **Wrap navigation**: Whether left/right wrap around
- **Show image counter**: In window title

**UI Elements:**
- Radio buttons for sort mode
- Checkbox for wrap navigation
- Checkbox for image counter

### 8. External Tools

**Priority: High** (Original request)

Configure external applications for opening images.

- **External viewers**: Ordered list with command, args, enable/disable
- **External editor**: Launch in Photoshop, GIMP, etc.
- **File manager integration**: "Show in Finder/Explorer" command

**UI Elements:**
- List view with:
  - Viewer name (editable)
  - Command field
  - Arguments field (with {path} placeholder documentation)
  - Enable/disable checkbox
  - Up/down buttons for reordering
  - Add/remove buttons
- Separate section for external editor
- Checkbox for file manager integration

---

## UI Design

### Layout

The Settings window will follow the existing overlay pattern but with a larger, tabbed/sectioned layout:

```
┌─────────────────────────────────────────────────────────────┐
│  Settings                                          [X] Close │
├─────────────────────────────────────────────────────────────┤
│ ┌──────────┬────────────────────────────────────────────────┐
│ │ General  │  Default zoom mode:                            │
│ │ Behavior │    (•) Fit to window  ( ) 100%                │
│ ├──────────┤                                                │
│ │ Perfor-  │  ☑ Remember per-image state (zoom, pan, etc.) │
│ │ mance    │                                                │
│ ├──────────┤  State cache size: [1000      ] images        │
│ │ Keyboard │                                                │
│ │ & Mouse  │  ☑ Auto-play animated images (GIFs, WEBPs)   │
│ ├──────────┤                                                │
│ │ Files    │                                                │
│ ├──────────┤                                                │
│ │ Appear-  │                                                │
│ │ ance     │                                                │
│ ├──────────┤                                                │
│ │ Filters  │                                                │
│ ├──────────┤                                                │
│ │ Naviga-  │                                                │
│ │ tion     │                                                │
│ ├──────────┤                                                │
│ │ External │                                                │
│ │ Tools    │                                                │
│ └──────────┴────────────────────────────────────────────────┘
│                                                               │
│           [Reset to Defaults]  [Cancel]  [Apply]            │
└─────────────────────────────────────────────────────────────┘
```

### Component Implementation

```rust
// components/settings_window.rs

pub struct SettingsWindow {
    settings: AppSettings,          // Working copy (not applied until Apply clicked)
    original_settings: AppSettings, // For Cancel operation
    selected_section: SettingsSection,
    // Widget entities for each setting
    // (sliders, checkboxes, text inputs, etc.)
}

pub enum SettingsSection {
    ViewerBehavior,
    Performance,
    KeyboardMouse,
    FileOperations,
    Appearance,
    Filters,
    SortNavigation,
    ExternalTools,
}

impl SettingsWindow {
    pub fn new(current_settings: AppSettings) -> Self {
        Self {
            settings: current_settings.clone(),
            original_settings: current_settings,
            selected_section: SettingsSection::ViewerBehavior,
        }
    }
    
    pub fn render_section(&self, section: SettingsSection) -> impl IntoElement {
        match section {
            SettingsSection::ViewerBehavior => self.render_viewer_behavior(),
            SettingsSection::Performance => self.render_performance(),
            // ... other sections
        }
    }
    
    pub fn apply(&mut self) -> AppSettings {
        self.original_settings = self.settings.clone();
        self.settings.clone()
    }
    
    pub fn cancel(&mut self) {
        self.settings = self.original_settings.clone();
    }
    
    pub fn reset_to_defaults(&mut self) {
        self.settings = AppSettings::default();
    }
}
```

### External Viewer Configuration UI

The External Tools section deserves special attention as it was the original request:

```
┌─────────────────────────────────────────────────────────────┐
│  External Image Viewers                                      │
│                                                               │
│  rpview will try these viewers in order until one succeeds.  │
│                                                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ ☑ Preview                                    [↑] [↓]  │  │
│  │   Command: open                                       │  │
│  │   Args: -a Preview {path}                    [Edit]  │  │
│  ├───────────────────────────────────────────────────────┤  │
│  │ ☐ Custom Viewer                              [↑] [↓]  │  │
│  │   Command: /usr/local/bin/myviewer                    │  │
│  │   Args: --fullscreen {path}                  [Edit]  │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                               │
│  [+ Add Viewer]  [- Remove Selected]                         │
│                                                               │
│  Note: Use {path} as a placeholder for the image file path.  │
└─────────────────────────────────────────────────────────────┘
```

---

## Persistence

### Storage Location

Use the `dirs` crate (already in dependencies) to locate the platform-appropriate config directory:

```rust
// utils/settings_io.rs

use dirs::config_dir;
use std::path::PathBuf;

pub fn get_settings_path() -> PathBuf {
    let config_dir = config_dir()
        .expect("Could not find config directory")
        .join("rpview");
    
    // Create directory if it doesn't exist
    std::fs::create_dir_all(&config_dir)
        .expect("Could not create config directory");
    
    config_dir.join("settings.json")
}
```

**Platform locations:**
- **macOS**: `~/Library/Application Support/rpview/settings.json`
- **Linux**: `~/.config/rpview/settings.json`
- **Windows**: `C:\Users\<User>\AppData\Roaming\rpview\settings.json`

### Serialization

Use `serde_json` for human-readable, editable config files:

```rust
// Add to Cargo.toml:
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = get_settings_path();
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;
    
    Ok(())
}

pub fn load_settings() -> AppSettings {
    let path = get_settings_path();
    
    if !path.exists() {
        return AppSettings::default();
    }
    
    let json = match std::fs::read_to_string(&path) {
        Ok(json) => json,
        Err(_) => return AppSettings::default(),
    };
    
    match serde_json::from_str(&json) {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("Failed to parse settings file: {}", e);
            eprintln!("Using default settings");
            AppSettings::default()
        }
    }
}
```

### Auto-save Strategy

- **Load on startup**: In `main()` before creating App
- **Save on Apply**: When user clicks Apply button in settings window
- **Save on quit**: In `Quit` action handler as safety measure
- **No auto-save on every change**: Prevents partial/invalid state

---

## Implementation Steps

### Phase 1: Foundation (2-3 hours)

1. **Create settings module structure**
   - [ ] Create `src/state/settings.rs`
   - [ ] Define all structs (AppSettings, ViewerBehavior, etc.)
   - [ ] Implement Default traits
   - [ ] Add serde derives

2. **Implement persistence**
   - [ ] Create `src/utils/settings_io.rs`
   - [ ] Implement `get_settings_path()`
   - [ ] Implement `save_settings()` and `load_settings()`
   - [ ] Add serde and serde_json to Cargo.toml

3. **Integrate into App**
   - [ ] Add `settings: AppSettings` field to App struct
   - [ ] Load settings in `main()` before window creation
   - [ ] Pass settings to components that need them
   - [ ] Save settings on quit

### Phase 2: Settings Window UI (3-4 hours)

1. **Create SettingsWindow component**
   - [ ] Create `src/components/settings_window.rs`
   - [ ] Define SettingsWindow struct and SettingsSection enum
   - [ ] Implement basic overlay layout (full-screen, centered box)
   - [ ] Add section navigation (left sidebar)
   - [ ] Implement Apply/Cancel/Reset buttons

2. **Implement section rendering**
   - [ ] `render_viewer_behavior()` - radio buttons, checkboxes, numeric inputs
   - [ ] `render_keyboard_mouse()` - sliders for speeds and sensitivity
   - [ ] `render_external_tools()` - list view with add/remove/reorder
   - [ ] Other sections as needed (start with high-priority ones)

3. **Wire up actions**
   - [ ] Add `ToggleSettings` action
   - [ ] Add `Cmd+,` keybinding
   - [ ] Add `show_settings: bool` to App struct
   - [ ] Implement toggle handler
   - [ ] Add conditional rendering in App::render()

### Phase 3: External Viewer Integration (1-2 hours)

1. **Update open_in_system_viewer()**
   - [ ] Read `settings.external_tools.external_viewers`
   - [ ] Loop through viewers in order
   - [ ] Replace `{path}` placeholder with actual image path
   - [ ] Try each enabled viewer until one succeeds
   - [ ] Fall back to platform defaults if all fail

2. **Add external editor action**
   - [ ] Create `OpenInExternalEditor` action
   - [ ] Add keybinding (e.g., `Cmd+E`)
   - [ ] Implement handler using `settings.external_tools.external_editor`

### Phase 4: Apply Settings Throughout App (2-3 hours)

1. **Viewer behavior settings**
   - [ ] Apply default zoom mode when loading images
   - [ ] Toggle state cache based on `remember_per_image_state`
   - [ ] Resize cache when `state_cache_size` changes
   - [ ] Control animation auto-play

2. **Keyboard & mouse settings**
   - [ ] Replace hardcoded pan speeds with settings values
   - [ ] Replace hardcoded zoom sensitivities with settings values
   - [ ] Implement spacebar acceleration toggle

3. **Appearance settings**
   - [ ] Apply background color to viewer
   - [ ] Apply overlay transparency
   - [ ] Apply font size scale to overlays
   - [ ] Apply window title format template

4. **File operations settings**
   - [ ] Use default save directory
   - [ ] Use default save format
   - [ ] Implement "remember last directory" behavior

### Phase 5: Testing & Polish (1-2 hours)

1. **Testing**
   - [ ] Test settings persistence across app restarts
   - [ ] Test all UI controls (sliders, checkboxes, text inputs)
   - [ ] Test external viewer configuration
   - [ ] Test settings reset
   - [ ] Test settings cancel

2. **Documentation**
   - [ ] Update help overlay with Cmd+, shortcut
   - [ ] Document settings file format
   - [ ] Add settings section to README

3. **Error handling**
   - [ ] Handle missing/corrupt settings file gracefully
   - [ ] Validate numeric inputs (min/max ranges)
   - [ ] Validate external viewer commands

---

## Estimated Total Implementation Time

**~12-15 hours** for complete settings system with all categories.

**Priority order for incremental implementation:**

1. **Phase 1 (Foundation)**: Required for any settings functionality - 2-3 hours
2. **External Tools**: Original request, high value - 2-3 hours (includes Phase 2 for this section)
3. **Keyboard & Mouse**: Commonly requested, easy to implement - 1-2 hours
4. **Viewer Behavior**: Good quality-of-life improvements - 1-2 hours
5. **File Operations**: Nice to have, medium value - 1 hour
6. **Appearance**: Visual preference, easy wins - 1 hour
7. **Performance**: Advanced users only - 1 hour
8. **Other sections**: Lower priority, can be added incrementally - 1-2 hours each

---

## Future Enhancements

After the initial implementation, consider:

- **Settings import/export**: Share configurations between machines
- **Profile system**: Multiple named settings profiles
- **Settings search**: Filter settings by keyword
- **Settings validation**: Warn when settings conflict (e.g., cache size too large)
- **Live preview**: Show effect of appearance changes in real-time
- **Keyboard shortcut customization**: Remap any action to any key
- **Plugin system**: Allow third-party extensions to add settings sections

---

## Notes

- All existing functionality must continue to work with default settings
- Settings should be validated on load (use defaults for invalid values)
- Settings window should be responsive and work at different window sizes
- Consider adding tooltips/help text for complex settings
- External viewer list should support drag-and-drop reordering (nice-to-have)
- Settings should be backwards-compatible (new fields added with defaults)
