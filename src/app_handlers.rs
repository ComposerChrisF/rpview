use super::*;
use crate::utils::debug_eprintln;
use crate::utils::file_scanner::SUPPORTED_EXTENSIONS;

impl App {
    /// Check if modal overlays (settings, delete confirmation) are blocking main window interactions
    /// Note: Menu bar state is handled separately via escape key
    pub(crate) fn is_modal_open(&self) -> bool {
        self.show_settings || self.pending_delete.is_some()
    }

    pub(crate) fn handle_escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Close menu bar if open (Windows/Linux)
        #[cfg(not(target_os = "macos"))]
        {
            let menu_open = self.menu_bar.read_with(cx, |mb, _| mb.is_menu_open());
            if menu_open {
                self.menu_bar.update(cx, |mb, cx| mb.close_menu(cx));
                return;
            }
        }

        // Dismiss delete confirmation first (highest priority)
        if self.pending_delete.is_some() {
            self.pending_delete = None;
            self.toast = Some(ToastState {
                message: "Delete cancelled".into(),
                detail: None,
                is_error: false,
                created_at: Instant::now(),
            });
            cx.notify();
            return;
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

        self.register_escape_press(cx);
    }

    /// Record an ESC press for the 3-presses-in-2-seconds quit shortcut.
    /// Used both by the main-window handler above and by floating sub-
    /// windows (Filter / Local Contrast) — an ESC in any of our windows
    /// counts toward the same quit counter.
    pub(crate) fn register_escape_press(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        self.escape_presses
            .retain(|&time| now.duration_since(time) < Duration::from_secs(2));
        self.escape_presses.push(now);
        if self.escape_presses.len() >= 3 {
            cx.quit();
        }
    }

    pub(crate) fn handle_toggle_help(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_help = !self.show_help;
        cx.notify();
    }

    pub(crate) fn handle_toggle_debug(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_debug = !self.show_debug;
        cx.notify();
    }

    pub(crate) fn handle_toggle_zoom_indicator(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_zoom_indicator = !self.show_zoom_indicator;
        cx.notify();
    }

    pub(crate) fn handle_toggle_background(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settings.appearance.use_light_background =
            !self.settings.appearance.use_light_background;
        if let Err(e) = settings_io::save_settings(&self.settings) {
            eprintln!("Error saving settings: {}", e);
        }
        cx.notify();
    }

    pub(crate) fn handle_toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(crate) fn handle_close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

        // Immediately apply new window title format
        self.update_window_title(window);

        cx.notify();
    }

    pub(crate) fn handle_reset_settings_to_defaults(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Reset settings window to defaults
        self.settings_window.update(cx, |sw, cx| {
            sw.reset_to_defaults(cx);
        });

        cx.notify();
    }

    pub(crate) fn handle_load_oversized_image_anyway(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    pub(crate) fn handle_toggle_filters(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.filter_window.is_some() {
            self.close_filter_window(cx);
        } else {
            self.open_filter_window(cx);
        }
        cx.notify();
    }

    /// Spawn the floating, always-on-top filter window. Does nothing if one is already open.
    pub(crate) fn open_filter_window(&mut self, cx: &mut Context<Self>) {
        if self.filter_window.is_some() {
            return;
        }
        let bounds = self
            .settings
            .appearance
            .filter_window_bounds
            .map(|b| b.to_bounds())
            .unwrap_or_else(|| {
                gpui::Bounds::centered(None, gpui::size(gpui::px(360.0), gpui::px(320.0)), cx)
            });

        let filter_controls = self.filter_controls.clone();
        let weak_app = cx.weak_entity();

        let result = cx.open_window(
            gpui::WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
                kind: gpui::WindowKind::Floating,
                is_resizable: true,
                is_movable: true,
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Filters".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |window, cx| {
                crate::utils::window_level::set_always_on_top(window);

                let weak_for_escape = weak_app.clone();
                let on_escape: crate::components::EscapeCallback =
                    Box::new(move |window, app_cx| {
                        window.remove_window();
                        let _ = weak_for_escape.update(app_cx, |app, inner_cx| {
                            app.register_escape_press(inner_cx);
                        });
                    });
                let view = cx.new(|inner_cx| {
                    FilterWindowView::new(filter_controls.clone(), on_escape, inner_cx)
                });

                // Focus the window root so its `EscapePressed` handler lands in
                // the key-dispatch path (see the GPU Pipeline window for the full
                // rationale). Otherwise ESC can't dismiss this window either.
                view.read(cx).focus_handle.clone().focus(window);

                // Persist bounds on move/resize.
                let weak_for_bounds = weak_app.clone();
                view.update(cx, |_, inner_cx| {
                    inner_cx
                        .observe_window_bounds(window, move |_, window, cx| {
                            let bounds = window.bounds();
                            let _ = weak_for_bounds.update(cx, |app, _| {
                                app.settings.appearance.filter_window_bounds = Some(
                                    crate::state::settings::PersistedWindowBounds::from_bounds(
                                        bounds,
                                    ),
                                );
                                crate::utils::settings_io::save_settings_debounced(&app.settings);
                            });
                        })
                        .detach();

                    // Clear the handle on the main App when this view (and thus its window) drops.
                    let weak_for_close = weak_app.clone();
                    inner_cx
                        .on_release(move |_, app_cx| {
                            let _ = weak_for_close.update(app_cx, |app, _| {
                                app.filter_window = None;
                                app.settings.appearance.filter_window_open = false;
                                let _ = crate::utils::settings_io::save_settings(&app.settings);
                            });
                        })
                        .detach();
                });

                view
            },
        );

        match result {
            Ok(handle) => {
                self.filter_window = Some(handle);
                self.settings.appearance.filter_window_open = true;
                let _ = crate::utils::settings_io::save_settings(&self.settings);
            }
            Err(e) => {
                eprintln!("Failed to open filter window: {:?}", e);
            }
        }
    }

    /// Close the floating filter window if one is open.
    pub(crate) fn close_filter_window(&mut self, cx: &mut Context<Self>) {
        if let Some(handle) = self.filter_window.take() {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            self.settings.appearance.filter_window_open = false;
            let _ = crate::utils::settings_io::save_settings(&self.settings);
        }
    }

    pub(crate) fn handle_toggle_gpu_pipeline(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.gpu_pipeline_window.is_some() {
            self.close_gpu_pipeline_window(cx);
        } else {
            self.open_gpu_pipeline_window(cx);
        }
        cx.notify();
    }

    pub(crate) fn handle_reset_gpu_pipeline(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.gpu_pipeline_controls
            .update(cx, |c, cx| c.reset_all(cx));
        self.viewer.reset_gpu_pipeline();
        cx.notify();
    }

    pub(crate) fn open_gpu_pipeline_window(&mut self, cx: &mut Context<Self>) {
        if self.gpu_pipeline_window.is_some() {
            return;
        }
        let bounds = self
            .settings
            .appearance
            .gpu_pipeline_window_bounds
            .map(|b| b.to_bounds())
            .unwrap_or_else(|| {
                gpui::Bounds::centered(None, gpui::size(gpui::px(340.0), gpui::px(620.0)), cx)
            });
        let controls = self.gpu_pipeline_controls.clone();
        let weak_app = cx.weak_entity();

        let result = cx.open_window(
            gpui::WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
                kind: gpui::WindowKind::Floating,
                is_resizable: true,
                is_movable: true,
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("GPU Pipeline".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |window, cx| {
                crate::utils::window_level::set_always_on_top(window);
                let weak_for_escape = weak_app.clone();
                let on_escape: crate::components::EscapeCallback =
                    Box::new(move |window, app_cx| {
                        window.remove_window();
                        let _ = weak_for_escape.update(app_cx, |app, inner_cx| {
                            app.register_escape_press(inner_cx);
                        });
                    });
                let view = cx.new(|inner_cx| {
                    crate::components::GpuPipelineWindowView::new(
                        controls.clone(),
                        on_escape,
                        inner_cx,
                    )
                });

                // Explicitly focus the window's root. Without this the window's
                // focus stays `None`, so GPUI dispatches keystrokes from the
                // synthetic dispatch-tree root — which sits *above* our root
                // `<div>` and its `EscapePressed` handler. The result: ESC never
                // reaches `on_escape` and the window can't be dismissed.
                view.read(cx).focus_handle.clone().focus(window);

                // Persist bounds on move/resize.
                let weak_for_bounds = weak_app.clone();
                view.update(cx, |_, inner_cx| {
                    inner_cx
                        .observe_window_bounds(window, move |_, window, cx| {
                            let bounds = window.bounds();
                            let _ = weak_for_bounds.update(cx, |app, _| {
                                app.settings.appearance.gpu_pipeline_window_bounds = Some(
                                    crate::state::settings::PersistedWindowBounds::from_bounds(
                                        bounds,
                                    ),
                                );
                                crate::utils::settings_io::save_settings_debounced(&app.settings);
                            });
                        })
                        .detach();

                    // Clear the handle + persisted-open flag when the view (window) drops.
                    let weak_for_close = weak_app.clone();
                    inner_cx
                        .on_release(move |_, app_cx| {
                            let _ = weak_for_close.update(app_cx, |app, _| {
                                app.gpu_pipeline_window = None;
                                app.settings.appearance.gpu_pipeline_window_open = false;
                                let _ = crate::utils::settings_io::save_settings(&app.settings);
                            });
                        })
                        .detach();
                });
                view
            },
        );
        match result {
            Ok(handle) => {
                self.gpu_pipeline_window = Some(handle);
                self.settings.appearance.gpu_pipeline_window_open = true;
                let _ = crate::utils::settings_io::save_settings(&self.settings);
            }
            Err(e) => eprintln!("Failed to open GPU Pipeline window: {:?}", e),
        }
    }

    pub(crate) fn close_gpu_pipeline_window(&mut self, cx: &mut Context<Self>) {
        if let Some(handle) = self.gpu_pipeline_window.take() {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            self.settings.appearance.gpu_pipeline_window_open = false;
            let _ = crate::utils::settings_io::save_settings(&self.settings);
        }
    }

    pub(crate) fn handle_disable_filters(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.clear_active_slot();
        self.viewer.image_state.filters_enabled = false;
        self.viewer.update_filtered_cache();
        self.viewer.set_gpu_pipeline_enabled(false);
        self.save_current_image_state();
        cx.notify();
    }

    pub(crate) fn handle_enable_filters(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.clear_active_slot();
        self.viewer.image_state.filters_enabled = true;
        self.viewer.update_filtered_cache();
        self.viewer.set_gpu_pipeline_enabled(true);
        self.save_current_image_state();
        cx.notify();
    }

    pub(crate) fn handle_store_slot(
        &mut self,
        slot: u8,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.viewer.store_slot(slot);
        cx.notify();
    }

    pub(crate) fn handle_recall_slot(
        &mut self,
        slot: u8,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.viewer.recall_slot(slot);
        cx.notify();
    }

    pub(crate) fn handle_reset_filters(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
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

    fn adjust_filter(
        &mut self,
        f: impl FnOnce(&mut state::image_state::FilterSettings),
        cx: &mut Context<Self>,
    ) {
        if self.is_modal_open() {
            return;
        }
        f(&mut self.viewer.image_state.filters);
        self.viewer.update_filtered_cache();
        self.save_current_image_state();
        cx.notify();
    }

    pub(crate) fn handle_brightness_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.adjust_filter(|f| f.brightness = (f.brightness + 5.0).min(100.0), cx);
    }

    pub(crate) fn handle_brightness_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.adjust_filter(|f| f.brightness = (f.brightness - 5.0).max(-100.0), cx);
    }

    pub(crate) fn handle_contrast_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.adjust_filter(|f| f.contrast = (f.contrast + 5.0).min(100.0), cx);
    }

    pub(crate) fn handle_contrast_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.adjust_filter(|f| f.contrast = (f.contrast - 5.0).max(-100.0), cx);
    }

    pub(crate) fn handle_gamma_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.adjust_filter(|f| f.gamma = (f.gamma + 0.1).min(10.0), cx);
    }

    pub(crate) fn handle_gamma_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.adjust_filter(|f| f.gamma = (f.gamma - 0.1).max(0.1), cx);
    }

    pub(crate) fn handle_open_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        // Open native file dialog for image selection (single file)
        let mut file_dialog = rfd::FileDialog::new()
            .add_filter("Images", SUPPORTED_EXTENSIONS)
            .set_title("Open Image");

        // Set default directory to current image's parent directory if available,
        // or to the no_images_path directory if we're showing the empty directory notice
        if let Some(current_path) = self.app_state.current_image() {
            if let Some(parent) = current_path.parent() {
                file_dialog = file_dialog.set_directory(parent);
            }
        } else if let Some(ref no_images_dir) = self.viewer.no_images_path {
            file_dialog = file_dialog.set_directory(no_images_dir);
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

                    // Re-sort according to the active sort mode
                    self.app_state.sort_images();

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

    pub(crate) fn handle_save_file(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.handle_save_file_impl(None, cx);
    }

    pub(crate) fn handle_save_file_to_downloads(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_modal_open() {
            return;
        }
        // Get the Downloads directory
        let downloads_dir = dirs::download_dir();
        self.handle_save_file_impl(downloads_dir, cx);
    }

    fn handle_save_file_impl(&mut self, default_dir: Option<PathBuf>, cx: &mut Context<Self>) {
        // Only save if we have a current image
        let Some(current_path) = self.app_state.current_image().cloned() else {
            return;
        };

        // Get original filename without extension
        let original_stem = current_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image");

        // Detect whether anything other than the raw source is being
        // displayed.  The bytes we save will come from
        // `viewer.capture_current_display()`, which already mirrors
        // the renderer's priority chain (slot → GPU pipeline → LC →
        // filtered → raw), so we only need to distinguish "raw
        // fallback" from "processed/recalled" here.
        let active_slot = self.viewer.active_slot.is_some();
        let gpu_pipeline_active = self.viewer.gpu_pipeline_enabled
            && self
                .viewer
                .current_image
                .as_ref()
                .is_some_and(|i| i.gpu_pipeline_render.is_some());
        let filters_active = self.viewer.image_state.filters_enabled
            && self
                .viewer
                .current_image
                .as_ref()
                .is_some_and(|i| i.filtered_render.is_some());
        let any_processing = active_slot || gpu_pipeline_active || filters_active;

        // Determine extension from settings when any processing is active
        let save_ext = if any_processing {
            use crate::state::settings::SaveFormat;
            match self.settings.file_operations.default_save_format {
                SaveFormat::SameAsLoaded => current_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("png"),
                SaveFormat::Png => "png",
                SaveFormat::Jpeg => "jpg",
                SaveFormat::Bmp => "bmp",
                SaveFormat::Tiff => "tiff",
                SaveFormat::Webp => "webp",
            }
        } else {
            current_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png")
        };

        // Generate suggested filename with `_filtered` suffix when processing is active.
        let suggested_name = if any_processing {
            format!("{}_filtered.{}", original_stem, save_ext)
        } else {
            format!("{}.{}", original_stem, save_ext)
        };

        // Resolve directory: arg → settings default → original parent.
        let directory: Option<PathBuf> = default_dir
            .or_else(|| self.settings.file_operations.default_save_directory.clone())
            .or_else(|| current_path.parent().map(PathBuf::from));

        // Capture all bytes needed for the eventual write *now*, while we
        // still hold `&mut self`.  The async block below cannot reach back
        // into the viewer to do this — the dialog is non-blocking precisely
        // so AppKit can dispatch other actions (incl. NextImage on right
        // arrow) which would race with a re-borrow of `self`.
        enum SaveSource {
            Processed {
                rgba: Vec<u8>,
                width: u32,
                height: u32,
            },
            OriginalCopy {
                source_path: PathBuf,
            },
        }
        let source: Option<SaveSource> = if any_processing {
            self.viewer.capture_current_display().and_then(|snapshot| {
                snapshot.render.as_bytes(0).map(|bgra| {
                    let mut rgba = bgra.to_vec();
                    for px in rgba.chunks_exact_mut(4) {
                        px.swap(0, 2);
                    }
                    SaveSource::Processed {
                        rgba,
                        width: snapshot.width,
                        height: snapshot.height,
                    }
                })
            })
        } else {
            self.viewer
                .current_image
                .as_ref()
                .map(|img| SaveSource::OriginalCopy {
                    source_path: img.path.clone(),
                })
        };
        let Some(source) = source else {
            eprintln!("[Save] Nothing to save (no current image / no display data)");
            return;
        };

        // Hand off the dialog + write to a foreground task.  Returns
        // immediately, releasing `&mut self` — when AppKit dispatches a key
        // event during the modal it goes straight to NSSavePanel (which
        // consumes arrow keys for file-list navigation) instead of trying
        // to re-enter our action handlers.
        cx.spawn(async move |_, _cx| {
            let mut dialog = rfd::AsyncFileDialog::new()
                .add_filter("PNG", &["png"])
                .add_filter("JPEG", &["jpg", "jpeg"])
                .add_filter("BMP", &["bmp"])
                .add_filter("TIFF", &["tiff", "tif"])
                .add_filter("WEBP", &["webp"])
                .set_file_name(&suggested_name)
                .set_title("Save Image");
            if let Some(dir) = directory {
                dialog = dialog.set_directory(dir);
            }
            let Some(handle) = dialog.save_file().await else {
                return;
            };
            let save_path = handle.path().to_path_buf();

            let result: Result<(), String> = match source {
                SaveSource::Processed {
                    rgba,
                    width,
                    height,
                } => match image::RgbaImage::from_raw(width, height, rgba) {
                    Some(img) => save_dynamic_image_to_path(
                        &image::DynamicImage::ImageRgba8(img),
                        &save_path,
                    ),
                    None => Err("Display buffer size mismatch".to_string()),
                },
                SaveSource::OriginalCopy { source_path } => {
                    let parent = save_path.parent().unwrap_or(&save_path);
                    tempfile::NamedTempFile::new_in(parent)
                        .map_err(|e| format!("Failed to create temp file: {}", e))
                        .and_then(|temp_file| {
                            std::fs::copy(&source_path, temp_file.path())
                                .map_err(|e| format!("Failed to copy image: {}", e))?;
                            temp_file
                                .persist(&save_path)
                                .map(|_| ())
                                .map_err(|e| format!("Failed to finalize save: {}", e))
                        })
                }
            };

            match result {
                Ok(()) => println!("Image saved to: {}", save_path.display()),
                Err(e) => eprintln!("Failed to save image: {}", e),
            }
        })
        .detach();
    }
}

/// Write `image_data` to `save_path` atomically (temp file + rename) using
/// the format inferred from `save_path`'s extension.  Free function — has
/// no `&self` dependency, callable from spawned futures that don't hold an
/// App borrow.
fn save_dynamic_image_to_path(
    image_data: &image::DynamicImage,
    save_path: &Path,
) -> Result<(), String> {
    let parent = save_path.parent().unwrap_or(save_path);
    let temp_file = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    let temp_path = temp_file.path().to_path_buf();

    let extension = save_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();

    let save_result = match extension.as_str() {
        "png" => image_data.save_with_format(&temp_path, image::ImageFormat::Png),
        "jpg" | "jpeg" => {
            // JPEG has no alpha channel.
            image_data
                .to_rgb8()
                .save_with_format(&temp_path, image::ImageFormat::Jpeg)
        }
        "bmp" => image_data.save_with_format(&temp_path, image::ImageFormat::Bmp),
        "tiff" | "tif" => image_data.save_with_format(&temp_path, image::ImageFormat::Tiff),
        "webp" => image_data.save_with_format(&temp_path, image::ImageFormat::WebP),
        // Default to PNG for unknown extensions.
        _ => image_data.save_with_format(&temp_path, image::ImageFormat::Png),
    };

    save_result.map_err(|e| format!("Failed to save image: {}", e))?;

    // Atomic rename to final destination.
    temp_file
        .persist(save_path)
        .map(|_| ())
        .map_err(|e| format!("Failed to finalize save: {}", e))
}

impl App {
    pub(crate) fn handle_open_in_external_viewer(
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
            }
        }
        cx.notify();
    }

    pub(crate) fn handle_open_in_external_viewer_and_quit(
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

    pub(crate) fn handle_open_in_external_editor(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    pub(crate) fn handle_reveal_in_finder(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
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

    pub(crate) fn handle_request_delete(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        if self.app_state.current_image().is_none() {
            return;
        }
        self.pending_delete = Some(DeleteMode::Trash);
        cx.notify();
    }

    pub(crate) fn handle_request_permanent_delete(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_modal_open() {
            return;
        }
        if self.app_state.current_image().is_none() {
            return;
        }
        self.pending_delete = Some(DeleteMode::Permanent);
        cx.notify();
    }

    pub(crate) fn handle_confirm_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mode = match self.pending_delete {
            Some(m) => m,
            None => return,
        };

        let path = match self.app_state.current_image().cloned() {
            Some(p) => p,
            None => {
                self.pending_delete = None;
                cx.notify();
                return;
            }
        };

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let full_path = path.display().to_string();

        let result = match mode {
            DeleteMode::Trash => trash::delete(&path).map_err(|e| e.to_string()),
            DeleteMode::Permanent => std::fs::remove_file(&path).map_err(|e| e.to_string()),
        };

        match result {
            Ok(()) => {
                let action_word = match mode {
                    DeleteMode::Trash => "Moved to Trash",
                    DeleteMode::Permanent => "Permanently deleted",
                };
                self.toast = Some(ToastState {
                    message: format!("{}: {}", action_word, filename),
                    detail: Some(full_path),
                    is_error: false,
                    created_at: Instant::now(),
                });
                self.app_state.remove_current_image();
                self.update_viewer(window, cx);
                self.update_window_title(window);
            }
            Err(e) => {
                self.toast = Some(ToastState {
                    message: format!("Delete failed: {}", e),
                    detail: Some(full_path),
                    is_error: true,
                    created_at: Instant::now(),
                });
            }
        }

        self.pending_delete = None;
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
    fn open_in_system_viewer(&self, image_path: &Path) -> Result<(), String> {
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
                    debug_eprintln!("Opened image with: {}", viewer_config.name);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Failed to launch {}: {}", viewer_config.name, e);
                    // Continue to next viewer
                }
            }
        }

        // All configured viewers failed, try platform defaults as fallback
        debug_eprintln!("All configured viewers failed, trying platform defaults...");

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

            debug_eprintln!("Opened image in external editor: {}", editor_config.name);
            Ok(())
        } else {
            Err("No external editor configured. Please set one in Settings (Cmd+,)".to_string())
        }
    }

    fn import_image_paths(
        &mut self,
        paths: &[PathBuf],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut all_images: Vec<PathBuf> = Vec::new();
        let mut target_index: usize = 0;

        if paths.len() == 1 {
            if let Ok((images, index)) = utils::file_scanner::process_dropped_path(&paths[0]) {
                all_images = images;
                target_index = index;
            }
        } else {
            for path in paths {
                if path.is_file() && utils::file_scanner::is_supported_image(path) {
                    all_images.push(path.to_path_buf());
                } else if path.is_dir() {
                    if let Ok(dir_images) = utils::file_scanner::scan_directory(path) {
                        all_images.extend(dir_images);
                    }
                }
            }
            // Dedup using a set instead of sort+dedup+sort
            let set: std::collections::HashSet<PathBuf> = all_images.drain(..).collect();
            all_images = set.into_iter().collect();
            utils::file_scanner::sort_alphabetically(&mut all_images);
        }

        if !all_images.is_empty() {
            // Set the paths and a temporary index pointing at the target file
            self.app_state.image_paths = all_images;
            self.app_state.current_index = target_index;

            // Re-sort according to the active sort mode (process_dropped_path
            // always sorts alphabetically; this corrects for ModifiedDate mode).
            // sort_images() preserves current_index by tracking the current path.
            self.app_state.sort_images();

            self.update_viewer(window, cx);
            self.update_window_title(window);
            self.focus_handle.focus(window);
            cx.notify();
        }
    }

    pub(crate) fn handle_dropped_files(
        &mut self,
        paths: &ExternalPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dropped: Vec<PathBuf> = paths.paths().to_vec();
        self.import_image_paths(&dropped, window, cx);
    }

    /// Check for and process any pending file open requests from macOS "Open With" events.
    pub(crate) fn process_pending_open_paths(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[allow(unused_mut)]
        let mut pending_paths: Vec<PathBuf> = {
            let Ok(mut pending) = PENDING_OPEN_PATHS.lock() else {
                return;
            };
            std::mem::take(&mut *pending)
        };

        #[cfg(target_os = "macos")]
        {
            pending_paths.extend(macos_open_handler::take_pending_paths());
        }

        if pending_paths.is_empty() {
            return;
        }

        self.import_image_paths(&pending_paths, window, cx);
    }

    pub(crate) fn handle_next_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }

        let wrap = self.settings.sort_navigation.wrap_navigation;
        self.app_state.next_image_with_wrap(wrap);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }

    pub(crate) fn handle_previous_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let wrap = self.settings.sort_navigation.wrap_navigation;
        self.app_state.previous_image_with_wrap(wrap);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }

    pub(crate) fn handle_toggle_animation(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        // Playback runs regardless of LC batch progress; unfilled frames
        // fall back to the unprocessed source.
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            anim_state.is_playing = !anim_state.is_playing;
            if anim_state.is_playing {
                // Reset timer when starting playback
                self.last_frame_update = Instant::now();
            }
            cx.notify();
        }
    }

    pub(crate) fn handle_next_frame(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            // Pause animation when manually navigating frames
            anim_state.is_playing = false;
        }
        if let Some(ref anim) = self.viewer.image_state.animation {
            let next = (anim.current_frame + 1) % anim.frame_count;
            // Goes through set_current_frame so the rescale fires when the
            // new frame's effective dimensions differ.
            self.viewer.set_current_frame(next);
        }
        // GPU pipeline output is per-frame; re-run on the new frame.
        self.reapply_gpu_pipeline_if_active(cx);
        cx.notify();
    }

    pub(crate) fn handle_previous_frame(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        if let Some(ref mut anim_state) = self.viewer.image_state.animation {
            anim_state.is_playing = false;
        }
        if let Some(ref anim) = self.viewer.image_state.animation {
            let prev = if anim.current_frame == 0 {
                anim.frame_count - 1
            } else {
                anim.current_frame - 1
            };
            self.viewer.set_current_frame(prev);
        }
        // GPU pipeline output is per-frame; re-run on the new frame.
        self.reapply_gpu_pipeline_if_active(cx);
        cx.notify();
    }

    pub(crate) fn handle_sort_alphabetical(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.app_state.set_sort_mode(state::SortMode::Alphabetical);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }

    pub(crate) fn handle_sort_by_modified(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        self.app_state.set_sort_mode(state::SortMode::ModifiedDate);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }

    pub(crate) fn handle_sort_by_type_toggle(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_modal_open() {
            return;
        }
        let next = match self.app_state.sort_mode {
            state::SortMode::Alphabetical => state::SortMode::TypeAlpha,
            state::SortMode::ModifiedDate => state::SortMode::TypeModified,
            state::SortMode::TypeAlpha => state::SortMode::TypeModified,
            state::SortMode::TypeModified => state::SortMode::TypeAlpha,
        };
        self.app_state.set_sort_mode(next);
        self.update_viewer(window, cx);
        self.update_window_title(window);
        cx.notify();
    }

    fn do_zoom(&mut self, zoom_fn: impl FnOnce(&mut ImageViewer), cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        zoom_fn(&mut self.viewer);
        self.save_current_image_state();
        cx.notify();
    }

    pub(crate) fn handle_zoom_in(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_zoom(|v| v.zoom_in(utils::zoom::ZOOM_STEP), cx);
    }

    pub(crate) fn handle_zoom_out(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_zoom(|v| v.zoom_out(utils::zoom::ZOOM_STEP), cx);
    }

    pub(crate) fn handle_zoom_reset(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_zoom(|v| v.reset_zoom(), cx);
    }

    pub(crate) fn handle_zoom_reset_and_center(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.do_zoom(|v| v.reset_zoom_and_pan(), cx);
    }

    pub(crate) fn handle_zoom_in_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_zoom(|v| v.zoom_in(utils::zoom::ZOOM_STEP_FAST), cx);
    }

    pub(crate) fn handle_zoom_out_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_zoom(|v| v.zoom_out(utils::zoom::ZOOM_STEP_FAST), cx);
    }

    pub(crate) fn handle_zoom_in_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_zoom(|v| v.zoom_in(utils::zoom::ZOOM_STEP_SLOW), cx);
    }

    pub(crate) fn handle_zoom_out_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_zoom(|v| v.zoom_out(utils::zoom::ZOOM_STEP_SLOW), cx);
    }

    pub(crate) fn handle_zoom_in_incremental(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.do_zoom(
            |v| {
                let new_zoom = utils::zoom::clamp_zoom(
                    v.image_state.zoom + utils::zoom::ZOOM_STEP_INCREMENTAL,
                );
                v.image_state.zoom = new_zoom;
                v.image_state.is_fit_to_window = false;
            },
            cx,
        );
    }

    pub(crate) fn handle_zoom_out_incremental(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.do_zoom(
            |v| {
                let new_zoom = utils::zoom::clamp_zoom(
                    v.image_state.zoom - utils::zoom::ZOOM_STEP_INCREMENTAL,
                );
                v.image_state.zoom = new_zoom;
                v.image_state.is_fit_to_window = false;
            },
            cx,
        );
    }

    /// Returns the sign multiplier for pan direction based on the user's preference.
    fn pan_sign(&self) -> f32 {
        use crate::state::settings::PanDirectionMode;
        match self.settings.keyboard_mouse.pan_direction_mode {
            PanDirectionMode::MoveImage => -1.0,
            PanDirectionMode::MoveViewport => 1.0,
        }
    }

    fn do_pan(&mut self, dx: f32, dy: f32, speed: f32, cx: &mut Context<Self>) {
        if self.is_modal_open() {
            return;
        }
        let sign = self.pan_sign();
        self.viewer.pan(dx * sign * speed, dy * sign * speed);
        self.save_current_image_state();
        cx.notify();
    }

    pub(crate) fn handle_pan_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(0.0, 1.0, self.settings.keyboard_mouse.pan_speed_normal, cx);
    }

    pub(crate) fn handle_pan_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(0.0, -1.0, self.settings.keyboard_mouse.pan_speed_normal, cx);
    }

    pub(crate) fn handle_pan_left(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(1.0, 0.0, self.settings.keyboard_mouse.pan_speed_normal, cx);
    }

    pub(crate) fn handle_pan_right(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(-1.0, 0.0, self.settings.keyboard_mouse.pan_speed_normal, cx);
    }

    /// Fast-pan speed is measured in image pixels (pre-zoom), so the on-screen
    /// distance grows with zoom. Multiplying by `image_state.zoom` converts the
    /// image-pixel step into screen pixels for `viewer.pan()`.
    fn fast_pan_speed(&self) -> f32 {
        self.settings.keyboard_mouse.pan_speed_fast * self.viewer.image_state.zoom
    }

    pub(crate) fn handle_pan_up_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(0.0, 1.0, self.fast_pan_speed(), cx);
    }

    pub(crate) fn handle_pan_down_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(0.0, -1.0, self.fast_pan_speed(), cx);
    }

    pub(crate) fn handle_pan_left_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(1.0, 0.0, self.fast_pan_speed(), cx);
    }

    pub(crate) fn handle_pan_right_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(-1.0, 0.0, self.fast_pan_speed(), cx);
    }

    pub(crate) fn handle_pan_up_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(0.0, 1.0, self.settings.keyboard_mouse.pan_speed_slow, cx);
    }

    pub(crate) fn handle_pan_down_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(0.0, -1.0, self.settings.keyboard_mouse.pan_speed_slow, cx);
    }

    pub(crate) fn handle_pan_left_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(1.0, 0.0, self.settings.keyboard_mouse.pan_speed_slow, cx);
    }

    pub(crate) fn handle_pan_right_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_pan(-1.0, 0.0, self.settings.keyboard_mouse.pan_speed_slow, cx);
    }

    pub(crate) fn save_current_image_state(&mut self) {
        // Notify SVG re-raster system of zoom/pan change
        self.viewer.notify_svg_zoom_pan_changed();

        // Only save state if enabled in settings
        if self.settings.viewer_behavior.remember_per_image_state {
            let state = self.viewer.get_image_state();
            self.app_state.save_current_state(state);
        }
    }

    pub(crate) fn load_current_image_state(&mut self, cx: &mut Context<Self>) {
        let default_filters = state::image_state::FilterSettings {
            brightness: self.settings.filters.default_brightness,
            contrast: self.settings.filters.default_contrast,
            gamma: self.settings.filters.default_gamma,
        };
        let state = self.app_state.get_current_state(default_filters);
        let filters = state.filters;
        let filters_enabled = state.filters_enabled;
        self.viewer.set_image_state(state); // move, no clone

        // Update filter controls UI to reflect the loaded filter values
        self.filter_controls.update(cx, |controls, cx| {
            controls.update_from_filters(filters, cx);
        });

        // Re-apply filters to the newly-loaded image if they're non-default.
        // This costs one LUT pass (fast in-memory) — no longer requires disk I/O.
        if filters_enabled
            && (filters.brightness.abs() >= 0.001
                || filters.contrast.abs() >= 0.001
                || (filters.gamma - 1.0).abs() >= 0.001)
        {
            self.viewer.update_filtered_cache();
        }
    }

    /// Re-run the GPU pipeline on the current image when the user is on
    /// the "show processed" side of the 1/2 toggle (`gpu_pipeline_enabled`)
    /// and the pipeline has a real effect (some stage enabled or resize
    /// != 1×).  Called after every successful image load so the GPU
    /// pipeline "follows" the user across images.  When the user has
    /// pressed `1` to view the source, this is a no-op — that's the whole
    /// point of the toggle: it expresses intent across navigation.
    pub(crate) fn reapply_gpu_pipeline_if_active(&mut self, cx: &mut Context<Self>) {
        if !self.viewer.gpu_pipeline_enabled {
            return;
        }
        let params = self.gpu_pipeline_controls.read(cx).get_params(cx);
        let resize_active = (params.resize_factor - 1.0).abs() > 0.001;
        if params.is_identity() && !resize_active {
            return;
        }
        self.viewer.update_gpu_pipeline(params);
    }

    pub(crate) fn update_viewer(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
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
            self.viewer.load_image_async(path, max_dim, force_load);

            // State will be loaded when async load completes (in render loop)
        } else {
            self.viewer.clear();
        }
    }

    pub(crate) fn update_window_title(&mut self, window: &mut Window) {
        let title = crate::window_title::format_window_title(
            self.app_state.current_image().map(|p| p.as_path()),
            self.app_state.current_index,
            self.app_state.image_paths.len(),
            self.app_state.sort_mode,
            &self.settings,
        );
        window.set_window_title(&title);
    }
}
