use gpui::*;
use adabraka_ui::components::slider::{Slider, SliderState};
use crate::utils::style::{Colors, Spacing, TextSize};
use crate::state::image_state::FilterSettings;

/// Filter controls overlay component
pub struct FilterControls {
    /// Slider states for each filter
    pub brightness_slider: Entity<SliderState>,
    pub contrast_slider: Entity<SliderState>,
    pub gamma_slider: Entity<SliderState>,
    
    /// Last known filter values (to detect changes)
    last_brightness: f32,
    last_contrast: f32,
    last_gamma: f32,
}

impl FilterControls {
    pub fn new(filters: FilterSettings, cx: &mut Context<Self>) -> Self {
        // Create brightness slider (-100 to +100, current value)
        let brightness_slider = cx.new(|cx| {
            let mut state = SliderState::new(cx);
            state.set_min(-100.0, cx);
            state.set_max(100.0, cx);
            state.set_value(filters.brightness, cx);
            state.set_step(1.0, cx);
            state
        });
        
        // Create contrast slider (-100 to +100, current value)
        let contrast_slider = cx.new(|cx| {
            let mut state = SliderState::new(cx);
            state.set_min(-100.0, cx);
            state.set_max(100.0, cx);
            state.set_value(filters.contrast, cx);
            state.set_step(1.0, cx);
            state
        });
        
        // Create gamma slider (0.1 to 10.0, current value)
        let gamma_slider = cx.new(|cx| {
            let mut state = SliderState::new(cx);
            state.set_min(0.1, cx);
            state.set_max(10.0, cx);
            state.set_value(filters.gamma, cx);
            state.set_step(0.01, cx);
            state
        });
        
        Self {
            brightness_slider,
            contrast_slider,
            gamma_slider,
            last_brightness: filters.brightness,
            last_contrast: filters.contrast,
            last_gamma: filters.gamma,
        }
    }
    
    /// Update slider values from filter settings (e.g., when filters are reset)
    pub fn update_from_filters(&mut self, filters: FilterSettings, cx: &mut Context<Self>) {
        self.last_brightness = filters.brightness;
        self.last_contrast = filters.contrast;
        self.last_gamma = filters.gamma;
        
        self.brightness_slider.update(cx, |state, cx| {
            state.set_value(filters.brightness, cx);
        });
        self.contrast_slider.update(cx, |state, cx| {
            state.set_value(filters.contrast, cx);
        });
        self.gamma_slider.update(cx, |state, cx| {
            state.set_value(filters.gamma, cx);
        });
    }
    
    /// Get current filter settings from sliders and detect if they changed
    /// Returns (current_filters, has_changed)
    pub fn get_filters_and_detect_change(&mut self, cx: &App) -> (FilterSettings, bool) {
        let current = FilterSettings {
            brightness: self.brightness_slider.read(cx).value(),
            contrast: self.contrast_slider.read(cx).value(),
            gamma: self.gamma_slider.read(cx).value(),
        };
        
        eprintln!("[FilterControls] Current values: brightness={:.1}, contrast={:.1}, gamma={:.2}", 
            current.brightness, current.contrast, current.gamma);
        eprintln!("[FilterControls] Last values: brightness={:.1}, contrast={:.1}, gamma={:.2}", 
            self.last_brightness, self.last_contrast, self.last_gamma);
        
        let changed = current.brightness != self.last_brightness
            || current.contrast != self.last_contrast
            || current.gamma != self.last_gamma;
        
        eprintln!("[FilterControls] Changed: {}", changed);
        
        if changed {
            self.last_brightness = current.brightness;
            self.last_contrast = current.contrast;
            self.last_gamma = current.gamma;
            eprintln!("[FilterControls] Updated last values");
        }
        
        (current, changed)
    }
}

impl Render for FilterControls {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let brightness_value = self.brightness_slider.read(cx).value();
        let contrast_value = self.contrast_slider.read(cx).value();
        let gamma_value = self.gamma_slider.read(cx).value();
        let platform_key = crate::utils::style::format_shortcut("Cmd");
        
        div()
            .absolute()
            .top(px(20.0))
            .right(px(20.0))
            .bg(rgba(0x00_00_00_CC))
            .border_1()
            .border_color(rgba(0x44_44_44_FF))
            .rounded(px(8.0))
            .p(Spacing::lg())
            .min_w(px(320.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(Spacing::md())
                    .child(
                        div()
                            .text_size(TextSize::lg())
                            .text_color(Colors::text())
                            .font_weight(FontWeight::BOLD)
                            .child("Filter Controls")
                    )
                    .child(
                        div()
                            .h(px(1.0))
                            .bg(rgba(0x44_44_44_FF))
                    )
                    // Brightness slider
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(TextSize::sm())
                                            .text_color(Colors::text())
                                            .child("Brightness")
                                    )
                                    .child(
                                        div()
                                            .text_size(TextSize::sm())
                                            .text_color(rgb(0xAAAAAA))
                                            .font_weight(FontWeight::BOLD)
                                            .child(format!("{:+.0}", brightness_value))
                                    )
                            )
                            .child(Slider::new(self.brightness_slider.clone()))
                    )
                    // Contrast slider
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(TextSize::sm())
                                            .text_color(Colors::text())
                                            .child("Contrast")
                                    )
                                    .child(
                                        div()
                                            .text_size(TextSize::sm())
                                            .text_color(rgb(0xAAAAAA))
                                            .font_weight(FontWeight::BOLD)
                                            .child(format!("{:+.0}", contrast_value))
                                    )
                            )
                            .child(Slider::new(self.contrast_slider.clone()))
                    )
                    // Gamma slider
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(TextSize::sm())
                                            .text_color(Colors::text())
                                            .child("Gamma")
                                    )
                                    .child(
                                        div()
                                            .text_size(TextSize::sm())
                                            .text_color(rgb(0xAAAAAA))
                                            .font_weight(FontWeight::BOLD)
                                            .child(format!("{:.2}", gamma_value))
                                    )
                            )
                            .child(Slider::new(self.gamma_slider.clone()))
                    )
                    .child(
                        div()
                            .h(px(1.0))
                            .bg(rgba(0x44_44_44_FF))
                            .mt(Spacing::sm())
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0xAAAAAA))
                                    .child("Click and drag sliders to adjust")
                            )
                            .child(
                                div()
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0xAAAAAA))
                                    .child(format!("{}/1: Disable/Enable", platform_key))
                            )
                            .child(
                                div()
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0xAAAAAA))
                                    .child(format!("{}/R: Reset all", platform_key))
                            )
                    )
            )
    }
}
