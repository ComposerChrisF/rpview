# Troubleshooting

Common issues and solutions for rpview.

## Startup Issues

### "No images found"

rpview found no supported image files in the specified path.

- **Check the directory** — rpview scans non-recursively. Images in subdirectories won't be found.
- **Check file extensions** — supported formats: PNG, JPEG/JPG, BMP, GIF, TIFF/TIF, ICO, WebP, SVG. Extensions are case-insensitive.
- **Use explicit paths** — pass files directly: `rpview photo1.png photo2.jpg`

### App launches but shows a blank/error screen

- **Permission denied** — rpview needs read access to the image files and their parent directory. Check file permissions with `ls -la`.
- **Corrupt image file** — try opening the file in another viewer. rpview shows an error overlay for files it can't decode.

### Settings don't load / unexpected defaults

rpview stores settings as JSON:

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/rpview/settings.json` |
| Linux | `~/.config/rpview/settings.json` |
| Windows | `%AppData%\rpview\settings.json` |

If the file is corrupt, rpview backs it up as `settings.json.backup` and starts with defaults. To fix:

1. Check stderr output for `"Warning: Failed to parse settings file"` messages.
2. Compare `settings.json.backup` against the working defaults to find the syntax error.
3. Delete `settings.json` to reset to defaults, or fix the JSON manually.

### Window doesn't appear (macOS)

GPUI requires a valid display. If you're running over SSH or in a headless environment, the window won't appear. rpview requires a graphical session.

## Image Display Issues

### Image appears oversized / won't load

Images exceeding the maximum dimension setting (default: 17000px on either axis) are blocked to prevent memory exhaustion. When this happens, rpview shows the file path and dimensions with a "Load Anyway" button.

To change the limit permanently, edit `settings.json`:
```json
{
  "performance": {
    "max_image_dimension": 20000
  }
}
```

### Colors look wrong (BGRA channel swap)

GPUI uses BGRA byte order internally. If you see red/blue channels swapped, this is a bug — rpview handles the conversion automatically. Please report it.

### SVG renders blurry

SVGs are rasterized at 2x scale on initial load for Retina displays. When you zoom in, rpview re-rasterizes at the current zoom level for crisp rendering. If the SVG appears blurry at high zoom:

- **Very large SVGs** (>16M pixels at the target scale) use viewport-only rendering with padding. Try panning slowly to let the renderer catch up.
- **System fonts missing** — SVGs with text require the referenced fonts to be installed. rpview loads system fonts once on first SVG open. Check stderr for `"[SVG] Loaded N font faces from system"`.

### Animated GIF/WebP doesn't play

- **Auto-play disabled** — check Settings > Viewer Behavior > Auto-play animations. Or press Space to toggle playback.
- **Single-frame GIF** — some GIF files have only one frame. Check the frame counter indicator.
- **Corrupt animation data** — rpview decodes frames on load. If frame durations are missing, it defaults to 100ms per frame.

## Filter & Local Contrast Issues

### Filters have no visible effect

- **Filters disabled** — press F to toggle filter visibility. The filter panel can be open but filters can be globally disabled.
- **Values at defaults** — brightness 0, contrast 0, gamma 1.0 produce no change. Small values (< 0.001) are treated as zero.

### Local contrast processing is slow

LC processing is CPU-intensive (histogram computation over sliding windows). To speed it up:

- **Reduce resize factor** — in the LC panel, set resize factor below 1.0 (e.g., 0.5 processes at half resolution).
- **Use Preview toggle** — turn off Preview while adjusting sliders, then turn it on to see the result.
- **Cancel in-flight computation** — click the Cancel button or press Escape in the LC panel if processing is taking too long.

### LC preset won't save

- **Reserved name** — the name "(Custom)" is reserved and cannot be used for saved presets.
- **Special characters** — preset names are sanitized: only alphanumeric characters, hyphens, underscores, and spaces are preserved. Other characters become underscores.
- **Preset directory** — presets are stored as individual JSON files in `{settings_dir}/lc-presets/`. Check that this directory is writable.

## Keyboard & Navigation Issues

### Keyboard shortcuts don't work

- **Settings window is open** — when the settings panel is active, some navigation keys (arrow keys, etc.) are redirected to settings controls. Close settings first (Escape or Cmd/Ctrl+,).
- **Filter/LC panel has focus** — floating panels can capture keyboard input. Click the main viewer to refocus.
- **Wrong modifier key** — rpview uses Cmd on macOS and Ctrl on Windows/Linux. Press F1 to see the full shortcut reference for your platform.

### Sort order doesn't change

The sort mode is set at startup from your settings. To change it at runtime:

- **S** — cycle through sort modes (Alphabetical, Modified Date)
- **T key + sort** — type-grouped sorting variants are also available

If modified-date sort shows unexpected order, check that file modification times are correct on disk (`ls -lt` or equivalent).

## External Viewer / Editor Issues

### "Open in External Viewer" does nothing

rpview tries each configured viewer in order, then falls back to the platform default (`open` on macOS, `start` on Windows, `xdg-open` on Linux).

- **Check settings** — in `settings.json`, the `external_tools.external_viewers` list defines the viewer priority. Each entry needs `command` with a `{path}` placeholder.
- **Check stderr** — failed viewer launches are logged to stderr.
- **Platform default missing** — on Linux, ensure `xdg-open` is installed and configured.

### "Open in External Editor" fails

The external editor is configured separately from viewers:
```json
{
  "external_tools": {
    "external_editor": {
      "command": "/usr/local/bin/gimp {path}",
      "enabled": true
    }
  }
}
```

Ensure the command path is absolute and the `{path}` placeholder is present.

## Platform-Specific Issues

### macOS: "Open With" doesn't pass files

rpview registers for "Open With" via the macOS application bundle. If it doesn't work:

- Ensure rpview is installed as a proper `.app` bundle (not just the bare binary).
- Try `rpview /path/to/image.png` from the terminal to verify the binary works.

### Windows: Floating windows flicker

The always-on-top behavior for filter and LC panels uses Win32 `SetWindowPos` with `HWND_TOPMOST`. Some window managers or display scaling configurations can cause flicker. This is a known limitation of the platform integration.

### Windows: Menu bar looks different

macOS uses native system menus. Windows and Linux use a custom-rendered menu bar within the window, which may look different from native applications.

### Linux: Always-on-top doesn't work

The always-on-top feature for floating panels depends on the window manager. GPUI uses `WindowKind::Floating` as a hint, but not all window managers respect it. Tiling window managers in particular may ignore this.

## Performance Issues

### High memory usage

- **Large image collections** — rpview caches per-image state (zoom, pan, filters) for up to 1000 images by default. Reduce `state_cache_size` in settings.
- **Animated images** — the first 3 frames are cached as temp PNGs on load. Large animated GIFs/WebPs consume proportionally more memory.
- **LC processing** — the local contrast pipeline uses planar f32 bitmaps (4 bytes per channel per pixel). A 4000x3000 image requires ~144MB for the float map alone.

### Slow startup

- **First SVG** — the first SVG file triggers system font discovery (~50-100ms). Subsequent SVGs reuse the cached font database.
- **Large directories** — scanning a directory with thousands of files takes time. rpview scans non-recursively, so moving images to subdirectories can help.
- **Settings file** — a very large settings file (unusual) can slow JSON parsing.

## Getting Help

If your issue isn't covered here:

1. Run rpview from a terminal to see stderr diagnostic output.
2. Press F12 to open the debug overlay, which shows image path, dimensions, zoom level, cache stats, and sort mode.
3. File a bug report with the debug overlay information and any stderr output.
