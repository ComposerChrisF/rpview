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
        // Render the viewer content directly
        let content = if let Some(ref error) = self.viewer.error_message {
            cx.new(|_cx| components::ErrorDisplay::new(error.clone()))
                .into_any_element()
        } else if let Some(ref loaded) = self.viewer.current_image {
            let image_data = &loaded.data;
            let width = image_data.width();
            let height = image_data.height();
            let path = &loaded.path;
            
            div()
                .flex()
                .flex_col()
                .size_full()
                .justify_center()
                .items_center()
                .gap(utils::style::Spacing::md())
                .child(
                    div()
                        .text_size(utils::style::TextSize::xl())
                        .text_color(utils::style::Colors::info())
                        .child("✓ Image Loaded")
                )
                .child(
                    div()
                        .text_size(utils::style::TextSize::md())
                        .text_color(utils::style::Colors::text())
                        .child(format!("File: {}", path.file_name().unwrap_or_default().to_string_lossy()))
                )
                .child(
                    div()
                        .text_size(utils::style::TextSize::md())
                        .text_color(utils::style::Colors::text())
                        .child(format!("Dimensions: {}x{}", width, height))
                )
                .child(
                    div()
                        .text_size(utils::style::TextSize::sm())
                        .text_color(utils::style::Colors::text())
                        .child("(Image rendering coming in next update)")
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .size_full()
                .justify_center()
                .items_center()
                .gap(utils::style::Spacing::md())
                .child(
                    div()
                        .text_size(utils::style::TextSize::xl())
                        .text_color(utils::style::Colors::text())
                        .child("No image loaded")
                )
                .child(
                    div()
                        .text_size(utils::style::TextSize::sm())
                        .text_color(utils::style::Colors::text())
                        .child("Use arrow keys to navigate (coming in Phase 3)")
                )
                .into_any_element()
        };
        
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(utils::style::Colors::background())
            .child(content)
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
                    
                    // Create the app with an empty viewer
                    // Image loading will be added in Phase 3 (Navigation)
                    App {
                        viewer: ImageViewer {
                            current_image: None,
                            error_message: None,
                            focus_handle: inner_cx.focus_handle(),
                        },
                        focus_handle,
                        escape_presses: Vec::new(),
                    }
                })
        })
        .unwrap();
    });
}
