# RPView Icon Requirements

This document describes the icon requirements for each platform.

## macOS

**File Format:** `.icns` (Apple Icon Image format)

**Location:** `packaging/macos/rpview.icns`

**Required Sizes:**
- 16x16
- 32x32
- 64x64
- 128x128
- 256x256
- 512x512
- 1024x1024 (for Retina displays)

**Creation:**
```bash
# Convert PNG to ICNS using iconutil (macOS only)
mkdir rpview.iconset
sips -z 16 16     icon.png --out rpview.iconset/icon_16x16.png
sips -z 32 32     icon.png --out rpview.iconset/icon_16x16@2x.png
sips -z 32 32     icon.png --out rpview.iconset/icon_32x32.png
sips -z 64 64     icon.png --out rpview.iconset/icon_32x32@2x.png
sips -z 128 128   icon.png --out rpview.iconset/icon_128x128.png
sips -z 256 256   icon.png --out rpview.iconset/icon_128x128@2x.png
sips -z 256 256   icon.png --out rpview.iconset/icon_256x256.png
sips -z 512 512   icon.png --out rpview.iconset/icon_256x256@2x.png
sips -z 512 512   icon.png --out rpview.iconset/icon_512x512.png
sips -z 1024 1024 icon.png --out rpview.iconset/icon_512x512@2x.png
iconutil -c icns rpview.iconset
```

## Windows

**File Format:** `.ico` (Windows Icon format)

**Location:** `packaging/windows/rpview.ico`

**Required Sizes:**
- 16x16
- 32x32
- 48x48
- 64x64
- 128x128
- 256x256

**Creation:**
```bash
# Use ImageMagick to create ICO file
convert icon.png -define icon:auto-resize=256,128,64,48,32,16 rpview.ico

# Or use online tools like:
# - https://icoconvert.com/
# - https://convertio.co/png-ico/
```

**Embedding in Binary:**
Add to `build.rs`:
```rust
#[cfg(target_os = "windows")]
{
    use std::path::Path;
    if Path::new("packaging/windows/rpview.ico").exists() {
        let mut res = winres::WindowsResource::new();
        res.set_icon("packaging/windows/rpview.ico");
        res.compile().unwrap();
    }
}
```

Add dependency to `Cargo.toml`:
```toml
[target.'cfg(target_os = "windows")'.build-dependencies]
winres = "0.1"
```

## Linux

**File Formats:** `.png` or `.svg` (multiple sizes)

**Location:** `packaging/linux/icons/hicolor/`

**Required Sizes:**
- 16x16 → `packaging/linux/icons/hicolor/16x16/apps/rpview.png`
- 22x22 → `packaging/linux/icons/hicolor/22x22/apps/rpview.png`
- 24x24 → `packaging/linux/icons/hicolor/24x24/apps/rpview.png`
- 32x32 → `packaging/linux/icons/hicolor/32x32/apps/rpview.png`
- 48x48 → `packaging/linux/icons/hicolor/48x48/apps/rpview.png`
- 64x64 → `packaging/linux/icons/hicolor/64x64/apps/rpview.png`
- 128x128 → `packaging/linux/icons/hicolor/128x128/apps/rpview.png`
- 256x256 → `packaging/linux/icons/hicolor/256x256/apps/rpview.png`
- scalable → `packaging/linux/icons/hicolor/scalable/apps/rpview.svg` (optional, recommended)

**Installation:**
The `install.sh` script should copy icons to `~/.local/share/icons/hicolor/`

## Icon Design Guidelines

**Visual Style:**
- Simple, recognizable image/photo icon
- Use a minimalist design that works at small sizes
- Consider using a picture frame or mountain/landscape silhouette
- Primary colors: Use colors that stand out (avoid too dark for dark mode compatibility)

**Suggested Design Elements:**
- A stylized image/photo icon
- Mountain silhouette (common for image viewers)
- Picture frame outline
- Camera lens aperture shape

**Color Palette Suggestions:**
- Primary: Blue (#5C9EFF) or Green (#50FA7B)
- Accent: White or light gray for contrast
- Background: Transparent or match system theme

## Testing Icons

**macOS:**
```bash
# View .icns file
qlmanage -p packaging/macos/rpview.icns
# Or open in Preview.app
```

**Windows:**
- View `.ico` file in Windows Explorer or IrfanView

**Linux:**
```bash
# Install and test
./packaging/linux/install.sh
# Check if icon appears in application menu
```

## Future Improvements

- [ ] Create actual icon assets
- [ ] Add app icon to GPUI window (using window options)
- [ ] Support adaptive icons for Android (if mobile support added)
- [ ] Create marketing materials (screenshots, banner images)
