use gpui::prelude::FluentBuilder;
use gpui::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod cli;
mod components;
mod error;
mod state;
mod utils;

use cli::Cli;
use components::{DebugOverlay, FilterControls, HelpOverlay, ImageViewer};
use state::AppState;

actions!(app, [
    CloseWindow, 
    Quit, 
    EscapePressed, 
    NextImage, 
    PreviousImage,
    ToggleAnimationPlayPause,
    NextFrame,
    PreviousFrame, 
    SortAlphabetical, 
    SortByModified,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomInFast,
    ZoomOutFast,
    ZoomInSlow,
    ZoomOutSlow,
    ZoomInIncremental,
    ZoomOutIncremental,
    PanUp,
    PanDown,
    PanLeft,
    PanRight,
    PanUpFast,
    PanDownFast,
    PanLeftFast,
    PanRightFast,
    PanUpSlow,
    PanDownSlow,
    PanLeftSlow,
    PanRightSlow,
    ToggleHelp,
    ToggleDebug,
    ToggleFilters,
    DisableFilters,
    EnableFilters,
    ResetFilters,
    BrightnessUp,
    BrightnessDown,
    ContrastUp,
    ContrastDown,
    GammaUp,
    GammaDown,
    OpenFile,
    SaveFile,
    SaveFileToDownloads,
]);

struct App {
    app_state: AppState,
    viewer: ImageViewer,
    focus_handle: FocusHandle,
    escape_presses: Vec<Instant>,
    /// Tracks if Z key is currently held down (for Z+drag zoom mode)
    z_key_held: bool,
    /// Tracks if spacebar is currently held down (for spacebar+drag pan mode)
    spacebar_held: bool,
    /// Tracks if left mouse button is currently pressed
    /// Used with MouseMoveEvent.pressed_button for robust button state tracking
    mouse_button_down: bool,
    /// Whether help overlay is visible
    show_help: bool,
    /// Whether debug overlay is visible
    show_debug: bool,
    /// Whether filter controls overlay is visible
    show_filters: bool,
    /// Filter controls component
    filter_controls: Entity<FilterControls>,
    /// Last time animation frame was updated (for animation playback)
    last_frame_update: Instant,
}
 
impl App {
    fn handle_escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // If help, debug, or filter overlay is open, close it instead of counting toward quit
        if self.show_help {
            self.show_help = false;
            self.focus_handle.focus(window);
            cx.notify();
            return;
        }
        if self.show_debug {
            self.show_debug = false;
            self.focus_handle.focus(window);
            cx.notify();
            return;
        }
        if self.show_filters {
            self.show_filters = false;
            self.focus_handle.focus(window);
            cx.notify();
            return;
        }
        
        let now = Instant::now();
        
        // Remove presses older than 2 seconds
        self.escape_presses.retain(|&time| now.duration_since(time) < Duration::from_secs(2));
        
        // Add current press
        self.escape_presses.push(now);
        
        // Check if we have 3 presses within 2 seconds
        if self.escape_presses.len() >= 3 {
            cx.quit();
        }
    }
    
    fn handle_toggle_help(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_help = !self.show_help;
        cx.notify();
    }
    
    fn handle_toggle_debug(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_debug = !self.show_debug;
        cx.notify();
    }
    
    fn handle_toggle_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_filters = !self.show_filters;
        
        // Restore focus to the main app when hiding filters
        if !self.show_filters {
            self.focus_handle.focus(window);
        }
        
        cx.notify();
    }
    
    fn handle_disable_filters(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.image_state.filters_enabled = false;
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_enable_filters(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.image_state.filters_enabled = true;
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_reset_filters(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.image_state.filters = state::image_state::FilterSettings::default();
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        
        // Update the filter controls sliders to reflect the reset values
        self.filter_controls.update(cx, |controls, cx| {
            controls.update_from_filters(state::image_state::FilterSettings::default(), cx);
        });
        
        cx.notify();
    }
    
    fn handle_brightness_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let current = self.viewer.image_state.filters.brightness;
        self.viewer.image_state.filters.brightness = (current + 5.0).min(100.0);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_brightness_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let current = self.viewer.image_state.filters.brightness;
        self.viewer.image_state.filters.brightness = (current - 5.0).max(-100.0);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_contrast_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let current = self.viewer.image_state.filters.contrast;
        self.viewer.image_state.filters.contrast = (current + 5.0).min(100.0);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_contrast_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let current = self.viewer.image_state.filters.contrast;
        self.viewer.image_state.filters.contrast = (current - 5.0).max(-100.0);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_gamma_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let current = self.viewer.image_state.filters.gamma;
        self.viewer.image_state.filters.gamma = (current + 0.1).min(10.0);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_gamma_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let current = self.viewer.image_state.filters.gamma;
        self.viewer.image_state.filters.gamma = (current - 0.1).max(0.1);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_open_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Open native file dialog for image selection
        let mut file_dialog = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "gif", "tiff", "tif", "ico", "webp"])
            .set_title("Open Image");
        
        // Set default directory to current image's parent directory if available
        if let Some(current_path) = self.app_state.current_image() {
            if let Some(parent) = current_path.parent() {
                file_dialog = file_dialog.set_directory(parent);
            }
        }
        
        // Get selected files (supports multiple selection)
        if let Some(files) = file_dialog.pick_files() {
            if !files.is_empty() {
                // Convert to PathBuf vector
                let new_paths: Vec<PathBuf> = files.into_iter().collect();
                
                // Replace the current image list with the new selection
                self.app_state.image_paths = new_paths;
                self.app_state.current_index = 0;
                
                // Update viewer with first image from new selection
                self.update_viewer(window, cx);
                self.update_window_title(window);
                cx.notify();
            }
        }
    }
    
    fn handle_save_file(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.handle_save_file_impl(None, cx);
    }
    
    fn handle_save_file_to_downloads(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Get the Downloads directory
        let downloads_dir = dirs::download_dir();
        self.handle_save_file_impl(downloads_dir, cx);
    }
    
    fn handle_save_file_impl(&mut self, default_dir: Option<PathBuf>, cx: &mut Context<Self>) {
        // Only save if we have a current image
        if let Some(current_path) = self.app_state.current_image() {
            // Get original filename without extension
            let original_stem = current_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("image");
            
            // Get original extension (default to png if none)
            let original_ext = current_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png");
            
            // Generate suggested filename with _filtered suffix if filters are enabled
            let suggested_name = if self.viewer.image_state.filters_enabled {
                format!("{}_filtered.{}", original_stem, original_ext)
            } else {
                format!("{}.{}", original_stem, original_ext)
            };
            
            // Open save dialog
            let mut file_dialog = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .add_filter("JPEG", &["jpg", "jpeg"])
                .add_filter("BMP", &["bmp"])
                .add_filter("TIFF", &["tiff", "tif"])
                .add_filter("WEBP", &["webp"])
                .set_file_name(&suggested_name)
                .set_title("Save Image");
            
            // Set default directory based on parameter
            if let Some(dir) = default_dir {
                file_dialog = file_dialog.set_directory(dir);
            } else if let Some(parent) = current_path.parent() {
                // Default to current image's parent directory
                file_dialog = file_dialog.set_directory(parent);
            }
            
            if let Some(save_path) = file_dialog.save_file() {
                // Determine what to save based on filter state
                let save_result = if self.viewer.image_state.filters_enabled {
                    // Save the filtered image if filters are enabled and cached
                    if let Some(loaded_image) = &self.viewer.current_image {
                        if let Some(ref filtered_path) = loaded_image.filtered_path {
                            // Copy the cached filtered image to the save location
                            std::fs::copy(filtered_path, &save_path)
                                .map(|_| ())
                                .map_err(|e| format!("Failed to copy filtered image: {}", e))
                        } else {
                            // Filters enabled but no cache - load original and apply filters
                            if let Ok(original_img) = image::open(&loaded_image.path) {
                                let filters = &self.viewer.image_state.filters;
                                let filtered_img = utils::filters::apply_filters(
                                    &original_img,
                                    filters.brightness,
                                    filters.contrast,
                                    filters.gamma,
                                );
                                self.save_dynamic_image_to_path(&filtered_img, &save_path)
                            } else {
                                Err("Failed to load original image".to_string())
                            }
                        }
                    } else {
                        Err("No image loaded".to_string())
                    }
                } else {
                    // Save original image without filters
                    if let Some(loaded_image) = &self.viewer.current_image {
                        std::fs::copy(&loaded_image.path, &save_path)
                            .map(|_| ())
                            .map_err(|e| format!("Failed to copy image: {}", e))
                    } else {
                        Err("No image loaded".to_string())
                    }
                };
                
                // Handle save result
                match save_result {
                    Ok(()) => {
                        println!("Image saved to: {}", save_path.display());
                    }
                    Err(e) => {
                        eprintln!("Failed to save image: {}", e);
                    }
                }
            }
        }
        
        cx.notify();
    }
    
    fn save_dynamic_image_to_path(&self, image_data: &image::DynamicImage, save_path: &PathBuf) -> Result<(), String> {
        // Determine output format from file extension
        let extension = save_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_lowercase();
        
        let save_result = match extension.as_str() {
            "png" => image_data.save_with_format(save_path, image::ImageFormat::Png),
            "jpg" | "jpeg" => {
                // Convert to RGB for JPEG (no alpha channel)
                let rgb_image = image_data.to_rgb8();
                rgb_image.save_with_format(save_path, image::ImageFormat::Jpeg)
            }
            "bmp" => image_data.save_with_format(save_path, image::ImageFormat::Bmp),
            "tiff" | "tif" => image_data.save_with_format(save_path, image::ImageFormat::Tiff),
            "webp" => image_data.save_with_format(save_path, image::ImageFormat::WebP),
            _ => {
                // Default to PNG for unknown extensions
                image_data.save_with_format(save_path, image::ImageFormat::Png)
            }
        };
        
        save_result.map_err(|e| format!("Failed to save image: {}", e))
    }
    
    fn handle_next_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.app_state.next_image();
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }
    
    fn handle_previous_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.app_state.previous_image();
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }
    
    fn handle_toggle_animation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            anim_state.is_playing = !anim_state.is_playing;
            if anim_state.is_playing {
                // Reset timer when starting playback
                self.last_frame_update = Instant::now();
                self.start_animation_timer(window, cx);
            }
            cx.notify();
        }
    }
    
    fn handle_update_animation(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // This is called by the background timer to update animation frames
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            if anim_state.is_playing && anim_state.frame_count > 0 {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_frame_update).as_millis() as u32;
                
                // Get current frame duration
                let frame_duration = anim_state.frame_durations
                    .get(anim_state.current_frame)
                    .copied()
                    .unwrap_or(100);
                
                // Check if it's time to advance to next frame
                if elapsed >= frame_duration {
                    anim_state.current_frame = (anim_state.current_frame + 1) % anim_state.frame_count;
                    self.last_frame_update = now;
                    cx.notify();
                }
            }
        }
    }
    
    fn update_animation_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Update animation frame if playing
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            if anim_state.is_playing && anim_state.frame_count > 0 {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_frame_update).as_millis() as u32;
                
                // Get current frame duration
                let frame_duration = anim_state.frame_durations
                    .get(anim_state.current_frame)
                    .copied()
                    .unwrap_or(100);
                
                // Check if it's time to advance to next frame
                if elapsed >= frame_duration {
                    anim_state.current_frame = (anim_state.current_frame + 1) % anim_state.frame_count;
                    self.last_frame_update = now;
                }
                
                // Schedule next frame update
                cx.on_next_frame(window, |this: &mut App, window, cx| {
                    this.update_animation_frame(window, cx);
                });
                
                cx.notify();
            }
        }
    }
    
    fn start_animation_timer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Simply trigger the first frame update, which will schedule subsequent ones
        self.update_animation_frame(window, cx);
    }
    
    fn handle_next_frame(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            // Pause animation when manually navigating frames
            anim_state.is_playing = false;
            anim_state.current_frame = (anim_state.current_frame + 1) % anim_state.frame_count;
            cx.notify();
        }
    }
    
    fn handle_previous_frame(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            // Pause animation when manually navigating frames
            anim_state.is_playing = false;
            if anim_state.current_frame == 0 {
                anim_state.current_frame = anim_state.frame_count - 1;
            } else {
                anim_state.current_frame -= 1;
            }
            cx.notify();
        }
    }
    
    fn handle_sort_alphabetical(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.app_state.set_sort_mode(state::SortMode::Alphabetical);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }
    
    fn handle_sort_by_modified(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.app_state.set_sort_mode(state::SortMode::ModifiedDate);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }
    
    fn handle_zoom_in(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_in(utils::zoom::ZOOM_STEP);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_out(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_out(utils::zoom::ZOOM_STEP);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_reset(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.reset_zoom();
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_in_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_in(utils::zoom::ZOOM_STEP_FAST);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_out_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_out(utils::zoom::ZOOM_STEP_FAST);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_in_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_in(utils::zoom::ZOOM_STEP_SLOW);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_out_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_out(utils::zoom::ZOOM_STEP_SLOW);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_in_incremental(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Incremental zoom: add 1% (0.01) to current zoom
        let current_zoom = self.viewer.image_state.zoom;
        let new_zoom = utils::zoom::clamp_zoom(current_zoom + utils::zoom::ZOOM_STEP_INCREMENTAL);
        self.viewer.image_state.zoom = new_zoom;
        self.viewer.image_state.is_fit_to_window = false;
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_out_incremental(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Incremental zoom: subtract 1% (0.01) from current zoom
        let current_zoom = self.viewer.image_state.zoom;
        let new_zoom = utils::zoom::clamp_zoom(current_zoom - utils::zoom::ZOOM_STEP_INCREMENTAL);
        self.viewer.image_state.zoom = new_zoom;
        self.viewer.image_state.is_fit_to_window = false;
        self.save_current_image_state();
        cx.notify();
    }
    

    fn handle_pan_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, -10.0);  // Pan up = move image down (negative Y)
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, 10.0);  // Pan down = move image up (positive Y)
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_left(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(-10.0, 0.0);  // Pan left = move image right (negative X)
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_right(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(10.0, 0.0);  // Pan right = move image left (positive X)
        self.save_current_image_state();
        cx.notify();
    }
    
    // Fast pan (3x speed = 30px) with Shift modifier
    fn handle_pan_up_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, -30.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_down_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, 30.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_left_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(-30.0, 0.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_right_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(30.0, 0.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    // Slow pan (3px) with Ctrl/Cmd modifier (0.3x speed)
    fn handle_pan_up_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, -3.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_down_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, 3.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_left_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(-3.0, 0.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_right_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(3.0, 0.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn save_current_image_state(&mut self) {
        let state = self.viewer.get_image_state();
        self.app_state.save_current_state(state);
    }
    
    fn load_current_image_state(&mut self) {
        let state = self.app_state.get_current_state();
        self.viewer.set_image_state(state);
    }
    
    fn update_viewer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.app_state.current_image().cloned() {
            // Load the image first (this will call fit_to_window)
            self.viewer.load_image(path.clone());
            
            // Only load cached state if we have previously saved state for this image
            if self.app_state.image_states.contains_key(&path) {
                self.load_current_image_state();
            }
            // Otherwise, keep the fit-to-window state that load_image set
            
            // Start animation if this is an animated image and it's set to play
            if let Some(ref anim_state) = self.viewer.image_state.animation {
                if anim_state.is_playing {
                    self.last_frame_update = Instant::now();
                    self.start_animation_timer(window, cx);
                }
            }
        } else {
            self.viewer.clear();
        }
    }
    
    fn update_window_title(&mut self, window: &mut Window) {
        if let Some(path) = self.app_state.current_image() {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");
            
            let position = self.app_state.current_index + 1;
            let total = self.app_state.image_paths.len();
            
            let title = format!("{} ({}/{})", filename, position, total);
            window.set_window_title(&title);
        } else {
            window.set_window_title("rpview-gpui");
        }
    }
    

}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Update viewer's viewport size from window's drawable content area
        let viewport_size = window.viewport_size();
        self.viewer.update_viewport_size(viewport_size);
        
        // Poll filter controls for changes
        if self.show_filters {
            eprintln!("[App::render] Polling filter controls...");
            let (current_filters, changed) = self.filter_controls.update(cx, |fc, cx| {
                fc.get_filters_and_detect_change(cx)
            });
            
            if changed {
                eprintln!("[App::render] Filters changed! Updating viewer with brightness={:.1}, contrast={:.1}, gamma={:.2}", 
                    current_filters.brightness, current_filters.contrast, current_filters.gamma);
                eprintln!("[App::render] Old viewer filters: brightness={:.1}, contrast={:.1}, gamma={:.2}",
                    self.viewer.image_state.filters.brightness,
                    self.viewer.image_state.filters.contrast,
                    self.viewer.image_state.filters.gamma);
                
                self.viewer.image_state.filters = current_filters;
                self.viewer.update_filtered_cache();
                self.save_current_image_state();
                
                eprintln!("[App::render] Viewer filters updated, cache regenerated");
            }
        }
        
        // Update Z-drag state based on z_key_held
        if self.z_key_held && self.viewer.z_drag_state.is_none() {
            self.viewer.z_drag_state = Some((0.0, 0.0, 0.0, 0.0));
        } else if !self.z_key_held && self.viewer.z_drag_state.is_some() {
            self.viewer.z_drag_state = None;
        }
        
        // Update spacebar-drag state based on spacebar_held
        if self.spacebar_held && self.viewer.spacebar_drag_state.is_none() {
            self.viewer.spacebar_drag_state = Some((0.0, 0.0));
        } else if !self.spacebar_held && self.viewer.spacebar_drag_state.is_some() {
            self.viewer.spacebar_drag_state = None;
        }
        
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(utils::style::Colors::background())
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                this.mouse_button_down = true;
                
                // Start spacebar-drag pan if spacebar is being held
                if this.viewer.spacebar_drag_state.is_some() {
                    let x: f32 = event.position.x.into();
                    let y: f32 = event.position.y.into();
                    // Store: (last_x, last_y) for 1:1 pixel movement
                    this.viewer.spacebar_drag_state = Some((x, y));
                    cx.notify();
                }
                // Start Z-drag zoom if Z key is being held (and spacebar is not)
                else if this.viewer.z_drag_state.is_some() {
                    let y: f32 = event.position.y.into();
                    let x: f32 = event.position.x.into();
                    // Store: (last_x, last_y, center_x, center_y) for zoom centering
                    this.viewer.z_drag_state = Some((x, y, x, y));
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _event: &MouseUpEvent, _window, cx| {
                this.mouse_button_down = false;
                
                // End spacebar-drag pan (but keep spacebar state active if still held)
                if let Some((_, _)) = this.viewer.spacebar_drag_state {
                    // Save state after panning
                    this.save_current_image_state();
                    // Reset to sentinel value to indicate spacebar is held but not dragging
                    this.viewer.spacebar_drag_state = Some((0.0, 0.0));
                    cx.notify();
                }
                // End Z-drag zoom (but keep Z key state active if still held)
                else if let Some((_, _, _, _)) = this.viewer.z_drag_state {
                    // Reset to sentinel value to indicate Z is held but not dragging
                    this.viewer.z_drag_state = Some((0.0, 0.0, 0.0, 0.0));
                    cx.notify();
                }
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                // Check if mouse button is actually pressed (safety check for button released outside window)
                let button_actually_pressed = event.pressed_button.is_some();
                
                // If we think the button is down but the event says it's not, correct our state
                if this.mouse_button_down && !button_actually_pressed {
                    this.mouse_button_down = false;
                    // End spacebar-drag pan if active
                    if this.viewer.spacebar_drag_state.is_some() {
                        this.viewer.spacebar_drag_state = Some((0.0, 0.0));
                    }
                    // End Z-drag zoom if active
                    if this.viewer.z_drag_state.is_some() {
                        this.viewer.z_drag_state = Some((0.0, 0.0, 0.0, 0.0));
                    }
                }
                
                // Handle spacebar-drag pan (only if mouse button is down and we have valid drag data)
                if this.mouse_button_down && button_actually_pressed {
                    if let Some((last_x, last_y)) = this.viewer.spacebar_drag_state {
                        // Check if this is actual drag data (not the sentinel 0,0)
                        if last_x != 0.0 || last_y != 0.0 {
                            let current_x: f32 = event.position.x.into();
                            let current_y: f32 = event.position.y.into();
                            
                            // Calculate 1:1 pixel movement delta
                            let delta_x = current_x - last_x;
                            let delta_y = current_y - last_y;
                            
                            // Apply pan directly (1:1 pixel movement)
                            this.viewer.pan(delta_x, delta_y);
                            
                            // Update last position for next delta calculation
                            this.viewer.spacebar_drag_state = Some((current_x, current_y));
                            
                            cx.notify();
                            return; // Don't process Z-drag if we're spacebar-dragging
                        }
                    }
                }
                
                // Handle Z-drag zoom (only if mouse button is down and we have valid drag data)
                if this.mouse_button_down && button_actually_pressed {
                    if let Some((last_x, last_y, center_x, center_y)) = this.viewer.z_drag_state {
                        // Check if this is actual drag data (not the sentinel 0,0,0,0)
                        if center_x != 0.0 || center_y != 0.0 {
                            let current_y: f32 = event.position.y.into();
                            let current_x: f32 = event.position.x.into();
                            
                            // Calculate INCREMENTAL delta from LAST position (not initial)
                            let delta_y = last_y - current_y; // Up is positive (zoom in)
                            let delta_x = current_x - last_x;  // Right is positive (zoom in)
                            let combined_delta = delta_y + delta_x;
                            
                            // Get the current zoom level (which changes during drag)
                            let current_zoom = this.viewer.image_state.zoom;
                            
                            // Scale zoom change proportionally to CURRENT zoom level
                            // At 100% zoom (1.0): 1% per pixel
                            // At 200% zoom (2.0): 2% per pixel (more sensitive)
                            // At 50% zoom (0.5): 0.5% per pixel (less sensitive)
                            let zoom_change = combined_delta * 0.01 * current_zoom;
                            let new_zoom = utils::zoom::clamp_zoom(current_zoom + zoom_change);
                            
                            // Apply zoom centered on initial click position
                            let old_zoom = this.viewer.image_state.zoom;
                            this.viewer.image_state.zoom = new_zoom;
                            
                            // Adjust pan to keep the click position at the same location
                            let (pan_x, pan_y) = this.viewer.image_state.pan;
                            let cursor_in_image_x = (center_x - pan_x) / old_zoom;
                            let cursor_in_image_y = (center_y - pan_y) / old_zoom;
                            let new_pan_x = center_x - cursor_in_image_x * new_zoom;
                            let new_pan_y = center_y - cursor_in_image_y * new_zoom;
                            
                            this.viewer.image_state.pan = (new_pan_x, new_pan_y);
                            this.viewer.image_state.is_fit_to_window = false;
                            
                            // Update last position for next delta calculation
                            this.viewer.z_drag_state = Some((current_x, current_y, center_x, center_y));
                            
                            cx.notify();
                        }
                    }
                }
            }))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, cx| {
                // Only handle scroll if Ctrl/Cmd is held
                // Use 'platform' field which is Cmd on macOS, Ctrl on other platforms
                if event.modifiers.platform {
                    // Get scroll delta in pixels (use window line height for conversion if needed)
                    let line_height = px(16.0); // Standard line height
                    let delta_y: f32 = event.delta.pixel_delta(line_height).y.into();
                    
                    // Positive delta_y means scrolling down (zoom out)
                    // Negative delta_y means scrolling up (zoom in)
                    let zoom_in = delta_y < 0.0;
                    
                    // Get cursor position relative to the viewport
                    let cursor_x: f32 = event.position.x.into();
                    let cursor_y: f32 = event.position.y.into();
                    
                    this.viewer.zoom_toward_point(cursor_x, cursor_y, zoom_in, utils::zoom::ZOOM_STEP_WHEEL);
                    this.save_current_image_state();
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                // Check for spacebar press (without modifiers)
                if event.keystroke.key.as_str() == "space" 
                    && !event.keystroke.modifiers.shift 
                    && !event.keystroke.modifiers.control 
                    && !event.keystroke.modifiers.platform 
                    && !event.keystroke.modifiers.alt {
                    // Enable spacebar-drag pan mode
                    this.spacebar_held = true;
                    cx.notify();
                }
                // Check for Z key press (without modifiers)
                else if event.keystroke.key.as_str() == "z" 
                    && !event.keystroke.modifiers.shift 
                    && !event.keystroke.modifiers.control 
                    && !event.keystroke.modifiers.platform 
                    && !event.keystroke.modifiers.alt {
                    // Enable Z-drag zoom mode
                    this.z_key_held = true;
                    cx.notify();
                }
            }))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _window, cx| {
                // Check for spacebar release
                if event.keystroke.key.as_str() == "space" {
                    // Disable spacebar-drag pan mode and save state
                    if this.spacebar_held {
                        this.spacebar_held = false;
                        this.save_current_image_state();
                        cx.notify();
                    }
                }
                // Check for Z key release
                else if event.keystroke.key.as_str() == "z" {
                    // Disable Z-drag zoom mode and save state
                    if this.z_key_held {
                        this.z_key_held = false;
                        this.save_current_image_state();
                        cx.notify();
                    }
                }
            }))
            .child(cx.new(|_cx| self.viewer.clone()))
            // Render overlays on top with proper z-order
            .when(self.show_help, |el| {
                el.child(cx.new(|_cx| HelpOverlay::new()))
            })
            .when(self.show_debug, |el| {
                let image_dimensions = self.viewer.current_image.as_ref().map(|img| (img.width, img.height));
                el.child(cx.new(|_cx| DebugOverlay::new(
                    self.app_state.current_image().cloned(),
                    self.app_state.current_index,
                    self.app_state.image_paths.len(),
                    self.viewer.image_state.clone(),
                    image_dimensions,
                    self.viewer.viewport_size,
                )))
            })
            .when(self.show_filters, |el| {
                el.child(self.filter_controls.clone())
            })
            .on_action(|_: &CloseWindow, window, _| {
                window.remove_window();
            })
            .on_action(cx.listener(|this, _: &EscapePressed, window, cx| {
                this.handle_escape(window, cx);
            }))
            .on_action(cx.listener(|this, _: &NextImage, window, cx| {
                this.handle_next_image(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousImage, window, cx| {
                this.handle_previous_image(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleAnimationPlayPause, window, cx| {
                this.handle_toggle_animation(window, cx);
            }))
            .on_action(cx.listener(|this, _: &NextFrame, window, cx| {
                this.handle_next_frame(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousFrame, window, cx| {
                this.handle_previous_frame(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SortAlphabetical, window, cx| {
                this.handle_sort_alphabetical(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SortByModified, window, cx| {
                this.handle_sort_by_modified(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomIn, window, cx| {
                this.handle_zoom_in(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOut, window, cx| {
                this.handle_zoom_out(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomReset, window, cx| {
                this.handle_zoom_reset(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomInFast, window, cx| {
                this.handle_zoom_in_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOutFast, window, cx| {
                this.handle_zoom_out_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomInSlow, window, cx| {
                this.handle_zoom_in_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOutSlow, window, cx| {
                this.handle_zoom_out_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomInIncremental, window, cx| {
                this.handle_zoom_in_incremental(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOutIncremental, window, cx| {
                this.handle_zoom_out_incremental(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanUp, window, cx| {
                this.handle_pan_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanDown, window, cx| {
                this.handle_pan_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanLeft, window, cx| {
                this.handle_pan_left(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanRight, window, cx| {
                this.handle_pan_right(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanUpFast, window, cx| {
                this.handle_pan_up_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanDownFast, window, cx| {
                this.handle_pan_down_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanLeftFast, window, cx| {
                this.handle_pan_left_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanRightFast, window, cx| {
                this.handle_pan_right_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanUpSlow, window, cx| {
                this.handle_pan_up_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanDownSlow, window, cx| {
                this.handle_pan_down_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanLeftSlow, window, cx| {
                this.handle_pan_left_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanRightSlow, window, cx| {
                this.handle_pan_right_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleHelp, window, cx| {
                this.handle_toggle_help(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleDebug, window, cx| {
                this.handle_toggle_debug(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleFilters, window, cx| {
                this.handle_toggle_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &DisableFilters, window, cx| {
                this.handle_disable_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &EnableFilters, window, cx| {
                this.handle_enable_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ResetFilters, window, cx| {
                this.handle_reset_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &BrightnessUp, window, cx| {
                this.handle_brightness_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &BrightnessDown, window, cx| {
                this.handle_brightness_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ContrastUp, window, cx| {
                this.handle_contrast_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ContrastDown, window, cx| {
                this.handle_contrast_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &GammaUp, window, cx| {
                this.handle_gamma_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &GammaDown, window, cx| {
                this.handle_gamma_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenFile, window, cx| {
                this.handle_open_file(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SaveFile, window, cx| {
                this.handle_save_file(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SaveFileToDownloads, window, cx| {
                this.handle_save_file_to_downloads(window, cx);
            }))
    }
}

fn setup_key_bindings(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("escape", EscapePressed, None),
        KeyBinding::new("right", NextImage, None),
        KeyBinding::new("left", PreviousImage, None),
        // Animation controls
        KeyBinding::new("o", ToggleAnimationPlayPause, None),
        KeyBinding::new("]", NextFrame, None),
        KeyBinding::new("[", PreviousFrame, None),
        KeyBinding::new("shift-cmd-a", SortAlphabetical, None),
        KeyBinding::new("shift-cmd-m", SortByModified, None),
        // Zoom controls - base (normal speed)
        KeyBinding::new("=", ZoomIn, None),  // = key (same as +)
        KeyBinding::new("+", ZoomIn, None),
        KeyBinding::new("-", ZoomOut, None),
        KeyBinding::new("0", ZoomReset, None),
        // Zoom controls - fast (with Shift)
        KeyBinding::new("shift-=", ZoomInFast, None),
        KeyBinding::new("shift-+", ZoomInFast, None),
        KeyBinding::new("shift--", ZoomOutFast, None),
        // Zoom controls - slow (with Cmd/Ctrl)
        KeyBinding::new("cmd-=", ZoomInSlow, None),
        KeyBinding::new("cmd-+", ZoomInSlow, None),
        KeyBinding::new("cmd--", ZoomOutSlow, None),
        // Zoom controls - incremental (with Shift+Cmd/Ctrl)
        KeyBinding::new("shift-cmd-=", ZoomInIncremental, None),
        KeyBinding::new("shift-cmd-+", ZoomInIncremental, None),
        KeyBinding::new("shift-cmd--", ZoomOutIncremental, None),
        // Pan controls with WASD (base speed: 10px)
        KeyBinding::new("w", PanUp, None),
        KeyBinding::new("a", PanLeft, None),
        KeyBinding::new("s", PanDown, None),
        KeyBinding::new("d", PanRight, None),
        // Pan controls with IJKL (base speed: 10px)
        KeyBinding::new("i", PanUp, None),
        KeyBinding::new("j", PanLeft, None),
        KeyBinding::new("k", PanDown, None),
        KeyBinding::new("l", PanRight, None),
        // Fast pan with Shift (3x speed: 30px)
        KeyBinding::new("shift-w", PanUpFast, None),
        KeyBinding::new("shift-a", PanLeftFast, None),
        KeyBinding::new("shift-s", PanDownFast, None),
        KeyBinding::new("shift-d", PanRightFast, None),
        KeyBinding::new("shift-i", PanUpFast, None),
        KeyBinding::new("shift-j", PanLeftFast, None),
        KeyBinding::new("shift-k", PanDownFast, None),
        KeyBinding::new("shift-l", PanRightFast, None),
        // Slow pan with Ctrl/Cmd (1px)
        KeyBinding::new("cmd-w", PanUpSlow, None),
        KeyBinding::new("cmd-a", PanLeftSlow, None),
        KeyBinding::new("cmd-s", PanDownSlow, None),
        KeyBinding::new("cmd-d", PanRightSlow, None),
        KeyBinding::new("cmd-i", PanUpSlow, None),
        KeyBinding::new("cmd-j", PanLeftSlow, None),
        KeyBinding::new("cmd-k", PanDownSlow, None),
        KeyBinding::new("cmd-l", PanRightSlow, None),
        // Help and debug overlays
        KeyBinding::new("h", ToggleHelp, None),
        KeyBinding::new("?", ToggleHelp, None),
        KeyBinding::new("f1", ToggleHelp, None),
        KeyBinding::new("f12", ToggleDebug, None),
        // Filter controls
        KeyBinding::new("cmd-f", ToggleFilters, None),
        KeyBinding::new("cmd-1", DisableFilters, None),
        KeyBinding::new("cmd-2", EnableFilters, None),
        KeyBinding::new("cmd-r", ResetFilters, None),
        // File operations
        KeyBinding::new("cmd-o", OpenFile, None),
        KeyBinding::new("cmd-s", SaveFile, None),
        KeyBinding::new("cmd-alt-s", SaveFileToDownloads, None),
    ]);
}

fn main() {
    // Parse command-line arguments to get image paths and starting index
    let (image_paths, start_index) = match Cli::parse_image_paths() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    
    // Determine the search directory for error messages
    let search_dir = if image_paths.is_empty() {
        // Try to get the directory from command-line args
        let cli_args: Vec<String> = std::env::args().collect();
        if cli_args.len() > 1 {
            let arg_path = std::path::PathBuf::from(&cli_args[1]);
            if arg_path.is_dir() {
                arg_path
            } else if let Some(parent) = arg_path.parent() {
                parent.to_path_buf()
            } else {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            }
        } else {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        }
    } else {
        std::path::PathBuf::new()
    };
    
    // Initialize application state with the starting index
    let app_state = AppState::new_with_index(image_paths.clone(), start_index);
    
    // Print startup info
    println!("rpview-gpui starting...");
    println!("Loaded {} image(s)", app_state.image_paths.len());
    if let Some(first_image) = app_state.current_image() {
        println!("Current image: {}", first_image.display());
    }
    
    // Get the first image path to load (or None if no images)
    let first_image_path = app_state.current_image().cloned();
    
    Application::new().run(move |cx: &mut gpui::App| {
        adabraka_ui::init(cx);
        
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        
        setup_key_bindings(cx);
        
        cx.on_action(|_: &Quit, cx| {
            cx.quit();
        });
        
        cx.activate(true);
        
        cx.open_window(
            WindowOptions {
                ..Default::default()
            },
            move |window, cx| {
                cx.new::<App>(|inner_cx| {
                    let focus_handle = inner_cx.focus_handle();
                    focus_handle.focus(window);
                    
                    // Create the viewer and load the first image if available
                    let mut viewer = ImageViewer {
                        current_image: None,
                        error_message: None,
                        error_path: None,
                        focus_handle: inner_cx.focus_handle(),
                        image_state: state::ImageState::new(),
                        viewport_size: None,
                        z_drag_state: None,
                        spacebar_drag_state: None,
                    };
                    
                    if let Some(ref path) = first_image_path {
                        viewer.load_image(path.clone());
                    } else {
                        // No images found - show error with canonical directory path
                        let canonical_dir = search_dir.canonicalize()
                            .unwrap_or_else(|_| search_dir.clone());
                        viewer.error_message = Some(format!("No images found in directory:\n{}", canonical_dir.display()));
                        viewer.error_path = None;
                    }
                    
                    // Set initial window title
                    if let Some(path) = app_state.current_image() {
                        let filename = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        let position = app_state.current_index + 1;
                        let total = app_state.image_paths.len();
                        let title = format!("{} ({}/{})", filename, position, total);
                        window.set_window_title(&title);
                    } else {
                        window.set_window_title("rpview-gpui");
                    }
                    
                    // Create filter controls
                    let filter_controls = inner_cx.new(|cx| {
                        FilterControls::new(viewer.image_state.filters, cx)
                    });
                    
                    App {
                        app_state,
                        viewer,
                        focus_handle,
                        escape_presses: Vec::new(),
                        z_key_held: false,
                        spacebar_held: false,
                        mouse_button_down: false,
                        show_help: false,
                        show_debug: false,
                        show_filters: false,
                        filter_controls,
                        last_frame_update: Instant::now(),
                    }
                })
        })
        .unwrap();
    });
}
