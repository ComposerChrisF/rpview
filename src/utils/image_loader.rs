use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use image::DynamicImage;
use crate::error::{AppError, AppResult};
use crate::utils::animation::AnimationData;

/// Result of an async image load operation
#[derive(Clone)]
pub struct LoadedImageData {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub animation_data: Option<AnimationData>,
    /// First 3 animation frames (if animated), cached for immediate display
    pub initial_frame_paths: Vec<PathBuf>,
}

/// Message sent from the background loader thread
pub enum LoaderMessage {
    /// Image loaded successfully
    Success(LoadedImageData),
    /// Image loading failed
    Error(PathBuf, String),
    /// Image exceeds size limit (path, width, height, max_dimension)
    OversizedImage(PathBuf, u32, u32, u32),
}

/// Handle to a background image loading operation
pub struct LoaderHandle {
    receiver: Receiver<LoaderMessage>,
    cancel_flag: Arc<Mutex<bool>>,
}

impl LoaderHandle {
    /// Try to receive a result from the loader (non-blocking)
    pub fn try_recv(&self) -> Option<LoaderMessage> {
        self.receiver.try_recv().ok()
    }
    
    /// Cancel the loading operation
    pub fn cancel(&self) {
        if let Ok(mut flag) = self.cancel_flag.lock() {
            *flag = true;
        }
    }
}

/// Start loading an image in the background
/// Returns a handle that can be used to check for completion or cancel
/// 
/// If max_dimension is Some(n) and either width or height exceeds n,
/// returns OversizedImage instead of Success
pub fn load_image_async(path: PathBuf, max_dimension: Option<u32>, force_load: bool) -> LoaderHandle {
    let (tx, rx) = mpsc::channel();
    let cancel_flag = Arc::new(Mutex::new(false));
    let cancel_flag_clone = cancel_flag.clone();
    
    thread::spawn(move || {
        // Check cancellation before starting
        if is_cancelled(&cancel_flag_clone) {
            return;
        }
        
        // Load image dimensions first (fast)
        let (width, height) = match get_image_dimensions(&path) {
            Ok(dims) => dims,
            Err(e) => {
                let _ = tx.send(LoaderMessage::Error(path, e.to_string()));
                return;
            }
        };
        
        // Check cancellation after dimensions
        if is_cancelled(&cancel_flag_clone) {
            return;
        }
        
        // Check if image exceeds size limit (unless force_load is true)
        if !force_load {
            if let Some(max_dim) = max_dimension {
                if width > max_dim || height > max_dim {
                    let _ = tx.send(LoaderMessage::OversizedImage(path, width, height, max_dim));
                    return;
                }
            }
        }
        
        // Try to load animation data if it's an animated image
        let animation_data = crate::utils::animation::load_animation(&path)
            .ok()
            .flatten();
        
        // Check cancellation after animation detection
        if is_cancelled(&cancel_flag_clone) {
            return;
        }
        
        // Cache first 3 frames for animated images
        let mut initial_frame_paths = Vec::new();
        if let Some(ref anim_data) = animation_data {
            if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
                let base_name = format!("rpview_{}_{}", 
                    std::process::id(), 
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("anim"));
                
                let initial_cache_count = std::cmp::min(3, anim_data.frames.len());
                eprintln!("[ASYNC LOAD] Caching first {} frames...", initial_cache_count);
                
                for i in 0..initial_cache_count {
                    // Check cancellation during frame caching
                    if is_cancelled(&cancel_flag_clone) {
                        return;
                    }
                    
                    let temp_path = temp_dir.join(format!("{}_{}.png", base_name, i));
                    match anim_data.frames[i].image.save(&temp_path) {
                        Ok(_) => {
                            eprintln!("[ASYNC LOAD] Cached frame {}", i);
                            initial_frame_paths.push(temp_path);
                        }
                        Err(e) => {
                            eprintln!("[ASYNC LOAD ERROR] Failed to cache frame {}: {}", i, e);
                            initial_frame_paths.push(PathBuf::new());
                        }
                    }
                }
            }
        }
        
        // Check cancellation before sending result
        if is_cancelled(&cancel_flag_clone) {
            return;
        }
        
        // Send success message
        let _ = tx.send(LoaderMessage::Success(LoadedImageData {
            path,
            width,
            height,
            animation_data,
            initial_frame_paths,
        }));
    });
    
    LoaderHandle {
        receiver: rx,
        cancel_flag,
    }
}

fn is_cancelled(flag: &Arc<Mutex<bool>>) -> bool {
    flag.lock().map(|f| *f).unwrap_or(false)
}

/// Load an image from a file path (synchronous)
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
