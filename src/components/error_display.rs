use crate::utils::style::{Colors, Spacing, TextSize};
use gpui::*;

/// Component for displaying error messages
pub struct ErrorDisplay {
    message: SharedString,
    text_color: Hsla,
}

impl ErrorDisplay {
    pub fn new(message: impl Into<SharedString>) -> Self {
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

impl Render for ErrorDisplay {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
                    .text_color(Colors::error())
                    .child("⚠️ Error"),
            )
            .child(
                div()
                    .text_size(TextSize::md())
                    .text_color(self.text_color)
                    .child(self.message.clone()),
            )
    }
}
