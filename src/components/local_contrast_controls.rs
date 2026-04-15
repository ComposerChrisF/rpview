//! Sliders for the Local Contrast dialog.
//!
//! Mirrors the pattern used by `FilterControls`: one `Entity<Slider>` per
//! knob, each slider emits a `Change` event that this component translates
//! to a higher-level `LocalContrastControlsEvent::ParametersChanged` so the
//! owning `App` can react (cancel any in-flight compute, kick off a new one).
//!
//! Phase D MVP exposes three parameters: contrast, lighten-shadows,
//! darken-highlights. The rest of `local_contrast::Parameters` stays at its
//! defaults; we can extend this UI in a follow-up without touching the
//! underlying algorithm.

use crate::utils::local_contrast::Parameters;
use crate::utils::style::{Colors, Spacing, scaled_text_size};
use ccf_gpui_widgets::prelude::{Slider, SliderEvent};
use gpui::*;

/// Events emitted by `LocalContrastControls`.
#[derive(Clone, Debug)]
pub enum LocalContrastControlsEvent {
    /// One of the sliders changed; owning App should read the new parameters
    /// and kick off a recompute.
    ParametersChanged,
}

pub struct LocalContrastControls {
    pub contrast_slider: Entity<Slider>,
    pub lighten_shadows_slider: Entity<Slider>,
    pub darken_highlights_slider: Entity<Slider>,
    pub font_size_scale: f32,
    pub status: String,
}

impl EventEmitter<LocalContrastControlsEvent> for LocalContrastControls {}

impl LocalContrastControls {
    pub fn new(font_size_scale: f32, cx: &mut Context<Self>) -> Self {
        let contrast_slider = cx.new(|cx| {
            Slider::new(cx)
                .with_value(0.0)
                .min(0.0)
                .max(1.0)
                .step(0.01)
                .display_precision(2)
        });
        cx.subscribe(
            &contrast_slider,
            |_this, _slider, event: &SliderEvent, cx| {
                if let SliderEvent::Change(_) = event {
                    cx.emit(LocalContrastControlsEvent::ParametersChanged);
                }
            },
        )
        .detach();

        let lighten_shadows_slider = cx.new(|cx| {
            Slider::new(cx)
                .with_value(0.0)
                .min(0.0)
                .max(1.0)
                .step(0.01)
                .display_precision(2)
        });
        cx.subscribe(
            &lighten_shadows_slider,
            |_this, _slider, event: &SliderEvent, cx| {
                if let SliderEvent::Change(_) = event {
                    cx.emit(LocalContrastControlsEvent::ParametersChanged);
                }
            },
        )
        .detach();

        let darken_highlights_slider = cx.new(|cx| {
            Slider::new(cx)
                .with_value(0.0)
                .min(0.0)
                .max(1.0)
                .step(0.01)
                .display_precision(2)
        });
        cx.subscribe(
            &darken_highlights_slider,
            |_this, _slider, event: &SliderEvent, cx| {
                if let SliderEvent::Change(_) = event {
                    cx.emit(LocalContrastControlsEvent::ParametersChanged);
                }
            },
        )
        .detach();

        Self {
            contrast_slider,
            lighten_shadows_slider,
            darken_highlights_slider,
            font_size_scale,
            status: String::new(),
        }
    }

    /// Read the current slider values into an `lc::Parameters`.
    pub fn get_parameters(&self, cx: &App) -> Parameters {
        Parameters {
            contrast: self.contrast_slider.read(cx).value() as f32,
            lighten_shadows: self.lighten_shadows_slider.read(cx).value() as f32,
            darken_highlights: self.darken_highlights_slider.read(cx).value() as f32,
            ..Default::default()
        }
    }

    /// Set all three sliders back to zero.
    pub fn reset_sliders(&mut self, cx: &mut Context<Self>) {
        self.contrast_slider
            .update(cx, |s, cx| s.set_value(0.0, cx));
        self.lighten_shadows_slider
            .update(cx, |s, cx| s.set_value(0.0, cx));
        self.darken_highlights_slider
            .update(cx, |s, cx| s.set_value(0.0, cx));
        cx.emit(LocalContrastControlsEvent::ParametersChanged);
    }

    /// Update the status label below the sliders (e.g. "Processing…" or "Ready").
    pub fn set_status(&mut self, status: impl Into<String>, cx: &mut Context<Self>) {
        self.status = status.into();
        cx.notify();
    }
}

fn labeled_slider(
    title: &'static str,
    value_text: String,
    slider: Entity<Slider>,
    font_scale: f32,
) -> impl IntoElement {
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
                        .text_size(scaled_text_size(12.0, font_scale))
                        .text_color(Colors::text())
                        .child(title),
                )
                .child(
                    div()
                        .text_size(scaled_text_size(12.0, font_scale))
                        .text_color(rgb(0xAAAAAA))
                        .font_weight(FontWeight::BOLD)
                        .child(value_text),
                ),
        )
        .child(slider)
}

impl Render for LocalContrastControls {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let contrast_val = self.contrast_slider.read(cx).value();
        let shadows_val = self.lighten_shadows_slider.read(cx).value();
        let highlights_val = self.darken_highlights_slider.read(cx).value();
        let font_scale = self.font_size_scale;

        div()
            .size_full()
            .bg(Colors::background())
            .p(Spacing::lg())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(Spacing::md())
                    .child(
                        div()
                            .text_size(scaled_text_size(16.0, font_scale))
                            .text_color(Colors::text())
                            .font_weight(FontWeight::BOLD)
                            .child("Local Contrast"),
                    )
                    .child(div().h(px(1.0)).bg(rgba(0x44_44_44_FF)))
                    .child(labeled_slider(
                        "Contrast",
                        format!("{:.2}", contrast_val),
                        self.contrast_slider.clone(),
                        font_scale,
                    ))
                    .child(labeled_slider(
                        "Lighten Shadows",
                        format!("{:.2}", shadows_val),
                        self.lighten_shadows_slider.clone(),
                        font_scale,
                    ))
                    .child(labeled_slider(
                        "Darken Highlights",
                        format!("{:.2}", highlights_val),
                        self.darken_highlights_slider.clone(),
                        font_scale,
                    ))
                    .child(div().h(px(1.0)).bg(rgba(0x44_44_44_FF)).mt(Spacing::sm()))
                    .child(
                        div()
                            .text_size(scaled_text_size(12.0, font_scale))
                            .text_color(rgb(0xAAAAAA))
                            .child(self.status.clone()),
                    ),
            )
    }
}
