use crate::error::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};

/// Supported image extensions
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "bmp", "gif", "tiff", "tif", "ico", "webp", "svg",
];

/// Check if a file is an SVG
pub fn is_svg(path: &Path) -> bool {
    if let Some(extension) = path.extension() {
        extension.to_string_lossy().to_lowercase() == "svg"
    } else {
        false
    }
}

/// Check if a file has a supported image extension
pub fn is_supported_image(path: &Path) -> bool {
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        SUPPORTED_EXTENSIONS.contains(&ext.as_str())
    } else {
        false
    }
}

/// Scan a directory for supported image files (non-recursive)
pub fn scan_directory(dir: &Path) -> AppResult<Vec<PathBuf>> {
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
        if path.is_file() && is_supported_image(&path) {
            images.push(path);
        }
    }

    Ok(images)
}

/// Sort image paths alphabetically (case-insensitive)
pub fn sort_alphabetically(paths: &mut [PathBuf]) {
    paths.sort_by(|a, b| {
        a.to_string_lossy()
            .to_lowercase()
            .cmp(&b.to_string_lossy().to_lowercase())
    });
}

/// Process a dropped file or directory and return a list of images with the index to display
///
/// If a file is dropped:
/// - Scan the parent directory for all images
/// - Find the index of the dropped file in the sorted list
/// - Return (all_images, index_of_dropped_file)
///
/// If a directory is dropped:
/// - Scan the directory for all images
/// - Return (all_images, 0)
pub fn process_dropped_path(path: &Path) -> AppResult<(Vec<PathBuf>, usize)> {
    if !path.exists() {
        return Err(AppError::FileNotFound(path.to_path_buf()));
    }

    if path.is_file() {
        // Verify it's a supported image
        if !is_supported_image(path) {
            return Err(AppError::InvalidFormat(
                path.to_path_buf(),
                "Unsupported image format".to_string(),
            ));
        }

        // Get the parent directory
        let parent_dir = path.parent().ok_or_else(|| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File has no parent directory",
            ))
        })?;

        // Scan the parent directory for all images
        let mut all_images = scan_directory(parent_dir)?;

        if all_images.is_empty() {
            return Err(AppError::NoImagesFound(parent_dir.to_path_buf()));
        }

        // Sort alphabetically
        sort_alphabetically(&mut all_images);

        // Find the index of the dropped file
        let start_index = all_images.iter().position(|p| p == path).unwrap_or(0);

        Ok((all_images, start_index))
    } else if path.is_dir() {
        // Scan the directory for all images
        let mut all_images = scan_directory(path)?;

        if all_images.is_empty() {
            return Err(AppError::NoImagesFound(path.to_path_buf()));
        }

        // Sort alphabetically
        sort_alphabetically(&mut all_images);

        Ok((all_images, 0))
    } else {
        Err(AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Path is neither a file nor a directory",
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_image() {
        assert!(is_supported_image(Path::new("test.png")));
        assert!(is_supported_image(Path::new("test.PNG")));
        assert!(is_supported_image(Path::new("test.jpg")));
        assert!(is_supported_image(Path::new("test.jpeg")));
        assert!(is_supported_image(Path::new("test.bmp")));
        assert!(is_supported_image(Path::new("test.gif")));
        assert!(is_supported_image(Path::new("test.webp")));
        assert!(!is_supported_image(Path::new("test.txt")));
        assert!(!is_supported_image(Path::new("test.pdf")));
        assert!(!is_supported_image(Path::new("test")));
    }
}
