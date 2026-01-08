# rpview-gpui

A fast, keyboard-driven image viewer built with GPUI.

**Status**: 🚀 Phase 14 Complete - Production-ready with comprehensive testing

## Features

- ✅ ⌨️ Keyboard-first navigation
- ✅ 🖼️ Support for multiple image formats (PNG, JPEG, BMP, GIF, TIFF, ICO, WEBP)
- ✅ 🔍 Advanced zoom and pan controls
- ✅ 🎨 Real-time image filters (brightness, contrast, gamma)
- ✅ 📁 Directory browsing with multiple sort modes
- ✅ 💾 Per-image in-memory state persistence (zoom, pan, filters)
- ✅ 🎬 Animated image support (GIF, WEBP)
- ✅ 🖱️ Drag-and-drop file/folder support
- ✅ 🌐 Cross-platform (macOS, Windows, Linux)
- ✅ ⚡ GPU-accelerated rendering with texture preloading
- ✅ 🧪 Comprehensive test coverage (129 tests)

## Current Status (Phase 14 ✅)

rpview-gpui is feature-complete and production-ready with comprehensive test coverage!

**Completed Phases:**
- ✅ Phase 1-14: All core features implemented
- ✅ 129 tests (100% passing)
- ✅ Cross-platform support
- ✅ GPU texture preloading for instant navigation
- ✅ Comprehensive documentation

See [TODO.md](TODO.md) for detailed phase summaries.

## Installation

### Prerequisites

- Rust (latest stable) - [Install Rustup](https://rustup.rs/)
- Platform-specific dependencies:
  - **macOS**: Xcode Command Line Tools
  - **Linux**: Development packages for X11
  - **Windows**: Visual Studio Build Tools

### Build

```bash
cargo build --release
```

## Usage

### Basic Usage

```bash
# View images in current directory
cargo run

# View all images in a directory, starting with a specific image
cargo run -- image.png

# View multiple images
cargo run -- img1.png img2.jpg img3.bmp

# View all images in a directory
cargo run -- /path/to/images

# Mixed files and directories
cargo run -- img1.png /path/to/images img2.jpg
```

### Help

```bash
cargo run -- --help
```

## Keyboard Shortcuts

### Currently Implemented
- `Cmd/Ctrl+W` - Close window
- `Cmd/Ctrl+Q` - Quit application
- `ESC` x3 (within 2 seconds) - Quick quit

### Navigation
- `←` / `→` - Navigate between images
- Drag & drop files/folders to open

### Zoom
- `=` / `-` - Zoom in/out
- `0` - Toggle fit-to-window / 100%
- `Ctrl/Cmd` + Mouse Wheel - Zoom at cursor
- `Z` + Mouse Drag - Dynamic zoom
- Shift/Ctrl modifiers for fine control

### Pan
- `W/A/S/D` or `I/J/K/L` - Pan image
- Space + Mouse Drag - Pan image
- Shift/Ctrl modifiers for speed control

### Filters
- `Ctrl/Cmd+F` - Toggle filter controls
- `Ctrl/Cmd+1` - Disable filters
- `Ctrl/Cmd+2` - Enable filters
- `Shift+Ctrl/Cmd+R` - Reset filters

### Animation
- `O` - Play/pause animation
- `[` / `]` - Previous/next frame

### Sorting (affects navigation to next image)
- `Shift+Cmd/Ctrl+A` - Alphabetical sort
- `Shift+Cmd/Ctrl+M` - Modified date sort

### File Operations
- `Ctrl/Cmd+O` - Open file(s)
- `Ctrl/Cmd+S` - Save file (current folder)
- `Ctrl/Cmd+Option+S` - Save to Downloads folder
- `Ctrl/Cmd+R` - Reveal in Finder/Explorer
- `Ctrl/Cmd+Option+V` - Open in external viewer
- `Shift+Ctrl/Cmd+Option+V` - Open in viewer and quit
- `Ctrl/Cmd+E` - Open in external editor

### Help & Info
- `H`, `?`, `F1` - Toggle help overlay
- `F12` - Toggle debug overlay
- `Cmd/Ctrl+,` - Open settings window

See [DESIGN.md](DESIGN.md) for complete keyboard shortcuts.

## Settings

RPView is highly customizable through its settings system. Press `Cmd+,` (macOS) or `Ctrl+,` (Windows/Linux) to open the interactive settings window.

### Settings Location

Settings are stored in a JSON file at:
- **macOS**: `~/Library/Application Support/rpview/settings.json`
- **Linux**: `~/.config/rpview/settings.json`
- **Windows**: `C:\Users\<User>\AppData\Roaming\rpview\settings.json`

### Configurable Settings

**Viewer Behavior**
- Default zoom mode (fit-to-window or 100%)
- Per-image state persistence
- Animation auto-play
- State cache size

**Performance**
- Adjacent image preloading
- Filter processing threads
- Maximum image dimensions

**Keyboard & Mouse**
- Pan speeds (normal, fast, slow)
- Zoom sensitivity (scroll wheel, Z-drag)
- Pan acceleration

**File Operations**
- Default save directory
- Default save format
- External viewer/editor integration

**Appearance**
- Background color
- Overlay transparency
- Font size scaling
- Window title format

**Filters**
- Default brightness/contrast/gamma values
- Filter presets (planned)

**Navigation**
- Default sort mode
- Wrap-around navigation
- Image counter display

### Settings Window Features

- ✅ **Interactive controls**: Checkboxes, radio buttons, numeric inputs
- ✅ **Live preview**: See changes before applying
- ✅ **Apply/Cancel/Reset**: Full control over changes
- ✅ **Keyboard shortcuts**: `Cmd+Enter` to apply, `Esc` to cancel
- ✅ **Auto-save**: Settings persist across restarts

### Manual Editing

You can also edit `settings.json` directly with any text editor. The file uses standard JSON format with pretty-printing for readability. Changes take effect on next launch.

**Example**: Change default zoom mode to 100%
```json
{
  "viewer_behavior": {
    "default_zoom_mode": "OneHundredPercent",
    ...
  }
}
```

### Documentation

See [docs/SETTINGS.md](docs/SETTINGS.md) for:
- Complete settings reference
- All available options and ranges
- Example configurations
- Troubleshooting guide
- Manual editing tips

## Documentation

### User Documentation
- [DESIGN.md](DESIGN.md) - Application design and architecture
- [CLI.md](CLI.md) - Command-line interface specification
- [docs/SETTINGS.md](docs/SETTINGS.md) - Settings documentation and configuration guide
- [docs/TESTING.md](docs/TESTING.md) - Testing infrastructure and guidelines

### Developer Documentation
- [TODO.md](TODO.md) - Development roadmap with phase summaries
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [docs/CROSS_PLATFORM.md](docs/CROSS_PLATFORM.md) - Cross-platform support details
- [docs/GPU_TEXTURE_PRELOADING.md](docs/GPU_TEXTURE_PRELOADING.md) - GPU preloading implementation
- [docs/ANIMATION_IMPLEMENTATION.md](docs/ANIMATION_IMPLEMENTATION.md) - Animation frame caching

## Project Structure

```
rpview-gpui/
├── src/
│   ├── main.rs           # Application entry point
│   ├── error.rs          # Error handling
│   ├── cli.rs            # CLI argument parsing
│   ├── state/            # State management
│   ├── components/       # UI components (planned)
│   └── utils/            # Utilities (styling, etc.)
├── DESIGN.md             # Design documentation
├── TODO.md               # Development roadmap
└── Cargo.toml            # Dependencies
```

## Development Roadmap

The project was developed in 15 phases:

1. ✅ **Phase 1**: Project Foundation & Basic Structure
2. ✅ **Phase 2**: Basic Image Display
3. ✅ **Phase 3**: Navigation & Sorting
4. ✅ **Phase 4**: Zoom & Pan Fundamentals
5. ✅ **Phase 5**: Per-Image State Management
6. ✅ **Phase 6**: Advanced Zoom Features
7. ✅ **Phase 7**: Advanced Pan Features
8. ✅ **Phase 8**: User Interface Overlays
9. ✅ **Phase 9**: Filter System
10. ✅ **Phase 10**: File Operations
11. ✅ **Phase 11**: Animation Support
12. ✅ **Phase 11.5**: Drag & Drop Support
13. ✅ **Phase 12**: Cross-Platform Polish
14. ✅ **Phase 13**: Performance Optimization
15. ✅ **Phase 14**: Testing & Quality (129 tests)
16. 🎯 **Phase 15**: Documentation & Release (Next)

See [TODO.md](TODO.md) for detailed phase summaries.

## Technologies

- This app largely written via Claude Code 2, Sonnet 4.5
- [GPUI](https://www.gpui.rs/) - High-performance GPU-accelerated UI framework
- [image](https://docs.rs/image/) - Image decoding/encoding
- [clap](https://docs.rs/clap/) - Command-line argument parsing
- [rfd](https://docs.rs/rfd/) - Native file dialogs
- [adabraka-ui](https://docs.rs/adabraka-ui/) - UI components
- Rust 2024 Edition

## Testing

rpview-gpui has comprehensive test coverage with **129 tests**:
- 93 unit tests (file ops, state, zoom/pan, filters)
- 36 integration tests (CLI, workflows, navigation)

```bash
# Run all tests
cargo test

# Run tests with coverage summary
cargo test --quiet
```

See [docs/TESTING.md](docs/TESTING.md) for detailed testing documentation.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Links

- [GPUI Documentation](https://github.com/zed-industries/zed/tree/main/crates/gpui/docs)
- [GPUI Examples](https://github.com/zed-industries/zed/tree/main/crates/gpui/examples)
- [Rust Documentation](https://doc.rust-lang.org/)
