//! Minimal root view for the floating GPU Pipeline window.

use crate::EscapePressed;
use crate::components::{EscapeCallback, GpuPipelineControls};
use gpui::*;

pub struct GpuPipelineWindowView {
    pub controls: Entity<GpuPipelineControls>,
    pub focus_handle: FocusHandle,
    pub on_escape: EscapeCallback,
}

impl GpuPipelineWindowView {
    pub fn new(
        controls: Entity<GpuPipelineControls>,
        on_escape: EscapeCallback,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            controls,
            focus_handle: cx.focus_handle(),
            on_escape,
        }
    }
}

impl Focusable for GpuPipelineWindowView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GpuPipelineWindowView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Share the `ImageViewer` key context so navigation/zoom shortcuts
        // remain functional while this dialog has focus.
        div()
            .size_full()
            .key_context("ImageViewer")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &EscapePressed, window, cx| {
                (this.on_escape)(window, cx);
            }))
            .child(self.controls.clone())
    }
}
