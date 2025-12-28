# TODO

This document outlines the development roadmap for rpview-gpui, organized by implementation phases.

## Progress Overview

- **Phase 1** (Foundation): ✅ Complete
- **Phase 2** (Basic Viewing): ✅ Complete
- **Phase 3** (Navigation): ✅ Complete
- **Phase 4** (Zoom & Pan): 🎯 Next Priority
- **Phase 5** (State Management): ⏳ Planned
- **Phase 6-15**: ⏳ Planned

## Phase 1: Project Foundation & Basic Structure ✅

### Project Setup
- [x] Create Cargo.toml with GPUI dependencies
- [x] Set up basic main.rs with GPUI app initialization
- [x] Create window with proper activation and focus
- [x] Implement window close handling (Cmd/Ctrl+W, Cmd/Ctrl+Q)
- [x] Implement triple-escape quit (3x within 2 seconds)
- [x] Set up error handling types and utilities
- [x] Create basic styling/layout framework

### Core Architecture
- [x] Design state management structure (AppState)
- [x] Design per-image state structure (ImageState)
- [x] Create component structure plan
- [x] Set up module organization (components/, state/, utils/)

### CLI Integration
- [x] Add clap dependency for CLI parsing
- [x] Implement CLI argument parsing (image paths)
- [x] Handle no arguments (default to current directory)
- [x] Handle single file argument
- [x] Handle multiple file arguments
- [x] Handle directory arguments
- [x] Handle mixed file/directory arguments

### Basic Documentation
- [x] Create DESIGN.md with application design
- [x] Create CLI.md with command-line interface design
- [x] Create TODO.md with implementation phases
- [x] Create CONTRIBUTING.md
- [x] Create CHANGELOG.md

## Phase 2: Basic Image Display ✅

### Image Loading Infrastructure
- [x] Add image crate dependency
- [x] Create image loading utilities (utils/image_loader.rs)
- [x] Implement synchronous image loading from file path
- [x] Add basic error handling for load failures
- [x] Support PNG format
- [x] Support JPEG/JPG format
- [x] Support BMP format

### File System Integration
- [x] Implement directory scanning for images (via CLI)
- [x] Filter files by supported extensions
- [x] Handle non-existent paths gracefully
- [x] Handle permission errors

### Basic Image Viewer Component
- [x] Create ImageViewer component structure
- [x] Display image information (dimensions, filename)
- [x] Handle missing/invalid image gracefully
- [x] Show loading state messages
- [x] Implement actual image rendering with GPUI using img() function
- [x] Implement fit-to-window display using ObjectFit::Contain
- [x] Add info panel showing filename and dimensions

### Application State
- [x] Create AppState structure  
- [x] Store list of image paths
- [x] Track current image index
- [x] Implement state initialization from CLI args
- [x] Connect state to ImageViewer component
- [x] Load first image on startup

### Error Display
- [x] Create basic error message display
- [x] Show "file not found" errors
- [x] Show "unsupported format" errors
- [x] Show "no images found" message
- [x] Show image loading errors

## Phase 3: Navigation & Sorting ✅

### Basic Navigation
- [x] Implement arrow key event handling (← →)
- [x] Add next_image() method to AppState
- [x] Add previous_image() method to AppState
- [x] Implement wrap-around navigation
- [x] Update ImageViewer when navigation occurs

### File List Management
- [x] Implement alphabetical sorting (case-insensitive)
- [x] Implement modified date sorting (newest first)
- [x] Track current sort mode in AppState
- [x] Default to alphabetical sort

### Sort Mode Switching
- [x] Add sort mode toggle keyboard shortcuts
- [x] Implement Shift+Cmd+A for alphabetical (Shift+Cmd+A on macOS)
- [x] Implement Shift+Cmd+M for modified date (Shift+Cmd+M on macOS)
- [x] Maintain current image when switching sort modes
- [x] Update display to show current sort mode

### Window Title
- [x] Update window title with current image name
- [x] Show position in list (e.g., "image.png (3/10)")
- [x] Update title on navigation

### Additional Image Formats
- [x] Add TIFF/TIF support
- [x] Add ICO support
- [x] Add WEBP support (static images only initially)
- [x] Add GIF support (static - first frame only initially)

## Phase 4: Zoom & Pan Fundamentals

### Zoom Infrastructure
- [ ] Add zoom level to AppState
- [ ] Implement zoom calculations (10% to 2000% range)
- [ ] Add zoom transformation to image rendering

### Fit-to-Window Zoom (Priority)
- [ ] Calculate fit-to-window zoom level
- [ ] Implement initial fit-to-window on image load
- [ ] Center image when fit-to-window
- [ ] Handle window resize events
- [ ] Update fit-to-window on resize

### Keyboard Zoom
- [ ] Implement + key for zoom in
- [ ] Implement - key for zoom out
- [ ] Implement 0 key for reset to fit-to-window
- [ ] Add logarithmic zoom stepping
- [ ] Center zoom on viewport center

### Zoom Display
- [ ] Create zoom indicator component
- [ ] Position in bottom-right corner
- [ ] Show current zoom percentage
- [ ] Show "Fit" when at fit-to-window size

### Basic Pan
- [ ] Add pan position (x, y) to AppState
- [ ] Implement pan offset in image rendering
- [ ] Add Shift + arrow key panning
- [ ] Set base pan speed (10 pixels)
- [ ] Constrain pan to keep image visible

## Phase 5: Per-Image State Management

### ImageState Structure
- [ ] Create ImageState struct
- [ ] Add zoom level field
- [ ] Add pan position (x, y) fields
- [ ] Add last accessed timestamp

### State Cache
- [ ] Create LRU cache for ImageState (1000 items)
- [ ] Implement cache eviction strategy
- [ ] Track cache size and statistics

### State Persistence
- [ ] Implement save_current_image_state() in AppState
- [ ] Implement load_current_image_state() in AppState
- [ ] Save state when navigating away from image
- [ ] Load state when navigating to image
- [ ] Handle missing state (use defaults)

### State Integration
- [ ] Apply loaded zoom/pan state to viewer
- [ ] Preserve zoom/pan when navigating back to image
- [ ] Reset state when opening new file set

## Phase 6: Advanced Zoom Features

### Mouse Wheel Zoom
- [ ] Detect mouse wheel events
- [ ] Implement Ctrl/Cmd modifier detection
- [ ] Calculate cursor position in image coordinates
- [ ] Implement cursor-centered zoom
- [ ] Use 1.1x zoom factor per scroll notch
- [ ] Prevent default scroll behavior when Ctrl/Cmd held

### Zoom Modifiers (Keyboard)
- [ ] Detect Shift modifier (faster zoom)
- [ ] Detect Ctrl/Cmd modifier (slower zoom)
- [ ] Detect Shift+Ctrl/Cmd (incremental 1% zoom)
- [ ] Adjust zoom step based on modifiers
- [ ] Apply modifier detection to +/- keys

### Z+Mouse Drag Zoom
- [ ] Detect Z key press
- [ ] Detect left mouse button down while Z held
- [ ] Track mouse movement during drag
- [ ] Calculate zoom change from movement (1% per 2px)
- [ ] Apply zoom centered on initial click position
- [ ] Handle mouse release to end zoom

## Phase 7: Advanced Pan Features

### Spacebar+Mouse Pan
- [ ] Detect spacebar press
- [ ] Detect left mouse button down while spacebar held
- [ ] Track mouse movement during drag
- [ ] Implement 1:1 pixel movement panning
- [ ] Update cursor during pan operation
- [ ] Handle mouse release to end pan

### WASD/IJKL Panning
- [ ] Implement W/I key for pan up
- [ ] Implement A/J key for pan left
- [ ] Implement S/K key for pan down
- [ ] Implement D/L key for pan right
- [ ] Set base pan speed (10 pixels)

### Pan Speed Modifiers
- [ ] Detect Shift modifier (3x speed = 30px)
- [ ] Detect Ctrl/Cmd modifier (0.3x speed = 3px)
- [ ] Apply modifiers to WASD/IJKL panning

### Pan Constraints
- [ ] Implement pan boundaries
- [ ] Prevent panning completely off-screen
- [ ] Handle pan with different zoom levels

## Phase 8: User Interface Overlays

### Help Overlay
- [ ] Create HelpOverlay component
- [ ] Design help content layout
- [ ] List all keyboard shortcuts
- [ ] Implement H key toggle
- [ ] Implement ? key toggle
- [ ] Implement F1 key toggle
- [ ] Add click-outside-to-close functionality
- [ ] Proper z-order management

### Debug Overlay
- [ ] Create DebugOverlay component
- [ ] Show current image path and index
- [ ] Show current zoom level and pan position
- [ ] Show image dimensions
- [ ] Show viewport dimensions
- [ ] Implement F12 toggle
- [ ] Format debug info clearly

### Status Indicators
- [ ] Create status bar component (optional)
- [ ] Show current file name
- [ ] Show position in list
- [ ] Show sort mode indicator
- [ ] Position at top or bottom of window

## Phase 9: Filter System

### Filter Infrastructure
- [ ] Add filter settings to ImageState
- [ ] Create filter processing utilities
- [ ] Implement brightness adjustment (-100 to +100)
- [ ] Implement contrast adjustment (-100 to +100)
- [ ] Implement gamma correction (0.1 to 10.0)

### Filter Application
- [ ] Apply filters during image rendering
- [ ] Cache filtered images for performance
- [ ] Update display when filters change
- [ ] Handle filter state in per-image state

### Filter Controls UI
- [ ] Create FilterControls component
- [ ] Design slider UI for each filter
- [ ] Implement real-time filter preview
- [ ] Add numeric value display
- [ ] Implement Ctrl/Cmd+F toggle

### Filter State Management
- [ ] Implement Ctrl/Cmd+1 to disable filters
- [ ] Implement Ctrl/Cmd+2 to re-enable filters
- [ ] Preserve filter values when disabled
- [ ] Reset filters to defaults when needed
- [ ] Persist filters per-image

## Phase 10: File Operations

### Open File Dialog
- [ ] Add rfd dependency for native file dialogs
- [ ] Implement Ctrl/Cmd+O handler
- [ ] Configure dialog for image formats
- [ ] Handle single file selection
- [ ] Handle multiple file selection
- [ ] Replace navigation list with new selection
- [ ] Reset to first image after opening

### Save Functionality
- [ ] Implement Ctrl/Cmd+S handler
- [ ] Generate suggested filename (_filtered suffix)
- [ ] Handle existing file conflicts (auto-increment)
- [ ] Show native save dialog
- [ ] Support multiple output formats
- [ ] Apply current filters to saved image
- [ ] Handle save errors gracefully

### File Format Support
- [ ] PNG output
- [ ] JPEG output
- [ ] BMP output
- [ ] TIFF output
- [ ] WEBP output

## Phase 11: Animation Support

### GIF Animation
- [ ] Detect GIF animation (multi-frame)
- [ ] Extract all frames from GIF
- [ ] Parse frame timing information
- [ ] Create frame playback system
- [ ] Implement O key play/pause toggle
- [ ] Show frame counter display
- [ ] Add play/pause status indicator

### Animation Controls
- [ ] Implement [ key for previous frame
- [ ] Implement ] key for next frame
- [ ] Handle animation loop
- [ ] Pause on window focus loss
- [ ] Add startup timer (prevent early pause)

### WEBP Animation
- [ ] Detect WEBP animation
- [ ] Extract WEBP frames
- [ ] Parse WEBP timing metadata
- [ ] Integrate with animation system
- [ ] Add format indicator in display

### Animation State
- [ ] Add animation state to ImageState
- [ ] Track current frame
- [ ] Track play/pause state
- [ ] Persist animation state per-image

## Phase 12: Cross-Platform Polish

### Platform-Specific Keyboard
- [ ] Properly detect Cmd key on macOS
- [ ] Properly detect Ctrl key on Windows/Linux
- [ ] Handle both in all keyboard shortcuts
- [ ] Test keyboard shortcuts on all platforms

### Platform Integration
- [ ] Create platform-specific build configs
- [ ] Add native file associations
- [ ] Implement platform-specific icons
- [ ] Handle platform menu integration (macOS)

### Window Behavior
- [ ] Test window focus/activation on all platforms
- [ ] Test file dialogs on all platforms
- [ ] Test drag-and-drop (if supported)
- [ ] Handle high-DPI displays properly

## Phase 13: Performance Optimization

### Image Loading
- [ ] Implement async image loading
- [ ] Add background thread for loading
- [ ] Show loading spinner/progress
- [ ] Cancel loading if navigation occurs

### Memory Management
- [ ] Monitor memory usage
- [ ] Implement image cache eviction
- [ ] Unload off-screen images
- [ ] Optimize filtered image caching

### Rendering Performance
- [ ] Profile render performance
- [ ] Optimize zoom/pan calculations
- [ ] Reduce unnecessary re-renders
- [ ] Use GPU acceleration where possible

## Phase 14: Testing & Quality

### Unit Tests
- [ ] File operations tests
- [ ] Image loading tests
- [ ] State management tests
- [ ] Zoom/pan calculation tests
- [ ] Filter application tests

### Integration Tests
- [ ] CLI argument parsing tests
- [ ] File loading workflows
- [ ] Navigation workflows
- [ ] Zoom/pan workflows

### Platform Testing
- [ ] Test on macOS
- [ ] Test on Windows
- [ ] Test on Linux
- [ ] Test with various image formats
- [ ] Test with large images (>100MB)
- [ ] Test with large collections (>1000 files)

## Phase 15: Documentation & Release

### User Documentation
- [x] Installation instructions (in README.md planned)
- [x] Usage guide (in DESIGN.md)
- [x] Keyboard shortcuts reference (in DESIGN.md)
- [x] CLI documentation (in CLI.md)
- [ ] Troubleshooting guide

### Developer Documentation
- [x] Architecture overview (in DESIGN.md)
- [ ] Component documentation
- [ ] API documentation (rustdoc)
- [ ] Contribution guidelines

### Release Preparation
- [ ] Set up semantic versioning
- [ ] Create CHANGELOG.md
- [ ] Create release notes template
- [ ] Final testing on all platforms
- [ ] Security review
- [ ] Create installation packages
- [ ] Publish to crates.io

## Future Enhancements (Post-1.0)

### Advanced Features
- [ ] Custom filter plugins
- [ ] Batch processing
- [ ] RAW image format support
- [ ] SVG support
- [ ] Video thumbnail support
- [ ] GPU-accelerated filter pipeline (wgpu)

### Productivity Features
- [ ] Slideshow mode
- [ ] Image comparison (side-by-side)
- [ ] Metadata viewing/editing
- [ ] Print support
- [ ] Multi-monitor support
- [ ] Thumbnail grid view

### Quality of Life
- [ ] Undo/redo for filter changes
- [ ] Presets for common filter combinations
- [ ] Recent files list
- [ ] Favorites/bookmarks
- [ ] Image rotation
- [ ] Copy/paste image data

---

**Current Focus**: Phase 4 - Zoom & Pan Fundamentals

**Last Completed**: Phase 3 - Navigation & Sorting ✅

### Phase 3 Summary
Phase 3 has been successfully completed! The application now:
- Supports arrow key navigation (← → to navigate between images)
- Implements wrap-around navigation (loops from last to first image)
- Provides two sort modes: Alphabetical (case-insensitive) and Modified Date (newest first)
- Allows switching sort modes with keyboard shortcuts (Shift+Cmd+A and Shift+Cmd+M)
- Updates window title dynamically with current image name and position (e.g., "banana.png (2/7)")
- Maintains current image context when switching sort modes
- Supports additional image formats: TIFF, ICO, WEBP, and GIF (all formats supported by the image crate)
- Re-sorts image list when sort mode changes

Key implementation details:
- Navigation methods (`next_image()` and `previous_image()`) added to AppState (src/state/app_state.rs:60)
- Sort mode enumeration and sorting logic in AppState (src/state/app_state.rs:6)
- Action handlers for navigation and sorting in main.rs (src/main.rs:38)
- Keyboard bindings configured for navigation and sort mode switching (src/main.rs:171)
- Window title updates on every navigation or sort change (src/main.rs:67)
- Image format support expanded in Cargo.toml with explicit feature flags

### Phase 2 Summary
Phase 2 has been successfully completed! The application now:
- Loads and displays images using GPUI's `img()` function
- Automatically handles format conversion (RGBA to BGRA for GPU)
- Displays images with fit-to-window scaling using `ObjectFit::Contain`
- Shows image information (filename and dimensions) in an info panel
- Loads the first image automatically on startup
- Handles errors gracefully with informative error messages

The implementation uses the recommended Approach 1 from the research documentation, leveraging GPUI's built-in image loading, caching, and format conversion.
