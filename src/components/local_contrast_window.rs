//! Minimal root view for the floating Local Contrast window.

use crate::EscapePressed;
use crate::components::LocalContrastControls;
use gpui::*;

/// Callback invoked when the user presses ESC while this window has focus.
/// The owning binary uses this to close the window and tick the main App's
/// quit counter; kept as a closure so this component lives in the crate
/// shared between lib and bin builds without depending on the binary's
/// `App` type.
pub type EscapeCallback = Box<dyn Fn(&mut Window, &mut App) + 'static>;

pub struct LocalContrastWindowView {
    pub controls: Entity<LocalContrastControls>,
    pub focus_handle: FocusHandle,
    pub on_escape: EscapeCallback,
}

impl LocalContrastWindowView {
    pub fn new(
        controls: Entity<LocalContrastControls>,
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

impl Focusable for LocalContrastWindowView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LocalContrastWindowView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Participate in the `ImageViewer` key context so the plain `1` and
        // `2` keybindings (DisableFilters / EnableFilters, which also toggle
        // LC) fire while this dialog has focus.
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
