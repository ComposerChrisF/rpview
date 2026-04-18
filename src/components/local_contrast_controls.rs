//! Sliders + checkboxes for the Local Contrast dialog.
//!
//! Mirrors the `PView/DspParamsWindow.xaml` parameter surface from the C#
//! original: window/block sizes (with Auto toggles), black/white alphas,
//! contrast with sign, document-mode knobs + sub-toggles, shadow/highlight
//! amounts with sign, and progress/cancel surface.

use crate::utils::lc_presets;
use crate::utils::local_contrast::Parameters;
use crate::utils::style::{Colors, Spacing, scaled_text_size};
use ccf_gpui_widgets::prelude::{
    Dropdown, DropdownEvent, Slider, SliderEvent, TextInput, TextInputEvent,
};
use gpui::prelude::FluentBuilder;
use gpui::*;

#[derive(Clone, Debug)]
pub enum LocalContrastControlsEvent {
    ParametersChanged,
    ResetRequested,
    CancelRequested,
    /// User clicked "Process All Frames" for animated images.
    ProcessAllFramesRequested,
}

/// Five fixed snap-points for the resize-factor toggle.
pub const RESIZE_CHOICES: [f32; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

pub struct LocalContrastControls {
    pub font_size_scale: f32,
    pub status: String,
    /// Progress 0.0..=1.0 while processing, else None (caller controls).
    pub progress: Option<f32>,

    /// Currently-selected value from `RESIZE_CHOICES` (ignored when `resize_auto` is true).
    pub resize_factor: f32,
    /// When true, resize factor is auto-computed to target ~4K on the largest axis.
    pub resize_auto: bool,
    /// Current image dimensions, set by the app when the image changes.
    /// Used to compute the auto resize factor.
    pub image_dimensions: Option<(u32, u32)>,
    /// When false, slider changes don't trigger processing and the viewer
    /// shows the unprocessed image. Flipping back to true re-triggers.
    pub preview_enabled: bool,

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

    // --- Presets ---------------------------------------------------------------
    pub preset_dropdown: Entity<Dropdown>,
    pub preset_name_input: Entity<TextInput>,
    /// The currently-loaded preset name (None = custom / unsaved).
    pub current_preset: Option<String>,

    // --- Animation batch processing -------------------------------------------
    /// Whether the current image is animated (controls visibility of batch UI).
    pub is_animated: bool,
    /// Batch processing progress: `(current_frame_0based, total_frames)`.
    /// `None` when no batch is running.
    pub batch_progress: Option<(usize, usize)>,
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

        let preset_dropdown = Self::build_preset_dropdown(None, cx);
        cx.subscribe(&preset_dropdown, Self::on_preset_dropdown_change)
            .detach();

        let preset_name_input = cx.new(|cx| TextInput::new(cx).placeholder("Preset name…"));
        cx.subscribe(
            &preset_name_input,
            |this, input, event: &TextInputEvent, cx| {
                if let TextInputEvent::Enter = event {
                    let name = input.read(cx).content().to_string();
                    this.save_current_as_preset(&name, cx);
                }
            },
        )
        .detach();

        Self {
            font_size_scale,
            status: String::new(),
            progress: None,
            resize_factor: 1.0,
            resize_auto: false,
            image_dimensions: None,
            preview_enabled: false,

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

            preset_dropdown,
            preset_name_input,
            current_preset: None,

            is_animated: false,
            batch_progress: None,
        }
    }

    /// Update the current image dimensions (used for auto resize factor).
    pub fn set_image_dimensions(&mut self, dims: Option<(u32, u32)>) {
        self.image_dimensions = dims;
    }

    /// Compute the effective resize factor. In auto mode, picks the factor
    /// from `RESIZE_CHOICES` (0.25x–4x) that brings the largest axis as
    /// close to 4K (3840px) as possible without exceeding it.
    pub fn effective_resize_factor(&self) -> f32 {
        if !self.resize_auto {
            return self.resize_factor;
        }
        let Some((w, h)) = self.image_dimensions else {
            return 1.0;
        };
        let largest = w.max(h) as f32;
        if largest < 1.0 {
            return 1.0;
        }
        let target = 3840.0_f32;
        let ideal = target / largest;
        // Pick the largest RESIZE_CHOICES value that doesn't exceed 4K,
        // falling back to the smallest if even 0.25x exceeds 4K.
        let mut best = RESIZE_CHOICES[0];
        for &factor in &RESIZE_CHOICES {
            if largest * factor <= target + 0.5 {
                best = factor;
            }
        }
        // If ideal is larger than best but still <=4x, we could use best.
        // But we only allow the fixed snap-points.
        let _ = ideal; // used only for the design doc
        best
    }

    /// Set whether the current image is animated (controls batch UI visibility).
    pub fn set_is_animated(&mut self, animated: bool, cx: &mut Context<Self>) {
        self.is_animated = animated;
        if !animated {
            self.batch_progress = None;
        }
        cx.notify();
    }

    /// Update batch processing progress. `None` = not running.
    pub fn set_batch_progress(
        &mut self,
        progress: Option<(usize, usize)>,
        cx: &mut Context<Self>,
    ) {
        self.batch_progress = progress;
        cx.notify();
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
            resize_factor: self.effective_resize_factor(),
            ..Default::default()
        }
    }

    /// Reset every control to its neutral default (matching `Parameters::default()`
    /// except we pin contrast/shadows/highlights to 0 for a clean "no effect" start).
    pub fn reset_sliders(&mut self, cx: &mut Context<Self>) {
        let defaults = Parameters::default();
        self.resize_factor = 1.0;
        self.resize_auto = false;
        self.preview_enabled = false;
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

    // --- Preset support --------------------------------------------------------

    fn build_preset_dropdown(selected: Option<&str>, cx: &mut Context<Self>) -> Entity<Dropdown> {
        let names = lc_presets::list_preset_names();
        let mut choices = vec![lc_presets::CUSTOM_PRESET_LABEL.to_string()];
        choices.extend(names);
        cx.new(|cx| {
            let mut d = Dropdown::new(cx).choices(choices);
            if let Some(name) = selected {
                d = d.with_selected_value(name);
            }
            d
        })
    }

    fn on_preset_dropdown_change(
        &mut self,
        _dd: Entity<Dropdown>,
        event: &DropdownEvent,
        cx: &mut Context<Self>,
    ) {
        if let DropdownEvent::Change(name) = event {
            if name == lc_presets::CUSTOM_PRESET_LABEL {
                self.current_preset = None;
                return;
            }
            if let Some(params) = lc_presets::load_preset(name) {
                self.apply_parameters(&params, cx);
                self.current_preset = Some(name.clone());
                cx.emit(LocalContrastControlsEvent::ParametersChanged);
            }
        }
    }

    fn apply_parameters(&mut self, p: &Parameters, cx: &mut Context<Self>) {
        self.resize_factor = p.resize_factor;
        self.resize_auto = false;
        self.cxy_window_auto = p.cxy_window == 0;
        if !self.cxy_window_auto {
            self.cxy_window_slider
                .update(cx, |s, cx| s.set_value(p.cxy_window as f64, cx));
        }
        self.cxy_block_auto = p.cxy_block == 0;
        if !self.cxy_block_auto {
            self.cxy_block_slider
                .update(cx, |s, cx| s.set_value(p.cxy_block as f64, cx));
        }
        self.alpha_black_slider
            .update(cx, |s, cx| s.set_value(p.alpha_black as f64, cx));
        self.alpha_white_slider
            .update(cx, |s, cx| s.set_value(p.alpha_white as f64, cx));
        self.use_median_for_contrast = p.use_median_for_contrast;
        self.contrast_slider
            .update(cx, |s, cx| s.set_value(p.contrast as f64, cx));
        self.use_document_contrast = p.use_document_contrast;
        self.mix_document_contrast_slider
            .update(cx, |s, cx| s.set_value(p.mix_document_contrast as f64, cx));
        self.apply_contrast_to_bw = p.apply_contrast_to_bw;
        self.apply_contrast_to_xition = p.apply_contrast_to_xition;
        self.tilt_black_slider.update(cx, |s, cx| {
            s.set_value(p.tilt_black_doc_contrast as f64, cx)
        });
        self.tilt_white_slider.update(cx, |s, cx| {
            s.set_value(p.tilt_white_doc_contrast as f64, cx)
        });
        self.darken_highlights_slider
            .update(cx, |s, cx| s.set_value(p.darken_highlights as f64, cx));
        self.lighten_shadows_slider
            .update(cx, |s, cx| s.set_value(p.lighten_shadows as f64, cx));
    }

    fn save_current_as_preset(&mut self, name: &str, cx: &mut Context<Self>) {
        let name = name.trim();
        if name.is_empty() || name == lc_presets::CUSTOM_PRESET_LABEL {
            return;
        }
        let params = self.get_parameters(cx);
        if let Err(e) = lc_presets::save_preset(name, &params) {
            eprintln!("Failed to save preset: {}", e);
            return;
        }
        self.current_preset = Some(name.to_string());
        self.rebuild_preset_dropdown(cx);
    }

    fn delete_current_preset(&mut self, cx: &mut Context<Self>) {
        let Some(name) = self.current_preset.take() else {
            return;
        };
        let _ = lc_presets::delete_preset(&name);
        self.rebuild_preset_dropdown(cx);
    }

    fn rebuild_preset_dropdown(&mut self, cx: &mut Context<Self>) {
        let selected = self.current_preset.as_deref();
        self.preset_dropdown = Self::build_preset_dropdown(selected, cx);
        cx.subscribe(&self.preset_dropdown, Self::on_preset_dropdown_change)
            .detach();
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

/// 6-way toggle row for the resize factor: 5 fixed snap-points plus Auto.
/// Each cell is clickable; the currently-selected factor (or Auto) is
/// highlighted. Click sets `resize_factor` / `resize_auto` and emits
/// `ParametersChanged`.
fn resize_toggle_row(
    current: f32,
    is_auto: bool,
    font_scale: f32,
    cx: &mut Context<LocalContrastControls>,
) -> impl IntoElement {
    let labels = ["¼×", "½×", "1×", "2×", "4×"];
    let mut row = div().flex().gap(px(4.0));
    for (i, factor) in RESIZE_CHOICES.iter().copied().enumerate() {
        let selected = !is_auto && (current - factor).abs() < 1e-4;
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
                    if this.resize_auto || (this.resize_factor - factor).abs() > 1e-4 {
                        this.resize_auto = false;
                        this.resize_factor = factor;
                        cx.emit(LocalContrastControlsEvent::ParametersChanged);
                        cx.notify();
                    }
                }),
            )
            .child(labels[i]);
        row = row.child(cell);
    }
    // Auto button
    let auto_bg = if is_auto {
        rgba(0x4A9E_FFFF)
    } else {
        rgba(0x2A_2A_2A_FF)
    };
    let auto_fg = if is_auto {
        Colors::text()
    } else {
        rgb(0xCCCCCC).into()
    };
    let auto_cell = div()
        .flex()
        .items_center()
        .justify_center()
        .flex_grow()
        .py(px(4.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(rgba(0x55_55_55_FF))
        .bg(auto_bg)
        .cursor_pointer()
        .text_size(scaled_text_size(11.0, font_scale))
        .text_color(auto_fg)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _evt, _window, cx| {
                if !this.resize_auto {
                    this.resize_auto = true;
                    cx.emit(LocalContrastControlsEvent::ParametersChanged);
                    cx.notify();
                }
            }),
        )
        .child("Auto");
    row = row.child(auto_cell);
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
                    // Presets: dropdown on its own line; name input + Save/Del on the next.
                    .child(self.preset_dropdown.clone())
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .items_center()
                            .child(div().flex_grow().child(self.preset_name_input.clone()))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(3.0))
                                    .border_1()
                                    .border_color(rgba(0x55_55_55_FF))
                                    .cursor_pointer()
                                    .text_size(scaled_text_size(10.0, font))
                                    .text_color(Colors::text())
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _evt, _window, cx| {
                                            let name = this
                                                .preset_name_input
                                                .read(cx)
                                                .content()
                                                .to_string();
                                            this.save_current_as_preset(&name, cx);
                                        }),
                                    )
                                    .child("Save"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(3.0))
                                    .border_1()
                                    .border_color(rgba(0x55_55_55_FF))
                                    .cursor_pointer()
                                    .text_size(scaled_text_size(10.0, font))
                                    .text_color(Colors::text())
                                    .when(self.current_preset.is_some(), |el| {
                                        el.on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _evt, _window, cx| {
                                                this.delete_current_preset(cx);
                                            }),
                                        )
                                    })
                                    .when(self.current_preset.is_none(), |el| {
                                        el.text_color(rgb(0x666666))
                                    })
                                    .child("Del"),
                            ),
                    )
                    .child(section_separator())
                    // Resize toggle (5-way)
                    .child(
                        div()
                            .text_size(scaled_text_size(11.0, font))
                            .text_color(Colors::text())
                            .child("Resize input"),
                    )
                    .child(resize_toggle_row(self.resize_factor, self.resize_auto, font, cx))
                    .child(checkbox_row(
                        "Preview".to_string(),
                        self.preview_enabled,
                        font,
                        cx,
                        |this| this.preview_enabled = !this.preview_enabled,
                    ))
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
                    // Batch processing UI for animated images.
                    .when(self.is_animated, |el| {
                        let batch_running = self.batch_progress.is_some();
                        let batch_label = if let Some((current, total)) = self.batch_progress {
                            format!("Processing frame {}/{}…", current + 1, total)
                        } else {
                            String::new()
                        };
                        el.child(section_separator())
                            .when(batch_running, |el| {
                                let (current, total) = self.batch_progress.unwrap();
                                let frac = if total > 0 {
                                    (current as f32 + 1.0) / total as f32
                                } else {
                                    0.0
                                };
                                el.child(
                                    div()
                                        .text_size(scaled_text_size(11.0, font))
                                        .text_color(rgb(0xAAAAAA))
                                        .child(batch_label),
                                )
                                .child(progress_bar(frac))
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .mt(px(4.0))
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
                                            if _this.batch_progress.is_some() {
                                                cx.emit(
                                                    LocalContrastControlsEvent::CancelRequested,
                                                );
                                            } else {
                                                cx.emit(
                                                    LocalContrastControlsEvent::ProcessAllFramesRequested,
                                                );
                                            }
                                        }),
                                    )
                                    .child(if batch_running {
                                        "Cancel Batch"
                                    } else {
                                        "Process All Frames"
                                    }),
                            )
                    })
                    .child(div().flex().justify_end().mt(px(4.0)).child(reset_button)),
            )
    }
}
