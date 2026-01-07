use image::{AnimationDecoder, DynamicImage};
use image::codecs::gif::GifDecoder;
use image::codecs::webp::WebPDecoder;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use crate::error::AppError;

/// Represents a single animation frame with its timing
#[derive(Clone)]
pub struct AnimationFrame {
    /// The image data for this frame
    pub image: DynamicImage,
    /// Duration in milliseconds
    pub duration_ms: u32,
}

/// Animation data extracted from an animated image
#[derive(Clone)]
pub struct AnimationData {
    /// All frames in the animation
    pub frames: Vec<AnimationFrame>,
    /// Total number of frames
    pub frame_count: usize,
}

impl AnimationData {
    /// Get frame durations as a vector of milliseconds
    pub fn frame_durations(&self) -> Vec<u32> {
        self.frames.iter().map(|f| f.duration_ms).collect()
    }
}

/// Check if a file is an animated GIF
pub fn is_animated_gif(path: &Path) -> Result<bool, AppError> {
    let file = File::open(path)
        .map_err(AppError::Io)?;
    let reader = BufReader::new(file);
    
    match GifDecoder::new(reader) {
        Ok(decoder) => {
            // Try to get frames to check if it's animated
            match decoder.into_frames().collect_frames() {
                Ok(frames) => Ok(frames.len() > 1),
                Err(_) => Ok(false),
            }
        }
        Err(_) => Ok(false),
    }
}

/// Check if a file is an animated WEBP
pub fn is_animated_webp(path: &Path) -> Result<bool, AppError> {
    let file = File::open(path)
        .map_err(AppError::Io)?;
    let reader = BufReader::new(file);
    
    match WebPDecoder::new(reader) {
        Ok(decoder) => Ok(decoder.has_animation()),
        Err(_) => Ok(false),
    }
}

/// Load animation frames from a GIF file
pub fn load_gif_animation(path: &Path) -> Result<AnimationData, AppError> {
    let file = File::open(path)
        .map_err(AppError::Io)?;
    let reader = BufReader::new(file);
    
    let decoder = GifDecoder::new(reader)
        .map_err(|e| AppError::Generic(e.to_string()))?;
    
    let frames_result = decoder.into_frames().collect_frames()
        .map_err(|e| AppError::Generic(e.to_string()))?;
    
    let animation_frames: Vec<AnimationFrame> = frames_result
        .into_iter()
        .map(|frame| {
            // Get delay in milliseconds
            let delay = frame.delay();
            let (numer, denom) = delay.numer_denom_ms();
            let duration_ms = if denom == 0 { 100 } else { numer / denom }; // Default to 100ms if invalid
            
            // Convert frame to DynamicImage
            let buffer = frame.into_buffer();
            let image = DynamicImage::ImageRgba8(buffer);
            
            AnimationFrame {
                image,
                duration_ms,
            }
        })
        .collect();
    
    let frame_count = animation_frames.len();
    
    Ok(AnimationData {
        frames: animation_frames,
        frame_count,
    })
}

/// Load animation frames from a WEBP file
pub fn load_webp_animation(path: &Path) -> Result<AnimationData, AppError> {
    let file = File::open(path)
        .map_err(AppError::Io)?;
    let reader = BufReader::new(file);
    
    let decoder = WebPDecoder::new(reader)
        .map_err(|e| AppError::Generic(e.to_string()))?;
    
    if !decoder.has_animation() {
        return Err(AppError::Generic("WEBP is not animated".to_string()));
    }
    
    let frames_result = decoder.into_frames().collect_frames()
        .map_err(|e| AppError::Generic(e.to_string()))?;
    
    let animation_frames: Vec<AnimationFrame> = frames_result
        .into_iter()
        .map(|frame| {
            // Get delay in milliseconds
            let delay = frame.delay();
            let (numer, denom) = delay.numer_denom_ms();
            let duration_ms = if denom == 0 { 100 } else { numer / denom }; // Default to 100ms if invalid
            
            // Convert frame to DynamicImage
            let buffer = frame.into_buffer();
            let image = DynamicImage::ImageRgba8(buffer);
            
            AnimationFrame {
                image,
                duration_ms,
            }
        })
        .collect();
    
    let frame_count = animation_frames.len();
    
    Ok(AnimationData {
        frames: animation_frames,
        frame_count,
    })
}

/// Detect and load animation from a file
/// Returns None if the file is not animated
pub fn load_animation(path: &Path) -> Result<Option<AnimationData>, AppError> {
    // Determine format from extension
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match extension.as_deref() {
        Some("gif") => {
            if is_animated_gif(path)? {
                Ok(Some(load_gif_animation(path)?))
            } else {
                Ok(None)
            }
        }
        Some("webp") => {
            if is_animated_webp(path)? {
                Ok(Some(load_webp_animation(path)?))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    // Test constants
    const DEFAULT_FRAME_DURATION_MS: u32 = 100;
    const TEST_FRAME_COUNT: usize = 3;

    #[test]
    fn test_animation_frame_creation() {
        // Arrange
        let image = image::DynamicImage::new_rgba8(10, 10);
        let duration_ms = 50;

        // Act
        let frame = AnimationFrame {
            image,
            duration_ms,
        };

        // Assert
        assert_eq!(frame.duration_ms, 50);
        assert_eq!(frame.image.width(), 10);
        assert_eq!(frame.image.height(), 10);
    }

    #[test]
    fn test_animation_data_frame_durations() {
        // Arrange
        let frames = vec![
            AnimationFrame {
                image: image::DynamicImage::new_rgba8(10, 10),
                duration_ms: 50,
            },
            AnimationFrame {
                image: image::DynamicImage::new_rgba8(10, 10),
                duration_ms: 100,
            },
            AnimationFrame {
                image: image::DynamicImage::new_rgba8(10, 10),
                duration_ms: 75,
            },
        ];

        let animation = AnimationData {
            frames,
            frame_count: TEST_FRAME_COUNT,
        };

        // Act
        let durations = animation.frame_durations();

        // Assert
        assert_eq!(durations.len(), TEST_FRAME_COUNT);
        assert_eq!(durations[0], 50);
        assert_eq!(durations[1], 100);
        assert_eq!(durations[2], 75);
    }

    #[test]
    fn test_animation_data_empty_frames() {
        // Arrange
        let animation = AnimationData {
            frames: vec![],
            frame_count: 0,
        };

        // Act
        let durations = animation.frame_durations();

        // Assert
        assert!(durations.is_empty());
        assert_eq!(animation.frame_count, 0);
    }

    #[test]
    fn test_is_animated_gif_nonexistent_file() {
        // Arrange
        let path = PathBuf::from("/nonexistent/path/to/image.gif");

        // Act
        let result = is_animated_gif(&path);

        // Assert - should return error for nonexistent file
        assert!(result.is_err());
    }

    #[test]
    fn test_is_animated_webp_nonexistent_file() {
        // Arrange
        let path = PathBuf::from("/nonexistent/path/to/image.webp");

        // Act
        let result = is_animated_webp(&path);

        // Assert - should return error for nonexistent file
        assert!(result.is_err());
    }

    #[test]
    fn test_load_gif_animation_nonexistent_file() {
        // Arrange
        let path = PathBuf::from("/nonexistent/path/to/animation.gif");

        // Act
        let result = load_gif_animation(&path);

        // Assert - should return error
        assert!(result.is_err());
    }

    #[test]
    fn test_load_webp_animation_nonexistent_file() {
        // Arrange
        let path = PathBuf::from("/nonexistent/path/to/animation.webp");

        // Act
        let result = load_webp_animation(&path);

        // Assert - should return error
        assert!(result.is_err());
    }

    #[test]
    fn test_load_animation_unsupported_extension() {
        // Arrange
        let path = PathBuf::from("/some/path/to/image.png");

        // Act
        let result = load_animation(&path);

        // Assert - should return Ok(None) for unsupported extensions
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_load_animation_no_extension() {
        // Arrange
        let path = PathBuf::from("/some/path/to/file_without_extension");

        // Act
        let result = load_animation(&path);

        // Assert - should return Ok(None) for files without extension
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_load_animation_case_insensitive_extension() {
        // Arrange - uppercase extension
        let path_upper = PathBuf::from("/nonexistent/IMAGE.GIF");
        let path_mixed = PathBuf::from("/nonexistent/Image.Gif");

        // Act - these will fail because files don't exist, but tests extension handling
        let result_upper = load_animation(&path_upper);
        let result_mixed = load_animation(&path_mixed);

        // Assert - both should attempt to load as GIF (error due to nonexistent)
        assert!(result_upper.is_err());
        assert!(result_mixed.is_err());
    }

    #[test]
    fn test_is_animated_gif_invalid_gif_data() {
        // Arrange - create a file with invalid GIF data
        let temp_dir = TempDir::new().unwrap();
        let gif_path = temp_dir.path().join("invalid.gif");
        let mut file = File::create(&gif_path).unwrap();
        file.write_all(b"not a gif file").unwrap();

        // Act
        let result = is_animated_gif(&gif_path);

        // Assert - should return Ok(false) for invalid GIF
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_is_animated_webp_invalid_webp_data() {
        // Arrange - create a file with invalid WEBP data
        let temp_dir = TempDir::new().unwrap();
        let webp_path = temp_dir.path().join("invalid.webp");
        let mut file = File::create(&webp_path).unwrap();
        file.write_all(b"not a webp file").unwrap();

        // Act
        let result = is_animated_webp(&webp_path);

        // Assert - should return Ok(false) for invalid WEBP
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_animation_data_clone() {
        // Arrange
        let frames = vec![
            AnimationFrame {
                image: image::DynamicImage::new_rgba8(5, 5),
                duration_ms: DEFAULT_FRAME_DURATION_MS,
            },
        ];
        let animation = AnimationData {
            frames,
            frame_count: 1,
        };

        // Act
        let cloned = animation.clone();

        // Assert
        assert_eq!(cloned.frame_count, animation.frame_count);
        assert_eq!(cloned.frames.len(), animation.frames.len());
    }
}
