use std::path::Path;
use image::DynamicImage;
use crate::error::{AppError, AppResult};

/// Load an image from a file path
pub fn load_image(path: &Path) -> AppResult<DynamicImage> {
    // Check if file exists
    if !path.exists() {
        return Err(AppError::FileNotFound(path.to_path_buf()));
    }
    
    // Try to load the image
    image::open(path).map_err(|e| {
        AppError::ImageLoadError(
            path.to_path_buf(),
            format!("Failed to load image: {}", e),
        )
    })
}

/// Get image dimensions without fully loading the image
pub fn get_image_dimensions(path: &Path) -> AppResult<(u32, u32)> {
    let reader = image::ImageReader::open(path).map_err(|e| {
        AppError::ImageLoadError(
            path.to_path_buf(),
            format!("Failed to open image: {}", e),
        )
    })?;
    
    let reader = reader.with_guessed_format().map_err(|e| {
        AppError::ImageLoadError(
            path.to_path_buf(),
            format!("Failed to guess image format: {}", e),
        )
    })?;
    
    let dimensions = reader.into_dimensions().map_err(|e| {
        AppError::ImageLoadError(
            path.to_path_buf(),
            format!("Failed to read dimensions: {}", e),
        )
    })?;
    
    Ok(dimensions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_load_nonexistent_image() {
        let path = PathBuf::from("nonexistent.png");
        let result = load_image(&path);
        assert!(result.is_err());
    }
}
