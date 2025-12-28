# Components Structure

This directory contains the UI components for rpview-gpui.

## Planned Components

### Core Components

- **ImageViewer** (`image_viewer.rs`) - Main component for displaying images
  - Renders the current image
  - Handles zoom and pan transformations
  - Shows loading states
  - Displays error messages for invalid images

- **ErrorDisplay** (`error_display.rs`) - Component for displaying error messages
  - File not found errors
  - Unsupported format errors
  - No images found messages
  - Permission denied errors

### Overlay Components (Future Phases)

- **HelpOverlay** (`help_overlay.rs`) - Keyboard shortcuts help
  - Toggle with H, ?, or F1
  - Lists all keyboard shortcuts
  - Click-outside-to-close functionality

- **DebugOverlay** (`debug_overlay.rs`) - Debug information display
  - Toggle with F12
  - Shows current image path, index, zoom, pan
  - Shows image and viewport dimensions

- **FilterControls** (`filter_controls.rs`) - Filter adjustment UI
  - Toggle with Ctrl/Cmd+F
  - Sliders for brightness, contrast, gamma
  - Real-time preview
  - Numeric value display

- **ZoomIndicator** (`zoom_indicator.rs`) - Zoom level display
  - Position in bottom-right corner
  - Shows current zoom percentage
  - Shows "Fit" when at fit-to-window size

- **StatusBar** (`status_bar.rs`) - Optional status bar
  - Shows current file name
  - Shows position in list (e.g., "3/10")
  - Shows current sort mode

## Component Organization

All components should:
- Implement the `Render` trait from GPUI
- Use the styling utilities from `utils/style.rs`
- Accept state via props or context
- Be composable and reusable
- Handle their own event listeners where appropriate

## Module Structure

```rust
// components/mod.rs
pub mod image_viewer;
pub mod error_display;
// Future additions:
// pub mod help_overlay;
// pub mod debug_overlay;
// pub mod filter_controls;
// pub mod zoom_indicator;
// pub mod status_bar;
```
