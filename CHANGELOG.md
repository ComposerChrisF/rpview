# Changelog

All notable changes to RPView will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.20.2] - 2026-04-18

### Fixed
- Keyboard shortcuts now work when the Filter or Local Contrast floating window has focus (arrows, zoom, pan, 1/2/3 slots, Cmd+F, Cmd+L, etc.)

## [0.20.0] - 2026-04-17

### Changed
- Click-and-drag now pans the image directly (spacebar no longer required)
- Local Contrast Preview defaults to OFF (use Cmd/Ctrl+P to apply, "2" to view)

### Added
- Cmd+P / Ctrl+P global shortcut to apply current LC settings without turning on Preview; result is accessible via the "2" key
- "Auto" resize option in LC panel: picks the factor (0.25x-4x) that brings the largest axis closest to 4K without exceeding it
- Per-image LC Preview on/off state: each image remembers whether Preview was enabled, restored when navigating back

## [0.19.0] - 2026-04-17

### Added
- Local Contrast processing for animated GIF/WebP images (Option D hybrid):
  - Auto-pauses animation when LC is enabled
  - Processes the current frame on demand (per-frame LC cache)
  - Frame stepping (arrow keys) triggers LC reprocessing with cache hits on revisit
  - "Process All Frames" batch button in LC panel with progress and cancellation
  - Animation playback with LC applied once all frames are processed
  - Toast message when attempting to resume playback before all frames are processed

### Fixed
- LC on animated images no longer shows "Image not loaded" errors
- LC no longer processes only frame 0 regardless of which frame is displayed

## [0.18.3] - 2026-04-17

### Changed
- Extracted `format_window_title` into dedicated `window_title` module with single-pass template expansion (was 5 chained `.replace()` allocations)
- Replaced `thread::sleep` in LRU eviction test with deterministic `Instant` backdating

### Fixed
- Strengthened cache eviction test assertion from `<=` to exact `assert_eq!`
- Filter extreme-values test now uses actual boundary values (±100 brightness/contrast, 0.1–10.0 gamma) plus beyond-clamp inputs
- Removed stale double blank lines left by prior test deletions
- Added comment explaining `recursion_limit = "256"` (required by GPUI derive macros)

## [0.18.2] - 2026-04-16

### Fixed
- Windows build: upgrade `windows` crate 0.58 → 0.61 to resolve windows-core version conflict with GPUI transitive dependencies

## [0.18.1] - 2026-04-16

### Changed
- Use Ctrl+3–9 for slot store on all platforms (was Cmd+3–9 on macOS)

## [0.18.0] - 2026-04-16

### Added
- Image save/recall slots: Ctrl+3–9 saves a snapshot of the current display (raw, filtered, or LC-processed) into a numbered slot; plain 3–9 recalls it
- Zoom and pan rescale automatically on slot transitions so the apparent image position stays constant

### Fixed
- LC parameter cache was never populated, causing every Preview re-enable to recompute from scratch
- `set_lc_enabled` and `check_lc_processing` now use `display_dimensions()` so LC changes don't incorrectly rescale zoom while a slot is displayed

## [0.17.4] - 2026-04-16

### Fixed
- Toggling Preview off no longer discards the rendered LC buffer — turning Preview back on with unchanged params is now instantaneous
- At most one LC worker thread is ever in flight; extra requests are queued, preventing memory spikes during rapid slider scrubbing

## [0.17.3] - 2026-04-16

### Changed
- When the effective image size changes (LC result with non-1× resize, or 1/2 A/B toggle), zoom is rescaled inversely so the image stays in the same screen position for clean A/B comparison

## [0.17.2] - 2026-04-16

### Changed
- Viewer now treats LC output pixel dimensions as the effective image size: 100% zoom, fit-to-window, pan constraints, and the resolution indicator all reflect the output, not the source

## [0.17.1] - 2026-04-16

### Changed
- Consolidated LC processing state into `LcJob` struct
- Cache planar FloatMap on LoadedImage to skip per-tick f32 conversion (saves 6–20 ms on large images)
- Add `Parameters::is_identity()` to replace duplicate noop checks

## [0.17.0] - 2026-04-16

### Changed
- ESC now closes Filter and Local Contrast windows before counting toward the triple-ESC quit sequence

## [0.16.1] - 2026-04-15

### Changed
- LC preset UI: dropdown and name input are now on separate rows for better readability

## [0.16.0] - 2026-04-15

### Added
- Local Contrast presets: save, load, and delete named parameter sets (stored as JSON in settings directory)
- Preset UI with dropdown selector, name input, Save/Delete buttons
- Preview toggle above the resize row — off suppresses LC render and cancels in-flight compute

### Fixed
- Moving alpha-black/white sliders or toggling document mode with other sliders at zero no longer silently produces no effect

## [0.15.0] - 2026-04-15

### Changed
- Remove fast-path algorithm (integral-image proxy diverged too much from faithful histogram equalization)
- Add pre-LC resize toggle (1/4×, 1/2×, 1×, 2×, 4×) using Lanczos3 resampling
- LC automatically re-applies when navigating to a new image

## [0.14.0] - 2026-04-15

### Added
- Reset button in Local Contrast dialog
- Collapsible Advanced section (Use Fast Path, Use Median Gray-Point, Document Mode)
- LC window position and open state persist across launches
- Help overlay entry for Shift+Cmd+L

## [0.13.0] - 2026-04-15

### Added
- Fast-path algorithm using integral-image box mean (orders-of-magnitude faster on large images)
- Progress percentage in LC status label ("Processing... 55%")
- 1/2 keys toggle Local Contrast for instant A/B comparison (works from LC and Filter dialogs too)

### Changed
- LC dialog default size increased to 380×440 to fit all controls

## [0.12.0] - 2026-04-15

### Added
- Local Contrast dialog (Shift+Cmd+L / Shift+Ctrl+L) with three sliders: Contrast, Lighten Shadows, Darken Highlights
- Background OkLCh processing on rayon thread pool with cancellation
- Status indicator (Processing / Ready)
- View menu entries: "Local Contrast..." and "Reset Local Contrast"

## [0.11.4] - 2026-04-15

### Changed
- Parallelize local-contrast hot paths with rayon (sRGB↔OkLCh conversion, histogram grid, per-pixel processing)
- Add cancellation support via feedback callback

## [0.11.3] - 2026-04-15

### Added
- Port of FraleyMusic-ImageDsp `LocallyNormalizeLuminance` algorithm to Rust using OkLCh color space
- Full parameter surface: contrast, document-contrast, lighten-shadows, darken-highlights, alpha-black/white, median/mean gray-point

## [0.11.2] - 2026-04-15

### Added
- `FloatMap`: planar f32 bitmap with exact round-trip to `RgbaImage`
- sRGB ↔ OkLCh color space conversion (Ottosson's Oklab matrices)
- `docs/local-contrast-spec.md`: language-neutral spec derived from C# original

## [0.11.1] - 2026-04-14

### Changed
- Replace temp-PNG round-trip with in-memory filter pipeline — filter ticks now take ~5–15 ms instead of 100–300 ms on large images
- Cache decoded source image per filter session

## [0.11.0] - 2026-04-14

### Changed
- Filter panel is now a separate floating always-on-top OS window (was an in-window overlay)
- Filter window position and open state persist across launches
- Platform-specific always-on-top: `NSFloatingWindowLevel` (macOS), `HWND_TOPMOST` (Windows), `WM_TRANSIENT_FOR` (Linux)

## [0.10.1] - 2026-04-14

### Changed
- Shift+WASD/IJKL fast-pan now scales by zoom level so each keypress steps a fixed number of image pixels regardless of zoom

## [0.10.0] - 2026-04-14

### Added
- Type+Alpha/Modified composite sort mode (Shift+Cmd+T): groups images by file type, then by alpha or mtime
- Plain 1/2 keys toggle filters (was Cmd+1/Cmd+2)

## [0.9.4] - 2026-03-15

### Fixed
- LRU cache never updated `last_accessed` on read, causing eviction of recently-viewed states

### Changed
- Optimize filter pixel iteration with direct slice access
- Gate diagnostic prints behind `debug_eprintln!` macro for silent release builds
- Reduce settings_window.rs by ~250 lines with `create_stepper!`/`create_toggle!` macros

## [0.9.3] - 2026-03-15

### Fixed
- Zoom Shift+minus keybindings not matching (Shift+- produces `_` on US keyboards)
- Pan shortcut help text showing modifier twice
- Clippy and formatting warnings

### Changed
- Help overlay uses platform-native glyphs (⇧/⌥ on macOS) for modifier keys

## [0.9.2] - 2026-03-15

### Fixed
- Shift+minus zoom-out keybinding not firing on US keyboards

## [0.9.1] - 2026-03-15

### Changed
- Default window title format now includes comma separator: `{filename} ({sm}, {index}/{total})`

## [0.9.0] - 2026-03-15

### Added
- `{sm}` and `{sortmode}` template parameters for window title format setting
- Sort mode indicator visible in title bar by default

## [0.8.5] - 2026-03-15

### Fixed
- Sort mode ignored on drag-drop and Finder "Open With" — images always appeared in alphabetical order regardless of setting

## [0.8.4] - 2026-03-14

### Fixed
- Startup sort mode: CLI was pre-sorting alphabetically before AppState applied the configured default, so Modified Date mode still showed alphabetical order

## [0.8.3] - 2026-03-14

### Fixed
- Sort mode switching not preserving current image — `sort_images()` now restores position after reordering

## [0.8.2] - 2026-03-14

### Added
- Sort mode indicator in debug overlay (F12)

## [0.8.1] - 2026-03-13

### Changed
- Code review round 3: deduplicate file dialog extensions, atomic SVG temp filenames, reuse preload Vec per frame, persistent DebugOverlay entity

## [0.8.0] - 2026-03-13

### Added
- Windows icon and version resource embedding via winresource crate
- ICO generation tooling and security reviews

## [0.7.11] - 2026-03-12

### Fixed
- Windows menu dropdowns rendering behind content (wrap in `deferred(anchored())`)
- Windows action dispatch not firing from menu items
- Arrow key navigation not reachable in focus chain on Windows

## [0.7.10] - 2026-03-12

### Fixed
- Windows test binaries silently swallowing output due to `/SUBSYSTEM:WINDOWS` linker flag applying to all targets

## [0.7.9] - 2026-03-11

### Changed
- Rename package from `rpview-gpui` to `rpview`

## [0.7.8] - 2026-03-10

### Fixed
- Cross-platform compilation issues (borrow, context, mutability)
- Menu bar return types on Linux/Windows (`Div` → `Stateful<Div>`)
- CI: Windows/Linux menu_bar Clone, security audit ignores, Metal toolchain step
- Drag sentinel at (0,0), format_shortcut double-Cmd, mutex unwrap panic, animation eprintln in release

### Changed
- Split main.rs from 2834 → 446 lines (new `app_handlers.rs`, `app_render.rs`, `app_keybindings.rs`)
- macOS shortcuts use native ⌘⇧⌥ glyphs in UI
- Deduplicate pan/zoom/filter/file-import handlers

## [0.7.7] - 2026-03-09

### Changed
- Code review round 2: clippy fixes, dead code removal, let-else patterns, `sort_by_cached_key` for performance, eliminate double GIF decode (-598 lines)

## [0.7.6] - 2026-03-09

### Changed
- Switch ccf-gpui-widgets to crates.io dependency; fix CI workflows

## [0.7.5] - 2026-03-09

### Fixed
- Toggle reset button alignment
- Show app version in Settings header

## [0.7.4] - 2026-03-09

### Changed
- Settings rows restructured to proper two-column layout with reset buttons in left margin

## [0.7.3] - 2026-03-09

### Changed
- Reset buttons placed in fixed-width left margin so setting titles align

## [0.7.2] - 2026-03-09

### Changed
- Per-setting reset buttons moved to left of property titles

## [0.7.1] - 2026-03-09

### Fixed
- Reset button icon size increased from 12px to 15px
- Text contrast on light backgrounds: added `Colors::text_for_background()` for luminance-aware text color

## [0.7.0] - 2026-03-09

### Added
- Per-setting reset buttons ("↺") in Settings window — grayed when at default, active with green hover when changed

## [0.6.3] - 2026-03-09

### Fixed
- Cmd+0 always called `set_one_hundred_percent` on both branches, never toggling back to fit-to-window
- `0` key used `adjust_pan_for_zoom` when switching to fit-to-window instead of fully centering

## [0.6.2] - 2026-03-09

### Fixed
- Keyboard zoom (+/-) now anchors on viewport center instead of image center, matching scroll-wheel zoom behavior

## [0.6.1] - 2026-03-09

### Fixed
- Pan direction mode signs inverted (pan values are CSS left/top, not scroll offsets)

## [0.6.0] - 2026-03-09

### Added
- Pan direction mode setting: Move Image vs Move Viewport (WASD/IJKL behavior)

## [0.5.1] - 2026-02-22

### Changed
- Redesign delete confirmation as card with filename, full path, and styled red button
- Toast notifications moved to bottom-center

## [0.5.0] - 2026-02-22

### Added
- File delete with confirmation dialog (Cmd+Delete → Trash, Shift+Cmd+Delete → permanent)
- Toast notifications for delete outcomes
- ESC to cancel delete confirmation

## [0.4.1] - 2026-02-22

### Fixed
- ccf-gpui-widgets dependency path corrected after directory reorganization

## [0.4.0] - 2026-02-20

### Added
- Toggle zoom/size indicator visibility with `T` key
- Dark/light background toggle with `B` key (both colors configurable in Settings > Appearance)

## [0.3.1] - 2026-02-20

### Changed
- Version display moved from zoom indicator to help overlay

## [0.3.0] - 2026-02-20

### Added
- Dynamic SVG re-rendering at current zoom level for always-crisp vector display
- Viewport-only rendering with padding for large SVGs
- Background thread with debounce and cancel support

## [0.2.1] - 2026-02-19

### Fixed
- SVG text rendering: load system fonts into fontdb (was empty by default)
- Cache font database via OnceLock to avoid repeated ~50 ms font discovery

### Added
- Version indicator in zoom overlay
- SVG UTI in macOS Info.plist

## [0.2.0] - 2026-02-19

### Added
- SVG file support via resvg rasterization

## [0.1.0] - 2026-02-19

### Added
- Image viewing: PNG, JPEG, BMP, GIF (animated), TIFF, ICO, WebP (animated)
- Arrow key navigation with preloaded adjacent images (zero-latency transitions)
- Five-speed zoom (normal, fast, slow, incremental, scroll-wheel at cursor)
- Z+drag dynamic zoom
- WASD/IJKL pan with three speed tiers, Space+drag pan
- Per-image state memory (zoom, pan, filters) with LRU cache (1,000 images)
- Real-time brightness, contrast, and gamma filters (background thread processing)
- Animated GIF/WebP playback with play/pause (`O`) and frame stepping (`[`/`]`)
- File operations: open dialog, save with filters, save to Downloads
- External viewer hand-off (Cmd+Alt+V, with optional quit)
- External editor (Cmd+E)
- Reveal in Finder/Explorer (Cmd+R)
- Drag and drop (files, folders, or mixed)
- Help overlay (H/?/F1), debug overlay (F12)
- Alphabetical and modified-date sorting (Shift+Cmd+A / Shift+Cmd+M)
- Interactive settings window (Cmd+,) with full UI for all 30+ settings
- GPU texture preloading for instant navigation
- Progressive animation frame caching (3-phase strategy)
- Async image loading with cancellation
- Cross-platform support (macOS, Windows, Linux)
- macOS .app bundle with DMG creation
- Windows icon and version resource embedding
- macOS "Open With" / Finder integration
- CI/CD with GitHub Actions
