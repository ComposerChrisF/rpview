# RPView

A fast, keyboard-driven image viewer for macOS, Windows, and Linux. Built for
people who browse lots of images and want something snappier and more capable
than the OS default — while keeping the default viewer one keypress away.

## Why RPView?

Your OS ships with an image viewer. It works fine for opening a single photo.
But if you regularly flip through directories of images, compare details at
different zoom levels, or adjust brightness on the fly, you'll hit its limits
fast. RPView fills the gap:

| | Preview.app / Photos / Eye of GNOME | RPView |
|---|---|---|
| **Navigation speed** | Loads each image on demand | Preloads adjacent images into GPU memory — navigation is instant |
| **Zoom precision** | Pinch or menu only | Five zoom speeds (keyboard), scroll-wheel zoom at cursor, Z+drag dynamic zoom |
| **Pan** | Scroll or trackpad | WASD/IJKL keys, Space+drag, three speed tiers |
| **Image filters** | None (need a separate editor) | Brightness, contrast, and gamma — live, per-image, remembered |
| **State memory** | Forgets zoom/pan when you move on | Remembers zoom, pan, and filter settings for up to 1,000 images |
| **Animated GIF/WebP** | Basic playback | Frame-by-frame stepping, play/pause, GPU-preloaded frames |
| **SVG rendering** | Static raster | Dynamic re-rendering at zoom level for always-crisp vectors |
| **Background toggle** | Fixed background | Dark/light background toggle for transparent images |
| **Keyboard-driven** | Mouse-oriented | Nearly everything has a shortcut |
| **Hand off to default viewer** | N/A | One keypress opens the image in Preview/Photos/etc., optionally quitting RPView |

## Supported Formats

PNG, JPEG, BMP, GIF (animated), TIFF, ICO, WebP (animated), SVG

## Installation

### From source

```
git clone https://github.com/ComposerChrisF/rpview.git
cd rpview
cargo build --release
```

The binary is at `target/release/rpview`. Copy it to a directory on your PATH,
or on macOS run the bundler:

```
bash packaging/macos/bundle.sh
```

This creates `target/release/RPView.app` which you can drag to /Applications.

### Prerequisites

- Rust (latest stable) — [rustup.rs](https://rustup.rs/)
- **macOS**: Xcode Command Line Tools
- **Linux**: X11 development packages
- **Windows**: Visual Studio Build Tools

## Quick Start

```bash
# View images in the current directory
rpview

# Start at a specific image (loads its entire directory)
rpview photo.png

# View only specific files
rpview a.png b.jpg c.webp

# View all images in a directory
rpview ~/Pictures/screenshots
```

Or drag and drop files and folders onto the RPView window.

## Keyboard Shortcuts

RPView is built around the keyboard. On macOS the modifier is Cmd; on
Windows/Linux it's Ctrl. The table below writes "Cmd" — substitute as needed.

### Navigation

| Key | Action |
|-----|--------|
| `Left` / `Right` | Previous / next image |
| `Shift+Cmd+A` | Sort alphabetically |
| `Shift+Cmd+M` | Sort by modified date |
| Drag & Drop | Open dropped files or folders |

### Zoom

| Key | Action |
|-----|--------|
| `+` / `-` | Zoom in / out (1.25x steps) |
| `Shift` + `+` / `-` | Fast zoom (1.5x steps) |
| `Cmd` + `+` / `-` | Slow zoom (1.05x steps) |
| `Shift+Cmd` + `+` / `-` | Incremental zoom (1% steps) |
| `0` | Toggle fit-to-window / 100% |
| `Cmd+0` | Reset zoom and re-center |
| `Cmd` + scroll wheel | Zoom at cursor position |
| `Z` + drag | Dynamic drag-to-zoom |

### Pan

| Key | Action |
|-----|--------|
| `W` `A` `S` `D` or `I` `J` `K` `L` | Pan (10 px) |
| `Shift` + above | Fast pan (30 px) |
| `Alt` + above | Slow pan (3 px) |
| `Space` + drag | Pan with mouse (1:1 movement) |

### Image Filters

| Key | Action |
|-----|--------|
| `Cmd+F` | Toggle filter controls panel |
| `Cmd+1` | Disable all filters |
| `Cmd+2` | Enable filters |
| `Shift+Cmd+R` | Reset filters to defaults |

Brightness, contrast, and gamma are adjusted interactively from the filter
panel. Filter state is remembered per-image.

### Animation (GIF / WebP)

| Key | Action |
|-----|--------|
| `O` | Play / pause |
| `[` / `]` | Previous / next frame |

Animated images auto-play by default (configurable).

### File Operations

| Key | Action |
|-----|--------|
| `Cmd+O` | Open file(s) via dialog |
| `Cmd+S` | Save image (to current folder) |
| `Cmd+Alt+S` | Save image to Downloads |
| `Cmd+R` | Reveal in Finder / Explorer |
| `Cmd+Alt+V` | **Open in external viewer** (e.g. Preview.app) |
| `Shift+Cmd+Alt+V` | Open in external viewer **and quit RPView** |
| `Cmd+E` | Open in external editor |
| `Cmd+Delete` | **Delete file** (move to Trash) |
| `Shift+Cmd+Delete` | **Permanently delete file** |

The "open externally" shortcuts are the fast hand-off: if you need the OS
default viewer for something RPView doesn't do (like markup or printing), one
chord sends the current image there. Add `Shift` to quit RPView at the same
time — useful if you're done browsing and just want to work with one file.

### Display

| Key | Action |
|-----|--------|
| `T` | Toggle zoom/size indicator |
| `B` | Toggle dark / light background |

The background toggle is especially useful for transparent PNGs and SVGs — flip
between dark and light to check edges and transparency.

### Window

| Key | Action |
|-----|--------|
| `H` / `?` / `F1` | Help overlay (all shortcuts) |
| `F12` | Debug overlay |
| `Cmd+,` | Settings |
| `Cmd+W` | Close window |
| `Cmd+Q` | Quit |
| `Esc` x3 (within 2 sec) | Quick quit |

## Features in Detail

### Instant Navigation with GPU Preloading

RPView loads the next and previous images into GPU texture memory before you
navigate to them. When you press the arrow key, the image appears immediately
— no loading spinner, no flash of black.

### Per-Image State Memory

Zoom level, pan position, and filter adjustments are cached for each image you
visit (up to 1,000 by default). Flip forward through a batch of photos, zoom
and adjust one, then flip back — it's still right where you left it.

### Five-Speed Zoom

Normal, fast (Shift), slow (Cmd), incremental (Shift+Cmd), and mouse-wheel
zoom at cursor. Plus Z+drag for Photoshop-style dynamic zoom. Each serves a
different task: quick overview, precise pixel inspection, or smooth animated
zoom.

### SVG Re-Rendering

SVGs are rasterized for display but **re-rendered at the current zoom level**
when you zoom in. The result is always crisp, no matter how far you zoom. Large
SVGs use viewport-only rendering to stay fast.

### Real-Time Filters

Brightness, contrast, and gamma — applied live, cached per-image, processed on
background threads. Useful for inspecting dark photos, checking print contrast,
or quickly comparing exposures.

### Animation Controls

GIF and animated WebP files play automatically. Press `O` to pause, then
`[` and `]` to step frame by frame. Frames are cached to disk and preloaded
into GPU memory for smooth playback without flicker.

### Configurable External Viewer Hand-Off

`Cmd+Alt+V` opens the current image in your OS default viewer (Preview on
macOS, Photos on Windows, Eye of GNOME on Linux). `Shift+Cmd+Alt+V` does the
same and quits RPView. The external viewers are fully configurable in settings
— you can add editors, other viewers, or custom commands.

### Dark / Light Background Toggle

Press `B` to switch between dark and light backgrounds. Both colors are
configurable in Settings > Appearance. This makes it easy to inspect
transparent PNGs and SVGs against different backgrounds without leaving the
viewer.

### File Delete with Confirmation

`Cmd+Delete` brings up a confirmation card showing the filename, full path, and
a red "Delete" button — the file moves to Trash. `Shift+Cmd+Delete` shows
"Permanently Delete" for irrecoverable removal. Press `Esc` to cancel. A brief
toast notification confirms the outcome. The next image loads automatically
after deletion.

### Drag and Drop

Drop a file to open its parent directory. Drop multiple files to view just
those files. Drop a folder to browse all images in it. Visual feedback shows a
green border while dragging.

## Settings

Press `Cmd+,` to open the interactive settings window, or edit the JSON file
directly.

### Settings File Location

- **macOS**: `~/Library/Application Support/rpview/settings.json`
- **Linux**: `~/.config/rpview/settings.json`
- **Windows**: `C:\Users\<User>\AppData\Roaming\rpview\settings.json`

### What You Can Configure

**Viewer Behavior** — Default zoom mode (fit-to-window or 100%), per-image
state memory, animation auto-play, state cache size.

**Performance** — Adjacent image preloading, filter processing threads,
maximum image dimension limit.

**Keyboard & Mouse** — Pan speeds (normal, fast, slow), scroll wheel zoom
sensitivity, Z-drag sensitivity, spacebar pan acceleration.

**File Operations** — Default save directory, default save format (PNG, JPEG,
BMP, TIFF, WebP, or same-as-original), external viewer and editor commands.

**Appearance** — Dark and light background colors, overlay transparency, font
size scale, window title format (with `{filename}`, `{index}`, `{total}`
placeholders).

**Filters** — Default brightness, contrast, and gamma values.

**Navigation** — Default sort mode, wrap-around navigation, image counter in
title bar.

**External Tools** — List of external viewers (tried in order), external
editor, Finder/Explorer integration toggle.

## Building

```bash
# Debug build
cargo build

# Release build (optimized, stripped)
cargo build --release

# Run tests (213 tests)
cargo test

# macOS .app bundle (after release build)
bash packaging/macos/bundle.sh --no-build
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
