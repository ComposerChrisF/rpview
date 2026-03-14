#![allow(clippy::collapsible_if)]

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

use cli::Cli;
use components::{
    DebugOverlay, DebugOverlayConfig, FilterControls, FilterControlsEvent, HelpOverlay,
    ImageViewer, SettingsWindow,
};
use state::{AppSettings, AppState};
use utils::settings_io;

// Import all actions from lib.rs (they're defined there to avoid duplication)
use rpview::{
    BrightnessDown, BrightnessUp, CloseSettings, CloseWindow, ConfirmDelete, ContrastDown,
    ContrastUp, DisableFilters, EnableFilters, EscapePressed, GammaDown, GammaUp, NextFrame,
    NextImage, OpenFile, OpenInExternalEditor, OpenInExternalViewer, OpenInExternalViewerAndQuit,
    PanDown, PanDownFast, PanDownSlow, PanLeft, PanLeftFast, PanLeftSlow, PanRight, PanRightFast,
    PanRightSlow, PanUp, PanUpFast, PanUpSlow, PreviousFrame, PreviousImage, Quit, RequestDelete,
    RequestPermanentDelete, ResetFilters, ResetSettingsToDefaults, RevealInFinder, SaveFile,
    SaveFileToDownloads, SortAlphabetical, SortByModified, ToggleAnimationPlayPause,
    ToggleBackground, ToggleDebug, ToggleFilters, ToggleHelp, ToggleSettings, ToggleZoomIndicator,
    ZoomIn, ZoomInFast, ZoomInIncremental, ZoomInSlow, ZoomOut, ZoomOutFast, ZoomOutIncremental,
    ZoomOutSlow, ZoomReset, ZoomResetAndCenter,
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
    /// Whether zoom indicator is visible
    show_zoom_indicator: bool,
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
    println!("rpview starting...");
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

        if let Err(e) = cx.open_window(
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
                    if let Some(path) = app_state.current_image() {
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        let position = app_state.current_index + 1;
                        let total = app_state.image_paths.len();
                        let title = if settings.sort_navigation.show_image_counter {
                            settings
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
                        window.set_window_title("rpview");
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
                            image_state: state::ImageState::new(),
                            image_dimensions: None,
                            viewport_size: None,
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
                        spacebar_held: false,
                        mouse_button_down: false,
                        show_zoom_indicator: true,
                        show_help: false,
                        show_debug: false,
                        show_settings: false,
                        show_filters: false,
                        filter_controls,
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
