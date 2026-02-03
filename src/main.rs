#![allow(clippy::collapsible_if)]

use gpui::prelude::FluentBuilder;
use gpui::*;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
mod macos_open_handler;

/// Global storage for pending file open requests from macOS "Open With" events.
/// This is necessary because the `on_open_urls` callback doesn't receive GPUI context,
/// so we store the paths here and process them when the app context is available.
static PENDING_OPEN_PATHS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

///// Parse a file:// URL to a PathBuf
fn parse_file_url(url: &str) -> Option<PathBuf> {
    if let Some(path_str) = url.strip_prefix("file://") {
        // URL decode the path (handle %XX encoding for spaces, etc.)
        let decoded = url_decode(path_str);
        Some(PathBuf::from(decoded))
    } else {
        None
    }
}

/// URL decoder for file paths that properly handles UTF-8
fn url_decode(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            // Try to read two hex digits
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    bytes.push(byte);
                    continue;
                }
            }
            // If decoding failed, keep the original %XX as bytes
            bytes.push(b'%');
            bytes.extend(hex.as_bytes());
        } else {
            // Regular ASCII character - add its UTF-8 bytes
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            bytes.extend(encoded.as_bytes());
        }
    }

    // Convert bytes to UTF-8 string, replacing invalid sequences
    String::from_utf8(bytes).unwrap_or_else(|e| {
        String::from_utf8_lossy(e.as_bytes()).into_owned()
    })
}

mod cli;
mod components;
mod error;
mod state;
mod utils;

use cli::Cli;
use components::{
    DebugOverlay, DebugOverlayConfig, FilterControls, HelpOverlay, ImageViewer, SettingsWindow,
};
use state::{AppSettings, AppState};
use utils::settings_io;

// Import all actions from lib.rs (they're defined there to avoid duplication)
use rpview_gpui::{
    BrightnessDown, BrightnessUp, CloseSettings, CloseWindow, ContrastDown, ContrastUp,
    DisableFilters, EnableFilters, EscapePressed, GammaDown, GammaUp, NextFrame, NextImage,
    OpenFile, OpenInExternalEditor, OpenInExternalViewer, OpenInExternalViewerAndQuit, PanDown,
    PanDownFast, PanDownSlow, PanLeft, PanLeftFast, PanLeftSlow, PanRight, PanRightFast,
    PanRightSlow, PanUp, PanUpFast, PanUpSlow, PreviousFrame, PreviousImage, Quit, ResetFilters,
    ResetSettingsToDefaults, RevealInFinder, SaveFile, SaveFileToDownloads, SortAlphabetical,
    SortByModified, ToggleAnimationPlayPause, ToggleDebug, ToggleFilters, ToggleHelp,
    ToggleSettings, ZoomIn, ZoomInFast, ZoomInIncremental, ZoomInSlow, ZoomOut, ZoomOutFast,
    ZoomOutIncremental, ZoomOutSlow, ZoomReset, ZoomResetAndCenter,
};

struct App {
    app_state: AppState,
    viewer: ImageViewer,
    focus_handle: FocusHandle,
    escape_presses: Vec<Instant>,
    /// Tracks if Z key is currently held down (for Z+drag zoom mode)
    z_key_held: bool,
    /// Tracks if spacebar is currently held down (for spacebar+drag pan mode)
    spacebar_held: bool,
    /// Tracks if left mouse button is currently pressed
    /// Used with MouseMoveEvent.pressed_button for robust button state tracking
    mouse_button_down: bool,
    /// Whether help overlay is visible
    show_help: bool,
    /// Whether debug overlay is visible
    show_debug: bool,
    /// Whether settings window is visible
    show_settings: bool,
    /// Whether filter controls overlay is visible
    show_filters: bool,
    /// Filter controls component
    filter_controls: Entity<FilterControls>,
    /// Settings window component
    settings_window: Entity<SettingsWindow>,
    /// Help overlay component
    help_overlay: Entity<HelpOverlay>,
    /// Menu bar component (Windows/Linux only)
    #[cfg(not(target_os = "macos"))]
    menu_bar: Entity<components::MenuBar>,
    /// Last time animation frame was updated (for animation playback)
    last_frame_update: Instant,
    /// Whether files are being dragged over the window
    drag_over: bool,
    /// Application settings (loaded on startup)
    settings: AppSettings,
}

impl App {
    /// Check if modal overlays (settings) are blocking main window interactions
    /// Note: Menu bar state is handled separately via escape key
    fn is_modal_open(&self) -> bool {
        self.show_settings
    }

    fn handle_escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Close menu bar if open (Windows/Linux)
        #[cfg(not(target_os = "macos"))]
        {
            let menu_open = self.menu_bar.read_with(&cx, |mb, _| mb.is_menu_open());
            if menu_open {
                self.menu_bar.update(cx, |mb, cx| mb.close_menu(cx));
                return;
            }
        }

        // If help, debug, settings, or filter overlay is open, close it instead of counting toward quit
        if self.show_help {
            self.show_help = false;
            self.focus_handle.focus(window);
            cx.notify();
            return;
        }
        if self.show_debug {
            self.show_debug = false;
            self.focus_handle.focus(window);
            cx.notify();
            return;
        }
        if self.show_settings {
            self.show_settings = false;
            self.focus_handle.focus(window);
            cx.notify();
            return;
        }
        if self.show_filters {
            self.show_filters = false;
            self.focus_handle.focus(window);
            cx.notify();
            return;
        }

        let now = Instant::now();

        // Remove presses older than 2 seconds
        self.escape_presses
            .retain(|&time| now.duration_since(time) < Duration::from_secs(2));

        // Add current press
        self.escape_presses.push(now);

        // Check if we have 3 presses within 2 seconds
        if self.escape_presses.len() >= 3 {
            cx.quit();
        }
    }

    fn handle_toggle_help(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_help = !self.show_help;
        cx.notify();
    }

    fn handle_toggle_debug(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_debug = !self.show_debug;
        cx.notify();
    }

    fn handle_toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_settings = !self.show_settings;

        if self.show_settings {
            // Focus the settings window when opening
            self.settings_window.update(cx, |settings, inner_cx| {
                let handle = settings.focus_handle(inner_cx);
                handle.focus(window);
            });
        } else {
            // Restore focus to the main app when hiding settings
            self.focus_handle.focus(window);
        }

        cx.notify();
    }

    fn handle_close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Get current settings from the settings window and save to disk
        let new_settings = self.settings_window.update(cx, |sw, _cx| sw.get_settings());

        // Save settings to disk
        if let Err(e) = settings_io::save_settings(&new_settings) {
            eprintln!("Error saving settings: {}", e);
        } else {
            println!("Settings saved successfully");
        }

        // Update app settings
        self.settings = new_settings;

        // Close the settings window
        self.show_settings = false;
        self.focus_handle.focus(window);

        cx.notify();
    }

    fn handle_reset_settings_to_defaults(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Reset settings window to defaults
        self.settings_window.update(cx, |sw, _cx| {
            sw.reset_to_defaults();
        });

        cx.notify();
    }

    fn handle_load_oversized_image_anyway(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Get the current image path from the oversized_image state
        if let Some((ref path, _, _, _)) = self.viewer.oversized_image {
            let path = path.clone();

            // Set the override flag in the image state cache
            let mut state = self
                .app_state
                .image_states
                .get(&path)
                .cloned()
                .unwrap_or_else(state::ImageState::new);
            state.override_size_limit = true;
            self.app_state.image_states.insert(path.clone(), state);

            // Reload the image with force_load = true
            let max_dim = Some(self.settings.performance.max_image_dimension);
            self.viewer.load_image_async(path, max_dim, true);

            cx.notify();
        }
    }

    fn handle_toggle_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_filters = !self.show_filters;

        // Restore focus to the main app when hiding filters
        if !self.show_filters {
            self.focus_handle.focus(window);
        }

        cx.notify();
    }

    fn handle_disable_filters(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.image_state.filters_enabled = false;
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_enable_filters(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.image_state.filters_enabled = true;
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_reset_filters(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Reset to default values from settings
        let default_filters = state::image_state::FilterSettings {
            brightness: self.settings.filters.default_brightness,
            contrast: self.settings.filters.default_contrast,
            gamma: self.settings.filters.default_gamma,
        };

        self.viewer.image_state.filters = default_filters;
        self.viewer.update_filtered_cache();
        self.save_current_image_state();

        // Update the filter controls sliders to reflect the reset values
        self.filter_controls.update(cx, |controls, cx| {
            controls.update_from_filters(default_filters, cx);
        });

        cx.notify();
    }

    fn handle_brightness_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let current = self.viewer.image_state.filters.brightness;
        self.viewer.image_state.filters.brightness = (current + 5.0).min(100.0);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_brightness_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let current = self.viewer.image_state.filters.brightness;
        self.viewer.image_state.filters.brightness = (current - 5.0).max(-100.0);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_contrast_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let current = self.viewer.image_state.filters.contrast;
        self.viewer.image_state.filters.contrast = (current + 5.0).min(100.0);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_contrast_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let current = self.viewer.image_state.filters.contrast;
        self.viewer.image_state.filters.contrast = (current - 5.0).max(-100.0);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_gamma_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let current = self.viewer.image_state.filters.gamma;
        self.viewer.image_state.filters.gamma = (current + 0.1).min(10.0);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_gamma_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let current = self.viewer.image_state.filters.gamma;
        self.viewer.image_state.filters.gamma = (current - 0.1).max(0.1);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_open_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        // Open native file dialog for image selection (single file)
        let mut file_dialog = rfd::FileDialog::new()
            .add_filter(
                "Images",
                &[
                    "png", "jpg", "jpeg", "bmp", "gif", "tiff", "tif", "ico", "webp",
                ],
            )
            .set_title("Open Image");

        // Set default directory to current image's parent directory if available
        if let Some(current_path) = self.app_state.current_image() {
            if let Some(parent) = current_path.parent() {
                file_dialog = file_dialog.set_directory(parent);
            }
        }

        // Get selected file (single selection)
        if let Some(file) = file_dialog.pick_file() {
            // Use process_dropped_path to scan the entire directory
            // and find the index of the selected file
            match utils::file_scanner::process_dropped_path(&file) {
                Ok((all_images, start_index)) => {
                    // Replace the current image list with all images from the directory
                    self.app_state.image_paths = all_images;
                    self.app_state.current_index = start_index;

                    // Re-apply current sort mode to maintain consistency
                    let current_sort_mode = self.app_state.sort_mode;
                    self.app_state.sort_mode = state::app_state::SortMode::Alphabetical; // Reset to force re-sort
                    self.app_state.set_sort_mode(current_sort_mode);

                    // Update viewer with selected image
                    self.update_viewer(window, cx);
                    self.update_window_title(window);
                    cx.notify();
                }
                Err(e) => {
                    eprintln!("Error opening file: {:?}", e);
                    self.viewer.error_message = Some(format!("Error opening file: {}", e));
                    cx.notify();
                }
            }
        }
    }

    fn handle_save_file(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.handle_save_file_impl(None, cx);
    }

    fn handle_save_file_to_downloads(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        // Get the Downloads directory
        let downloads_dir = dirs::download_dir();
        self.handle_save_file_impl(downloads_dir, cx);
    }

    fn handle_save_file_impl(&mut self, default_dir: Option<PathBuf>, cx: &mut Context<Self>) {
        // Only save if we have a current image
        if let Some(current_path) = self.app_state.current_image() {
            // Get original filename without extension
            let original_stem = current_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("image");

            // Determine extension from settings when filters are enabled
            let save_ext = if self.viewer.image_state.filters_enabled {
                // Use default save format from settings
                use crate::state::settings::SaveFormat;
                match self.settings.file_operations.default_save_format {
                    SaveFormat::SameAsLoaded => {
                        // Use original extension
                        current_path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("png")
                    }
                    SaveFormat::Png => "png",
                    SaveFormat::Jpeg => "jpg",
                    SaveFormat::Bmp => "bmp",
                    SaveFormat::Tiff => "tiff",
                    SaveFormat::Webp => "webp",
                }
            } else {
                // Use original extension for unfiltered saves
                current_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("png")
            };

            // Generate suggested filename with _filtered suffix if filters are enabled
            let suggested_name = if self.viewer.image_state.filters_enabled {
                format!("{}_filtered.{}", original_stem, save_ext)
            } else {
                format!("{}.{}", original_stem, save_ext)
            };

            // Open save dialog
            let mut file_dialog = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .add_filter("JPEG", &["jpg", "jpeg"])
                .add_filter("BMP", &["bmp"])
                .add_filter("TIFF", &["tiff", "tif"])
                .add_filter("WEBP", &["webp"])
                .set_file_name(&suggested_name)
                .set_title("Save Image");

            // Set default directory based on parameter or settings
            if let Some(dir) = default_dir {
                file_dialog = file_dialog.set_directory(dir);
            } else if let Some(ref default_save_dir) =
                self.settings.file_operations.default_save_directory
            {
                // Use default save directory from settings
                file_dialog = file_dialog.set_directory(default_save_dir);
            } else if let Some(parent) = current_path.parent() {
                // Fall back to current image's parent directory
                file_dialog = file_dialog.set_directory(parent);
            }

            if let Some(save_path) = file_dialog.save_file() {
                // Determine what to save based on filter state
                let save_result = if self.viewer.image_state.filters_enabled {
                    // Save the filtered image if filters are enabled and cached
                    if let Some(loaded_image) = &self.viewer.current_image {
                        if let Some(ref filtered_path) = loaded_image.filtered_path {
                            // Copy the cached filtered image to the save location
                            std::fs::copy(filtered_path, &save_path)
                                .map(|_| ())
                                .map_err(|e| format!("Failed to copy filtered image: {}", e))
                        } else {
                            // Filters enabled but no cache - load original and apply filters
                            if let Ok(original_img) = image::open(&loaded_image.path) {
                                let filters = &self.viewer.image_state.filters;
                                let filtered_img = utils::filters::apply_filters(
                                    &original_img,
                                    filters.brightness,
                                    filters.contrast,
                                    filters.gamma,
                                );
                                self.save_dynamic_image_to_path(&filtered_img, &save_path)
                            } else {
                                Err("Failed to load original image".to_string())
                            }
                        }
                    } else {
                        Err("No image loaded".to_string())
                    }
                } else {
                    // Save original image without filters
                    if let Some(loaded_image) = &self.viewer.current_image {
                        std::fs::copy(&loaded_image.path, &save_path)
                            .map(|_| ())
                            .map_err(|e| format!("Failed to copy image: {}", e))
                    } else {
                        Err("No image loaded".to_string())
                    }
                };

                // Handle save result
                match save_result {
                    Ok(()) => {
                        println!("Image saved to: {}", save_path.display());
                    }
                    Err(e) => {
                        eprintln!("Failed to save image: {}", e);
                    }
                }
            }
        }

        cx.notify();
    }

    fn save_dynamic_image_to_path(
        &self,
        image_data: &image::DynamicImage,
        save_path: &PathBuf,
    ) -> Result<(), String> {
        // Determine output format from file extension
        let extension = save_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_lowercase();

        let save_result = match extension.as_str() {
            "png" => image_data.save_with_format(save_path, image::ImageFormat::Png),
            "jpg" | "jpeg" => {
                // Convert to RGB for JPEG (no alpha channel)
                let rgb_image = image_data.to_rgb8();
                rgb_image.save_with_format(save_path, image::ImageFormat::Jpeg)
            }
            "bmp" => image_data.save_with_format(save_path, image::ImageFormat::Bmp),
            "tiff" | "tif" => image_data.save_with_format(save_path, image::ImageFormat::Tiff),
            "webp" => image_data.save_with_format(save_path, image::ImageFormat::WebP),
            _ => {
                // Default to PNG for unknown extensions
                image_data.save_with_format(save_path, image::ImageFormat::Png)
            }
        };

        save_result.map_err(|e| format!("Failed to save image: {}", e))
    }

    fn handle_open_in_external_viewer(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        if let Some(current_path) = self.app_state.current_image() {
            if let Err(e) = self.open_in_system_viewer(current_path) {
                eprintln!("Failed to open image in external viewer: {}", e);
            }
        }
        cx.notify();
    }

    fn handle_open_in_external_viewer_and_quit(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_modal_open() {
            return;
        }
        if let Some(current_path) = self.app_state.current_image() {
            if let Err(e) = self.open_in_system_viewer(current_path) {
                eprintln!("Failed to open image in external viewer: {}", e);
            } else {
                // Only quit if we successfully opened the image
                cx.quit();
            }
        }
    }

    fn handle_open_in_external_editor(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        if let Some(current_path) = self.app_state.current_image() {
            if let Err(e) = self.open_in_external_editor(current_path) {
                eprintln!("Failed to open image in external editor: {}", e);
            }
        }
        cx.notify();
    }

    fn handle_reveal_in_finder(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        if let Some(current_path) = self.app_state.current_image() {
            if let Err(e) = self.reveal_in_finder(current_path) {
                eprintln!("Failed to reveal in file manager: {}", e);
            }
        }
        cx.notify();
    }

    #[allow(clippy::needless_return)]
    fn reveal_in_finder(&self, path: &std::path::Path) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg("-R")
                .arg(path)
                .spawn()
                .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg("/select,")
                .arg(path)
                .spawn()
                .map_err(|e| format!("Failed to reveal in Explorer: {}", e))?;
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            // Try to get the parent directory and open it
            if let Some(parent) = path.parent() {
                std::process::Command::new("xdg-open")
                    .arg(parent)
                    .spawn()
                    .map_err(|e| format!("Failed to open file manager: {}", e))?;
                return Ok(());
            }
            return Err("Could not determine parent directory".to_string());
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Err("Reveal in file manager not supported on this platform".to_string())
        }
    }

    #[allow(clippy::needless_return)]
    fn open_in_system_viewer(&self, image_path: &PathBuf) -> Result<(), String> {
        // Get the configured external viewers from settings
        let viewers = &self.settings.external_tools.external_viewers;

        // Try each enabled viewer in order
        for viewer_config in viewers.iter().filter(|v| v.enabled) {
            // Replace {path} placeholder with actual image path
            let path_str = image_path
                .to_str()
                .ok_or_else(|| "Invalid image path: cannot convert to string".to_string())?;

            let args: Vec<String> = viewer_config
                .args
                .iter()
                .map(|arg| arg.replace("{path}", path_str))
                .collect();

            // Try to launch the viewer
            let result = std::process::Command::new(&viewer_config.command)
                .args(&args)
                .spawn();

            match result {
                Ok(_) => {
                    eprintln!("Opened image with: {}", viewer_config.name);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Failed to launch {}: {}", viewer_config.name, e);
                    // Continue to next viewer
                }
            }
        }

        // All configured viewers failed, try platform defaults as fallback
        eprintln!("All configured viewers failed, trying platform defaults...");

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(image_path)
                .spawn()
                .map_err(|e| format!("Failed to open with default viewer: {}", e))?;
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(&["/C", "start", "", image_path.to_str().unwrap_or("")])
                .spawn()
                .map_err(|e| format!("Failed to open with default viewer: {}", e))?;
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(image_path)
                .spawn()
                .map_err(|e| format!("Failed to open with default viewer: {}", e))?;
            return Ok(());
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Err("No suitable image viewer found for this platform".to_string())
        }
    }

    fn open_in_external_editor(&self, image_path: &std::path::Path) -> Result<(), String> {
        // Check if an external editor is configured
        if let Some(editor_config) = &self.settings.external_tools.external_editor {
            if !editor_config.enabled {
                return Err("External editor is configured but disabled".to_string());
            }

            // Replace {path} placeholder with actual image path
            let path_str = image_path
                .to_str()
                .ok_or_else(|| "Invalid image path: cannot convert to string".to_string())?;

            let args: Vec<String> = editor_config
                .args
                .iter()
                .map(|arg| arg.replace("{path}", path_str))
                .collect();

            // Try to launch the editor
            std::process::Command::new(&editor_config.command)
                .args(&args)
                .spawn()
                .map_err(|e| format!("Failed to launch {}: {}", editor_config.name, e))?;

            eprintln!("Opened image in external editor: {}", editor_config.name);
            Ok(())
        } else {
            Err("No external editor configured. Please set one in Settings (Cmd+,)".to_string())
        }
    }

    fn handle_dropped_files(
        &mut self,
        paths: &ExternalPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dropped_paths: Vec<&PathBuf> = paths.paths().iter().collect();

        // Determine the drop strategy based on what was dropped
        // If only 1 file: scan parent directory and set index to that file
        // If multiple files: use only those files (don't scan directories)
        // If 1 directory: scan that directory and start at index 0
        // If multiple items with directories: scan each directory and collect files

        let mut all_images: Vec<PathBuf> = Vec::new();
        let mut target_index: usize = 0;

        if dropped_paths.len() == 1 {
            // Single item dropped: use the smart scanning logic
            let path = dropped_paths[0];

            match utils::file_scanner::process_dropped_path(path) {
                Ok((images, index)) => {
                    all_images = images;
                    target_index = index;
                }
                Err(_e) => {
                    // Error processing dropped path - continue to check if we have any images
                }
            }
        } else {
            // Multiple items dropped: collect only the specific files/directories dropped
            for path in dropped_paths {
                if path.is_file() {
                    // For files: verify they're valid images and add them
                    if utils::file_scanner::is_supported_image(path) {
                        all_images.push(path.to_path_buf());
                    }
                } else if path.is_dir() {
                    // For directories: scan and add all images from that directory
                    if let Ok(dir_images) = utils::file_scanner::scan_directory(path) {
                        all_images.extend(dir_images);
                    }
                }
            }

            // Remove duplicates
            all_images.sort();
            all_images.dedup();

            // Sort alphabetically (case-insensitive)
            utils::file_scanner::sort_alphabetically(&mut all_images);

            // Start at the first image
            target_index = 0;
        }

        // Only update if we found at least one image
        if !all_images.is_empty() {
            // Update app state with new image list
            self.app_state.image_paths = all_images;
            self.app_state.current_index = target_index;

            // Update viewer and window title
            self.update_viewer(window, cx);
            self.update_window_title(window);

            // Refocus the window to ensure render triggers
            self.focus_handle.focus(window);

            // Force a re-render
            cx.notify();
        }
    }

    /// Check for and process any pending file open requests from macOS "Open With" events.
    /// This is called when the app becomes active or on a timer.
    fn process_pending_open_paths(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Take ownership of pending paths from the global storage
        let mut pending_paths: Vec<PathBuf> = {
            let mut pending = PENDING_OPEN_PATHS.lock().unwrap();
            std::mem::take(&mut *pending)
        };

        // Also collect paths from the macOS-specific handler
        #[cfg(target_os = "macos")]
        {
            pending_paths.extend(macos_open_handler::take_pending_paths());
        }

        if pending_paths.is_empty() {
            return;
        }

        // Process the paths similar to handle_dropped_files
        let mut all_images: Vec<PathBuf> = Vec::new();
        let mut target_index: usize = 0;

        if pending_paths.len() == 1 {
            // Single file: scan parent directory and set index to that file
            let path = &pending_paths[0];
            match utils::file_scanner::process_dropped_path(path) {
                Ok((images, index)) => {
                    all_images = images;
                    target_index = index;
                }
                Err(_e) => {
                    // Error processing path
                }
            }
        } else {
            // Multiple files: collect only the specific files
            for path in &pending_paths {
                if path.is_file() {
                    if utils::file_scanner::is_supported_image(path) {
                        all_images.push(path.clone());
                    }
                } else if path.is_dir() {
                    if let Ok(dir_images) = utils::file_scanner::scan_directory(path) {
                        all_images.extend(dir_images);
                    }
                }
            }

            // Remove duplicates and sort
            all_images.sort();
            all_images.dedup();
            utils::file_scanner::sort_alphabetically(&mut all_images);
            target_index = 0;
        }

        // Update if we found images
        if !all_images.is_empty() {
            self.app_state.image_paths = all_images;
            self.app_state.current_index = target_index;
            self.update_viewer(window, cx);
            self.update_window_title(window);
            self.focus_handle.focus(window);
            cx.notify();
        }
    }

    fn handle_next_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }

        let wrap = self.settings.sort_navigation.wrap_navigation;
        self.app_state.next_image_with_wrap(wrap);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }

    fn handle_previous_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let wrap = self.settings.sort_navigation.wrap_navigation;
        self.app_state.previous_image_with_wrap(wrap);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }

    fn handle_toggle_animation(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            anim_state.is_playing = !anim_state.is_playing;
            if anim_state.is_playing {
                // Reset timer when starting playback
                self.last_frame_update = Instant::now();
            }
            cx.notify();
        }
    }

    fn handle_next_frame(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            // Pause animation when manually navigating frames
            anim_state.is_playing = false;
            anim_state.current_frame = (anim_state.current_frame + 1) % anim_state.frame_count;
            cx.notify();
        }
    }

    fn handle_previous_frame(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            // Pause animation when manually navigating frames
            anim_state.is_playing = false;
            if anim_state.current_frame == 0 {
                anim_state.current_frame = anim_state.frame_count - 1;
            } else {
                anim_state.current_frame -= 1;
            }
            cx.notify();
        }
    }

    fn handle_sort_alphabetical(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.app_state.set_sort_mode(state::SortMode::Alphabetical);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }

    fn handle_sort_by_modified(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.app_state.set_sort_mode(state::SortMode::ModifiedDate);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }

    fn handle_zoom_in(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.viewer.zoom_in(utils::zoom::ZOOM_STEP);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_zoom_out(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.viewer.zoom_out(utils::zoom::ZOOM_STEP);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_zoom_reset(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.viewer.reset_zoom();
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_zoom_reset_and_center(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.viewer.reset_zoom_and_pan();
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_zoom_in_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.viewer.zoom_in(utils::zoom::ZOOM_STEP_FAST);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_zoom_out_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.viewer.zoom_out(utils::zoom::ZOOM_STEP_FAST);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_zoom_in_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.viewer.zoom_in(utils::zoom::ZOOM_STEP_SLOW);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_zoom_out_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.viewer.zoom_out(utils::zoom::ZOOM_STEP_SLOW);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_zoom_in_incremental(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        // Incremental zoom: add 1% (0.01) to current zoom
        let current_zoom = self.viewer.image_state.zoom;
        let new_zoom = utils::zoom::clamp_zoom(current_zoom + utils::zoom::ZOOM_STEP_INCREMENTAL);
        self.viewer.image_state.zoom = new_zoom;
        self.viewer.image_state.is_fit_to_window = false;
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_zoom_out_incremental(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        // Incremental zoom: subtract 1% (0.01) from current zoom
        let current_zoom = self.viewer.image_state.zoom;
        let new_zoom = utils::zoom::clamp_zoom(current_zoom - utils::zoom::ZOOM_STEP_INCREMENTAL);
        self.viewer.image_state.zoom = new_zoom;
        self.viewer.image_state.is_fit_to_window = false;
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_pan_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_normal;
        self.viewer.pan(0.0, -speed); // Pan up = move viewport up = image moves down (positive Y)
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_pan_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_normal;
        self.viewer.pan(0.0, speed); // Pan down = move viewport down = image moves up (negative Y)
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_pan_left(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_normal;
        self.viewer.pan(-speed, 0.0); // Pan left = move image right (negative X)
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_pan_right(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_normal;
        self.viewer.pan(speed, 0.0); // Pan right = move image left (positive X)
        self.save_current_image_state();
        cx.notify();
    }

    // Fast pan with Shift modifier
    fn handle_pan_up_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_fast;
        self.viewer.pan(0.0, -speed);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_pan_down_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_fast;
        self.viewer.pan(0.0, speed);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_pan_left_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_fast;
        self.viewer.pan(-speed, 0.0);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_pan_right_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_fast;
        self.viewer.pan(speed, 0.0);
        self.save_current_image_state();
        cx.notify();
    }

    // Slow pan with Ctrl/Cmd modifier
    fn handle_pan_up_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_slow;
        self.viewer.pan(0.0, -speed);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_pan_down_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_slow;
        self.viewer.pan(0.0, speed);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_pan_left_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_slow;
        self.viewer.pan(-speed, 0.0);
        self.save_current_image_state();
        cx.notify();
    }

    fn handle_pan_right_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let speed = self.settings.keyboard_mouse.pan_speed_slow;
        self.viewer.pan(speed, 0.0);
        self.save_current_image_state();
        cx.notify();
    }

    fn save_current_image_state(&mut self) {
        // Only save state if enabled in settings
        if self.settings.viewer_behavior.remember_per_image_state {
            let state = self.viewer.get_image_state();
            self.app_state.save_current_state(state);
        }
    }

    fn load_current_image_state(&mut self, cx: &mut Context<Self>) {
        let default_filters = state::image_state::FilterSettings {
            brightness: self.settings.filters.default_brightness,
            contrast: self.settings.filters.default_contrast,
            gamma: self.settings.filters.default_gamma,
        };
        let state = self.app_state.get_current_state(default_filters);
        self.viewer.set_image_state(state.clone());

        // Update filter controls UI to reflect the loaded filter values
        self.filter_controls.update(cx, |controls, cx| {
            controls.update_from_filters(state.filters, cx);
        });

        // Restore cached filtered image if it exists (AFTER state is loaded)
        self.viewer.restore_filtered_image_from_state();

        // Only trigger filter processing if filters are applied AND we don't have a cached filtered image
        if state.filters_enabled
            && (state.filters.brightness.abs() >= 0.001
                || state.filters.contrast.abs() >= 0.001
                || (state.filters.gamma - 1.0).abs() >= 0.001)
            && state.filtered_image_path.is_none()
        {
            self.viewer.update_filtered_cache();
        }
    }

    fn update_viewer(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(path) = self.app_state.current_image().cloned() {
            // Ensure viewport size is set before loading
            let viewport_size = window.viewport_size();
            self.viewer.update_viewport_size(viewport_size);

            // Check if user has overridden size limit for this image
            let force_load = self
                .app_state
                .image_states
                .get(&path)
                .map(|state| state.override_size_limit)
                .unwrap_or(false);

            // Load the image asynchronously (non-blocking)
            let max_dim = Some(self.settings.performance.max_image_dimension);
            self.viewer
                .load_image_async(path.clone(), max_dim, force_load);

            // State will be loaded when async load completes (in render loop)
        } else {
            self.viewer.clear();
        }
    }

    fn update_window_title(&mut self, window: &mut Window) {
        if let Some(path) = self.app_state.current_image() {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");

            let position = self.app_state.current_index + 1;
            let total = self.app_state.image_paths.len();

            // Apply window title format from settings
            let title = if self.settings.sort_navigation.show_image_counter {
                self.settings
                    .appearance
                    .window_title_format
                    .replace("{filename}", filename)
                    .replace("{index}", &position.to_string())
                    .replace("{total}", &total.to_string())
            } else {
                filename.to_string()
            };

            window.set_window_title(&title);
        } else {
            window.set_window_title("rpview-gpui");
        }
    }
}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Check if async image loading has completed
        if self.viewer.check_async_load() {
            // Image loaded successfully or failed - load state and setup animation
            if let Some(path) = self.app_state.current_image().cloned() {
                // Load cached state if available and enabled in settings
                if self.settings.viewer_behavior.remember_per_image_state
                    && self.app_state.image_states.contains_key(&path)
                {
                    self.load_current_image_state(cx);
                } else {
                    // Apply default zoom mode from settings for new images
                    use crate::state::settings::ZoomMode;
                    match self.settings.viewer_behavior.default_zoom_mode {
                        ZoomMode::FitToWindow => {
                            self.viewer.fit_to_window();
                        }
                        ZoomMode::OneHundredPercent => {
                            self.viewer.set_one_hundred_percent();
                        }
                    }

                    // Reset filter controls to default (no filters)
                    let default_filters = state::image_state::FilterSettings {
                        brightness: 0.0,
                        contrast: 0.0,
                        gamma: 1.0,
                    };
                    self.filter_controls.update(cx, |controls, cx| {
                        controls.update_from_filters(default_filters, cx);
                    });
                }

                // Apply animation auto-play setting
                if let Some(ref mut anim_state) = self.viewer.image_state.animation {
                    // Set is_playing based on settings (unless we loaded cached state)
                    if !self.settings.viewer_behavior.remember_per_image_state
                        || !self.app_state.image_states.contains_key(&path)
                    {
                        anim_state.is_playing = self.settings.viewer_behavior.animation_auto_play;
                    }

                    if anim_state.is_playing {
                        self.last_frame_update = Instant::now();
                    }
                }
            }

            // Request re-render to show the loaded image
            cx.notify();
        }

        // Check if filter processing has completed
        let just_finished_processing = self.viewer.check_filter_processing();
        if just_finished_processing {
            // Filter processing completed - the pending path will be preloaded in this render
            // Reset preload frame counter
            self.viewer.pending_filter_preload_frames = 0;
            window.request_animation_frame();
            cx.notify();
        }

        // Track preload frames and apply pending filtered image after GPU has loaded texture
        if self.viewer.pending_filtered_path.is_some() {
            if !just_finished_processing {
                self.viewer.pending_filter_preload_frames += 1;
            }

            // Apply after 3 frames of preloading to ensure GPU has texture loaded
            // Frame 0: Set pending, start invisible render
            // Frame 1-2: Continue invisible render (GPU loads texture)
            // Frame 3: Apply (texture ready, no black flash)
            if self.viewer.pending_filter_preload_frames >= 3 {
                self.viewer.apply_pending_filtered_image();
                self.viewer.pending_filter_preload_frames = 0;
                cx.notify();
            } else {
                // Still preloading, request another frame
                window.request_animation_frame();
            }
        }

        // If still loading or processing filters, request another render to check again
        if self.viewer.is_loading || self.viewer.is_processing_filters {
            window.request_animation_frame();
        }

        // Update viewer's viewport size from window's drawable content area
        let viewport_size = window.viewport_size();
        self.viewer.update_viewport_size(viewport_size);

        // Set preload paths for next/previous images to prime GPU cache
        // This must happen in render() so images are preloaded BEFORE navigation occurs
        // This eliminates black flashing by ensuring textures are already in GPU memory
        let mut preload_paths = Vec::with_capacity(2);
        if let Some(next_path) = self.app_state.next_image_path() {
            preload_paths.push(next_path.clone());
        }
        if let Some(prev_path) = self.app_state.previous_image_path() {
            preload_paths.push(prev_path.clone());
        }
        self.viewer.set_preload_paths(preload_paths);

        // Update animation frame if playing (GPUI's suggested pattern)
        let should_update_animation = self
            .viewer
            .image_state
            .animation
            .as_ref()
            .map(|a| a.is_playing && a.frame_count > 0)
            .unwrap_or(false);

        if should_update_animation {
            // Progressive frame caching: cache next 3 frames ahead of playback
            // This is part of the animation loading strategy:
            // 1. load_image() caches first 3 frames immediately (frame 0, 1, 2)
            // 2. This loop caches the next 3 frames while animation plays
            // 3. By the time we reach a frame, it's already cached (smooth playback)
            if let Some(ref anim_state) = self.viewer.image_state.animation {
                let current = anim_state.current_frame;
                let total = anim_state.frame_count;

                // Cache next 3 frames ahead (look-ahead caching)
                for offset in 1..=3 {
                    let frame_to_cache = (current + offset) % total;
                    self.viewer.cache_frame(frame_to_cache);
                }
            }

            if let Some(ref mut anim_state) = self.viewer.image_state.animation {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_frame_update).as_millis() as u32;

                // Get current frame duration
                let frame_duration = anim_state
                    .frame_durations
                    .get(anim_state.current_frame)
                    .copied()
                    .unwrap_or(100);

                // Advance to next frame when duration has elapsed
                if elapsed >= frame_duration {
                    let next_frame = (anim_state.current_frame + 1) % anim_state.frame_count;
                    eprintln!(
                        "[ANIMATION] Advancing from frame {} to frame {}",
                        anim_state.current_frame, next_frame
                    );
                    anim_state.current_frame = next_frame;
                    self.last_frame_update = now;
                }
            }

            // Request next animation frame (GPUI's pattern for continuous animation)
            window.request_animation_frame();
        }

        // Poll filter controls for changes
        if self.show_filters {
            eprintln!("[App::render] Polling filter controls...");
            let (current_filters, changed) = self
                .filter_controls
                .update(cx, |fc, cx| fc.get_filters_and_detect_change(cx));

            if changed {
                eprintln!(
                    "[App::render] Filters changed! Updating viewer with brightness={:.1}, contrast={:.1}, gamma={:.2}",
                    current_filters.brightness, current_filters.contrast, current_filters.gamma
                );
                eprintln!(
                    "[App::render] Old viewer filters: brightness={:.1}, contrast={:.1}, gamma={:.2}",
                    self.viewer.image_state.filters.brightness,
                    self.viewer.image_state.filters.contrast,
                    self.viewer.image_state.filters.gamma
                );

                self.viewer.image_state.filters = current_filters;
                self.viewer.update_filtered_cache();
                self.save_current_image_state();

                // If we just started filter processing, request animation frames to poll for completion
                if self.viewer.is_processing_filters {
                    window.request_animation_frame();
                }
            }
        }

        // Update Z-drag state based on z_key_held
        if self.z_key_held && self.viewer.z_drag_state.is_none() {
            self.viewer.z_drag_state = Some((0.0, 0.0, 0.0, 0.0));
        } else if !self.z_key_held && self.viewer.z_drag_state.is_some() {
            self.viewer.z_drag_state = None;
        }

        // Update spacebar-drag state based on spacebar_held
        if self.spacebar_held && self.viewer.spacebar_drag_state.is_none() {
            self.viewer.spacebar_drag_state = Some((0.0, 0.0));
        } else if !self.spacebar_held && self.viewer.spacebar_drag_state.is_some() {
            self.viewer.spacebar_drag_state = None;
        }

        // Calculate background color once
        let bg_color = rgb(
            ((self.settings.appearance.background_color[0] as u32) << 16)
                | ((self.settings.appearance.background_color[1] as u32) << 8)
                | (self.settings.appearance.background_color[2] as u32),
        );

        // Main content area (takes remaining space after menu bar)
        let content = div()
            .flex_1()
            .min_h_0() // Allow shrinking below content size
            .bg(bg_color)
            .when(self.drag_over, |div| {
                // Show highlighted border when dragging files over the window
                div.border_4().border_color(gpui::rgb(0x50fa7b)) // Green highlight
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                    this.mouse_button_down = true;

                    // Close menu bar when clicking on main content (Windows/Linux)
                    #[cfg(not(target_os = "macos"))]
                    this.menu_bar.update(cx, |mb, cx| mb.close_menu(cx));

                    // Start spacebar-drag pan if spacebar is being held
                    if this.viewer.spacebar_drag_state.is_some() {
                        let x: f32 = event.position.x.into();
                        let y: f32 = event.position.y.into();
                        // Store: (last_x, last_y) for 1:1 pixel movement
                        this.viewer.spacebar_drag_state = Some((x, y));
                        cx.notify();
                    }
                    // Start Z-drag zoom if Z key is being held (and spacebar is not)
                    else if this.viewer.z_drag_state.is_some() {
                        let y: f32 = event.position.y.into();
                        let x: f32 = event.position.x.into();
                        // Store: (last_x, last_y, center_x, center_y) for zoom centering
                        this.viewer.z_drag_state = Some((x, y, x, y));
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _event: &MouseUpEvent, _window, cx| {
                    this.mouse_button_down = false;

                    // End spacebar-drag pan (but keep spacebar state active if still held)
                    if let Some((_, _)) = this.viewer.spacebar_drag_state {
                        // Save state after panning
                        this.save_current_image_state();
                        // Reset to sentinel value to indicate spacebar is held but not dragging
                        this.viewer.spacebar_drag_state = Some((0.0, 0.0));
                        cx.notify();
                    }
                    // End Z-drag zoom (but keep Z key state active if still held)
                    else if let Some((_, _, _, _)) = this.viewer.z_drag_state {
                        // Reset to sentinel value to indicate Z is held but not dragging
                        this.viewer.z_drag_state = Some((0.0, 0.0, 0.0, 0.0));
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                // Check if mouse button is actually pressed (safety check for button released outside window)
                let button_actually_pressed = event.pressed_button.is_some();

                // If we think the button is down but the event says it's not, correct our state
                if this.mouse_button_down && !button_actually_pressed {
                    this.mouse_button_down = false;
                    // End spacebar-drag pan if active
                    if this.viewer.spacebar_drag_state.is_some() {
                        this.viewer.spacebar_drag_state = Some((0.0, 0.0));
                    }
                    // End Z-drag zoom if active
                    if this.viewer.z_drag_state.is_some() {
                        this.viewer.z_drag_state = Some((0.0, 0.0, 0.0, 0.0));
                    }
                }

                // Handle spacebar-drag pan (only if mouse button is down and we have valid drag data)
                if this.mouse_button_down && button_actually_pressed {
                    if let Some((last_x, last_y)) = this.viewer.spacebar_drag_state {
                        // Check if this is actual drag data (not the sentinel 0,0)
                        if last_x != 0.0 || last_y != 0.0 {
                            let current_x: f32 = event.position.x.into();
                            let current_y: f32 = event.position.y.into();

                            // Calculate 1:1 pixel movement delta
                            let delta_x = current_x - last_x;
                            let delta_y = current_y - last_y;

                            // Apply pan directly (1:1 pixel movement)
                            this.viewer.pan(delta_x, delta_y);

                            // Update last position for next delta calculation
                            this.viewer.spacebar_drag_state = Some((current_x, current_y));

                            cx.notify();
                            return; // Don't process Z-drag if we're spacebar-dragging
                        }
                    }
                }

                // Handle Z-drag zoom (only if mouse button is down and we have valid drag data)
                if this.mouse_button_down && button_actually_pressed {
                    if let Some((last_x, last_y, center_x, center_y)) = this.viewer.z_drag_state {
                        // Check if this is actual drag data (not the sentinel 0,0,0,0)
                        if center_x != 0.0 || center_y != 0.0 {
                            let current_y: f32 = event.position.y.into();
                            let current_x: f32 = event.position.x.into();

                            // Calculate INCREMENTAL delta from LAST position (not initial)
                            let delta_y = last_y - current_y; // Up is positive (zoom in)
                            let delta_x = current_x - last_x; // Right is positive (zoom in)
                            let combined_delta = delta_y + delta_x;

                            // Get the current zoom level (which changes during drag)
                            let current_zoom = this.viewer.image_state.zoom;

                            // Scale zoom change proportionally to CURRENT zoom level
                            // At 100% zoom (1.0): sensitivity% per pixel
                            // At 200% zoom (2.0): 2*sensitivity% per pixel (more sensitive)
                            // At 50% zoom (0.5): 0.5*sensitivity% per pixel (less sensitive)
                            let sensitivity = this.settings.keyboard_mouse.z_drag_sensitivity;
                            let zoom_change = combined_delta * sensitivity * current_zoom;
                            let new_zoom = utils::zoom::clamp_zoom(current_zoom + zoom_change);

                            // Apply zoom centered on initial click position
                            let old_zoom = this.viewer.image_state.zoom;
                            this.viewer.image_state.zoom = new_zoom;

                            // Adjust pan to keep the click position at the same location
                            let (pan_x, pan_y) = this.viewer.image_state.pan;
                            let cursor_in_image_x = (center_x - pan_x) / old_zoom;
                            let cursor_in_image_y = (center_y - pan_y) / old_zoom;
                            let new_pan_x = center_x - cursor_in_image_x * new_zoom;
                            let new_pan_y = center_y - cursor_in_image_y * new_zoom;

                            this.viewer.image_state.pan = (new_pan_x, new_pan_y);
                            this.viewer.image_state.is_fit_to_window = false;

                            // Update last position for next delta calculation
                            this.viewer.z_drag_state =
                                Some((current_x, current_y, center_x, center_y));

                            cx.notify();
                        }
                    }
                }
            }))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, cx| {
                // Only handle scroll if Ctrl/Cmd is held
                // Use 'platform' field which is Cmd on macOS, Ctrl on other platforms
                if event.modifiers.platform {
                    // Get scroll delta in pixels (use window line height for conversion if needed)
                    let line_height = px(16.0); // Standard line height
                    let delta_y: f32 = event.delta.pixel_delta(line_height).y.into();

                    // Positive delta_y means scrolling down (zoom out)
                    // Negative delta_y means scrolling up (zoom in)
                    let zoom_in = delta_y < 0.0;

                    // Get cursor position relative to the viewport
                    let cursor_x: f32 = event.position.x.into();
                    let cursor_y: f32 = event.position.y.into();

                    // Use scroll wheel sensitivity from settings
                    let zoom_step = this.settings.keyboard_mouse.scroll_wheel_sensitivity;
                    this.viewer
                        .zoom_toward_point(cursor_x, cursor_y, zoom_in, zoom_step);
                    this.save_current_image_state();
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                // Don't process keyboard events if modal overlays are open
                if this.is_modal_open() {
                    return;
                }

                // Check for spacebar press (without modifiers)
                if event.keystroke.key.as_str() == "space"
                    && !event.keystroke.modifiers.shift
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.platform
                    && !event.keystroke.modifiers.alt
                {
                    // Enable spacebar-drag pan mode
                    this.spacebar_held = true;
                    cx.notify();
                }
                // Check for Z key press (without modifiers)
                else if event.keystroke.key.as_str() == "z"
                    && !event.keystroke.modifiers.shift
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.platform
                    && !event.keystroke.modifiers.alt
                {
                    // Enable Z-drag zoom mode
                    this.z_key_held = true;
                    cx.notify();
                }
            }))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _window, cx| {
                // Check for spacebar release
                if event.keystroke.key.as_str() == "space" {
                    // Disable spacebar-drag pan mode and save state
                    if this.spacebar_held {
                        this.spacebar_held = false;
                        this.save_current_image_state();
                        cx.notify();
                    }
                }
                // Check for Z key release
                else if event.keystroke.key.as_str() == "z" {
                    // Disable Z-drag zoom mode and save state
                    if this.z_key_held {
                        this.z_key_held = false;
                        this.save_current_image_state();
                        cx.notify();
                    }
                }
            }))
            .on_drag_move(cx.listener(
                |this, _event: &DragMoveEvent<ExternalPaths>, _window, cx| {
                    // Set drag-over state to show visual feedback
                    if !this.drag_over {
                        this.drag_over = true;
                        cx.notify();
                    }
                },
            ))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                // Clear drag-over state
                this.drag_over = false;
                this.handle_dropped_files(paths, window, cx);
            }))
            .child(self.viewer.render_view(
                self.settings.appearance.background_color,
                self.settings.appearance.overlay_transparency,
                self.settings.appearance.font_size_scale,
                cx,
            ))
            // Render overlays on top with proper z-order
            .when(self.show_help, |el| el.child(self.help_overlay.clone()))
            .when(self.show_debug, |el| {
                let image_dimensions = self
                    .viewer
                    .current_image
                    .as_ref()
                    .map(|img| (img.width, img.height));
                el.child(cx.new(|_cx| {
                    DebugOverlay::new(DebugOverlayConfig {
                        current_path: self.app_state.current_image().cloned(),
                        current_index: self.app_state.current_index,
                        total_images: self.app_state.image_paths.len(),
                        image_state: self.viewer.image_state.clone(),
                        image_dimensions,
                        viewport_size: self.viewer.viewport_size,
                        overlay_transparency: self.settings.appearance.overlay_transparency,
                        font_size_scale: self.settings.appearance.font_size_scale,
                    })
                }))
            })
            .when(self.show_settings, |el| {
                el.child(self.settings_window.clone())
            })
            .when(self.show_filters, |el| {
                el.child(self.filter_controls.clone())
            });

        // Outer container with menu bar (Windows/Linux) and content
        // Action handlers are registered here so they're available for menu items
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(bg_color)
            // Add menu bar for Windows/Linux
            .when(cfg!(not(target_os = "macos")), |el| {
                #[cfg(not(target_os = "macos"))]
                {
                    el.child(self.menu_bar.clone())
                }
                #[cfg(target_os = "macos")]
                {
                    el
                }
            })
            .child(content)
            // Action handlers - registered on focused element so menu items work
            .on_action(|_: &CloseWindow, window, _| {
                window.remove_window();
            })
            .on_action(cx.listener(|this, _: &EscapePressed, window, cx| {
                this.handle_escape(window, cx);
            }))
            .on_action(cx.listener(|this, _: &NextImage, window, cx| {
                this.handle_next_image(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousImage, window, cx| {
                this.handle_previous_image(window, cx);
            }))
            .on_action(
                cx.listener(|this, _: &ToggleAnimationPlayPause, window, cx| {
                    this.handle_toggle_animation(window, cx);
                }),
            )
            .on_action(cx.listener(|this, _: &NextFrame, window, cx| {
                this.handle_next_frame(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousFrame, window, cx| {
                this.handle_previous_frame(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SortAlphabetical, window, cx| {
                this.handle_sort_alphabetical(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SortByModified, window, cx| {
                this.handle_sort_by_modified(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomIn, window, cx| {
                this.handle_zoom_in(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOut, window, cx| {
                this.handle_zoom_out(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomReset, window, cx| {
                this.handle_zoom_reset(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomResetAndCenter, window, cx| {
                this.handle_zoom_reset_and_center(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomInFast, window, cx| {
                this.handle_zoom_in_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOutFast, window, cx| {
                this.handle_zoom_out_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomInSlow, window, cx| {
                this.handle_zoom_in_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOutSlow, window, cx| {
                this.handle_zoom_out_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomInIncremental, window, cx| {
                this.handle_zoom_in_incremental(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOutIncremental, window, cx| {
                this.handle_zoom_out_incremental(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanUp, window, cx| {
                this.handle_pan_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanDown, window, cx| {
                this.handle_pan_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanLeft, window, cx| {
                this.handle_pan_left(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanRight, window, cx| {
                this.handle_pan_right(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanUpFast, window, cx| {
                this.handle_pan_up_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanDownFast, window, cx| {
                this.handle_pan_down_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanLeftFast, window, cx| {
                this.handle_pan_left_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanRightFast, window, cx| {
                this.handle_pan_right_fast(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanUpSlow, window, cx| {
                this.handle_pan_up_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanDownSlow, window, cx| {
                this.handle_pan_down_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanLeftSlow, window, cx| {
                this.handle_pan_left_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PanRightSlow, window, cx| {
                this.handle_pan_right_slow(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleHelp, window, cx| {
                this.handle_toggle_help(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleDebug, window, cx| {
                this.handle_toggle_debug(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleSettings, window, cx| {
                this.handle_toggle_settings(window, cx);
            }))
            .on_action(cx.listener(|this, _: &CloseSettings, window, cx| {
                this.handle_close_settings(window, cx);
            }))
            .on_action(
                cx.listener(|this, _: &ResetSettingsToDefaults, window, cx| {
                    this.handle_reset_settings_to_defaults(window, cx);
                }),
            )
            .on_action(cx.listener(
                |this, _: &rpview_gpui::LoadOversizedImageAnyway, window, cx| {
                    this.handle_load_oversized_image_anyway(window, cx);
                },
            ))
            .on_action(cx.listener(|this, _: &ToggleFilters, window, cx| {
                this.handle_toggle_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &DisableFilters, window, cx| {
                this.handle_disable_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &EnableFilters, window, cx| {
                this.handle_enable_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ResetFilters, window, cx| {
                this.handle_reset_filters(window, cx);
            }))
            .on_action(cx.listener(|this, _: &BrightnessUp, window, cx| {
                this.handle_brightness_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &BrightnessDown, window, cx| {
                this.handle_brightness_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ContrastUp, window, cx| {
                this.handle_contrast_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &ContrastDown, window, cx| {
                this.handle_contrast_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &GammaUp, window, cx| {
                this.handle_gamma_up(window, cx);
            }))
            .on_action(cx.listener(|this, _: &GammaDown, window, cx| {
                this.handle_gamma_down(window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenFile, window, cx| {
                this.handle_open_file(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SaveFile, window, cx| {
                this.handle_save_file(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SaveFileToDownloads, window, cx| {
                this.handle_save_file_to_downloads(window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenInExternalViewer, window, cx| {
                this.handle_open_in_external_viewer(window, cx);
            }))
            .on_action(
                cx.listener(|this, _: &OpenInExternalViewerAndQuit, window, cx| {
                    this.handle_open_in_external_viewer_and_quit(window, cx);
                }),
            )
            .on_action(cx.listener(|this, _: &OpenInExternalEditor, window, cx| {
                this.handle_open_in_external_editor(window, cx);
            }))
            .on_action(cx.listener(|this, _: &RevealInFinder, window, cx| {
                this.handle_reveal_in_finder(window, cx);
            }))
    }
}

fn setup_key_bindings(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("escape", EscapePressed, None),
        KeyBinding::new("right", NextImage, None),
        KeyBinding::new("left", PreviousImage, None),
        // Animation controls
        KeyBinding::new("o", ToggleAnimationPlayPause, None),
        KeyBinding::new("]", NextFrame, None),
        KeyBinding::new("[", PreviousFrame, None),
        KeyBinding::new("shift-cmd-a", SortAlphabetical, None),
        KeyBinding::new("shift-cmd-m", SortByModified, None),
        // Zoom controls - base (normal speed)
        KeyBinding::new("=", ZoomIn, None), // = key (same as +)
        KeyBinding::new("+", ZoomIn, None),
        KeyBinding::new("-", ZoomOut, None),
        KeyBinding::new("0", ZoomReset, None),
        KeyBinding::new("cmd-0", ZoomResetAndCenter, None),
        // Zoom controls - fast (with Shift)
        KeyBinding::new("shift-=", ZoomInFast, None),
        KeyBinding::new("shift-+", ZoomInFast, None),
        KeyBinding::new("shift--", ZoomOutFast, None),
        // Zoom controls - slow (with Cmd/Ctrl)
        KeyBinding::new("cmd-=", ZoomInSlow, None),
        KeyBinding::new("cmd-+", ZoomInSlow, None),
        KeyBinding::new("cmd--", ZoomOutSlow, None),
        // Zoom controls - incremental (with Shift+Cmd/Ctrl)
        KeyBinding::new("shift-cmd-=", ZoomInIncremental, None),
        KeyBinding::new("shift-cmd-+", ZoomInIncremental, None),
        KeyBinding::new("shift-cmd--", ZoomOutIncremental, None),
        // Pan controls with WASD (base speed: 10px)
        KeyBinding::new("w", PanUp, None),
        KeyBinding::new("a", PanLeft, None),
        KeyBinding::new("s", PanDown, None),
        KeyBinding::new("d", PanRight, None),
        // Pan controls with IJKL (base speed: 10px)
        KeyBinding::new("i", PanUp, None),
        KeyBinding::new("j", PanLeft, None),
        KeyBinding::new("k", PanDown, None),
        KeyBinding::new("l", PanRight, None),
        // Fast pan with Shift (3x speed: 30px)
        KeyBinding::new("shift-w", PanUpFast, None),
        KeyBinding::new("shift-a", PanLeftFast, None),
        KeyBinding::new("shift-s", PanDownFast, None),
        KeyBinding::new("shift-d", PanRightFast, None),
        KeyBinding::new("shift-i", PanUpFast, None),
        KeyBinding::new("shift-j", PanLeftFast, None),
        KeyBinding::new("shift-k", PanDownFast, None),
        KeyBinding::new("shift-l", PanRightFast, None),
        // Slow pan with Alt (1px) - using Alt to avoid conflicts with Cmd/Ctrl shortcuts
        KeyBinding::new("alt-w", PanUpSlow, None),
        KeyBinding::new("alt-a", PanLeftSlow, None),
        KeyBinding::new("alt-s", PanDownSlow, None),
        KeyBinding::new("alt-d", PanRightSlow, None),
        KeyBinding::new("alt-i", PanUpSlow, None),
        KeyBinding::new("alt-j", PanLeftSlow, None),
        KeyBinding::new("alt-k", PanDownSlow, None),
        KeyBinding::new("alt-l", PanRightSlow, None),
        // Help and debug overlays
        KeyBinding::new("h", ToggleHelp, None),
        KeyBinding::new("?", ToggleHelp, None),
        KeyBinding::new("f1", ToggleHelp, None),
        KeyBinding::new("f12", ToggleDebug, None),
        // Settings window
        KeyBinding::new("cmd-,", ToggleSettings, None),
        KeyBinding::new("escape", CloseSettings, Some("SettingsWindow")),
        KeyBinding::new("cmd-enter", CloseSettings, Some("SettingsWindow")),
        // Filter controls
        KeyBinding::new("cmd-f", ToggleFilters, None),
        KeyBinding::new("cmd-1", DisableFilters, None),
        KeyBinding::new("cmd-2", EnableFilters, None),
        KeyBinding::new("shift-cmd-r", ResetFilters, None),
        // File operations
        KeyBinding::new("cmd-o", OpenFile, None),
        KeyBinding::new("cmd-s", SaveFile, None),
        KeyBinding::new("cmd-alt-s", SaveFileToDownloads, None),
        KeyBinding::new("cmd-r", RevealInFinder, None),
        // External viewer
        KeyBinding::new("cmd-alt-v", OpenInExternalViewer, None),
        KeyBinding::new("shift-cmd-alt-v", OpenInExternalViewerAndQuit, None),
        // External editor
        KeyBinding::new("cmd-e", OpenInExternalEditor, None),
        // Windows/Linux explicit Ctrl bindings (GPUI 0.2.2 doesn't translate cmd to ctrl)
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-w", CloseWindow, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-q", Quit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-a", SortAlphabetical, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-m", SortByModified, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-0", ZoomResetAndCenter, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-=", ZoomInSlow, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-+", ZoomInSlow, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl--", ZoomOutSlow, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-=", ZoomInIncremental, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-+", ZoomInIncremental, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl--", ZoomOutIncremental, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-,", ToggleSettings, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-enter", CloseSettings, Some("SettingsWindow")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-f", ToggleFilters, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-1", DisableFilters, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-2", EnableFilters, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-r", ResetFilters, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-o", OpenFile, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-s", SaveFile, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-s", SaveFileToDownloads, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-r", RevealInFinder, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-v", OpenInExternalViewer, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("shift-ctrl-alt-v", OpenInExternalViewerAndQuit, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-e", OpenInExternalEditor, None),
    ]);
}

/// Set up native application menus (macOS menu bar, Windows/Linux menus)
fn setup_menus(cx: &mut gpui::App) {
    cx.set_menus(vec![
        // Application menu (macOS only - shows as "RPView" menu)
        Menu {
            name: "RPView".into(),
            items: vec![
                #[cfg(target_os = "macos")]
                MenuItem::action("Preferences...", ToggleSettings),
                #[cfg(target_os = "macos")]
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        },
        // Edit menu (for Windows/Linux settings)
        #[cfg(not(target_os = "macos"))]
        Menu {
            name: "Edit".into(),
            items: vec![MenuItem::action("Settings...", ToggleSettings)],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Open File...", OpenFile),
                MenuItem::action("Save File...", SaveFile),
                MenuItem::action("Save to Downloads...", SaveFileToDownloads),
                MenuItem::separator(),
                MenuItem::action("Reveal in Finder", RevealInFinder),
                MenuItem::action("Open in External Viewer", OpenInExternalViewer),
                MenuItem::action("Open in Viewer and Quit", OpenInExternalViewerAndQuit),
                MenuItem::action("Open in External Editor", OpenInExternalEditor),
                MenuItem::separator(),
                MenuItem::action("Close Window", CloseWindow),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::action("Reset Zoom", ZoomReset),
                MenuItem::separator(),
                MenuItem::action("Toggle Filters", ToggleFilters),
                MenuItem::action("Disable Filters", DisableFilters),
                MenuItem::action("Enable Filters", EnableFilters),
                MenuItem::action("Reset Filters", ResetFilters),
                MenuItem::separator(),
                MenuItem::action("Toggle Help", ToggleHelp),
                MenuItem::action("Toggle Debug", ToggleDebug),
            ],
        },
        Menu {
            name: "Navigate".into(),
            items: vec![
                MenuItem::action("Next Image", NextImage),
                MenuItem::action("Previous Image", PreviousImage),
                MenuItem::separator(),
                MenuItem::action("Sort Alphabetically", SortAlphabetical),
                MenuItem::action("Sort by Modified Date", SortByModified),
            ],
        },
        Menu {
            name: "Animation".into(),
            items: vec![
                MenuItem::action("Play/Pause", ToggleAnimationPlayPause),
                MenuItem::action("Next Frame", NextFrame),
                MenuItem::action("Previous Frame", PreviousFrame),
            ],
        },
    ]);
}

fn main() {
    // Load settings from disk (or use defaults if file doesn't exist)
    let settings = settings_io::load_settings();
    println!(
        "Settings loaded from: {}",
        settings_io::get_settings_path().display()
    );

    // Parse command-line arguments to get image paths and starting index
    let (image_paths, start_index) = match Cli::parse_image_paths() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Determine the search directory for error messages
    let search_dir = if image_paths.is_empty() {
        // Try to get the directory from command-line args
        let cli_args: Vec<String> = std::env::args().collect();
        if cli_args.len() > 1 {
            let arg_path = std::path::PathBuf::from(&cli_args[1]);
            if arg_path.is_dir() {
                arg_path
            } else if let Some(parent) = arg_path.parent() {
                parent.to_path_buf()
            } else {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            }
        } else {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        }
    } else {
        std::path::PathBuf::new()
    };

    // Initialize application state with the starting index and settings
    let app_state = AppState::new_with_settings(
        image_paths,
        start_index,
        settings.sort_navigation.default_sort_mode.into(),
        settings.viewer_behavior.state_cache_size,
    );

    // Print startup info
    println!("rpview-gpui starting...");
    println!("Loaded {} image(s)", app_state.image_paths.len());
    if let Some(first_image) = app_state.current_image() {
        println!("Current image: {}", first_image.display());
    }

    // Get the first image path to load (or None if no images)
    let first_image_path = app_state.current_image().cloned();

    let application = Application::new();

    // Register the application:openFiles: handler on GPUI's delegate class
    // This must be done after Application::new() creates the delegate class
    #[cfg(target_os = "macos")]
    {
        macos_open_handler::register_open_files_handler();
    }

    // Register handler for macOS "Open With" events
    application.on_open_urls(|urls| {
        let paths: Vec<PathBuf> = urls
            .iter()
            .filter_map(|url| parse_file_url(url))
            .collect();

        if !paths.is_empty() {
            if let Ok(mut pending) = PENDING_OPEN_PATHS.lock() {
                pending.extend(paths);
            }
        }
    });

    // Handle app reactivation (e.g., clicking dock icon while running)
    // This also processes any pending open paths
    application.on_reopen(|cx| {
        check_and_process_pending_paths(cx);
    });

    application.run(move |cx: &mut gpui::App| {
        adabraka_ui::init(cx);

        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        setup_key_bindings(cx);
        setup_menus(cx);

        cx.on_action(|_: &Quit, cx| {
            cx.quit();
        });

        cx.activate(true);

        if let Err(e) = cx.open_window(
            WindowOptions {
                ..Default::default()
            },
            move |window, cx| {
                cx.new::<App>(|inner_cx| {
                    let focus_handle = inner_cx.focus_handle();
                    focus_handle.focus(window);

                    // Create the viewer and load the first image if available
                    let mut viewer = ImageViewer {
                        current_image: None,
                        error_message: None,
                        error_path: None,
                        oversized_image: None,
                        focus_handle: inner_cx.focus_handle(),
                        image_state: state::ImageState::new(),
                        viewport_size: None,
                        z_drag_state: None,
                        spacebar_drag_state: None,
                        preload_paths: Vec::new(),
                        loading_handle: None,
                        is_loading: false,
                        is_processing_filters: false,
                        filter_processing_handle: None,
                        pending_filtered_path: None,
                        pending_filter_preload_frames: 0,
                    };

                    if let Some(ref path) = first_image_path {
                        let max_dim = Some(settings.performance.max_image_dimension);
                        viewer.load_image_async(path.clone(), max_dim, false);
                    } else {
                        // No images found - show error with canonical directory path
                        let canonical_dir = search_dir
                            .canonicalize()
                            .unwrap_or_else(|_| search_dir.clone());
                        viewer.error_message = Some(format!(
                            "No images found in directory:\n{}",
                            canonical_dir.display()
                        ));
                        viewer.error_path = None;
                    }

                    // Set initial window title
                    if let Some(path) = app_state.current_image() {
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        let position = app_state.current_index + 1;
                        let total = app_state.image_paths.len();
                        let title = format!("{} ({}/{})", filename, position, total);
                        window.set_window_title(&title);
                    } else {
                        window.set_window_title("rpview-gpui");
                    }

                    // Create filter controls
                    let filter_controls = inner_cx.new(|cx| {
                        FilterControls::new(
                            viewer.image_state.filters,
                            settings.appearance.overlay_transparency,
                            settings.appearance.font_size_scale,
                            cx,
                        )
                    });

                    // Create settings window
                    let settings_window =
                        inner_cx.new(|cx| SettingsWindow::new(settings.clone(), cx));

                    // Create help overlay
                    let help_overlay = inner_cx.new(|_cx| {
                        HelpOverlay::new(
                            settings.appearance.overlay_transparency,
                            settings.appearance.font_size_scale,
                        )
                    });

                    // Create menu bar for Windows/Linux
                    #[cfg(not(target_os = "macos"))]
                    let menu_bar = cx.new(|cx| components::MenuBar::new(cx));

                    App {
                        app_state,
                        viewer,
                        focus_handle,
                        escape_presses: Vec::new(),
                        z_key_held: false,
                        spacebar_held: false,
                        mouse_button_down: false,
                        show_help: false,
                        show_debug: false,
                        show_settings: false,
                        show_filters: false,
                        filter_controls,
                        settings_window,
                        help_overlay,
                        #[cfg(not(target_os = "macos"))]
                        menu_bar,
                        last_frame_update: Instant::now(),
                        drag_over: false,
                        settings: settings.clone(),
                    }
                })
            },
        ) {
            eprintln!("Failed to open window: {:?}", e);
            return;
        }

        // Check for pending open paths from macOS "Open With" events
        // Use defer to ensure the window is fully set up first
        cx.defer(|cx| {
            check_and_process_pending_paths(cx);
        });

        // Set up a recurring timer to check for pending open paths
        // This handles the case where files are opened while the app is already running
        let executor = cx.background_executor().clone();
        cx.spawn(async move |cx| {
            loop {
                executor.timer(Duration::from_millis(250)).await;

                let has_pending = PENDING_OPEN_PATHS.lock()
                    .map(|p| !p.is_empty())
                    .unwrap_or(false);

                #[cfg(target_os = "macos")]
                let has_pending = has_pending || macos_open_handler::has_pending_paths();

                if has_pending {
                    let _ = cx.update(|cx| {
                        check_and_process_pending_paths(cx);
                    });
                }
            }
        }).detach();
    });
}

/// Helper function to check for and process pending file open paths
fn check_and_process_pending_paths(cx: &mut gpui::App) {
    if let Some(window) = cx.windows().first() {
        let _ = window.update(cx, |view, window, cx| {
            if let Ok(app) = view.downcast::<App>() {
                app.update(cx, |app, cx| {
                    app.process_pending_open_paths(window, cx);
                });
            }
        });
    }
}
