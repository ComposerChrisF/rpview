//! Settings window component for rpview-gpui
//!
//! Provides a full-screen overlay with a settings panel allowing users to
//! view and edit application settings interactively.
//!
//! ## Current Status: Interactive UI (Phase 16.7 Complete)
//!
//! The settings window provides interactive controls for all numeric and boolean settings:
//!
//! ### Working Features:
//! - ✅ **Toggle switches**: Toggle boolean settings (10+ settings including animation auto-play, pan acceleration, etc.)
//! - ✅ **Radio buttons**: Select enum values (zoom mode, sort mode, save format)
//! - ✅ **Numeric inputs**: Increment/decrement buttons for all numeric settings (15+ settings)
//!   - Pan speeds (normal, fast, slow)
//!   - Zoom sensitivities (scroll wheel, Z-drag)
//!   - Cache sizes and thread counts
//!   - Filter defaults (brightness, contrast, gamma)
//!   - Appearance settings (transparency, font scale, RGB color picker)
//! - ✅ **Text input**: Window title format is editable with proper focus handling
//! - ✅ **Range validation**: All numeric values are clamped to valid ranges
//! - ✅ **Auto-save**: Changes are automatically saved when closing the settings window
//! - ✅ **Settings persistence**: Changes are saved to JSON on close
//!
//! ### Pending Features (Low Priority):
//! - ⏳ **File browser**: Default save directory requires JSON editing
//! - ⏳ **External viewer list editor**: Add/remove/reorder viewers requires JSON editing
//!
//! ### Keyboard Shortcuts:
//! - `Cmd+,` (or `Ctrl+,` on Windows/Linux): Open/close settings
//! - `Cmd+Enter` or `Esc`: Close and save settings
//!
//! ### Settings File Location:
//! - macOS: `~/Library/Application Support/rpview/settings.json`
//! - Linux: `~/.config/rpview/settings.json`
//! - Windows: `C:\Users\<User>\AppData\Roaming\rpview\settings.json`
//!
//! ## Architecture
//!
//! The component maintains settings in `working_settings` which are immediately
//! visible in the UI and saved to disk when the settings window is closed.

use crate::state::settings::*;
use crate::utils::settings_io;
use crate::utils::style::{Colors, Spacing, TextSize};
use crate::{CloseSettings, ResetSettingsToDefaults};
use ccf_gpui_widgets::prelude::{
    scrollable_vertical, NumberStepper, NumberStepperEvent,
    SegmentedControl, SegmentedControlEvent, Theme, ToggleSwitch, ToggleSwitchEvent,
};
use gpui::prelude::*;
use gpui::*;

/// Available settings sections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    ViewerBehavior,
    Performance,
    KeyboardMouse,
    FileOperations,
    Appearance,
    SortNavigation,
    ExternalTools,
    SettingsFile,
}

impl SettingsSection {
    /// Get display name for the section
    pub fn name(&self) -> &'static str {
        match self {
            Self::ViewerBehavior => "Viewer Behavior",
            Self::Performance => "Performance",
            Self::KeyboardMouse => "Keyboard & Mouse",
            Self::FileOperations => "File Operations",
            Self::Appearance => "Appearance",
            Self::SortNavigation => "Sort & Navigation",
            Self::ExternalTools => "External Tools",
            Self::SettingsFile => "Settings File",
        }
    }

    /// Get all sections in order
    pub fn all() -> Vec<Self> {
        vec![
            Self::ViewerBehavior,
            Self::Performance,
            Self::KeyboardMouse,
            Self::FileOperations,
            Self::Appearance,
            Self::SortNavigation,
            Self::ExternalTools,
            Self::SettingsFile,
        ]
    }
}

/// Settings window component
pub struct SettingsWindow {
    /// Working copy of settings (being edited)
    pub working_settings: AppSettings,
    /// Currently selected section
    pub current_section: SettingsSection,
    /// Focus handle for the settings window
    focus_handle: FocusHandle,
    /// Text input buffer for window title format
    window_title_input: String,
    /// Focus handle for the text input
    text_input_focus: FocusHandle,

    // Number steppers for numeric settings
    state_cache_size_stepper: Entity<NumberStepper>,
    filter_processing_threads_stepper: Entity<NumberStepper>,
    max_image_dimension_stepper: Entity<NumberStepper>,
    pan_speed_normal_stepper: Entity<NumberStepper>,
    pan_speed_fast_stepper: Entity<NumberStepper>,
    pan_speed_slow_stepper: Entity<NumberStepper>,
    scroll_wheel_sensitivity_stepper: Entity<NumberStepper>,
    z_drag_sensitivity_stepper: Entity<NumberStepper>,
    overlay_transparency_stepper: Entity<NumberStepper>,
    font_size_scale_stepper: Entity<NumberStepper>,
    default_brightness_stepper: Entity<NumberStepper>,
    default_contrast_stepper: Entity<NumberStepper>,
    default_gamma_stepper: Entity<NumberStepper>,

    // Segmented controls
    zoom_mode_control: Entity<SegmentedControl>,

    // Toggle switches for boolean settings
    remember_per_image_state_toggle: Entity<ToggleSwitch>,
    animation_auto_play_toggle: Entity<ToggleSwitch>,
    preload_adjacent_images_toggle: Entity<ToggleSwitch>,
    spacebar_pan_accelerated_toggle: Entity<ToggleSwitch>,
    auto_save_filtered_cache_toggle: Entity<ToggleSwitch>,
    remember_last_directory_toggle: Entity<ToggleSwitch>,
    remember_filter_state_toggle: Entity<ToggleSwitch>,
    wrap_navigation_toggle: Entity<ToggleSwitch>,
    show_image_counter_toggle: Entity<ToggleSwitch>,
    file_manager_integration_toggle: Entity<ToggleSwitch>,
}

impl SettingsWindow {
    /// Create a new settings window with the given settings
    pub fn new(settings: AppSettings, cx: &mut Context<Self>) -> Self {
        // Custom theme for steppers with lime green focus color
        let stepper_theme = Theme::dark().with_border_focus(0x50fa7b);

        // Create number steppers for each numeric setting
        // step_small is set to give "lesser of 1 or normal step" behavior
        let state_cache_size_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.viewer_behavior.state_cache_size as f64)
                .min(10.0)
                .max(10000.0)
                .step(100.0)
                .step_small(0.01)  // Alt+click steps by 1
                .display_precision(0)
                .theme(stepper_theme)
        });
        cx.subscribe(&state_cache_size_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.viewer_behavior.state_cache_size = *value as usize;
            cx.notify();
        }).detach();

        let filter_processing_threads_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.performance.filter_processing_threads as f64)
                .min(1.0)
                .max(32.0)
                .step(1.0)
                .display_precision(0)
                .theme(stepper_theme)
        });
        cx.subscribe(&filter_processing_threads_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.performance.filter_processing_threads = *value as usize;
            cx.notify();
        }).detach();

        let max_image_dimension_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.performance.max_image_dimension as f64)
                .min(1000.0)
                .max(100000.0)
                .step(1000.0)
                .step_small(0.001)  // Alt+click steps by 1
                .display_precision(0)
                .theme(stepper_theme)
        });
        cx.subscribe(&max_image_dimension_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.performance.max_image_dimension = *value as u32;
            cx.notify();
        }).detach();

        let pan_speed_normal_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.keyboard_mouse.pan_speed_normal.into())
                .min(1.0)
                .max(100.0)
                .step(1.0)
                .display_precision(1)
                .theme(stepper_theme)
        });
        cx.subscribe(&pan_speed_normal_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.keyboard_mouse.pan_speed_normal = *value as f32;
            cx.notify();
        }).detach();

        let pan_speed_fast_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.keyboard_mouse.pan_speed_fast.into())
                .min(1.0)
                .max(200.0)
                .step(5.0)
                .step_small(0.2)  // Alt+click steps by 1
                .display_precision(1)
                .theme(stepper_theme)
        });
        cx.subscribe(&pan_speed_fast_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.keyboard_mouse.pan_speed_fast = *value as f32;
            cx.notify();
        }).detach();

        let pan_speed_slow_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.keyboard_mouse.pan_speed_slow.into())
                .min(0.5)
                .max(50.0)
                .step(0.5)
                .display_precision(1)
                .theme(stepper_theme)
        });
        cx.subscribe(&pan_speed_slow_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.keyboard_mouse.pan_speed_slow = *value as f32;
            cx.notify();
        }).detach();

        let scroll_wheel_sensitivity_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.keyboard_mouse.scroll_wheel_sensitivity.into())
                .min(1.01)
                .max(2.0)
                .step(0.05)
                .display_precision(2)
                .theme(stepper_theme)
        });
        cx.subscribe(&scroll_wheel_sensitivity_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.keyboard_mouse.scroll_wheel_sensitivity = *value as f32;
            cx.notify();
        }).detach();

        let z_drag_sensitivity_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.keyboard_mouse.z_drag_sensitivity.into())
                .min(0.001)
                .max(0.1)
                .step(0.001)
                .display_precision(3)
                .theme(stepper_theme)
        });
        cx.subscribe(&z_drag_sensitivity_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.keyboard_mouse.z_drag_sensitivity = *value as f32;
            cx.notify();
        }).detach();

        let overlay_transparency_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.appearance.overlay_transparency as f64)
                .min(0.0)
                .max(255.0)
                .step(10.0)
                .step_small(0.1)  // Alt+click steps by 1
                .display_precision(0)
                .theme(stepper_theme)
        });
        cx.subscribe(&overlay_transparency_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.appearance.overlay_transparency = *value as u8;
            cx.notify();
        }).detach();

        let font_size_scale_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.appearance.font_size_scale.into())
                .min(0.5)
                .max(8.0)
                .step(0.1)
                .display_precision(1)
                .theme(stepper_theme)
        });
        cx.subscribe(&font_size_scale_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.appearance.font_size_scale = *value as f32;
            cx.notify();
        }).detach();

        let default_brightness_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.filters.default_brightness.into())
                .min(-100.0)
                .max(100.0)
                .step(5.0)
                .step_small(0.2)  // Alt+click steps by 1
                .display_precision(0)
                .theme(stepper_theme)
        });
        cx.subscribe(&default_brightness_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.filters.default_brightness = *value as f32;
            cx.notify();
        }).detach();

        let default_contrast_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.filters.default_contrast.into())
                .min(-100.0)
                .max(100.0)
                .step(5.0)
                .step_small(0.2)  // Alt+click steps by 1
                .display_precision(0)
                .theme(stepper_theme)
        });
        cx.subscribe(&default_contrast_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.filters.default_contrast = *value as f32;
            cx.notify();
        }).detach();

        let default_gamma_stepper = cx.new(|cx| {
            NumberStepper::new(cx)
                .with_value(settings.filters.default_gamma.into())
                .min(0.1)
                .max(10.0)
                .step(0.1)
                .display_precision(2)
                .theme(stepper_theme)
        });
        cx.subscribe(&default_gamma_stepper, |this, _stepper, event: &NumberStepperEvent, cx| {
            let NumberStepperEvent::Change(value) = event;
            this.working_settings.filters.default_gamma = *value as f32;
            cx.notify();
        }).detach();

        // Segmented control for zoom mode
        let initial_zoom = match settings.viewer_behavior.default_zoom_mode {
            ZoomMode::FitToWindow => "fit",
            ZoomMode::OneHundredPercent => "100",
        };
        let zoom_mode_control = cx.new(|cx| {
            SegmentedControl::new(cx)
                .options(vec![
                    ("fit", "Fit to Window"),
                    ("100", "100% (Actual Size)"),
                ])
                .with_selected(initial_zoom)
                .theme(stepper_theme)
        });
        cx.subscribe(&zoom_mode_control, |this, _control, event: &SegmentedControlEvent, cx| {
            let SegmentedControlEvent::Change(value) = event;
            this.working_settings.viewer_behavior.default_zoom_mode = match value.as_str() {
                "fit" => ZoomMode::FitToWindow,
                "100" => ZoomMode::OneHundredPercent,
                _ => ZoomMode::FitToWindow,
            };
            cx.notify();
        }).detach();

        // Custom theme for toggle switches with lime green theme
        // On state: saturated lime, Off state: very dark lime
        let toggle_theme = Theme::dark()
            .with_primary(0x2ecc71)
            .with_border_focus(0x2ecc71)
            .with_bg_input(0x143d14);

        // Create toggle switch entities for boolean settings
        let remember_per_image_state_toggle = cx.new(|cx| {
            ToggleSwitch::new(cx)
                .with_on(settings.viewer_behavior.remember_per_image_state)
                .label("Remember per-image state")
                .theme(toggle_theme)
        });
        cx.subscribe(&remember_per_image_state_toggle, |this, _toggle, event: &ToggleSwitchEvent, cx| {
            let ToggleSwitchEvent::Toggle(on) = event;
            this.working_settings.viewer_behavior.remember_per_image_state = *on;
            cx.notify();
        }).detach();

        let animation_auto_play_toggle = cx.new(|cx| {
            ToggleSwitch::new(cx)
                .with_on(settings.viewer_behavior.animation_auto_play)
                .label("Auto-play animations")
                .theme(toggle_theme)
        });
        cx.subscribe(&animation_auto_play_toggle, |this, _toggle, event: &ToggleSwitchEvent, cx| {
            let ToggleSwitchEvent::Toggle(on) = event;
            this.working_settings.viewer_behavior.animation_auto_play = *on;
            cx.notify();
        }).detach();

        let preload_adjacent_images_toggle = cx.new(|cx| {
            ToggleSwitch::new(cx)
                .with_on(settings.performance.preload_adjacent_images)
                .label("Preload adjacent images")
                .theme(toggle_theme)
        });
        cx.subscribe(&preload_adjacent_images_toggle, |this, _toggle, event: &ToggleSwitchEvent, cx| {
            let ToggleSwitchEvent::Toggle(on) = event;
            this.working_settings.performance.preload_adjacent_images = *on;
            cx.notify();
        }).detach();

        let spacebar_pan_accelerated_toggle = cx.new(|cx| {
            ToggleSwitch::new(cx)
                .with_on(settings.keyboard_mouse.spacebar_pan_accelerated)
                .label("Spacebar pan acceleration")
                .theme(toggle_theme)
        });
        cx.subscribe(&spacebar_pan_accelerated_toggle, |this, _toggle, event: &ToggleSwitchEvent, cx| {
            let ToggleSwitchEvent::Toggle(on) = event;
            this.working_settings.keyboard_mouse.spacebar_pan_accelerated = *on;
            cx.notify();
        }).detach();

        let auto_save_filtered_cache_toggle = cx.new(|cx| {
            ToggleSwitch::new(cx)
                .with_on(settings.file_operations.auto_save_filtered_cache)
                .label("Auto-save filtered cache")
                .theme(toggle_theme)
        });
        cx.subscribe(&auto_save_filtered_cache_toggle, |this, _toggle, event: &ToggleSwitchEvent, cx| {
            let ToggleSwitchEvent::Toggle(on) = event;
            this.working_settings.file_operations.auto_save_filtered_cache = *on;
            cx.notify();
        }).detach();

        let remember_last_directory_toggle = cx.new(|cx| {
            ToggleSwitch::new(cx)
                .with_on(settings.file_operations.remember_last_directory)
                .label("Remember last directory")
                .theme(toggle_theme)
        });
        cx.subscribe(&remember_last_directory_toggle, |this, _toggle, event: &ToggleSwitchEvent, cx| {
            let ToggleSwitchEvent::Toggle(on) = event;
            this.working_settings.file_operations.remember_last_directory = *on;
            cx.notify();
        }).detach();

        let remember_filter_state_toggle = cx.new(|cx| {
            ToggleSwitch::new(cx)
                .with_on(settings.filters.remember_filter_state)
                .label("Remember filter state per-image")
                .theme(toggle_theme)
        });
        cx.subscribe(&remember_filter_state_toggle, |this, _toggle, event: &ToggleSwitchEvent, cx| {
            let ToggleSwitchEvent::Toggle(on) = event;
            this.working_settings.filters.remember_filter_state = *on;
            cx.notify();
        }).detach();

        let wrap_navigation_toggle = cx.new(|cx| {
            ToggleSwitch::new(cx)
                .with_on(settings.sort_navigation.wrap_navigation)
                .label("Wrap navigation")
                .theme(toggle_theme)
        });
        cx.subscribe(&wrap_navigation_toggle, |this, _toggle, event: &ToggleSwitchEvent, cx| {
            let ToggleSwitchEvent::Toggle(on) = event;
            this.working_settings.sort_navigation.wrap_navigation = *on;
            cx.notify();
        }).detach();

        let show_image_counter_toggle = cx.new(|cx| {
            ToggleSwitch::new(cx)
                .with_on(settings.sort_navigation.show_image_counter)
                .label("Show image counter")
                .theme(toggle_theme)
        });
        cx.subscribe(&show_image_counter_toggle, |this, _toggle, event: &ToggleSwitchEvent, cx| {
            let ToggleSwitchEvent::Toggle(on) = event;
            this.working_settings.sort_navigation.show_image_counter = *on;
            cx.notify();
        }).detach();

        let file_manager_integration_toggle = cx.new(|cx| {
            ToggleSwitch::new(cx)
                .with_on(settings.external_tools.enable_file_manager_integration)
                .label("File manager integration")
                .theme(toggle_theme)
        });
        cx.subscribe(&file_manager_integration_toggle, |this, _toggle, event: &ToggleSwitchEvent, cx| {
            let ToggleSwitchEvent::Toggle(on) = event;
            this.working_settings.external_tools.enable_file_manager_integration = *on;
            cx.notify();
        }).detach();

        Self {
            window_title_input: settings.appearance.window_title_format.clone(),
            working_settings: settings,
            current_section: SettingsSection::ViewerBehavior,
            focus_handle: cx.focus_handle(),
            text_input_focus: cx.focus_handle(),
            state_cache_size_stepper,
            filter_processing_threads_stepper,
            max_image_dimension_stepper,
            pan_speed_normal_stepper,
            pan_speed_fast_stepper,
            pan_speed_slow_stepper,
            scroll_wheel_sensitivity_stepper,
            z_drag_sensitivity_stepper,
            overlay_transparency_stepper,
            font_size_scale_stepper,
            default_brightness_stepper,
            default_contrast_stepper,
            default_gamma_stepper,
            zoom_mode_control,
            remember_per_image_state_toggle,
            animation_auto_play_toggle,
            preload_adjacent_images_toggle,
            spacebar_pan_accelerated_toggle,
            auto_save_filtered_cache_toggle,
            remember_last_directory_toggle,
            remember_filter_state_toggle,
            wrap_navigation_toggle,
            show_image_counter_toggle,
            file_manager_integration_toggle,
        }
    }

    /// Change the current section
    pub fn set_section(&mut self, section: SettingsSection, cx: &mut Context<Self>) {
        self.current_section = section;
        cx.notify();
    }

    /// Reset all settings to defaults
    pub fn reset_to_defaults(&mut self, cx: &mut Context<Self>) {
        let defaults = AppSettings::default();
        self.working_settings = defaults.clone();
        self.window_title_input = defaults.appearance.window_title_format.clone();

        // Reset all stepper values
        self.state_cache_size_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.viewer_behavior.state_cache_size as f64, cx);
        });
        self.filter_processing_threads_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.performance.filter_processing_threads as f64, cx);
        });
        self.max_image_dimension_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.performance.max_image_dimension as f64, cx);
        });
        self.pan_speed_normal_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.keyboard_mouse.pan_speed_normal.into(), cx);
        });
        self.pan_speed_fast_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.keyboard_mouse.pan_speed_fast.into(), cx);
        });
        self.pan_speed_slow_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.keyboard_mouse.pan_speed_slow.into(), cx);
        });
        self.scroll_wheel_sensitivity_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.keyboard_mouse.scroll_wheel_sensitivity.into(), cx);
        });
        self.z_drag_sensitivity_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.keyboard_mouse.z_drag_sensitivity.into(), cx);
        });
        self.overlay_transparency_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.appearance.overlay_transparency as f64, cx);
        });
        self.font_size_scale_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.appearance.font_size_scale.into(), cx);
        });
        self.default_brightness_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.filters.default_brightness.into(), cx);
        });
        self.default_contrast_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.filters.default_contrast.into(), cx);
        });
        self.default_gamma_stepper.update(cx, |stepper, cx| {
            stepper.set_value(defaults.filters.default_gamma.into(), cx);
        });

        // Reset segmented controls
        let zoom_value = match defaults.viewer_behavior.default_zoom_mode {
            ZoomMode::FitToWindow => "fit",
            ZoomMode::OneHundredPercent => "100",
        };
        self.zoom_mode_control.update(cx, |control, cx| {
            control.set_selected(zoom_value, cx);
        });

        // Reset toggle switches
        self.remember_per_image_state_toggle.update(cx, |toggle, cx| {
            toggle.set_on(defaults.viewer_behavior.remember_per_image_state, cx);
        });
        self.animation_auto_play_toggle.update(cx, |toggle, cx| {
            toggle.set_on(defaults.viewer_behavior.animation_auto_play, cx);
        });
        self.preload_adjacent_images_toggle.update(cx, |toggle, cx| {
            toggle.set_on(defaults.performance.preload_adjacent_images, cx);
        });
        self.spacebar_pan_accelerated_toggle.update(cx, |toggle, cx| {
            toggle.set_on(defaults.keyboard_mouse.spacebar_pan_accelerated, cx);
        });
        self.auto_save_filtered_cache_toggle.update(cx, |toggle, cx| {
            toggle.set_on(defaults.file_operations.auto_save_filtered_cache, cx);
        });
        self.remember_last_directory_toggle.update(cx, |toggle, cx| {
            toggle.set_on(defaults.file_operations.remember_last_directory, cx);
        });
        self.remember_filter_state_toggle.update(cx, |toggle, cx| {
            toggle.set_on(defaults.filters.remember_filter_state, cx);
        });
        self.wrap_navigation_toggle.update(cx, |toggle, cx| {
            toggle.set_on(defaults.sort_navigation.wrap_navigation, cx);
        });
        self.show_image_counter_toggle.update(cx, |toggle, cx| {
            toggle.set_on(defaults.sort_navigation.show_image_counter, cx);
        });
        self.file_manager_integration_toggle.update(cx, |toggle, cx| {
            toggle.set_on(defaults.external_tools.enable_file_manager_integration, cx);
        });
    }

    /// Get the final settings (for apply)
    pub fn get_settings(&self) -> AppSettings {
        let mut settings = self.working_settings.clone();
        // Apply text input buffer to settings
        settings.appearance.window_title_format = self.window_title_input.clone();
        settings
    }

    /// Render the header
    fn render_header(&self) -> impl IntoElement {
        div()
            .px(Spacing::xl())
            .pt(Spacing::xl())
            .pb(Spacing::md())
            .border_b_1()
            .border_color(rgb(0x444444))
            .text_size(TextSize::xxl())
            .text_color(Colors::text())
            .font_weight(FontWeight::BOLD)
            .child("Settings")
    }

    /// Render the sidebar navigation
    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.current_section;

        div()
            .flex()
            .flex_col()
            .w(px(200.0))
            .bg(rgb(0x2a2a2a))
            .border_r_1()
            .border_color(rgb(0x444444))
            .p(Spacing::md())
            .children(SettingsSection::all().into_iter().map(move |section| {
                let is_selected = section == current;
                div()
                    .px(Spacing::md())
                    .py(Spacing::sm())
                    .mb(Spacing::xs())
                    .rounded(px(4.0))
                    .when(is_selected, |div| {
                        div.bg(rgb(0x444444)).text_color(Colors::info())
                    })
                    .when(!is_selected, |div| {
                        div.bg(rgb(0x2a2a2a)).text_color(Colors::text())
                    })
                    .text_size(TextSize::md())
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                            this.set_section(section, cx);
                        }),
                    )
                    .child(section.name())
            }))
    }

    /// Render a section header within the content area
    fn render_section_header(&self, title: String) -> impl IntoElement {
        div()
            .text_size(TextSize::lg())
            .text_color(Colors::text())
            .font_weight(FontWeight::BOLD)
            .mb(Spacing::md())
            .child(title)
    }

    /// Render a label for a setting
    fn render_label(&self, text: String, description: Option<String>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .mb(Spacing::xs())
            .child(
                div()
                    .text_size(TextSize::md())
                    .text_color(Colors::text())
                    .child(text),
            )
            .when_some(description, |el, desc| {
                el.child(
                    div()
                        .text_size(TextSize::sm())
                        .text_color(rgb(0xaaaaaa))
                        .child(desc),
                )
            })
    }

    /// Render a simple text input
    #[allow(dead_code)]
    fn render_text_input(
        &mut self,
        label: String,
        value: String,
        description: Option<String>,
        _on_change: impl Fn(&mut SettingsWindow, String, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .mb(Spacing::md())
            .child(self.render_label(label, description))
            .child(
                div()
                    .w_full()
                    .px(Spacing::sm())
                    .py(Spacing::xs())
                    .bg(rgb(0x2a2a2a))
                    .border_1()
                    .border_color(rgb(0x444444))
                    .rounded(px(4.0))
                    .text_size(TextSize::sm())
                    .text_color(Colors::text())
                    .cursor(gpui::CursorStyle::IBeam)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_this, _event: &MouseDownEvent, _window, cx| {
                            // Simple click-to-edit: For now, we'll use a simpler approach
                            // User can edit the value in the JSON file for complex editing
                            cx.notify();
                        }),
                    )
                    .child(value),
            )
    }

    /// Render a row with a label and NumberStepper
    fn render_stepper_row(
        &self,
        label: String,
        description: Option<String>,
        stepper: &Entity<NumberStepper>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .mb(Spacing::md())
            .child(self.render_label(label, description))
            .child(
                div()
                    .w(px(150.0))  // Constrain stepper width
                    .child(stepper.clone())
            )
    }

    /// Render a row with a ToggleSwitch widget (label is part of the toggle)
    fn render_toggle_row(
        &self,
        description: Option<String>,
        toggle: &Entity<ToggleSwitch>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .mb(Spacing::md())
            .child(toggle.clone())
            .when_some(description, |el, desc| {
                el.child(
                    div()
                        .text_size(TextSize::sm())
                        .text_color(rgb(0x888888))
                        .mt(Spacing::xs())
                        .child(desc)
                )
            })
    }

    /// Render viewer behavior section
    fn render_viewer_behavior(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Viewer Behavior".to_string()))
            .child(
                div()
                    .mb(Spacing::md())
                    .child(self.render_label(
                        "Default Zoom Mode".to_string(),
                        Some("How images are initially displayed".to_string()),
                    ))
                    .child(self.zoom_mode_control.clone()),
            )
            .child(self.render_toggle_row(
                Some("Remember zoom, pan, and filters for each image".to_string()),
                &self.remember_per_image_state_toggle,
            ))
            .child(self.render_stepper_row(
                "State cache size".to_string(),
                Some("Maximum number of images to cache state for".to_string()),
                &self.state_cache_size_stepper,
            ))
            .child(self.render_toggle_row(
                Some("Start animated GIFs/WEBPs playing automatically".to_string()),
                &self.animation_auto_play_toggle,
            ))
    }

    /// Render performance section
    fn render_performance(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Performance".to_string()))
            .child(self.render_toggle_row(
                Some("Load next/previous images in background for faster navigation".to_string()),
                &self.preload_adjacent_images_toggle,
            ))
            .child(self.render_stepper_row(
                "Filter processing threads".to_string(),
                Some("Number of CPU threads for filter processing".to_string()),
                &self.filter_processing_threads_stepper,
            ))
            .child(self.render_stepper_row(
                "Maximum image dimension".to_string(),
                Some("Maximum allowed width or height for loading images".to_string()),
                &self.max_image_dimension_stepper,
            ))
    }

    /// Render keyboard & mouse section
    fn render_keyboard_mouse(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Keyboard & Mouse".to_string()))
            .child(self.render_stepper_row(
                "Pan speed (normal)".to_string(),
                Some("Base keyboard pan speed in pixels".to_string()),
                &self.pan_speed_normal_stepper,
            ))
            .child(self.render_stepper_row(
                "Pan speed (fast, with Shift)".to_string(),
                Some("Pan speed with Shift modifier".to_string()),
                &self.pan_speed_fast_stepper,
            ))
            .child(self.render_stepper_row(
                "Pan speed (slow, with Alt)".to_string(),
                Some("Pan speed with Alt modifier".to_string()),
                &self.pan_speed_slow_stepper,
            ))
            .child(self.render_stepper_row(
                "Scroll wheel sensitivity".to_string(),
                Some("Zoom factor per scroll wheel notch".to_string()),
                &self.scroll_wheel_sensitivity_stepper,
            ))
            .child(self.render_stepper_row(
                "Z-drag zoom sensitivity".to_string(),
                Some("Zoom percentage change per pixel when Z-dragging".to_string()),
                &self.z_drag_sensitivity_stepper,
            ))
            .child(self.render_toggle_row(
                Some("Enable acceleration for spacebar+mouse panning".to_string()),
                &self.spacebar_pan_accelerated_toggle,
            ))
    }

    /// Render file operations section
    fn render_file_operations(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let default_save_directory = self
            .working_settings
            .file_operations
            .default_save_directory
            .clone();
        let default_save_format = self.working_settings.file_operations.default_save_format;

        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("File Operations".to_string()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label(
                        "Default save directory".to_string(),
                        Some("Where filtered images are saved by default".to_string()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(Spacing::sm())
                            .child(
                                div()
                                    .flex_1()
                                    .px(Spacing::sm())
                                    .py(Spacing::xs())
                                    .bg(rgb(0x2a2a2a))
                                    .border_1()
                                    .border_color(rgb(0x444444))
                                    .rounded(px(4.0))
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0xaaaaaa))
                                    .child(
                                        default_save_directory
                                            .as_ref()
                                            .map(|p| p.display().to_string())
                                            .unwrap_or_else(|| "Same as current image".to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .px(Spacing::md())
                                    .py(Spacing::xs())
                                    .bg(rgb(0x444444))
                                    .rounded(px(4.0))
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .cursor_pointer()
                                    .child("Browse..."),
                            ),
                    ),
            )
            .child(
                div()
                    .mb(Spacing::md())
                    .child(self.render_label(
                        "Default save format".to_string(),
                        Some("Format for saving filtered images".to_string()),
                    ))
                    .child(div().w(px(200.0)).child(
                        div().flex().flex_row().gap(Spacing::xs()).children(
                            SaveFormat::all().into_iter().map(|format| {
                                let is_selected = format == default_save_format;
                                div()
                                    .px(Spacing::md())
                                    .py(Spacing::sm())
                                    .rounded(px(4.0))
                                    .border_1()
                                    .cursor_pointer()
                                    .when(is_selected, |div| {
                                        div.border_color(Colors::info()).bg(rgba(0x50fa7b22))
                                    })
                                    .when(!is_selected, |div| div.border_color(rgb(0x444444)))
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            move |this, _event: &MouseDownEvent, _window, cx| {
                                                this.working_settings
                                                    .file_operations
                                                    .default_save_format = format;
                                                cx.notify();
                                            },
                                        ),
                                    )
                                    .child(format.display_name())
                            }),
                        ),
                    )),
            )
            .child(self.render_toggle_row(
                Some("Permanently save filtered image cache to disk".to_string()),
                &self.auto_save_filtered_cache_toggle,
            ))
            .child(self.render_toggle_row(
                Some("Remember last used directory in file dialogs".to_string()),
                &self.remember_last_directory_toggle,
            ))
    }

    /// Render appearance section
    fn render_appearance(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let background_color = self.working_settings.appearance.background_color;

        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Appearance".to_string()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label("Background color".to_string(), Some("Image viewer background color".to_string())))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(Spacing::xs())
                            .child(
                                // Row with swatch and RGB sliders (vertically centered)
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap(Spacing::md())
                                    .items_center()
                                    .child(
                                        // Color swatch (just the square)
                                        div()
                                            .w(px(100.0))
                                            .h(px(100.0))
                                            .rounded(px(4.0))
                                            .border_1()
                                            .border_color(rgb(0x666666))
                                            .bg(rgb(
                                                ((background_color[0] as u32) << 16) |
                                                ((background_color[1] as u32) << 8) |
                                                (background_color[2] as u32)
                                            ))
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(Spacing::xs())
                                            .flex_1()
                                    // Red channel
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap(Spacing::xs())
                                            .child(
                                                div()
                                                    .w(px(20.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(rgb(0xff6666))
                                                    .child("R")
                                            )
                                            .child(
                                                div()
                                                    .px(Spacing::xs())
                                                    .py(px(2.0))
                                                    .bg(rgb(0x444444))
                                                    .rounded(px(3.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(Colors::text())
                                                    .cursor_pointer()
                                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                                                        this.working_settings.appearance.background_color[0] =
                                                            this.working_settings.appearance.background_color[0].saturating_sub(5);
                                                        cx.notify();
                                                    }))
                                                    .child("−")
                                            )
                                            .child(
                                                div()
                                                    .w(px(40.0))
                                                    .px(Spacing::xs())
                                                    .py(px(2.0))
                                                    .bg(rgb(0x2a2a2a))
                                                    .border_1()
                                                    .border_color(rgb(0x444444))
                                                    .rounded(px(3.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(Colors::text())
                                                    .text_align(gpui::TextAlign::Center)
                                                    .child(format!("{}", background_color[0]))
                                            )
                                            .child(
                                                div()
                                                    .px(Spacing::xs())
                                                    .py(px(2.0))
                                                    .bg(rgb(0x444444))
                                                    .rounded(px(3.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(Colors::text())
                                                    .cursor_pointer()
                                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                                                        this.working_settings.appearance.background_color[0] =
                                                            this.working_settings.appearance.background_color[0].saturating_add(5);
                                                        cx.notify();
                                                    }))
                                                    .child("+")
                                            )
                                    )
                                    // Green channel
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap(Spacing::xs())
                                            .child(
                                                div()
                                                    .w(px(20.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(rgb(0x66ff66))
                                                    .child("G")
                                            )
                                            .child(
                                                div()
                                                    .px(Spacing::xs())
                                                    .py(px(2.0))
                                                    .bg(rgb(0x444444))
                                                    .rounded(px(3.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(Colors::text())
                                                    .cursor_pointer()
                                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                                                        this.working_settings.appearance.background_color[1] =
                                                            this.working_settings.appearance.background_color[1].saturating_sub(5);
                                                        cx.notify();
                                                    }))
                                                    .child("−")
                                            )
                                            .child(
                                                div()
                                                    .w(px(40.0))
                                                    .px(Spacing::xs())
                                                    .py(px(2.0))
                                                    .bg(rgb(0x2a2a2a))
                                                    .border_1()
                                                    .border_color(rgb(0x444444))
                                                    .rounded(px(3.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(Colors::text())
                                                    .text_align(gpui::TextAlign::Center)
                                                    .child(format!("{}", background_color[1]))
                                            )
                                            .child(
                                                div()
                                                    .px(Spacing::xs())
                                                    .py(px(2.0))
                                                    .bg(rgb(0x444444))
                                                    .rounded(px(3.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(Colors::text())
                                                    .cursor_pointer()
                                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                                                        this.working_settings.appearance.background_color[1] =
                                                            this.working_settings.appearance.background_color[1].saturating_add(5);
                                                        cx.notify();
                                                    }))
                                                    .child("+")
                                            )
                                    )
                                    // Blue channel
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap(Spacing::xs())
                                            .child(
                                                div()
                                                    .w(px(20.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(rgb(0x6666ff))
                                                    .child("B")
                                            )
                                            .child(
                                                div()
                                                    .px(Spacing::xs())
                                                    .py(px(2.0))
                                                    .bg(rgb(0x444444))
                                                    .rounded(px(3.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(Colors::text())
                                                    .cursor_pointer()
                                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                                                        this.working_settings.appearance.background_color[2] =
                                                            this.working_settings.appearance.background_color[2].saturating_sub(5);
                                                        cx.notify();
                                                    }))
                                                    .child("−")
                                            )
                                            .child(
                                                div()
                                                    .w(px(40.0))
                                                    .px(Spacing::xs())
                                                    .py(px(2.0))
                                                    .bg(rgb(0x2a2a2a))
                                                    .border_1()
                                                    .border_color(rgb(0x444444))
                                                    .rounded(px(3.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(Colors::text())
                                                    .text_align(gpui::TextAlign::Center)
                                                    .child(format!("{}", background_color[2]))
                                            )
                                            .child(
                                                div()
                                                    .px(Spacing::xs())
                                                    .py(px(2.0))
                                                    .bg(rgb(0x444444))
                                                    .rounded(px(3.0))
                                                    .text_size(TextSize::sm())
                                                    .text_color(Colors::text())
                                                    .cursor_pointer()
                                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                                                        this.working_settings.appearance.background_color[2] =
                                                            this.working_settings.appearance.background_color[2].saturating_add(5);
                                                        cx.notify();
                                                    }))
                                                    .child("+")
                                            )
                                    )
                                    )
                            )
                            .child(
                                // Hex label row (centered under swatch)
                                div()
                                    .w(px(100.0))
                                    .flex()
                                    .justify_center()
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0xaaaaaa))
                                    .child(format!(
                                        "#{:02x}{:02x}{:02x}",
                                        background_color[0],
                                        background_color[1],
                                        background_color[2]
                                    ))
                            )
                    )
            )
            .child(self.render_stepper_row(
                "Overlay transparency".to_string(),
                Some("Transparency for overlay backgrounds (0-255)".to_string()),
                &self.overlay_transparency_stepper,
            ))
            .child(self.render_stepper_row(
                "Font size scale".to_string(),
                Some("Scale factor for overlay text (0.5 - 8.0)".to_string()),
                &self.font_size_scale_stepper,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label("Window title format".to_string(), Some("Template: {filename}, {index}, {total}".to_string())))
                    .child({
                        let text_input_focus = self.text_input_focus.clone();
                        let is_focused = text_input_focus.contains_focused(window, cx);
                        let text_with_cursor = if is_focused {
                            format!("{}|", self.window_title_input)
                        } else {
                            self.window_title_input.clone()
                        };

                        div()
                            .w_full()
                            .px(Spacing::sm())
                            .py(Spacing::xs())
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(if is_focused { rgb(0x50fa7b) } else { rgb(0x444444) })
                            .rounded(px(4.0))
                            .text_size(TextSize::md())
                            .text_color(Colors::text())
                            .cursor(gpui::CursorStyle::IBeam)
                            .track_focus(&text_input_focus)
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                this.text_input_focus.focus(window);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                // Handle text input
                                let key = &event.keystroke.key;
                                match key.as_str() {
                                    "backspace" => {
                                        this.window_title_input.pop();
                                        cx.notify();
                                    }
                                    "escape" => {
                                        // Unfocus the text input and focus the main settings window
                                        this.focus_handle.focus(window);
                                        cx.notify();
                                    }
                                    _ => {
                                        // Add printable characters (single character keys)
                                        if key.len() == 1 && !event.keystroke.modifiers.control && !event.keystroke.modifiers.platform {
                                            this.window_title_input.push_str(key);
                                            cx.notify();
                                        }
                                    }
                                }
                            }))
                            .child(text_with_cursor)
                    })
            )
    }

    /// Render filters section
    #[allow(dead_code)]
    fn render_filters(&self) -> impl IntoElement {
        let filter_presets = self.working_settings.filters.filter_presets.clone();

        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Filters".to_string()))
            .child(self.render_stepper_row(
                "Default brightness".to_string(),
                Some("Default brightness value when resetting (-100 to +100)".to_string()),
                &self.default_brightness_stepper,
            ))
            .child(self.render_stepper_row(
                "Default contrast".to_string(),
                Some("Default contrast value when resetting (-100 to +100)".to_string()),
                &self.default_contrast_stepper,
            ))
            .child(self.render_stepper_row(
                "Default gamma".to_string(),
                Some("Default gamma value when resetting (0.1 to 10.0)".to_string()),
                &self.default_gamma_stepper,
            ))
            .child(self.render_toggle_row(
                Some("Remember filter settings for each image separately".to_string()),
                &self.remember_filter_state_toggle,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label(
                        "Filter presets".to_string(),
                        Some("Saved filter combinations".to_string()),
                    ))
                    .child(
                        div()
                            .px(Spacing::md())
                            .py(Spacing::md())
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(rgb(0x444444))
                            .rounded(px(4.0))
                            .text_size(TextSize::sm())
                            .text_color(rgb(0xaaaaaa))
                            .when(filter_presets.is_empty(), |el| el.child("No presets saved"))
                            .when(!filter_presets.is_empty(), |el| {
                                el.children(filter_presets.iter().map(|preset| {
                                    div().mb(Spacing::xs()).child(preset.name.clone())
                                }))
                            }),
                    ),
            )
    }

    /// Render sort & navigation section
    fn render_sort_navigation(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let default_sort_mode = self.working_settings.sort_navigation.default_sort_mode;

        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Sort & Navigation".to_string()))
            .child(
                div()
                    .mb(Spacing::md())
                    .child(self.render_label(
                        "Default sort mode".to_string(),
                        Some("How images are sorted on startup".to_string()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(Spacing::md())
                            .child(
                                div()
                                    .px(Spacing::md())
                                    .py(Spacing::sm())
                                    .rounded(px(4.0))
                                    .border_1()
                                    .cursor_pointer()
                                    .when(
                                        default_sort_mode == SortModeWrapper::Alphabetical,
                                        |div| div.border_color(Colors::info()).bg(rgba(0x50fa7b22)),
                                    )
                                    .when(
                                        default_sort_mode != SortModeWrapper::Alphabetical,
                                        |div| div.border_color(rgb(0x444444)),
                                    )
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            |this, _event: &MouseDownEvent, _window, cx| {
                                                this.working_settings
                                                    .sort_navigation
                                                    .default_sort_mode =
                                                    SortModeWrapper::Alphabetical;
                                                cx.notify();
                                            },
                                        ),
                                    )
                                    .child("Alphabetical"),
                            )
                            .child(
                                div()
                                    .px(Spacing::md())
                                    .py(Spacing::sm())
                                    .rounded(px(4.0))
                                    .border_1()
                                    .cursor_pointer()
                                    .when(
                                        default_sort_mode == SortModeWrapper::ModifiedDate,
                                        |div| div.border_color(Colors::info()).bg(rgba(0x50fa7b22)),
                                    )
                                    .when(
                                        default_sort_mode != SortModeWrapper::ModifiedDate,
                                        |div| div.border_color(rgb(0x444444)),
                                    )
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            |this, _event: &MouseDownEvent, _window, cx| {
                                                this.working_settings
                                                    .sort_navigation
                                                    .default_sort_mode =
                                                    SortModeWrapper::ModifiedDate;
                                                cx.notify();
                                            },
                                        ),
                                    )
                                    .child("Modified Date"),
                            ),
                    ),
            )
            .child(self.render_toggle_row(
                Some("Navigate from last image to first (and vice versa)".to_string()),
                &self.wrap_navigation_toggle,
            ))
            .child(self.render_toggle_row(
                Some("Display image position in window title".to_string()),
                &self.show_image_counter_toggle,
            ))
    }

    /// Render external tools section
    fn render_external_tools(&self) -> impl IntoElement {
        let external_viewers = self
            .working_settings
            .external_tools
            .external_viewers
            .clone();
        let external_editor = self.working_settings.external_tools.external_editor.clone();

        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("External Tools".to_string()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label(
                        "External viewers".to_string(),
                        Some(
                            "External applications to open images (in priority order)".to_string(),
                        ),
                    ))
                    .child(
                        div()
                            .px(Spacing::md())
                            .py(Spacing::md())
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(rgb(0x444444))
                            .rounded(px(4.0))
                            .max_h(px(200.0))
                            .child(
                                scrollable_vertical(
                                    div()
                                        .when(external_viewers.is_empty(), |el| {
                                            el.text_size(TextSize::sm())
                                                .text_color(rgb(0xaaaaaa))
                                                .child("No external viewers configured")
                                        })
                                        .when(!external_viewers.is_empty(), |el| {
                                            el.children(external_viewers.iter().enumerate().map(
                                                |(i, viewer)| {
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .justify_between()
                                                        .mb(Spacing::sm())
                                                        .px(Spacing::sm())
                                                        .py(Spacing::xs())
                                                        .bg(rgb(0x353535))
                                                        .rounded(px(4.0))
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .flex_col()
                                                                .child(
                                                                    div()
                                                                        .text_size(TextSize::sm())
                                                                        .text_color(
                                                                            if viewer.enabled {
                                                                                Colors::text()
                                                                            } else {
                                                                                rgb(0x666666).into()
                                                                            },
                                                                        )
                                                                        .child(format!(
                                                                            "{}. {}",
                                                                            i + 1,
                                                                            viewer.name
                                                                        )),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(TextSize::sm())
                                                                        .text_color(rgb(0x888888))
                                                                        .child(format!(
                                                                            "{} {}",
                                                                            viewer.command,
                                                                            viewer.args.join(" ")
                                                                        )),
                                                                ),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(TextSize::sm())
                                                                .text_color(if viewer.enabled {
                                                                    Colors::info()
                                                                } else {
                                                                    rgb(0x666666).into()
                                                                })
                                                                .child(if viewer.enabled {
                                                                    "✓ Enabled"
                                                                } else {
                                                                    "✗ Disabled"
                                                                }),
                                                        )
                                                },
                                            ))
                                        }),
                                )
                                .id("external-viewers-scroll"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label(
                        "External editor".to_string(),
                        Some("Application to edit images".to_string()),
                    ))
                    .child(
                        div()
                            .px(Spacing::sm())
                            .py(Spacing::xs())
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(rgb(0x444444))
                            .rounded(px(4.0))
                            .text_size(TextSize::sm())
                            .text_color(rgb(0xaaaaaa))
                            .child(
                                external_editor
                                    .as_ref()
                                    .map(|e| format!("{} {}", e.command, e.args.join(" ")))
                                    .unwrap_or_else(|| "Not configured".to_string()),
                            ),
                    ),
            )
            .child(self.render_toggle_row(
                Some("Show 'Reveal in Finder/Explorer' menu option".to_string()),
                &self.file_manager_integration_toggle,
            ))
    }

    /// Render settings file section
    fn render_settings_file(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_path = settings_io::get_settings_path();
        let path_str = settings_path.display().to_string();

        div()
            .flex()
            .flex_col()
            .max_w_full() // Ensure we don't exceed parent width
            .child(self.render_section_header("Settings File".to_string()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .max_w_full()
                    .child(self.render_label(
                        "Settings file location".to_string(),
                        Some("Path to the JSON settings file".to_string()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(Spacing::sm())
                            .max_w_full()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0() // Allow flex child to shrink below content size
                                    .px(Spacing::sm())
                                    .py(Spacing::xs())
                                    .bg(rgb(0x2a2a2a))
                                    .border_1()
                                    .border_color(rgb(0x444444))
                                    .rounded(px(4.0))
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .line_height(relative(1.3))
                                    .cursor(gpui::CursorStyle::IBeam)
                                    .child(div().id("settings-path-text").child(path_str.clone())),
                            )
                            .child(
                                div()
                                    .flex_shrink_0() // Prevent button from shrinking
                                    .px(Spacing::md())
                                    .py(Spacing::xs())
                                    .bg(rgb(0x444444))
                                    .rounded(px(4.0))
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            move |_this, _event: &MouseDownEvent, _window, cx| {
                                                // Copy path to clipboard
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    path_str.clone(),
                                                ));
                                            },
                                        ),
                                    )
                                    .child("Copy"),
                            ),
                    ),
            )
            .child(
                div()
                    .mb(Spacing::md())
                    .flex()
                    .flex_row()
                    .gap(Spacing::md())
                    .child(
                        div()
                            .px(Spacing::md())
                            .py(Spacing::sm())
                            .bg(Colors::info())
                            .rounded(px(4.0))
                            .text_size(TextSize::sm())
                            .text_color(rgb(0x000000))
                            .font_weight(FontWeight::BOLD)
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_this, _event: &MouseDownEvent, _window, _cx| {
                                    // Reveal settings file in file manager
                                    let path = settings_io::get_settings_path();
                                    #[cfg(target_os = "macos")]
                                    {
                                        std::process::Command::new("open")
                                            .arg("-R")
                                            .arg(&path)
                                            .spawn()
                                            .ok();
                                    }
                                    #[cfg(target_os = "windows")]
                                    {
                                        std::process::Command::new("explorer")
                                            .arg("/select,")
                                            .arg(&path)
                                            .spawn()
                                            .ok();
                                    }
                                    #[cfg(target_os = "linux")]
                                    {
                                        // Try to get the parent directory
                                        if let Some(parent) = path.parent() {
                                            std::process::Command::new("xdg-open")
                                                .arg(parent)
                                                .spawn()
                                                .ok();
                                        }
                                    }
                                }),
                            )
                            .child({
                                #[cfg(target_os = "macos")]
                                {
                                    "Reveal settings file in Finder"
                                }
                                #[cfg(target_os = "windows")]
                                {
                                    "Reveal settings file in File Explorer"
                                }
                                #[cfg(target_os = "linux")]
                                {
                                    "Reveal settings file in file manager"
                                }
                            }),
                    )
                    .child(
                        div()
                            .px(Spacing::md())
                            .py(Spacing::sm())
                            .bg(rgb(0x884444))
                            .rounded(px(4.0))
                            .text_size(TextSize::sm())
                            .text_color(Colors::text())
                            .font_weight(FontWeight::BOLD)
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                    this.reset_to_defaults(cx);
                                    cx.notify();
                                    // Dispatch the action to parent using window context
                                    window
                                        .dispatch_action(ResetSettingsToDefaults.boxed_clone(), cx);
                                }),
                            )
                            .child("Reset all settings to Defaults"),
                    ),
            )
            .child(
                div()
                    .max_w_full()
                    .px(Spacing::md())
                    .py(Spacing::md())
                    .bg(rgba(0x50fa7b22))
                    .border_1()
                    .border_color(Colors::info())
                    .rounded(px(4.0))
                    .text_size(TextSize::sm())
                    .text_color(Colors::text())
                    .line_height(relative(1.4))
                    .child(
                        div()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .mb(Spacing::xs())
                                    .child("Note:"),
                            )
                            .child("Edit settings.json to configure advanced options."),
                    ),
            )
    }

    /// Render the content area based on selected section
    fn render_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex_1().child(
            scrollable_vertical(div().p(Spacing::xl()).child(match self.current_section {
                SettingsSection::ViewerBehavior => {
                    self.render_viewer_behavior().into_any_element()
                }
                SettingsSection::Performance => self.render_performance().into_any_element(),
                SettingsSection::KeyboardMouse => self.render_keyboard_mouse().into_any_element(),
                SettingsSection::FileOperations => {
                    self.render_file_operations(cx).into_any_element()
                }
                SettingsSection::Appearance => {
                    self.render_appearance(window, cx).into_any_element()
                }
                SettingsSection::SortNavigation => {
                    self.render_sort_navigation(cx).into_any_element()
                }
                SettingsSection::ExternalTools => self.render_external_tools().into_any_element(),
                SettingsSection::SettingsFile => self.render_settings_file(cx).into_any_element(),
            }))
            .id("settings-content-scroll"),
        )
    }

    /// Render the footer with buttons
    fn render_footer(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let platform_key = if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        };

        div()
            .px(Spacing::xl())
            .pb(Spacing::xl())
            .pt(Spacing::md())
            .border_t_1()
            .border_color(rgb(0x444444))
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .child(
                div()
                    .text_size(TextSize::sm())
                    .text_color(rgb(0xaaaaaa))
                    .child(format!("{}-Enter or Esc to close and save", platform_key)),
            )
            .child(
                div()
                    .px(Spacing::lg())
                    .py(Spacing::sm())
                    .bg(Colors::info())
                    .rounded(px(4.0))
                    .text_size(TextSize::md())
                    .text_color(rgb(0x000000))
                    .font_weight(FontWeight::BOLD)
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this, _event: &MouseDownEvent, window, cx| {
                            // Dispatch action to parent App to save and close
                            window.dispatch_action(CloseSettings.boxed_clone(), cx);
                        }),
                    )
                    .child("Close"),
            )
    }
}

impl Focusable for SettingsWindow {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            // Full screen overlay with semi-transparent background
            .absolute()
            .inset_0()
            .bg(Colors::overlay_bg_alpha(
                self.working_settings.appearance.overlay_transparency,
            ))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.focus_handle)
            .child(
                // Settings window box
                div()
                    .bg(rgb(0x1e1e1e))
                    .border_1()
                    .border_color(rgb(0x444444))
                    .rounded(px(8.0))
                    .w(px(900.0))
                    .h(px(700.0))
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .child(self.render_header())
                    .child(
                        // Main content area with sidebar and content
                        div()
                            .flex_1()
                            .flex()
                            .flex_row()
                            .overflow_hidden()
                            .child(self.render_sidebar(cx))
                            .child(self.render_content(window, cx)),
                    )
                    .child(self.render_footer(cx)),
            )
    }
}
