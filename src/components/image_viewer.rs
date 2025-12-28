use gpui::*;
use std::path::PathBuf;
use std::sync::Arc;
use crate::utils::style::{Colors, Spacing, TextSize};
use crate::utils::image_loader;
use crate::components::error_display::ErrorDisplay;

/// Loaded image data
#[derive(Clone)]
pub struct LoadedImage {
    pub data: Arc<image::RgbaImage>,
    pub path: PathBuf,
}

/// Component for viewing images
pub struct ImageViewer {
    /// Currently loaded image
    pub current_image: Option<LoadedImage>,
    /// Error message if image failed to load
    pub error_message: Option<String>,
    /// Focus handle for keyboard events
    pub focus_handle: FocusHandle,
}

impl ImageViewer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            current_image: None,
            error_message: None,
            focus_handle: cx.focus_handle(),
        }
    }
    
    /// Load an image from a path
    pub fn load_image(&mut self, path: PathBuf, _cx: &mut Context<Self>) {
        match image_loader::load_image_rgba(&path) {
            Ok(img) => {
                self.current_image = Some(LoadedImage {
                    data: Arc::new(img),
                    path: path.clone(),
                });
                self.error_message = None;
            }
            Err(e) => {
                self.current_image = None;
                self.error_message = Some(e.to_string());
            }
        }
    }
    
    /// Clear the current image
    pub fn clear(&mut self) {
        self.current_image = None;
        self.error_message = None;
    }
}

impl Render for ImageViewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = if let Some(ref error) = self.error_message {
            // Show error message
            div()
                .size_full()
                .child(cx.new(|_cx| ErrorDisplay::new(error.clone())))
                .into_any_element()
        } else if let Some(ref loaded) = self.current_image {
            // Show image info (actual rendering will come in a future update)
            let image_data = &loaded.data;
            let width = image_data.width();
            let height = image_data.height();
            let path = &loaded.path;
            
            // For now, display image information
            // TODO: Implement actual image rendering in future phase
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
                        .text_color(Colors::info())
                        .child("✓ Image Loaded")
                )
                .child(
                    div()
                        .text_size(TextSize::md())
                        .text_color(Colors::text())
                        .child(format!("File: {}", path.file_name().unwrap_or_default().to_string_lossy()))
                )
                .child(
                    div()
                        .text_size(TextSize::md())
                        .text_color(Colors::text())
                        .child(format!("Dimensions: {}x{}", width, height))
                )
                .child(
                    div()
                        .text_size(TextSize::sm())
                        .text_color(Colors::text())
                        .child("(Image rendering coming in next update)")
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
