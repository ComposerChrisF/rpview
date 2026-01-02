# Settings Documentation

## Overview

RPView stores all user preferences in a JSON configuration file located at:

- **macOS**: `~/Library/Application Support/rpview/settings.json`
- **Linux**: `~/.config/rpview/settings.json`
- **Windows**: `C:\Users\<User>\AppData\Roaming\rpview\settings.json`

Settings are automatically loaded on startup and can be modified either:
1. Through the settings window (press `Cmd+,` or `Ctrl+,`)
2. By manually editing the `settings.json` file

## Accessing Settings

### Settings Window
Press `Cmd+,` (macOS) or `Ctrl+,` (Windows/Linux) to open the interactive settings window where you can:
- View all current settings
- Modify numeric values using increment/decrement buttons
- Toggle boolean options with checkboxes
- Select enum values with radio buttons
- Apply changes or cancel
- Reset all settings to defaults

### Manual Editing
You can also edit the `settings.json` file directly with any text editor. The file uses standard JSON format with pretty-printing for readability. Changes take effect the next time you launch RPView.

## Settings File Structure

```json
{
  "viewer_behavior": { ... },
  "performance": { ... },
  "keyboard_mouse": { ... },
  "file_operations": { ... },
  "appearance": { ... },
  "filters": { ... },
  "sort_navigation": { ... },
  "external_tools": { ... }
}
```

## Viewer Behavior Settings

Controls how images are displayed and managed.

### `default_zoom_mode`
**Type**: String enum (`"FitToWindow"` or `"OneHundredPercent"`)  
**Default**: `"FitToWindow"`

Determines how new images are displayed when first loaded:
- `"FitToWindow"`: Image is scaled to fit within the window while maintaining aspect ratio
- `"OneHundredPercent"`: Image is displayed at actual pixel size (100% zoom)

```json
"default_zoom_mode": "FitToWindow"
```

### `remember_per_image_state`
**Type**: Boolean  
**Default**: `true`

When enabled, RPView remembers zoom level, pan position, and filter settings for each image. When you navigate back to a previously viewed image, it restores your exact view state.

```json
"remember_per_image_state": true
```

### `state_cache_size`
**Type**: Integer  
**Default**: `1000`  
**Range**: 1 - 10000

Maximum number of images to remember state for. When this limit is reached, the least recently viewed images are forgotten (LRU eviction). Increase this if you work with large collections and want to preserve state for more images.

```json
"state_cache_size": 1000
```

### `animation_auto_play`
**Type**: Boolean  
**Default**: `true`

Controls whether animated GIFs and WebP images automatically start playing when loaded. When disabled, animations remain paused until you press the `O` key.

```json
"animation_auto_play": true
```

## Performance Settings

Controls performance-related optimizations.

### `preload_adjacent_images`
**Type**: Boolean  
**Default**: `true`

When enabled, RPView preloads the next and previous images into GPU memory for instant navigation. This eliminates the black flash when switching between images but uses more GPU memory.

```json
"preload_adjacent_images": true
```

### `filter_processing_threads`
**Type**: Integer  
**Default**: `4`  
**Range**: 1 - 16

Number of CPU threads to use for filter processing. Higher values speed up filter calculations but use more CPU resources. Set to match your CPU core count for best performance.

```json
"filter_processing_threads": 4
```

### `max_image_dimensions`
**Type**: Array of two integers `[width, height]`  
**Default**: `[16384, 16384]`

Safety limit for image dimensions. Images larger than this are rejected to prevent memory exhaustion. Default is 16K x 16K which should handle most images.

```json
"max_image_dimensions": [16384, 16384]
```

## Keyboard & Mouse Settings

Controls input sensitivity and speeds.

### `pan_speed_normal`
**Type**: Float  
**Default**: `10.0`  
**Range**: 1.0 - 100.0

Distance in pixels the image moves with each press of WASD/IJKL keys (without modifiers).

```json
"pan_speed_normal": 10.0
```

### `pan_speed_fast`
**Type**: Float  
**Default**: `30.0`  
**Range**: 1.0 - 100.0

Distance in pixels the image moves when holding Shift while pressing WASD/IJKL keys.

```json
"pan_speed_fast": 30.0
```

### `pan_speed_slow`
**Type**: Float  
**Default**: `3.0`  
**Range**: 0.1 - 100.0

Distance in pixels the image moves when holding Cmd/Ctrl while pressing WASD/IJKL keys. Useful for precise positioning.

```json
"pan_speed_slow": 3.0
```

### `scroll_wheel_sensitivity`
**Type**: Float  
**Default**: `1.1`  
**Range**: 1.01 - 2.0

Zoom multiplier per mouse wheel notch when holding Cmd/Ctrl. Default `1.1` means 10% zoom per scroll. Higher values = faster zoom, lower values = finer control.

```json
"scroll_wheel_sensitivity": 1.1
```

### `z_drag_sensitivity`
**Type**: Float  
**Default**: `0.01`  
**Range**: 0.001 - 0.1

Controls how much mouse movement affects zoom when using Z+Drag zoom. Lower values = finer control, higher values = faster zoom.

```json
"z_drag_sensitivity": 0.01
```

### `spacebar_pan_accelerated`
**Type**: Boolean  
**Default**: `false`

When enabled, spacebar+drag panning accelerates with faster mouse movement. Currently a placeholder for future implementation.

```json
"spacebar_pan_accelerated": false
```

## File Operations Settings

Controls file saving and directory behavior.

### `default_save_directory`
**Type**: String (optional)  
**Default**: `null`

Default directory for saving filtered images. When `null`, saves to the same directory as the source image. Set to a path like `"/Users/you/Pictures/Edited"` to always save to a specific folder.

```json
"default_save_directory": null
```

Or:
```json
"default_save_directory": "/Users/username/Pictures/Filtered"
```

### `default_save_format`
**Type**: String enum  
**Default**: `"Png"`  
**Values**: `"Png"`, `"Jpeg"`, `"Bmp"`, `"Tiff"`, `"Webp"`

Default file format when saving filtered images. Unfiltered images keep their original format.

```json
"default_save_format": "Png"
```

### `auto_save_filtered_cache`
**Type**: Boolean  
**Default**: `false`

Placeholder for future feature to automatically save filtered images to disk cache.

```json
"auto_save_filtered_cache": false
```

### `remember_last_directory`
**Type**: Boolean  
**Default**: `true`

Placeholder for future feature to remember the last directory used in file dialogs.

```json
"remember_last_directory": true
```

## Appearance Settings

Controls visual appearance and UI elements.

### `background_color`
**Type**: Array of three integers `[R, G, B]`  
**Default**: `[30, 30, 30]`  
**Range**: Each value 0 - 255

Background color behind images in RGB format. Default is dark gray `(30, 30, 30)`.

```json
"background_color": [30, 30, 30]
```

### `overlay_transparency`
**Type**: Integer  
**Default**: `204`  
**Range**: 0 - 255

Alpha transparency for overlay backgrounds (help, debug, filters). 0 = fully transparent, 255 = fully opaque. Default `204` = ~80% opaque.

```json
"overlay_transparency": 204
```

### `font_size_scale`
**Type**: Float  
**Default**: `1.0`  
**Range**: 0.5 - 2.0

Multiplier for all UI text sizes. 1.0 = default size, 1.5 = 150% larger, 0.8 = 80% smaller. Useful for high-DPI displays or accessibility.

```json
"font_size_scale": 1.0
```

### `window_title_format`
**Type**: String  
**Default**: `"{filename} ({index}/{total})"`

Template for window title text. Supports placeholders:
- `{filename}`: Current image filename
- `{index}`: Current image index (1-based)
- `{total}`: Total number of images

```json
"window_title_format": "{filename} ({index}/{total})"
```

Examples:
- `"{filename}"` - Just the filename
- `"[{index}/{total}] {filename}"` - Index in brackets before filename
- `"RPView - {filename}"` - App name prefix

## Filter Settings

Controls default filter values and behavior.

### `default_brightness`
**Type**: Float  
**Default**: `0.0`  
**Range**: -100.0 to 100.0

Default brightness adjustment when resetting filters. 0 = no change, positive = brighter, negative = darker.

```json
"default_brightness": 0.0
```

### `default_contrast`
**Type**: Float  
**Default**: `0.0`  
**Range**: -100.0 to 100.0

Default contrast adjustment when resetting filters. 0 = no change, positive = more contrast, negative = less contrast.

```json
"default_contrast": 0.0
```

### `default_gamma`
**Type**: Float  
**Default**: `1.0`  
**Range**: 0.1 to 10.0

Default gamma correction when resetting filters. 1.0 = no change, <1.0 = darker midtones, >1.0 = brighter midtones.

```json
"default_gamma": 1.0
```

### `remember_filter_state`
**Type**: Boolean  
**Default**: `true`

When enabled, filter settings are saved per-image. Currently always enabled (cannot be disabled).

```json
"remember_filter_state": true
```

### `filter_presets`
**Type**: Array of preset objects  
**Default**: `[]`

Placeholder for future feature to save and load filter preset combinations.

```json
"filter_presets": []
```

## Sort & Navigation Settings

Controls how images are sorted and navigated.

### `default_sort_mode`
**Type**: String enum  
**Default**: `"Alphabetical"`  
**Values**: `"Alphabetical"`, `"ModifiedDate"`

Default sorting mode when opening images:
- `"Alphabetical"`: Sort by filename (case-insensitive)
- `"ModifiedDate"`: Sort by file modification date (newest first)

```json
"default_sort_mode": "Alphabetical"
```

### `wrap_navigation`
**Type**: Boolean  
**Default**: `true`

When enabled, navigation wraps around: pressing → on the last image goes to the first, and ← on the first image goes to the last. When disabled, navigation stops at boundaries.

```json
"wrap_navigation": true
```

### `show_image_counter`
**Type**: Boolean  
**Default**: `true`

Controls whether image counter `(index/total)` is shown in the window title. When disabled, only the filename is shown.

```json
"show_image_counter": true
```

## External Tools Settings

Controls integration with external applications.

### `external_viewers`
**Type**: Array of viewer objects  
**Default**: Platform-specific

List of external image viewers to try when using Cmd+Option+F. RPView tries each enabled viewer in order until one succeeds, then falls back to system default.

Each viewer object has:
- `name`: Display name
- `command`: Executable to run
- `args`: Array of arguments (use `{path}` placeholder for image path)
- `enabled`: Whether to try this viewer

**macOS default:**
```json
"external_viewers": [
  {
    "name": "Preview",
    "command": "open",
    "args": ["-a", "Preview", "{path}"],
    "enabled": true
  }
]
```

**Windows example:**
```json
"external_viewers": [
  {
    "name": "Windows Photos",
    "command": "cmd",
    "args": ["/c", "start", "{path}"],
    "enabled": true
  },
  {
    "name": "IrfanView",
    "command": "C:\\Program Files\\IrfanView\\i_view64.exe",
    "args": ["{path}"],
    "enabled": true
  }
]
```

**Linux example:**
```json
"external_viewers": [
  {
    "name": "Eye of GNOME",
    "command": "eog",
    "args": ["{path}"],
    "enabled": true
  },
  {
    "name": "Gwenview",
    "command": "gwenview",
    "args": ["{path}"],
    "enabled": true
  }
]
```

### `external_editor`
**Type**: String (optional)  
**Default**: `null`

Path to external image editor for Cmd+E shortcut. When `null`, the feature is disabled.

```json
"external_editor": null
```

Or:
```json
"external_editor": "/Applications/Photoshop.app/Contents/MacOS/Photoshop"
```

### `enable_file_manager_integration`
**Type**: Boolean  
**Default**: `true`

Placeholder for future "Show in Finder/Explorer" feature.

```json
"enable_file_manager_integration": true
```

## Complete Example

Here's a complete `settings.json` file with all defaults:

```json
{
  "viewer_behavior": {
    "default_zoom_mode": "FitToWindow",
    "remember_per_image_state": true,
    "state_cache_size": 1000,
    "animation_auto_play": true
  },
  "performance": {
    "preload_adjacent_images": true,
    "filter_processing_threads": 4,
    "max_image_dimensions": [16384, 16384]
  },
  "keyboard_mouse": {
    "pan_speed_normal": 10.0,
    "pan_speed_fast": 30.0,
    "pan_speed_slow": 3.0,
    "scroll_wheel_sensitivity": 1.1,
    "z_drag_sensitivity": 0.01,
    "spacebar_pan_accelerated": false
  },
  "file_operations": {
    "default_save_directory": null,
    "default_save_format": "Png",
    "auto_save_filtered_cache": false,
    "remember_last_directory": true
  },
  "appearance": {
    "background_color": [30, 30, 30],
    "overlay_transparency": 204,
    "font_size_scale": 1.0,
    "window_title_format": "{filename} ({index}/{total})"
  },
  "filters": {
    "default_brightness": 0.0,
    "default_contrast": 0.0,
    "default_gamma": 1.0,
    "remember_filter_state": true,
    "filter_presets": []
  },
  "sort_navigation": {
    "default_sort_mode": "Alphabetical",
    "wrap_navigation": true,
    "show_image_counter": true
  },
  "external_tools": {
    "external_viewers": [
      {
        "name": "Preview",
        "command": "open",
        "args": ["-a", "Preview", "{path}"],
        "enabled": true
      }
    ],
    "external_editor": null,
    "enable_file_manager_integration": true
  }
}
```

## Troubleshooting

### Settings Not Persisting

If your settings aren't being saved:
1. Check file permissions on the config directory
2. Look for error messages in the terminal output
3. Verify the JSON file is valid (use a JSON validator)
4. Make sure you clicked "Apply" in the settings window

### Corrupt Settings File

If RPView detects a corrupt settings file, it will:
1. Print a warning message with the parse error
2. Create a backup at `settings.json.backup`
3. Load default settings
4. Overwrite the corrupt file with valid defaults

You can restore from the backup if needed:
```bash
# macOS/Linux
cd ~/Library/Application\ Support/rpview/  # or ~/.config/rpview/ on Linux
mv settings.json.backup settings.json

# Windows
cd %APPDATA%\rpview\
move settings.json.backup settings.json
```

### Resetting to Defaults

To completely reset all settings:

**Option 1: Via Settings Window**
1. Press `Cmd+,` (or `Ctrl+,`)
2. Click "Reset to Defaults"
3. Click "Apply"

**Option 2: Delete Settings File**
```bash
# macOS
rm ~/Library/Application\ Support/rpview/settings.json

# Linux
rm ~/.config/rpview/settings.json

# Windows
del %APPDATA%\rpview\settings.json
```

RPView will create a new settings file with defaults on next launch.

### Finding the Settings File

If you can't find the settings file, check the console output when launching RPView. It prints:
```
Settings loaded from: /path/to/settings.json
```

### Manual Editing Tips

When editing `settings.json` manually:
- Use a JSON-aware editor (VS Code, Sublime, etc.) for syntax highlighting
- Validate your JSON before saving (use jsonlint.com or similar)
- Keep a backup before making changes
- Close RPView before editing (changes load at startup)
- Numbers don't need quotes, strings and enum values do
- Booleans are lowercase: `true` and `false`

### Common Mistakes

**Wrong**: String numbers
```json
"pan_speed_normal": "10.0"  // Don't quote numbers
```

**Correct**:
```json
"pan_speed_normal": 10.0
```

**Wrong**: Capitalized booleans
```json
"wrap_navigation": True  // JavaScript-style boolean
```

**Correct**:
```json
"wrap_navigation": true  // JSON requires lowercase
```

**Wrong**: Missing quotes on enum values
```json
"default_zoom_mode": FitToWindow  // Must be quoted
```

**Correct**:
```json
"default_zoom_mode": "FitToWindow"
```

## See Also

- [User Guide](../README.md) - General usage instructions
- [Keyboard Shortcuts](../DESIGN.md) - Complete shortcut reference
- [Settings Design](SETTINGS_DESIGN.md) - Technical architecture details
