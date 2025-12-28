use gpui::*;
use std::time::{Duration, Instant};

mod cli;
mod components;
mod error;
mod state;
mod utils;

use cli::Cli;
use components::ImageViewer;
use state::AppState;

actions!(app, [
    CloseWindow, 
    Quit, 
    EscapePressed, 
    NextImage, 
    PreviousImage, 
    SortAlphabetical, 
    SortByModified,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    PanUp,
    PanDown,
    PanLeft,
    PanRight,
]);

struct App {
    app_state: AppState,
    viewer: ImageViewer,
    focus_handle: FocusHandle,
    escape_presses: Vec<Instant>,
}
 
impl App {
    fn handle_escape(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        
        // Remove presses older than 2 seconds
        self.escape_presses.retain(|&time| now.duration_since(time) < Duration::from_secs(2));
        
        // Add current press
        self.escape_presses.push(now);
        
        // Check if we have 3 presses within 2 seconds
        if self.escape_presses.len() >= 3 {
            cx.quit();
        }
    }
    
    fn handle_next_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.app_state.next_image();
        self.update_viewer();
        self.update_window_title(window);
        cx.notify();
    }
    
    fn handle_previous_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.app_state.previous_image();
        self.update_viewer();
        self.update_window_title(window);
        cx.notify();
    }
    
    fn handle_sort_alphabetical(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.app_state.set_sort_mode(state::SortMode::Alphabetical);
        self.update_viewer();
        self.update_window_title(window);
        cx.notify();
    }
    
    fn handle_sort_by_modified(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.app_state.set_sort_mode(state::SortMode::ModifiedDate);
        self.update_viewer();
        self.update_window_title(window);
        cx.notify();
    }
    
    fn handle_zoom_in(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_in(utils::zoom::ZOOM_STEP);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_out(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_out(utils::zoom::ZOOM_STEP);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_reset(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.reset_zoom();
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, 10.0);  // Pan up = positive Y
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, -10.0);  // Pan down = negative Y
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_left(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(10.0, 0.0);  // Pan left = positive X
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_right(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(-10.0, 0.0);  // Pan right = negative X
        self.save_current_image_state();
        cx.notify();
    }
    
    fn save_current_image_state(&mut self) {
        let state = self.viewer.get_image_state();
        self.app_state.save_current_state(state);
    }
    
    fn load_current_image_state(&mut self) {
        let state = self.app_state.get_current_state();
        self.viewer.set_image_state(state);
    }
    
    fn update_viewer(&mut self) {
        if let Some(path) = self.app_state.current_image().cloned() {
            // Load the image first (this will call fit_to_window)
            self.viewer.load_image(path.clone());
            
            // Only load cached state if we have previously saved state for this image
            if self.app_state.image_states.contains_key(&path) {
                self.load_current_image_state();
            }
            // Otherwise, keep the fit-to-window state that load_image set
        } else {
            self.viewer.clear();
        }
    }
    
    fn update_window_title(&mut self, window: &mut Window) {
        if let Some(path) = self.app_state.current_image() {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");
            
            let position = self.app_state.current_index + 1;
            let total = self.app_state.image_paths.len();
            
            let title = format!("{} ({}/{})", filename, position, total);
            window.set_window_title(&title);
        } else {
            window.set_window_title("rpview-gpui");
        }
    }
}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Update viewer's viewport size before rendering
        let bounds = window.bounds();
        self.viewer.update_viewport_from_window(bounds.size);
        
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(utils::style::Colors::background())
            .child(cx.new(|_cx| self.viewer.clone()))
            .on_action(|_: &CloseWindow, window, _| {
                window.remove_window();
            })
            .on_action(cx.listener(|this, _: &EscapePressed, _window, cx| {
                this.handle_escape(cx);
            }))
            .on_action(cx.listener(|this, _: &NextImage, window, cx| {
                this.handle_next_image(window, cx);
            }))
            .on_action(cx.listener(|this, _: &PreviousImage, window, cx| {
                this.handle_previous_image(window, cx);
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
    }
}

fn main() {
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
    
    // Initialize application state with the starting index
    let app_state = AppState::new_with_index(image_paths.clone(), start_index);
    
    // Print startup info
    println!("rpview-gpui starting...");
    println!("Loaded {} image(s)", app_state.image_paths.len());
    if let Some(first_image) = app_state.current_image() {
        println!("Current image: {}", first_image.display());
    }
    
    // Get the first image path to load (or None if no images)
    let first_image_path = app_state.current_image().cloned();
    
    Application::new().run(move |cx: &mut gpui::App| {
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        
        cx.bind_keys([
            KeyBinding::new("cmd-w", CloseWindow, None),
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("escape", EscapePressed, None),
            KeyBinding::new("right", NextImage, None),
            KeyBinding::new("left", PreviousImage, None),
            KeyBinding::new("shift-cmd-a", SortAlphabetical, None),
            KeyBinding::new("shift-cmd-m", SortByModified, None),
            // Zoom controls
            KeyBinding::new("=", ZoomIn, None),  // = key (same as +)
            KeyBinding::new("+", ZoomIn, None),
            KeyBinding::new("-", ZoomOut, None),
            KeyBinding::new("0", ZoomReset, None),
            // Pan controls with WASD
            KeyBinding::new("w", PanUp, None),
            KeyBinding::new("a", PanLeft, None),
            KeyBinding::new("s", PanDown, None),
            KeyBinding::new("d", PanRight, None),
            // Pan controls with IJKL
            KeyBinding::new("i", PanUp, None),
            KeyBinding::new("j", PanLeft, None),
            KeyBinding::new("k", PanDown, None),
            KeyBinding::new("l", PanRight, None),
        ]);
        
        cx.on_action(|_: &Quit, cx| {
            cx.quit();
        });
        
        cx.activate(true);
        
        cx.open_window(
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
                        focus_handle: inner_cx.focus_handle(),
                        image_state: state::ImageState::new(),
                        viewport_size: None,
                    };
                    
                    if let Some(ref path) = first_image_path {
                        viewer.load_image(path.clone());
                    } else {
                        // No images found - show error with canonical directory path
                        let canonical_dir = search_dir.canonicalize()
                            .unwrap_or_else(|_| search_dir.clone());
                        viewer.error_message = Some(format!("No images found in directory:\n{}", canonical_dir.display()));
                        viewer.error_path = None;
                    }
                    
                    // Set initial window title
                    if let Some(path) = app_state.current_image() {
                        let filename = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        let position = app_state.current_index + 1;
                        let total = app_state.image_paths.len();
                        let title = format!("{} ({}/{})", filename, position, total);
                        window.set_window_title(&title);
                    } else {
                        window.set_window_title("rpview-gpui");
                    }
                    
                    App {
                        app_state,
                        viewer,
                        focus_handle,
                        escape_presses: Vec::new(),
                    }
                })
        })
        .unwrap();
    });
}
