//! Sliders + checkboxes for the Local Contrast dialog.
//!
//! Mirrors the `PView/DspParamsWindow.xaml` parameter surface from the C#
//! original: window/block sizes (with Auto toggles), black/white alphas,
//! contrast with sign, document-mode knobs + sub-toggles, shadow/highlight
//! amounts with sign, and progress/cancel surface.

use crate::utils::local_contrast::Parameters;
use crate::utils::style::{Colors, Spacing, scaled_text_size};
use ccf_gpui_widgets::prelude::{Slider, SliderEvent};
use gpui::prelude::FluentBuilder;
use gpui::*;

#[derive(Clone, Debug)]
pub enum LocalContrastControlsEvent {
    ParametersChanged,
    ResetRequested,
    CancelRequested,
}

/// Five fixed snap-points for the resize-factor toggle.
pub const RESIZE_CHOICES: [f32; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

pub struct LocalContrastControls {
    pub font_size_scale: f32,
    pub status: String,
    /// Progress 0.0..=1.0 while processing, else None (caller controls).
    pub progress: Option<f32>,

    /// Currently-selected value from `RESIZE_CHOICES`.
    pub resize_factor: f32,

    // --- Window / block sizes ------------------------------------------------
    pub cxy_window_auto: bool,
    pub cxy_window_slider: Entity<Slider>,
    pub cxy_block_auto: bool,
    pub cxy_block_slider: Entity<Slider>,

    // --- Alphas --------------------------------------------------------------
    pub alpha_black_slider: Entity<Slider>,
    pub alpha_white_slider: Entity<Slider>,

    // --- Contrast / gray-point -----------------------------------------------
    pub use_median_for_contrast: bool,
    pub contrast_slider: Entity<Slider>,

    // --- Document mode -------------------------------------------------------
    pub use_document_contrast: bool,
    pub mix_document_contrast_slider: Entity<Slider>,
    pub apply_contrast_to_bw: bool,
    pub apply_contrast_to_xition: bool,
    pub tilt_black_slider: Entity<Slider>,
    pub tilt_white_slider: Entity<Slider>,

    // --- Highlights / shadows ------------------------------------------------
    pub darken_highlights_slider: Entity<Slider>,
    pub lighten_shadows_slider: Entity<Slider>,
}

impl EventEmitter<LocalContrastControlsEvent> for LocalContrastControls {}

/// Helper to create a slider + subscribe for ParametersChanged events.
fn make_slider(
    cx: &mut Context<LocalContrastControls>,
    initial: f64,
    min: f64,
    max: f64,
    step: f64,
    precision: usize,
) -> Entity<Slider> {
    let s = cx.new(|cx| {
        Slider::new(cx)
            .with_value(initial)
            .min(min)
            .max(max)
            .step(step)
            .display_precision(precision)
    });
    cx.subscribe(&s, |_this, _slider, event: &SliderEvent, cx| {
        if let SliderEvent::Change(_) = event {
            cx.emit(LocalContrastControlsEvent::ParametersChanged);
        }
    })
    .detach();
    s
}

impl LocalContrastControls {
    pub fn new(font_size_scale: f32, cx: &mut Context<Self>) -> Self {
        let cxy_window_slider = make_slider(cx, 32.0, 1.0, 500.0, 1.0, 0);
        let cxy_block_slider = make_slider(cx, 8.0, 1.0, 200.0, 1.0, 0);
        let alpha_black_slider = make_slider(cx, 0.04, 0.0, 1.0, 0.01, 2);
        let alpha_white_slider = make_slider(cx, 0.04, 0.0, 1.0, 0.01, 2);
        let contrast_slider = make_slider(cx, 0.0, -1.0, 1.0, 0.01, 2);
        let mix_document_contrast_slider = make_slider(cx, 1.0, -1.0, 1.0, 0.01, 2);
        let tilt_black_slider = make_slider(cx, -0.20, -1.0, 1.0, 0.01, 2);
        let tilt_white_slider = make_slider(cx, -0.05, -1.0, 1.0, 0.01, 2);
        let darken_highlights_slider = make_slider(cx, 0.0, -1.0, 1.0, 0.01, 2);
        let lighten_shadows_slider = make_slider(cx, 0.0, -1.0, 1.0, 0.01, 2);

        Self {
            font_size_scale,
            status: String::new(),
            progress: None,
            resize_factor: 1.0,

            cxy_window_auto: true,
            cxy_window_slider,
            cxy_block_auto: true,
            cxy_block_slider,

            alpha_black_slider,
            alpha_white_slider,

            use_median_for_contrast: false,
            contrast_slider,

            use_document_contrast: false,
            mix_document_contrast_slider,
            apply_contrast_to_bw: true,
            apply_contrast_to_xition: true,
            tilt_black_slider,
            tilt_white_slider,

            darken_highlights_slider,
            lighten_shadows_slider,
        }
    }

    pub fn get_parameters(&self, cx: &App) -> Parameters {
        let cxy_window = if self.cxy_window_auto {
            0
        } else {
            self.cxy_window_slider.read(cx).value() as u32
        };
        let cxy_block = if self.cxy_block_auto {
            0
        } else {
            self.cxy_block_slider.read(cx).value() as u32
        };
        Parameters {
            cxy_window,
            cxy_block,
            alpha_black: self.alpha_black_slider.read(cx).value() as f32,
            alpha_white: self.alpha_white_slider.read(cx).value() as f32,
            use_median_for_contrast: self.use_median_for_contrast,
            use_document_contrast: self.use_document_contrast,
            tilt_black_doc_contrast: self.tilt_black_slider.read(cx).value() as f32,
            tilt_white_doc_contrast: self.tilt_white_slider.read(cx).value() as f32,
            apply_contrast_to_bw: self.apply_contrast_to_bw,
            apply_contrast_to_xition: self.apply_contrast_to_xition,
            mix_document_contrast: self.mix_document_contrast_slider.read(cx).value() as f32,
            contrast: self.contrast_slider.read(cx).value() as f32,
            lighten_shadows: self.lighten_shadows_slider.read(cx).value() as f32,
            darken_highlights: self.darken_highlights_slider.read(cx).value() as f32,
            resize_factor: self.resize_factor,
            ..Default::default()
        }
    }

    /// Reset every control to its neutral default (matching `Parameters::default()`
    /// except we pin contrast/shadows/highlights to 0 for a clean "no effect" start).
    pub fn reset_sliders(&mut self, cx: &mut Context<Self>) {
        let defaults = Parameters::default();
        self.resize_factor = 1.0;
        self.cxy_window_auto = true;
        self.cxy_block_auto = true;
        self.cxy_window_slider
            .update(cx, |s, cx| s.set_value(32.0, cx));
        self.cxy_block_slider
            .update(cx, |s, cx| s.set_value(8.0, cx));
        self.alpha_black_slider
            .update(cx, |s, cx| s.set_value(defaults.alpha_black as f64, cx));
        self.alpha_white_slider
            .update(cx, |s, cx| s.set_value(defaults.alpha_white as f64, cx));
        self.use_median_for_contrast = false;
        self.contrast_slider
            .update(cx, |s, cx| s.set_value(0.0, cx));
        self.use_document_contrast = false;
        self.mix_document_contrast_slider.update(cx, |s, cx| {
            s.set_value(defaults.mix_document_contrast as f64, cx)
        });
        self.apply_contrast_to_bw = true;
        self.apply_contrast_to_xition = true;
        self.tilt_black_slider.update(cx, |s, cx| {
            s.set_value(defaults.tilt_black_doc_contrast as f64, cx)
        });
        self.tilt_white_slider.update(cx, |s, cx| {
            s.set_value(defaults.tilt_white_doc_contrast as f64, cx)
        });
        self.darken_highlights_slider
            .update(cx, |s, cx| s.set_value(0.0, cx));
        self.lighten_shadows_slider
            .update(cx, |s, cx| s.set_value(0.0, cx));
        cx.emit(LocalContrastControlsEvent::ParametersChanged);
    }

    pub fn set_status(&mut self, status: impl Into<String>, cx: &mut Context<Self>) {
        self.status = status.into();
        cx.notify();
    }

    pub fn set_progress(&mut self, progress: Option<f32>, cx: &mut Context<Self>) {
        self.progress = progress;
        cx.notify();
    }
}

// ---------------------------------------------------------------------------
// Render helpers
// ---------------------------------------------------------------------------

fn section_separator() -> Div {
    div().h(px(1.0)).bg(rgba(0x44_44_44_FF)).mt(px(4.0))
}

fn labeled_slider(
    title: String,
    value_text: String,
    slider: Entity<Slider>,
    font_scale: f32,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .flex()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_size(scaled_text_size(11.0, font_scale))
                        .text_color(Colors::text())
                        .child(title),
                )
                .child(
                    div()
                        .text_size(scaled_text_size(11.0, font_scale))
                        .text_color(rgb(0xAAAAAA))
                        .font_weight(FontWeight::BOLD)
                        .child(value_text),
                ),
        )
        .child(slider)
}

fn checkbox_row<F>(
    label: String,
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
                .w(px(13.0))
                .h(px(13.0))
                .border_1()
                .border_color(rgba(0x88_88_88_FF))
                .rounded(px(2.0))
                .bg(box_fill),
        )
        .child(
            div()
                .text_size(scaled_text_size(11.0, font_scale))
                .text_color(Colors::text())
                .child(label),
        )
}

/// 5-way toggle row for the resize factor. Each cell is clickable; the
/// currently-selected factor is highlighted. Click sets `resize_factor` and
/// emits `ParametersChanged`.
fn resize_toggle_row(
    current: f32,
    font_scale: f32,
    cx: &mut Context<LocalContrastControls>,
) -> impl IntoElement {
    let labels = ["¼×", "½×", "1×", "2×", "4×"];
    let mut row = div().flex().gap(px(4.0));
    for (i, factor) in RESIZE_CHOICES.iter().copied().enumerate() {
        let selected = (current - factor).abs() < 1e-4;
        let bg = if selected {
            rgba(0x4A9E_FFFF)
        } else {
            rgba(0x2A_2A_2A_FF)
        };
        let fg = if selected {
            Colors::text()
        } else {
            rgb(0xCCCCCC).into()
        };
        let cell = div()
            .flex()
            .items_center()
            .justify_center()
            .flex_grow()
            .py(px(4.0))
            .rounded(px(3.0))
            .border_1()
            .border_color(rgba(0x55_55_55_FF))
            .bg(bg)
            .cursor_pointer()
            .text_size(scaled_text_size(11.0, font_scale))
            .text_color(fg)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _evt, _window, cx| {
                    if (this.resize_factor - factor).abs() > 1e-4 {
                        this.resize_factor = factor;
                        cx.emit(LocalContrastControlsEvent::ParametersChanged);
                        cx.notify();
                    }
                }),
            )
            .child(labels[i]);
        row = row.child(cell);
    }
    row
}

fn progress_bar(fraction: f32) -> impl IntoElement {
    let clamped = fraction.clamp(0.0, 1.0);
    div()
        .h(px(8.0))
        .w_full()
        .bg(rgba(0x2A_2A_2A_FF))
        .rounded(px(2.0))
        .border_1()
        .border_color(rgba(0x55_55_55_FF))
        .child(
            div()
                .h_full()
                .w(relative(clamped))
                .bg(rgba(0x4A9E_FFFF))
                .rounded(px(2.0)),
        )
}

impl Render for LocalContrastControls {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let font = self.font_size_scale;

        // Read slider values for display.
        let cxy_window_val = self.cxy_window_slider.read(cx).value();
        let cxy_block_val = self.cxy_block_slider.read(cx).value();
        let alpha_black_val = self.alpha_black_slider.read(cx).value();
        let alpha_white_val = self.alpha_white_slider.read(cx).value();
        let contrast_val = self.contrast_slider.read(cx).value();
        let mix_doc_val = self.mix_document_contrast_slider.read(cx).value();
        let tilt_black_val = self.tilt_black_slider.read(cx).value();
        let tilt_white_val = self.tilt_white_slider.read(cx).value();
        let darken_val = self.darken_highlights_slider.read(cx).value();
        let lighten_val = self.lighten_shadows_slider.read(cx).value();

        let doc_mode = self.use_document_contrast;
        let processing = self.progress.is_some();

        // Cancel / Reset buttons.
        let cancel_button = div()
            .flex()
            .items_center()
            .justify_center()
            .px(px(10.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(rgba(0x66_66_66_FF))
            .cursor_pointer()
            .text_size(scaled_text_size(10.0, font))
            .text_color(Colors::text())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_this, _evt, _window, cx| {
                    cx.emit(LocalContrastControlsEvent::CancelRequested);
                }),
            )
            .child("Cancel");

        let reset_button = div()
            .flex()
            .items_center()
            .justify_center()
            .px(px(12.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(rgba(0x66_66_66_FF))
            .cursor_pointer()
            .text_size(scaled_text_size(11.0, font))
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
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(scaled_text_size(15.0, font))
                            .text_color(Colors::text())
                            .font_weight(FontWeight::BOLD)
                            .child("Local Contrast"),
                    )
                    .child(section_separator())
                    // Resize toggle (5-way)
                    .child(
                        div()
                            .text_size(scaled_text_size(11.0, font))
                            .text_color(Colors::text())
                            .child("Resize input"),
                    )
                    .child(resize_toggle_row(self.resize_factor, font, cx))
                    .child(section_separator())
                    // Window size
                    .child(checkbox_row(
                        "Auto window size".to_string(),
                        self.cxy_window_auto,
                        font,
                        cx,
                        |this| this.cxy_window_auto = !this.cxy_window_auto,
                    ))
                    .when(!self.cxy_window_auto, |el| {
                        el.child(labeled_slider(
                            "Window Size".to_string(),
                            format!("{:.0} px", cxy_window_val),
                            self.cxy_window_slider.clone(),
                            font,
                        ))
                    })
                    // Block size
                    .child(checkbox_row(
                        "Auto block size".to_string(),
                        self.cxy_block_auto,
                        font,
                        cx,
                        |this| this.cxy_block_auto = !this.cxy_block_auto,
                    ))
                    .when(!self.cxy_block_auto, |el| {
                        el.child(labeled_slider(
                            "Block Size".to_string(),
                            format!("every {:.0} px", cxy_block_val),
                            self.cxy_block_slider.clone(),
                            font,
                        ))
                    })
                    .child(section_separator())
                    // Alphas
                    .child(labeled_slider(
                        "Alpha of pure Black".to_string(),
                        format!("{:.0}%", alpha_black_val * 100.0),
                        self.alpha_black_slider.clone(),
                        font,
                    ))
                    .child(labeled_slider(
                        "Alpha of pure White".to_string(),
                        format!("{:.0}%", alpha_white_val * 100.0),
                        self.alpha_white_slider.clone(),
                        font,
                    ))
                    .child(section_separator())
                    // Contrast block
                    .child(checkbox_row(
                        "Use Median Gray-Point".to_string(),
                        self.use_median_for_contrast,
                        font,
                        cx,
                        |this| {
                            this.use_median_for_contrast = !this.use_median_for_contrast;
                        },
                    ))
                    .child(labeled_slider(
                        "Contrast".to_string(),
                        format!("{:+.0}%", contrast_val * 100.0),
                        self.contrast_slider.clone(),
                        font,
                    ))
                    // Document mode
                    .child(checkbox_row(
                        "Document-Style Contrast".to_string(),
                        self.use_document_contrast,
                        font,
                        cx,
                        |this| this.use_document_contrast = !this.use_document_contrast,
                    ))
                    .when(doc_mode, |el| {
                        el.child(labeled_slider(
                            "Mix Doc Contrast".to_string(),
                            format!("{:+.0}%", mix_doc_val * 100.0),
                            self.mix_document_contrast_slider.clone(),
                            font,
                        ))
                        .child(
                            div()
                                .flex()
                                .gap(px(12.0))
                                .child(checkbox_row(
                                    "Adjust BW".to_string(),
                                    self.apply_contrast_to_bw,
                                    font,
                                    cx,
                                    |this| {
                                        this.apply_contrast_to_bw = !this.apply_contrast_to_bw;
                                    },
                                ))
                                .child(checkbox_row(
                                    "Adjust Gray".to_string(),
                                    self.apply_contrast_to_xition,
                                    font,
                                    cx,
                                    |this| {
                                        this.apply_contrast_to_xition =
                                            !this.apply_contrast_to_xition;
                                    },
                                )),
                        )
                        .child(labeled_slider(
                            "Tilt Black-Point".to_string(),
                            format!("{:+.0}%", tilt_black_val * 100.0),
                            self.tilt_black_slider.clone(),
                            font,
                        ))
                        .child(labeled_slider(
                            "Tilt White-Point".to_string(),
                            format!("{:+.0}%", tilt_white_val * 100.0),
                            self.tilt_white_slider.clone(),
                            font,
                        ))
                    })
                    // Shadows / highlights
                    .child(labeled_slider(
                        "Darken Highlights".to_string(),
                        format!("{:+.0}%", darken_val * 100.0),
                        self.darken_highlights_slider.clone(),
                        font,
                    ))
                    .child(labeled_slider(
                        "Lighten Shadows".to_string(),
                        format!("{:+.0}%", lighten_val * 100.0),
                        self.lighten_shadows_slider.clone(),
                        font,
                    ))
                    .child(section_separator())
                    // Status + progress + controls
                    .child(
                        div()
                            .text_size(scaled_text_size(11.0, font))
                            .text_color(rgb(0xAAAAAA))
                            .child(self.status.clone()),
                    )
                    .when(processing, |el| {
                        let frac = self.progress.unwrap_or(0.0);
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(div().flex_grow().child(progress_bar(frac)))
                                .child(cancel_button),
                        )
                    })
                    .child(div().flex().justify_end().mt(px(4.0)).child(reset_button)),
            )
    }
}
