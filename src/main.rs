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

actions!(app, [CloseWindow, Quit, EscapePressed, NextImage, PreviousImage, SortAlphabetical, SortByModified]);

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
    
    fn update_viewer(&mut self) {
        if let Some(path) = self.app_state.current_image().cloned() {
            self.viewer.load_image(path);
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
    }
}

fn main() {
    // Parse command-line arguments to get image paths
    let image_paths = match Cli::parse_image_paths() {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    
    // Initialize application state
    let app_state = AppState::new(image_paths.clone());
    
    // Print startup info
    println!("rpview-gpui starting...");
    println!("Loaded {} image(s)", app_state.image_paths.len());
    if let Some(first_image) = app_state.current_image() {
        println!("Current image: {}", first_image.display());
    }
    
    // Get the first image path to load
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
                    };
                    
                    if let Some(ref path) = first_image_path {
                        viewer.load_image(path.clone());
                    } else {
                        viewer.error_message = Some("No images found. Please provide image paths as arguments.".to_string());
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
