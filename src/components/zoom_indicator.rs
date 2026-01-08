use crate::utils::style::{Colors, Spacing, scaled_text_size};
use crate::utils::zoom;
use gpui::*;

/// Component for displaying the current zoom level
#[derive(Clone)]
pub struct ZoomIndicator {
    /// Current zoom level
    pub zoom: f32,
    /// Whether at fit-to-window size
    pub is_fit_to_window: bool,
    /// Image dimensions (width, height)
    pub image_dimensions: Option<(u32, u32)>,
    /// Overlay transparency (0-255)
    pub overlay_transparency: u8,
    /// Font size scale multiplier
    pub font_size_scale: f32,
}

impl ZoomIndicator {
    pub fn new(
        zoom: f32,
        is_fit_to_window: bool,
        image_dimensions: Option<(u32, u32)>,
        overlay_transparency: u8,
        font_size_scale: f32,
    ) -> Self {
        Self {
            zoom,
            is_fit_to_window,
            image_dimensions,
            overlay_transparency,
            font_size_scale,
        }
    }
}

impl Render for ZoomIndicator {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let zoom_text = if self.is_fit_to_window {
            format!("Fit ({})", zoom::format_zoom_percentage(self.zoom))
        } else {
            zoom::format_zoom_percentage(self.zoom)
        };

        let mut container = div()
            .absolute()
            .bottom(Spacing::lg())
            .right(Spacing::lg())
            .p(Spacing::md())
            .bg(Colors::overlay_bg_alpha(self.overlay_transparency))
            .rounded(px(6.0))
            .border_1()
            .border_color(rgba(0x444444FF))
            .flex()
            .flex_col()
            .items_center() // Center all children horizontally
            .gap(px(2.0))
            .child(
                div()
                    .text_size(scaled_text_size(12.0, self.font_size_scale))
                    .text_color(Colors::text())
                    .child(zoom_text),
            );

        // Add dimensions line if available
        if let Some((width, height)) = self.image_dimensions {
            container = container.child(
                div()
                    .text_size(scaled_text_size(11.0, self.font_size_scale))
                    .text_color(rgba(0xAAAAAAFF))
                    .child(format!("{}×{}", width, height)),
            );
        }

        container
    }
}
