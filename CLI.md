# Command Line Interface Design

## Overview

rpview provides a simple command-line interface for opening images and directories. The application is designed to be launched from the command line with optional file or directory arguments.

## Usage

```bash
rpview [IMAGE_PATHS]...
```

## Arguments

### `[IMAGE_PATHS]...`

**Type:** Optional positional arguments (zero or more)  
**Description:** One or more image files or directories to display

**Behavior:**
- **No arguments provided**: Treated the same as specifying `.` (the current directory) on the command line
- **Single file**: Adds all images in the file's directory to the navigation list, with the current image set to the specified file
- **Multiple files**: Adds all specified image files to the navigation list, displaying the first one
- **Single directory**: Scans the directory for supported image files, adds them to the navigation list, and displays the "first" one (where "first" depends on the current sort mode: alphabetical or modified date)
- **Multiple paths**: Can mix files and directories; all will be scanned and added to the navigation list

**Examples:**

```bash
# Open current directory
rpview

# Open a specific image
rpview photo.jpg

# Open multiple images
rpview image1.png image2.jpg image3.gif

# Open images from a directory
rpview /path/to/photos/

# Mix files and directories
rpview photo.jpg /path/to/more/photos/ another.png
```

## Options

### `-h, --help`

Display help information about the command-line interface.

**Example:**
```bash
rpview --help
```

### `-V, --version`

Display the version number of rpview.

**Example:**
```bash
rpview --version
```

## Image File Discovery

When directories are provided (or when no arguments are given):

1. **Scanning:** The application recursively scans the directory for image files
2. **Filtering:** Only files with supported extensions are included
3. **Sorting:** Files are sorted alphabetically by default (can be changed in-app to sort by modification date)
4. **Display:** The first image in the sorted list is displayed initially

**Supported Extensions:** `.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.tiff`, `.tif`, `.ico`, `.webp`

## Navigation After Launch

Once the application launches:

- Use **arrow keys** (← →) to navigate between images
- Use **keyboard shortcuts** for zoom, pan, filters, and other operations
- Press **H**, **?**, or **F1** to view all available keyboard shortcuts

## Exit Codes

- **0**: Successful execution
- **Non-zero**: Error occurred (invalid file path, unsupported format, etc.)

## Error Handling

**File Not Found:**
- If a specified file doesn't exist, the application will display an error message and continue with other valid files

**No Images Found:**
- If no supported image files are found in the specified paths, the application will display a "No images found" message

**Unsupported Format:**
- If a file has an unsupported format, it will be skipped during directory scanning
- If explicitly specified on the command line, an error message will be shown

## Platform-Specific Behavior

### macOS
```bash
# Standard invocation
rpview photo.jpg

# Can be launched from Finder by dragging files onto the app icon
```

### Windows
```bash
# Standard invocation
rpview.exe photo.jpg

# Can be associated with image file types for "Open with rpview"
```

### Linux
```bash
# Standard invocation
rpview photo.jpg

# Can be set as default image viewer in desktop environment settings
```

## Integration with System

### File Associations (Planned)

Future versions will support file associations, allowing users to:
- Right-click an image and select "Open with rpview"
- Set rpview as the default image viewer
- Double-click images to open in rpview

### Desktop Files (Linux)

A `.desktop` file will be provided for integration with Linux desktop environments, enabling:
- Application menu entries
- File manager integration
- Mime type associations

## Future Enhancements

Potential future command-line options (not yet implemented):

```bash
# Specify initial zoom level
rpview --zoom 150 photo.jpg

# Start in slideshow mode
rpview --slideshow --interval 5 /path/to/photos/

# Apply filters on launch
rpview --brightness 20 --contrast 10 photo.jpg

# Specify sort order
rpview --sort date /path/to/photos/

# Start fullscreen
rpview --fullscreen photo.jpg

# Recursive directory scanning
rpview --recursive /path/to/photos/
```

## Implementation Notes

The CLI is implemented using the `clap` crate with derive macros:

```rust
#[derive(Parser, Debug, Clone)]
#[command(version, about = "A simple cross-platform image viewer", long_about = None)]
struct Cli {
    /// One or more image files or directories to display
    #[arg(required = false)]
    image_paths: Vec<PathBuf>,
}
```

**Default Behavior:**
- When no paths are provided, defaults to current directory (`.`)
- Path resolution is relative to the current working directory
- All paths are validated and expanded before the GUI launches

## Examples by Use Case

### Quick Image Viewing
```bash
# View a screenshot you just took
rpview ~/Desktop/screenshot.png

# View all images in Downloads
rpview ~/Downloads/
```

### Photography Workflow
```bash
# Review photos from a shoot
rpview /path/to/photoshoot/

# Compare specific images
rpview photo1.jpg photo2.jpg photo3.jpg
```

### Development/Design
```bash
# Review UI mockups
rpview ./designs/

# Check exported assets
rpview logo.png icon.png banner.jpg
```

### System Integration
```bash
# Open from file manager (via context menu)
rpview "$SELECTED_FILE"

# Open as default image viewer
rpview "$1"  # where $1 is passed by the system
```
