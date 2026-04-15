//! Minimal root view for the floating Filter window. Wraps the shared
//! `FilterControls` entity so the same sliders can be rendered outside the
//! main window.

use crate::components::FilterControls;
use gpui::*;

pub struct FilterWindowView {
    pub filter_controls: Entity<FilterControls>,
    pub focus_handle: FocusHandle,
}

impl FilterWindowView {
    pub fn new(filter_controls: Entity<FilterControls>, cx: &mut Context<Self>) -> Self {
        Self {
            filter_controls,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for FilterWindowView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FilterWindowView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.focus_handle)
            .child(self.filter_controls.clone())
    }
}
