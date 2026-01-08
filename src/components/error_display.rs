use crate::utils::style::{Colors, Spacing, TextSize};
use gpui::*;

/// Component for displaying error messages
pub struct ErrorDisplay {
    message: SharedString,
}

impl ErrorDisplay {
    pub fn new(message: impl Into<SharedString>) -> Self {
        Self {
            message: message.into(),
        }
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
                    .text_color(Colors::text())
                    .child(self.message.clone()),
            )
    }
}
