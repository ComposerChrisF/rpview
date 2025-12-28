/// Zoom-related constants and utilities

/// Minimum zoom level (10%)
pub const MIN_ZOOM: f32 = 0.1;

/// Maximum zoom level (2000%)
pub const MAX_ZOOM: f32 = 20.0;

/// Default zoom step multiplier
pub const ZOOM_STEP: f32 = 1.2;

/// Fast zoom step multiplier (with Shift)
pub const ZOOM_STEP_FAST: f32 = 1.5;

/// Slow zoom step multiplier (with Ctrl/Cmd)
pub const ZOOM_STEP_SLOW: f32 = 1.05;

/// Incremental zoom step (with Shift+Ctrl/Cmd) - 1% per step
pub const ZOOM_STEP_INCREMENTAL: f32 = 0.01;

/// Clamp zoom level to valid range
pub fn clamp_zoom(zoom: f32) -> f32 {
    zoom.max(MIN_ZOOM).min(MAX_ZOOM)
}

/// Calculate zoom level to fit image in viewport
/// Returns the zoom factor needed to fit the image dimensions within the viewport
pub fn calculate_fit_to_window(
    image_width: u32,
    image_height: u32,
    viewport_width: f32,
    viewport_height: f32,
) -> f32 {
    if image_width == 0 || image_height == 0 || viewport_width <= 0.0 || viewport_height <= 0.0 {
        return 1.0;
    }

    let width_ratio = viewport_width / image_width as f32;
    let height_ratio = viewport_height / image_height as f32;

    // Use the smaller ratio to ensure the image fits entirely
    clamp_zoom(width_ratio.min(height_ratio))
}

/// Zoom in by the given step
pub fn zoom_in(current_zoom: f32, step: f32) -> f32 {
    clamp_zoom(current_zoom * step)
}

/// Zoom out by the given step
pub fn zoom_out(current_zoom: f32, step: f32) -> f32 {
    clamp_zoom(current_zoom / step)
}

/// Format zoom level as percentage string
pub fn format_zoom_percentage(zoom: f32) -> String {
    format!("{:.0}%", zoom * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_zoom() {
        assert_eq!(clamp_zoom(0.05), MIN_ZOOM);
        assert_eq!(clamp_zoom(1.0), 1.0);
        assert_eq!(clamp_zoom(25.0), MAX_ZOOM);
    }

    #[test]
    fn test_calculate_fit_to_window() {
        // Image larger than viewport - should scale down
        assert_eq!(calculate_fit_to_window(1000, 1000, 500.0, 500.0), 0.5);
        
        // Image smaller than viewport - should scale up
        assert_eq!(calculate_fit_to_window(500, 500, 1000.0, 1000.0), 2.0);
        
        // Image fits exactly
        assert_eq!(calculate_fit_to_window(800, 600, 800.0, 600.0), 1.0);
        
        // Wide image
        assert_eq!(calculate_fit_to_window(1600, 800, 800.0, 600.0), 0.5);
        
        // Tall image
        assert_eq!(calculate_fit_to_window(800, 1600, 800.0, 600.0), 0.375);
    }

    #[test]
    fn test_zoom_in_out() {
        let zoom = 1.0;
        let zoomed_in = zoom_in(zoom, ZOOM_STEP);
        assert_eq!(zoomed_in, 1.2);
        
        let zoomed_out = zoom_out(zoomed_in, ZOOM_STEP);
        assert!((zoomed_out - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_format_zoom_percentage() {
        assert_eq!(format_zoom_percentage(1.0), "100%");
        assert_eq!(format_zoom_percentage(0.5), "50%");
        assert_eq!(format_zoom_percentage(2.0), "200%");
    }
}
