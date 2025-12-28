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
}

impl ZoomIndicator {
    pub fn new(zoom: f32, is_fit_to_window: bool) -> Self {
        Self {
            zoom,
            is_fit_to_window,
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
        
        div()
            .absolute()
            .bottom(Spacing::lg())
            .right(Spacing::lg())
            .p(Spacing::md())
            .bg(rgba(0x000000AA))
            .rounded(px(6.0))
            .border_1()
            .border_color(rgba(0x444444FF))
            .child(
                div()
                    .text_size(TextSize::sm())
                    .text_color(Colors::text())
                    .child(zoom_text)
            )
    }
}
