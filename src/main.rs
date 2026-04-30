#![allow(clippy::collapsible_if)]
// GPUI derive macros (e.g., IntoElement, Render) expand deeply; 128 is not enough.
#![recursion_limit = "256"]

use gpui::prelude::FluentBuilder;
use gpui::*;
use std::path::{Path, PathBuf};
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
    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

mod app_handlers;
mod app_keybindings;
mod app_render;
mod cli;
mod components;
mod error;
mod state;
mod utils;
mod window_title;

use cli::Cli;
use components::{
    DebugOverlay, DebugOverlayConfig, FilterControls, FilterControlsEvent, FilterWindowView,
    HelpOverlay, ImageViewer, LocalContrastControls, LocalContrastControlsEvent,
    LocalContrastWindowView, SettingsWindow,
};
use state::{AppSettings, AppState};
use utils::debug_eprintln;
use utils::settings_io;

// Import all actions from lib.rs (they're defined there to avoid duplication)
use rpview::{
    ApplyLocalContrast, ApplyLocalContrastAll, BrightnessDown, BrightnessUp, CloseSettings, CloseWindow, ConfirmDelete,
    ContrastDown, ContrastUp, DisableFilters, EnableFilters, EscapePressed, GammaDown, GammaUp,
    NextFrame, NextImage, OpenFile, OpenInExternalEditor, OpenInExternalViewer,
    OpenInExternalViewerAndQuit,
    PanDown, PanDownFast, PanDownSlow, PanLeft, PanLeftFast, PanLeftSlow, PanRight, PanRightFast,
    PanRightSlow, PanUp, PanUpFast, PanUpSlow, PreviousFrame, PreviousImage, Quit, RecallSlot3,
    RecallSlot4, RecallSlot5, RecallSlot6, RecallSlot7, RecallSlot8, RecallSlot9, RequestDelete,
    RequestPermanentDelete, ResetFilters, ResetLocalContrast, ResetSettingsToDefaults,
    RevealInFinder, SaveFile, SaveFileToDownloads, SortAlphabetical, SortByModified,
    SortByTypeToggle, StoreSlot3, StoreSlot4, StoreSlot5, StoreSlot6, StoreSlot7, StoreSlot8,
    StoreSlot9, ToggleAnimationPlayPause, ToggleBackground, ToggleDebug, ToggleFilters, ToggleHelp,
    ToggleLocalContrast, ToggleSettings, ToggleZoomIndicator, ZoomIn, ZoomInFast,
    ZoomInIncremental, ZoomInSlow, ZoomOut, ZoomOutFast, ZoomOutIncremental, ZoomOutSlow,
    ZoomReset, ZoomResetAndCenter,
};

/// What kind of delete is pending
#[derive(Clone, Copy, PartialEq)]
enum DeleteMode {
    Trash,
    Permanent,
}

/// State for a toast notification
#[derive(Clone)]
struct ToastState {
    message: String,
    detail: Option<String>,
    is_error: bool,
    created_at: Instant,
}

pub(crate) struct App {
    app_state: AppState,
    viewer: ImageViewer,
    focus_handle: FocusHandle,
    escape_presses: Vec<Instant>,
    /// Tracks if Z key is currently held down (for Z+drag zoom mode)
    z_key_held: bool,
    /// Tracks if left mouse button is currently pressed
    /// Used with MouseMoveEvent.pressed_button for robust button state tracking
    mouse_button_down: bool,
    /// Whether zoom indicator is visible
    show_zoom_indicator: bool,
    /// Whether help overlay is visible
    show_help: bool,
    /// Whether debug overlay is visible
    show_debug: bool,
    /// Whether settings window is visible
    show_settings: bool,
    /// Open floating filter window handle (None = closed)
    filter_window: Option<WindowHandle<FilterWindowView>>,
    /// Filter controls component (shared between no-window state and the filter window)
    filter_controls: Entity<FilterControls>,
    /// Open floating Local Contrast window handle (None = closed)
    local_contrast_window: Option<WindowHandle<LocalContrastWindowView>>,
    /// Local Contrast controls (sliders). Always exists; lives in the App so
    /// its slider values persist across open/close of the LC window.
    local_contrast_controls: Entity<LocalContrastControls>,
    /// Settings window component
    settings_window: Entity<SettingsWindow>,
    /// Help overlay component
    help_overlay: Entity<HelpOverlay>,
    /// Debug overlay component
    debug_overlay: Entity<DebugOverlay>,
    /// Menu bar component (Windows/Linux only)
    #[cfg(not(target_os = "macos"))]
    menu_bar: Entity<components::MenuBar>,
    /// Last time animation frame was updated (for animation playback)
    last_frame_update: Instant,
    /// Whether files are being dragged over the window
    drag_over: bool,
    /// Pending delete mode (Some = confirmation bar is visible)
    pending_delete: Option<DeleteMode>,
    /// Toast notification (auto-dismisses after ~2.5 seconds)
    toast: Option<ToastState>,
    /// Application settings (loaded on startup)
    settings: AppSettings,
}

// Handler methods, render, and keybindings are in their respective modules:
// - app_handlers.rs: impl App handler methods
// - app_render.rs: impl Render for App
// - app_keybindings.rs: setup_key_bindings(), setup_menus()

fn main() {
    // Parse command-line arguments first.  This makes clap short-circuit on
    // `--help` / `--version` before any settings I/O, so the help output
    // isn't preceded by debug logs in dev builds.
    let (image_paths, start_path) = match Cli::parse_image_paths() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Load settings from disk (or use defaults if file doesn't exist)
    let settings = settings_io::load_settings();
    debug_eprintln!(
        "Settings loaded from: {}",
        settings_io::get_settings_path().display()
    );

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

    // Initialize application state with the starting path and settings
    let app_state = AppState::new_with_settings(
        image_paths,
        start_path,
        settings.sort_navigation.default_sort_mode,
        settings.viewer_behavior.state_cache_size,
    );

    // Print startup info
    debug_eprintln!("rpview starting...");
    debug_eprintln!("Loaded {} image(s)", app_state.image_paths.len());
    if let Some(_first_image) = app_state.current_image() {
        debug_eprintln!("Current image: {}", _first_image.display());
    }

    // Get the first image path to load (or None if no images)
    let first_image_path = app_state.current_image().cloned();

    let application = Application::new();

    // Register the application:openFiles: handler on GPUI's delegate class.
    // Call after `Application::new()` (which constructs the delegate class)
    // but before `application.run()` (which starts dispatching events).
    #[cfg(target_os = "macos")]
    {
        macos_open_handler::register_open_files_handler();
    }

    // Register handler for macOS "Open With" events
    application.on_open_urls(|urls| {
        let paths: Vec<PathBuf> = urls.iter().filter_map(|url| parse_file_url(url)).collect();

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
        ccf_gpui_widgets::register_all_keybindings(cx);

        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        app_keybindings::setup_key_bindings(cx);
        app_keybindings::setup_menus(cx);

        cx.on_action(|_: &Quit, cx| {
            cx.quit();
        });

        cx.activate(true);

        let reopen_filter_window = settings.appearance.filter_window_open;
        let reopen_lc_window = settings.appearance.local_contrast_window_open;
        let main_window = match cx.open_window(
            WindowOptions {
                ..Default::default()
            },
            move |window, cx| {
                cx.new::<App>(|inner_cx| {
                    let focus_handle = inner_cx.focus_handle();
                    focus_handle.focus(window);

                    // Create the viewer and load the first image if available
                    let mut viewer = ImageViewer::new(inner_cx.focus_handle());

                    if let Some(ref path) = first_image_path {
                        let max_dim = Some(settings.performance.max_image_dimension);
                        viewer.load_image_async(path.clone(), max_dim, false);
                    } else {
                        // No images found - show friendly notice (not an error)
                        let canonical_dir = search_dir
                            .canonicalize()
                            .unwrap_or_else(|_| search_dir.clone());
                        viewer.no_images_path = Some(canonical_dir);
                    }

                    // Set initial window title using the user's format setting
                    let title = window_title::format_window_title(
                        app_state.current_image().map(|p| p.as_path()),
                        app_state.current_index,
                        app_state.image_paths.len(),
                        app_state.sort_mode,
                        &settings,
                    );
                    window.set_window_title(&title);

                    // Create filter controls (shared between no-window state and the floating filter window)
                    let filter_controls = inner_cx.new(|cx| {
                        FilterControls::new(
                            viewer.image_state.filters,
                            settings.appearance.font_size_scale,
                            cx,
                        )
                    });

                    // Subscribe to filter control changes (event-based, not polling)
                    inner_cx
                        .subscribe(
                            &filter_controls,
                            |this, _fc, _event: &FilterControlsEvent, cx| {
                                // Update viewer with new filter values
                                let current_filters = this.filter_controls.read(cx).get_filters(cx);
                                this.viewer.image_state.filters = current_filters;
                                this.viewer.update_filtered_cache();
                                this.save_current_image_state();
                                cx.notify();
                            },
                        )
                        .detach();

                    // Create local-contrast controls (sliders live on the App; the
                    // LC window renders this entity when open).
                    let local_contrast_controls = inner_cx.new(|cx| {
                        LocalContrastControls::new(settings.appearance.font_size_scale, cx)
                    });
                    inner_cx
                        .subscribe(
                            &local_contrast_controls,
                            |this, _entity, event: &LocalContrastControlsEvent, cx| match event {
                                LocalContrastControlsEvent::ResetRequested => {
                                    this.local_contrast_controls.update(cx, |c, cx| {
                                        c.reset_sliders(cx);
                                        c.set_status("", cx);
                                        c.set_progress(None, cx);
                                        c.set_batch_progress(None, cx);
                                    });
                                    this.viewer.cancel_lc_batch();
                                    if let Some(loaded) = this.viewer.current_image.as_mut() {
                                        loaded.lc_render = None;
                                        loaded.cached_lc_params = None;
                                        loaded
                                            .lc_frame_renders
                                            .iter_mut()
                                            .for_each(|s| *s = None);
                                    }
                                    cx.notify();
                                }
                                LocalContrastControlsEvent::CancelRequested => {
                                    this.viewer.cancel_lc_processing();
                                    this.viewer.cancel_lc_batch();
                                    this.local_contrast_controls.update(cx, |c, cx| {
                                        c.set_status("Cancelled", cx);
                                        c.set_progress(None, cx);
                                        c.set_batch_progress(None, cx);
                                    });
                                    cx.notify();
                                }
                                LocalContrastControlsEvent::ProcessAllFramesRequested => {
                                    let params =
                                        this.local_contrast_controls.read(cx).get_parameters(cx);
                                    this.viewer.spawn_lc_batch(params);
                                    cx.notify();
                                }
                                LocalContrastControlsEvent::ParametersChanged => {
                                    let auto = this
                                        .local_contrast_controls
                                        .read(cx)
                                        .auto_process;
                                    // Persist auto-process state per-image (only when it changes).
                                    if this.viewer.image_state.lc_auto_process != auto {
                                        this.viewer.image_state.lc_auto_process = auto;
                                        this.save_current_image_state();
                                    }
                                    if !auto {
                                        // Auto Process off: don't trigger processing on
                                        // slider changes. Cancel any in-flight job. Do NOT
                                        // change lc_enabled — keep showing whatever is
                                        // currently displayed.
                                        this.viewer.cancel_lc_processing();
                                        this.local_contrast_controls.update(cx, |c, cx| {
                                            c.set_progress(None, cx);
                                        });
                                        cx.notify();
                                        return;
                                    }
                                    // Auto Process on: process and display the result.
                                    this.viewer.set_lc_enabled(true);
                                    // Auto-pause animation when LC is enabled.
                                    if let Some(ref mut anim) =
                                        this.viewer.image_state.animation
                                    {
                                        anim.is_playing = false;
                                    }
                                    let params =
                                        this.local_contrast_controls.read(cx).get_parameters(cx);
                                    this.viewer.update_local_contrast(params);
                                    if this.viewer.is_processing_lc() {
                                        this.local_contrast_controls.update(cx, |c, cx| {
                                            c.set_status("Processing…", cx);
                                            c.set_progress(Some(0.0), cx);
                                        });
                                    } else {
                                        this.local_contrast_controls.update(cx, |c, cx| {
                                            c.set_status("", cx);
                                            c.set_progress(None, cx);
                                        });
                                    }
                                    cx.notify();
                                }
                                LocalContrastControlsEvent::ClearCacheForCurrentImageRequested => {
                                    let key = this
                                        .viewer
                                        .current_image
                                        .as_ref()
                                        .and_then(|loaded| loaded.image_key.clone());
                                    let toast_msg = match key {
                                        Some(ref k) => match crate::utils::frame_cache::purge_image(k) {
                                            Ok(freed) => format!(
                                                "Cleared cache for current image ({:.2} MB)",
                                                freed as f64 / (1024.0 * 1024.0)
                                            ),
                                            Err(e) => format!("Cache clear failed: {e}"),
                                        },
                                        None => "No cache key for current image".to_string(),
                                    };
                                    // Drop in-memory caches so the next batch
                                    // does not hand back references to deleted
                                    // disk files.
                                    this.viewer.cancel_lc_batch();
                                    if let Some(loaded) = this.viewer.current_image.as_mut() {
                                        loaded.lc_render = None;
                                        loaded.lc_render_size = None;
                                        loaded.cached_lc_params = None;
                                        loaded
                                            .lc_frame_renders
                                            .iter_mut()
                                            .for_each(|s| *s = None);
                                        loaded.lc_pending_frame_renders = None;
                                        for slot in loaded.frame_cache_paths.iter_mut() {
                                            *slot = std::path::PathBuf::new();
                                        }
                                    }
                                    this.toast = Some(crate::ToastState {
                                        message: toast_msg,
                                        detail: None,
                                        is_error: false,
                                        created_at: std::time::Instant::now(),
                                    });
                                    cx.notify();
                                }
                                LocalContrastControlsEvent::ClearAllCachesRequested => {
                                    let toast_msg =
                                        match crate::utils::frame_cache::purge_all() {
                                            Ok(freed) => format!(
                                                "Cleared all cached frames ({:.2} MB)",
                                                freed as f64 / (1024.0 * 1024.0)
                                            ),
                                            Err(e) => format!("Cache clear failed: {e}"),
                                        };
                                    this.viewer.cancel_lc_batch();
                                    if let Some(loaded) = this.viewer.current_image.as_mut() {
                                        loaded.lc_render = None;
                                        loaded.lc_render_size = None;
                                        loaded.cached_lc_params = None;
                                        loaded
                                            .lc_frame_renders
                                            .iter_mut()
                                            .for_each(|s| *s = None);
                                        loaded.lc_pending_frame_renders = None;
                                        for slot in loaded.frame_cache_paths.iter_mut() {
                                            *slot = std::path::PathBuf::new();
                                        }
                                    }
                                    this.toast = Some(crate::ToastState {
                                        message: toast_msg,
                                        detail: None,
                                        is_error: false,
                                        created_at: std::time::Instant::now(),
                                    });
                                    cx.notify();
                                }
                            },
                        )
                        .detach();

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

                    // Create debug overlay
                    let debug_overlay = inner_cx.new(|_cx| {
                        DebugOverlay::new(DebugOverlayConfig {
                            current_path: None,
                            current_index: 0,
                            total_images: 0,
                            zoom: 1.0,
                            pan: (0.0, 0.0),
                            is_fit_to_window: true,
                            image_dimensions: None,
                            viewport_size: None,
                            sort_mode: app_state.sort_mode,
                            overlay_transparency: settings.appearance.overlay_transparency,
                            font_size_scale: settings.appearance.font_size_scale,
                        })
                    });

                    // Create menu bar for Windows/Linux
                    #[cfg(not(target_os = "macos"))]
                    let menu_bar = inner_cx.new(|cx| components::MenuBar::new(cx));

                    App {
                        app_state,
                        viewer,
                        focus_handle,
                        escape_presses: Vec::new(),
                        z_key_held: false,
                        mouse_button_down: false,
                        show_zoom_indicator: true,
                        show_help: false,
                        show_debug: false,
                        show_settings: false,
                        filter_window: None,
                        filter_controls,
                        local_contrast_window: None,
                        local_contrast_controls,
                        settings_window,
                        help_overlay,
                        debug_overlay,
                        #[cfg(not(target_os = "macos"))]
                        menu_bar,
                        last_frame_update: Instant::now(),
                        drag_over: false,
                        pending_delete: None,
                        toast: None,
                        settings: settings.clone(),
                    }
                })
            },
        ) {
            Ok(handle) => handle,
            Err(e) => {
                eprintln!("Failed to open window: {:?}", e);
                return;
            }
        };

        // Reopen the floating filter window if it was open when the app last quit.
        if reopen_filter_window {
            cx.defer(move |cx| {
                let _ = main_window.update(cx, |app, _window, app_cx| {
                    app.open_filter_window(app_cx);
                });
            });
        }

        // Same for the Local Contrast window.
        if reopen_lc_window {
            cx.defer(move |cx| {
                let _ = main_window.update(cx, |app, _window, app_cx| {
                    app.open_local_contrast_window(app_cx);
                });
            });
        }

        // Register global action handlers so keyboard shortcuts work even
        // when a floating window (Filter, Local Contrast) has focus. When
        // the main window is focused its per-element handlers take priority
        // and these don't fire. When a floating window is focused and has
        // no handler for the action, it bubbles here.
        macro_rules! forward {
            ($action:ty, $method:ident) => {
                cx.on_action({
                    let wh = main_window;
                    move |_: &$action, cx| {
                        let _ = wh.update(cx, |app, window, cx| app.$method(window, cx));
                    }
                });
            };
        }
        macro_rules! forward_slot {
            ($action:ty, $method:ident, $slot:expr) => {
                cx.on_action({
                    let wh = main_window;
                    move |_: &$action, cx| {
                        let _ = wh.update(cx, |app, window, cx| app.$method($slot, window, cx));
                    }
                });
            };
        }
        // Navigation
        forward!(NextImage, handle_next_image);
        forward!(PreviousImage, handle_previous_image);
        // Animation
        forward!(ToggleAnimationPlayPause, handle_toggle_animation);
        forward!(NextFrame, handle_next_frame);
        forward!(PreviousFrame, handle_previous_frame);
        // Sort
        forward!(SortAlphabetical, handle_sort_alphabetical);
        forward!(SortByModified, handle_sort_by_modified);
        forward!(SortByTypeToggle, handle_sort_by_type_toggle);
        // Local Contrast
        forward!(ToggleLocalContrast, handle_toggle_local_contrast);
        forward!(ApplyLocalContrast, handle_apply_local_contrast);
        forward!(ApplyLocalContrastAll, handle_apply_local_contrast_all);
        forward!(ResetLocalContrast, handle_reset_local_contrast);
        // Zoom
        forward!(ZoomIn, handle_zoom_in);
        forward!(ZoomOut, handle_zoom_out);
        forward!(ZoomReset, handle_zoom_reset);
        forward!(ZoomResetAndCenter, handle_zoom_reset_and_center);
        forward!(ZoomInFast, handle_zoom_in_fast);
        forward!(ZoomOutFast, handle_zoom_out_fast);
        forward!(ZoomInSlow, handle_zoom_in_slow);
        forward!(ZoomOutSlow, handle_zoom_out_slow);
        forward!(ZoomInIncremental, handle_zoom_in_incremental);
        forward!(ZoomOutIncremental, handle_zoom_out_incremental);
        // Pan
        forward!(PanUp, handle_pan_up);
        forward!(PanDown, handle_pan_down);
        forward!(PanLeft, handle_pan_left);
        forward!(PanRight, handle_pan_right);
        forward!(PanUpFast, handle_pan_up_fast);
        forward!(PanDownFast, handle_pan_down_fast);
        forward!(PanLeftFast, handle_pan_left_fast);
        forward!(PanRightFast, handle_pan_right_fast);
        forward!(PanUpSlow, handle_pan_up_slow);
        forward!(PanDownSlow, handle_pan_down_slow);
        forward!(PanLeftSlow, handle_pan_left_slow);
        forward!(PanRightSlow, handle_pan_right_slow);
        // View toggles
        forward!(ToggleHelp, handle_toggle_help);
        forward!(ToggleDebug, handle_toggle_debug);
        forward!(ToggleZoomIndicator, handle_toggle_zoom_indicator);
        forward!(ToggleBackground, handle_toggle_background);
        forward!(ToggleSettings, handle_toggle_settings);
        // Filters
        forward!(ToggleFilters, handle_toggle_filters);
        forward!(DisableFilters, handle_disable_filters);
        forward!(EnableFilters, handle_enable_filters);
        forward!(ResetFilters, handle_reset_filters);
        forward!(BrightnessUp, handle_brightness_up);
        forward!(BrightnessDown, handle_brightness_down);
        forward!(ContrastUp, handle_contrast_up);
        forward!(ContrastDown, handle_contrast_down);
        forward!(GammaUp, handle_gamma_up);
        forward!(GammaDown, handle_gamma_down);
        // Slots
        forward_slot!(RecallSlot3, handle_recall_slot, 3);
        forward_slot!(RecallSlot4, handle_recall_slot, 4);
        forward_slot!(RecallSlot5, handle_recall_slot, 5);
        forward_slot!(RecallSlot6, handle_recall_slot, 6);
        forward_slot!(RecallSlot7, handle_recall_slot, 7);
        forward_slot!(RecallSlot8, handle_recall_slot, 8);
        forward_slot!(RecallSlot9, handle_recall_slot, 9);
        forward_slot!(StoreSlot3, handle_store_slot, 3);
        forward_slot!(StoreSlot4, handle_store_slot, 4);
        forward_slot!(StoreSlot5, handle_store_slot, 5);
        forward_slot!(StoreSlot6, handle_store_slot, 6);
        forward_slot!(StoreSlot7, handle_store_slot, 7);
        forward_slot!(StoreSlot8, handle_store_slot, 8);
        forward_slot!(StoreSlot9, handle_store_slot, 9);
        // File operations
        forward!(OpenFile, handle_open_file);
        forward!(SaveFile, handle_save_file);
        forward!(SaveFileToDownloads, handle_save_file_to_downloads);
        forward!(OpenInExternalViewer, handle_open_in_external_viewer);
        forward!(OpenInExternalViewerAndQuit, handle_open_in_external_viewer_and_quit);
        forward!(OpenInExternalEditor, handle_open_in_external_editor);
        forward!(RevealInFinder, handle_reveal_in_finder);
        forward!(RequestDelete, handle_request_delete);
        forward!(RequestPermanentDelete, handle_request_permanent_delete);

        // Check for pending open paths from macOS "Open With" events
        // Use defer to ensure the window is fully set up first
        cx.defer(|cx| {
            check_and_process_pending_paths(cx);
        });

        // Poll for pending open paths from on_open_urls / macOS open handler.
        // GPUI lacks a cross-thread wake mechanism, so the OS callbacks write to
        // a static Mutex and this loop checks it. 250ms latency is imperceptible
        // for file-open events.
        let executor = cx.background_executor().clone();
        cx.spawn(async move |cx| {
            loop {
                executor.timer(Duration::from_millis(250)).await;

                let has_pending = PENDING_OPEN_PATHS
                    .lock()
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
        })
        .detach();
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
