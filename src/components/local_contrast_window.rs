//! Minimal root view for the floating Local Contrast window. Analogous to
//! `FilterWindowView`: wraps the shared `LocalContrastControls` entity.

use crate::components::LocalContrastControls;
use gpui::*;

pub struct LocalContrastWindowView {
    pub controls: Entity<LocalContrastControls>,
    pub focus_handle: FocusHandle,
}

impl LocalContrastWindowView {
    pub fn new(controls: Entity<LocalContrastControls>, cx: &mut Context<Self>) -> Self {
        Self {
            controls,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for LocalContrastWindowView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LocalContrastWindowView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(self.controls.clone())
    }
}
