//! Minimal root view for the floating Filter window.

use crate::EscapePressed;
use crate::components::FilterControls;
use gpui::*;

pub type EscapeCallback = Box<dyn Fn(&mut Window, &mut App) + 'static>;

pub struct FilterWindowView {
    pub filter_controls: Entity<FilterControls>,
    pub focus_handle: FocusHandle,
    pub on_escape: EscapeCallback,
}

impl FilterWindowView {
    pub fn new(
        filter_controls: Entity<FilterControls>,
        on_escape: EscapeCallback,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            filter_controls,
            focus_handle: cx.focus_handle(),
            on_escape,
        }
    }
}

impl Focusable for FilterWindowView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FilterWindowView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Share the `ImageViewer` key context so plain `1` / `2` fire here.
        div()
            .size_full()
            .key_context("ImageViewer")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &EscapePressed, window, cx| {
                (this.on_escape)(window, cx);
            }))
            .child(self.filter_controls.clone())
    }
}
