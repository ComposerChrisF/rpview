use crate::error::{AppError, AppResult};
use crate::utils::file_scanner;
use clap::Parser;
use std::path::PathBuf;

/// rpview - A fast, keyboard-driven image viewer built with GPUI
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = None,
    after_long_help = "EXIT CODES:\n  \
        0  Success\n  \
        1  Argument resolution failed (file not found, unsupported format, no images)\n  \
        2  Invalid command-line usage (clap parse error)\n\n\
CONFIGURATION:\n  \
    Settings file (auto-created on first run):\n    \
        macOS:    ~/Library/Application Support/rpview/settings.json\n    \
        Linux:    ~/.config/rpview/settings.json\n    \
        Windows:  %APPDATA%\\rpview\\settings.json\n\n  \
    Window-title template placeholders\n  \
    (set via settings.json appearance.window_title_template):\n    \
        {filename}  current image filename\n    \
        {index}     1-based position in the list\n    \
        {total}     total image count\n    \
        {sm}        sort-mode short label\n    \
        {sortmode}  sort-mode long label\n",
)]
pub struct Cli {
    /// Image files or directories to view
    ///
    /// If no arguments are provided, defaults to the current directory.
    /// Can specify:
    /// - A single file: `rpview image.png`
    /// - Multiple files: `rpview img1.png img2.jpg img3.bmp`
    /// - A directory: `rpview /path/to/images`
    /// - Mixed: `rpview img1.png /path/to/images img2.jpg`
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

impl Cli {
    /// Parse command-line arguments and return a list of image paths and an
    /// optional starting path (when the user specified a single file).
    /// The returned list is unsorted — sorting is handled by AppState.
    pub fn parse_image_paths() -> AppResult<(Vec<PathBuf>, Option<PathBuf>)> {
        let cli = Cli::parse();

        let paths = if cli.paths.is_empty() {
            // No arguments: default to current directory
            return Ok((
                Self::collect_image_paths(&[std::env::current_dir()?])?,
                None,
            ));
        } else {
            cli.paths
        };

        // Special case: single file specified
        if paths.len() == 1 && paths[0].is_file() {
            let specified_file = paths[0].clone();

            // Check if it's a supported image
            if !file_scanner::is_supported_image(&specified_file) {
                return Err(AppError::InvalidFormat(
                    specified_file,
                    "Unsupported image format".to_string(),
                ));
            }

            // Get the parent directory
            if let Some(parent_dir) = specified_file.parent() {
                // Scan the directory for all images
                let all_images = file_scanner::scan_directory(parent_dir)?;

                // Even if empty, return the result - the app will display a message
                if all_images.is_empty() {
                    return Ok((vec![], None));
                }

                return Ok((all_images, Some(specified_file)));
            } else {
                // File has no parent (shouldn't happen, but handle gracefully)
                return Ok((vec![specified_file.clone()], Some(specified_file)));
            }
        }

        // Multiple files or directories: use the existing logic
        Ok((Self::collect_image_paths(&paths)?, None))
    }

    /// Collect all image paths from the given list of files/directories
    fn collect_image_paths(paths: &[PathBuf]) -> AppResult<Vec<PathBuf>> {
        let mut image_paths = Vec::new();

        for path in paths {
            if !path.exists() {
                return Err(AppError::FileNotFound(path.clone()));
            }

            if path.is_file() {
                // Single file: check if it's a supported image format
                if file_scanner::is_supported_image(path) {
                    image_paths.push(path.clone());
                } else {
                    return Err(AppError::InvalidFormat(
                        path.clone(),
                        "Unsupported image format".to_string(),
                    ));
                }
            } else if path.is_dir() {
                // Directory: scan for all supported images
                let dir_images = file_scanner::scan_directory(path)?;
                image_paths.extend(dir_images);
            }
        }

        // Return empty list if no images found - app will display a message
        Ok(image_paths)
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::file_scanner;
    use std::path::Path;

    #[test]
    fn test_is_supported_image() {
        assert!(file_scanner::is_supported_image(Path::new("test.png")));
        assert!(file_scanner::is_supported_image(Path::new("test.PNG")));
        assert!(file_scanner::is_supported_image(Path::new("test.jpg")));
        assert!(file_scanner::is_supported_image(Path::new("test.jpeg")));
        assert!(file_scanner::is_supported_image(Path::new("test.bmp")));
        assert!(file_scanner::is_supported_image(Path::new("test.gif")));
        assert!(file_scanner::is_supported_image(Path::new("test.webp")));
        assert!(!file_scanner::is_supported_image(Path::new("test.txt")));
        assert!(!file_scanner::is_supported_image(Path::new("test.pdf")));
        assert!(!file_scanner::is_supported_image(Path::new("test")));
    }
}
