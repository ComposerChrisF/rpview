//! Zoom-related constants and utilities

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

/// Mouse wheel zoom step (smaller for smoother scrolling)
#[allow(dead_code)]
pub const ZOOM_STEP_WHEEL: f32 = 1.1;

/// Clamp zoom level to valid range
pub fn clamp_zoom(zoom: f32) -> f32 {
    zoom.clamp(MIN_ZOOM, MAX_ZOOM)
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

    // Test constants for boundary values
    const ZOOM_BELOW_MIN: f32 = 0.05;
    const ZOOM_ABOVE_MAX: f32 = 25.0;
    const TOLERANCE: f32 = 0.001;

    #[test]
    fn test_clamp_zoom() {
        // Arrange & Act & Assert
        assert_eq!(clamp_zoom(ZOOM_BELOW_MIN), MIN_ZOOM);
        assert_eq!(clamp_zoom(1.0), 1.0);
        assert_eq!(clamp_zoom(ZOOM_ABOVE_MAX), MAX_ZOOM);
    }

    #[test]
    fn test_clamp_zoom_at_boundaries() {
        // Arrange & Act & Assert - exactly at boundaries
        assert_eq!(clamp_zoom(MIN_ZOOM), MIN_ZOOM);
        assert_eq!(clamp_zoom(MAX_ZOOM), MAX_ZOOM);
    }

    #[test]
    fn test_clamp_zoom_negative() {
        // Arrange & Act & Assert - negative values should clamp to MIN
        assert_eq!(clamp_zoom(-1.0), MIN_ZOOM);
        assert_eq!(clamp_zoom(-100.0), MIN_ZOOM);
    }

    #[test]
    fn test_clamp_zoom_just_inside_boundaries() {
        // Arrange
        let just_above_min = MIN_ZOOM + 0.01;
        let just_below_max = MAX_ZOOM - 0.01;

        // Act & Assert - values just inside boundaries should pass through
        assert_eq!(clamp_zoom(just_above_min), just_above_min);
        assert_eq!(clamp_zoom(just_below_max), just_below_max);
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
    fn test_calculate_fit_to_window_zero_image_dimensions() {
        // Arrange & Act & Assert - zero dimensions should return 1.0
        assert_eq!(calculate_fit_to_window(0, 100, 500.0, 500.0), 1.0);
        assert_eq!(calculate_fit_to_window(100, 0, 500.0, 500.0), 1.0);
        assert_eq!(calculate_fit_to_window(0, 0, 500.0, 500.0), 1.0);
    }

    #[test]
    fn test_calculate_fit_to_window_zero_viewport() {
        // Arrange & Act & Assert - zero viewport should return 1.0
        assert_eq!(calculate_fit_to_window(100, 100, 0.0, 500.0), 1.0);
        assert_eq!(calculate_fit_to_window(100, 100, 500.0, 0.0), 1.0);
        assert_eq!(calculate_fit_to_window(100, 100, 0.0, 0.0), 1.0);
    }

    #[test]
    fn test_calculate_fit_to_window_negative_viewport() {
        // Arrange & Act & Assert - negative viewport should return 1.0
        assert_eq!(calculate_fit_to_window(100, 100, -500.0, 500.0), 1.0);
        assert_eq!(calculate_fit_to_window(100, 100, 500.0, -500.0), 1.0);
    }

    #[test]
    fn test_calculate_fit_to_window_respects_zoom_limits() {
        // Arrange - very small image in large viewport would exceed MAX_ZOOM
        // 10x10 image in 300x300 viewport = 30x zoom, but MAX is 20
        let result = calculate_fit_to_window(10, 10, 300.0, 300.0);

        // Assert - should be clamped to MAX_ZOOM
        assert_eq!(result, MAX_ZOOM);
    }

    #[test]
    fn test_calculate_fit_to_window_very_large_image() {
        // Arrange - very large image that would scale below MIN_ZOOM
        // 100000x100000 image in 100x100 viewport = 0.001 zoom
        let result = calculate_fit_to_window(100000, 100000, 100.0, 100.0);

        // Assert - should be clamped to MIN_ZOOM
        assert_eq!(result, MIN_ZOOM);
    }

    #[test]
    fn test_zoom_in_out() {
        let zoom = 1.0;
        let zoomed_in = zoom_in(zoom, ZOOM_STEP);
        assert_eq!(zoomed_in, 1.2);

        let zoomed_out = zoom_out(zoomed_in, ZOOM_STEP);
        assert!((zoomed_out - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn test_zoom_in_respects_max() {
        // Arrange - start near max zoom
        let near_max = MAX_ZOOM - 1.0;

        // Act
        let result = zoom_in(near_max, ZOOM_STEP);

        // Assert - should be clamped to MAX_ZOOM
        assert_eq!(result, MAX_ZOOM);
    }

    #[test]
    fn test_zoom_out_respects_min() {
        // Arrange - start at min zoom
        let at_min = MIN_ZOOM;

        // Act - zooming out from MIN should stay at MIN
        let result = zoom_out(at_min, ZOOM_STEP);

        // Assert - should be clamped to MIN_ZOOM
        assert_eq!(result, MIN_ZOOM);
    }

    #[test]
    fn test_zoom_in_with_different_steps() {
        // Arrange
        let base_zoom = 1.0;

        // Act & Assert - verify different step sizes work correctly
        assert_eq!(zoom_in(base_zoom, ZOOM_STEP_FAST), 1.5);
        assert_eq!(zoom_in(base_zoom, ZOOM_STEP_SLOW), 1.05);
    }

    #[test]
    fn test_zoom_out_with_different_steps() {
        // Arrange
        let base_zoom = 1.5;

        // Act
        let result = zoom_out(base_zoom, ZOOM_STEP_FAST);

        // Assert - 1.5 / 1.5 = 1.0
        assert!((result - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn test_zoom_incremental_step() {
        // Arrange
        let base_zoom = 1.0;

        // Act - incremental zoom is additive (1%)
        let result = zoom_in(base_zoom, 1.0 + ZOOM_STEP_INCREMENTAL);

        // Assert - should be 1.01
        assert!((result - 1.01).abs() < TOLERANCE);
    }

    #[test]
    fn test_format_zoom_percentage() {
        assert_eq!(format_zoom_percentage(1.0), "100%");
        assert_eq!(format_zoom_percentage(0.5), "50%");
        assert_eq!(format_zoom_percentage(2.0), "200%");
    }

    #[test]
    fn test_format_zoom_percentage_boundaries() {
        // Arrange & Act & Assert - test at zoom limits
        assert_eq!(format_zoom_percentage(MIN_ZOOM), "10%");
        assert_eq!(format_zoom_percentage(MAX_ZOOM), "2000%");
    }

    #[test]
    fn test_format_zoom_percentage_fractional() {
        // Arrange & Act & Assert - fractional values should round
        assert_eq!(format_zoom_percentage(0.123), "12%");
        assert_eq!(format_zoom_percentage(0.999), "100%");
        assert_eq!(format_zoom_percentage(1.234), "123%");
    }
}
