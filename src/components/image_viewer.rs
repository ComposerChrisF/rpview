use gpui::*;
use std::path::PathBuf;
use crate::utils::style::{Colors, Spacing, TextSize};
use crate::utils::image_loader;
use crate::components::error_display::ErrorDisplay;

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
}

impl ImageViewer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            current_image: None,
            error_message: None,
            error_path: None,
            focus_handle: cx.focus_handle(),
        }
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
            
            div()
                .flex()
                .flex_col()
                .size_full()
                .child(
                    // Main image area
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(Colors::background())
                        .child(
                            img(path.clone())
                                .object_fit(ObjectFit::Contain)
                                .max_w_full()
                                .max_h_full()
                        )
                )
                .child(
                    // Info panel at bottom
                    div()
                        .flex()
                        .flex_col()
                        .gap(Spacing::sm())
                        .p(Spacing::md())
                        .bg(rgb(0x2d2d2d))
                        .child(
                            div()
                                .text_size(TextSize::sm())
                                .text_color(Colors::text())
                                .child(format!("File: {}", path.file_name().unwrap_or_default().to_string_lossy()))
                        )
                        .child(
                            div()
                                .text_size(TextSize::sm())
                                .text_color(Colors::text())
                                .child(format!("Dimensions: {}x{}", width, height))
                        )
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
                        .child("Use arrow keys to navigate (coming in Phase 3)")
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
