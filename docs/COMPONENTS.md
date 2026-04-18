# Component Architecture

Developer reference for rpview's GPUI component system.

## Overview

rpview is built on [GPUI](https://github.com/nickel-org/gpui), a GPU-accelerated UI framework. Each component is a GPUI entity — a struct managed by the framework's entity system — that implements `Render` to produce its view tree each frame.

```
App (main window root)
├── ImageViewer          — image display, zoom, pan, filter/LC processing
├── HelpOverlay          — keyboard shortcut reference (F1)
├── DebugOverlay         — diagnostic info (F12)
├── ZoomIndicator        — zoom %, resolution, sort mode
├── AnimationIndicator   — frame counter for animated images
├── ProcessingIndicator  — progress bar during filter/LC work
├── LoadingIndicator     — spinner during async image load
├── ErrorDisplay         — error messages
├── SettingsWindow       — full settings UI (Cmd/Ctrl+,)
└── MenuBar              — application menu (Windows/Linux only)

FilterWindow (separate OS window, always-on-top)
└── FilterControls       — brightness/contrast/gamma sliders

LocalContrastWindow (separate OS window, always-on-top)
└── LocalContrastControls — LC parameter sliders and preset UI
```

## Core Pattern

### Entity lifecycle

Components are created with `cx.new(|cx| ...)` and stored as `Entity<T>` handles. The framework tracks ownership and notifies dependents when state changes:

```rust
// Creating a component
let viewer = cx.new(|cx| ImageViewer::new(settings, cx));

// Reading state
viewer.read(cx, |viewer, cx| { ... });

// Mutating state (triggers re-render)
viewer.update(cx, |viewer, cx| {
    viewer.set_zoom(2.0);
    cx.notify(); // signal the framework to re-render
});
```

### Event propagation

Components communicate via typed events using `EventEmitter<T>`:

```rust
// Component declares its event type
impl EventEmitter<FilterControlsEvent> for FilterControls {}

// Parent subscribes to events
cx.subscribe(&filter_controls, |this, _emitter, event, cx| {
    match event {
        FilterControlsEvent::FiltersChanged(settings) => {
            this.apply_filters(settings, cx);
        }
    }
});

// Component emits events
cx.emit(FilterControlsEvent::FiltersChanged(self.current_settings()));
```

### Rendering

Components implement `Render` to produce a view tree. GPUI re-calls `render()` whenever `cx.notify()` is invoked:

```rust
impl Render for MyComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(Colors::background())
            .child(self.render_content(cx))
    }
}
```

## Component Reference

### ImageViewer (`src/components/image_viewer.rs`)

The central component. Manages the loaded image, zoom/pan state, filter processing, LC processing, animation playback, and save/recall slots.

**Key state:**
- `loaded_image: Option<LoadedImage>` — currently displayed image with its raw pixel data
- `saved_slots: [Option<SavedSlot>; 7]` — save/recall slots (keys 3-9)
- Zoom, pan, and fit-to-window state are delegated to `ImageState` in the cache

**Background work:**
- Filter and LC processing run on background threads via `std::thread::spawn`
- Results are polled each frame in `check_filter_processing()` / `check_lc_processing()`
- Cancellation uses `Arc<AtomicBool>` flags checked at safe points in the worker

**GPUI interactions:**
- Receives mouse events for drag-to-pan and scroll-to-zoom
- Renders using `img()` with the image data converted to BGRA byte order

### FilterControls (`src/components/filter_controls.rs`)

Hosts three `Slider` entities (from ccf-gpui-widgets) for brightness, contrast, and gamma.

**Events:** `FilterControlsEvent::FiltersChanged(FilterSettings)`

**Slider ranges:**
- Brightness: -100.0 to +100.0 (default 0)
- Contrast: -100.0 to +100.0 (default 0)
- Gamma: 0.1 to 10.0 (default 1.0)

### FilterWindow (`src/components/filter_window.rs`)

A separate OS window (not an overlay) that hosts `FilterControls`. Uses platform-specific FFI to set always-on-top:
- macOS: `NSFloatingWindowLevel` via objc2
- Windows: `SetWindowPos` with `HWND_TOPMOST`
- Linux: GPUI's `WindowKind::Floating` hint

### LocalContrastControls (`src/components/local_contrast_controls.rs`)

Complex control panel with 17+ parameter sliders, a preset dropdown, preview toggle, and resize factor selector.

**Events:** `LocalContrastControlsEvent` with variants for parameter changes, preset operations, and preview toggle.

**Key UI elements:**
- Window/block size auto-toggles
- Alpha, contrast, shadow/highlight amount sliders
- Document mode toggle
- Preset dropdown with save/delete
- Resize factor segmented control (0.25x - 4.0x)
- Preview on/off toggle
- Progress bar with cancel button

### LocalContrastWindow (`src/components/local_contrast_window.rs`)

Separate always-on-top OS window hosting `LocalContrastControls`. Same platform FFI pattern as `FilterWindow`.

### SettingsWindow (`src/components/settings_window.rs`)

Full interactive settings UI rendered as an overlay on the main window. Organized into 8 sections with sidebar navigation.

**Sections:**
1. **Viewer Behavior** — default zoom mode, state cache, animation auto-play
2. **Performance** — preload, threads, max dimensions
3. **Keyboard & Mouse** — pan speeds, zoom sensitivity
4. **File Operations** — default save directory/format
5. **Appearance** — background color (with color picker), overlay transparency, font scale
6. **Filters** — default values for brightness/contrast/gamma
7. **Sort & Navigation** — default sort mode, wrap navigation
8. **External Tools** — viewer and editor configuration display

**Pattern:** Uses a working copy of `AppSettings`. Changes are buffered until Apply (Cmd/Ctrl+Enter) is pressed. Cancel (Escape) reverts to the original. Reset restores all defaults.

**Interactive controls:**
- Checkboxes for booleans
- `SegmentedControl` for enums (zoom mode, sort mode, save format)
- Numeric steppers with +/- buttons and range validation
- `ColorSwatch` from ccf-gpui-widgets for background color
- Per-setting reset buttons (↺ icon)

### HelpOverlay (`src/components/help_overlay.rs`)

Scrollable keyboard shortcut reference, toggled with F1. Renders 6+ sections covering navigation, zoom, pan, filters, file operations, and general controls. Uses platform-aware modifier key glyphs (⌘ vs Ctrl).

### DebugOverlay (`src/components/debug_overlay.rs`)

F12 diagnostic panel showing:
- Current image path and index
- Image dimensions (original and display)
- Zoom level and pan position
- Sort mode
- State cache size and hit rate

Configured via `DebugOverlayConfig`.

### ZoomIndicator (`src/components/zoom_indicator.rs`)

Bottom-corner indicator showing zoom percentage, image resolution, and current sort mode. Auto-hides after 3 seconds of inactivity.

### AnimationIndicator (`src/components/animation_indicator.rs`)

Shows `"Frame N / Total"` for animated GIF/WebP images. Only visible when an animated image is loaded.

### ProcessingIndicator (`src/components/processing_indicator.rs`)

Progress bar shown during filter or LC processing. Includes a cancel button. Appears automatically when background work is in progress.

### LoadingIndicator (`src/components/loading_indicator.rs`)

Pulsing green dot with a status message, shown during async image loading.

### ErrorDisplay (`src/components/error_display.rs`)

Simple error message display with a warning icon. Supports custom text color for contrast against different backgrounds.

### MenuBar (`src/components/menu_bar.rs`)

Custom-rendered menu bar for Windows and Linux (macOS uses native system menus via GPUI). Provides dropdown menus for File, View, and Help operations.

## Floating Window Pattern

Filter and LC panels use a shared pattern for always-on-top windows:

1. **Window creation** — opened via `cx.open_window()` with initial size and position
2. **Always-on-top** — `set_always_on_top()` in `src/utils/window_level.rs` uses platform FFI
3. **Communication** — the floating window holds an `Entity<T>` handle to its controls. The main `App` subscribes to events from the controls entity to receive parameter changes.
4. **Lifecycle** — closing the floating window destroys the view but the controls entity persists (owned by `App`), preserving slider state.

## State Flow

```
User input (keyboard/mouse)
    ↓
App handler (app_handlers.rs)
    ↓
ImageViewer mutation → cx.notify()
    ↓
ImageState update → cached in AppState LRU
    ↓
render() called → GPUI produces GPU draw commands
```

Settings flow separately:
```
SettingsWindow Apply → AppSettings saved to disk
    ↓
App reads new settings → applies to active viewer
```
