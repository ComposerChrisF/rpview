pub mod animation_indicator;
pub mod debug_overlay;

/// Callback invoked when the user presses ESC while a floating companion
/// window (filter, local-contrast, …) has focus. The owning binary uses this
/// to close the window and tick the main App's quit counter; kept as a
/// closure so these window-root views can live in the crate shared between
/// lib and bin builds without depending on the binary's `App` type.
pub type EscapeCallback = Box<dyn Fn(&mut gpui::Window, &mut gpui::App) + 'static>;
pub mod error_display;
pub mod filter_controls;
pub mod filter_window;
pub mod gpu_pipeline_controls;
pub mod gpu_pipeline_window;
pub mod help_overlay;
pub mod image_viewer;
pub mod loading_indicator;
pub mod local_contrast_controls;
pub mod local_contrast_window;
#[cfg(not(target_os = "macos"))]
pub mod menu_bar;
pub mod processing_indicator;
pub mod settings_window;
pub mod zoom_indicator;

pub use debug_overlay::{DebugOverlay, DebugOverlayConfig};
pub use filter_controls::{FilterControls, FilterControlsEvent};
pub use filter_window::FilterWindowView;
pub use gpu_pipeline_controls::{GpuPipelineControls, GpuPipelineControlsEvent};
pub use gpu_pipeline_window::GpuPipelineWindowView;
pub use help_overlay::HelpOverlay;
pub use image_viewer::ImageViewer;
pub use local_contrast_controls::{LocalContrastControls, LocalContrastControlsEvent};
pub use local_contrast_window::LocalContrastWindowView;
#[cfg(not(target_os = "macos"))]
pub use menu_bar::MenuBar;
pub use settings_window::SettingsWindow;
