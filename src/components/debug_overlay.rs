use gpui::*;
use std::path::PathBuf;
use crate::utils::style::{Colors, Spacing, scaled_text_size};
use crate::state::ImageState;

/// Configuration for creating a DebugOverlay
#[derive(Clone)]
pub struct DebugOverlayConfig {
    pub current_path: Option<PathBuf>,
    pub current_index: usize,
    pub total_images: usize,
    pub image_state: ImageState,
    pub image_dimensions: Option<(u32, u32)>,
    pub viewport_size: Option<Size<Pixels>>,
    /// Overlay transparency (0-255)
    pub overlay_transparency: u8,
    /// Font size scale multiplier
    pub font_size_scale: f32,
}

/// Component for displaying debug information
#[derive(Clone)]
pub struct DebugOverlay {
    config: DebugOverlayConfig,
}

impl DebugOverlay {
    pub fn new(config: DebugOverlayConfig) -> Self {
        Self { config }
    }
    
    /// Render a debug info line
    fn render_info_line(&self, label: &str, value: String) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .gap(Spacing::sm())
            .mb(Spacing::xs())
            .child(
                div()
                    .min_w(px(140.0))
                    .text_size(scaled_text_size(12.0, self.config.font_size_scale))
                    .text_color(rgb(0x888888))
                    .child(format!("{}:", label))
            )
            .child(
                div()
                    .text_size(scaled_text_size(12.0, self.config.font_size_scale))
                    .text_color(Colors::text())
                    .font_family("monospace")
                    .child(value)
            )
    }

    /// Render a debug info line with word wrapping for long values
    fn render_info_line_wrapping(&self, label: &str, value: String) -> impl IntoElement {
        div()
            .flex()
            .flex_col() // Stack vertically instead of horizontally
            .mb(Spacing::sm())
            .child(
                div()
                    .text_size(scaled_text_size(12.0, self.config.font_size_scale))
                    .text_color(rgb(0x888888))
                    .mb(px(2.0))
                    .child(format!("{}:", label))
            )
            .child(
                div()
                    .text_size(scaled_text_size(12.0, self.config.font_size_scale))
                    .text_color(Colors::text())
                    .font_family("monospace")
                    .line_height(relative(1.4))
                    .child(value)
            )
    }
}

impl Render for DebugOverlay {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let (filename_str, folder_str) = if let Some(ref path) = self.config.current_path {
            let filename = path.file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            let folder = path.parent()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "None".to_string());
            (filename, folder)
        } else {
            ("None".to_string(), "None".to_string())
        };

        let index_str = if self.config.total_images > 0 {
            format!("{} / {}", self.config.current_index + 1, self.config.total_images)
        } else {
            "0 / 0".to_string()
        };

        let zoom_str = format!("{:.2}% ({})",
            self.config.image_state.zoom * 100.0,
            if self.config.image_state.is_fit_to_window { "fit" } else { "manual" }
        );

        let pan_str = format!("({:.1}, {:.1})",
            self.config.image_state.pan.0,
            self.config.image_state.pan.1
        );

        let image_dims_str = if let Some((w, h)) = self.config.image_dimensions {
            format!("{}x{}", w, h)
        } else {
            "N/A".to_string()
        };

        let viewport_str = if let Some(size) = self.config.viewport_size {
            let w: f32 = size.width.into();
            let h: f32 = size.height.into();
            format!("{:.0}x{:.0}", w, h)
        } else {
            "N/A".to_string()
        };

        // Calculate max width as 33% of viewport width, with min of 200px
        let max_width = if let Some(viewport) = self.config.viewport_size {
            let viewport_width: f32 = viewport.width.into();
            let calculated_width = (viewport_width * 0.33).max(200.0);
            px(calculated_width)
        } else {
            px(400.0) // Fallback if viewport size not available
        };

        div()
            // Position in top-right corner
            .absolute()
            .top(Spacing::md())
            .right(Spacing::md())
            .bg(Colors::overlay_bg_alpha(self.config.overlay_transparency))
            .border_1()
            .border_color(rgb(0x444444))
            .rounded(px(6.0))
            .p(Spacing::md())
            .shadow_lg()
            .min_w(px(200.0))
            .max_w(max_width)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(Spacing::xs())
                    // Title
                    .child(
                        div()
                            .text_size(scaled_text_size(14.0, self.config.font_size_scale))
                            .text_color(Colors::text())
                            .font_weight(FontWeight::BOLD)
                            .mb(Spacing::sm())
                            .pb(Spacing::xs())
                            .border_b_1()
                            .border_color(rgb(0x444444))
                            .child("Debug Information")
                    )
                    // Image info
                    .child(self.render_info_line_wrapping("Image Filename", filename_str))
                    .child(self.render_info_line_wrapping("Image Folder", folder_str))
                    .child(self.render_info_line("Image Index", index_str))
                    .child(self.render_info_line("Image Size", image_dims_str))

                    // Zoom & Pan info
                    .child(
                        div()
                            .mt(Spacing::sm())
                            .mb(Spacing::xs())
                            .text_size(scaled_text_size(12.0, self.config.font_size_scale))
                            .text_color(rgb(0x888888))
                            .font_weight(FontWeight::BOLD)
                            .child("Transform")
                    )
                    .child(self.render_info_line("Zoom", zoom_str))
                    .child(self.render_info_line("Pan (x, y)", pan_str))

                    // Viewport info
                    .child(
                        div()
                            .mt(Spacing::sm())
                            .mb(Spacing::xs())
                            .text_size(scaled_text_size(12.0, self.config.font_size_scale))
                            .text_color(rgb(0x888888))
                            .font_weight(FontWeight::BOLD)
                            .child("Viewport")
                    )
                    .child(self.render_info_line("Viewport Size", viewport_str))

                    // Close instructions
                    .child(
                        div()
                            .mt(Spacing::md())
                            .pt(Spacing::sm())
                            .border_t_1()
                            .border_color(rgb(0x444444))
                            .text_size(scaled_text_size(12.0, self.config.font_size_scale))
                            .text_color(rgb(0x888888))
                            .text_align(TextAlign::Center)
                            .child("Press F12 to close")
                    )
            )
    }
}
