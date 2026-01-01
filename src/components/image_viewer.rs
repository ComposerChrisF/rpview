use gpui::*;
use std::path::PathBuf;
use crate::utils::style::{Colors, Spacing, TextSize};
use crate::utils::image_loader;
use crate::utils::zoom;
use crate::utils::filters;
use crate::utils::animation::AnimationData;
use crate::components::error_display::ErrorDisplay;
use crate::components::zoom_indicator::ZoomIndicator;
use crate::components::animation_indicator::AnimationIndicator;
use crate::components::processing_indicator::ProcessingIndicator;
use crate::state::ImageState;
use crate::state::image_state::FilterSettings;

/// Loaded image data
/// 
/// # Animation Frame Caching Strategy
/// 
/// For animated images (GIF, WEBP), frames are cached progressively to balance
/// responsiveness and performance:
/// 
/// **Phase 1: Initial Load** (in `load_image()`)
/// - Cache first 3 frames immediately to disk (~100-200ms)
/// - Pre-allocate empty PathBuf slots for remaining frames
/// - Display frame 0 immediately (fast UI feedback)
/// 
/// **Phase 2: Playback** (in `App::render()`)
/// - Cache next 3 frames ahead while animation plays (look-ahead caching)
/// - Frames are ready by the time playback reaches them
/// - After first loop, all frames are cached (smooth playback)
/// 
/// **Phase 3: GPU Preloading** (in `ImageViewer::render()`)
/// - Render next frame invisibly off-screen with `opacity(0.0)`
/// - Forces GPUI to load frame into GPU memory before display
/// - Eliminates black flashing between frames
/// 
/// This 3-phase approach provides:
/// - Fast initial display (user sees image within 200ms)
/// - No black flashing (GPU preload)
/// - Smooth playback after first loop (all frames cached)
#[derive(Clone)]
pub struct LoadedImage {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    /// Cached filtered image path (if filters are applied)
    pub filtered_path: Option<PathBuf>,
    /// Filter settings used to generate the cached filtered image
    pub cached_filter_settings: Option<FilterSettings>,
    /// Animation data (if this is an animated image)
    pub animation_data: Option<AnimationData>,
    /// Cached paths for each animation frame (disk cache)
    /// Empty PathBuf means frame not yet cached (will be cached on-demand)
    pub frame_cache_paths: Vec<PathBuf>,
}

/// Component for viewing images
pub struct ImageViewer {
    /// Currently loaded image
    pub current_image: Option<LoadedImage>,
    /// Error message if image failed to load
    pub error_message: Option<String>,
    /// Path of the image that failed to load (for full path display)
    pub error_path: Option<PathBuf>,
    /// Focus handle for keyboard events
    pub focus_handle: FocusHandle,
    /// Current image state (zoom, pan, etc.)
    pub image_state: ImageState,
    /// Last known viewport size (for fit-to-window calculations)
    pub viewport_size: Option<Size<Pixels>>,
    /// Z key drag zoom state: (last_mouse_x, last_mouse_y, zoom_center_x, zoom_center_y)
    /// - last_mouse_x, last_mouse_y: Previous mouse position for calculating incremental delta
    /// - zoom_center_x, zoom_center_y: Initial click position that zoom is centered on
    /// - Sentinel value (0,0,0,0) indicates Z key held but not actively dragging
    pub z_drag_state: Option<(f32, f32, f32, f32)>,
    /// Spacebar drag pan state: (last_mouse_x, last_mouse_y)
    /// - Tracks previous mouse position for 1:1 pixel movement panning
    /// - Sentinel value (0,0) indicates spacebar held but not actively dragging
    pub spacebar_drag_state: Option<(f32, f32)>,
    /// Paths to preload into GPU (for smooth navigation)
    /// These images are rendered invisibly to prime the GPU texture cache
    pub preload_paths: Vec<PathBuf>,
    /// Active async loading operation
    pub loading_handle: Option<image_loader::LoaderHandle>,
    /// Loading state indicator
    pub is_loading: bool,
    /// Filter processing state
    pub is_processing_filters: bool,
    /// Handle for async filter processing
    pub filter_processing_handle: Option<std::sync::mpsc::Receiver<Result<PathBuf, String>>>,
}

impl ImageViewer {
    /// Set paths to preload into GPU for smooth navigation
    pub fn set_preload_paths(&mut self, paths: Vec<PathBuf>) {
        self.preload_paths = paths;
    }
    
    /// Set the image state
    pub fn set_image_state(&mut self, state: ImageState) {
        self.image_state = state;
    }
    
    /// Get the current image state
    pub fn get_image_state(&self) -> ImageState {
        self.image_state.clone()
    }
    
    /// Calculate and set fit-to-window zoom for the current image
    pub fn fit_to_window(&mut self) {
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            let viewport_width: f32 = viewport.width.into();
            let viewport_height: f32 = viewport.height.into();
            
            let fit_zoom = zoom::calculate_fit_to_window(
                img.width,
                img.height,
                viewport_width,
                viewport_height,
            );
            
            // Calculate pan to center the image in the viewing area
            let zoomed_width = img.width as f32 * fit_zoom;
            let zoomed_height = img.height as f32 * fit_zoom;
            let pan_x = (viewport_width - zoomed_width) / 2.0;
            let pan_y = (viewport_height - zoomed_height) / 2.0;
            
            self.image_state.zoom = fit_zoom;
            self.image_state.is_fit_to_window = true;
            self.image_state.pan = (pan_x, pan_y);
        }
    }
    
    /// Update viewport size and recalculate fit-to-window if needed
    pub fn update_viewport_size(&mut self, size: Size<Pixels>) {
        let size_changed = self.viewport_size
            .map(|old| {
                let width_diff: f32 = (old.width - size.width).into();
                let height_diff: f32 = (old.height - size.height).into();
                width_diff.abs() > 1.0 || height_diff.abs() > 1.0
            })
            .unwrap_or(true);
        
        if size_changed {
            self.viewport_size = Some(size);
            
            // If we're in fit-to-window mode, recalculate
            if self.image_state.is_fit_to_window {
                self.fit_to_window();
            }
        }
    }
    
    /// Zoom in, keeping the center of the image at the same screen location
    pub fn zoom_in(&mut self, step: f32) {
        let old_zoom = self.image_state.zoom;
        let new_zoom = zoom::zoom_in(old_zoom, step);
        
        // Adjust pan to keep center of image at same screen location (if we have the data)
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            self.adjust_pan_for_zoom(img.width, img.height, viewport, old_zoom, new_zoom);
        }
        
        self.image_state.zoom = new_zoom;
        self.image_state.is_fit_to_window = false;
    }
    
    /// Zoom out, keeping the center of the image at the same screen location
    pub fn zoom_out(&mut self, step: f32) {
        let old_zoom = self.image_state.zoom;
        let new_zoom = zoom::zoom_out(old_zoom, step);
        
        // Adjust pan to keep center of image at same screen location (if we have the data)
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            self.adjust_pan_for_zoom(img.width, img.height, viewport, old_zoom, new_zoom);
        }
        
        self.image_state.zoom = new_zoom;
        self.image_state.is_fit_to_window = false;
    }
    
    /// Adjust pan offset to keep the center of the image at the same screen location when zooming
    fn adjust_pan_for_zoom(&mut self, img_width: u32, img_height: u32, _viewport: Size<Pixels>, old_zoom: f32, new_zoom: f32) {
        let (pan_x, pan_y) = self.image_state.pan;
        
        // Calculate the center of the image in screen coordinates (before zoom)
        let old_img_width = img_width as f32 * old_zoom;
        let old_img_height = img_height as f32 * old_zoom;
        let old_img_center_x = pan_x + old_img_width / 2.0;
        let old_img_center_y = pan_y + old_img_height / 2.0;
        
        // Calculate the new image dimensions
        let new_img_width = img_width as f32 * new_zoom;
        let new_img_height = img_height as f32 * new_zoom;
        
        // Calculate the offset needed to keep the image center at the same position
        let new_pan_x = old_img_center_x - new_img_width / 2.0;
        let new_pan_y = old_img_center_y - new_img_height / 2.0;
        
        // Apply pan constraints
        self.image_state.pan = self.constrain_pan(new_pan_x, new_pan_y);
    }
    
    /// Toggle between fit-to-window and 100% zoom, preserving center pixel position
    pub fn reset_zoom(&mut self) {
        if let (Some(_img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            let viewport_width: f32 = viewport.width.into();
            let viewport_height: f32 = viewport.height.into();
            
            if self.image_state.is_fit_to_window {
                // Currently at fit-to-window, switch to 100%
                // Calculate the center point of the viewport in image coordinates
                let center_x = viewport_width / 2.0;
                let center_y = viewport_height / 2.0;
                
                // At fit-to-window, the image is centered and scaled
                // We need to find what image pixel is at the viewport center
                let current_zoom = self.image_state.zoom;
                let (current_pan_x, current_pan_y) = self.image_state.pan;
                
                // Image pixel at viewport center (in image coordinates)
                let image_x = (center_x - current_pan_x) / current_zoom;
                let image_y = (center_y - current_pan_y) / current_zoom;
                
                // Now set to 100% zoom and adjust pan to keep that pixel centered
                let new_zoom = 1.0;
                let new_pan_x = center_x - (image_x * new_zoom);
                let new_pan_y = center_y - (image_y * new_zoom);
                
                self.image_state.zoom = new_zoom;
                self.image_state.pan = (new_pan_x, new_pan_y);
                self.image_state.is_fit_to_window = false;
            } else {
                // Currently at custom zoom, switch to fit-to-window
                self.fit_to_window();
            }
        }
    }
    
    /// Set zoom to 100% (actual size) with image centered
    pub fn set_one_hundred_percent(&mut self) {
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            let viewport_width: f32 = viewport.width.into();
            let viewport_height: f32 = viewport.height.into();
            
            // Calculate pan to center the image at 100% zoom
            let zoomed_width = img.width as f32;
            let zoomed_height = img.height as f32;
            let pan_x = (viewport_width - zoomed_width) / 2.0;
            let pan_y = (viewport_height - zoomed_height) / 2.0;
            
            self.image_state.zoom = 1.0;
            self.image_state.pan = (pan_x, pan_y);
            self.image_state.is_fit_to_window = false;
        }
    }
    
    /// Reset both zoom and pan - set to 100% centered or fit-to-window
    pub fn reset_zoom_and_pan(&mut self) {
        if self.image_state.is_fit_to_window {
            // Already at fit-to-window (which is centered), do nothing or toggle to 100% centered
            self.set_one_hundred_percent();
        } else {
            // Set to 100% and center
            self.set_one_hundred_percent();
        }
    }
    
    /// Pan the image with constraints to prevent panning completely off-screen
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let (pan_x, pan_y) = self.image_state.pan;
        let new_pan_x = pan_x + delta_x;
        let new_pan_y = pan_y + delta_y;
        self.image_state.pan = self.constrain_pan(new_pan_x, new_pan_y);
    }
    
    /// Constrain pan to prevent the image from going completely off-screen
    /// Ensures at least a small portion of the image remains visible
    fn constrain_pan(&self, pan_x: f32, pan_y: f32) -> (f32, f32) {
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            let viewport_width: f32 = viewport.width.into();
            let viewport_height: f32 = viewport.height.into();
            
            let zoomed_width = img.width as f32 * self.image_state.zoom;
            let zoomed_height = img.height as f32 * self.image_state.zoom;
            
            // Define minimum visible portion (e.g., 50 pixels or 10% of image, whichever is smaller)
            let min_visible_x = (zoomed_width * 0.1).min(50.0);
            let min_visible_y = (zoomed_height * 0.1).min(50.0);
            
            // Calculate allowed pan range
            // Image can be panned right until only min_visible_x pixels show on the left
            let max_pan_x = viewport_width - min_visible_x;
            // Image can be panned left until only min_visible_x pixels show on the right
            let min_pan_x = -(zoomed_width - min_visible_x);
            
            // Image can be panned down until only min_visible_y pixels show on the top
            let max_pan_y = viewport_height - min_visible_y;
            // Image can be panned up until only min_visible_y pixels show on the bottom
            let min_pan_y = -(zoomed_height - min_visible_y);
            
            // Clamp pan values to allowed range
            let constrained_x = pan_x.max(min_pan_x).min(max_pan_x);
            let constrained_y = pan_y.max(min_pan_y).min(max_pan_y);
            
            (constrained_x, constrained_y)
        } else {
            // No image or viewport, return unconstrained values
            (pan_x, pan_y)
        }
    }
    
    /// Zoom toward a specific point (cursor position)
    /// cursor_x and cursor_y are in viewport coordinates (pixels from top-left of viewport)
    pub fn zoom_toward_point(&mut self, cursor_x: f32, cursor_y: f32, zoom_in: bool, step: f32) {
        if self.current_image.is_none() {
            return;
        }
        
        let old_zoom = self.image_state.zoom;
        let new_zoom = if zoom_in {
            zoom::zoom_in(old_zoom, step)
        } else {
            zoom::zoom_out(old_zoom, step)
        };
        
        // Calculate the cursor position in image coordinates (before zoom)
        let (pan_x, pan_y) = self.image_state.pan;
        let cursor_in_image_x = (cursor_x - pan_x) / old_zoom;
        let cursor_in_image_y = (cursor_y - pan_y) / old_zoom;
        
        // Calculate the new pan to keep the cursor at the same image location
        let new_pan_x = cursor_x - cursor_in_image_x * new_zoom;
        let new_pan_y = cursor_y - cursor_in_image_y * new_zoom;
        
        // Update zoom first, then constrain pan
        self.image_state.zoom = new_zoom;
        self.image_state.pan = self.constrain_pan(new_pan_x, new_pan_y);
        self.image_state.is_fit_to_window = false;
    }
    
    /// Load an image from a path (synchronous, legacy)
    /// 
    /// For animated images (GIF, WEBP), this implements a progressive loading strategy:
    /// 1. Cache first 3 frames immediately for instant display
    /// 2. Pre-allocate slots for remaining frames
    /// 3. Remaining frames are cached on-demand during playback (see `cache_frame()`)
    /// 
    /// This approach provides:
    /// - Fast initial display (~100-200ms instead of waiting for all frames)
    /// - Smooth playback after first loop (all frames cached)
    /// - No black flashing (GPU preloading in render)
    /// 
    /// Note: This method is kept for testing but not used in production.
    /// Use `load_image_async()` instead for non-blocking loading.
    #[allow(dead_code)]
    pub fn load_image(&mut self, path: PathBuf) {
        // Get dimensions to validate the image can be loaded
        match image_loader::get_image_dimensions(&path) {
            Ok((width, height)) => {
                // Try to load animation data if it's an animated image
                let animation_data = crate::utils::animation::load_animation(&path)
                    .ok()
                    .flatten();
                
                // Cache first 3 frames immediately for instant display, rest will load in background
                let mut frame_cache_paths = Vec::new();
                if let Some(ref anim_data) = animation_data {
                    if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
                        let base_name = format!("rpview_{}_{}", std::process::id(), 
                            path.file_name().and_then(|n| n.to_str()).unwrap_or("anim"));
                        
                        // Cache first 3 frames for immediate display (gives UI time to show)
                        let initial_cache_count = std::cmp::min(3, anim_data.frames.len());
                        eprintln!("[LOAD] Caching first {} frames for immediate display...", initial_cache_count);
                        for i in 0..initial_cache_count {
                            let temp_path = temp_dir.join(format!("{}_{}.png", base_name, i));
                            match anim_data.frames[i].image.save(&temp_path) {
                                Ok(_) => {
                                    eprintln!("[LOAD] Cached frame {}", i);
                                    frame_cache_paths.push(temp_path);
                                }
                                Err(e) => {
                                    eprintln!("[ERROR] Failed to cache frame {}: {}", i, e);
                                    frame_cache_paths.push(PathBuf::new());
                                }
                            }
                        }
                        
                        // Pre-allocate paths for remaining frames (will be filled on-demand)
                        for _ in initial_cache_count..anim_data.frames.len() {
                            frame_cache_paths.push(PathBuf::new());
                        }
                        eprintln!("[LOAD] Initial caching complete: {}/{} frames ready", initial_cache_count, anim_data.frames.len());
                    }
                }
                
                // Initialize animation state if we have animation data
                if let Some(ref anim_data) = animation_data {
                    use crate::state::image_state::AnimationState;
                    let mut anim_state = AnimationState::new(
                        anim_data.frame_count,
                        anim_data.frame_durations(),
                    );
                    // First few frames are cached, rest will load on-demand
                    // Check if we have at least 2 frames cached (frame 0 and frame 1)
                    let cached_count = frame_cache_paths.iter()
                        .filter(|p| !p.as_os_str().is_empty() && p.exists())
                        .count();
                    anim_state.next_frame_ready = cached_count >= 2;
                    self.image_state.animation = Some(anim_state);
                } else {
                    self.image_state.animation = None;
                }
                
                self.current_image = Some(LoadedImage {
                    path: path.clone(),
                    width,
                    height,
                    filtered_path: None,
                    cached_filter_settings: None,
                    animation_data,
                    frame_cache_paths,
                });
                self.error_message = None;
                self.error_path = None;
                
                // Fit to window on load (if viewport size is known)
                self.fit_to_window();
            }
            Err(e) => {
                self.current_image = None;
                self.error_message = Some(e.to_string());
                self.error_path = Some(path);
            }
        }
    }
    
    /// Start loading an image asynchronously in the background
    pub fn load_image_async(&mut self, path: PathBuf) {
        // Cancel any previous loading operation
        if let Some(handle) = self.loading_handle.take() {
            handle.cancel();
        }
        
        // Start new async load
        eprintln!("[ASYNC] Starting async load for: {}", path.display());
        self.loading_handle = Some(image_loader::load_image_async(path));
        self.is_loading = true;
        
        // Clear previous image and errors
        self.current_image = None;
        self.error_message = None;
        self.error_path = None;
    }
    
    /// Check if async loading has completed and process the result
    /// Returns true if an image was loaded or an error occurred
    pub fn check_async_load(&mut self) -> bool {
        if let Some(handle) = &self.loading_handle {
            if let Some(msg) = handle.try_recv() {
                // Clear the handle since loading is complete
                self.loading_handle = None;
                self.is_loading = false;
                
                match msg {
                    image_loader::LoaderMessage::Success(data) => {
                        eprintln!("[ASYNC] Load complete: {}", data.path.display());
                        
                        // Prepare frame cache paths
                        let mut frame_cache_paths = data.initial_frame_paths.clone();
                        if let Some(ref anim_data) = data.animation_data {
                            // Pre-allocate empty slots for remaining frames
                            while frame_cache_paths.len() < anim_data.frames.len() {
                                frame_cache_paths.push(PathBuf::new());
                            }
                        }
                        
                        // Initialize animation state if we have animation data
                        if let Some(ref anim_data) = data.animation_data {
                            use crate::state::image_state::AnimationState;
                            let mut anim_state = AnimationState::new(
                                anim_data.frame_count,
                                anim_data.frame_durations(),
                            );
                            let cached_count = frame_cache_paths.iter()
                                .filter(|p| !p.as_os_str().is_empty() && p.exists())
                                .count();
                            anim_state.next_frame_ready = cached_count >= 2;
                            self.image_state.animation = Some(anim_state);
                        } else {
                            self.image_state.animation = None;
                        }
                        
                        self.current_image = Some(LoadedImage {
                            path: data.path,
                            width: data.width,
                            height: data.height,
                            filtered_path: None,
                            cached_filter_settings: None,
                            animation_data: data.animation_data,
                            frame_cache_paths,
                        });
                        self.error_message = None;
                        self.error_path = None;
                        
                        // Fit to window on load
                        self.fit_to_window();
                        
                        return true;
                    }
                    image_loader::LoaderMessage::Error(path, msg) => {
                        eprintln!("[ASYNC] Load failed: {}: {}", path.display(), msg);
                        self.current_image = None;
                        self.error_message = Some(msg);
                        self.error_path = Some(path);
                        
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// Update filtered image cache if needed (async)
    pub fn update_filtered_cache(&mut self) {
        eprintln!("[ImageViewer::update_filtered_cache] Called");
        
        // Cancel any previous filter processing when starting new one
        // This allows rapid slider changes to cancel old processing
        if self.is_processing_filters {
            eprintln!("[ImageViewer::update_filtered_cache] Canceling previous processing");
            self.filter_processing_handle = None;
            self.is_processing_filters = false;
        }
        
        if let Some(ref mut loaded) = self.current_image {
            let filters = &self.image_state.filters;
            let filters_enabled = self.image_state.filters_enabled;
            
            eprintln!("[ImageViewer::update_filtered_cache] Current filters: brightness={:.1}, contrast={:.1}, gamma={:.2}, enabled={}",
                filters.brightness, filters.contrast, filters.gamma, filters_enabled);
            eprintln!("[ImageViewer::update_filtered_cache] Cached filters: {:?}",
                loaded.cached_filter_settings);
            
            // Check if we need to regenerate the filtered image
            let needs_update = if !filters_enabled {
                // Filters disabled, clear cache
                loaded.filtered_path.is_some()
            } else if filters.brightness.abs() < 0.001 && filters.contrast.abs() < 0.001 && (filters.gamma - 1.0).abs() < 0.001 {
                // No filters applied, clear cache
                loaded.filtered_path.is_some()
            } else {
                // Check if cached filters match current filters
                loaded.cached_filter_settings.as_ref() != Some(filters)
            };
            
            eprintln!("[ImageViewer::update_filtered_cache] needs_update={}", needs_update);
            
            if needs_update {
                if !filters_enabled || (filters.brightness.abs() < 0.001 && filters.contrast.abs() < 0.001 && (filters.gamma - 1.0).abs() < 0.001) {
                    // Clear filtered cache (no processing needed)
                    self.is_processing_filters = false;
                    if let Some(ref filtered_path) = loaded.filtered_path {
                        let _ = std::fs::remove_file(filtered_path);
                    }
                    loaded.filtered_path = None;
                    loaded.cached_filter_settings = None;
                } else {
                    // Start async filter processing
                    self.is_processing_filters = true;
                    
                    let image_path = loaded.path.clone();
                    let brightness = filters.brightness;
                    let contrast = filters.contrast;
                    let gamma = filters.gamma;
                    
                    let (sender, receiver) = std::sync::mpsc::channel();
                    self.filter_processing_handle = Some(receiver);
                    
                    // Spawn background thread to process filters
                    std::thread::spawn(move || {
                        eprintln!("[FILTER_THREAD] Starting filter processing");
                        
                        let result = (|| {
                            // Load image
                            let img = image_loader::load_image(&image_path)
                                .map_err(|e| format!("Failed to load image: {}", e))?;
                            
                            // Apply filters
                            let filtered = filters::apply_filters(&img, brightness, contrast, gamma);
                            
                            // Save to temp file
                            let temp_dir = std::env::temp_dir().canonicalize()
                                .map_err(|e| format!("Failed to get temp dir: {}", e))?;
                            
                            use std::time::{SystemTime, UNIX_EPOCH};
                            let timestamp = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_nanos();
                            let temp_path = temp_dir.join(format!("rpview_filtered_{}_{}.png", std::process::id(), timestamp));
                            
                            eprintln!("[FILTER_THREAD] Saving filtered image to: {:?}", temp_path);
                            filtered.save(&temp_path)
                                .map_err(|e| format!("Failed to save filtered image: {}", e))?;
                            
                            Ok(temp_path)
                        })();
                        
                        eprintln!("[FILTER_THREAD] Filter processing complete: {:?}", result.is_ok());
                        let _ = sender.send(result);
                    });
                }
            } else {
                self.is_processing_filters = false;
            }
        }
    }
    
    /// Check for completed filter processing and update the image
    pub fn check_filter_processing(&mut self) -> bool {
        if let Some(receiver) = &self.filter_processing_handle {
            if let Ok(result) = receiver.try_recv() {
                eprintln!("[ImageViewer::check_filter_processing] Received result: {:?}", result.is_ok());
                
                match result {
                    Ok(new_filtered_path) => {
                        if let Some(ref mut loaded) = self.current_image {
                            // Clean up old filtered image
                            if let Some(ref old_filtered_path) = loaded.filtered_path {
                                let _ = std::fs::remove_file(old_filtered_path);
                            }
                            
                            // Update to new filtered image
                            loaded.filtered_path = Some(new_filtered_path);
                            loaded.cached_filter_settings = Some(self.image_state.filters);
                        }
                    }
                    Err(e) => {
                        eprintln!("[ImageViewer::check_filter_processing] Filter processing failed: {}", e);
                    }
                }
                
                self.is_processing_filters = false;
                self.filter_processing_handle = None;
                return true;
            }
        }
        false
    }
    
    /// Clear the current image
    pub fn clear(&mut self) {
        self.current_image = None;
        self.error_message = None;
        self.error_path = None;
    }
    
    /// Cache a specific animation frame to disk if not already cached
    /// 
    /// This is part of the progressive loading strategy for animations.
    /// Called from the render loop to cache frames 3+ ahead of playback.
    /// 
    /// # Arguments
    /// * `frame_index` - The frame index to cache (0-based)
    /// 
    /// # Returns
    /// * `true` if the frame is now cached (either was already cached or just cached)
    /// * `false` if caching failed or this is not an animated image
    /// 
    /// # Performance
    /// Caching happens synchronously but is called during animation playback,
    /// so it happens while previous frames are being displayed (non-blocking UX).
    pub fn cache_frame(&mut self, frame_index: usize) -> bool {
        let loaded = match self.current_image.as_mut() {
            Some(l) => l,
            None => return false,
        };
        
        let anim_data = match &loaded.animation_data {
            Some(d) => d,
            None => return false,
        };
        
        // Check if frame is already cached
        if frame_index < loaded.frame_cache_paths.len() {
            let cached_path = &loaded.frame_cache_paths[frame_index];
            if !cached_path.as_os_str().is_empty() && cached_path.exists() {
                return true; // Already cached
            }
        }
        
        // Cache the frame
        if frame_index < anim_data.frames.len() {
            if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
                let base_name = match loaded.path.file_name() {
                    Some(name) => format!("rpview_{}_{}", std::process::id(), name.to_string_lossy()),
                    None => return false,
                };
                let temp_path = temp_dir.join(format!("{}_{}.png", base_name, frame_index));
                
                // Save frame to disk
                match anim_data.frames[frame_index].image.save(&temp_path) {
                    Ok(_) => {
                        eprintln!("[CACHE] Cached frame {} on-demand", frame_index);
                        // Update the cache path
                        if frame_index < loaded.frame_cache_paths.len() {
                            loaded.frame_cache_paths[frame_index] = temp_path;
                        }
                        return true;
                    }
                    Err(e) => {
                        eprintln!("[ERROR] Failed to cache frame {}: {}", frame_index, e);
                    }
                }
            }
        }
        
        false
    }
}

impl ImageViewer {
    /// Render the image viewer as an element (for inline rendering without cx.new())
    pub fn render_view<V>(&self, cx: &mut Context<V>) -> impl IntoElement {
        let content = if self.is_loading {
            // Show loading indicator
            use crate::components::loading_indicator::LoadingIndicator;
            div()
                .size_full()
                .child(cx.new(|_cx| LoadingIndicator::new("Loading image...")))
                .into_any_element()
        } else if let Some(ref error) = self.error_message {
            // Show error message with full canonical path if available
            let full_message = if let Some(ref path) = self.error_path {
                let canonical_path = path.canonicalize()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| path.display().to_string());
                format!("{}\n\nFull path: {}", error, canonical_path)
            } else {
                error.clone()
            };
            
            div()
                .size_full()
                .child(cx.new(|_cx| ErrorDisplay::new(full_message)))
                .into_any_element()
        } else if let Some(ref loaded) = self.current_image {
            self.render_image(loaded, cx)
        } else {
            div().size_full().into_any_element()
        };
        
        content
    }
    
    fn render_image<V>(&self, loaded: &LoadedImage, cx: &mut Context<V>) -> AnyElement {
        let width = loaded.width;
        let height = loaded.height;
        
        // Get the display path (handles animation frames and filters)
        let path = if let Some(ref anim_state) = self.image_state.animation {
            let frame_index = anim_state.current_frame;
            
            // Get current frame path
            if frame_index < loaded.frame_cache_paths.len() {
                let cached_path = &loaded.frame_cache_paths[frame_index];
                if !cached_path.as_os_str().is_empty() && cached_path.exists() {
                    cached_path.clone()
                } else {
                    // Frame not cached, show error
                    return div()
                        .size_full()
                        .child(cx.new(|_cx| ErrorDisplay::new("Failed to load image frame".to_string())))
                        .into_any_element();
                }
            } else {
                // Invalid frame index
                return div()
                    .size_full()
                    .child(cx.new(|_cx| ErrorDisplay::new("Invalid frame index".to_string())))
                    .into_any_element();
            }
        } else {
            // Static image - use filtered path if available, otherwise original
            loaded.filtered_path.as_ref().unwrap_or(&loaded.path).clone()
        };
        
        // Apply zoom to image dimensions
        let zoomed_width = (width as f32 * self.image_state.zoom) as u32;
        let zoomed_height = (height as f32 * self.image_state.zoom) as u32;
        
        // Get pan offset
        let (pan_x, pan_y) = self.image_state.pan;
        
        // Main image area with zoom indicator overlay
        let zoom_level = self.image_state.zoom;
        let is_fit = self.image_state.is_fit_to_window;
        
        // Create a unique ID for the image based on its path to force GPUI to reload when path changes
        let image_id = ElementId::Name(format!("image-{}", path.display()).into());
        
        let mut container = div()
            .size_full()
            .bg(Colors::background())
            .overflow_hidden()
            .relative()
            .child(
                img(path.clone())
                    .id(image_id)
                    .w(px(zoomed_width as f32))
                    .h(px(zoomed_height as f32))
                    .absolute()
                    .left(px(pan_x))
                    .top(px(pan_y))
            );
        
        // Preload next frame for animations
        if let Some(ref anim_state) = self.image_state.animation {
            let next_frame_index = (anim_state.current_frame + 1) % anim_state.frame_count;
            if next_frame_index < loaded.frame_cache_paths.len() {
                let next_frame_path = &loaded.frame_cache_paths[next_frame_index];
                if !next_frame_path.as_os_str().is_empty() && next_frame_path.exists() {
                    container = container.child(
                        img(next_frame_path.clone())
                            .w(px(zoomed_width as f32))
                            .h(px(zoomed_height as f32))
                            .absolute()
                            .left(px(-10000.0))
                            .top(px(0.0))
                            .opacity(0.0)
                    );
                }
            }
        }
        
        // Preload next/previous images in navigation list
        for preload_path in &self.preload_paths {
            if preload_path.exists() {
                let preload_id = ElementId::Name(format!("preload-{}", preload_path.display()).into());
                container = container.child(
                    img(preload_path.clone())
                        .id(preload_id)
                        .w(px(zoomed_width as f32))
                        .h(px(zoomed_height as f32))
                        .absolute()
                        .left(px(-10000.0))
                        .top(px(0.0))
                        .opacity(0.0)
                );
            }
        }
        
        container = container.child(
            cx.new(|_cx| ZoomIndicator::new(zoom_level, is_fit, Some((width, height))))
        );
        
        // Add processing indicator if filters are being processed
        if self.is_processing_filters {
            container = container.child(
                cx.new(|_cx| ProcessingIndicator::new("Processing filters..."))
            );
        }
        
        // Add animation indicator if this is an animated image
        if let Some(ref anim_state) = self.image_state.animation {
            container = container.child(
                cx.new(|_cx| AnimationIndicator::new(
                    anim_state.current_frame,
                    anim_state.frame_count,
                    anim_state.is_playing,
                ))
            );
        }
        
        container.into_any_element()
    }
}

impl Render for ImageViewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Note: viewport size is now updated in App::render() before calling render
        
        let content = if self.is_loading {
            // Show loading indicator
            use crate::components::loading_indicator::LoadingIndicator;
            div()
                .size_full()
                .child(cx.new(|_cx| LoadingIndicator::new("Loading image...")))
                .into_any_element()
        } else if let Some(ref error) = self.error_message {
            // Show error message with full canonical path if available
            let full_message = if let Some(ref path) = self.error_path {
                let canonical_path = path.canonicalize()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| path.display().to_string());
                format!("{}\n\nFull path: {}", error, canonical_path)
            } else {
                error.clone()
            };
            
            div()
                .size_full()
                .child(cx.new(|_cx| ErrorDisplay::new(full_message)))
                .into_any_element()
        } else if let Some(ref loaded) = self.current_image {
            // Render the actual image using GPUI's img() function
            let width = loaded.width;
            let height = loaded.height;
            
            // Get the display path (handles animation frames and filters)
            let path = if let Some(ref anim_state) = self.image_state.animation {
                let frame_index = anim_state.current_frame;
                
                // Get current frame path
                if frame_index < loaded.frame_cache_paths.len() {
                    let cached_path = &loaded.frame_cache_paths[frame_index];
                    if !cached_path.as_os_str().is_empty() && cached_path.exists() {
                        cached_path.clone()
                    } else {
                        // Frame not cached, show error
                        return div()
                            .size_full()
                            .child(cx.new(|_cx| ErrorDisplay::new("Failed to load image frame".to_string())))
                            .into_any_element();
                    }
                } else {
                    // Invalid frame index
                    return div()
                        .size_full()
                        .child(cx.new(|_cx| ErrorDisplay::new("Invalid frame index".to_string())))
                        .into_any_element();
                }
            } else {
                // Static image - use filtered path if available, otherwise original
                loaded.filtered_path.as_ref().unwrap_or(&loaded.path).clone()
            };
            
            // Apply zoom to image dimensions
            let zoomed_width = (width as f32 * self.image_state.zoom) as u32;
            let zoomed_height = (height as f32 * self.image_state.zoom) as u32;
            
            // Get pan offset
            let (pan_x, pan_y) = self.image_state.pan;
            
            // Main image area with zoom indicator overlay
            let zoom_level = self.image_state.zoom;
            let is_fit = self.image_state.is_fit_to_window;
            
            // Create a unique ID for the image based on its path to force GPUI to reload when path changes
            let image_id = ElementId::Name(format!("image-{}", path.display()).into());
            
            let mut container = div()
                .size_full()
                .bg(Colors::background())
                .overflow_hidden()
                .relative()
                .child(
                    img(path.clone())
                        .id(image_id)
                        .w(px(zoomed_width as f32))
                        .h(px(zoomed_height as f32))
                        .absolute()
                        .left(px(pan_x))
                        .top(px(pan_y))
                );
            
            // Preload next frame for animations to avoid GPU texture loading flash
            // This is critical: even though frames are cached to disk, GPUI needs time
            // to load them into GPU memory. By rendering the next frame invisibly,
            // we force GPUI to load it into the GPU before it's needed for display.
            if let Some(ref anim_state) = self.image_state.animation {
                let next_frame_index = (anim_state.current_frame + 1) % anim_state.frame_count;
                if next_frame_index < loaded.frame_cache_paths.len() {
                    let next_frame_path = &loaded.frame_cache_paths[next_frame_index];
                    if !next_frame_path.as_os_str().is_empty() && next_frame_path.exists() {
                        // Render next frame invisibly to preload it into GPU memory
                        // This prevents black flashing when advancing to the next frame
                        container = container.child(
                            img(next_frame_path.clone())
                                .w(px(zoomed_width as f32))
                                .h(px(zoomed_height as f32))
                                .absolute()
                                .left(px(-10000.0))  // Position off-screen
                                .top(px(0.0))
                                .opacity(0.0)  // Make invisible
                        );
                    }
                }
            }
            
            // Preload next/previous images in navigation list to avoid GPU texture loading flash
            // Uses the EXACT same technique as animation frame preloading above:
            // - Render off-screen at -10000px
            // - Use opacity(0.0) to make invisible
            // - Use full zoomed dimensions to ensure texture is loaded
            // This forces GPUI to load textures into GPU memory before navigation
            for preload_path in &self.preload_paths {
                if preload_path.exists() {
                    let preload_id = ElementId::Name(format!("preload-{}", preload_path.display()).into());
                    container = container.child(
                        img(preload_path.clone())
                            .id(preload_id)
                            .w(px(zoomed_width as f32))  // Use full size like animation preload
                            .h(px(zoomed_height as f32))
                            .absolute()
                            .left(px(-10000.0))  // Position off-screen like animation preload
                            .top(px(0.0))
                            .opacity(0.0)  // Make invisible like animation preload
                    );
                }
            }
            
            container = container.child(
                // Zoom indicator overlay
                cx.new(|_cx| ZoomIndicator::new(zoom_level, is_fit, Some((width, height))))
            );
            
            // Add processing indicator if filters are being processed
            if self.is_processing_filters {
                container = container.child(
                    cx.new(|_cx| ProcessingIndicator::new("Processing filters..."))
                );
            }
            
            // Add animation indicator if this is an animated image
            if let Some(ref anim_state) = self.image_state.animation {
                container = container.child(
                    cx.new(|_cx| AnimationIndicator::new(
                        anim_state.current_frame,
                        anim_state.frame_count,
                        anim_state.is_playing,
                    ))
                );
            }
            
            container.into_any_element()
        } else {
            // Show "no image" message
            div()
                .flex()
                .flex_col()
                .size_full()
                .justify_center()
                .items_center()
                .gap(Spacing::md())
                .child(
                    div()
                        .text_size(TextSize::xl())
                        .text_color(Colors::text())
                        .child("No image loaded")
                )
                .child(
                    div()
                        .text_size(TextSize::sm())
                        .text_color(Colors::text())
                        .child(format!("Press {} to open an image", crate::utils::style::format_shortcut("O")))
                )
                .into_any_element()
        };
        
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(Colors::background())
            .child(content)
            .into_any_element()
    }
}
