//! Sliders + enable checkboxes for the unified GPU pixel-shader pipeline.
//!
//! Single window — five sections (Resize, LC, BC, Vibrance, Hue), each with
//! an enable checkbox and a collapse toggle (Resize is global, no enable).
//! Disabled stages are skipped entirely in the pipeline; collapsed sections
//! just hide their sliders.
//!
//! Events:
//!   * `ParametersChanged` fires on every slider tick, on enable/collapse
//!     toggle, and on resize-button click.  The parent app rebuilds
//!     [`crate::gpu::UnifiedParams`] via [`GpuPipelineControls::get_params`]
//!     and feeds the viewer.
//!   * `ResetRequested` clears every stage to its default and disables it.

use crate::gpu::{BcParams, EqualizeParams, HueParams, LcParams, UnifiedParams, VibranceParams};
use crate::utils::gpu_presets::{self, GpuPreset};
use crate::utils::style::{Colors, Spacing, scaled_text_size};
use ccf_gpui_widgets::prelude::{
    Dropdown, DropdownEvent, Slider, SliderEvent, TextInput, TextInputEvent, scrollable_vertical,
};
use gpui::prelude::FluentBuilder;
use gpui::*;

#[derive(Clone, Debug)]
pub enum GpuPipelineControlsEvent {
    ParametersChanged,
    ResetRequested,
}

/// Discrete resize choices the user can pick from explicit buttons.  These
/// match the snap-points in `local_contrast_controls::RESIZE_CHOICES` so
/// muscle memory carries between the two panels.
pub const RESIZE_CHOICES: [f32; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

/// When `resize_auto` is on, the effective factor is computed so the longer
/// image dimension lands as close as possible to this target without
/// exceeding it.  4096 ≈ "4K-ish" — comfortably within GPU memory budgets
/// while still preserving real detail.
const AUTO_TARGET_PX: u32 = 4096;

/// LC Radius slider bounds — _in source-image pixels_, before the
/// resize-factor multiplier is applied.  The slider position itself is
/// normalized to 0..1 and mapped logarithmically across this range so a
/// drag of 5 % near the bottom shifts the radius by a few pixels while a
/// 5 % drag near the top shifts it by tens.
const MIN_DISPLAYED_RADIUS: f32 = 4.0;
const MAX_DISPLAYED_RADIUS: f32 = 1000.0;
const DEFAULT_DISPLAYED_RADIUS: f32 = 60.0;

/// `t ∈ [0, 1]` → radius (px).  Logarithmic.
fn radius_t_to_displayed(t: f32) -> f32 {
    let ln_min = MIN_DISPLAYED_RADIUS.ln();
    let ln_max = MAX_DISPLAYED_RADIUS.ln();
    (ln_min + t * (ln_max - ln_min)).exp()
}

/// Inverse of `radius_t_to_displayed`.  Used to seed the slider from a
/// desired radius value.
fn radius_displayed_to_t(displayed: f32) -> f32 {
    let ln_min = MIN_DISPLAYED_RADIUS.ln();
    let ln_max = MAX_DISPLAYED_RADIUS.ln();
    let clamped = displayed.clamp(MIN_DISPLAYED_RADIUS, MAX_DISPLAYED_RADIUS);
    (clamped.ln() - ln_min) / (ln_max - ln_min)
}

pub struct GpuPipelineControls {
    pub font_size_scale: f32,

    /// Vertical scroll state for the panel — kept across renders so the
    /// scroll position survives slider changes (which trigger re-renders).
    pub scroll_handle: ScrollHandle,

    /// Currently-selected discrete factor (ignored when `resize_auto` is on).
    pub resize_factor: f32,
    /// When true, the effective factor is computed from `image_dimensions`.
    pub resize_auto: bool,
    /// Current image dimensions, set by the app when an image loads.  Only
    /// consulted in auto mode.
    pub image_dimensions: Option<(u32, u32)>,

    pub lc_enabled: bool,
    pub lc_expanded: bool,
    pub lc_radius: Entity<Slider>,
    pub lc_strength: Entity<Slider>,
    pub lc_shadows: Entity<Slider>,
    pub lc_highlights: Entity<Slider>,
    pub lc_midpoint: Entity<Slider>,

    pub bc_enabled: bool,
    pub bc_expanded: bool,
    pub bc_brightness: Entity<Slider>,
    pub bc_contrast: Entity<Slider>,

    pub vibrance_enabled: bool,
    pub vibrance_expanded: bool,
    pub vibrance_amount: Entity<Slider>,
    pub vibrance_saturation: Entity<Slider>,

    pub hue_enabled: bool,
    pub hue_expanded: bool,
    pub hue_value: Entity<Slider>,

    pub equalize_enabled: bool,
    pub equalize_expanded: bool,
    pub equalize_amount: Entity<Slider>,
    pub equalize_shadows: Entity<Slider>,
    pub equalize_highlights: Entity<Slider>,

    // --- Presets ---
    pub preset_dropdown: Entity<Dropdown>,
    pub preset_name_input: Entity<TextInput>,
    /// Currently-loaded preset name (None = custom / unsaved).  Drives
    /// the dropdown's selected value and gates the Del button.
    pub current_preset: Option<String>,
}

impl EventEmitter<GpuPipelineControlsEvent> for GpuPipelineControls {}

fn slider(
    cx: &mut Context<GpuPipelineControls>,
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
            cx.emit(GpuPipelineControlsEvent::ParametersChanged);
        }
    })
    .detach();
    s
}

impl GpuPipelineControls {
    pub fn new(font_size_scale: f32, cx: &mut Context<Self>) -> Self {
        // Radius slider position is a normalized 0..1 "knob position";
        // get_params maps it logarithmically to a 4..=1000 px radius.
        let lc_radius = slider(
            cx,
            radius_displayed_to_t(DEFAULT_DISPLAYED_RADIUS) as f64,
            0.0,
            1.0,
            0.001,
            3,
        );
        let lc_strength = slider(cx, 0.5, 0.0, 2.0, 0.01, 2);
        let lc_shadows = slider(cx, 0.0, 0.0, 1.0, 0.01, 2);
        let lc_highlights = slider(cx, 0.0, 0.0, 1.0, 0.01, 2);
        let lc_midpoint = slider(cx, 0.5, 0.1, 0.9, 0.01, 2);

        let bc_brightness = slider(cx, 0.0, -1.0, 1.0, 0.01, 2);
        let bc_contrast = slider(cx, 0.0, -1.0, 2.0, 0.01, 2);

        let vibrance_amount = slider(cx, 0.5, -1.0, 1.0, 0.01, 2);
        let vibrance_saturation = slider(cx, 0.0, -1.0, 1.0, 0.01, 2);

        // Hue slider centered on 0.5 — that's "no rotation."  0.0 = −180°,
        // 1.0 = +180°.  get_params subtracts 0.5 to produce shader turns.
        let hue_value = slider(cx, 0.5, 0.0, 1.0, 0.001, 3);

        let equalize_amount = slider(cx, 0.5, 0.0, 1.0, 0.01, 2);
        let equalize_shadows = slider(cx, 0.0, 0.0, 1.0, 0.01, 2);
        let equalize_highlights = slider(cx, 0.0, 0.0, 1.0, 0.01, 2);

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
            scroll_handle: ScrollHandle::new(),
            resize_factor: 1.0,
            resize_auto: false,
            image_dimensions: None,
            lc_enabled: false,
            lc_expanded: true,
            lc_radius,
            lc_strength,
            lc_shadows,
            lc_highlights,
            lc_midpoint,
            bc_enabled: false,
            bc_expanded: true,
            bc_brightness,
            bc_contrast,
            vibrance_enabled: false,
            vibrance_expanded: true,
            vibrance_amount,
            vibrance_saturation,
            hue_enabled: false,
            hue_expanded: true,
            hue_value,
            equalize_enabled: false,
            equalize_expanded: true,
            equalize_amount,
            equalize_shadows,
            equalize_highlights,
            preset_dropdown,
            preset_name_input,
            current_preset: None,
        }
    }

    /// Update the cached image dimensions (used by Auto resize).  The app
    /// pushes this whenever the current image changes; we just store it.
    pub fn set_image_dimensions(&mut self, dims: Option<(u32, u32)>) {
        if self.image_dimensions != dims {
            self.image_dimensions = dims;
        }
    }

    /// Effective resize factor: in auto mode, the largest discrete choice
    /// from `RESIZE_CHOICES` that doesn't exceed `AUTO_TARGET_PX` on the
    /// longer dimension; otherwise `self.resize_factor` directly.  When auto
    /// mode is on but image dimensions are unknown, falls back to 1.0.
    pub fn effective_resize_factor(&self) -> f32 {
        if !self.resize_auto {
            return self.resize_factor;
        }
        let Some((w, h)) = self.image_dimensions else {
            return 1.0;
        };
        let longer = w.max(h) as f32;
        let target = AUTO_TARGET_PX as f32;
        // Walk choices descending; pick the largest that keeps longer*factor ≤ target.
        let mut best = 0.25_f32;
        for &f in RESIZE_CHOICES.iter() {
            if longer * f <= target {
                best = best.max(f);
            }
        }
        best
    }

    /// `true` when picking `factor` for the current image would produce
    /// dimensions that exceed the GPU's `max_texture_dimension_2d`.  The
    /// `process_pipeline` guard already turns this into a graceful no-op
    /// rather than a crash, but the UI also gates the offending buttons
    /// so the user gets clear "this won't work" feedback instead of a
    /// silent click.  Returns `false` (button enabled) when image
    /// dimensions or the GPU context aren't yet available — the worker's
    /// guard remains the safety net.
    pub fn factor_exceeds_gpu_limits(&self, factor: f32) -> bool {
        let Some((w, h)) = self.image_dimensions else {
            return false;
        };
        let Some(ctx) = crate::gpu::device::get_context() else {
            return false;
        };
        let max_dim = ctx.device.limits().max_texture_dimension_2d;
        let out_w = ((w as f32) * factor).round().max(1.0) as u32;
        let out_h = ((h as f32) * factor).round().max(1.0) as u32;
        out_w > max_dim || out_h > max_dim || w > max_dim || h > max_dim
    }

    // --- Preset support -----------------------------------------------------

    fn build_preset_dropdown(selected: Option<&str>, cx: &mut Context<Self>) -> Entity<Dropdown> {
        let names = gpu_presets::list_preset_names();
        let mut choices = vec![gpu_presets::CUSTOM_PRESET_LABEL.to_string()];
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
            if name == gpu_presets::CUSTOM_PRESET_LABEL {
                self.current_preset = None;
                return;
            }
            if let Some(preset) = gpu_presets::load_preset(name) {
                self.apply_preset(&preset, cx);
                self.current_preset = Some(name.clone());
                cx.emit(GpuPipelineControlsEvent::ParametersChanged);
            }
        }
    }

    /// Snapshot the full panel state into a `GpuPreset`, suitable for
    /// JSON serialisation.
    fn to_preset(&self, cx: &App) -> GpuPreset {
        GpuPreset {
            resize_factor: self.resize_factor,
            lc_enabled: self.lc_enabled,
            lc_radius_t: self.lc_radius.read(cx).value() as f32,
            lc_strength: self.lc_strength.read(cx).value() as f32,
            lc_shadows: self.lc_shadows.read(cx).value() as f32,
            lc_highlights: self.lc_highlights.read(cx).value() as f32,
            lc_midpoint: self.lc_midpoint.read(cx).value() as f32,
            bc_enabled: self.bc_enabled,
            bc_brightness: self.bc_brightness.read(cx).value() as f32,
            bc_contrast: self.bc_contrast.read(cx).value() as f32,
            vibrance_enabled: self.vibrance_enabled,
            vibrance_amount: self.vibrance_amount.read(cx).value() as f32,
            vibrance_saturation: self.vibrance_saturation.read(cx).value() as f32,
            hue_enabled: self.hue_enabled,
            hue_value: self.hue_value.read(cx).value() as f32,
            equalize_enabled: self.equalize_enabled,
            equalize_amount: self.equalize_amount.read(cx).value() as f32,
            equalize_shadows: self.equalize_shadows.read(cx).value() as f32,
            equalize_highlights: self.equalize_highlights.read(cx).value() as f32,
        }
    }

    /// Restore the panel to a saved `GpuPreset`.  Pushes every value back
    /// onto its slider entity and resets `resize_auto` so the explicit
    /// factor takes precedence.
    fn apply_preset(&mut self, p: &GpuPreset, cx: &mut Context<Self>) {
        self.resize_factor = p.resize_factor;
        self.resize_auto = false;
        self.lc_enabled = p.lc_enabled;
        self.lc_radius
            .update(cx, |s, cx| s.set_value(p.lc_radius_t as f64, cx));
        self.lc_strength
            .update(cx, |s, cx| s.set_value(p.lc_strength as f64, cx));
        self.lc_shadows
            .update(cx, |s, cx| s.set_value(p.lc_shadows as f64, cx));
        self.lc_highlights
            .update(cx, |s, cx| s.set_value(p.lc_highlights as f64, cx));
        self.lc_midpoint
            .update(cx, |s, cx| s.set_value(p.lc_midpoint as f64, cx));
        self.bc_enabled = p.bc_enabled;
        self.bc_brightness
            .update(cx, |s, cx| s.set_value(p.bc_brightness as f64, cx));
        self.bc_contrast
            .update(cx, |s, cx| s.set_value(p.bc_contrast as f64, cx));
        self.vibrance_enabled = p.vibrance_enabled;
        self.vibrance_amount
            .update(cx, |s, cx| s.set_value(p.vibrance_amount as f64, cx));
        self.vibrance_saturation
            .update(cx, |s, cx| s.set_value(p.vibrance_saturation as f64, cx));
        self.hue_enabled = p.hue_enabled;
        self.hue_value
            .update(cx, |s, cx| s.set_value(p.hue_value as f64, cx));
        self.equalize_enabled = p.equalize_enabled;
        self.equalize_amount
            .update(cx, |s, cx| s.set_value(p.equalize_amount as f64, cx));
        self.equalize_shadows
            .update(cx, |s, cx| s.set_value(p.equalize_shadows as f64, cx));
        self.equalize_highlights
            .update(cx, |s, cx| s.set_value(p.equalize_highlights as f64, cx));
    }

    fn save_current_as_preset(&mut self, name: &str, cx: &mut Context<Self>) {
        let name = name.trim();
        if name.is_empty() || name == gpu_presets::CUSTOM_PRESET_LABEL {
            return;
        }
        let preset = self.to_preset(cx);
        if let Err(e) = gpu_presets::save_preset(name, &preset) {
            eprintln!("Failed to save GPU preset: {}", e);
            return;
        }
        self.current_preset = Some(name.to_string());
        self.rebuild_preset_dropdown(cx);
    }

    fn delete_current_preset(&mut self, cx: &mut Context<Self>) {
        let Some(name) = self.current_preset.take() else {
            return;
        };
        let _ = gpu_presets::delete_preset(&name);
        self.rebuild_preset_dropdown(cx);
    }

    fn rebuild_preset_dropdown(&mut self, cx: &mut Context<Self>) {
        let selected = self.current_preset.as_deref();
        self.preset_dropdown = Self::build_preset_dropdown(selected, cx);
        cx.subscribe(&self.preset_dropdown, Self::on_preset_dropdown_change)
            .detach();
        cx.notify();
    }

    /// Per-slider transformations:
    /// * **Radius** is logarithmic in slider position (fine control near the
    ///   low end) and is interpreted as a radius in *source* pixels.  We
    ///   multiply by the effective resize factor so a "60 px at native"
    ///   radius is applied as 240 px on a 4× upscaled buffer (or 15 px on a
    ///   ¼× downscaled buffer) — the perceptual reach stays constant.
    /// * **Midpoint** is shared between the LC and BC stages.
    /// * **Hue** slider is centered on 0.5 (no rotation); the shader takes
    ///   turns directly, so we subtract 0.5 here.
    pub fn get_params(&self, cx: &App) -> UnifiedParams {
        let midpoint = self.lc_midpoint.read(cx).value() as f32;
        let resize = self.effective_resize_factor();
        let radius_t = self.lc_radius.read(cx).value() as f32;
        let radius_displayed = radius_t_to_displayed(radius_t);
        let radius_effective = radius_displayed * resize;
        UnifiedParams {
            resize_factor: resize,
            lc: self.lc_enabled.then(|| LcParams {
                radius: radius_effective,
                strength: self.lc_strength.read(cx).value() as f32,
                shadows: self.lc_shadows.read(cx).value() as f32,
                highlights: self.lc_highlights.read(cx).value() as f32,
            }),
            bc: self.bc_enabled.then(|| BcParams {
                brightness: self.bc_brightness.read(cx).value() as f32,
                contrast: self.bc_contrast.read(cx).value() as f32,
                midpoint,
            }),
            vibrance: self.vibrance_enabled.then(|| VibranceParams {
                amount: self.vibrance_amount.read(cx).value() as f32,
                saturation: self.vibrance_saturation.read(cx).value() as f32,
            }),
            hue: self.hue_enabled.then(|| HueParams {
                hue: self.hue_value.read(cx).value() as f32 - 0.5,
            }),
            equalize: self.equalize_enabled.then(|| EqualizeParams {
                amount: self.equalize_amount.read(cx).value() as f32,
                shadows: self.equalize_shadows.read(cx).value() as f32,
                highlights: self.equalize_highlights.read(cx).value() as f32,
            }),
        }
    }

    /// Reset every slider to its default and disable every stage.
    pub fn reset_all(&mut self, cx: &mut Context<Self>) {
        self.resize_factor = 1.0;
        self.resize_auto = false;
        self.lc_enabled = false;
        self.bc_enabled = false;
        self.vibrance_enabled = false;
        self.hue_enabled = false;
        self.equalize_enabled = false;

        self.lc_radius.update(cx, |s, cx| {
            s.set_value(radius_displayed_to_t(DEFAULT_DISPLAYED_RADIUS) as f64, cx)
        });
        self.lc_strength.update(cx, |s, cx| s.set_value(0.5, cx));
        self.lc_shadows.update(cx, |s, cx| s.set_value(0.0, cx));
        self.lc_highlights.update(cx, |s, cx| s.set_value(0.0, cx));
        self.lc_midpoint.update(cx, |s, cx| s.set_value(0.5, cx));
        self.bc_brightness.update(cx, |s, cx| s.set_value(0.0, cx));
        self.bc_contrast.update(cx, |s, cx| s.set_value(0.0, cx));
        self.vibrance_amount
            .update(cx, |s, cx| s.set_value(0.5, cx));
        self.vibrance_saturation
            .update(cx, |s, cx| s.set_value(0.0, cx));
        self.hue_value.update(cx, |s, cx| s.set_value(0.5, cx));
        self.equalize_amount
            .update(cx, |s, cx| s.set_value(0.5, cx));
        self.equalize_shadows
            .update(cx, |s, cx| s.set_value(0.0, cx));
        self.equalize_highlights
            .update(cx, |s, cx| s.set_value(0.0, cx));
        cx.emit(GpuPipelineControlsEvent::ResetRequested);
    }
}

/// Slider row with label + numeric readout.
fn slider_row(
    label: &str,
    value_text: String,
    slider: Entity<Slider>,
    font_size_scale: f32,
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
                        .text_size(scaled_text_size(11.0, font_size_scale))
                        .text_color(Colors::text())
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .text_size(scaled_text_size(11.0, font_size_scale))
                        .text_color(rgb(0xAAAAAA))
                        .font_weight(FontWeight::BOLD)
                        .child(value_text),
                ),
        )
        .child(slider)
}

/// Header row with enable checkbox, title, and collapse caret.
fn stage_header<F1, F2>(
    title: &str,
    enabled: bool,
    expanded: bool,
    font_size_scale: f32,
    on_enable: F1,
    on_expand: F2,
) -> impl IntoElement
where
    F1: Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    F2: Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
{
    let caret = if expanded { "▼" } else { "▶" };
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .id(SharedString::from(format!("gpu-cb-{title}")))
                .w(px(14.0))
                .h(px(14.0))
                .border_1()
                .border_color(rgb(0x888888))
                .rounded(px(2.0))
                .when(enabled, |d| d.bg(rgb(0x4080FF)))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, on_enable),
        )
        .child(
            div()
                .id(SharedString::from(format!("gpu-h-{title}")))
                .flex_grow()
                .flex()
                .items_center()
                .justify_between()
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, on_expand)
                .child(
                    div()
                        .text_size(scaled_text_size(13.0, font_size_scale))
                        .text_color(Colors::text())
                        .font_weight(FontWeight::BOLD)
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .text_size(scaled_text_size(10.0, font_size_scale))
                        .text_color(rgb(0x888888))
                        .child(caret),
                ),
        )
}

/// Build one resize button.  Caller wires its own cx.listener so the captured
/// factor goes through GPUI's listener machinery cleanly.  When `disabled`
/// is true the button renders dimmed and ignores clicks — used for resize
/// factors that would push the output past the GPU's max texture dim.
fn resize_button(
    label: &'static str,
    active: bool,
    disabled: bool,
    font_size_scale: f32,
    on_click: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let mut button = div()
        .id(SharedString::from(format!("gpu-rs-{label}")))
        .px(px(8.0))
        .py(px(3.0))
        .border_1()
        .rounded(px(3.0))
        .text_size(scaled_text_size(11.0, font_size_scale))
        .child(label);
    if disabled {
        button = button
            .border_color(rgb(0x3a3a3a))
            .text_color(rgb(0x666666))
            .cursor_default();
    } else {
        button = button
            .border_color(rgb(0x666666))
            .text_color(Colors::text())
            .cursor_pointer()
            .when(active, |d| d.bg(rgb(0x4080FF)))
            .on_mouse_down(MouseButton::Left, on_click);
    }
    button
}

impl Render for GpuPipelineControls {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fs = self.font_size_scale;
        let sep = || div().h(px(1.0)).bg(rgba(0x44_44_44_FF));

        // --- Resize section ---
        let auto_resolved = self.effective_resize_factor();
        let auto_on = self.resize_auto;
        let selected_factor = self.resize_factor;
        let label_for = |f: f32| match f {
            f if (f - 0.25).abs() < 0.001 => "¼×",
            f if (f - 0.5).abs() < 0.001 => "½×",
            f if (f - 1.0).abs() < 0.001 => "1×",
            f if (f - 2.0).abs() < 0.001 => "2×",
            f if (f - 4.0).abs() < 0.001 => "4×",
            _ => "?",
        };
        let mut button_row = div().flex().items_center().gap(px(4.0));
        for &f in RESIZE_CHOICES.iter() {
            let active = !auto_on && (selected_factor - f).abs() < 0.001;
            let disabled = self.factor_exceeds_gpu_limits(f);
            button_row = button_row.child(resize_button(
                label_for(f),
                active,
                disabled,
                fs,
                cx.listener(move |this, _evt: &MouseDownEvent, _window, cx| {
                    this.resize_auto = false;
                    this.resize_factor = f;
                    cx.emit(GpuPipelineControlsEvent::ParametersChanged);
                    cx.notify();
                }),
            ));
        }
        let auto_label_owned = if auto_on {
            format!("Auto ({:.2}×)", auto_resolved)
        } else {
            "Auto".to_string()
        };
        button_row = button_row.child(
            div()
                .id("gpu-rs-auto")
                .px(px(8.0))
                .py(px(3.0))
                .border_1()
                .border_color(rgb(0x666666))
                .rounded(px(3.0))
                .cursor_pointer()
                .when(auto_on, |d| d.bg(rgb(0x4080FF)))
                .text_size(scaled_text_size(11.0, fs))
                .text_color(Colors::text())
                .child(auto_label_owned)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _evt: &MouseDownEvent, _window, cx| {
                        this.resize_auto = !this.resize_auto;
                        cx.emit(GpuPipelineControlsEvent::ParametersChanged);
                        cx.notify();
                    }),
                ),
        );
        let resize_section = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(scaled_text_size(13.0, fs))
                    .text_color(Colors::text())
                    .font_weight(FontWeight::BOLD)
                    .child("Resize"),
            )
            .child(button_row);

        // --- Presets section ---
        // Dropdown on its own line; name input + Save / Del on the next.
        // Mirrors the LC controls' preset block.
        let preset_section = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(scaled_text_size(13.0, fs))
                    .text_color(Colors::text())
                    .font_weight(FontWeight::BOLD)
                    .child("Presets"),
            )
            .child(self.preset_dropdown.clone())
            .child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .items_center()
                    .child(div().flex_grow().child(self.preset_name_input.clone()))
                    .child(
                        div()
                            .id("gpu-preset-save")
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(rgba(0x55_55_55_FF))
                            .cursor_pointer()
                            .text_size(scaled_text_size(10.0, fs))
                            .text_color(Colors::text())
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _evt: &MouseDownEvent, _window, cx| {
                                    let name =
                                        this.preset_name_input.read(cx).content().to_string();
                                    this.save_current_as_preset(&name, cx);
                                }),
                            )
                            .child("Save"),
                    )
                    .child(
                        div()
                            .id("gpu-preset-del")
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(rgba(0x55_55_55_FF))
                            .cursor_pointer()
                            .text_size(scaled_text_size(10.0, fs))
                            .text_color(Colors::text())
                            .when(self.current_preset.is_some(), |el| {
                                el.on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _evt: &MouseDownEvent, _window, cx| {
                                        this.delete_current_preset(cx);
                                    }),
                                )
                            })
                            .when(self.current_preset.is_none(), |el| {
                                el.text_color(rgb(0x666666))
                            })
                            .child("Del"),
                    ),
            );

        // --- LC section ---
        let lc_radius_t = self.lc_radius.read(cx).value() as f32;
        let lc_radius_displayed = radius_t_to_displayed(lc_radius_t);
        let lc_strength_v = self.lc_strength.read(cx).value();
        let lc_shadows_v = self.lc_shadows.read(cx).value();
        let lc_highlights_v = self.lc_highlights.read(cx).value();
        let lc_midpoint_v = self.lc_midpoint.read(cx).value();
        let lc_section = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(stage_header(
                "Local Contrast",
                self.lc_enabled,
                self.lc_expanded,
                fs,
                cx.listener(|this, _evt: &MouseDownEvent, _, cx| {
                    this.lc_enabled = !this.lc_enabled;
                    cx.emit(GpuPipelineControlsEvent::ParametersChanged);
                    cx.notify();
                }),
                cx.listener(|this, _evt: &MouseDownEvent, _, cx| {
                    this.lc_expanded = !this.lc_expanded;
                    cx.notify();
                }),
            ))
            .when(self.lc_expanded, |d| {
                d.child(slider_row(
                    "Radius",
                    format!("{:.0} px", lc_radius_displayed),
                    self.lc_radius.clone(),
                    fs,
                ))
                .child(slider_row(
                    "Strength",
                    format!("{:.2}", lc_strength_v),
                    self.lc_strength.clone(),
                    fs,
                ))
                .child(slider_row(
                    "Shadows",
                    format!("{:.2}", lc_shadows_v),
                    self.lc_shadows.clone(),
                    fs,
                ))
                .child(slider_row(
                    "Highlights",
                    format!("{:.2}", lc_highlights_v),
                    self.lc_highlights.clone(),
                    fs,
                ))
                .child(slider_row(
                    "Midpoint (BC pivot)",
                    format!("{:.2}", lc_midpoint_v),
                    self.lc_midpoint.clone(),
                    fs,
                ))
            });

        // --- BC section ---
        let bc_bright_v = self.bc_brightness.read(cx).value();
        let bc_cont_v = self.bc_contrast.read(cx).value();
        let bc_section = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(stage_header(
                "Global Contrast",
                self.bc_enabled,
                self.bc_expanded,
                fs,
                cx.listener(|this, _evt: &MouseDownEvent, _, cx| {
                    this.bc_enabled = !this.bc_enabled;
                    cx.emit(GpuPipelineControlsEvent::ParametersChanged);
                    cx.notify();
                }),
                cx.listener(|this, _evt: &MouseDownEvent, _, cx| {
                    this.bc_expanded = !this.bc_expanded;
                    cx.notify();
                }),
            ))
            .when(self.bc_expanded, |d| {
                d.child(slider_row(
                    "Brightness",
                    format!("{:+.2}", bc_bright_v),
                    self.bc_brightness.clone(),
                    fs,
                ))
                .child(slider_row(
                    "Contrast",
                    format!("{:+.2}", bc_cont_v),
                    self.bc_contrast.clone(),
                    fs,
                ))
            });

        // --- Vibrance section (with Saturation) ---
        let vib_v = self.vibrance_amount.read(cx).value();
        let sat_v = self.vibrance_saturation.read(cx).value();
        let vibrance_section = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(stage_header(
                "Vibrance",
                self.vibrance_enabled,
                self.vibrance_expanded,
                fs,
                cx.listener(|this, _evt: &MouseDownEvent, _, cx| {
                    this.vibrance_enabled = !this.vibrance_enabled;
                    cx.emit(GpuPipelineControlsEvent::ParametersChanged);
                    cx.notify();
                }),
                cx.listener(|this, _evt: &MouseDownEvent, _, cx| {
                    this.vibrance_expanded = !this.vibrance_expanded;
                    cx.notify();
                }),
            ))
            .when(self.vibrance_expanded, |d| {
                d.child(slider_row(
                    "Amount",
                    format!("{:+.2}", vib_v),
                    self.vibrance_amount.clone(),
                    fs,
                ))
                .child(slider_row(
                    "Saturation",
                    format!("{:+.2}", sat_v),
                    self.vibrance_saturation.clone(),
                    fs,
                ))
            });

        // --- Hue section ---
        let hue_v = self.hue_value.read(cx).value();
        let hue_section = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(stage_header(
                "Hue Rotation",
                self.hue_enabled,
                self.hue_expanded,
                fs,
                cx.listener(|this, _evt: &MouseDownEvent, _, cx| {
                    this.hue_enabled = !this.hue_enabled;
                    cx.emit(GpuPipelineControlsEvent::ParametersChanged);
                    cx.notify();
                }),
                cx.listener(|this, _evt: &MouseDownEvent, _, cx| {
                    this.hue_expanded = !this.hue_expanded;
                    cx.notify();
                }),
            ))
            .when(self.hue_expanded, |d| {
                let degrees = (hue_v - 0.5) * 360.0;
                d.child(slider_row(
                    "Hue",
                    format!("{:+.0}°", degrees),
                    self.hue_value.clone(),
                    fs,
                ))
            });

        // --- Equalize section ---
        let eq_v = self.equalize_amount.read(cx).value();
        let eq_shadow_v = self.equalize_shadows.read(cx).value();
        let eq_highlight_v = self.equalize_highlights.read(cx).value();
        let equalize_section = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(stage_header(
                "Equalize",
                self.equalize_enabled,
                self.equalize_expanded,
                fs,
                cx.listener(|this, _evt: &MouseDownEvent, _, cx| {
                    this.equalize_enabled = !this.equalize_enabled;
                    cx.emit(GpuPipelineControlsEvent::ParametersChanged);
                    cx.notify();
                }),
                cx.listener(|this, _evt: &MouseDownEvent, _, cx| {
                    this.equalize_expanded = !this.equalize_expanded;
                    cx.notify();
                }),
            ))
            .when(self.equalize_expanded, |d| {
                d.child(slider_row(
                    "Amount",
                    format!("{:.2}", eq_v),
                    self.equalize_amount.clone(),
                    fs,
                ))
                .child(slider_row(
                    "Shadows",
                    format!("{:.2}", eq_shadow_v),
                    self.equalize_shadows.clone(),
                    fs,
                ))
                .child(slider_row(
                    "Highlights",
                    format!("{:.2}", eq_highlight_v),
                    self.equalize_highlights.clone(),
                    fs,
                ))
            });

        // --- Reset row ---
        let reset_row = div().flex().justify_end().child(
            div()
                .id("gpu-reset")
                .px(px(10.0))
                .py(px(4.0))
                .border_1()
                .border_color(rgb(0x666666))
                .rounded(px(3.0))
                .cursor_pointer()
                .text_size(scaled_text_size(11.0, fs))
                .text_color(Colors::text())
                .child("Reset all")
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _evt: &MouseDownEvent, _, cx| this.reset_all(cx)),
                ),
        );

        // Outer flex container fills the window; the scrollable inner div
        // takes the remaining height (after the title) and scrolls when the
        // expanded sections overflow.
        div()
            .id("gpu-pipeline-root")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .bg(Colors::background())
            .child(
                div()
                    .px(Spacing::lg())
                    .pt(Spacing::lg())
                    .pb(Spacing::sm())
                    .text_size(scaled_text_size(15.0, fs))
                    .text_color(Colors::text())
                    .font_weight(FontWeight::BOLD)
                    .child("GPU Pipeline"),
            )
            .child(
                scrollable_vertical(
                    div()
                        .flex()
                        .flex_col()
                        .gap(Spacing::md())
                        .px(Spacing::lg())
                        .pb(Spacing::lg())
                        .child(sep())
                        .child(preset_section)
                        .child(sep())
                        .child(resize_section)
                        .child(sep())
                        .child(lc_section)
                        .child(sep())
                        .child(bc_section)
                        .child(sep())
                        .child(vibrance_section)
                        .child(sep())
                        .child(hue_section)
                        .child(sep())
                        .child(equalize_section)
                        .child(sep())
                        .child(reset_row),
                )
                .with_scroll_handle(self.scroll_handle.clone())
                .id("gpu-pipeline-scroll"),
            )
    }
}
