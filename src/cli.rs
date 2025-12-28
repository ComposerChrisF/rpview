use clap::Parser;
use std::path::{Path, PathBuf};
use std::fs;
use crate::error::{AppError, AppResult};

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

/// Supported image extensions
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "bmp", "gif", "tiff", "tif", "ico", "webp",
];

impl Cli {
    /// Parse command-line arguments and return a list of image paths
    pub fn parse_image_paths() -> AppResult<Vec<PathBuf>> {
        let cli = Cli::parse();
        
        let paths = if cli.paths.is_empty() {
            // No arguments: default to current directory
            vec![std::env::current_dir()?]
        } else {
            cli.paths
        };
        
        Self::collect_image_paths(&paths)
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
                if Self::is_supported_image(path) {
                    image_paths.push(path.clone());
                } else {
                    return Err(AppError::InvalidFormat(
                        path.clone(),
                        "Unsupported image format".to_string(),
                    ));
                }
            } else if path.is_dir() {
                // Directory: scan for all supported images
                let dir_images = Self::scan_directory(path)?;
                image_paths.extend(dir_images);
            }
        }
        
        if image_paths.is_empty() {
            // Determine the best error message
            if paths.len() == 1 && paths[0].is_dir() {
                return Err(AppError::NoImagesFound(paths[0].clone()));
            } else {
                return Err(AppError::Generic("No valid images found".to_string()));
            }
        }
        
        // Sort alphabetically by default (case-insensitive)
        image_paths.sort_by(|a, b| {
            a.to_string_lossy()
                .to_lowercase()
                .cmp(&b.to_string_lossy().to_lowercase())
        });
        
        Ok(image_paths)
    }
    
    /// Scan a directory for supported image files
    fn scan_directory(dir: &Path) -> AppResult<Vec<PathBuf>> {
        let mut images = Vec::new();
        
        let entries = fs::read_dir(dir).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                AppError::PermissionDenied(dir.to_path_buf())
            } else {
                AppError::from(e)
            }
        })?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // Only process files (not subdirectories)
            if path.is_file() && Self::is_supported_image(&path) {
                images.push(path);
            }
        }
        
        Ok(images)
    }
    
    /// Check if a file has a supported image extension
    fn is_supported_image(path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            SUPPORTED_EXTENSIONS.contains(&ext.as_str())
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_supported_image() {
        assert!(Cli::is_supported_image(Path::new("test.png")));
        assert!(Cli::is_supported_image(Path::new("test.PNG")));
        assert!(Cli::is_supported_image(Path::new("test.jpg")));
        assert!(Cli::is_supported_image(Path::new("test.jpeg")));
        assert!(Cli::is_supported_image(Path::new("test.bmp")));
        assert!(Cli::is_supported_image(Path::new("test.gif")));
        assert!(Cli::is_supported_image(Path::new("test.webp")));
        assert!(!Cli::is_supported_image(Path::new("test.txt")));
        assert!(!Cli::is_supported_image(Path::new("test.pdf")));
        assert!(!Cli::is_supported_image(Path::new("test")));
    }
}
