use crate::utils::style::{Colors, Spacing, TextSize};
use gpui::*;

/// Loading indicator component
/// Displays a spinner and optional message while images are loading
#[derive(Clone)]
pub struct LoadingIndicator {
    pub message: String,
    text_color: Hsla,
}

impl LoadingIndicator {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            text_color: Colors::text(),
        }
    }

    /// Set custom text color (for contrast with light backgrounds)
    pub fn with_text_color(mut self, color: Hsla) -> Self {
        self.text_color = color;
        self
    }
}

impl Render for LoadingIndicator {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(Spacing::md())
            .child(
                // Spinner animation (simple pulsing dot)
                div().flex().items_center().justify_center().child(
                    div().size(px(16.0)).rounded(px(8.0)).bg(rgb(0x50fa7b)), // Green color
                ),
            )
            .child(
                div()
                    .text_size(TextSize::lg())
                    .text_color(self.text_color)
                    .child(self.message.clone()),
            )
    }
}
