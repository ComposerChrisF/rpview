use gpui::*;
use std::path::PathBuf;
use crate::utils::style::{Colors, Spacing, TextSize};
use crate::state::ImageState;

/// Component for displaying debug information
#[derive(Clone)]
pub struct DebugOverlay {
    pub current_path: Option<PathBuf>,
    pub current_index: usize,
    pub total_images: usize,
    pub image_state: ImageState,
    pub image_dimensions: Option<(u32, u32)>,
    pub viewport_size: Option<Size<Pixels>>,
}

impl DebugOverlay {
    pub fn new(
        current_path: Option<PathBuf>,
        current_index: usize,
        total_images: usize,
        image_state: ImageState,
        image_dimensions: Option<(u32, u32)>,
        viewport_size: Option<Size<Pixels>>,
    ) -> Self {
        Self {
            current_path,
            current_index,
            total_images,
            image_state,
            image_dimensions,
            viewport_size,
        }
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
                    .text_size(TextSize::sm())
                    .text_color(rgb(0x888888))
                    .child(format!("{}:", label))
            )
            .child(
                div()
                    .text_size(TextSize::sm())
                    .text_color(Colors::text())
                    .font_family("monospace")
                    .child(value)
            )
    }
}

impl Render for DebugOverlay {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let path_str = if let Some(ref path) = self.current_path {
            path.display().to_string()
        } else {
            "None".to_string()
        };
        
        let index_str = if self.total_images > 0 {
            format!("{} / {}", self.current_index + 1, self.total_images)
        } else {
            "0 / 0".to_string()
        };
        
        let zoom_str = format!("{:.2}% ({})", 
            self.image_state.zoom * 100.0,
            if self.image_state.is_fit_to_window { "fit" } else { "manual" }
        );
        
        let pan_str = format!("({:.1}, {:.1})", 
            self.image_state.pan.0, 
            self.image_state.pan.1
        );
        
        let image_dims_str = if let Some((w, h)) = self.image_dimensions {
            format!("{}x{}", w, h)
        } else {
            "N/A".to_string()
        };
        
        let viewport_str = if let Some(size) = self.viewport_size {
            let w: f32 = size.width.into();
            let h: f32 = size.height.into();
            format!("{:.0}x{:.0}", w, h)
        } else {
            "N/A".to_string()
        };
        
        div()
            // Position in top-right corner
            .absolute()
            .top(Spacing::md())
            .right(Spacing::md())
            .bg(rgba(0x000000DD))
            .border_1()
            .border_color(rgb(0x444444))
            .rounded(px(6.0))
            .p(Spacing::md())
            .shadow_lg()
            .min_w(px(400.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(Spacing::xs())
                    // Title
                    .child(
                        div()
                            .text_size(TextSize::md())
                            .text_color(Colors::text())
                            .font_weight(FontWeight::BOLD)
                            .mb(Spacing::sm())
                            .pb(Spacing::xs())
                            .border_b_1()
                            .border_color(rgb(0x444444))
                            .child("Debug Information")
                    )
                    // Image info
                    .child(self.render_info_line("Image Path", path_str))
                    .child(self.render_info_line("Image Index", index_str))
                    .child(self.render_info_line("Image Size", image_dims_str))
                    
                    // Zoom & Pan info
                    .child(
                        div()
                            .mt(Spacing::sm())
                            .mb(Spacing::xs())
                            .text_size(TextSize::sm())
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
                            .text_size(TextSize::sm())
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
                            .text_size(TextSize::sm())
                            .text_color(rgb(0x888888))
                            .text_align(TextAlign::Center)
                            .child("Press F12 to close")
                    )
            )
    }
}
