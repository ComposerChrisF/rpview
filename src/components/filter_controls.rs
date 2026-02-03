use crate::state::image_state::FilterSettings;
use crate::utils::style::{Colors, Spacing, TextSize, scaled_text_size};
use ccf_gpui_widgets::prelude::{Slider, SliderEvent};
use gpui::*;

/// Events emitted by FilterControls
#[derive(Clone, Debug)]
pub enum FilterControlsEvent {
    /// Filter settings changed via slider interaction
    FiltersChanged,
}

/// Filter controls overlay component
pub struct FilterControls {
    /// Slider entities for each filter
    pub brightness_slider: Entity<Slider>,
    pub contrast_slider: Entity<Slider>,
    pub gamma_slider: Entity<Slider>,

    /// Overlay transparency (0-255)
    pub overlay_transparency: u8,
    /// Font size scale multiplier
    pub font_size_scale: f32,
}

impl EventEmitter<FilterControlsEvent> for FilterControls {}

impl FilterControls {
    pub fn new(
        filters: FilterSettings,
        overlay_transparency: u8,
        font_size_scale: f32,
        cx: &mut Context<Self>,
    ) -> Self {
        // Create brightness slider (-100 to +100, current value)
        let brightness_slider = cx.new(|cx| {
            Slider::new(cx)
                .with_value(filters.brightness as f64)
                .min(-100.0)
                .max(100.0)
                .step(1.0)
                .display_precision(0)
        });

        // Subscribe to brightness slider changes
        cx.subscribe(&brightness_slider, |_this, _slider, event: &SliderEvent, cx| {
            if let SliderEvent::Change(_) = event {
                cx.emit(FilterControlsEvent::FiltersChanged);
            }
        })
        .detach();

        // Create contrast slider (-100 to +100, current value)
        let contrast_slider = cx.new(|cx| {
            Slider::new(cx)
                .with_value(filters.contrast as f64)
                .min(-100.0)
                .max(100.0)
                .step(1.0)
                .display_precision(0)
        });

        // Subscribe to contrast slider changes
        cx.subscribe(&contrast_slider, |_this, _slider, event: &SliderEvent, cx| {
            if let SliderEvent::Change(_) = event {
                cx.emit(FilterControlsEvent::FiltersChanged);
            }
        })
        .detach();

        // Create gamma slider (0.1 to 10.0, current value)
        let gamma_slider = cx.new(|cx| {
            Slider::new(cx)
                .with_value(filters.gamma as f64)
                .min(0.1)
                .max(10.0)
                .step(0.01)
                .display_precision(2)
        });

        // Subscribe to gamma slider changes
        cx.subscribe(&gamma_slider, |_this, _slider, event: &SliderEvent, cx| {
            if let SliderEvent::Change(_) = event {
                cx.emit(FilterControlsEvent::FiltersChanged);
            }
        })
        .detach();

        Self {
            brightness_slider,
            contrast_slider,
            gamma_slider,
            overlay_transparency,
            font_size_scale,
        }
    }

    /// Update slider values from filter settings (e.g., when filters are reset)
    pub fn update_from_filters(&mut self, filters: FilterSettings, cx: &mut Context<Self>) {
        self.brightness_slider.update(cx, |slider, cx| {
            slider.set_value(filters.brightness as f64, cx);
        });
        self.contrast_slider.update(cx, |slider, cx| {
            slider.set_value(filters.contrast as f64, cx);
        });
        self.gamma_slider.update(cx, |slider, cx| {
            slider.set_value(filters.gamma as f64, cx);
        });
    }

    /// Get current filter settings from sliders
    pub fn get_filters(&self, cx: &App) -> FilterSettings {
        FilterSettings {
            brightness: self.brightness_slider.read(cx).value() as f32,
            contrast: self.contrast_slider.read(cx).value() as f32,
            gamma: self.gamma_slider.read(cx).value() as f32,
        }
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
            .bg(Colors::overlay_bg_alpha(self.overlay_transparency))
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
                            .text_size(scaled_text_size(16.0, self.font_size_scale))
                            .text_color(Colors::text())
                            .font_weight(FontWeight::BOLD)
                            .child("Filter Controls"),
                    )
                    .child(div().h(px(1.0)).bg(rgba(0x44_44_44_FF)))
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
                                            .text_size(scaled_text_size(12.0, self.font_size_scale))
                                            .text_color(Colors::text())
                                            .child("Brightness"),
                                    )
                                    .child(
                                        div()
                                            .text_size(scaled_text_size(12.0, self.font_size_scale))
                                            .text_color(rgb(0xAAAAAA))
                                            .font_weight(FontWeight::BOLD)
                                            .child(format!("{:+.0}", brightness_value)),
                                    ),
                            )
                            .child(self.brightness_slider.clone()),
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
                                            .text_size(scaled_text_size(12.0, self.font_size_scale))
                                            .text_color(Colors::text())
                                            .child("Contrast"),
                                    )
                                    .child(
                                        div()
                                            .text_size(scaled_text_size(12.0, self.font_size_scale))
                                            .text_color(rgb(0xAAAAAA))
                                            .font_weight(FontWeight::BOLD)
                                            .child(format!("{:+.0}", contrast_value)),
                                    ),
                            )
                            .child(self.contrast_slider.clone()),
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
                                            .text_size(scaled_text_size(12.0, self.font_size_scale))
                                            .text_color(Colors::text())
                                            .child("Gamma"),
                                    )
                                    .child(
                                        div()
                                            .text_size(scaled_text_size(12.0, self.font_size_scale))
                                            .text_color(rgb(0xAAAAAA))
                                            .font_weight(FontWeight::BOLD)
                                            .child(format!("{:.2}", gamma_value)),
                                    ),
                            )
                            .child(self.gamma_slider.clone()),
                    )
                    .child(div().h(px(1.0)).bg(rgba(0x44_44_44_FF)).mt(Spacing::sm()))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0xAAAAAA))
                                    .child("Click and drag sliders to adjust"),
                            )
                            .child(
                                div()
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0xAAAAAA))
                                    .child(format!("{}/1: Disable/Enable", platform_key)),
                            )
                            .child(
                                div()
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0xAAAAAA))
                                    .child(format!("Shift+{}/R: Reset all", platform_key)),
                            ),
                    ),
            )
    }
}
