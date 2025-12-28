# rpview-gpui

A fast, keyboard-driven image viewer built with GPUI.

**Status**: 🚧 Phase 1 Complete - Foundation established

## Features (Planned)

- ⌨️ Keyboard-first navigation
- 🖼️ Support for multiple image formats (PNG, JPEG, BMP, GIF, TIFF, ICO, WEBP)
- 🔍 Advanced zoom and pan controls
- 🎨 Real-time image filters (brightness, contrast, gamma)
- 📁 Directory browsing with multiple sort modes
- 💾 Per-image state persistence (zoom, pan, filters)
- 🎬 Animated image support (GIF, WEBP)
- ⚡ Built on GPUI for native performance

## Current Status (Phase 1 ✅)

Phase 1 has been completed with the following implementations:

- ✅ Project foundation and module structure
- ✅ Error handling system
- ✅ State management architecture
- ✅ CLI argument parsing
- ✅ Styling framework
- ✅ Window management (Cmd/Ctrl+W to close, Cmd/Ctrl+Q to quit, triple-ESC to quit)
- ✅ Comprehensive documentation

See [PHASE1_SUMMARY.md](PHASE1_SUMMARY.md) for detailed information.

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

# View a single image
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

### Planned (Future Phases)
- Arrow keys - Navigate between images
- `+`/`-` - Zoom in/out
- `0` - Reset to fit-to-window
- Space + Mouse - Pan image
- `Z` + Mouse drag - Zoom
- `H`, `?`, `F1` - Show help
- `F12` - Debug overlay
- And many more... (see [DESIGN.md](DESIGN.md))

## Documentation

- [DESIGN.md](DESIGN.md) - Application design and architecture
- [CLI.md](CLI.md) - Command-line interface specification
- [TODO.md](TODO.md) - Development roadmap (15 phases)
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [PHASE1_SUMMARY.md](PHASE1_SUMMARY.md) - Phase 1 implementation summary

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

The project is being developed in 15 phases:

1. ✅ **Phase 1**: Project Foundation & Basic Structure
2. 🎯 **Phase 2**: Basic Image Display (Next)
3. ⏳ **Phase 3**: Navigation & Sorting
4. ⏳ **Phase 4**: Zoom & Pan Fundamentals
5. ⏳ **Phase 5**: Per-Image State Management
6. ⏳ **Phase 6**: Advanced Zoom Features
7. ⏳ **Phase 7**: Advanced Pan Features
8. ⏳ **Phase 8**: User Interface Overlays
9. ⏳ **Phase 9**: Filter System
10. ⏳ **Phase 10**: File Operations
11. ⏳ **Phase 11**: Animation Support
12. ⏳ **Phase 12**: Cross-Platform Polish
13. ⏳ **Phase 13**: Performance Optimization
14. ⏳ **Phase 14**: Testing & Quality
15. ⏳ **Phase 15**: Documentation & Release

See [TODO.md](TODO.md) for detailed task breakdowns.

## Technologies

- [GPUI](https://www.gpui.rs/) - High-performance UI framework
- [clap](https://docs.rs/clap/) - Command-line argument parsing
- Rust 2024 Edition

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

TBD

## Links

- [GPUI Documentation](https://github.com/zed-industries/zed/tree/main/crates/gpui/docs)
- [GPUI Examples](https://github.com/zed-industries/zed/tree/main/crates/gpui/examples)
- [Rust Documentation](https://doc.rust-lang.org/)
