use gpui::*;
use crate::utils::style::{Colors, Spacing, TextSize};

/// Component that displays animation status and frame counter
pub struct AnimationIndicator {
    current_frame: usize,
    total_frames: usize,
    is_playing: bool,
}

impl AnimationIndicator {
    pub fn new(current_frame: usize, total_frames: usize, is_playing: bool) -> Self {
        Self {
            current_frame,
            total_frames,
            is_playing,
        }
    }
}

impl Render for AnimationIndicator {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let play_status = if self.is_playing { "▶" } else { "⏸" };
        let frame_text = format!("{} Frame {}/{}", play_status, self.current_frame + 1, self.total_frames);
        
        div()
            .absolute()
            .bottom(Spacing::md())
            .left(Spacing::md())
            .px(Spacing::md())
            .py(Spacing::sm())
            .bg(rgba(0x000000AA))
            .rounded(px(4.0))
            .text_size(TextSize::sm())
            .text_color(Colors::text())
            .child(frame_text)
    }
}
