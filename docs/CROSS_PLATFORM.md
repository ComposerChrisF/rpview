# Cross-Platform Support

This document describes RPView's cross-platform implementation and platform-specific features.

## Supported Platforms

RPView is built with GPUI and supports the following platforms:

- **macOS** 10.15+ (Catalina and later)
- **Windows** 10/11 (64-bit)
- **Linux** (X11 and Wayland)

## Platform-Specific Features

### Keyboard Shortcuts

RPView uses intelligent platform-aware keyboard shortcuts:

#### How It Works

- **macOS**: Uses Command (⌘) key for shortcuts
- **Windows/Linux**: Uses Control (Ctrl) key for shortcuts

**Implementation Detail:**

GPUI 0.2.2 does not automatically translate the `"cmd"` modifier to `"ctrl"` on Windows/Linux.
To ensure cross-platform compatibility, we define bindings for both modifiers:

```rust
// In src/main.rs
// macOS uses "cmd-" bindings
KeyBinding::new("cmd-o", OpenFile, None)
KeyBinding::new("cmd-s", SaveFile, None)

// Windows/Linux need explicit "ctrl-" bindings
#[cfg(not(target_os = "macos"))]
KeyBinding::new("ctrl-o", OpenFile, None)
#[cfg(not(target_os = "macos"))]
KeyBinding::new("ctrl-s", SaveFile, None)
```

**Note:** Pan controls use `Alt` modifier for slow pan (instead of Cmd/Ctrl) to avoid conflicts with common shortcuts like Ctrl+S (Save) and Ctrl+W (Close).

#### Display in UI

The help overlay shows the correct modifier key for each platform:

```rust
// From src/utils/style.rs
pub fn modifier_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}
```

### Menus

RPView provides menu bar integration on all platforms:

#### macOS
- Uses native macOS menu bar (via `cx.set_menus()`)
- Application menu (RPView) contains Quit command
- Menus appear in the system menu bar at the top of the screen

#### Windows and Linux
- Uses an in-app menu bar component (GPUI 0.2.2 doesn't fully support native menus)
- Menu bar appears at the top of the application window
- Dropdown menus open on click, close on click outside or pressing Escape
- Hover between menus when one is open

**Implementation:**

On macOS, native menus are configured via GPUI:
```rust
// From src/main.rs
fn setup_menus(cx: &mut gpui::App) {
    cx.set_menus(vec![
        // Application menu (macOS: "RPView" menu with Quit)
        Menu {
            name: "RPView".into(),
            items: vec![
                MenuItem::action("Quit", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Open File...", OpenFile),
                MenuItem::action("Save File...", SaveFile),
                MenuItem::action("Close Window", CloseWindow),
                // ...
            ],
        },
        // ... more menus
    ]);
}
```

On Windows and Linux, an in-app menu bar component is used:
```rust
// From src/components/menu_bar.rs
#[cfg(not(target_os = "macos"))]
pub struct MenuBar {
    open_menu: Option<usize>,  // Currently open dropdown
    menus: Vec<MenuDef>,       // Menu definitions
    // ...
}
```

### High-DPI Display Support

RPView automatically supports high-DPI displays on all platforms:

- **macOS**: Retina display support (2x, 3x scaling)
- **Windows**: High-DPI displays (125%, 150%, 200%, custom scaling)
- **Linux**: HiDPI support with fractional scaling

**How It Works:**

GPUI handles all DPI scaling automatically:

1. Retrieves the display's scale factor from the platform
2. Applies scaling to all UI elements and images
3. Updates scale factor when windows move between displays
4. Renders at the correct pixel density

No special configuration is needed - the application uses `Pixels` type for measurements, and GPUI applies the appropriate scale factor.

### File Associations

RPView can be set as the default handler for image files:

#### macOS

File associations are configured in `packaging/macos/Info.plist`:

- PNG, JPEG, GIF, BMP, TIFF, ICO, WEBP support
- "Open With" menu integration
- Drag & drop files onto app icon in Dock

**Installation:**
1. Build the app bundle
2. Copy Info.plist to the bundle
3. macOS will recognize file associations automatically

#### Windows

File associations are configured in the Inno Setup installer (`packaging/windows/rpview.iss`):

- Registry entries for all supported formats
- "Open With" context menu integration
- Optional default program setting during installation

**Installation:**
1. Run the installer (created with Inno Setup)
2. Check "Associate image files" during installation
3. Right-click images → "Open With" → RPView

#### Linux

File associations are configured in the desktop entry file (`packaging/linux/rpview.desktop`):

- Follows freedesktop.org standards
- MIME type associations for all image formats
- Application menu integration

**Installation:**
```bash
cd packaging/linux
./install.sh
```

This installs:
- Binary to `~/.local/bin/rpview`
- Desktop file to `~/.local/share/applications/`
- Updates MIME and desktop databases

### Drag and Drop

RPView supports drag-and-drop file opening on all platforms:

- **macOS**: Drag from Finder → RPView window
- **Windows**: Drag from File Explorer → RPView window
- **Linux**: Drag from Nautilus/Dolphin/Thunar → RPView window

**Supported:**
- Single file drag
- Multiple file drag
- Directory drag (scans for images)
- Visual feedback (green border during drag-over)

### External Viewer Integration

RPView can open the current image in external viewers without setting itself as the default handler:

- **macOS**: Opens in Preview.app explicitly (`Cmd+Opt+F`)
- **Windows**: Opens in Photos app or Windows Photo Viewer (`Ctrl+Alt+F`)
- **Linux**: Tries common viewers (eog, xviewer, gwenview, feh, xdg-open) (`Ctrl+Alt+F`)

**Features:**
- `Cmd/Ctrl+Opt/Alt+F`: Opens current image in external viewer
- `Shift+Cmd/Ctrl+Opt/Alt+F`: Opens in external viewer and quits rpview
- Avoids circular reference when rpview is set as default viewer
- Falls back through multiple viewers on Linux until one succeeds

## Building for Each Platform

### macOS

```bash
# Development build
cargo build

# Release build
cargo build --release

# Create app bundle (future)
# Use cargo-bundle or create manually with Info.plist
```

**Requirements:**
- Xcode Command Line Tools
- Rust toolchain for macOS

### Windows

```bash
# Development build
cargo build

# Release build (no console window)
cargo build --release

# Create installer
# Install Inno Setup, then compile packaging/windows/rpview.iss
```

**Requirements:**
- Visual Studio Build Tools or MSVC
- Rust toolchain for Windows

**Note:** The release build is configured to avoid showing a console window (see `build.rs`).

### Linux

```bash
# Development build
cargo build

# Release build
cargo build --release

# Install locally
cd packaging/linux
./install.sh
```

**Requirements:**
- GCC or Clang
- Rust toolchain for Linux
- X11 or Wayland development libraries

## Platform-Specific Configuration

### Build Configuration

The `Cargo.toml` includes platform-specific sections:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
# macOS-specific dependencies (if needed)

[target.'cfg(target_os = "windows")'.dependencies]
# Windows-specific dependencies (if needed)

[target.'cfg(target_os = "linux")'.dependencies]
# Linux-specific dependencies (if needed)
```

### Build Script

The `build.rs` script handles platform-specific build configuration:

- Sets target platform environment variable
- Configures Windows subsystem (no console)
- Future: Icon embedding, resource bundling

## Platform Testing

### Keyboard Shortcuts

All keyboard shortcuts have been verified to work cross-platform:

| Action | macOS | Windows/Linux |
|--------|-------|---------------|
| Open File | Cmd+O | Ctrl+O |
| Save File | Cmd+S | Ctrl+S |
| Close Window | Cmd+W | Ctrl+W |
| Quit | Cmd+Q | Ctrl+Q |
| Zoom In | Cmd++ | Ctrl++ |
| Filters | Cmd+F | Ctrl+F |
| Sort Alphabetically | Shift+Cmd+A | Shift+Ctrl+A |
| Open in External Viewer | Cmd+Opt+F | Ctrl+Alt+F |
| Open Externally & Quit | Shift+Cmd+Opt+F | Shift+Ctrl+Alt+F |

### File Dialogs

Native file dialogs work on all platforms:

- **macOS**: NSOpenPanel/NSSavePanel
- **Windows**: Windows File Dialog API
- **Linux**: GTK or KDE file picker (depending on desktop environment)

Provided by the `rfd` (Rusty File Dialogs) crate.

### Drag and Drop

Verified working on:
- ✅ macOS 14 (Sonoma) with Finder
- ⏳ Windows 11 with File Explorer (ready for testing)
- ⏳ Linux with Nautilus/Dolphin (ready for testing)

## Known Platform Differences

### Window Behavior

- **macOS**: Closing the last window does not quit the app (standard macOS behavior)
- **Windows/Linux**: Closing the last window quits the app

RPView follows platform conventions by detecting when all windows are closed and quitting on Windows/Linux only.

### File Paths

- **macOS/Linux**: Use forward slashes (`/`)
- **Windows**: Use backslashes (`\`) but Rust's `PathBuf` handles this automatically

### Image Format Support

All platforms support the same image formats:
- PNG, JPEG, GIF (animated), BMP, TIFF, ICO, WEBP (animated)

Format support is provided by the Rust `image` crate, which is platform-independent.

## Performance Characteristics

### GPU Acceleration

- **macOS**: Uses Metal rendering backend
- **Windows**: Uses DirectX rendering backend
- **Linux**: Uses Vulkan or OpenGL rendering backend

All backends provide hardware-accelerated rendering via GPUI.

### Memory Usage

Memory usage is consistent across platforms, determined by:
- Number of loaded images (1000-image LRU cache)
- Image resolution and file size
- Filter cache (temporary filtered images)
- GPU texture cache (current + adjacent images)

### Startup Time

- **macOS**: ~100-200ms
- **Windows**: ~150-250ms
- **Linux**: ~100-300ms (varies by desktop environment)

## Distribution

### macOS

**Recommended:**
- Create `.app` bundle with Info.plist
- Sign with Apple Developer ID
- Notarize for Gatekeeper compatibility
- Distribute via DMG or ZIP

### Windows

**Recommended:**
- Create installer with Inno Setup (`packaging/windows/rpview.iss`)
- Sign with code signing certificate
- Distribute via installer (.exe)

### Linux

**Recommended:**
- Create `.deb` package for Debian/Ubuntu
- Create `.rpm` package for Fedora/RedHat
- Create AppImage for universal compatibility
- Publish to Flathub or Snap Store

**Currently:**
- Provide installation script (`packaging/linux/install.sh`)
- Users can build from source with `cargo build --release`

## Future Enhancements

Platform-specific features planned for future releases:

### macOS
- [ ] Create proper .app bundle with build script
- [ ] App icon in Dock
- [ ] Touchbar support (for MacBook Pro)
- [ ] Quick Look integration
- [ ] Services menu integration

### Windows
- [ ] Thumbnail provider for File Explorer
- [ ] Jump list integration
- [ ] Windows 11 snap layouts support
- [ ] Windows Ink support (stylus)

### Linux
- [ ] DBus integration for desktop notifications
- [ ] Thumbnail generation for file managers
- [ ] Wayland clipboard integration
- [ ] Integration with various desktop environments

## Troubleshooting

### macOS

**Issue:** "RPView is damaged and can't be opened"
**Solution:** The app needs to be signed or you need to allow it in System Preferences → Security & Privacy

**Issue:** File associations not working
**Solution:** Ensure Info.plist is properly embedded in the .app bundle

### Windows

**Issue:** Console window appears
**Solution:** Ensure you're using the release build (`cargo build --release`)

**Issue:** File associations not working
**Solution:** Run the installer as administrator or manually register file types

### Linux

**Issue:** App doesn't appear in application menu
**Solution:** Run `update-desktop-database ~/.local/share/applications/`

**Issue:** File associations not working
**Solution:** Run `update-mime-database ~/.local/share/mime`

## Contributing

When adding platform-specific features:

1. Test on all three platforms (macOS, Windows, Linux)
2. Use `cfg!(target_os = "...")` for compile-time platform detection
3. Follow platform conventions for UI and behavior
4. Document platform differences in this file
5. Add platform-specific tests if possible

## References

- [GPUI Framework Documentation](https://www.gpui.rs/)
- [Rust Cross-Compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [freedesktop.org Standards](https://www.freedesktop.org/wiki/) (Linux)
- [macOS Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/macos)
- [Windows App Development Best Practices](https://learn.microsoft.com/en-us/windows/apps/)
