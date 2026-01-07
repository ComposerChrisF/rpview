use gpui::*;
use crate::utils::style::{Colors, Spacing, scaled_text_size};

/// Processing indicator component
/// Displays in the upper left corner while filters are being processed
#[derive(Clone)]
pub struct ProcessingIndicator {
    pub message: String,
    /// Overlay transparency (0-255)
    pub overlay_transparency: u8,
    /// Font size scale multiplier
    pub font_size_scale: f32,
}

impl ProcessingIndicator {
    pub fn new(message: impl Into<String>, overlay_transparency: u8, font_size_scale: f32) -> Self {
        Self {
            message: message.into(),
            overlay_transparency,
            font_size_scale,
        }
    }
}

impl Render for ProcessingIndicator {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .top(Spacing::lg())
            .left(Spacing::lg())
            .p(Spacing::md())
            .bg(Colors::overlay_bg_alpha(self.overlay_transparency))
            .rounded(px(6.0))
            .border_1()
            .border_color(rgba(0x444444FF))
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::sm())
            .child(
                // Spinner/processing indicator (pulsing dot)
                div()
                    .size(px(12.0))
                    .rounded(px(6.0))
                    .bg(rgb(0x50fa7b)) // Green color
            )
            .child(
                div()
                    .text_size(scaled_text_size(13.0, self.font_size_scale))
                    .text_color(Colors::text())
                    .child(self.message.clone())
            )
    }
}
