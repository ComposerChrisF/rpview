use gpui::prelude::*;
use gpui::*;
use crate::utils::style::{Colors, Spacing, TextSize};

/// Component for displaying help information and keyboard shortcuts
#[derive(Clone)]
pub struct HelpOverlay;

impl HelpOverlay {
    pub fn new() -> Self {
        Self
    }
    
    /// Render a section header
    fn render_section_header(&self, title: String) -> impl IntoElement {
        div()
            .text_size(TextSize::md())
            .text_color(Colors::text())
            .font_weight(FontWeight::BOLD)
            .mb(Spacing::sm())
            .mt(Spacing::md())
            .child(title)
    }
    
    /// Render a keyboard shortcut entry
    fn render_shortcut(&self, keys: String, description: String) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .gap(Spacing::md())
            .mb(Spacing::xs())
            .child(
                div()
                    .min_w(px(140.0))
                    .text_size(TextSize::sm())
                    .text_color(rgb(0xaaaaaa))
                    .font_family("monospace")
                    .child(keys)
            )
            .child(
                div()
                    .text_size(TextSize::sm())
                    .text_color(Colors::text())
                    .child(description)
            )
    }
}

impl Render for HelpOverlay {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let platform_key = if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        };
        
        div()
            // Full screen overlay with semi-transparent background
            .absolute()
            .inset_0()
            .bg(rgba(0x00000099))
            .flex()
            .items_center()
            .justify_center()
            .child(
                // Help content box
                div()
                    .bg(rgb(0x1e1e1e))
                    .border_1()
                    .border_color(rgb(0x444444))
                    .rounded(px(8.0))
                    .w(px(400.0))
                    .h(px(950.0))
                    .shadow_lg()
                    .p(Spacing::xl())
                    .child(
                        // Inner content: grows naturally beyond container height
                        div()
                            .flex()
                            .flex_col()
                            .gap(Spacing::xs())
                            // Title
                            .child(
                                div()
                                    .text_size(TextSize::xl())
                                    .text_color(Colors::text())
                                    .font_weight(FontWeight::BOLD)
                                    .mb(Spacing::md())
                                    .child("Keyboard Shortcuts")
                            )
                            // Navigation section
                            .child(self.render_section_header("Navigation".to_string()))
                            .child(self.render_shortcut("← →".to_string(), "Previous/Next image".to_string()))
                            .child(self.render_shortcut(format!("Shift+{}-A", platform_key), "Sort alphabetically".to_string()))
                            .child(self.render_shortcut(format!("Shift+{}-M", platform_key), "Sort by modified date".to_string()))
                            
                            // Zoom section
                            .child(self.render_section_header("Zoom".to_string()))
                            .child(self.render_shortcut("+ / =".to_string(), "Zoom in".to_string()))
                            .child(self.render_shortcut("-".to_string(), "Zoom out".to_string()))
                            .child(self.render_shortcut("0".to_string(), "Toggle fit-to-window / 100%".to_string()))
                            .child(self.render_shortcut("Shift + / -".to_string(), "Fast zoom (1.5x steps)".to_string()))
                            .child(self.render_shortcut(format!("{} + / -", platform_key), "Slow zoom (1.05x steps)".to_string()))
                            .child(self.render_shortcut(format!("Shift+{} + / -", platform_key), "Incremental zoom (1% steps)".to_string()))
                            .child(self.render_shortcut(format!("{}-Scroll", platform_key), "Zoom at cursor (mouse wheel)".to_string()))
                            .child(self.render_shortcut("Z + Drag".to_string(), "Drag to zoom (dynamic)".to_string()))
                            
                            // Pan section
                            .child(self.render_section_header("Pan".to_string()))
                            .child(self.render_shortcut("W A S D / I J K L".to_string(), "Pan image".to_string()))
                            .child(self.render_shortcut("Shift + WASD/IJKL".to_string(), "Fast pan (3x speed)".to_string()))
                            .child(self.render_shortcut(format!("{} + WASD/IJKL", platform_key), "Slow pan (0.3x speed)".to_string()))
                            .child(self.render_shortcut("Space + Drag".to_string(), "Pan with mouse (1:1 movement)".to_string()))
                            
                            // Window section
                            .child(self.render_section_header("Window".to_string()))
                            .child(self.render_shortcut(format!("{}-W", platform_key), "Close window".to_string()))
                            .child(self.render_shortcut(format!("{}-Q", platform_key), "Quit application".to_string()))
                            .child(self.render_shortcut("Esc (3x)".to_string(), "Quick quit (press 3 times within 2s)".to_string()))
                            
                            // Help section
                            .child(self.render_section_header("Help & Debug".to_string()))
                            .child(self.render_shortcut("H / ? / F1".to_string(), "Toggle this help overlay".to_string()))
                            .child(self.render_shortcut("F12".to_string(), "Toggle debug overlay".to_string()))
                            
                            // Close instructions
                            .child(
                                div()
                                    .mt(Spacing::lg())
                                    .pt(Spacing::md())
                                    .border_t_1()
                                    .border_color(rgb(0x444444))
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0xaaaaaa))
                                    .text_align(TextAlign::Center)
                                    .child("Press H, ?, F1, or Esc to close this help")
                            )
                    )
            )
    }
}
