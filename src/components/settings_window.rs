//! Settings window component for rpview-gpui
//! 
//! Provides a full-screen overlay with a settings panel allowing users to
//! view all application settings.
//!
//! ## Current Status: READ-ONLY (Display-Only)
//!
//! **Important:** The settings window currently displays settings values but
//! does not provide interactive controls to edit them. Users must manually
//! edit the `settings.json` file to change settings:
//!
//! - macOS: `~/Library/Application Support/rpview/settings.json`
//! - Linux: `~/.config/rpview/settings.json`
//! - Windows: `C:\Users\<User>\AppData\Roaming\rpview\settings.json`
//!
//! ## Making the UI Interactive
//!
//! To enable in-app editing, the following needs to be implemented:
//!
//! 1. **Add event handlers** to UI controls (checkboxes, radio buttons, inputs)
//! 2. **Update working_settings** field when users interact with controls
//! 3. **Add input validation** for numeric fields and paths
//! 4. **Wire up Apply button** to save working_settings to disk
//! 5. **Add text input components** for numeric and string fields
//!
//! See TODO.md Phase 16.7 for detailed implementation tasks.
//!
//! ## Architecture
//!
//! The component maintains two copies of settings:
//! - `working_settings`: Current edits (not yet saved)
//! - `original_settings`: Original values (for Cancel/revert)
//!
//! Apply/Cancel/Reset handlers exist in main.rs but working_settings is
//! never modified since controls are not interactive.

use adabraka_ui::prelude::scrollable_vertical;
use gpui::prelude::*;
use gpui::*;
use crate::state::settings::*;
use crate::utils::style::{Colors, Spacing, TextSize};

/// Available settings sections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    ViewerBehavior,
    Performance,
    KeyboardMouse,
    FileOperations,
    Appearance,
    Filters,
    SortNavigation,
    ExternalTools,
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
            Self::Filters => "Filters",
            Self::SortNavigation => "Sort & Navigation",
            Self::ExternalTools => "External Tools",
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
            Self::Filters,
            Self::SortNavigation,
            Self::ExternalTools,
        ]
    }
}

/// Settings window component
pub struct SettingsWindow {
    /// Working copy of settings (being edited)
    pub working_settings: AppSettings,
    /// Original settings (for cancel/revert)
    pub original_settings: AppSettings,
    /// Currently selected section
    pub current_section: SettingsSection,
    /// Focus handle for the settings window
    focus_handle: FocusHandle,
}

impl SettingsWindow {
    /// Create a new settings window with the given settings
    pub fn new(settings: AppSettings, cx: &mut Context<Self>) -> Self {
        Self {
            working_settings: settings.clone(),
            original_settings: settings,
            current_section: SettingsSection::ViewerBehavior,
            focus_handle: cx.focus_handle(),
        }
    }
    
    /// Change the current section
    pub fn set_section(&mut self, section: SettingsSection, cx: &mut Context<Self>) {
        self.current_section = section;
        cx.notify();
    }

    /// Reset working settings to original
    pub fn cancel(&mut self) {
        self.working_settings = self.original_settings.clone();
    }

    /// Reset all settings to defaults
    pub fn reset_to_defaults(&mut self) {
        self.working_settings = AppSettings::default();
    }

    /// Get the final settings (for apply)
    pub fn get_settings(&self) -> AppSettings {
        self.working_settings.clone()
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
            .children(
                SettingsSection::all().into_iter().map(move |section| {
                    let is_selected = section == current;
                    div()
                        .px(Spacing::md())
                        .py(Spacing::sm())
                        .mb(Spacing::xs())
                        .rounded(px(4.0))
                        .when(is_selected, |div| {
                            div.bg(rgb(0x444444))
                                .text_color(Colors::info())
                        })
                        .when(!is_selected, |div| {
                            div.bg(rgb(0x2a2a2a))
                                .text_color(Colors::text())
                        })
                        .text_size(TextSize::md())
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                            this.set_section(section, cx);
                        }))
                        .child(section.name())
                })
            )
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
                    .child(text)
            )
            .when_some(description, |el, desc| {
                el.child(
                    div()
                        .text_size(TextSize::sm())
                        .text_color(rgb(0xaaaaaa))
                        .child(desc)
                )
            })
    }

    /// Render a checkbox setting
    fn render_checkbox(&self, label: String, value: bool, description: Option<String>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .mb(Spacing::md())
            .child(self.render_label(label, description))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .border_1()
                            .border_color(rgb(0x666666))
                            .rounded(px(3.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(value, |div| {
                                div.bg(Colors::info())
                                    .child("✓")
                            })
                    )
                    .child(
                        div()
                            .ml(Spacing::sm())
                            .text_size(TextSize::sm())
                            .text_color(rgb(0xaaaaaa))
                            .child(if value { "Enabled" } else { "Disabled" })
                    )
            )
    }

    /// Render a numeric input
    fn render_numeric_input(&self, label: String, value: String, description: Option<String>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .mb(Spacing::md())
            .child(self.render_label(label, description))
            .child(
                div()
                    .w(px(200.0))
                    .px(Spacing::sm())
                    .py(Spacing::xs())
                    .bg(rgb(0x2a2a2a))
                    .border_1()
                    .border_color(rgb(0x444444))
                    .rounded(px(4.0))
                    .text_size(TextSize::md())
                    .text_color(Colors::text())
                    .child(value)
            )
    }

    /// Render viewer behavior section
    fn render_viewer_behavior(&self) -> impl IntoElement {
        let settings = &self.working_settings.viewer_behavior;
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Viewer Behavior".to_string()))
            .child(
                div()
                    .mb(Spacing::md())
                    .child(self.render_label("Default Zoom Mode".to_string(), Some("How images are initially displayed".to_string())))
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
                                    .when(settings.default_zoom_mode == ZoomMode::FitToWindow, |div| {
                                        div.border_color(Colors::info())
                                            .bg(rgba(0x50fa7b22))
                                    })
                                    .when(settings.default_zoom_mode != ZoomMode::FitToWindow, |div| {
                                        div.border_color(rgb(0x444444))
                                    })
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .child("Fit to Window")
                            )
                            .child(
                                div()
                                    .px(Spacing::md())
                                    .py(Spacing::sm())
                                    .rounded(px(4.0))
                                    .border_1()
                                    .when(settings.default_zoom_mode == ZoomMode::OneHundredPercent, |div| {
                                        div.border_color(Colors::info())
                                            .bg(rgba(0x50fa7b22))
                                    })
                                    .when(settings.default_zoom_mode != ZoomMode::OneHundredPercent, |div| {
                                        div.border_color(rgb(0x444444))
                                    })
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .child("100% (Actual Size)")
                            )
                    )
            )
            .child(self.render_checkbox(
                "Remember per-image state".to_string(),
                settings.remember_per_image_state,
                Some("Remember zoom, pan, and filters for each image".to_string())
            ))
            .child(self.render_numeric_input(
                "State cache size".to_string(),
                settings.state_cache_size.to_string(),
                Some("Maximum number of images to cache state for".to_string())
            ))
            .child(self.render_checkbox(
                "Auto-play animations".to_string(),
                settings.animation_auto_play,
                Some("Start animated GIFs/WEBPs playing automatically".to_string())
            ))
    }

    /// Render performance section
    fn render_performance(&self) -> impl IntoElement {
        let settings = &self.working_settings.performance;
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Performance".to_string()))
            .child(self.render_checkbox(
                "Preload adjacent images".to_string(),
                settings.preload_adjacent_images,
                Some("Load next/previous images in background for faster navigation".to_string())
            ))
            .child(self.render_numeric_input(
                "Filter processing threads".to_string(),
                settings.filter_processing_threads.to_string(),
                Some("Number of CPU threads for filter processing".to_string())
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label("Maximum image dimensions".to_string(), Some("Safety limit for loading images".to_string())))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(Spacing::sm())
                            .child(
                                div()
                                    .w(px(100.0))
                                    .px(Spacing::sm())
                                    .py(Spacing::xs())
                                    .bg(rgb(0x2a2a2a))
                                    .border_1()
                                    .border_color(rgb(0x444444))
                                    .rounded(px(4.0))
                                    .text_size(TextSize::md())
                                    .text_color(Colors::text())
                                    .child(format!("{}px", settings.max_image_dimensions.0))
                            )
                            .child(
                                div()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("×")
                            )
                            .child(
                                div()
                                    .w(px(100.0))
                                    .px(Spacing::sm())
                                    .py(Spacing::xs())
                                    .bg(rgb(0x2a2a2a))
                                    .border_1()
                                    .border_color(rgb(0x444444))
                                    .rounded(px(4.0))
                                    .text_size(TextSize::md())
                                    .text_color(Colors::text())
                                    .child(format!("{}px", settings.max_image_dimensions.1))
                            )
                    )
            )
    }

    /// Render keyboard & mouse section
    fn render_keyboard_mouse(&self) -> impl IntoElement {
        let settings = &self.working_settings.keyboard_mouse;
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Keyboard & Mouse".to_string()))
            .child(self.render_numeric_input(
                "Pan speed (normal)".to_string(),
                format!("{:.1} px", settings.pan_speed_normal),
                Some("Base keyboard pan speed in pixels".to_string())
            ))
            .child(self.render_numeric_input(
                "Pan speed (fast, with Shift)".to_string(),
                format!("{:.1} px", settings.pan_speed_fast),
                Some("Pan speed with Shift modifier".to_string())
            ))
            .child(self.render_numeric_input(
                "Pan speed (slow, with Cmd/Ctrl)".to_string(),
                format!("{:.1} px", settings.pan_speed_slow),
                Some("Pan speed with Cmd/Ctrl modifier".to_string())
            ))
            .child(self.render_numeric_input(
                "Scroll wheel sensitivity".to_string(),
                format!("{:.2}x", settings.scroll_wheel_sensitivity),
                Some("Zoom factor per scroll wheel notch".to_string())
            ))
            .child(self.render_numeric_input(
                "Z-drag zoom sensitivity".to_string(),
                format!("{:.3}", settings.z_drag_sensitivity),
                Some("Zoom percentage change per pixel when Z-dragging".to_string())
            ))
            .child(self.render_checkbox(
                "Spacebar pan acceleration".to_string(),
                settings.spacebar_pan_accelerated,
                Some("Enable acceleration for spacebar+mouse panning".to_string())
            ))
    }

    /// Render file operations section
    fn render_file_operations(&self) -> impl IntoElement {
        let settings = &self.working_settings.file_operations;
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("File Operations".to_string()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label("Default save directory".to_string(), Some("Where filtered images are saved by default".to_string())))
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
                                        settings.default_save_directory
                                            .as_ref()
                                            .map(|p| p.display().to_string())
                                            .unwrap_or_else(|| "Same as current image".to_string())
                                    )
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
                                    .child("Browse...")
                            )
                    )
            )
            .child(
                div()
                    .mb(Spacing::md())
                    .child(self.render_label("Default save format".to_string(), Some("Format for saving filtered images".to_string())))
                    .child(
                        div()
                            .w(px(150.0))
                            .px(Spacing::sm())
                            .py(Spacing::xs())
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(rgb(0x444444))
                            .rounded(px(4.0))
                            .text_size(TextSize::md())
                            .text_color(Colors::text())
                            .child(match settings.default_save_format {
                                SaveFormat::Png => "PNG",
                                SaveFormat::Jpeg => "JPEG",
                                SaveFormat::Bmp => "BMP",
                                SaveFormat::Tiff => "TIFF",
                                SaveFormat::Webp => "WEBP",
                            })
                    )
            )
            .child(self.render_checkbox(
                "Auto-save filtered cache".to_string(),
                settings.auto_save_filtered_cache,
                Some("Permanently save filtered image cache to disk".to_string())
            ))
            .child(self.render_checkbox(
                "Remember last directory".to_string(),
                settings.remember_last_directory,
                Some("Remember last used directory in file dialogs".to_string())
            ))
    }

    /// Render appearance section
    fn render_appearance(&self) -> impl IntoElement {
        let settings = &self.working_settings.appearance;
        
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
                            .flex_row()
                            .gap(Spacing::sm())
                            .items_center()
                            .child(
                                div()
                                    .w(px(40.0))
                                    .h(px(40.0))
                                    .rounded(px(4.0))
                                    .border_1()
                                    .border_color(rgb(0x666666))
                                    .bg(rgb(
                                        ((settings.background_color[0] as u32) << 16) |
                                        ((settings.background_color[1] as u32) << 8) |
                                        (settings.background_color[2] as u32)
                                    ))
                            )
                            .child(
                                div()
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0xaaaaaa))
                                    .child(format!(
                                        "#{:02x}{:02x}{:02x}",
                                        settings.background_color[0],
                                        settings.background_color[1],
                                        settings.background_color[2]
                                    ))
                            )
                    )
            )
            .child(self.render_numeric_input(
                "Overlay transparency".to_string(),
                format!("{}", settings.overlay_transparency),
                Some("Transparency for overlay backgrounds (0-255)".to_string())
            ))
            .child(self.render_numeric_input(
                "Font size scale".to_string(),
                format!("{:.1}x", settings.font_size_scale),
                Some("Scale factor for overlay text (0.5 - 2.0)".to_string())
            ))
            .child(self.render_numeric_input(
                "Window title format".to_string(),
                settings.window_title_format.clone(),
                Some("Template: {filename}, {index}, {total}".to_string())
            ))
    }

    /// Render filters section
    fn render_filters(&self) -> impl IntoElement {
        let settings = &self.working_settings.filters;
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Filters".to_string()))
            .child(self.render_numeric_input(
                "Default brightness".to_string(),
                format!("{:.0}", settings.default_brightness),
                Some("Default brightness value when resetting (-100 to +100)".to_string())
            ))
            .child(self.render_numeric_input(
                "Default contrast".to_string(),
                format!("{:.0}", settings.default_contrast),
                Some("Default contrast value when resetting (-100 to +100)".to_string())
            ))
            .child(self.render_numeric_input(
                "Default gamma".to_string(),
                format!("{:.2}", settings.default_gamma),
                Some("Default gamma value when resetting (0.1 to 10.0)".to_string())
            ))
            .child(self.render_checkbox(
                "Remember filter state per-image".to_string(),
                settings.remember_filter_state,
                Some("Remember filter settings for each image separately".to_string())
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label("Filter presets".to_string(), Some("Saved filter combinations".to_string())))
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
                            .when(settings.filter_presets.is_empty(), |el| {
                                el.child("No presets saved")
                            })
                            .when(!settings.filter_presets.is_empty(), |el| {
                                el.children(
                                    settings.filter_presets.iter().map(|preset| {
                                        div()
                                            .mb(Spacing::xs())
                                            .child(preset.name.clone())
                                    })
                                )
                            })
                    )
            )
    }

    /// Render sort & navigation section
    fn render_sort_navigation(&self) -> impl IntoElement {
        let settings = &self.working_settings.sort_navigation;
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Sort & Navigation".to_string()))
            .child(
                div()
                    .mb(Spacing::md())
                    .child(self.render_label("Default sort mode".to_string(), Some("How images are sorted on startup".to_string())))
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
                                    .when(settings.default_sort_mode == SortModeWrapper::Alphabetical, |div| {
                                        div.border_color(Colors::info())
                                            .bg(rgba(0x50fa7b22))
                                    })
                                    .when(settings.default_sort_mode != SortModeWrapper::Alphabetical, |div| {
                                        div.border_color(rgb(0x444444))
                                    })
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .child("Alphabetical")
                            )
                            .child(
                                div()
                                    .px(Spacing::md())
                                    .py(Spacing::sm())
                                    .rounded(px(4.0))
                                    .border_1()
                                    .when(settings.default_sort_mode == SortModeWrapper::ModifiedDate, |div| {
                                        div.border_color(Colors::info())
                                            .bg(rgba(0x50fa7b22))
                                    })
                                    .when(settings.default_sort_mode != SortModeWrapper::ModifiedDate, |div| {
                                        div.border_color(rgb(0x444444))
                                    })
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .child("Modified Date")
                            )
                    )
            )
            .child(self.render_checkbox(
                "Wrap navigation".to_string(),
                settings.wrap_navigation,
                Some("Navigate from last image to first (and vice versa)".to_string())
            ))
            .child(self.render_checkbox(
                "Show image counter".to_string(),
                settings.show_image_counter,
                Some("Display image position in window title".to_string())
            ))
    }

    /// Render external tools section
    fn render_external_tools(&self) -> impl IntoElement {
        let settings = &self.working_settings.external_tools;
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("External Tools".to_string()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label("External viewers".to_string(), Some("External applications to open images (in priority order)".to_string())))
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
                                        .when(settings.external_viewers.is_empty(), |el| {
                                            el.text_size(TextSize::sm())
                                                .text_color(rgb(0xaaaaaa))
                                                .child("No external viewers configured")
                                        })
                                        .when(!settings.external_viewers.is_empty(), |el| {
                                            el.children(
                                                settings.external_viewers.iter().enumerate().map(|(i, viewer)| {
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
                                                                        .text_color(if viewer.enabled { Colors::text() } else { rgb(0x666666).into() })
                                                                        .child(format!("{}. {}", i + 1, viewer.name))
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(TextSize::sm())
                                                                        .text_color(rgb(0x888888))
                                                                        .child(format!("{} {}", viewer.command, viewer.args.join(" ")))
                                                                )
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(TextSize::sm())
                                                                .text_color(if viewer.enabled { Colors::info() } else { rgb(0x666666).into() })
                                                                .child(if viewer.enabled { "✓ Enabled" } else { "✗ Disabled" })
                                                        )
                                                })
                                            )
                                        })
                                )
                                .id("external-viewers-scroll")
                            )
                    )
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label("External editor".to_string(), Some("Application to edit images".to_string())))
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
                                settings.external_editor
                                    .as_ref()
                                    .map(|e| format!("{} {}", e.command, e.args.join(" ")))
                                    .unwrap_or_else(|| "Not configured".to_string())
                            )
                    )
            )
            .child(self.render_checkbox(
                "File manager integration".to_string(),
                settings.enable_file_manager_integration,
                Some("Show 'Reveal in Finder/Explorer' menu option".to_string())
            ))
    }

    /// Render the content area based on selected section
    fn render_content(&self) -> impl IntoElement {
        div()
            .flex_1()
            .child(
                scrollable_vertical(
                    div()
                        .p(Spacing::xl())
                        .child(match self.current_section {
                            SettingsSection::ViewerBehavior => self.render_viewer_behavior().into_any_element(),
                            SettingsSection::Performance => self.render_performance().into_any_element(),
                            SettingsSection::KeyboardMouse => self.render_keyboard_mouse().into_any_element(),
                            SettingsSection::FileOperations => self.render_file_operations().into_any_element(),
                            SettingsSection::Appearance => self.render_appearance().into_any_element(),
                            SettingsSection::Filters => self.render_filters().into_any_element(),
                            SettingsSection::SortNavigation => self.render_sort_navigation().into_any_element(),
                            SettingsSection::ExternalTools => self.render_external_tools().into_any_element(),
                        })
                )
                .id("settings-content-scroll")
            )
    }

    /// Render the footer with buttons
    fn render_footer(&self) -> impl IntoElement {
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
                    .child(format!("Press {}-comma or Esc to close", platform_key))
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(Spacing::md())
                    .child(
                        div()
                            .px(Spacing::lg())
                            .py(Spacing::sm())
                            .bg(rgb(0x444444))
                            .rounded(px(4.0))
                            .text_size(TextSize::md())
                            .text_color(Colors::text())
                            .cursor_pointer()
                            .child("Reset to Defaults")
                    )
                    .child(
                        div()
                            .px(Spacing::lg())
                            .py(Spacing::sm())
                            .bg(rgb(0x444444))
                            .rounded(px(4.0))
                            .text_size(TextSize::md())
                            .text_color(Colors::text())
                            .cursor_pointer()
                            .child("Cancel")
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
                            .child("Apply")
                    )
            )
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            // Full screen overlay with semi-transparent background
            .absolute()
            .inset_0()
            .bg(rgba(0x00000099))
            .flex()
            .items_center()
            .justify_center()
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
                            .child(self.render_content())
                    )
                    .child(self.render_footer())
            )
    }
}
