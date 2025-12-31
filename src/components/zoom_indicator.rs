use gpui::*;
use crate::utils::style::{Colors, Spacing, TextSize};
use crate::utils::zoom;

/// Component for displaying the current zoom level
#[derive(Clone)]
pub struct ZoomIndicator {
    /// Current zoom level
    pub zoom: f32,
    /// Whether at fit-to-window size
    pub is_fit_to_window: bool,
    /// Image dimensions (width, height)
    pub image_dimensions: Option<(u32, u32)>,
}

impl ZoomIndicator {
    pub fn new(zoom: f32, is_fit_to_window: bool, image_dimensions: Option<(u32, u32)>) -> Self {
        Self {
            zoom,
            is_fit_to_window,
            image_dimensions,
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
            .bg(rgba(0x000000AA))
            .rounded(px(6.0))
            .border_1()
            .border_color(rgba(0x444444FF))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .text_size(TextSize::sm())
                    .text_color(Colors::text())
                    .child(zoom_text)
            );
        
        // Add dimensions line if available
        if let Some((width, height)) = self.image_dimensions {
            container = container.child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgba(0xAAAAAAFF))
                    .child(format!("{}×{}", width, height))
            );
        }
        
        container
    }
}
