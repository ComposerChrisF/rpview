use gpui::*;
use crate::utils::style::{Colors, Spacing, scaled_text_size};

/// Component that displays animation status and frame counter
pub struct AnimationIndicator {
    current_frame: usize,
    total_frames: usize,
    is_playing: bool,
    /// Overlay transparency (0-255)
    overlay_transparency: u8,
    /// Font size scale multiplier
    font_size_scale: f32,
}

impl AnimationIndicator {
    pub fn new(current_frame: usize, total_frames: usize, is_playing: bool, overlay_transparency: u8, font_size_scale: f32) -> Self {
        Self {
            current_frame,
            total_frames,
            is_playing,
            overlay_transparency,
            font_size_scale,
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
            .bg(Colors::overlay_bg_alpha(self.overlay_transparency))
            .rounded(px(4.0))
            .text_size(scaled_text_size(12.0, self.font_size_scale))
            .text_color(Colors::text())
            .child(frame_text)
    }
}
