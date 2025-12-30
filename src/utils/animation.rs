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
        .map_err(|e| AppError::Io(e))?;
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
        .map_err(|e| AppError::Io(e))?;
    let reader = BufReader::new(file);
    
    match WebPDecoder::new(reader) {
        Ok(decoder) => Ok(decoder.has_animation()),
        Err(_) => Ok(false),
    }
}

/// Load animation frames from a GIF file
pub fn load_gif_animation(path: &Path) -> Result<AnimationData, AppError> {
    let file = File::open(path)
        .map_err(|e| AppError::Io(e))?;
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
        .map_err(|e| AppError::Io(e))?;
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
