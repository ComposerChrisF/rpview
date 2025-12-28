# rpview-gpui Design Specification

A modern, cross-platform image viewer built with Rust and GPUI.

## Overview

rpview (Rust Picture Viewer) is a simple, fast image viewer designed for viewing and navigating through images with minimal distractions. Built with the GPUI framework, it offers a clean interface and smooth performance across Windows, macOS, and Linux.

**Key Philosophy:**
- Simple and easy to use
- Cross-platform (Windows, macOS, Linux)
- Minimal UI distractions - focus on the image
- Smooth, responsive interactions

## Features

### Current Features

- **Image Viewing**: Display PNG, JPEG, GIF, BMP, TIFF, ICO, and WEBP images
- **Navigation**: Browse through images using arrow keys
- **Exit Handling**: Triple-escape (3 times within 2 seconds) to quit
- **Standard Shortcuts**: 
  - macOS: Cmd+W to close window, Cmd+Q to quit
  - Windows/Linux: Ctrl+W to close window, Ctrl+Q to quit

### Planned Features

- Image display and basic navigation
- Zoom controls (keyboard, mouse wheel with Ctrl modifier)
- Pan controls (keyboard, mouse-based)
- Per-image state (zoom/pan persistence)
- Help overlay (keyboard shortcuts)
- Debug info overlay
- Image filters (brightness, contrast, gamma)
- File operations (open, save with filters)
- Sorting modes (alphabetical, modified date)
- Animation support for GIF and WEBP

## Architecture

### State Management

**AppState**: Global application state
- Image list and current index
- Current zoom/pan values (active image)
- Per-image state cache (LRU eviction at 1000 items)
- UI overlay visibility flags
- Navigation mode (alphabetical/modified date)

**ImageState**: Per-image settings
- Zoom level, pan position (x, y)
- Filter settings (brightness, contrast, gamma)
- Last accessed timestamp for LRU cache

### Component Hierarchy

```
App (root)
├── ImageViewer - Image display and controls
├── HelpOverlay - Keyboard shortcuts
├── ErrorDisplay - Error messages
└── DebugOverlay - Debug information
```

## Design Decisions

### Zoom and Pan System

**Zoom Features:**
- Mouse wheel zoom: Cursor-centered zoom - the image pixel under the cursor stays under the cursor as zoom changes
- Keyboard zoom: +/- keys with logarithmic stepping (centered on viewport)
- Z + mouse drag: Zoom in/out by dragging up/down, centered on initial click position
- Zoom range: 10% to 2000%
- Modifier keys affect zoom speed (keyboard only):
  - Shift: Faster steps (larger increments)
  - Ctrl (Windows/Linux) / Cmd (macOS): Slower steps (smaller increments)
  - Shift+Ctrl/Cmd: Incremental 1% steps

**Pan Features:**
- Spacebar + mouse drag: Grab-and-drag panning - the image pixel under the cursor follows the mouse
- WASD/IJKL keyboard panning: 10px base speed
- Modifier keys affect pan speed:
  - Shift: 3x speed (30px)
  - Ctrl (Windows/Linux) / Cmd (macOS): 0.3x speed (3px)

**State Persistence:**
- Zoom and pan settings retained per image
- Maximum 1000 images in LRU cache
- NOT persisted to disk between sessions

### Filter System

**Filter Controls:**
- Brightness: -100 to +100 (0 = no change)
- Contrast: -100 to +100 (0 = no change)
- Gamma: 0.1 to 10.0 (1.0 = no change, logarithmic scale)

**Filter Operations:**
- Ctrl+F (Windows/Linux) / Cmd+F (macOS): Toggle filter controls
- Ctrl+1 (Windows/Linux) / Cmd+1 (macOS): Disable filters (preserve values)
- Ctrl+2 (Windows/Linux) / Cmd+2 (macOS): Re-enable filters
- Filters retained per image like zoom/pan

**File Operations:**
- Ctrl+O (Windows/Linux) / Cmd+O (macOS): Open file dialog
- Ctrl+S (Windows/Linux) / Cmd+S (macOS): Save with filters applied
- Auto-increment filenames: image.png → image_filtered.png → image_filtered_2.png

### Navigation

**File System Integration:**
- Directory scanning for images
- Command line file/directory argument support
- Handle non-existent files gracefully
- File extension detection
- Supported formats: PNG, JPEG, JPG, GIF, BMP, TIFF, TIF, ICO, WEBP

**Navigation Controls:**
- Arrow keys (left/right) for previous/next image
- Wrap-around navigation
- Current file position tracking
- Window title updates with position (e.g., "image.png (3/10)")

**Sorting System:**
- Alphabetical sorting (case insensitive)
- Modified date sorting (newest first)
- Sort mode toggle shortcuts
- Display sort mode indicator in UI
- Maintain current image when switching sort modes

### Exit Handling

**Multiple Exit Methods:**
- Triple-escape: Press Escape 3 times within 2 seconds
- Ctrl/Cmd+W: Close window
- Ctrl/Cmd+Q: Quit application
- Window close button (X)

### Animation Support

**GIF Animation:**
- Frame extraction and timing
- Playback loop
- Frame counter display
- Play/pause toggle with O key
- Frame stepping with [ and ] keys

**WEBP Animation:**
- Animation detection
- Frame extraction
- Timing metadata parsing
- Format indicator in frame counter

**Performance:**
- Smart pause on window focus loss
- Optimize memory for large animated images
- Handle animation loops

## User Interface

### Help Overlay
- Toggle with H, ?, or F1 keys
- Complete keyboard shortcuts documentation
- Click-outside-to-close functionality

### Zoom Indicator
- Bottom-right corner display
- Current zoom percentage
- Auto-hide after zoom changes (2s delay)
- Sort mode indicator

### Error Display
- File load error messages
- Unsupported format messages
- "No images found" message
- Auto-dismiss after 5 seconds
- Manual dismiss on click

### Debug Overlay
- F12 toggle
- Current image metadata
- Zoom/pan state
- Current index and total
- Performance metrics

## Keyboard Shortcuts

### Navigation
- `←` / `→` - Previous/Next image
- `Escape` (3x within 2s) - Exit application
- `Ctrl+Q` / `Ctrl+W` (Windows/Linux) - Quit/Close window
- `Cmd+Q` / `Cmd+W` (macOS) - Quit/Close window

### Zoom
- `+` / `=` - Zoom in
- `-` - Zoom out
- `0` - Reset zoom to 100%
- Mouse wheel + Ctrl (Windows/Linux) / Cmd (macOS) - Zoom at cursor position

### Pan
- `Shift+←→↑↓` - Pan image in direction
- `WASD` / `IJKL` - Keyboard panning
- Spacebar + mouse drag - Grab-and-drag panning

### Filters
- `Ctrl+F` (Windows/Linux) / `Cmd+F` (macOS) - Toggle filter controls
- `Ctrl+1` (Windows/Linux) / `Cmd+1` (macOS) - Disable filters
- `Ctrl+2` (Windows/Linux) / `Cmd+2` (macOS) - Re-enable filters

### View
- `H` / `?` / `F1` - Show/hide help overlay
- `F12` - Show/hide debug information

### File Operations
- `Ctrl+O` (Windows/Linux) / `Cmd+O` (macOS) - Open file dialog
- `Ctrl+S` (Windows/Linux) / `Cmd+S` (macOS) - Save with filters

### Animation
- `O` - Play/pause animation
- `[` / `]` - Previous/Next frame

## Supported Image Formats

- PNG - Portable Network Graphics
- JPEG/JPG - Joint Photographic Experts Group
- GIF - Graphics Interchange Format (with animation)
- BMP - Windows Bitmap
- TIFF/TIF - Tagged Image File Format
- ICO - Windows Icon Format
- WEBP - WebP Format (with animation)

## Platform-Specific Notes

**macOS**: 
- Use `Cmd` key for shortcuts
- Native menu integration (planned)
- Retina display support
- App bundle creation

**Windows**: 
- Use `Ctrl` key for shortcuts
- File dialog integration
- Window behavior
- Installation packages

**Linux**: 
- Use `Ctrl` key for shortcuts
- GTK integration
- Window manager compatibility
- Desktop file creation

## Performance Considerations

**Image Loading:**
- Async image loading
- Efficient state updates
- Image caching for recently viewed files
- Lazy loading for non-displayed images

**Animation:**
- Frame-based playback
- Efficient memory management
- Smart pause on focus loss
- Startup timer to prevent early pausing

**State Management:**
- LRU cache with 1000 item limit
- Per-image state persistence
- Efficient state serialization
- Cache eviction strategy

## Future Enhancements

### Advanced Features
- Custom filter plugins
- Batch processing
- Filter scripting
- RAW image format support
- SVG support
- Video thumbnail support

### Productivity Features
- Image organization tools
- Metadata editing
- Slideshow mode
- Print support
- Multi-monitor support

### GPU Acceleration
- GPU-based filter pipeline (wgpu + naga)
- Real-time shader effects
- Advanced color grading
- Convolution filters
