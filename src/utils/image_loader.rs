#![allow(clippy::collapsible_if)]

use super::debug_eprintln;
use crate::error::{AppError, AppResult};
use crate::utils::animation::AnimationData;
use image::DynamicImage;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use resvg::usvg;

/// Result of an async image load operation
#[derive(Clone)]
pub struct LoadedImageData {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub animation_data: Option<AnimationData>,
    /// First 3 animation frames (if animated), cached for immediate display
    pub initial_frame_paths: Vec<PathBuf>,
    /// Rasterized temp PNG path (for SVG files)
    pub rasterized_path: Option<PathBuf>,
    /// Parsed SVG tree for dynamic re-rendering at different zoom levels
    pub svg_tree: Option<Arc<usvg::Tree>>,
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
pub fn load_image_async(
    path: PathBuf,
    max_dimension: Option<u32>,
    force_load: bool,
) -> LoaderHandle {
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

        // Rasterize SVGs to temp PNGs (2x for Retina) and keep parsed tree for re-rendering
        let (rasterized_path, svg_tree) = if crate::utils::file_scanner::is_svg(&path) {
            match crate::utils::svg::parse_svg(&path) {
                Ok(tree) => match crate::utils::svg::rerasterize_svg_full(&tree, 2.0) {
                    Ok(temp_path) => (Some(temp_path), Some(Arc::new(tree))),
                    Err(e) => {
                        let _ = tx.send(LoaderMessage::Error(path, e));
                        return;
                    }
                },
                Err(e) => {
                    let _ = tx.send(LoaderMessage::Error(path, e.to_string()));
                    return;
                }
            }
        } else {
            (None, None)
        };

        // Check cancellation after SVG rasterization
        if is_cancelled(&cancel_flag_clone) {
            return;
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
            let initial_cache_count = std::cmp::min(3, anim_data.frames.len());
            debug_eprintln!(
                "[ASYNC LOAD] Caching first {} frames...",
                initial_cache_count
            );

            for i in 0..initial_cache_count {
                // Check cancellation during frame caching
                if is_cancelled(&cancel_flag_clone) {
                    return;
                }

                match tempfile::Builder::new()
                    .prefix("rpview_frame_")
                    .suffix(".png")
                    .tempfile()
                {
                    Ok(temp_file) => {
                        match anim_data.frames[i].image.save(temp_file.path()) {
                            Ok(_) => {
                                match temp_file.into_temp_path().keep() {
                                    Ok(kept) => {
                                        debug_eprintln!("[ASYNC LOAD] Cached frame {}", i);
                                        initial_frame_paths.push(kept);
                                    }
                                    Err(_e) => {
                                        debug_eprintln!(
                                            "[ASYNC LOAD ERROR] Failed to persist frame {}: {}",
                                            i, _e
                                        );
                                        initial_frame_paths.push(PathBuf::new());
                                    }
                                }
                            }
                            Err(_e) => {
                                debug_eprintln!(
                                    "[ASYNC LOAD ERROR] Failed to cache frame {}: {}",
                                    i, _e
                                );
                                initial_frame_paths.push(PathBuf::new());
                            }
                        }
                    }
                    Err(_e) => {
                        debug_eprintln!(
                            "[ASYNC LOAD ERROR] Failed to create temp file for frame {}: {}",
                            i, _e
                        );
                        initial_frame_paths.push(PathBuf::new());
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
            rasterized_path,
            svg_tree,
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
        AppError::ImageLoadError(path.to_path_buf(), format!("Failed to load image: {}", e))
    })
}

/// Get image dimensions without fully loading the image
pub fn get_image_dimensions(path: &Path) -> AppResult<(u32, u32)> {
    // SVG files need special handling — image::ImageReader can't read them
    if crate::utils::file_scanner::is_svg(path) {
        return crate::utils::svg::get_svg_dimensions(path);
    }

    let reader = image::ImageReader::open(path).map_err(|e| {
        AppError::ImageLoadError(path.to_path_buf(), format!("Failed to open image: {}", e))
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

    #[test]
    fn load_nonexistent_returns_file_not_found() {
        let path = PathBuf::from("/no_such_dir/missing.png");
        let result = load_image(&path);
        assert!(matches!(result, Err(AppError::FileNotFound(_))));
    }

    #[test]
    fn load_real_png_succeeds() {
        // Create a minimal valid PNG in a temp file
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.png");
        let img = image::DynamicImage::new_rgba8(2, 2);
        img.save(&path).unwrap();

        let loaded = load_image(&path).unwrap();
        assert_eq!(loaded.width(), 2);
        assert_eq!(loaded.height(), 2);
    }

    #[test]
    fn get_dimensions_for_real_png() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("dim_test.png");
        let img = image::DynamicImage::new_rgba8(37, 53);
        img.save(&path).unwrap();

        let (w, h) = get_image_dimensions(&path).unwrap();
        assert_eq!(w, 37);
        assert_eq!(h, 53);
    }

    #[test]
    fn get_dimensions_nonexistent_returns_error() {
        let result = get_image_dimensions(Path::new("/tmp/no_such_file_12345.png"));
        assert!(result.is_err());
    }

    #[test]
    fn cancel_flag_starts_false() {
        let flag = Arc::new(Mutex::new(false));
        assert!(!is_cancelled(&flag));
    }

    #[test]
    fn cancel_flag_becomes_true() {
        let flag = Arc::new(Mutex::new(false));
        *flag.lock().unwrap() = true;
        assert!(is_cancelled(&flag));
    }

    #[test]
    fn loader_handle_cancel_sets_flag() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("cancel_test.png");
        let img = image::DynamicImage::new_rgba8(1, 1);
        img.save(&path).unwrap();

        let handle = load_image_async(path, None, false);
        handle.cancel();
        // After cancel, the flag should be set (regardless of whether
        // the thread already finished)
        assert!(is_cancelled(&handle.cancel_flag));
    }

    #[test]
    fn async_load_sends_success_for_valid_image() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("async_test.png");
        let img = image::DynamicImage::new_rgba8(10, 10);
        img.save(&path).unwrap();

        let handle = load_image_async(path, None, false);
        // Wait for result (with timeout)
        let mut result = None;
        for _ in 0..100 {
            if let Some(msg) = handle.try_recv() {
                result = Some(msg);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            matches!(result, Some(LoaderMessage::Success(_))),
            "expected Success, got {:?}",
            result.as_ref().map(|m| match m {
                LoaderMessage::Success(_) => "Success",
                LoaderMessage::Error(_, _) => "Error",
                LoaderMessage::OversizedImage(_, _, _, _) => "OversizedImage",
            })
        );
    }

    #[test]
    fn async_load_sends_oversized_when_exceeds_limit() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("big.png");
        let img = image::DynamicImage::new_rgba8(500, 500);
        img.save(&path).unwrap();

        let max_dim = 100;
        let handle = load_image_async(path, Some(max_dim), false);
        let mut result = None;
        for _ in 0..100 {
            if let Some(msg) = handle.try_recv() {
                result = Some(msg);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(matches!(result, Some(LoaderMessage::OversizedImage(_, _, _, _))));
    }

    #[test]
    fn async_load_force_ignores_size_limit() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("big_force.png");
        let img = image::DynamicImage::new_rgba8(500, 500);
        img.save(&path).unwrap();

        let max_dim = 100;
        let handle = load_image_async(path, Some(max_dim), true);
        let mut result = None;
        for _ in 0..100 {
            if let Some(msg) = handle.try_recv() {
                result = Some(msg);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            matches!(result, Some(LoaderMessage::Success(_))),
            "force_load should bypass size limit"
        );
    }

    #[test]
    fn async_load_error_for_nonexistent_file() {
        let handle = load_image_async(PathBuf::from("/no_such_file_12345.png"), None, false);
        let mut result = None;
        for _ in 0..100 {
            if let Some(msg) = handle.try_recv() {
                result = Some(msg);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(matches!(result, Some(LoaderMessage::Error(_, _))));
    }
}
