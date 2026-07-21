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
#[allow(dead_code)] // Public API of the new GPU filter pipeline; consumers
// are wired up incrementally. Remove this once the UI calls the module.
mod gpu;
mod state;
mod utils;
mod window_title;

use cli::Cli;
use components::{
    DebugOverlay, DebugOverlayConfig, FilterControls, FilterControlsEvent, FilterWindowView,
    GpuPipelineControls, GpuPipelineControlsEvent, GpuPipelineWindowView, HelpOverlay, ImageViewer,
    SettingsWindow,
};
use state::{AppSettings, AppState};
use utils::debug_eprintln;
use utils::settings_io;

// Import all actions from lib.rs (they're defined there to avoid duplication)
use rpview::{
    BrightnessDown, BrightnessUp, CloseSettings, CloseWindow, ConfirmDelete, ContrastDown,
    ContrastUp, DisableFilters, EnableFilters, EscapePressed, GammaDown, GammaUp, NextFrame,
    NextImage, OpenFile, OpenInExternalEditor, OpenInExternalViewer, OpenInExternalViewerAndQuit,
    PanDown, PanDownFast, PanDownSlow, PanLeft, PanLeftFast, PanLeftSlow, PanRight, PanRightFast,
    PanRightSlow, PanUp, PanUpFast, PanUpSlow, PreviousFrame, PreviousImage, Quit, RecallSlot3,
    RecallSlot4, RecallSlot5, RecallSlot6, RecallSlot7, RecallSlot8, RecallSlot9, RequestDelete,
    RequestPermanentDelete, ResetFilters, ResetGpuPipeline, ResetSettingsToDefaults,
    RevealInFinder, SaveFile, SaveFileToDownloads, SortAlphabetical, SortByModified,
    SortByTypeToggle, StoreSlot3, StoreSlot4, StoreSlot5, StoreSlot6, StoreSlot7, StoreSlot8,
    StoreSlot9, ToggleAnimationPlayPause, ToggleBackground, ToggleDebug, ToggleFilters,
    ToggleGpuPipeline, ToggleHelp, ToggleSettings, ToggleZoomIndicator, ZoomIn, ZoomInFast,
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
    /// This App's own window.  Needed because several close paths reach the
    /// App without a `&mut Window` in hand — notably ESC forwarded from a
    /// floating panel, which arrives through a weak entity handle.
    window_handle: AnyWindowHandle,
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
    /// Open floating GPU Pipeline window handle (None = closed)
    gpu_pipeline_window: Option<WindowHandle<GpuPipelineWindowView>>,
    /// GPU pipeline controls (sliders + per-stage enables). Always exists;
    /// lives in the App so its values persist across open/close of the
    /// GPU pipeline window.
    gpu_pipeline_controls: Entity<GpuPipelineControls>,
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
    let cli_paths = match Cli::parse_image_paths() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    let (image_paths, start_path) = (cli_paths.images, cli_paths.start);

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

    // Print startup info
    debug_eprintln!("rpview starting...");

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

        app_keybindings::setup_key_bindings(cx);
        app_keybindings::setup_menus(cx);

        cx.on_action(|_: &Quit, cx| {
            cx.quit();
        });

        // Cmd+W fallback for windows with no local `CloseWindow` handler —
        // i.e. the floating Filter / GPU Pipeline panels.  An image window
        // handles the action itself (see `app_render.rs`), so this never
        // double-fires.
        cx.on_action(|_: &CloseWindow, cx| {
            if let Some(handle) = cx.active_window() {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            }
        });

        cx.activate(true);

        // AppKit turns each path argument into an `application:openFiles:`
        // event and delivers it *before* this launch callback runs, so the
        // command line is already sitting in the pending queue — where it
        // would be read as a second, independent "open this" request and get
        // its own window.  Drop that one launch batch; argv is already on
        // screen.  A Finder double-click that supplied no argv paths is left
        // alone, since the event is the only way it reaches us at all.
        if cli_paths.from_arguments {
            discard_pending_open_paths();
        }

        let reopen_filter_window = settings.appearance.filter_window_open;
        let reopen_gpu_pipeline_window = settings.appearance.gpu_pipeline_window_open;
        let Some(main_window) =
            open_image_window(cx, image_paths, start_path, &settings, Some(search_dir))
        else {
            return;
        };

        // The app lives as long as at least one image window does.  Closing
        // the last one quits, even if floating panels (Filter / GPU Pipeline)
        // are still open — they are torn down by `cx.quit()` anyway.
        cx.on_window_closed(|cx| {
            // Observers fire *after* the closed window has been removed from
            // `cx.windows()`, so this no longer counts the one that just went.
            let Some(front) = app_windows(cx).into_iter().next() else {
                cx.quit();
                return;
            };
            // Something else closed — a floating panel, or one image window of
            // several.  The OS may not hand focus back to us automatically, so
            // re-activate the frontmost image window; that keeps keyboard focus
            // (and the 3-press ESC shortcut) working.
            let _ = front.update(cx, |_, window, _| window.activate_window());
        })
        .detach();

        // Reopen the floating filter window if it was open when the app last quit.
        if reopen_filter_window {
            cx.defer(move |cx| {
                let _ = main_window.update(cx, |app, _window, app_cx| {
                    app.open_filter_window(app_cx);
                });
            });
        }

        // Same for the GPU Pipeline window.
        if reopen_gpu_pipeline_window {
            cx.defer(move |cx| {
                let _ = main_window.update(cx, |app, _window, app_cx| {
                    app.open_gpu_pipeline_window(app_cx);
                });
            });
        }

        // Register global action handlers so keyboard shortcuts work even
        // when a floating window (Filter, GPU Pipeline) has focus. When an
        // image window is focused its per-element handlers take priority
        // and these don't fire. When a floating window is focused and has
        // no handler for the action, it bubbles here.
        //
        // The target is resolved per-invocation rather than captured, because
        // there may now be several image windows: `active_app_window` picks
        // the focused one, or the frontmost one behind the focused panel.
        macro_rules! forward {
            ($action:ty, $method:ident) => {
                cx.on_action(move |_: &$action, cx| {
                    if let Some(wh) = active_app_window(cx) {
                        let _ = wh.update(cx, |app, window, cx| app.$method(window, cx));
                    }
                });
            };
        }
        macro_rules! forward_slot {
            ($action:ty, $method:ident, $slot:expr) => {
                cx.on_action(move |_: &$action, cx| {
                    if let Some(wh) = active_app_window(cx) {
                        let _ = wh.update(cx, |app, window, cx| app.$method($slot, window, cx));
                    }
                });
            };
        }
        // ESC fallback: when a focused window has no local `EscapePressed`
        // handler in its dispatch path, the action bubbles to this global
        // handler so ESC still reaches the main App — keeping the 3-press quit
        // shortcut working "without regard for what has focus". A window with a
        // local handler (main window, Filter, GPU Pipeline) consumes ESC first,
        // so this never double-fires.
        forward!(EscapePressed, handle_escape);
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
        forward!(
            OpenInExternalViewerAndQuit,
            handle_open_in_external_viewer_and_quit
        );
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

/// Throw away any file-open requests the OS has queued but nobody has read.
fn discard_pending_open_paths() {
    if let Ok(mut pending) = PENDING_OPEN_PATHS.lock() {
        for _path in pending.iter() {
            debug_eprintln!("Discarding launch-echo open request: {}", _path.display());
        }
        pending.clear();
    }

    #[cfg(target_os = "macos")]
    for _path in macos_open_handler::take_pending_paths() {
        debug_eprintln!("Discarding launch-echo open request: {}", _path.display());
    }
}

/// Every open image window, frontmost first.  Floating panels (Filter / GPU
/// Pipeline) are filtered out by the downcast.
///
/// The two GPUI sources are used for different things on purpose:
///
/// - `windows()` decides **which** windows exist.  It is authoritative — a
///   closed window is out of it by the time `on_window_closed` observers run.
/// - `window_stack()` only supplies front-to-back **order**, and only on macOS
///   (elsewhere it is `None` and creation order stands in).
///
/// They must not be swapped.  `window_stack()` reads `NSApp.orderedWindows`,
/// which still lists a programmatically-removed window (Cmd+W, 3×ESC) at the
/// moment the close observers fire — trusting it for existence would resurrect
/// a dead window and stop the app quitting when its last window closed.
fn app_windows(cx: &gpui::App) -> Vec<WindowHandle<App>> {
    let live: Vec<WindowHandle<App>> = cx
        .windows()
        .into_iter()
        .filter_map(|handle| handle.downcast::<App>())
        .collect();

    let Some(stack) = cx.window_stack() else {
        return live;
    };

    let mut ordered: Vec<WindowHandle<App>> = stack
        .into_iter()
        .filter_map(|handle| handle.downcast::<App>())
        .filter(|handle| live.contains(handle))
        .collect();
    // A window the platform stack doesn't mention yet (just opened, or not on
    // screen) is still live; keep it, at the back.
    let missing: Vec<WindowHandle<App>> = live
        .into_iter()
        .filter(|handle| !ordered.contains(handle))
        .collect();
    ordered.extend(missing);
    ordered
}

/// The image window a globally-dispatched action should act on: the focused
/// one, or — when a floating panel has focus — the frontmost one behind it.
fn active_app_window(cx: &gpui::App) -> Option<WindowHandle<App>> {
    let windows = app_windows(cx);
    cx.active_window()
        .and_then(|handle| handle.downcast::<App>())
        // Same caveat as in `app_windows`: the platform's idea of the key
        // window can outlive GPUI's, so confirm it is still live.
        .filter(|handle| windows.contains(handle))
        .or_else(|| windows.into_iter().next())
}

/// A window with nothing loaded — the placeholder rpview opens when launched
/// with no file to show, or what is left after the last image is deleted.
///
/// A cold-start "Open With" arrives *after* that window exists (AppKit sends
/// `application:openFiles:` once launching is done), so its file belongs in
/// the placeholder rather than in a second window opened beside it.
fn empty_app_window(cx: &gpui::App) -> Option<WindowHandle<App>> {
    app_windows(cx).into_iter().find(|handle| {
        handle
            .read(cx)
            .is_ok_and(|app| app.app_state.image_paths.is_empty())
    })
}

/// The image window currently displaying `path`, if any.
///
/// Paths are compared canonicalized so that `/tmp/x.png` and
/// `/private/tmp/x.png` (or a relative CLI argument) count as the same file.
/// A path that can't be canonicalized falls back to its literal form — the
/// worst case is a missed match, which merely opens an extra window.
fn window_showing(path: &Path, cx: &gpui::App) -> Option<WindowHandle<App>> {
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let target = canonical(path);

    app_windows(cx).into_iter().find(|handle| {
        handle
            .read(cx)
            .ok()
            .and_then(|app| app.app_state.current_image())
            .is_some_and(|current| canonical(current) == target)
    })
}

/// Settings for a newly opened window: those of an already-open window if
/// there is one (they are live, and may hold changes still inside the save
/// debounce), otherwise whatever is on disk.
fn current_settings(cx: &gpui::App) -> AppSettings {
    app_windows(cx)
        .into_iter()
        .find_map(|handle| handle.read(cx).ok().map(|app| app.settings.clone()))
        .unwrap_or_else(settings_io::load_settings)
}

/// Open a new image window and return its handle.
///
/// Image windows are peers: each owns its own `AppState`, viewer, overlays and
/// floating panels, and the app quits when the last one closes (see
/// `on_window_closed` in `main`).
///
/// `no_images_dir` is the directory to name in the "no images here" notice when
/// `image_paths` is empty; `None` means don't open a window at all in that case.
fn open_image_window(
    cx: &mut gpui::App,
    image_paths: Vec<PathBuf>,
    start_path: Option<PathBuf>,
    settings: &AppSettings,
    no_images_dir: Option<PathBuf>,
) -> Option<WindowHandle<App>> {
    if image_paths.is_empty() && no_images_dir.is_none() {
        return None;
    }

    let app_state = AppState::new_with_settings(
        image_paths,
        start_path,
        settings.sort_navigation.default_sort_mode,
        settings.viewer_behavior.state_cache_size,
    );

    debug_eprintln!(
        "Opening window with {} image(s)",
        app_state.image_paths.len()
    );
    if let Some(_first_image) = app_state.current_image() {
        debug_eprintln!("Current image: {}", _first_image.display());
    }

    let first_image_path = app_state.current_image().cloned();
    let settings = settings.clone();

    let result = cx.open_window(
        WindowOptions {
            ..Default::default()
        },
        move |window, cx| {
            cx.new::<App>(|inner_cx| {
                let focus_handle = inner_cx.focus_handle();
                focus_handle.focus(window);

                // Closing an image window closes the floating panels it owns.
                // Without this they outlive their owner as orphans alongside
                // whichever image windows remain.
                inner_cx
                    .on_release(|app, cx| {
                        if let Some(handle) = app.filter_window.take() {
                            let _ = handle.update(cx, |_, window, _| window.remove_window());
                        }
                        if let Some(handle) = app.gpu_pipeline_window.take() {
                            let _ = handle.update(cx, |_, window, _| window.remove_window());
                        }
                    })
                    .detach();

                // Create the viewer and load the first image if available
                let mut viewer = ImageViewer::new(inner_cx.focus_handle());

                if let Some(ref path) = first_image_path {
                    let max_dim = Some(settings.performance.max_image_dimension);
                    viewer.load_image_async(path.clone(), max_dim, false);
                } else if let Some(ref search_dir) = no_images_dir {
                    // No images found - show friendly notice (not an error)
                    let canonical_dir = search_dir
                        .canonicalize()
                        .unwrap_or_else(|_| search_dir.clone());
                    viewer.no_images_path = Some(canonical_dir);
                }

                build_app(app_state, viewer, focus_handle, settings, window, inner_cx)
            })
        },
    );

    match result {
        Ok(handle) => Some(handle),
        Err(e) => {
            eprintln!("Failed to open window: {:?}", e);
            None
        }
    }
}

/// Deliver file-open requests from the OS ("Open With", Finder double-click,
/// dock drop) to the right window.
///
/// A request for a file some window is *already* showing just brings that
/// window forward.  Everything else opens a NEW window — a second "Open With"
/// must never replace the image you were already looking at.
fn check_and_process_pending_paths(cx: &mut gpui::App) {
    #[allow(unused_mut)]
    let mut paths: Vec<PathBuf> = {
        let Ok(mut pending) = PENDING_OPEN_PATHS.lock() else {
            return;
        };
        std::mem::take(&mut *pending)
    };

    #[cfg(target_os = "macos")]
    {
        paths.extend(macos_open_handler::take_pending_paths());
    }

    if paths.is_empty() {
        return;
    }

    // A single path is an "open this file" request, so it can match an open
    // window.  A multi-selection is a new set of its own and always gets a
    // fresh window.
    if paths.len() == 1 {
        if let Some(existing) = window_showing(&paths[0], cx) {
            debug_eprintln!(
                "Open With: {} is already open; activating its window",
                paths[0].display()
            );
            cx.activate(true);
            let _ = existing.update(cx, |_, window, _| window.activate_window());
            return;
        }
    }

    // Fill the empty placeholder window, if there is one, before opening a peer.
    if let Some(placeholder) = empty_app_window(cx) {
        cx.activate(true);
        let _ = placeholder.update(cx, |app, window, cx| {
            app.import_image_paths(&paths, window, cx);
            window.activate_window();
        });
        return;
    }

    let Some((images, target_index)) = App::resolve_import_paths(&paths) else {
        debug_eprintln!(
            "Open With: no supported images among {} path(s)",
            paths.len()
        );
        return;
    };

    let start_path = images.get(target_index).cloned();
    let settings = current_settings(cx);
    if open_image_window(cx, images, start_path, &settings, None).is_some() {
        cx.activate(true);
    }
}

/// Assemble the `App` entity for one image window: title, controls, overlays,
/// and the event subscriptions that wire them to the viewer.
fn build_app(
    app_state: AppState,
    viewer: ImageViewer,
    focus_handle: FocusHandle,
    settings: AppSettings,
    window: &mut Window,
    cx: &mut Context<App>,
) -> App {
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
    let filter_controls = cx.new(|cx| {
        FilterControls::new(
            viewer.image_state.filters,
            settings.appearance.font_size_scale,
            cx,
        )
    });

    // Subscribe to filter control changes (event-based, not polling)
    cx.subscribe(
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

    // Create GPU-pipeline controls (sliders + per-stage enables).
    // Lives on the App so values persist across window open/close.
    let gpu_pipeline_controls =
        cx.new(|cx| GpuPipelineControls::new(settings.appearance.font_size_scale, cx));
    cx.subscribe(
        &gpu_pipeline_controls,
        |this, _entity, event: &GpuPipelineControlsEvent, cx| match event {
            GpuPipelineControlsEvent::ParametersChanged => {
                let params = this.gpu_pipeline_controls.read(cx).get_params(cx);
                this.viewer.update_gpu_pipeline(params);
                cx.notify();
            }
            GpuPipelineControlsEvent::ResetRequested => {
                this.viewer.reset_gpu_pipeline();
                cx.notify();
            }
        },
    )
    .detach();

    // Create settings window
    let settings_window = cx.new(|cx| SettingsWindow::new(settings.clone(), cx));

    // Create help overlay
    let help_overlay = cx.new(|_cx| {
        HelpOverlay::new(
            settings.appearance.overlay_transparency,
            settings.appearance.font_size_scale,
        )
    });

    // Create debug overlay
    let debug_overlay = cx.new(|_cx| {
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
    let menu_bar = cx.new(|cx| components::MenuBar::new(cx));

    App {
        app_state,
        viewer,
        focus_handle,
        window_handle: window.window_handle(),
        escape_presses: Vec::new(),
        z_key_held: false,
        mouse_button_down: false,
        show_zoom_indicator: true,
        show_help: false,
        show_debug: false,
        show_settings: false,
        filter_window: None,
        filter_controls,
        gpu_pipeline_window: None,
        gpu_pipeline_controls,
        settings_window,
        help_overlay,
        debug_overlay,
        #[cfg(not(target_os = "macos"))]
        menu_bar,
        last_frame_update: Instant::now(),
        drag_over: false,
        pending_delete: None,
        toast: None,
        settings,
    }
}
