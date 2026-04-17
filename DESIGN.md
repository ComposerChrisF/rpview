# rpview Design Specification

A fast, keyboard-driven image viewer built with Rust and GPUI.

## Overview

rpview (Rust Picture Viewer) is a cross-platform image viewer designed for
fast browsing of large image directories. Built with the GPUI framework, it
emphasizes instant navigation, precise zoom/pan, and a keyboard-first workflow
with minimal UI distractions.

**Key Philosophy:**
- Keyboard-driven — nearly everything has a shortcut
- Instant — GPU preloading eliminates navigation latency
- Minimal UI — the image dominates; overlays and panels are optional
- Cross-platform — macOS, Windows, Linux

## Supported Formats

PNG, JPEG, BMP, GIF (animated), TIFF, ICO, WebP (animated), SVG

SVGs are re-rasterized at the current zoom level for always-crisp vector
display. Large SVGs use viewport-only rendering with padding for performance.

## Architecture

### Module Layout

```
src/
├── main.rs                    # App entry point, window creation
├── lib.rs                     # Action definitions, shared types
├── cli.rs                     # Clap CLI parsing
├── error.rs                   # AppError, AppResult
├── macos_open_handler.rs      # macOS Finder "Open With" integration
├── app_handlers.rs            # Event handlers (navigation, zoom, pan, etc.)
├── app_keybindings.rs         # Keybinding registration
├── app_render.rs              # Main window render method
├── components/
│   ├── image_viewer.rs        # Image display, GPU preloading
│   ├── filter_controls.rs     # Brightness/contrast/gamma sliders (entity)
│   ├── filter_window.rs       # Floating always-on-top filter panel
│   ├── local_contrast_controls.rs  # LC parameter sliders and presets
│   ├── local_contrast_window.rs    # Floating always-on-top LC panel
│   ├── settings_window.rs     # Interactive settings UI
│   ├── menu_bar.rs            # Application menu (File, View, etc.)
│   ├── help_overlay.rs        # Keyboard shortcuts reference
│   ├── debug_overlay.rs       # F12 diagnostic info
│   ├── zoom_indicator.rs      # Zoom %, resolution, sort mode display
│   ├── loading_indicator.rs   # Spinner during async image load
│   ├── processing_indicator.rs # Filter/LC processing status
│   ├── animation_indicator.rs # Frame counter for animated images
│   └── error_display.rs       # Error messages with auto-dismiss
├── state/
│   ├── app_state.rs           # Image list, index, zoom/pan, LC state, slots
│   ├── image_state.rs         # Per-image zoom/pan/filter cache (LRU)
│   ├── settings.rs            # AppSettings with 8 nested config structs
│   └── mod.rs
└── utils/
    ├── image_loader.rs        # Async image loading with cancellation
    ├── file_scanner.rs        # Directory scanning, extension filtering
    ├── filters.rs             # RGB LUT for brightness/contrast/gamma
    ├── local_contrast.rs      # OkLCh local luminance normalization
    ├── float_map.rs           # Planar f32 bitmap for LC pipeline
    ├── color.rs               # sRGB ↔ OkLCh color space conversion
    ├── lc_presets.rs           # LC preset save/load/delete
    ├── svg.rs                 # SVG rasterization via resvg
    ├── animation.rs           # GIF/WebP frame extraction and caching
    ├── zoom.rs                # Zoom math, clamping, fit-to-window
    ├── style.rs               # Colors, spacing, format_shortcut()
    ├── settings_io.rs         # Settings JSON read/write
    └── window_level.rs        # Platform FFI for always-on-top windows
```

### State Management

**AppState** — global application state, stored as a GPUI `Entity<App>`:
- Image list (`Vec<PathBuf>`) and current index
- Active zoom level, pan position, fit-to-window flag
- Per-image state cache (`HashMap` with LRU eviction at 1,000 items)
- Filter state (brightness, contrast, gamma) and enabled flag
- Local Contrast state (parameters, worker handle, progress, result buffer)
- Save/recall slots (numbered 3–9, each storing a snapshot image)
- Animation state (current frame, playing/paused, frame cache)
- UI flags (help visible, debug visible, delete confirmation, etc.)
- Sort mode (Alphabetical, ModifiedDate, TypeAlpha, TypeModified)

**ImageState** — per-image settings cached in the LRU:
- Zoom level and pan position (x, y)
- Filter settings (brightness, contrast, gamma)
- Last-accessed timestamp for LRU eviction

**AppSettings** — persisted to `settings.json`, 8 sections:
- `ViewerBehavior` — default zoom mode, state cache size, animation auto-play
- `Performance` — preload toggle, filter threads, max image dimension
- `KeyboardMouse` — pan speeds (normal 10px, fast 30px, slow 3px), pan direction mode, scroll/Z-drag sensitivity
- `FileOperations` — save directory, save format, remember last directory
- `Appearance` — dark/light background colors, overlay transparency, font scale, window title format, filter/LC window bounds
- `Filters` — default B/C/G values, filter presets
- `SortNavigation` — default sort mode, wrap navigation, show image counter
- `ExternalTools` — viewer list, editor, Finder/Explorer integration

## Features

### Zoom System

Five zoom speeds via keyboard, plus scroll-wheel and Z+drag:

| Input | Behavior |
|-------|----------|
| `+` / `-` | 1.25× steps, centered on viewport center |
| `Shift` + `+` / `-` | 1.5× steps (fast) |
| `Cmd/Ctrl` + `+` / `-` | 1.05× steps (slow) |
| `Shift+Cmd/Ctrl` + `+` / `-` | 1% steps (incremental) |
| Scroll wheel | Zoom at cursor position (pixel under cursor stays fixed) |
| `Z` + drag | Dynamic zoom centered on initial click position |

- Zoom range: 10% – 2,000%
- `0` toggles fit-to-window ↔ 100% (viewport-center preserved)
- `Cmd/Ctrl+0` resets zoom and pan to centered fit-to-window

### Pan System

Three speed tiers for keyboard panning, plus Space+drag:

| Input | Speed |
|-------|-------|
| `WASD` / `IJKL` | 10 px (screen pixels) |
| `Shift` + above | 30 px × zoom factor (image pixels) |
| `Alt` + above | 3 px (precise) |
| `Space` + drag | 1:1 mouse tracking |

Pan direction mode (configurable): Move Image (default) or Move Viewport.

### Image Filters

Brightness (-100 to +100), Contrast (-100 to +100), Gamma (0.1 to 10.0) —
applied as a 256-entry RGB LUT on a background thread. Filter state is
remembered per-image.

- `Cmd/Ctrl+F` or `F` — toggle floating filter window
- `1` / `2` — disable / enable filters (instant A/B comparison)
- `Shift+Cmd/Ctrl+R` — reset filters to defaults

The filter panel is a separate always-on-top OS window with persisted
position. Platform FFI: `NSFloatingWindowLevel` (macOS),
`SetWindowPos(HWND_TOPMOST)` (Windows), `WM_TRANSIENT_FOR` (Linux).

### Local Contrast

Perceptual local luminance normalization ported from FraleyMusic-ImageDsp,
operating in OkLCh color space. Opens as a floating dialog
(`Shift+Cmd/Ctrl+L`) with:

- **Sliders**: Contrast, Lighten Shadows, Darken Highlights
- **Pre-LC resize toggle**: 1/4×, 1/2×, 1×, 2×, 4× (Lanczos3 resampling)
- **Advanced section**: Use Median Gray-Point, Document Mode
- **Presets**: save/load/delete named parameter sets (JSON in settings directory)
- **Preview toggle**: suppress/resume LC render without losing the cached result

Processing runs on a rayon thread pool with cancellation and progress
reporting. The viewer treats the LC output's pixel dimensions as the
effective image size (zoom, fit, pan constraints, resolution indicator all
reflect the output). Zoom is rescaled on size transitions for clean A/B
comparison.

### Save/Recall Slots

`Ctrl+3–9` saves a snapshot of the current display (raw, filtered, or
LC-processed) into a numbered slot. Plain `3–9` recalls the saved snapshot.
`1` / `2` returns to the normal display path. Zoom and pan are rescaled on
slot transitions so the apparent image position stays constant.

### File Operations

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+O` | Open file(s) via dialog |
| `Cmd/Ctrl+S` | Save image (to current folder) |
| `Cmd/Ctrl+Alt+S` | Save image to Downloads |
| `Cmd/Ctrl+R` | Reveal in Finder / Explorer |
| `Cmd/Ctrl+Alt+V` | Open in external viewer (e.g. Preview.app) |
| `Shift+Cmd/Ctrl+Alt+V` | Open in external viewer and quit RPView |
| `Cmd/Ctrl+E` | Open in external editor |
| `Cmd/Ctrl+Delete` | Delete file (move to Trash) |
| `Shift+Cmd/Ctrl+Delete` | Permanently delete file |

Delete shows a confirmation card with filename and path; ESC cancels. Toast
notification confirms the outcome. Auto-increment filenames on save:
`image.png` → `image_filtered.png` → `image_filtered_2.png`.

### Navigation & Sorting

- Arrow keys (← →) navigate between images
- Wrap-around navigation (configurable)
- Four sort modes:
  - `Shift+Cmd/Ctrl+A` — Alphabetical (case-insensitive)
  - `Shift+Cmd/Ctrl+M` — Modified date (newest first)
  - `Shift+Cmd/Ctrl+T` — Type+Alpha / Type+Modified (toggles secondary)
- Sort mode preserved when switching; current image position maintained
- Window title template with `{filename}`, `{index}`, `{total}`, `{sm}`, `{sortmode}` placeholders
- Drag & drop: file, folder, or mixed — opens in current sort mode

### Animation Support

GIF and animated WebP playback with:
- `O` — play / pause
- `[` / `]` — step frame by frame
- 3-phase progressive frame caching:
  1. Cache first 3 frames immediately
  2. Look-ahead cache next 3 frames during playback
  3. GPU preload next frame to prevent black flash
- Auto-play on load (configurable)

### GPU Preloading

Adjacent images (next/previous) are preloaded into GPU texture memory during
the render loop. Navigation is instant — no loading spinner, no flash.
Off-screen rendering at `left(-10000px)` with `opacity(0.0)`. Only 2
additional GPU textures in memory.

### Settings Window

Press `Cmd/Ctrl+,` to open the interactive settings window. All 30+ settings
are editable through the UI:
- Toggle switches for booleans
- Segmented controls for enums (zoom mode, sort mode, save format, pan direction)
- Number steppers with clamped ranges for numeric values
- Color swatches for background colors
- Per-setting reset buttons (↺) — grayed at default, active when changed
- Global "Reset to Defaults" button
- Apply (`Cmd/Ctrl+Enter`) / Cancel (`Esc`)

Settings are also editable as JSON at the platform config path:
- **macOS**: `~/Library/Application Support/rpview/settings.json`
- **Linux**: `~/.config/rpview/settings.json`
- **Windows**: `C:\Users\<User>\AppData\Roaming\rpview\settings.json`

### Display Toggles

- `T` — toggle zoom/size indicator
- `B` — toggle dark / light background (colors configurable)
- `H` / `?` / `F1` — help overlay (all shortcuts)
- `F12` — debug overlay (image metadata, zoom/pan state, performance)

### Exit Handling

- `Cmd/Ctrl+W` — close window
- `Cmd/Ctrl+Q` — quit
- `ESC` — closes Filter or LC window first, then counts toward triple-ESC quit (3× within 2 seconds)
- Window close button (X)

## Cross-Platform

### Platform-Specific Behavior

- **macOS**: Cmd modifier, native ⌘⇧⌥ glyphs in UI, .app bundle with DMG creation, Finder "Open With" integration, `NSFloatingWindowLevel` for floating panels
- **Windows**: Ctrl modifier, .exe with embedded icon and version resource (winresource), `SetWindowPos(HWND_TOPMOST)` for floating panels, `/SUBSYSTEM:WINDOWS` (bins only)
- **Linux**: Ctrl modifier, `WM_TRANSIENT_FOR` hints for floating panels

### Utility Functions (`src/utils/style.rs`)

```rust
// Format a shortcut with platform-appropriate modifiers and glyphs
format_shortcut(key: &str, shift: bool, option: bool) -> String
```

Use `format_shortcut()` whenever displaying keyboard shortcuts in
user-facing text to ensure correct platform behavior.

## Performance

- **Async image loading** with cancellation (`LoaderHandle`) — UI stays responsive during large image loads
- **GPU texture preloading** — next/previous images ready before navigation
- **In-memory filter pipeline** — LUT pass takes ~5–15 ms vs 100–300 ms with the old temp-PNG round-trip
- **Rayon-parallelized LC** — sRGB↔OkLCh conversion, histogram grid, and per-pixel processing all parallel
- **Cached FloatMap** — planar f32 conversion cached on `LoadedImage` to skip per-tick conversion
- **LRU state cache** — 1,000-image limit with correct last-accessed tracking
- **Progressive animation caching** — 3-phase strategy for smooth GIF/WebP playback
- **SVG viewport rendering** — only rasterizes the visible portion for large SVGs at high zoom

## Future Considerations

- HEIC/HEIF support (under investigation: ffmpeg subprocess or macOS-native ImageIO)
- RAW image format support
- GPU-based filter pipeline (wgpu + naga)
