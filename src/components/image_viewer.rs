use gpui::*;
use std::path::PathBuf;
use crate::utils::style::{Colors, Spacing, TextSize};
use crate::utils::image_loader;
use crate::utils::zoom;
use crate::components::error_display::ErrorDisplay;
use crate::components::zoom_indicator::ZoomIndicator;
use crate::state::ImageState;

/// Loaded image data
#[derive(Clone)]
pub struct LoadedImage {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}

/// Component for viewing images
#[derive(Clone)]
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
}

impl ImageViewer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            current_image: None,
            error_message: None,
            error_path: None,
            focus_handle: cx.focus_handle(),
            image_state: ImageState::new(),
            viewport_size: None,
            z_drag_state: None,
            spacebar_drag_state: None,
        }
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
    
    /// Toggle between fit-to-window and 100% zoom
    pub fn reset_zoom(&mut self) {
        if self.image_state.is_fit_to_window {
            // Currently at fit-to-window, switch to 100%
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
        } else {
            // Currently at custom zoom, switch to fit-to-window
            self.fit_to_window();
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
    
    /// Load an image from a path
    pub fn load_image(&mut self, path: PathBuf) {
        // Get dimensions to validate the image can be loaded
        match image_loader::get_image_dimensions(&path) {
            Ok((width, height)) => {
                self.current_image = Some(LoadedImage {
                    path: path.clone(),
                    width,
                    height,
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
    
    /// Clear the current image
    pub fn clear(&mut self) {
        self.current_image = None;
        self.error_message = None;
        self.error_path = None;
    }
}

impl Render for ImageViewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Note: viewport size is now updated in App::render() before ImageViewer is cloned
        
        let content = if let Some(ref error) = self.error_message {
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
            let path = &loaded.path;
            
            // Apply zoom to image dimensions
            let zoomed_width = (width as f32 * self.image_state.zoom) as u32;
            let zoomed_height = (height as f32 * self.image_state.zoom) as u32;
            
            // Get pan offset
            let (pan_x, pan_y) = self.image_state.pan;
            
            // Main image area with zoom indicator overlay
            let zoom_level = self.image_state.zoom;
            let is_fit = self.image_state.is_fit_to_window;
            
            div()
                .size_full()
                .bg(Colors::background())
                .overflow_hidden()
                .relative()
                .child(
                    img(path.clone())
                        .w(px(zoomed_width as f32))
                        .h(px(zoomed_height as f32))
                        .absolute()
                        .left(px(pan_x))
                        .top(px(pan_y))
                )
                .child(
                    // Zoom indicator overlay
                    cx.new(|_cx| ZoomIndicator::new(zoom_level, is_fit))
                )
                .into_any_element()
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
    }
}
