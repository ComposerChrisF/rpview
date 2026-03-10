use crate::error::{AppError, AppResult};
use crate::utils::file_scanner;
use clap::Parser;
use std::path::PathBuf;

/// rpview-gpui - A fast, keyboard-driven image viewer built with GPUI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
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
    /// Parse command-line arguments and return a list of image paths and the starting index
    pub fn parse_image_paths() -> AppResult<(Vec<PathBuf>, usize)> {
        let cli = Cli::parse();

        let paths = if cli.paths.is_empty() {
            // No arguments: default to current directory
            return Ok((Self::collect_image_paths(&[std::env::current_dir()?])?, 0));
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
                let mut all_images = file_scanner::scan_directory(parent_dir)?;

                // Even if empty, return the result - the app will display a message
                if all_images.is_empty() {
                    return Ok((vec![], 0));
                }

                // Sort alphabetically (case-insensitive)
                file_scanner::sort_alphabetically(&mut all_images);

                // Find the index of the specified file
                let start_index = all_images
                    .iter()
                    .position(|p| p == &specified_file)
                    .unwrap_or(0);

                return Ok((all_images, start_index));
            } else {
                // File has no parent (shouldn't happen, but handle gracefully)
                return Ok((vec![specified_file], 0));
            }
        }

        // Multiple files or directories: use the existing logic
        Ok((Self::collect_image_paths(&paths)?, 0))
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

        // Sort alphabetically by default (case-insensitive)
        file_scanner::sort_alphabetically(&mut image_paths);

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
