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
//! - ✅ **Checkboxes**: Toggle boolean settings (10+ settings including animation auto-play, pan acceleration, etc.)
//! - ✅ **Radio buttons**: Select enum values (zoom mode, sort mode, save format)
//! - ✅ **Numeric inputs**: Increment/decrement buttons for all numeric settings (15+ settings)
//!   - Pan speeds (normal, fast, slow)
//!   - Zoom sensitivities (scroll wheel, Z-drag)
//!   - Cache sizes and thread counts
//!   - Filter defaults (brightness, contrast, gamma)
//!   - Appearance settings (transparency, font scale)
//! - ✅ **Range validation**: All numeric values are clamped to valid ranges
//! - ✅ **Apply/Cancel/Reset**: Keyboard shortcuts (Cmd+Enter to apply, Esc to cancel)
//! - ✅ **Settings persistence**: Changes are saved to JSON on Apply
//!
//! ### Pending Features (Low Priority):
//! - ⏳ **Text inputs**: Window title format and file paths still require JSON editing
//! - ⏳ **Color picker**: Background color requires JSON editing
//! - ⏳ **File browser**: Default save directory requires JSON editing
//! - ⏳ **External viewer list editor**: Add/remove/reorder viewers requires JSON editing
//!
//! ### Keyboard Shortcuts:
//! - `Cmd+,` (or `Ctrl+,` on Windows/Linux): Open/close settings
//! - `Cmd+Enter`: Apply changes and close
//! - `Esc`: Cancel changes and close
//!
//! ### Settings File Location:
//! - macOS: `~/Library/Application Support/rpview/settings.json`
//! - Linux: `~/.config/rpview/settings.json`
//! - Windows: `C:\Users\<User>\AppData\Roaming\rpview\settings.json`
//!
//! ## Architecture
//!
//! The component maintains two copies of settings:
//! - `working_settings`: Current edits (modified by UI interactions)
//! - `original_settings`: Original values (for Cancel/revert)

use adabraka_ui::prelude::scrollable_vertical;
use gpui::prelude::*;
use gpui::*;
use crate::state::settings::*;
use crate::utils::style::{Colors, Spacing, TextSize};
use crate::utils::settings_io;
use crate::{ApplySettings, CancelSettings, ResetSettingsToDefaults};

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
            Self::Filters => "Filters",
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
            Self::Filters,
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
    fn render_checkbox<F>(&mut self, label: String, value: bool, description: Option<String>, on_toggle: F, cx: &mut Context<Self>) -> impl IntoElement 
    where
        F: Fn(&mut SettingsWindow, &mut Context<Self>) + 'static,
    {
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
                            .cursor_pointer()
                            .when(value, |div| {
                                div.bg(Colors::info())
                                    .child("✓")
                            })
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                                on_toggle(this, cx);
                                cx.notify();
                            }))
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

    /// Render a numeric input with increment/decrement buttons
    fn render_numeric_input(
        &mut self,
        label: String,
        value: String,
        description: Option<String>,
        on_increment: impl Fn(&mut SettingsWindow, &mut Context<Self>) + 'static + Clone,
        on_decrement: impl Fn(&mut SettingsWindow, &mut Context<Self>) + 'static + Clone,
        cx: &mut Context<Self>
    ) -> impl IntoElement
    {
        let increment_handler = on_increment.clone();
        let decrement_handler = on_decrement;
        
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
                    .gap(Spacing::xs())
                    .child(
                        // Decrement button
                        div()
                            .px(Spacing::sm())
                            .py(Spacing::xs())
                            .bg(rgb(0x444444))
                            .border_1()
                            .border_color(rgb(0x666666))
                            .rounded(px(4.0))
                            .text_size(TextSize::md())
                            .text_color(Colors::text())
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                                decrement_handler(this, cx);
                                cx.notify();
                            }))
                            .child("−")
                    )
                    .child(
                        // Value display
                        div()
                            .w(px(120.0))
                            .px(Spacing::sm())
                            .py(Spacing::xs())
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(rgb(0x444444))
                            .rounded(px(4.0))
                            .text_size(TextSize::md())
                            .text_color(Colors::text())
                            .text_align(gpui::TextAlign::Center)
                            .child(value)
                    )
                    .child(
                        // Increment button
                        div()
                            .px(Spacing::sm())
                            .py(Spacing::xs())
                            .bg(rgb(0x444444))
                            .border_1()
                            .border_color(rgb(0x666666))
                            .rounded(px(4.0))
                            .text_size(TextSize::md())
                            .text_color(Colors::text())
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                                increment_handler(this, cx);
                                cx.notify();
                            }))
                            .child("+")
                    )
            )
    }

    /// Render viewer behavior section
    fn render_viewer_behavior(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // Copy values needed for rendering to avoid borrow checker issues
        let default_zoom_mode = self.working_settings.viewer_behavior.default_zoom_mode;
        let remember_per_image_state = self.working_settings.viewer_behavior.remember_per_image_state;
        let state_cache_size = self.working_settings.viewer_behavior.state_cache_size;
        let animation_auto_play = self.working_settings.viewer_behavior.animation_auto_play;
        
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
                                    .cursor_pointer()
                                    .when(default_zoom_mode == ZoomMode::FitToWindow, |div| {
                                        div.border_color(Colors::info())
                                            .bg(rgba(0x50fa7b22))
                                    })
                                    .when(default_zoom_mode != ZoomMode::FitToWindow, |div| {
                                        div.border_color(rgb(0x444444))
                                    })
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                        this.working_settings.viewer_behavior.default_zoom_mode = ZoomMode::FitToWindow;
                                        cx.notify();
                                    }))
                                    .child("Fit to Window")
                            )
                            .child(
                                div()
                                    .px(Spacing::md())
                                    .py(Spacing::sm())
                                    .rounded(px(4.0))
                                    .border_1()
                                    .cursor_pointer()
                                    .when(default_zoom_mode == ZoomMode::OneHundredPercent, |div| {
                                        div.border_color(Colors::info())
                                            .bg(rgba(0x50fa7b22))
                                    })
                                    .when(default_zoom_mode != ZoomMode::OneHundredPercent, |div| {
                                        div.border_color(rgb(0x444444))
                                    })
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                        this.working_settings.viewer_behavior.default_zoom_mode = ZoomMode::OneHundredPercent;
                                        cx.notify();
                                    }))
                                    .child("100% (Actual Size)")
                            )
                    )
            )
            .child(self.render_checkbox(
                "Remember per-image state".to_string(),
                remember_per_image_state,
                Some("Remember zoom, pan, and filters for each image".to_string()),
                |this, _cx| {
                    this.working_settings.viewer_behavior.remember_per_image_state = !this.working_settings.viewer_behavior.remember_per_image_state;
                },
                cx
            ))
            .child(self.render_numeric_input(
                "State cache size".to_string(),
                state_cache_size.to_string(),
                Some("Maximum number of images to cache state for".to_string()),
                |this, _cx| {
                    this.working_settings.viewer_behavior.state_cache_size = 
                        (this.working_settings.viewer_behavior.state_cache_size + 100).min(10000);
                },
                |this, _cx| {
                    this.working_settings.viewer_behavior.state_cache_size = 
                        this.working_settings.viewer_behavior.state_cache_size.saturating_sub(100).max(10);
                },
                cx
            ))
            .child(self.render_checkbox(
                "Auto-play animations".to_string(),
                animation_auto_play,
                Some("Start animated GIFs/WEBPs playing automatically".to_string()),
                |this, _cx| {
                    this.working_settings.viewer_behavior.animation_auto_play = !this.working_settings.viewer_behavior.animation_auto_play;
                },
                cx
            ))
    }

    /// Render performance section
    fn render_performance(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let preload_adjacent_images = self.working_settings.performance.preload_adjacent_images;
        let filter_processing_threads = self.working_settings.performance.filter_processing_threads;
        let max_image_dimension = self.working_settings.performance.max_image_dimension;
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Performance".to_string()))
            .child(self.render_checkbox(
                "Preload adjacent images".to_string(),
                preload_adjacent_images,
                Some("Load next/previous images in background for faster navigation".to_string()),
                |this, _cx| {
                    this.working_settings.performance.preload_adjacent_images = !this.working_settings.performance.preload_adjacent_images;
                },
                cx
            ))
            .child(self.render_numeric_input(
                "Filter processing threads".to_string(),
                filter_processing_threads.to_string(),
                Some("Number of CPU threads for filter processing".to_string()),
                |this, _cx| {
                    this.working_settings.performance.filter_processing_threads = 
                        (this.working_settings.performance.filter_processing_threads + 1).min(32);
                },
                |this, _cx| {
                    this.working_settings.performance.filter_processing_threads = 
                        this.working_settings.performance.filter_processing_threads.saturating_sub(1).max(1);
                },
                cx
            ))
            .child(self.render_numeric_input(
                "Maximum image dimension".to_string(),
                format!("{}px", max_image_dimension),
                Some("Maximum allowed width or height for loading images".to_string()),
                |this, _cx| {
                    this.working_settings.performance.max_image_dimension = 
                        (this.working_settings.performance.max_image_dimension + 1000).min(100000);
                },
                |this, _cx| {
                    this.working_settings.performance.max_image_dimension = 
                        this.working_settings.performance.max_image_dimension.saturating_sub(1000).max(1000);
                },
                cx
            ))
    }

    /// Render keyboard & mouse section
    fn render_keyboard_mouse(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let pan_speed_normal = self.working_settings.keyboard_mouse.pan_speed_normal;
        let pan_speed_fast = self.working_settings.keyboard_mouse.pan_speed_fast;
        let pan_speed_slow = self.working_settings.keyboard_mouse.pan_speed_slow;
        let scroll_wheel_sensitivity = self.working_settings.keyboard_mouse.scroll_wheel_sensitivity;
        let z_drag_sensitivity = self.working_settings.keyboard_mouse.z_drag_sensitivity;
        let spacebar_pan_accelerated = self.working_settings.keyboard_mouse.spacebar_pan_accelerated;
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Keyboard & Mouse".to_string()))
            .child(self.render_numeric_input(
                "Pan speed (normal)".to_string(),
                format!("{:.1} px", pan_speed_normal),
                Some("Base keyboard pan speed in pixels".to_string()),
                |this, _cx| {
                    this.working_settings.keyboard_mouse.pan_speed_normal = 
                        (this.working_settings.keyboard_mouse.pan_speed_normal + 1.0).min(100.0);
                },
                |this, _cx| {
                    this.working_settings.keyboard_mouse.pan_speed_normal = 
                        (this.working_settings.keyboard_mouse.pan_speed_normal - 1.0).max(1.0);
                },
                cx
            ))
            .child(self.render_numeric_input(
                "Pan speed (fast, with Shift)".to_string(),
                format!("{:.1} px", pan_speed_fast),
                Some("Pan speed with Shift modifier".to_string()),
                |this, _cx| {
                    this.working_settings.keyboard_mouse.pan_speed_fast = 
                        (this.working_settings.keyboard_mouse.pan_speed_fast + 5.0).min(200.0);
                },
                |this, _cx| {
                    this.working_settings.keyboard_mouse.pan_speed_fast = 
                        (this.working_settings.keyboard_mouse.pan_speed_fast - 5.0).max(1.0);
                },
                cx
            ))
            .child(self.render_numeric_input(
                "Pan speed (slow, with Cmd/Ctrl)".to_string(),
                format!("{:.1} px", pan_speed_slow),
                Some("Pan speed with Cmd/Ctrl modifier".to_string()),
                |this, _cx| {
                    this.working_settings.keyboard_mouse.pan_speed_slow = 
                        (this.working_settings.keyboard_mouse.pan_speed_slow + 0.5).min(50.0);
                },
                |this, _cx| {
                    this.working_settings.keyboard_mouse.pan_speed_slow = 
                        (this.working_settings.keyboard_mouse.pan_speed_slow - 0.5).max(0.5);
                },
                cx
            ))
            .child(self.render_numeric_input(
                "Scroll wheel sensitivity".to_string(),
                format!("{:.2}x", scroll_wheel_sensitivity),
                Some("Zoom factor per scroll wheel notch".to_string()),
                |this, _cx| {
                    this.working_settings.keyboard_mouse.scroll_wheel_sensitivity = 
                        (this.working_settings.keyboard_mouse.scroll_wheel_sensitivity + 0.05).min(2.0);
                },
                |this, _cx| {
                    this.working_settings.keyboard_mouse.scroll_wheel_sensitivity = 
                        (this.working_settings.keyboard_mouse.scroll_wheel_sensitivity - 0.05).max(1.01);
                },
                cx
            ))
            .child(self.render_numeric_input(
                "Z-drag zoom sensitivity".to_string(),
                format!("{:.3}", z_drag_sensitivity),
                Some("Zoom percentage change per pixel when Z-dragging".to_string()),
                |this, _cx| {
                    this.working_settings.keyboard_mouse.z_drag_sensitivity = 
                        (this.working_settings.keyboard_mouse.z_drag_sensitivity + 0.001).min(0.1);
                },
                |this, _cx| {
                    this.working_settings.keyboard_mouse.z_drag_sensitivity = 
                        (this.working_settings.keyboard_mouse.z_drag_sensitivity - 0.001).max(0.001);
                },
                cx
            ))
            .child(self.render_checkbox(
                "Spacebar pan acceleration".to_string(),
                spacebar_pan_accelerated,
                Some("Enable acceleration for spacebar+mouse panning".to_string()),
                |this, _cx| {
                    this.working_settings.keyboard_mouse.spacebar_pan_accelerated = !this.working_settings.keyboard_mouse.spacebar_pan_accelerated;
                },
                cx
            ))
    }

    /// Render file operations section
    fn render_file_operations(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let default_save_directory = self.working_settings.file_operations.default_save_directory.clone();
        let default_save_format = self.working_settings.file_operations.default_save_format;
        let auto_save_filtered_cache = self.working_settings.file_operations.auto_save_filtered_cache;
        let remember_last_directory = self.working_settings.file_operations.remember_last_directory;
        
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
                                        default_save_directory
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
            .child({
                let formats: Vec<_> = SaveFormat::all().into_iter().map(|format| {
                    let is_selected = format == default_save_format;
                    
                    div()
                        .px(Spacing::md())
                        .py(Spacing::sm())
                        .rounded(px(4.0))
                        .border_1()
                        .cursor_pointer()
                        .when(is_selected, |div| {
                            div.border_color(Colors::info())
                                .bg(rgba(0x50fa7b22))
                        })
                        .when(!is_selected, |div| {
                            div.border_color(rgb(0x444444))
                        })
                        .text_size(TextSize::sm())
                        .text_color(Colors::text())
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                            this.working_settings.file_operations.default_save_format = format;
                            cx.notify();
                        }))
                        .child(format.display_name())
                }).collect();
                
                div()
                    .mb(Spacing::md())
                    .child(self.render_label("Default save format".to_string(), Some("Format for saving filtered images".to_string())))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(Spacing::xs())
                            .children(formats)
                    )
            })
            .child(self.render_checkbox(
                "Auto-save filtered cache".to_string(),
                auto_save_filtered_cache,
                Some("Permanently save filtered image cache to disk".to_string()),
                |this, _cx| {
                    this.working_settings.file_operations.auto_save_filtered_cache = !this.working_settings.file_operations.auto_save_filtered_cache;
                },
                cx
            ))
            .child(self.render_checkbox(
                "Remember last directory".to_string(),
                remember_last_directory,
                Some("Remember last used directory in file dialogs".to_string()),
                |this, _cx| {
                    this.working_settings.file_operations.remember_last_directory = !this.working_settings.file_operations.remember_last_directory;
                },
                cx
            ))
    }

    /// Render appearance section
    fn render_appearance(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let background_color = self.working_settings.appearance.background_color;
        let overlay_transparency = self.working_settings.appearance.overlay_transparency;
        let font_size_scale = self.working_settings.appearance.font_size_scale;
        let window_title_format = self.working_settings.appearance.window_title_format.clone();
        
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
                                        ((background_color[0] as u32) << 16) |
                                        ((background_color[1] as u32) << 8) |
                                        (background_color[2] as u32)
                                    ))
                            )
                            .child(
                                div()
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
            .child(self.render_numeric_input(
                "Overlay transparency".to_string(),
                format!("{}", overlay_transparency),
                Some("Transparency for overlay backgrounds (0-255)".to_string()),
                |this, _cx| {
                    this.working_settings.appearance.overlay_transparency = 
                        this.working_settings.appearance.overlay_transparency.saturating_add(10).min(255);
                },
                |this, _cx| {
                    this.working_settings.appearance.overlay_transparency = 
                        this.working_settings.appearance.overlay_transparency.saturating_sub(10);
                },
                cx
            ))
            .child(self.render_numeric_input(
                "Font size scale".to_string(),
                format!("{:.1}x", font_size_scale),
                Some("Scale factor for overlay text (0.5 - 2.0)".to_string()),
                |this, _cx| {
                    this.working_settings.appearance.font_size_scale = 
                        (this.working_settings.appearance.font_size_scale + 0.1).min(2.0);
                },
                |this, _cx| {
                    this.working_settings.appearance.font_size_scale = 
                        (this.working_settings.appearance.font_size_scale - 0.1).max(0.5);
                },
                cx
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label("Window title format".to_string(), Some("Template: {filename}, {index}, {total}".to_string())))
                    .child(
                        div()
                            .px(Spacing::sm())
                            .py(Spacing::xs())
                            .bg(rgb(0x2a2a2a))
                            .border_1()
                            .border_color(rgb(0x444444))
                            .rounded(px(4.0))
                            .text_size(TextSize::md())
                            .text_color(rgb(0xaaaaaa))
                            .child(window_title_format.clone())
                            .child(
                                div()
                                    .text_size(TextSize::sm())
                                    .text_color(rgb(0x666666))
                                    .mt(Spacing::xs())
                                    .child("(Edit in settings JSON file)")
                            )
                    )
            )
    }

    /// Render filters section
    fn render_filters(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let default_brightness = self.working_settings.filters.default_brightness;
        let default_contrast = self.working_settings.filters.default_contrast;
        let default_gamma = self.working_settings.filters.default_gamma;
        let remember_filter_state = self.working_settings.filters.remember_filter_state;
        let filter_presets = self.working_settings.filters.filter_presets.clone();
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Filters".to_string()))
            .child(self.render_numeric_input(
                "Default brightness".to_string(),
                format!("{:.0}", default_brightness),
                Some("Default brightness value when resetting (-100 to +100)".to_string()),
                |this, _cx| {
                    this.working_settings.filters.default_brightness = 
                        (this.working_settings.filters.default_brightness + 5.0).min(100.0);
                },
                |this, _cx| {
                    this.working_settings.filters.default_brightness = 
                        (this.working_settings.filters.default_brightness - 5.0).max(-100.0);
                },
                cx
            ))
            .child(self.render_numeric_input(
                "Default contrast".to_string(),
                format!("{:.0}", default_contrast),
                Some("Default contrast value when resetting (-100 to +100)".to_string()),
                |this, _cx| {
                    this.working_settings.filters.default_contrast = 
                        (this.working_settings.filters.default_contrast + 5.0).min(100.0);
                },
                |this, _cx| {
                    this.working_settings.filters.default_contrast = 
                        (this.working_settings.filters.default_contrast - 5.0).max(-100.0);
                },
                cx
            ))
            .child(self.render_numeric_input(
                "Default gamma".to_string(),
                format!("{:.2}", default_gamma),
                Some("Default gamma value when resetting (0.1 to 10.0)".to_string()),
                |this, _cx| {
                    this.working_settings.filters.default_gamma = 
                        (this.working_settings.filters.default_gamma + 0.1).min(10.0);
                },
                |this, _cx| {
                    this.working_settings.filters.default_gamma = 
                        (this.working_settings.filters.default_gamma - 0.1).max(0.1);
                },
                cx
            ))
            .child(self.render_checkbox(
                "Remember filter state per-image".to_string(),
                remember_filter_state,
                Some("Remember filter settings for each image separately".to_string()),
                |this, _cx| {
                    this.working_settings.filters.remember_filter_state = !this.working_settings.filters.remember_filter_state;
                },
                cx
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
                            .when(filter_presets.is_empty(), |el| {
                                el.child("No presets saved")
                            })
                            .when(!filter_presets.is_empty(), |el| {
                                el.children(
                                    filter_presets.iter().map(|preset| {
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
    fn render_sort_navigation(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let default_sort_mode = self.working_settings.sort_navigation.default_sort_mode;
        let wrap_navigation = self.working_settings.sort_navigation.wrap_navigation;
        let show_image_counter = self.working_settings.sort_navigation.show_image_counter;
        
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
                                    .cursor_pointer()
                                    .when(default_sort_mode == SortModeWrapper::Alphabetical, |div| {
                                        div.border_color(Colors::info())
                                            .bg(rgba(0x50fa7b22))
                                    })
                                    .when(default_sort_mode != SortModeWrapper::Alphabetical, |div| {
                                        div.border_color(rgb(0x444444))
                                    })
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                        this.working_settings.sort_navigation.default_sort_mode = SortModeWrapper::Alphabetical;
                                        cx.notify();
                                    }))
                                    .child("Alphabetical")
                            )
                            .child(
                                div()
                                    .px(Spacing::md())
                                    .py(Spacing::sm())
                                    .rounded(px(4.0))
                                    .border_1()
                                    .cursor_pointer()
                                    .when(default_sort_mode == SortModeWrapper::ModifiedDate, |div| {
                                        div.border_color(Colors::info())
                                            .bg(rgba(0x50fa7b22))
                                    })
                                    .when(default_sort_mode != SortModeWrapper::ModifiedDate, |div| {
                                        div.border_color(rgb(0x444444))
                                    })
                                    .text_size(TextSize::sm())
                                    .text_color(Colors::text())
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                        this.working_settings.sort_navigation.default_sort_mode = SortModeWrapper::ModifiedDate;
                                        cx.notify();
                                    }))
                                    .child("Modified Date")
                            )
                    )
            )
            .child(self.render_checkbox(
                "Wrap navigation".to_string(),
                wrap_navigation,
                Some("Navigate from last image to first (and vice versa)".to_string()),
                |this, _cx| {
                    this.working_settings.sort_navigation.wrap_navigation = !this.working_settings.sort_navigation.wrap_navigation;
                },
                cx
            ))
            .child(self.render_checkbox(
                "Show image counter".to_string(),
                show_image_counter,
                Some("Display image position in window title".to_string()),
                |this, _cx| {
                    this.working_settings.sort_navigation.show_image_counter = !this.working_settings.sort_navigation.show_image_counter;
                },
                cx
            ))
    }

    /// Render external tools section
    fn render_external_tools(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let external_viewers = self.working_settings.external_tools.external_viewers.clone();
        let external_editor = self.working_settings.external_tools.external_editor.clone();
        let enable_file_manager_integration = self.working_settings.external_tools.enable_file_manager_integration;
        
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
                                        .when(external_viewers.is_empty(), |el| {
                                            el.text_size(TextSize::sm())
                                                .text_color(rgb(0xaaaaaa))
                                                .child("No external viewers configured")
                                        })
                                        .when(!external_viewers.is_empty(), |el| {
                                            el.children(
                                                external_viewers.iter().enumerate().map(|(i, viewer)| {
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
                                external_editor
                                    .as_ref()
                                    .map(|e| format!("{} {}", e.command, e.args.join(" ")))
                                    .unwrap_or_else(|| "Not configured".to_string())
                            )
                    )
            )
            .child(self.render_checkbox(
                "File manager integration".to_string(),
                enable_file_manager_integration,
                Some("Show 'Reveal in Finder/Explorer' menu option".to_string()),
                |this, _cx| {
                    this.working_settings.external_tools.enable_file_manager_integration = !this.working_settings.external_tools.enable_file_manager_integration;
                },
                cx
            ))
    }

    /// Render settings file section
    fn render_settings_file(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_path = settings_io::get_settings_path();
        let path_str = settings_path.display().to_string();
        
        div()
            .flex()
            .flex_col()
            .child(self.render_section_header("Settings File".to_string()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label("Settings file location".to_string(), Some("Path to the JSON settings file".to_string())))
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
                                    .text_color(Colors::text())
                                    .overflow_x_hidden()
                                    .child(path_str.clone())
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
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _event: &MouseDownEvent, _window, cx| {
                                        // Copy path to clipboard
                                        cx.write_to_clipboard(ClipboardItem::new_string(path_str.clone()));
                                    }))
                                    .child("Copy Path")
                            )
                    )
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb(Spacing::md())
                    .child(self.render_label("Quick actions".to_string(), Some("Open the settings file or its containing folder".to_string())))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(Spacing::sm())
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
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |_this, _event: &MouseDownEvent, _window, _cx| {
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
                                    }))
                                    .child("Reveal in File Manager")
                            )
                    )
            )
            .child(
                div()
                    .px(Spacing::md())
                    .py(Spacing::md())
                    .bg(rgba(0x50fa7b22))
                    .border_1()
                    .border_color(Colors::info())
                    .rounded(px(4.0))
                    .text_size(TextSize::sm())
                    .text_color(Colors::text())
                    .child("Note: You can manually edit the settings.json file to configure advanced options like window title format, background color, and external tool commands.")
            )
    }

    /// Render the content area based on selected section
    fn render_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .child(
                scrollable_vertical(
                    div()
                        .p(Spacing::xl())
                        .child(match self.current_section {
                            SettingsSection::ViewerBehavior => self.render_viewer_behavior(cx).into_any_element(),
                            SettingsSection::Performance => self.render_performance(cx).into_any_element(),
                            SettingsSection::KeyboardMouse => self.render_keyboard_mouse(cx).into_any_element(),
                            SettingsSection::FileOperations => self.render_file_operations(cx).into_any_element(),
                            SettingsSection::Appearance => self.render_appearance(cx).into_any_element(),
                            SettingsSection::Filters => self.render_filters(cx).into_any_element(),
                            SettingsSection::SortNavigation => self.render_sort_navigation(cx).into_any_element(),
                            SettingsSection::ExternalTools => self.render_external_tools(cx).into_any_element(),
                            SettingsSection::SettingsFile => self.render_settings_file(cx).into_any_element(),
                        })
                )
                .id("settings-content-scroll")
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
                    .child(format!("{}-Enter to apply • Esc to cancel", platform_key))
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
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                this.reset_to_defaults();
                                cx.notify();
                                // Dispatch the action to parent using window context
                                window.dispatch_action(ResetSettingsToDefaults.boxed_clone(), cx);
                            }))
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
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                                this.cancel();
                                // Dispatch action to parent App to close the settings window
                                window.dispatch_action(CancelSettings.boxed_clone(), cx);
                            }))
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
                            .on_mouse_down(MouseButton::Left, cx.listener(|_this, _event: &MouseDownEvent, window, cx| {
                                // Dispatch action to parent App to save and close
                                window.dispatch_action(ApplySettings.boxed_clone(), cx);
                            }))
                            .child("Apply")
                    )
            )
    }
}

impl Focusable for SettingsWindow {
    fn focus_handle(&self, _cx: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
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
                            .child(self.render_content(cx))
                    )
                    .child(self.render_footer(cx))
            )
    }
}
