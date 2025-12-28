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

actions!(app, [CloseWindow, Quit, EscapePressed]);

struct App {
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
                        focus_handle: inner_cx.focus_handle(),
                    };
                    
                    if let Some(ref path) = first_image_path {
                        viewer.load_image(path.clone());
                    } else {
                        viewer.error_message = Some("No images found. Please provide image paths as arguments.".to_string());
                    }
                    
                    App {
                        viewer,
                        focus_handle,
                        escape_presses: Vec::new(),
                    }
                })
        })
        .unwrap();
    });
}
