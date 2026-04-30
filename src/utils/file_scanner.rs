use crate::error::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};

/// Supported image extensions
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "bmp", "gif", "tiff", "tif", "ico", "webp", "svg",
];

/// Check if a file is an SVG
pub fn is_svg(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("svg"))
}

/// Check if a file has a supported image extension
pub fn is_supported_image(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    SUPPORTED_EXTENSIONS
        .iter()
        .any(|s| ext.eq_ignore_ascii_case(s))
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

        // Only process files (not subdirectories) — use cached DirEntry metadata
        if entry.file_type()?.is_file() && is_supported_image(&path) {
            images.push(path);
        }
    }

    Ok(images)
}

/// Sort image paths alphabetically (case-insensitive)
pub fn sort_alphabetically(paths: &mut [PathBuf]) {
    paths.sort_by_cached_key(|p| p.to_string_lossy().to_lowercase());
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
    use tempfile::TempDir;

    // -- is_supported_image ---------------------------------------------------

    #[test]
    fn supported_image_all_extensions() {
        for ext in SUPPORTED_EXTENSIONS {
            let p = PathBuf::from(format!("file.{ext}"));
            assert!(is_supported_image(&p), "expected true for .{ext}");
        }
    }

    #[test]
    fn supported_image_case_insensitive() {
        assert!(is_supported_image(Path::new("test.PNG")));
        assert!(is_supported_image(Path::new("test.JpEg")));
        assert!(is_supported_image(Path::new("test.SVG")));
    }

    #[test]
    fn unsupported_extensions_rejected() {
        for ext in &["txt", "pdf", "doc", "mp4", "html", "rs", "heic"] {
            let p = PathBuf::from(format!("file.{ext}"));
            assert!(!is_supported_image(&p), "expected false for .{ext}");
        }
    }

    #[test]
    fn no_extension_rejected() {
        assert!(!is_supported_image(Path::new("README")));
        assert!(!is_supported_image(Path::new(".")));
    }

    // -- is_svg ---------------------------------------------------------------

    #[test]
    fn is_svg_true_for_svg_files() {
        assert!(is_svg(Path::new("drawing.svg")));
        assert!(is_svg(Path::new("drawing.SVG")));
        assert!(is_svg(Path::new("drawing.Svg")));
    }

    #[test]
    fn is_svg_false_for_non_svg() {
        assert!(!is_svg(Path::new("photo.png")));
        assert!(!is_svg(Path::new("photo.jpg")));
        assert!(!is_svg(Path::new("noext")));
    }

    // -- scan_directory -------------------------------------------------------

    #[test]
    fn scan_directory_finds_images() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.png"), b"fake").unwrap();
        std::fs::write(dir.path().join("b.jpg"), b"fake").unwrap();
        std::fs::write(dir.path().join("c.txt"), b"fake").unwrap();

        let result = scan_directory(dir.path()).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|p| is_supported_image(p)));
    }

    #[test]
    fn scan_directory_empty_dir_returns_empty() {
        let dir = TempDir::new().unwrap();
        let result = scan_directory(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn scan_directory_ignores_subdirectories() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("top.png"), b"fake").unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.png"), b"fake").unwrap();

        let result = scan_directory(dir.path()).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn scan_directory_nonexistent_returns_error() {
        let result = scan_directory(Path::new("/nonexistent_dir_12345"));
        assert!(result.is_err());
    }

    // -- sort_alphabetically --------------------------------------------------

    #[test]
    fn sort_alphabetically_orders_case_insensitive() {
        let mut paths = vec![
            PathBuf::from("Zebra.png"),
            PathBuf::from("apple.jpg"),
            PathBuf::from("BANANA.gif"),
        ];
        sort_alphabetically(&mut paths);
        let names: Vec<_> = paths.iter().map(|p| p.to_str().unwrap()).collect();
        assert_eq!(names, vec!["apple.jpg", "BANANA.gif", "Zebra.png"]);
    }

    #[test]
    fn sort_alphabetically_empty_is_noop() {
        let mut paths: Vec<PathBuf> = vec![];
        sort_alphabetically(&mut paths);
        assert!(paths.is_empty());
    }

    #[test]
    fn sort_alphabetically_single_element() {
        let mut paths = vec![PathBuf::from("only.png")];
        sort_alphabetically(&mut paths);
        assert_eq!(paths.len(), 1);
    }

    // -- process_dropped_path -------------------------------------------------

    #[test]
    fn process_dropped_file_scans_parent() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.png"), b"fake").unwrap();
        std::fs::write(dir.path().join("b.jpg"), b"fake").unwrap();
        std::fs::write(dir.path().join("c.txt"), b"fake").unwrap();

        let (images, idx) = process_dropped_path(&dir.path().join("b.jpg")).unwrap();
        assert_eq!(images.len(), 2);
        // Result is sorted alphabetically, so a.png is at 0 and b.jpg is at 1
        assert_eq!(idx, 1);
    }

    #[test]
    fn process_dropped_directory_starts_at_zero() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("x.png"), b"fake").unwrap();

        let (images, idx) = process_dropped_path(dir.path()).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(idx, 0);
    }

    #[test]
    fn process_dropped_nonexistent_returns_error() {
        let result = process_dropped_path(Path::new("/no_such_file_12345.png"));
        assert!(result.is_err());
    }

    #[test]
    fn process_dropped_unsupported_format_returns_error() {
        let dir = TempDir::new().unwrap();
        let txt = dir.path().join("notes.txt");
        std::fs::write(&txt, b"hello").unwrap();

        let result = process_dropped_path(&txt);
        assert!(result.is_err());
    }

    #[test]
    fn process_dropped_empty_dir_returns_error() {
        let dir = TempDir::new().unwrap();
        let result = process_dropped_path(dir.path());
        assert!(result.is_err());
    }
}
