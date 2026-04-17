# Testing Documentation

This document describes the test infrastructure for rpview.

## Overview

rpview has comprehensive test coverage with **~326 tests** covering all critical functionality:
- **215 unit tests** in the library crate (error, state, utils modules)
- **~111 integration and binary-crate tests** for end-to-end workflows and binary-only modules

## Running Tests

```bash
# Run all tests
cargo test

# Run tests quietly (summary only)
cargo test --quiet

# Run specific test file
cargo test --test file_operations_test

# Run specific test
cargo test test_zoom_in_normal

# Run tests in release mode
cargo test --release
```

## Test Organization

### Unit Tests (93 tests)

Unit tests are organized by module functionality:

#### File Operations Tests (13 tests)
**Location**: `tests/file_operations_test.rs`

Tests file system operations and image format detection:
- Image format detection (PNG, JPEG, GIF, BMP, TIFF, ICO, WEBP)
- Case-insensitive extension matching
- Directory scanning with filtering
- Alphabetical sorting (case-insensitive)
- Dropped file/directory processing
- Error handling for nonexistent paths
- Error handling for unsupported formats

#### State Management Tests (19 tests)
**Location**: `tests/state_management_test.rs`

Tests application state and per-image state persistence:
- AppState creation and initialization
- Navigation (next/previous with wraparound)
- Current image tracking
- Per-image state persistence (zoom, pan, filters, animation)
- LRU cache management (max 1000 items)
- Sort mode switching (alphabetical, modified date)
- Edge cases (empty lists, single image, invalid indices)

#### Zoom/Pan Tests (36 tests)
**Location**: `tests/zoom_pan_test.rs`

Tests zoom and pan calculations:
- Zoom constants validation (MIN_ZOOM=0.1, MAX_ZOOM=20.0)
- Zoom clamping to valid range
- Fit-to-window calculations for various image/viewport sizes
- Zoom in/out with different step sizes
- Modifier-based zoom (normal, fast, slow, incremental, wheel)
- Zoom percentage formatting
- Multiple zoom steps and reversibility
- Portrait/landscape/square image handling

#### Filter Tests (25 tests)
**Location**: `tests/filter_test.rs`

Tests image filter processing:
- Brightness adjustment (-100 to +100)
- Contrast adjustment (-100 to +100)
- Gamma correction (0.1 to 10.0)
- Combined filter application
- Alpha channel preservation
- Input clamping and validation
- Edge case handling (black, white, midtones)
- Lookup table optimization for gamma

### Integration Tests (36 tests)

Integration tests verify end-to-end workflows:

#### CLI Workflow Tests
**Location**: `tests/integration_test.rs`

Tests command-line interface workflows:
- Empty directory handling
- Directory with images
- Single file with parent directory scan
- Mixed file types filtering

#### File Loading Workflows

Tests image loading scenarios:
- Sequential image loading
- State persistence across navigation
- Drag-and-drop file processing

#### Navigation Workflows

Tests navigation functionality:
- Forward/backward navigation
- Wraparound at boundaries
- Sort mode changes
- Preload path calculation

#### Complete Workflows

Tests end-to-end user scenarios:
- Fit-to-window then manual zoom
- Toggle between fit and 100%
- Different zoom modifiers
- Pan with various zoom levels
- Load → Navigate → Zoom → Pan workflows

## Test Coverage

### What's Tested

✅ **File Operations**
- All supported image formats (PNG, JPEG, GIF, BMP, TIFF, ICO, WEBP)
- Directory scanning and filtering
- Alphabetical sorting (case-insensitive)
- File/directory drop handling

✅ **State Management**
- Image list navigation
- Per-image state persistence
- LRU cache eviction
- Sort mode switching

✅ **Zoom & Pan**
- Fit-to-window calculations
- Manual zoom with modifiers
- Zoom range validation (10%-2000%)
- Pan position tracking

✅ **Filters**
- Brightness, contrast, gamma processing
- Alpha channel preservation
- Input validation and clamping
- Combined filter application

✅ **Workflows**
- CLI argument parsing
- Image loading and navigation
- State persistence across sessions
- Error handling

### What's Not Tested

⚠️ **UI Components** (GPUI-specific)
- ImageViewer rendering
- Help/Debug overlays
- Filter controls UI
- Animation indicators

⚠️ **Platform-Specific**
- Native file dialogs (rfd)
- Native menus
- Drag-and-drop events (GPUI)
- High-DPI scaling

⚠️ **Performance**
- Large images (>100MB)
- Large collections (>1000 files)
- Memory usage profiling
- Render performance

## Test Infrastructure

### Dependencies

```toml
[dev-dependencies]
tempfile = "3.8"  # Temporary directories for file system tests
```

### Test Utilities

**Temporary Directories**: Tests use `tempfile::TempDir` for isolated file system operations:
```rust
let temp_dir = TempDir::new().unwrap();
let dir_path = temp_dir.path();
fs::write(dir_path.join("image.png"), b"fake png").unwrap();
```

**Test Images**: Tests create minimal fake image files (not valid images):
```rust
fn create_test_image(r: u8, g: u8, b: u8) -> DynamicImage {
    DynamicImage::ImageRgba8(ImageBuffer::from_pixel(10, 10, Rgba([r, g, b, 255])))
}
```

## Writing Tests

### Guidelines

1. **Descriptive Names**: Test names should clearly indicate what is being tested
   ```rust
   #[test]
   fn test_zoom_in_respects_max() { ... }
   ```

2. **Single Responsibility**: Each test should focus on one aspect
   ```rust
   #[test]
   fn test_apply_brightness_zero() {
       // Only test zero brightness (no change)
   }
   ```

3. **Arrange-Act-Assert**: Follow AAA pattern
   ```rust
   #[test]
   fn test_navigation_wraparound() {
       // Arrange
       let paths = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
       let mut state = AppState::new(paths);
       
       // Act
       state.current_index = 1;
       state.next_image();
       
       // Assert
       assert_eq!(state.current_index, 0);
   }
   ```

4. **Edge Cases**: Test boundary conditions
   ```rust
   #[test]
   fn test_navigation_empty_list() {
       let mut state = AppState::new(vec![]);
       state.next_image();  // Should not panic
       assert_eq!(state.current_index, 0);
   }
   ```

5. **Error Handling**: Test error conditions
   ```rust
   #[test]
   fn test_process_dropped_path_nonexistent() {
       let result = process_dropped_path(&PathBuf::from("/nonexistent"));
       assert!(result.is_err());
   }
   ```

### Example Test

```rust
#[test]
fn test_zoom_in_normal() {
    // Arrange: Start at 100% zoom
    let current = 1.0;
    
    // Act: Zoom in with normal step
    let zoomed = zoom_in(current, ZOOM_STEP);
    
    // Assert: Should be 120%
    assert_eq!(zoomed, 1.2);
}
```

## Test Results

### Current Status

```
Total Tests: ~326 (215 lib + 111 bin/integration)
├── Library Unit Tests: 215
│   ├── Error types: 10
│   ├── CLI: 1
│   ├── State (app_state): 30+
│   ├── State (image_state): 8
│   ├── State (settings): 16
│   ├── Utils — file_scanner: 15
│   ├── Utils — filters: 18
│   ├── Utils — zoom: 16
│   ├── Utils — color: 19
│   ├── Utils — float_map: 17
│   ├── Utils — local_contrast: 8
│   ├── Utils — lc_presets: 10
│   ├── Utils — animation: 11
│   ├── Utils — image_loader: 12
│   └── Utils — settings_io: 9
├── Binary Crate Tests: ~221 (includes lib tests + app_handlers)
│   └── app_handlers: 6 (window title formatting)
└── Standalone Integration Tests: 111
    ├── file_operations_test.rs: 13
    ├── filter_test.rs: 25
    ├── integration_test.rs: 18
    ├── state_management_test.rs: 19
    └── zoom_pan_test.rs: 36

Pass Rate: 100%
Execution Time: < 1 second
Platform: macOS (verified)
```

### Platform Testing

- ✅ **macOS**: All tests passing
- ⏳ **Windows**: Ready for testing (cross-platform code)
- ⏳ **Linux**: Ready for testing (cross-platform code)

## Continuous Integration

Tests are designed to run in CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Run tests
  run: cargo test --all-features
```

## Future Enhancements

### Additional Test Coverage

- [ ] Property-based testing (proptest)
- [ ] Fuzzing for image format parsing
- [ ] Performance benchmarks (criterion)
- [ ] Memory leak detection (valgrind)
- [ ] Code coverage reporting (tarpaulin)

### Platform Testing

- [ ] Automated Windows testing
- [ ] Automated Linux testing
- [ ] Large image stress tests (>100MB)
- [ ] Large collection tests (>1000 files)

### UI Testing

- [ ] GPUI component testing (if framework supports it)
- [ ] Screenshot comparison tests
- [ ] Accessibility testing

## Troubleshooting

### Common Issues

**Test fails on different platforms**:
- Use `TempDir` for file system tests (automatic cleanup)
- Sort file lists alphabetically for predictable order
- Don't rely on file modification times in tests

**Lifetime issues in tests**:
```rust
// ❌ Bad: Temporary value dropped
let pixel = result.to_rgba8().get_pixel(0, 0);

// ✅ Good: Bind to variable first
let rgba = result.to_rgba8();
let pixel = rgba.get_pixel(0, 0);
```

**Comparison warnings**:
```rust
// ❌ Bad: u8 is always 0-255
assert!(pixel[0] >= 0);

// ✅ Good: Test actual values
assert_eq!(pixel[0], 128);
```

## Resources

- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Rust By Example - Testing](https://doc.rust-lang.org/rust-by-example/testing.html)
- [cargo test documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
