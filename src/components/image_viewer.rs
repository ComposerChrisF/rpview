#![allow(clippy::collapsible_if)]

use crate::OpenFile;
use crate::components::animation_indicator::AnimationIndicator;
use crate::components::error_display::ErrorDisplay;
use crate::components::processing_indicator::ProcessingIndicator;
use crate::components::zoom_indicator::ZoomIndicator;
use crate::state::ImageState;
use crate::state::image_state::FilterSettings;
use crate::utils::animation::AnimationData;
use crate::utils::debug_eprintln;
use crate::utils::filters;
use crate::utils::image_loader;
use crate::utils::style::{Colors, Spacing, TextSize};
use crate::utils::svg::SvgRerasterRegion;
use crate::utils::zoom;
use gpui::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;

/// Result type for SVG re-rasterization background tasks
type SvgRerasterResult = Result<(PathBuf, Option<SvgRerasterRegion>), String>;

/// Bundle of state for an in-flight local-contrast computation. `Some` while
/// a worker thread is running, `None` otherwise — consolidates what used to
/// be four lockstep fields (`is_processing_lc`, handle, cancel, progress).
/// Payload the LC worker thread sends back on completion: the rendered
/// image plus its pixel dimensions (which may differ from the source's when
/// the user picked a non-1× resize factor).
pub(crate) type LcResult = Option<(Arc<gpui::RenderImage>, (u32, u32))>;

/// Effective display dimensions for `loaded`. When an LC render is active
/// (enabled *and* present), its pixel size drives zoom/fit math and the
/// resolution readout so that "100%" matches the actual pixels on the GPU —
/// otherwise the source file's dimensions do.
fn effective_image_size(loaded: &LoadedImage, lc_enabled: bool) -> (u32, u32) {
    if lc_enabled
        && loaded.lc_render.is_some()
        && let Some(size) = loaded.lc_render_size
    {
        size
    } else {
        (loaded.width, loaded.height)
    }
}

pub(crate) struct LcJob {
    pub handle: std::sync::mpsc::Receiver<LcResult>,
    pub cancel: Arc<std::sync::atomic::AtomicBool>,
    pub progress_bips: Arc<std::sync::atomic::AtomicU32>,
}

impl LcJob {
    /// Progress of the background LC job as a percentage in `0.0..=100.0`.
    pub fn progress_percent(&self) -> f32 {
        let bips = self
            .progress_bips
            .load(std::sync::atomic::Ordering::Relaxed);
        (bips as f32) / 100.0
    }
    /// Signal the worker thread to abort at its next checkpoint.
    pub fn request_cancel(&self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Loaded image data
///
/// # Animation Frame Caching Strategy
///
/// For animated images (GIF, WEBP), frames are cached progressively to balance
/// responsiveness and performance:
///
/// **Phase 1: Initial Load** (in `load_image()`)
/// - Cache first 3 frames immediately to disk (~100-200ms)
/// - Pre-allocate empty PathBuf slots for remaining frames
/// - Display frame 0 immediately (fast UI feedback)
///
/// **Phase 2: Playback** (in `App::render()`)
/// - Cache next 3 frames ahead while animation plays (look-ahead caching)
/// - Frames are ready by the time playback reaches them
/// - After first loop, all frames are cached (smooth playback)
///
/// **Phase 3: GPU Preloading** (in `ImageViewer::render()`)
/// - Render next frame invisibly off-screen with `opacity(0.0)`
/// - Forces GPUI to load frame into GPU memory before display
/// - Eliminates black flashing between frames
///
/// This 3-phase approach provides:
/// - Fast initial display (user sees image within 200ms)
/// - No black flashing (GPU preload)
/// - Smooth playback after first loop (all frames cached)
#[derive(Clone)]
pub struct LoadedImage {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    /// Decoded RGBA source, cached in memory so the filter thread doesn't have to
    /// reload and re-decode from disk on every slider tick. Populated lazily on
    /// the first filter application.
    pub decoded_rgba8: Option<Arc<image::RgbaImage>>,
    /// In-memory filtered image (most-recent-filtered only). Handed directly to
    /// GPUI via `ImageSource::Render`, no temp file.
    pub filtered_render: Option<Arc<gpui::RenderImage>>,
    /// Filter settings used to produce `filtered_render` (for change detection).
    pub cached_filter_settings: Option<FilterSettings>,
    /// Planar-float copy of `decoded_rgba8` cached for the local-contrast
    /// worker. Avoids re-doing the f32/255 conversion (6–20ms for a large
    /// RGBA image) on every slider tick. Populated lazily on the first LC
    /// application.
    pub lc_source_fmap: Option<Arc<crate::utils::float_map::FloatMap>>,
    /// In-memory local-contrast output. Takes priority over `filtered_render`
    /// when present. Cleared on navigation or Reset.
    pub lc_render: Option<Arc<gpui::RenderImage>>,
    /// Pixel dimensions of `lc_render`. Differs from `width`/`height` when
    /// the LC dialog's resize factor is not 1× — zoom math and the readout
    /// need the effective size so e.g. 100% zoom matches the 2× output.
    pub lc_render_size: Option<(u32, u32)>,
    /// LC parameters used to produce `lc_render` (for change detection).
    pub cached_lc_params: Option<crate::utils::local_contrast::Parameters>,
    /// Animation data (if this is an animated image)
    pub animation_data: Option<AnimationData>,
    /// Cached paths for each animation frame (disk cache)
    /// Empty PathBuf means frame not yet cached (will be cached on-demand)
    pub frame_cache_paths: Vec<PathBuf>,
    /// Rasterized temp PNG path (for SVG files)
    pub rasterized_path: Option<PathBuf>,
    /// Parsed SVG tree for dynamic re-rendering at different zoom levels
    pub svg_tree: Option<Arc<resvg::usvg::Tree>>,
    /// Scale factor used for the initial SVG rasterization (typically 2.0)
    pub svg_base_scale: f32,
}

/// Component for viewing images
pub struct ImageViewer {
    /// Currently loaded image
    pub(crate) current_image: Option<LoadedImage>,
    /// Error message if image failed to load
    pub(crate) error_message: Option<String>,
    /// Path of the image that failed to load (for full path display)
    pub(crate) error_path: Option<PathBuf>,
    /// Path to directory with no images (for friendly notice, not an error)
    pub(crate) no_images_path: Option<PathBuf>,
    /// Oversized image warning: (path, width, height, max_dimension)
    pub(crate) oversized_image: Option<(PathBuf, u32, u32, u32)>,
    /// Focus handle for keyboard events
    pub(crate) focus_handle: FocusHandle,
    /// Current image state (zoom, pan, etc.)
    pub(crate) image_state: ImageState,
    /// Last known viewport size (for fit-to-window calculations)
    pub(crate) viewport_size: Option<Size<Pixels>>,
    // These fields are accessed from the binary crate (app_render.rs) but the lib crate
    // can't see that usage, so the compiler warns about dead code.
    /// Z key drag zoom state: outer Option = Z key held, inner Option = actively dragging
    /// Inner tuple: (last_mouse_x, last_mouse_y, zoom_center_x, zoom_center_y)
    #[allow(dead_code)]
    pub(crate) z_drag_state: Option<Option<(f32, f32, f32, f32)>>,
    /// Spacebar drag pan state: outer Option = spacebar held, inner Option = actively dragging
    /// Inner tuple: (last_mouse_x, last_mouse_y) for 1:1 pixel movement panning
    #[allow(dead_code)]
    pub(crate) spacebar_drag_state: Option<Option<(f32, f32)>>,
    /// Paths to preload into GPU (for smooth navigation)
    /// These images are rendered invisibly to prime the GPU texture cache
    pub(crate) preload_paths: Vec<PathBuf>,
    /// Active async loading operation
    pub(crate) loading_handle: Option<image_loader::LoaderHandle>,
    /// Loading state indicator
    pub(crate) is_loading: bool,
    /// Filter processing state
    pub(crate) is_processing_filters: bool,
    /// Handle for async filter processing — returns the filtered image in memory.
    pub(crate) filter_processing_handle:
        Option<std::sync::mpsc::Receiver<Result<Arc<gpui::RenderImage>, String>>>,

    /// In-flight local-contrast job (cancel flag, progress counter, result
    /// channel). `None` when no worker thread is running.
    pub(crate) lc_job: Option<LcJob>,
    /// Session-wide toggle for the LC output (so the `1`/`2` keys can A/B
    /// the processed image against the un-processed one).
    pub(crate) lc_enabled: bool,

    // --- SVG dynamic re-rasterization state ---
    /// Active sharp re-raster path (replaces blurry base raster when zoomed in)
    pub(crate) svg_reraster_path: Option<PathBuf>,
    /// Region info if this is a viewport-only re-raster (None = full render)
    pub(crate) svg_reraster_region: Option<SvgRerasterRegion>,
    /// Zoom level the active re-raster was rendered at
    pub(crate) svg_reraster_scale: Option<f32>,

    /// Pending re-raster path (GPU preloading before swap)
    pub(crate) pending_svg_reraster_path: Option<PathBuf>,
    /// Pending region info
    pub(crate) pending_svg_reraster_region: Option<SvgRerasterRegion>,
    /// GPU preload frame counter for pending re-raster
    pub(crate) pending_svg_reraster_preload_frames: u32,

    /// Channel receiving completed re-raster results from background thread
    pub(crate) svg_reraster_handle: Option<mpsc::Receiver<SvgRerasterResult>>,
    /// Whether a re-raster is currently in progress on a background thread
    pub(crate) is_svg_rerastering: bool,
    /// Cancel flag for in-flight re-raster
    pub(crate) svg_reraster_cancel: Option<Arc<Mutex<bool>>>,

    /// Timestamp of last zoom/pan change (for debouncing re-raster triggers)
    pub(crate) last_zoom_pan_change: Option<Instant>,
}

impl ImageViewer {
    pub fn new(focus_handle: FocusHandle) -> Self {
        Self {
            current_image: None,
            error_message: None,
            error_path: None,
            no_images_path: None,
            oversized_image: None,
            focus_handle,
            image_state: ImageState::new(),
            viewport_size: None,
            z_drag_state: None,
            spacebar_drag_state: None,
            preload_paths: Vec::new(),
            loading_handle: None,
            is_loading: false,
            is_processing_filters: false,
            filter_processing_handle: None,
            lc_job: None,
            lc_enabled: true,
            svg_reraster_path: None,
            svg_reraster_region: None,
            svg_reraster_scale: None,
            pending_svg_reraster_path: None,
            pending_svg_reraster_region: None,
            pending_svg_reraster_preload_frames: 0,
            svg_reraster_handle: None,
            is_svg_rerastering: false,
            svg_reraster_cancel: None,
            last_zoom_pan_change: None,
        }
    }

    /// Set the image state
    pub fn set_image_state(&mut self, state: ImageState) {
        self.image_state = state;
    }

    /// Get the current image state
    pub fn get_image_state(&self) -> ImageState {
        self.image_state.clone()
    }

    /// Calculate and set fit-to-window zoom for the current image
    pub fn fit_to_window(&mut self) {
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            let viewport_width: f32 = viewport.width.into();
            let viewport_height: f32 = viewport.height.into();
            let (eff_w, eff_h) = effective_image_size(img, self.lc_enabled);

            let fit_zoom =
                zoom::calculate_fit_to_window(eff_w, eff_h, viewport_width, viewport_height);

            // Calculate pan to center the image in the viewing area
            let zoomed_width = eff_w as f32 * fit_zoom;
            let zoomed_height = eff_h as f32 * fit_zoom;
            let pan_x = (viewport_width - zoomed_width) / 2.0;
            let pan_y = (viewport_height - zoomed_height) / 2.0;

            self.image_state.zoom = fit_zoom;
            self.image_state.is_fit_to_window = true;
            self.image_state.pan = (pan_x, pan_y);
        }
    }

    /// Update viewport size and recalculate fit-to-window if needed
    pub fn update_viewport_size(&mut self, size: Size<Pixels>) {
        let size_changed = self
            .viewport_size
            .map(|old| {
                let width_diff: f32 = (old.width - size.width).into();
                let height_diff: f32 = (old.height - size.height).into();
                width_diff.abs() > 1.0 || height_diff.abs() > 1.0
            })
            .unwrap_or(true);

        if size_changed {
            self.viewport_size = Some(size);

            // If we're in fit-to-window mode, recalculate
            if self.image_state.is_fit_to_window {
                self.fit_to_window();
            }
        }
    }

    /// Zoom in, keeping the center of the image at the same screen location
    pub fn zoom_in(&mut self, step: f32) {
        let old_zoom = self.image_state.zoom;
        let new_zoom = zoom::zoom_in(old_zoom, step);

        // Adjust pan to keep center of image at same screen location (if we have the data)
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            let (eff_w, eff_h) = effective_image_size(img, self.lc_enabled);
            self.adjust_pan_for_zoom(eff_w, eff_h, viewport, old_zoom, new_zoom);
        }

        self.image_state.zoom = new_zoom;
        self.image_state.is_fit_to_window = false;
    }

    /// Zoom out, keeping the center of the image at the same screen location
    pub fn zoom_out(&mut self, step: f32) {
        let old_zoom = self.image_state.zoom;
        let new_zoom = zoom::zoom_out(old_zoom, step);

        // Adjust pan to keep center of image at same screen location (if we have the data)
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            let (eff_w, eff_h) = effective_image_size(img, self.lc_enabled);
            self.adjust_pan_for_zoom(eff_w, eff_h, viewport, old_zoom, new_zoom);
        }

        self.image_state.zoom = new_zoom;
        self.image_state.is_fit_to_window = false;
    }

    /// Adjust pan so the image pixel at the viewport center stays at the viewport center after zoom.
    /// Same math as zoom_toward_point but anchored on the viewport center instead of cursor.
    fn adjust_pan_for_zoom(
        &mut self,
        _img_width: u32,
        _img_height: u32,
        viewport: Size<Pixels>,
        old_zoom: f32,
        new_zoom: f32,
    ) {
        let (pan_x, pan_y) = self.image_state.pan;
        let vp_center_x: f32 = f32::from(viewport.width) / 2.0;
        let vp_center_y: f32 = f32::from(viewport.height) / 2.0;

        // Find which image pixel is currently at the viewport center
        let pixel_x = (vp_center_x - pan_x) / old_zoom;
        let pixel_y = (vp_center_y - pan_y) / old_zoom;

        // Compute new pan so that same pixel remains at the viewport center
        let new_pan_x = vp_center_x - pixel_x * new_zoom;
        let new_pan_y = vp_center_y - pixel_y * new_zoom;

        // Apply pan constraints
        self.image_state.pan = self.constrain_pan(new_pan_x, new_pan_y);
    }

    /// Toggle between fit-to-window (centered) and 100% zoom.
    /// When going to fit-to-window, the image is fully centered.
    /// When going to 100%, the viewport-center anchor point is preserved.
    pub fn reset_zoom(&mut self) {
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            if self.image_state.is_fit_to_window {
                // Currently at fit-to-window → switch to 100% keeping viewport center stable
                let old_zoom = self.image_state.zoom;
                let (eff_w, eff_h) = effective_image_size(img, self.lc_enabled);
                self.adjust_pan_for_zoom(eff_w, eff_h, viewport, old_zoom, 1.0);
                self.image_state.zoom = 1.0;
                self.image_state.is_fit_to_window = false;
            } else {
                // Any other zoom → switch to fit-to-window, fully centered
                self.fit_to_window();
            }
        }
    }

    /// Set zoom to 100% (actual size) with image centered
    pub fn set_one_hundred_percent(&mut self) {
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            let viewport_width: f32 = viewport.width.into();
            let viewport_height: f32 = viewport.height.into();
            let (eff_w, eff_h) = effective_image_size(img, self.lc_enabled);

            let zoomed_width = eff_w as f32;
            let zoomed_height = eff_h as f32;
            let pan_x = (viewport_width - zoomed_width) / 2.0;
            let pan_y = (viewport_height - zoomed_height) / 2.0;

            self.image_state.zoom = 1.0;
            self.image_state.pan = (pan_x, pan_y);
            self.image_state.is_fit_to_window = false;
        }
    }

    /// Toggle between fit-to-window (centered) and 100% (centered).
    /// Both states are fully centered — this is Cmd+0 / Ctrl+0.
    pub fn reset_zoom_and_pan(&mut self) {
        if self.image_state.is_fit_to_window {
            self.set_one_hundred_percent();
        } else {
            self.fit_to_window();
        }
    }

    /// Pan the image with constraints to prevent panning completely off-screen
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let (pan_x, pan_y) = self.image_state.pan;
        let new_pan_x = pan_x + delta_x;
        let new_pan_y = pan_y + delta_y;
        self.image_state.pan = self.constrain_pan(new_pan_x, new_pan_y);
    }

    /// Constrain pan to prevent the image from going completely off-screen
    /// Ensures at least a small portion of the image remains visible
    fn constrain_pan(&self, pan_x: f32, pan_y: f32) -> (f32, f32) {
        if let (Some(img), Some(viewport)) = (&self.current_image, self.viewport_size) {
            let viewport_width: f32 = viewport.width.into();
            let viewport_height: f32 = viewport.height.into();
            let (eff_w, eff_h) = effective_image_size(img, self.lc_enabled);

            let zoomed_width = eff_w as f32 * self.image_state.zoom;
            let zoomed_height = eff_h as f32 * self.image_state.zoom;

            // Define minimum visible portion (e.g., 50 pixels or 10% of image, whichever is smaller)
            let min_visible_x = (zoomed_width * 0.1).min(50.0);
            let min_visible_y = (zoomed_height * 0.1).min(50.0);

            // Calculate allowed pan range
            // Image can be panned right until only min_visible_x pixels show on the left
            let max_pan_x = viewport_width - min_visible_x;
            // Image can be panned left until only min_visible_x pixels show on the right
            let min_pan_x = -(zoomed_width - min_visible_x);

            // Image can be panned down until only min_visible_y pixels show on the top
            let max_pan_y = viewport_height - min_visible_y;
            // Image can be panned up until only min_visible_y pixels show on the bottom
            let min_pan_y = -(zoomed_height - min_visible_y);

            // Clamp pan values to allowed range
            let constrained_x = pan_x.max(min_pan_x).min(max_pan_x);
            let constrained_y = pan_y.max(min_pan_y).min(max_pan_y);

            (constrained_x, constrained_y)
        } else {
            // No image or viewport, return unconstrained values
            (pan_x, pan_y)
        }
    }

    /// Zoom toward a specific point (cursor position)
    /// cursor_x and cursor_y are in viewport coordinates (pixels from top-left of viewport)
    pub fn zoom_toward_point(&mut self, cursor_x: f32, cursor_y: f32, zoom_in: bool, step: f32) {
        if self.current_image.is_none() {
            return;
        }

        let old_zoom = self.image_state.zoom;
        let new_zoom = if zoom_in {
            zoom::zoom_in(old_zoom, step)
        } else {
            zoom::zoom_out(old_zoom, step)
        };

        // Calculate the cursor position in image coordinates (before zoom)
        let (pan_x, pan_y) = self.image_state.pan;
        let cursor_in_image_x = (cursor_x - pan_x) / old_zoom;
        let cursor_in_image_y = (cursor_y - pan_y) / old_zoom;

        // Calculate the new pan to keep the cursor at the same image location
        let new_pan_x = cursor_x - cursor_in_image_x * new_zoom;
        let new_pan_y = cursor_y - cursor_in_image_y * new_zoom;

        // Update zoom first, then constrain pan
        self.image_state.zoom = new_zoom;
        self.image_state.pan = self.constrain_pan(new_pan_x, new_pan_y);
        self.image_state.is_fit_to_window = false;
    }

    /// Load an image from a path (synchronous, legacy)
    ///
    /// For animated images (GIF, WEBP), this implements a progressive loading strategy:
    /// 1. Cache first 3 frames immediately for instant display
    /// 2. Pre-allocate slots for remaining frames
    /// 3. Remaining frames are cached on-demand during playback (see `cache_frame()`)
    ///
    /// This approach provides:
    /// - Fast initial display (~100-200ms instead of waiting for all frames)
    /// - Smooth playback after first loop (all frames cached)
    /// - No black flashing (GPU preloading in render)
    ///
    /// Note: This method is kept for testing but not used in production.
    /// Use `load_image_async()` instead for non-blocking loading.
    #[allow(dead_code)]
    pub fn load_image(&mut self, path: PathBuf) {
        // Get dimensions to validate the image can be loaded
        match image_loader::get_image_dimensions(&path) {
            Ok((width, height)) => {
                // Try to load animation data if it's an animated image
                let animation_data = crate::utils::animation::load_animation(&path)
                    .ok()
                    .flatten();

                // Cache first 3 frames immediately for instant display, rest will load in background
                let mut frame_cache_paths = Vec::new();
                if let Some(ref anim_data) = animation_data {
                    if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
                        let base_name = format!(
                            "rpview_{}_{}",
                            std::process::id(),
                            path.file_name().and_then(|n| n.to_str()).unwrap_or("anim")
                        );

                        // Cache first 3 frames for immediate display (gives UI time to show)
                        let initial_cache_count = std::cmp::min(3, anim_data.frames.len());
                        debug_eprintln!(
                            "[LOAD] Caching first {} frames for immediate display...",
                            initial_cache_count
                        );
                        for i in 0..initial_cache_count {
                            let temp_path = temp_dir.join(format!("{}_{}.png", base_name, i));
                            match anim_data.frames[i].image.save(&temp_path) {
                                Ok(_) => {
                                    debug_eprintln!("[LOAD] Cached frame {}", i);
                                    frame_cache_paths.push(temp_path);
                                }
                                Err(_e) => {
                                    debug_eprintln!("[ERROR] Failed to cache frame {}: {}", i, _e);
                                    frame_cache_paths.push(PathBuf::new());
                                }
                            }
                        }

                        // Pre-allocate paths for remaining frames (will be filled on-demand)
                        for _ in initial_cache_count..anim_data.frames.len() {
                            frame_cache_paths.push(PathBuf::new());
                        }
                        debug_eprintln!(
                            "[LOAD] Initial caching complete: {}/{} frames ready",
                            initial_cache_count,
                            anim_data.frames.len()
                        );
                    }
                }

                // Initialize animation state if we have animation data
                if let Some(ref anim_data) = animation_data {
                    use crate::state::image_state::AnimationState;
                    let mut anim_state =
                        AnimationState::new(anim_data.frame_count, anim_data.frame_durations());
                    // First few frames are cached, rest will load on-demand
                    // Check if we have at least 2 frames cached (frame 0 and frame 1)
                    let cached_count = frame_cache_paths
                        .iter()
                        .filter(|p| !p.as_os_str().is_empty() && p.exists())
                        .count();
                    anim_state.next_frame_ready = cached_count >= 2;
                    self.image_state.animation = Some(anim_state);
                } else {
                    self.image_state.animation = None;
                }

                self.current_image = Some(LoadedImage {
                    path: path.clone(),
                    width,
                    height,
                    decoded_rgba8: None,
                    filtered_render: None,
                    cached_filter_settings: None,
                    lc_source_fmap: None,
                    lc_render: None,
                    lc_render_size: None,
                    cached_lc_params: None,
                    animation_data,
                    frame_cache_paths,
                    rasterized_path: None,
                    svg_tree: None,
                    svg_base_scale: 2.0,
                });
                self.error_message = None;
                self.error_path = None;
                self.no_images_path = None;

                // Fit to window on load (if viewport size is known)
                self.fit_to_window();
            }
            Err(e) => {
                self.current_image = None;
                self.error_message = Some(e.to_string());
                self.error_path = Some(path);
            }
        }
    }

    /// Start loading an image asynchronously in the background
    pub fn load_image_async(
        &mut self,
        path: PathBuf,
        max_dimension: Option<u32>,
        force_load: bool,
    ) {
        // Cancel any previous loading operation
        if let Some(handle) = self.loading_handle.take() {
            handle.cancel();
        }

        // Start new async load
        debug_eprintln!("[ASYNC] Starting async load for: {}", path.display());
        self.loading_handle = Some(image_loader::load_image_async(
            path,
            max_dimension,
            force_load,
        ));
        self.is_loading = true;

        // Clear previous image and errors
        self.current_image = None;
        self.error_message = None;
        self.error_path = None;
        self.no_images_path = None;

        // Clear SVG re-raster state from previous image
        self.clear_svg_reraster_state();
    }

    /// Check if async loading has completed and process the result
    /// Returns true if an image was loaded or an error occurred
    pub fn check_async_load(&mut self) -> bool {
        if let Some(handle) = &self.loading_handle {
            if let Some(msg) = handle.try_recv() {
                // Clear the handle since loading is complete
                self.loading_handle = None;
                self.is_loading = false;

                match msg {
                    image_loader::LoaderMessage::Success(mut data) => {
                        debug_eprintln!("[ASYNC] Load complete: {}", data.path.display());

                        // Prepare frame cache paths
                        let mut frame_cache_paths = std::mem::take(&mut data.initial_frame_paths);
                        if let Some(ref anim_data) = data.animation_data {
                            // Pre-allocate empty slots for remaining frames
                            while frame_cache_paths.len() < anim_data.frames.len() {
                                frame_cache_paths.push(PathBuf::new());
                            }
                        }

                        // Initialize animation state if we have animation data
                        if let Some(ref anim_data) = data.animation_data {
                            use crate::state::image_state::AnimationState;
                            let mut anim_state = AnimationState::new(
                                anim_data.frame_count,
                                anim_data.frame_durations(),
                            );
                            let cached_count = frame_cache_paths
                                .iter()
                                .filter(|p| !p.as_os_str().is_empty() && p.exists())
                                .count();
                            anim_state.next_frame_ready = cached_count >= 2;
                            self.image_state.animation = Some(anim_state);
                        } else {
                            self.image_state.animation = None;
                        }

                        self.current_image = Some(LoadedImage {
                            path: data.path,
                            width: data.width,
                            height: data.height,
                            decoded_rgba8: None,
                            filtered_render: None,
                            cached_filter_settings: None,
                            lc_source_fmap: None,
                            lc_render: None,
                            lc_render_size: None,
                            cached_lc_params: None,
                            animation_data: data.animation_data,
                            frame_cache_paths,
                            rasterized_path: data.rasterized_path,
                            svg_tree: data.svg_tree,
                            svg_base_scale: 2.0,
                        });
                        self.error_message = None;
                        self.error_path = None;
                        self.no_images_path = None;
                        self.oversized_image = None;

                        // Fit to window on load
                        self.fit_to_window();

                        return true;
                    }
                    image_loader::LoaderMessage::Error(path, msg) => {
                        debug_eprintln!("[ASYNC] Load failed: {}: {}", path.display(), msg);
                        self.current_image = None;
                        self.error_message = Some(msg);
                        self.error_path = Some(path);
                        self.oversized_image = None;

                        return true;
                    }
                    image_loader::LoaderMessage::OversizedImage(path, width, height, max_dim) => {
                        debug_eprintln!(
                            "[ASYNC] Image oversized: {}×{} exceeds max {}",
                            width,
                            height,
                            max_dim
                        );
                        self.current_image = None;
                        self.error_message = None;
                        self.error_path = None;
                        self.no_images_path = None;
                        self.oversized_image = Some((path, width, height, max_dim));

                        return true;
                    }
                }
            }
        }

        false
    }

    /// Update filtered image cache if needed (async)
    pub fn update_filtered_cache(&mut self) {
        debug_eprintln!("[ImageViewer::update_filtered_cache] Called");

        // Filters not supported for SVG files (image::open can't read SVGs)
        if let Some(ref loaded) = self.current_image {
            if crate::utils::file_scanner::is_svg(&loaded.path) {
                return;
            }
        }

        // Cancel any previous filter processing when starting new one
        if self.is_processing_filters {
            debug_eprintln!("[ImageViewer::update_filtered_cache] Canceling previous processing");
            self.filter_processing_handle = None;
            self.is_processing_filters = false;
        }

        let Some(loaded) = self.current_image.as_mut() else {
            return;
        };

        let filters = self.image_state.filters;
        let filters_enabled = self.image_state.filters_enabled;

        let is_noop = !filters_enabled
            || (filters.brightness.abs() < 0.001
                && filters.contrast.abs() < 0.001
                && (filters.gamma - 1.0).abs() < 0.001);

        let needs_update = if is_noop {
            loaded.filtered_render.is_some()
        } else {
            loaded.cached_filter_settings.as_ref() != Some(&filters)
        };

        debug_eprintln!(
            "[ImageViewer::update_filtered_cache] noop={}, needs_update={}",
            is_noop,
            needs_update
        );

        if !needs_update {
            self.is_processing_filters = false;
            return;
        }

        if is_noop {
            // Drop the filtered buffer; render will fall back to the original.
            loaded.filtered_render = None;
            loaded.cached_filter_settings = None;
            self.is_processing_filters = false;
            return;
        }

        // Lazily decode the source RGBA8 the first time we need to filter this image.
        // All subsequent slider ticks reuse the same decoded buffer — no disk I/O.
        if loaded.decoded_rgba8.is_none() {
            match image_loader::load_image(&loaded.path) {
                Ok(img) => {
                    loaded.decoded_rgba8 = Some(Arc::new(img.to_rgba8()));
                }
                Err(_e) => {
                    debug_eprintln!("[ImageViewer::update_filtered_cache] Decode failed: {}", _e);
                    return;
                }
            }
        }
        let source = loaded
            .decoded_rgba8
            .as_ref()
            .expect("decoded source just populated")
            .clone();

        self.is_processing_filters = true;
        let (sender, receiver) = std::sync::mpsc::channel();
        self.filter_processing_handle = Some(receiver);

        std::thread::spawn(move || {
            debug_eprintln!("[FILTER_THREAD] LUT pass starting");
            let bgra = filters::apply_filters_to_bgra(
                &source,
                filters.brightness,
                filters.contrast,
                filters.gamma,
            );
            let frame = image::Frame::new(bgra);
            let render_image = Arc::new(gpui::RenderImage::new(smallvec::SmallVec::from_elem(
                frame, 1,
            )));
            debug_eprintln!("[FILTER_THREAD] LUT pass complete");
            let _ = sender.send(Ok(render_image));
        });
    }

    /// Kick off (or cancel and restart) local-contrast processing for the
    /// current image using `params`. A worker thread reads the cached
    /// `decoded_rgba8`, runs `locally_normalize_luminance`, and sends the
    /// resulting in-memory image back via an mpsc channel. Cancelled by
    /// flipping an `AtomicBool` the worker's feedback callback watches.
    pub fn update_local_contrast(&mut self, params: crate::utils::local_contrast::Parameters) {
        use std::sync::atomic::{AtomicBool, Ordering};
        // LC doesn't make sense for SVGs (rasterized at a fixed scale).
        if let Some(ref loaded) = self.current_image {
            if crate::utils::file_scanner::is_svg(&loaded.path) {
                return;
            }
        }

        // Cancel any in-flight LC computation.
        if let Some(job) = self.lc_job.take() {
            job.request_cancel();
        }

        let Some(loaded) = self.current_image.as_mut() else {
            return;
        };

        if params.is_identity() {
            loaded.lc_render = None;
            loaded.lc_render_size = None;
            loaded.cached_lc_params = None;
            return;
        }

        // Skip re-processing if params haven't changed since the last render.
        if loaded.cached_lc_params.as_ref() == Some(&params) && loaded.lc_render.is_some() {
            return;
        }

        // Lazily decode the source and build the planar float copy (same
        // pattern as filters). The f32/255 conversion is 6–20ms for a large
        // RGBA image and would otherwise repeat on every slider tick.
        if loaded.decoded_rgba8.is_none() {
            match image_loader::load_image(&loaded.path) {
                Ok(img) => {
                    loaded.decoded_rgba8 = Some(Arc::new(img.to_rgba8()));
                }
                Err(_e) => {
                    debug_eprintln!("[LC] Decode failed: {}", _e);
                    return;
                }
            }
        }
        if loaded.lc_source_fmap.is_none() {
            let rgba = loaded
                .decoded_rgba8
                .as_ref()
                .expect("decoded source just populated");
            loaded.lc_source_fmap = Some(Arc::new(crate::utils::float_map::FloatMap::from_rgba8(
                rgba,
            )));
        }
        let fmap = loaded
            .lc_source_fmap
            .as_ref()
            .expect("float map just populated")
            .clone();

        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_thread = cancel.clone();
        let progress_bips = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let progress_for_thread = progress_bips.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        self.lc_job = Some(LcJob {
            handle: rx,
            cancel,
            progress_bips,
        });

        std::thread::spawn(move || {
            use crate::utils::local_contrast::FeedbackFn;
            debug_eprintln!("[LC_THREAD] start");
            let cancel_watch = cancel_for_thread.clone();
            let progress_watch = progress_for_thread.clone();
            let feedback: Box<FeedbackFn> = Box::new(move |p, _msg| {
                let bips = (p * 10_000.0).clamp(0.0, 10_000.0) as u32;
                progress_watch.store(bips, Ordering::Relaxed);
                cancel_watch.load(Ordering::Relaxed)
            });
            let out = crate::utils::local_contrast::locally_normalize_luminance(
                fmap.as_ref(),
                &params,
                Some(&*feedback),
            );
            let result = out.map(|out_map| {
                let size = (out_map.width, out_map.height);
                let bgra = out_map.to_bgra_image();
                let frame = image::Frame::new(bgra);
                let render = Arc::new(gpui::RenderImage::new(smallvec::SmallVec::from_elem(
                    frame, 1,
                )));
                (render, size)
            });
            progress_for_thread.store(10_000, Ordering::Relaxed);
            debug_eprintln!("[LC_THREAD] done, produced={}", result.is_some());
            let _ = tx.send(result);
        });
    }

    /// `true` while a worker thread is running.
    pub fn is_processing_lc(&self) -> bool {
        self.lc_job.is_some()
    }

    /// Current LC progress in percent (0.0–100.0) while processing. Returns
    /// `None` when nothing is in-flight.
    pub fn lc_progress_percent(&self) -> Option<f32> {
        self.lc_job.as_ref().map(LcJob::progress_percent)
    }

    /// Cancel any in-flight LC computation. The worker observes the cancel
    /// flag at its next checkpoint and drops its result silently.
    pub fn cancel_lc_processing(&mut self) {
        if let Some(job) = self.lc_job.take() {
            job.request_cancel();
        }
    }

    /// A/B toggle: hide the LC render so the main window shows the
    /// unprocessed (or filter-processed) image. Does not destroy
    /// `LoadedImage.lc_render`, so re-enabling is free.
    pub fn set_lc_enabled(&mut self, enabled: bool) {
        let changed = self.lc_enabled != enabled;
        self.lc_enabled = enabled;
        // If the effective image size just flipped (LC output differs from
        // the source's dimensions), refit so the on-screen image still fills
        // the viewport instead of overflowing or shrinking.
        if changed && self.image_state.is_fit_to_window {
            self.fit_to_window();
        }
    }
    #[allow(dead_code)]
    pub fn is_lc_enabled(&self) -> bool {
        self.lc_enabled
    }

    /// Check for completed LC processing; if a result arrived, install it on
    /// the current image. Returns true if a new LC render was applied.
    pub fn check_lc_processing(&mut self) -> bool {
        let Some(job) = self.lc_job.as_ref() else {
            return false;
        };
        let Ok(result) = job.handle.try_recv() else {
            return false;
        };
        self.lc_job = None;
        match result {
            Some((render_image, size)) => {
                let size_changed = if let Some(loaded) = self.current_image.as_mut() {
                    let prev = loaded.lc_render_size;
                    loaded.lc_render = Some(render_image);
                    loaded.lc_render_size = Some(size);
                    // We don't round-trip the params from the worker here;
                    // the caller that invoked `update_local_contrast` is
                    // responsible for treating its input params as the
                    // "current" cache key. Good enough for MVP — see the
                    // re-entrancy test in the test module.
                    prev != Some(size)
                } else {
                    false
                };
                // If the LC output's pixel dimensions just changed (e.g. the
                // user nudged the resize factor from 1× to 2×), refit so the
                // on-screen image stays inside the viewport at the new size.
                if size_changed && self.image_state.is_fit_to_window && self.lc_enabled {
                    self.fit_to_window();
                }
                true
            }
            None => false,
        }
    }

    /// Check for completed filter processing and install the resulting in-memory image.
    /// Returns true if a new filtered image was applied this tick (caller may want to notify).
    pub fn check_filter_processing(&mut self) -> bool {
        let Some(receiver) = &self.filter_processing_handle else {
            return false;
        };
        let Ok(result) = receiver.try_recv() else {
            return false;
        };
        self.is_processing_filters = false;
        self.filter_processing_handle = None;
        match result {
            Ok(render_image) => {
                if let Some(loaded) = self.current_image.as_mut() {
                    loaded.filtered_render = Some(render_image);
                    loaded.cached_filter_settings = Some(self.image_state.filters);
                }
                true
            }
            Err(_e) => {
                debug_eprintln!(
                    "[ImageViewer::check_filter_processing] Filter failed: {}",
                    _e
                );
                false
            }
        }
    }

    // --- SVG dynamic re-rasterization methods ---

    /// Notify that zoom or pan changed, starting the debounce timer for SVG re-rasters.
    pub fn notify_svg_zoom_pan_changed(&mut self) {
        if let Some(ref loaded) = self.current_image {
            if loaded.svg_tree.is_some() {
                self.last_zoom_pan_change = Some(Instant::now());
            }
        }
    }

    /// Check whether a new SVG re-raster should be triggered.
    /// Call from the render loop; returns true if a background render was spawned.
    pub fn check_svg_reraster_needed(&mut self) -> bool {
        use crate::utils::svg;

        // Must have a pending debounce timestamp
        let Some(changed_at) = self.last_zoom_pan_change else {
            return false;
        };

        // Debounce: wait at least 150ms since the last zoom/pan change
        if changed_at.elapsed().as_millis() < 150 {
            return false;
        }

        // Don't start a new render if one is already in flight
        if self.is_svg_rerastering {
            return false;
        }

        let Some(loaded) = &self.current_image else {
            return false;
        };

        let Some(tree) = &loaded.svg_tree else {
            return false;
        };

        let current_zoom = self.image_state.zoom;
        let base_scale = loaded.svg_base_scale;

        // If current zoom is at or below the base scale (with 10% tolerance), the
        // initial rasterization is sharp enough — no need to re-render.
        if current_zoom <= base_scale * 1.1 {
            // Clear any existing re-raster since we're zoomed out enough
            if let Some(ref path) = self.svg_reraster_path {
                let _ = std::fs::remove_file(path);
            }
            self.svg_reraster_path = None;
            self.svg_reraster_region = None;
            self.svg_reraster_scale = None;
            self.last_zoom_pan_change = None;
            return false;
        }

        // If we already have a re-raster at this scale, check if viewport is still covered
        if let Some(existing_scale) = self.svg_reraster_scale {
            let scale_ratio = current_zoom / existing_scale;
            if (scale_ratio - 1.0).abs() < 0.05 {
                // Same scale — check if viewport is within the padded region (viewport-only case)
                if let Some(ref region) = self.svg_reraster_region {
                    if let Some(viewport) = self.viewport_size {
                        let vp_w: f32 = viewport.width.into();
                        let vp_h: f32 = viewport.height.into();
                        let (pan_x, pan_y) = self.image_state.pan;

                        // Visible SVG-space rect
                        let vis_x = -pan_x / current_zoom;
                        let vis_y = -pan_y / current_zoom;
                        let vis_w = vp_w / current_zoom;
                        let vis_h = vp_h / current_zoom;

                        // Check if fully contained in rendered region
                        if vis_x >= region.svg_x
                            && vis_y >= region.svg_y
                            && vis_x + vis_w <= region.svg_x + region.svg_w
                            && vis_y + vis_h <= region.svg_y + region.svg_h
                        {
                            self.last_zoom_pan_change = None;
                            return false;
                        }
                    }
                } else {
                    // Full render at same scale — no need to re-render
                    self.last_zoom_pan_change = None;
                    return false;
                }
            }
        }

        // Cancel any in-flight re-raster
        if let Some(ref cancel) = self.svg_reraster_cancel {
            if let Ok(mut flag) = cancel.lock() {
                *flag = true;
            }
        }
        self.svg_reraster_handle = None;
        self.svg_reraster_cancel = None;

        // Determine full vs viewport-only strategy
        let svg_size = tree.size();
        let full_w = (svg_size.width() * current_zoom).ceil() as u64;
        let full_h = (svg_size.height() * current_zoom).ceil() as u64;
        let total_pixels = full_w * full_h;

        let tree_clone = Arc::clone(tree);
        let (tx, rx) = mpsc::channel();
        let cancel_flag = Arc::new(Mutex::new(false));
        let cancel_clone = cancel_flag.clone();

        if total_pixels <= svg::MAX_FULL_RERASTER_PIXELS {
            // Full re-raster
            let zoom = current_zoom;
            std::thread::spawn(move || {
                if cancel_clone.lock().map(|f| *f).unwrap_or(false) {
                    return;
                }
                let result = svg::rerasterize_svg_full(&tree_clone, zoom).map(|path| (path, None));
                let _ = tx.send(result);
            });
        } else {
            // Viewport-only re-raster
            let viewport = self.viewport_size;
            let (pan_x, pan_y) = self.image_state.pan;
            let zoom = current_zoom;

            std::thread::spawn(move || {
                if cancel_clone.lock().map(|f| *f).unwrap_or(false) {
                    return;
                }
                let vp = match viewport {
                    Some(v) => v,
                    None => return,
                };
                let vp_w: f32 = vp.width.into();
                let vp_h: f32 = vp.height.into();

                // Convert viewport to SVG-space coordinates
                let vis_x = -pan_x / zoom;
                let vis_y = -pan_y / zoom;
                let vis_w = vp_w / zoom;
                let vis_h = vp_h / zoom;

                let result = svg::rerasterize_svg_viewport(
                    &tree_clone,
                    (vis_x, vis_y, vis_w, vis_h),
                    svg::VIEWPORT_PADDING_FACTOR,
                    zoom,
                )
                .map(|(path, region)| (path, Some(region)));

                let _ = tx.send(result);
            });
        }

        self.svg_reraster_handle = Some(rx);
        self.is_svg_rerastering = true;
        self.svg_reraster_cancel = Some(cancel_flag);
        self.last_zoom_pan_change = None;

        true
    }

    /// Poll the background re-raster thread. Returns true if a result was received.
    pub fn check_svg_reraster_processing(&mut self) -> bool {
        if let Some(ref receiver) = self.svg_reraster_handle {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok((path, region)) => {
                        self.pending_svg_reraster_path = Some(path);
                        self.pending_svg_reraster_region = region;
                        debug_eprintln!("[SVG] Re-raster complete, pending GPU preload");
                    }
                    Err(_e) => {
                        debug_eprintln!("[SVG] Re-raster failed: {}", _e);
                    }
                }
                self.is_svg_rerastering = false;
                self.svg_reraster_handle = None;
                self.svg_reraster_cancel = None;
                return true;
            }
        }
        false
    }

    /// Swap the pending re-raster into the active slot after GPU preloading.
    pub fn apply_pending_svg_reraster(&mut self) {
        if let Some(pending_path) = self.pending_svg_reraster_path.take() {
            // Delete old re-raster temp file
            if let Some(ref old_path) = self.svg_reraster_path {
                let _ = std::fs::remove_file(old_path);
            }

            self.svg_reraster_path = Some(pending_path);
            self.svg_reraster_region = self.pending_svg_reraster_region.take();
            self.svg_reraster_scale = Some(self.image_state.zoom);
            self.pending_svg_reraster_preload_frames = 0;

            debug_eprintln!(
                "[SVG] Applied re-raster at zoom {:.2}",
                self.image_state.zoom
            );
        }
    }

    /// Clean up all SVG re-raster state (call on image change).
    pub fn clear_svg_reraster_state(&mut self) {
        // Cancel in-flight render
        if let Some(ref cancel) = self.svg_reraster_cancel {
            if let Ok(mut flag) = cancel.lock() {
                *flag = true;
            }
        }

        // Delete temp files
        if let Some(ref path) = self.svg_reraster_path {
            let _ = std::fs::remove_file(path);
        }
        if let Some(ref path) = self.pending_svg_reraster_path {
            let _ = std::fs::remove_file(path);
        }

        self.svg_reraster_path = None;
        self.svg_reraster_region = None;
        self.svg_reraster_scale = None;
        self.pending_svg_reraster_path = None;
        self.pending_svg_reraster_region = None;
        self.pending_svg_reraster_preload_frames = 0;
        self.svg_reraster_handle = None;
        self.is_svg_rerastering = false;
        self.svg_reraster_cancel = None;
        self.last_zoom_pan_change = None;
    }

    /// Clear the current image
    pub fn clear(&mut self) {
        self.current_image = None;
        self.error_message = None;
        self.error_path = None;
        self.no_images_path = None;
    }

    /// Cache a specific animation frame to disk if not already cached
    ///
    /// This is part of the progressive loading strategy for animations.
    /// Called from the render loop to cache frames 3+ ahead of playback.
    ///
    /// # Arguments
    /// * `frame_index` - The frame index to cache (0-based)
    ///
    /// # Returns
    /// * `true` if the frame is now cached (either was already cached or just cached)
    /// * `false` if caching failed or this is not an animated image
    ///
    /// # Performance
    /// Caching happens synchronously but is called during animation playback,
    /// so it happens while previous frames are being displayed (non-blocking UX).
    pub fn cache_frame(&mut self, frame_index: usize) -> bool {
        let Some(loaded) = self.current_image.as_mut() else {
            return false;
        };

        let Some(anim_data) = &loaded.animation_data else {
            return false;
        };

        // Check if frame is already cached
        if frame_index < loaded.frame_cache_paths.len() {
            let cached_path = &loaded.frame_cache_paths[frame_index];
            if !cached_path.as_os_str().is_empty() && cached_path.exists() {
                return true; // Already cached
            }
        }

        // Cache the frame
        if frame_index < anim_data.frames.len() {
            if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
                let base_name = match loaded.path.file_name() {
                    Some(name) => {
                        format!("rpview_{}_{}", std::process::id(), name.to_string_lossy())
                    }
                    None => return false,
                };
                let temp_path = temp_dir.join(format!("{}_{}.png", base_name, frame_index));

                // Save frame to disk
                match anim_data.frames[frame_index].image.save(&temp_path) {
                    Ok(_) => {
                        debug_eprintln!("[CACHE] Cached frame {} on-demand", frame_index);
                        // Update the cache path
                        if frame_index < loaded.frame_cache_paths.len() {
                            loaded.frame_cache_paths[frame_index] = temp_path;
                        }
                        return true;
                    }
                    Err(_e) => {
                        debug_eprintln!("[ERROR] Failed to cache frame {}: {}", frame_index, _e);
                    }
                }
            }
        }

        false
    }
}

impl ImageViewer {
    /// Render the image viewer as an element (for inline rendering without cx.new())
    pub fn render_view<V: 'static>(
        &self,
        background_color: [u8; 3],
        overlay_transparency: u8,
        font_size_scale: f32,
        show_zoom_indicator: bool,
        cx: &mut Context<V>,
    ) -> impl IntoElement {
        if self.is_loading {
            // Show loading indicator
            use crate::components::loading_indicator::LoadingIndicator;
            let text_color = Colors::text_for_background(background_color);
            div()
                .size_full()
                .child(cx.new(|_cx| {
                    LoadingIndicator::new("Loading image...").with_text_color(text_color)
                }))
                .into_any_element()
        } else if let Some((ref path, width, height, max_dim)) = self.oversized_image {
            // Show oversized image warning with Load Anyway button
            use crate::utils::style::{Colors, Spacing, TextSize};

            let canonical_path = path
                .canonicalize()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());

            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(
                    ((background_color[0] as u32) << 16) |
                    ((background_color[1] as u32) << 8) |
                    (background_color[2] as u32)
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(Spacing::lg())
                        .max_w(px(600.0))
                        .px(Spacing::xl())
                        .py(Spacing::xl())
                        .bg(rgba(0x50fa7b22))
                        .border_2()
                        .border_color(Colors::info())
                        .rounded(px(8.0))
                        .child(
                            div()
                                .text_size(TextSize::xl())
                                .text_color(Colors::info())
                                .font_weight(FontWeight::BOLD)
                                .child("⚠ Large Image Protection")
                        )
                        .child(
                            div()
                                .text_size(TextSize::md())
                                .text_color(Colors::text())
                                .text_align(gpui::TextAlign::Center)
                                .child(format!(
                                    "This image is {}×{} pixels, which exceeds the maximum dimension limit of {} pixels.",
                                    width, height, max_dim
                                ))
                        )
                        .child(
                            div()
                                .text_size(TextSize::sm())
                                .text_color(rgb(0xaaaaaa))
                                .text_align(gpui::TextAlign::Center)
                                .child("Loading very large images may cause slowdowns or high memory usage.")
                        )
                        .child(
                            div()
                                .text_size(TextSize::sm())
                                .text_color(rgb(0x888888))
                                .text_align(gpui::TextAlign::Center)
                                .px(Spacing::md())
                                .py(Spacing::xs())
                                .bg(rgb(0x2a2a2a))
                                .rounded(px(4.0))
                                .child(canonical_path)
                        )
                        .child(
                            div()
                                .text_size(TextSize::md())
                                .text_color(Colors::text())
                                .text_align(gpui::TextAlign::Center)
                                .mt(Spacing::md())
                                .child("To load this image:")
                        )
                        .child(
                            div()
                                .text_size(TextSize::sm())
                                .text_color(rgb(0xaaaaaa))
                                .text_align(gpui::TextAlign::Center)
                                .child("Open Settings (Cmd+,) > Performance > Maximum image dimension")
                        )
                        .child(
                            div()
                                .text_size(TextSize::sm())
                                .text_color(rgb(0xaaaaaa))
                                .text_align(gpui::TextAlign::Center)
                                .child(format!("and increase the limit above {} px", max_dim))
                        )
                )
                .into_any_element()
        } else if let Some(ref path) = self.no_images_path {
            // Show friendly notice when directory has no images (not an error)
            let display_path = path.display().to_string();
            let text_color = Colors::text_for_background(background_color);
            div()
                .size_full()
                .flex()
                .flex_col()
                .justify_center()
                .items_center()
                .gap(Spacing::lg())
                .child(
                    div()
                        .text_size(TextSize::xl())
                        .text_color(text_color)
                        .text_align(gpui::TextAlign::Center)
                        .child("The current directory does not contain any images."),
                )
                .child(
                    div()
                        .text_size(TextSize::xl())
                        .text_color(text_color)
                        .text_align(gpui::TextAlign::Center)
                        .child(display_path),
                )
                .child(
                    div()
                        .mt(Spacing::lg())
                        .px(Spacing::lg())
                        .py(Spacing::sm())
                        .bg(Colors::info())
                        .rounded(px(6.0))
                        .text_size(TextSize::md())
                        .text_color(rgb(0x1a1a1a))
                        .font_weight(FontWeight::MEDIUM)
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(0x6272a4)))
                        .on_mouse_down(MouseButton::Left, |_event, window, cx| {
                            window.dispatch_action(OpenFile.boxed_clone(), cx);
                        })
                        .child("Open Image"),
                )
                .into_any_element()
        } else if let Some(ref error) = self.error_message {
            // Show error message with full canonical path if available
            let full_message = if let Some(ref path) = self.error_path {
                let canonical_path = path
                    .canonicalize()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| path.display().to_string());
                format!("{}\n\nFull path: {}", error, canonical_path)
            } else {
                error.clone()
            };

            let text_color = Colors::text_for_background(background_color);
            div()
                .size_full()
                .child(cx.new(|_cx| ErrorDisplay::new(full_message).with_text_color(text_color)))
                .into_any_element()
        } else if let Some(ref loaded) = self.current_image {
            self.render_image(
                loaded,
                background_color,
                overlay_transparency,
                font_size_scale,
                show_zoom_indicator,
                cx,
            )
        } else {
            div().size_full().into_any_element()
        }
    }

    fn render_image<V>(
        &self,
        loaded: &LoadedImage,
        background_color: [u8; 3],
        overlay_transparency: u8,
        font_size_scale: f32,
        show_zoom_indicator: bool,
        cx: &mut Context<V>,
    ) -> AnyElement {
        let (width, height) = effective_image_size(loaded, self.lc_enabled);

        // Get the display path (handles animation frames and filters)
        let path =
            if let Some(ref anim_state) = self.image_state.animation {
                let frame_index = anim_state.current_frame;

                // Get current frame path
                if frame_index < loaded.frame_cache_paths.len() {
                    let cached_path = &loaded.frame_cache_paths[frame_index];
                    if !cached_path.as_os_str().is_empty() && cached_path.exists() {
                        cached_path.clone()
                    } else {
                        // Frame not cached, show error
                        return div()
                            .size_full()
                            .child(cx.new(|_cx| {
                                ErrorDisplay::new("Failed to load image frame".to_string())
                            }))
                            .into_any_element();
                    }
                } else {
                    // Invalid frame index
                    return div()
                        .size_full()
                        .child(cx.new(|_cx| ErrorDisplay::new("Invalid frame index".to_string())))
                        .into_any_element();
                }
            } else {
                // Static image priority: full SVG re-raster (no region) → rasterized → original.
                // The in-memory filtered image is handled below via `filtered_source`.
                let full_reraster = self
                    .svg_reraster_path
                    .as_ref()
                    .filter(|_| self.svg_reraster_region.is_none());
                full_reraster
                    .or(loaded.rasterized_path.as_ref())
                    .unwrap_or(&loaded.path)
                    .clone()
            };

        // Apply zoom to image dimensions
        let zoomed_width = (width as f32 * self.image_state.zoom) as u32;
        let zoomed_height = (height as f32 * self.image_state.zoom) as u32;

        // Get pan offset
        let (pan_x, pan_y) = self.image_state.pan;

        // Main image area with zoom indicator overlay
        let zoom_level = self.image_state.zoom;
        let is_fit = self.image_state.is_fit_to_window;

        // Priority: local-contrast output > B/C/G filtered output > file path.
        // Unique per-result ID ensures GPUI treats each fresh buffer as a new
        // texture rather than reusing a cached one. The `lc_enabled` flag
        // lets the `1` / `2` keys toggle LC off for A/B comparison without
        // discarding the processed buffer.
        let lc_candidate = if self.lc_enabled {
            loaded.lc_render.as_ref()
        } else {
            None
        };
        let (image_source, image_id): (gpui::ImageSource, ElementId) =
            if let Some(render_image) = lc_candidate {
                let id = ElementId::Name(format!("lc-{}", render_image.id.0).into());
                (gpui::ImageSource::Render(render_image.clone()), id)
            } else if let Some(ref render_image) = loaded.filtered_render {
                let id = ElementId::Name(format!("filtered-{}", render_image.id.0).into());
                (gpui::ImageSource::Render(render_image.clone()), id)
            } else {
                let id = ElementId::Name(format!("image-{}", path.display()).into());
                (gpui::ImageSource::from(path.clone()), id)
            };

        let mut container = div()
            .size_full()
            .bg(rgb(((background_color[0] as u32) << 16)
                | ((background_color[1] as u32) << 8)
                | (background_color[2] as u32)))
            .overflow_hidden()
            .relative()
            .child(
                img(image_source)
                    .id(image_id)
                    .w(px(zoomed_width as f32))
                    .h(px(zoomed_height as f32))
                    .absolute()
                    .left(px(pan_x))
                    .top(px(pan_y)),
            );

        // Overlay sharp viewport-only SVG re-raster on top of the base image
        if let (Some(rr_path), Some(region)) = (&self.svg_reraster_path, &self.svg_reraster_region)
        {
            if rr_path.exists() {
                let screen_x = region.svg_x * zoom_level + pan_x;
                let screen_y = region.svg_y * zoom_level + pan_y;
                let screen_w = region.svg_w * zoom_level;
                let screen_h = region.svg_h * zoom_level;
                container = container.child(
                    img(rr_path.clone())
                        .id(ElementId::Name(
                            format!("svg-reraster-{}", rr_path.display()).into(),
                        ))
                        .w(px(screen_w))
                        .h(px(screen_h))
                        .absolute()
                        .left(px(screen_x))
                        .top(px(screen_y)),
                );
            }
        }

        // Preload next frame for animations
        if let Some(ref anim_state) = self.image_state.animation {
            let next_frame_index = (anim_state.current_frame + 1) % anim_state.frame_count;
            if next_frame_index < loaded.frame_cache_paths.len() {
                let next_frame_path = &loaded.frame_cache_paths[next_frame_index];
                if !next_frame_path.as_os_str().is_empty() && next_frame_path.exists() {
                    container = container.child(
                        img(next_frame_path.clone())
                            .w(px(zoomed_width as f32))
                            .h(px(zoomed_height as f32))
                            .absolute()
                            .left(px(-10000.0))
                            .top(px(0.0))
                            .opacity(0.0),
                    );
                }
            }
        }

        // Preload pending SVG re-raster (GPU cache priming before swap)
        if let Some(ref pending_path) = self.pending_svg_reraster_path {
            if pending_path.exists() {
                let preload_id = ElementId::Name(
                    format!("pending-svg-reraster-{}", pending_path.display()).into(),
                );
                container = container.child(
                    img(pending_path.clone())
                        .id(preload_id)
                        .w(px(zoomed_width as f32))
                        .h(px(zoomed_height as f32))
                        .absolute()
                        .left(px(-10000.0))
                        .top(px(0.0))
                        .opacity(0.0),
                );
            }
        }

        // Preload next/previous images in navigation list
        for preload_path in &self.preload_paths {
            if preload_path.exists() {
                let preload_id =
                    ElementId::Name(format!("preload-{}", preload_path.display()).into());
                container = container.child(
                    img(preload_path.clone())
                        .id(preload_id)
                        .w(px(zoomed_width as f32))
                        .h(px(zoomed_height as f32))
                        .absolute()
                        .left(px(-10000.0))
                        .top(px(0.0))
                        .opacity(0.0),
                );
            }
        }

        if show_zoom_indicator {
            container = container.child(cx.new(|_cx| {
                ZoomIndicator::new(
                    zoom_level,
                    is_fit,
                    Some((width, height)),
                    overlay_transparency,
                    font_size_scale,
                )
            }));
        }

        // Add processing indicator if filters are being processed
        if self.is_processing_filters {
            container = container.child(cx.new(|_cx| {
                ProcessingIndicator::new(
                    "Processing filters...",
                    overlay_transparency,
                    font_size_scale,
                )
            }));
        }

        // Add animation indicator if this is an animated image
        if let Some(ref anim_state) = self.image_state.animation {
            container = container.child(cx.new(|_cx| {
                AnimationIndicator::new(
                    anim_state.current_frame,
                    anim_state.frame_count,
                    anim_state.is_playing,
                    overlay_transparency,
                    font_size_scale,
                )
            }));
        }

        container.into_any_element()
    }
}

impl Render for ImageViewer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .focus(|s| s)
            .size_full()
            .bg(Colors::background())
            .child(self.render_view([0x1e, 0x1e, 0x1e], 204, 1.0, true, cx))
            .into_any_element()
    }
}
