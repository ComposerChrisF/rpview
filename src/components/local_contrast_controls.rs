//! Sliders + toggles for the Local Contrast dialog.
//!
//! Slider changes emit `ParametersChanged`; the Reset button emits
//! `ResetRequested`. The owning `App` reacts to both (kicks off a recompute
//! or clears the LC render respectively).

use crate::utils::local_contrast::Parameters;
use crate::utils::style::{Colors, Spacing, scaled_text_size};
use ccf_gpui_widgets::prelude::{Slider, SliderEvent};
use gpui::prelude::FluentBuilder;
use gpui::*;

#[derive(Clone, Debug)]
pub enum LocalContrastControlsEvent {
    /// A slider or advanced toggle changed; App should read current params
    /// via `get_parameters` and restart processing.
    ParametersChanged,
    /// User clicked the Reset button; App should zero the sliders and clear
    /// any cached LC render.
    ResetRequested,
}

pub struct LocalContrastControls {
    pub contrast_slider: Entity<Slider>,
    pub lighten_shadows_slider: Entity<Slider>,
    pub darken_highlights_slider: Entity<Slider>,
    pub font_size_scale: f32,
    pub status: String,

    // --- Advanced toggles (see `local_contrast::Parameters` for semantics) ---
    pub show_advanced: bool,
    pub use_fast_path: bool,
    pub use_median_for_contrast: bool,
    pub use_document_contrast: bool,
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
            show_advanced: false,
            use_fast_path: true,
            use_median_for_contrast: false,
            use_document_contrast: false,
        }
    }

    /// Read the current slider + toggle values into an `lc::Parameters`.
    pub fn get_parameters(&self, cx: &App) -> Parameters {
        Parameters {
            contrast: self.contrast_slider.read(cx).value() as f32,
            lighten_shadows: self.lighten_shadows_slider.read(cx).value() as f32,
            darken_highlights: self.darken_highlights_slider.read(cx).value() as f32,
            use_fast_path: self.use_fast_path,
            use_median_for_contrast: self.use_median_for_contrast,
            use_document_contrast: self.use_document_contrast,
            ..Default::default()
        }
    }

    pub fn reset_sliders(&mut self, cx: &mut Context<Self>) {
        self.contrast_slider
            .update(cx, |s, cx| s.set_value(0.0, cx));
        self.lighten_shadows_slider
            .update(cx, |s, cx| s.set_value(0.0, cx));
        self.darken_highlights_slider
            .update(cx, |s, cx| s.set_value(0.0, cx));
        cx.emit(LocalContrastControlsEvent::ParametersChanged);
    }

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

/// Render a checkbox + label row. Clicking anywhere on the row toggles the
/// field via `toggle_fn` and emits `ParametersChanged`.
fn checkbox_row<F>(
    label: &'static str,
    checked: bool,
    font_scale: f32,
    cx: &mut Context<LocalContrastControls>,
    toggle_fn: F,
) -> impl IntoElement
where
    F: Fn(&mut LocalContrastControls) + 'static,
{
    let box_fill = if checked {
        rgba(0x4A9E_FFFF)
    } else {
        rgba(0x0000_0000)
    };
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _evt, _window, cx| {
                toggle_fn(this);
                cx.emit(LocalContrastControlsEvent::ParametersChanged);
                cx.notify();
            }),
        )
        .child(
            div()
                .w(px(14.0))
                .h(px(14.0))
                .border_1()
                .border_color(rgba(0x88_88_88_FF))
                .rounded(px(2.0))
                .bg(box_fill),
        )
        .child(
            div()
                .text_size(scaled_text_size(12.0, font_scale))
                .text_color(Colors::text())
                .child(label),
        )
}

impl Render for LocalContrastControls {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let contrast_val = self.contrast_slider.read(cx).value();
        let shadows_val = self.lighten_shadows_slider.read(cx).value();
        let highlights_val = self.darken_highlights_slider.read(cx).value();
        let font_scale = self.font_size_scale;

        let advanced_label = if self.show_advanced {
            "▼ Advanced"
        } else {
            "▶ Advanced"
        };

        let advanced_section: Option<Div> = if self.show_advanced {
            Some(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .pl(px(4.0))
                    .child(checkbox_row(
                        "Use Fast Path (integral-image mean)",
                        self.use_fast_path,
                        font_scale,
                        cx,
                        |this| this.use_fast_path = !this.use_fast_path,
                    ))
                    .child(checkbox_row(
                        "Use Median Gray-Point",
                        self.use_median_for_contrast,
                        font_scale,
                        cx,
                        |this| {
                            this.use_median_for_contrast = !this.use_median_for_contrast;
                        },
                    ))
                    .child(checkbox_row(
                        "Document Mode",
                        self.use_document_contrast,
                        font_scale,
                        cx,
                        |this| this.use_document_contrast = !this.use_document_contrast,
                    )),
            )
        } else {
            None
        };

        let advanced_toggle = div()
            .cursor_pointer()
            .text_size(scaled_text_size(12.0, font_scale))
            .text_color(rgb(0xAAAAAA))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _evt, _window, cx| {
                    this.show_advanced = !this.show_advanced;
                    cx.notify();
                }),
            )
            .child(advanced_label);

        let reset_button = div()
            .flex()
            .items_center()
            .justify_center()
            .px(px(12.0))
            .py(px(6.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(rgba(0x66_66_66_FF))
            .cursor_pointer()
            .text_size(scaled_text_size(12.0, font_scale))
            .text_color(Colors::text())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_this, _evt, _window, cx| {
                    cx.emit(LocalContrastControlsEvent::ResetRequested);
                }),
            )
            .child("Reset");

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
                    .child(advanced_toggle)
                    .when_some(advanced_section, |parent, section| parent.child(section))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .mt(Spacing::sm())
                            .child(
                                div()
                                    .text_size(scaled_text_size(12.0, font_scale))
                                    .text_color(rgb(0xAAAAAA))
                                    .child(self.status.clone()),
                            )
                            .child(reset_button),
                    ),
            )
    }
}
