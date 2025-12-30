# Changelog

All notable changes to rpview-gpui will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Phase 13: Performance Optimization
- Async image loading with background threads
  - Non-blocking image loading via `load_image_async()` in `utils/image_loader.rs`
  - Cancellable loading operations with `LoaderHandle`
  - Loading spinner indicator component (`components/loading_indicator.rs`)
  - Main thread checks for completion in render loop without blocking
  - UI remains responsive during large image loads
- GPU texture preloading for smooth navigation
  - Eliminates black flash when navigating between images
  - Preloads next/previous images into GPU cache during render loop
  - Off-screen rendering at `left(-10000px)` with `opacity(0.0)`
  - Seamless, instant transitions with 0ms black flash
  - Only 2 additional GPU textures in memory (minimal overhead)
- Progressive animation frame caching (3-phase strategy)
  - Phase 1: Cache first 3 frames immediately (~100-200ms)
  - Phase 2: Look-ahead caching of next 3 frames during playback
  - Phase 3: GPU preloading of next frame to prevent black flash
  - Result: Fast initial display + smooth playback + zero flashing
- Documentation
  - `docs/GPU_TEXTURE_PRELOADING.md` - Comprehensive GPU preloading guide
  - Updated `docs/ANIMATION_IMPLEMENTATION.md` with 3-phase caching details
  - Inline code comments documenting async loading and preloading systems

#### Phase 1: Project Foundation & Basic Structure
- Basic GPUI application structure with window management
- Window close handling (Cmd/Ctrl+W, Cmd/Ctrl+Q)
- Triple-escape quit functionality (3x within 2 seconds)
- Error handling types and utilities (`AppError`, `AppResult`)
- Styling and layout framework (colors, spacing, text sizes)
- State management structures:
  - `AppState` for application-wide state
  - `ImageState` for per-image state with LRU caching
  - `FilterSettings` for image filters
  - `AnimationState` for animated images
  - `SortMode` for image list sorting
- Component structure planning and organization
- CLI argument parsing with clap:
  - Support for no arguments (defaults to current directory)
  - Support for single file argument
  - Support for multiple file arguments
  - Support for directory arguments
  - Support for mixed file/directory arguments
  - Automatic image file detection by extension
- Project documentation:
  - DESIGN.md (application design and architecture)
  - CLI.md (command-line interface documentation)
  - TODO.md (development roadmap with 15 phases)
  - CONTRIBUTING.md (contribution guidelines)
  - CHANGELOG.md (this file)

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A

## [0.1.0] - TBD

### Added
- Initial release (planned)

---

## Version History

- **Unreleased**: Active development (Phase 1 complete)
- **0.1.0**: Planned initial release

## How to Read This Changelog

- **Added**: New features
- **Changed**: Changes to existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security vulnerability fixes

## Links

- [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
- [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
