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
    ZoomInFast,
    ZoomOutFast,
    ZoomInSlow,
    ZoomOutSlow,
    ZoomInIncremental,
    ZoomOutIncremental,
    PanUp,
    PanDown,
    PanLeft,
    PanRight,
    PanUpFast,
    PanDownFast,
    PanLeftFast,
    PanRightFast,
    PanUpSlow,
    PanDownSlow,
    PanLeftSlow,
    PanRightSlow,
]);

struct App {
    app_state: AppState,
    viewer: ImageViewer,
    focus_handle: FocusHandle,
    escape_presses: Vec<Instant>,
    /// Tracks if Z key is currently held down (for Z+drag zoom mode)
    z_key_held: bool,
    /// Tracks if left mouse button is currently pressed
    /// Used with MouseMoveEvent.pressed_button for robust button state tracking
    mouse_button_down: bool,
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
    
    fn handle_zoom_in_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_in(utils::zoom::ZOOM_STEP_FAST);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_out_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_out(utils::zoom::ZOOM_STEP_FAST);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_in_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_in(utils::zoom::ZOOM_STEP_SLOW);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_out_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.zoom_out(utils::zoom::ZOOM_STEP_SLOW);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_in_incremental(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Incremental zoom: add 1% (0.01) to current zoom
        let current_zoom = self.viewer.image_state.zoom;
        let new_zoom = utils::zoom::clamp_zoom(current_zoom + utils::zoom::ZOOM_STEP_INCREMENTAL);
        self.viewer.image_state.zoom = new_zoom;
        self.viewer.image_state.is_fit_to_window = false;
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_zoom_out_incremental(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // Incremental zoom: subtract 1% (0.01) from current zoom
        let current_zoom = self.viewer.image_state.zoom;
        let new_zoom = utils::zoom::clamp_zoom(current_zoom - utils::zoom::ZOOM_STEP_INCREMENTAL);
        self.viewer.image_state.zoom = new_zoom;
        self.viewer.image_state.is_fit_to_window = false;
        self.save_current_image_state();
        cx.notify();
    }
    

    fn handle_pan_up(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, -10.0);  // Pan up = move image down (negative Y)
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_down(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, 10.0);  // Pan down = move image up (positive Y)
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_left(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(-10.0, 0.0);  // Pan left = move image right (negative X)
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_right(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(10.0, 0.0);  // Pan right = move image left (positive X)
        self.save_current_image_state();
        cx.notify();
    }
    
    // Fast pan (3x speed = 30px) with Shift modifier
    fn handle_pan_up_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, -30.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_down_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, 30.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_left_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(-30.0, 0.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_right_fast(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(30.0, 0.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    // Slow pan (1px) with Ctrl/Cmd modifier
    fn handle_pan_up_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, -1.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_down_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(0.0, 1.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_left_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(-1.0, 0.0);
        self.save_current_image_state();
        cx.notify();
    }
    
    fn handle_pan_right_slow(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.viewer.pan(1.0, 0.0);
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
        
        // Update Z-drag state based on z_key_held
        if self.z_key_held && self.viewer.z_drag_state.is_none() {
            self.viewer.z_drag_state = Some((0.0, 0.0, 0.0, 0.0));
        } else if !self.z_key_held && self.viewer.z_drag_state.is_some() {
            self.viewer.z_drag_state = None;
        }
        
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(utils::style::Colors::background())
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                this.mouse_button_down = true;
                // Start Z-drag zoom if Z key is being held
                if this.viewer.z_drag_state.is_some() {
                    let y: f32 = event.position.y.into();
                    let x: f32 = event.position.x.into();
                    // Store: (last_x, last_y, center_x, center_y) for zoom centering
                    this.viewer.z_drag_state = Some((x, y, x, y));
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _event: &MouseUpEvent, _window, cx| {
                this.mouse_button_down = false;
                // End Z-drag zoom (but keep Z key state active if still held)
                if let Some((_, _, _, _)) = this.viewer.z_drag_state {
                    // Reset to sentinel value to indicate Z is held but not dragging
                    this.viewer.z_drag_state = Some((0.0, 0.0, 0.0, 0.0));
                    cx.notify();
                }
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                // Check if mouse button is actually pressed (safety check for button released outside window)
                let button_actually_pressed = event.pressed_button.is_some();
                
                // If we think the button is down but the event says it's not, correct our state
                if this.mouse_button_down && !button_actually_pressed {
                    this.mouse_button_down = false;
                    // End Z-drag zoom if active
                    if this.viewer.z_drag_state.is_some() {
                        this.viewer.z_drag_state = Some((0.0, 0.0, 0.0, 0.0));
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
                            let delta_x = current_x - last_x;  // Right is positive (zoom in)
                            let combined_delta = delta_y + delta_x;
                            
                            // Get the current zoom level (which changes during drag)
                            let current_zoom = this.viewer.image_state.zoom;
                            
                            // Scale zoom change proportionally to CURRENT zoom level
                            // At 100% zoom (1.0): 1% per pixel
                            // At 200% zoom (2.0): 2% per pixel (more sensitive)
                            // At 50% zoom (0.5): 0.5% per pixel (less sensitive)
                            let zoom_change = combined_delta * 0.01 * current_zoom;
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
                            this.viewer.z_drag_state = Some((current_x, current_y, center_x, center_y));
                            
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
                    
                    this.viewer.zoom_toward_point(cursor_x, cursor_y, zoom_in, utils::zoom::ZOOM_STEP_WHEEL);
                    this.save_current_image_state();
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                // Check for Z key press (without modifiers)
                if event.keystroke.key.as_str() == "z" 
                    && !event.keystroke.modifiers.shift 
                    && !event.keystroke.modifiers.control 
                    && !event.keystroke.modifiers.platform 
                    && !event.keystroke.modifiers.alt {
                    // Enable Z-drag zoom mode
                    this.z_key_held = true;
                    cx.notify();
                }
            }))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _window, cx| {
                // Check for Z key release
                if event.keystroke.key.as_str() == "z" {
                    // Disable Z-drag zoom mode and save state
                    if this.z_key_held {
                        this.z_key_held = false;
                        this.save_current_image_state();
                        cx.notify();
                    }
                }
            }))
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
            // Zoom controls - base (normal speed)
            KeyBinding::new("=", ZoomIn, None),  // = key (same as +)
            KeyBinding::new("+", ZoomIn, None),
            KeyBinding::new("-", ZoomOut, None),
            KeyBinding::new("0", ZoomReset, None),
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
            // Slow pan with Ctrl/Cmd (1px)
            KeyBinding::new("cmd-w", PanUpSlow, None),
            KeyBinding::new("cmd-a", PanLeftSlow, None),
            KeyBinding::new("cmd-s", PanDownSlow, None),
            KeyBinding::new("cmd-d", PanRightSlow, None),
            KeyBinding::new("cmd-i", PanUpSlow, None),
            KeyBinding::new("cmd-j", PanLeftSlow, None),
            KeyBinding::new("cmd-k", PanDownSlow, None),
            KeyBinding::new("cmd-l", PanRightSlow, None),
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
                        z_drag_state: None,
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
                        z_key_held: false,
                        mouse_button_down: false,
                    }
                })
        })
        .unwrap();
    });
}
