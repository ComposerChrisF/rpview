# Drag and Drop File Support Research

This document contains research findings about implementing drag-and-drop file support in GPUI applications.

## Overview

GPUI (the GPU UI framework from Zed Industries) supports drag-and-drop operations for files from the OS file manager (Finder on macOS, File Explorer on Windows, etc.).

## Key GPUI Types and APIs

### ExternalPaths

GPUI provides an `ExternalPaths` type which represents:
> "A collection of paths from the platform, such as from a file drop"

This is the primary type you'll receive when files are dropped onto your application window.

### Event Handler

GPUI elements support an `.on_drop()` event handler for receiving dropped files:

```rust
div()
    .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
        // Handle dropped file paths
        // paths contains PathBuf instances for each dropped file/directory
    }))
```

## Official Examples

### GPUI Drag-Drop Example

The GPUI crate includes a drag_drop example that demonstrates the functionality:

**Location**: `crates/gpui/examples/drag_drop.rs` in the Zed repository

**Run command**:
```bash
cargo run -p gpui --example drag_drop
```

This example shows:
- How to set up the `.on_drop()` handler
- How to extract paths from `ExternalPaths`
- Visual feedback during drag operations
- Handling multiple dropped files

## Useful Resources

### Documentation

1. **GPUI Rust Docs**: https://docs.rs/gpui
   - Official API documentation
   - Search for "ExternalPaths", "on_drop", and drag-related types

2. **GPUI Website**: https://www.gpui.rs/
   - High-level overview and getting started guides

3. **Zed Blog - GPUI Ownership**: https://zed.dev/blog/gpui-ownership
   - Explains ownership and data flow patterns in GPUI
   - Helpful for understanding how to structure event handlers

### Source Code Examples

1. **Zed Repository Examples**: https://github.com/zed-industries/zed/tree/main/crates/gpui/examples
   - Official examples including drag_drop.rs
   - Best place to see real working code

2. **Zed Source Code**: https://github.com/zed-industries/zed
   - Search for "on_drop" to see production usage in Zed editor
   - Shows advanced patterns and edge case handling

### Community Discussions

1. **Files panel drag and drop support**: https://github.com/zed-industries/zed/issues/7386
   - Discussion about implementing drag-and-drop in Zed's file panel
   - Contains implementation ideas and challenges

2. **Feature Request: Support for Drag and Drop Functionality**: https://github.com/zed-industries/zed/issues/16830
   - General discussion about drag-and-drop features
   - Community requests and use cases

3. **Drag'n'drop file to Zed window**: https://github.com/zed-industries/zed/issues/6069
   - Specific discussion about dropping files into Zed
   - Platform-specific considerations

4. **Keep drag cursor style when dragging**: https://github.com/zed-industries/zed/pull/24797
   - Recent PR improving drag cursor behavior
   - Shows current state of drag-and-drop polish

### Community Support

- **Zed Discord**: Best place to ask GPUI-specific questions
  - Active developers respond to questions
  - Can get help with implementation details
  - Link available from https://zed.dev/

## Implementation Strategy for rpview-gpui

### Current Architecture Advantages

The rpview-gpui codebase already has most of the necessary infrastructure:

1. **Directory Scanning**: `cli.rs` has logic to scan directories for images
2. **File Loading**: `handle_open_file()` demonstrates file list replacement
3. **Navigation Updates**: `update_viewer()` and `update_window_title()` handle state changes
4. **Image State Management**: Per-image caching system is already in place

### Recommended Implementation Steps

1. **Add `.on_drop()` handler** to the main div in `App::render()`
   ```rust
   .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
       this.handle_dropped_files(paths, window, cx);
   }))
   ```

2. **Create `handle_dropped_files()` method**:
   - Extract paths from `ExternalPaths`
   - Determine if dropped item is file or directory
   - If file: get parent directory, scan for all images, find index of dropped file
   - If directory: scan directory for all images, start at index 0
   - Reuse `cli::scan_directory_for_images()` logic
   - Update `app_state.image_paths` and `app_state.current_index`
   - Call `update_viewer()` and `update_window_title()`

3. **Add visual feedback** (optional but recommended):
   - Highlight window border during drag-over
   - Show drop indicator
   - Use GPUI's drag state tracking

4. **Handle edge cases**:
   - Empty directories
   - Non-image files dropped
   - Permission errors
   - Multiple files/directories dropped simultaneously

### Estimated Complexity

- **Difficulty**: Moderate (4-6 hours)
- **Main Challenge**: Understanding GPUI's event system
- **Code Reuse**: High - most logic already exists
- **Cross-Platform**: GPUI handles platform differences automatically

### Testing Checklist

- [ ] Drop single image file from Finder/Explorer
- [ ] Drop multiple image files
- [ ] Drop directory containing images
- [ ] Drop mixed files (images + non-images)
- [ ] Drop onto window while image is displayed
- [ ] Drop onto window when no image is loaded
- [ ] Test on macOS
- [ ] Test on Windows
- [ ] Test on Linux
- [ ] Verify navigation list updates correctly
- [ ] Verify current image index is set properly
- [ ] Verify window title updates

## Platform-Specific Notes

### macOS

- Native drag-and-drop support is mature
- Recent improvements in PR #24797 for cursor behavior
- File permissions handled by OS

### Windows

- Standard Windows drag-and-drop protocol
- GPUI abstracts platform differences
- Test with File Explorer

### Linux

- X11 and Wayland support may differ
- Test on both when possible
- File manager variations (Nautilus, Dolphin, Thunar)

## Future Enhancements

Once basic drag-and-drop is working:

1. **Visual Polish**:
   - Animated drop zone
   - Preview thumbnail during drag
   - Drop location indicator

2. **Advanced Features**:
   - Remember drop-to-open in recent files
   - Support dragging images out of the viewer
   - Drag to reorder in navigation list

3. **Integration**:
   - Combine with Cmd+O dialog in recent files list
   - Support drag-drop for save operations

## Last Updated

January 2025 - Research based on GPUI 0.2.2 and Zed source code as of early 2025.
