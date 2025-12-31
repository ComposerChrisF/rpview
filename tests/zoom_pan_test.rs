use rpview_gpui::utils::zoom::*;

#[test]
fn test_zoom_constants() {
    assert_eq!(MIN_ZOOM, 0.1);
    assert_eq!(MAX_ZOOM, 20.0);
    assert_eq!(ZOOM_STEP, 1.2);
    assert_eq!(ZOOM_STEP_FAST, 1.5);
    assert_eq!(ZOOM_STEP_SLOW, 1.05);
    assert_eq!(ZOOM_STEP_INCREMENTAL, 0.01);
    assert_eq!(ZOOM_STEP_WHEEL, 1.1);
}

#[test]
fn test_clamp_zoom_within_range() {
    assert_eq!(clamp_zoom(1.0), 1.0);
    assert_eq!(clamp_zoom(5.0), 5.0);
    assert_eq!(clamp_zoom(0.5), 0.5);
}

#[test]
fn test_clamp_zoom_below_min() {
    assert_eq!(clamp_zoom(0.05), MIN_ZOOM);
    assert_eq!(clamp_zoom(0.01), MIN_ZOOM);
    assert_eq!(clamp_zoom(-1.0), MIN_ZOOM);
}

#[test]
fn test_clamp_zoom_above_max() {
    assert_eq!(clamp_zoom(25.0), MAX_ZOOM);
    assert_eq!(clamp_zoom(100.0), MAX_ZOOM);
}

#[test]
fn test_calculate_fit_to_window_scale_down() {
    // Image larger than viewport - should scale down
    let zoom = calculate_fit_to_window(2000, 1000, 1000.0, 500.0);
    assert_eq!(zoom, 0.5);
}

#[test]
fn test_calculate_fit_to_window_scale_up() {
    // Image smaller than viewport - should scale up
    let zoom = calculate_fit_to_window(500, 500, 1000.0, 1000.0);
    assert_eq!(zoom, 2.0);
}

#[test]
fn test_calculate_fit_to_window_exact_fit() {
    // Image exactly fits viewport
    let zoom = calculate_fit_to_window(800, 600, 800.0, 600.0);
    assert_eq!(zoom, 1.0);
}

#[test]
fn test_calculate_fit_to_window_wide_image() {
    // Very wide image - width is limiting factor
    let zoom = calculate_fit_to_window(2000, 500, 1000.0, 1000.0);
    assert_eq!(zoom, 0.5); // Limited by width
}

#[test]
fn test_calculate_fit_to_window_tall_image() {
    // Very tall image - height is limiting factor
    let zoom = calculate_fit_to_window(500, 2000, 1000.0, 1000.0);
    assert_eq!(zoom, 0.5); // Limited by height
}

#[test]
fn test_calculate_fit_to_window_zero_dimensions() {
    // Zero image dimensions should return 1.0
    assert_eq!(calculate_fit_to_window(0, 500, 1000.0, 500.0), 1.0);
    assert_eq!(calculate_fit_to_window(500, 0, 1000.0, 500.0), 1.0);
    assert_eq!(calculate_fit_to_window(0, 0, 1000.0, 500.0), 1.0);
}

#[test]
fn test_calculate_fit_to_window_zero_viewport() {
    // Zero viewport dimensions should return 1.0
    assert_eq!(calculate_fit_to_window(500, 500, 0.0, 500.0), 1.0);
    assert_eq!(calculate_fit_to_window(500, 500, 1000.0, 0.0), 1.0);
    assert_eq!(calculate_fit_to_window(500, 500, 0.0, 0.0), 1.0);
}

#[test]
fn test_calculate_fit_to_window_respects_max_zoom() {
    // Very small image in huge viewport - should clamp to MAX_ZOOM
    let zoom = calculate_fit_to_window(10, 10, 10000.0, 10000.0);
    assert_eq!(zoom, MAX_ZOOM);
}

#[test]
fn test_zoom_in_normal() {
    let current = 1.0;
    let zoomed = zoom_in(current, ZOOM_STEP);
    assert_eq!(zoomed, 1.2);
}

#[test]
fn test_zoom_in_fast() {
    let current = 1.0;
    let zoomed = zoom_in(current, ZOOM_STEP_FAST);
    assert_eq!(zoomed, 1.5);
}

#[test]
fn test_zoom_in_slow() {
    let current = 1.0;
    let zoomed = zoom_in(current, ZOOM_STEP_SLOW);
    assert_eq!(zoomed, 1.05);
}

#[test]
fn test_zoom_in_incremental() {
    let current = 1.0;
    let zoomed = zoom_in(current, 1.0 + ZOOM_STEP_INCREMENTAL);
    assert!((zoomed - 1.01).abs() < 0.0001);
}

#[test]
fn test_zoom_in_respects_max() {
    let current = 19.0;
    let zoomed = zoom_in(current, ZOOM_STEP);
    assert_eq!(zoomed, MAX_ZOOM);
}

#[test]
fn test_zoom_out_normal() {
    let current = 1.2;
    let zoomed = zoom_out(current, ZOOM_STEP);
    assert!((zoomed - 1.0).abs() < 0.001);
}

#[test]
fn test_zoom_out_fast() {
    let current = 1.5;
    let zoomed = zoom_out(current, ZOOM_STEP_FAST);
    assert!((zoomed - 1.0).abs() < 0.001);
}

#[test]
fn test_zoom_out_slow() {
    let current = 1.05;
    let zoomed = zoom_out(current, ZOOM_STEP_SLOW);
    assert!((zoomed - 1.0).abs() < 0.001);
}

#[test]
fn test_zoom_out_respects_min() {
    let current = 0.11; // Close to MIN_ZOOM
    let zoomed = zoom_out(current, ZOOM_STEP);
    assert_eq!(zoomed, MIN_ZOOM);
}

#[test]
fn test_zoom_in_out_reversible() {
    let original = 1.5;
    let zoomed_in = zoom_in(original, ZOOM_STEP);
    let zoomed_out = zoom_out(zoomed_in, ZOOM_STEP);
    assert!((zoomed_out - original).abs() < 0.001);
}

#[test]
fn test_format_zoom_percentage_100() {
    assert_eq!(format_zoom_percentage(1.0), "100%");
}

#[test]
fn test_format_zoom_percentage_50() {
    assert_eq!(format_zoom_percentage(0.5), "50%");
}

#[test]
fn test_format_zoom_percentage_200() {
    assert_eq!(format_zoom_percentage(2.0), "200%");
}

#[test]
fn test_format_zoom_percentage_10() {
    assert_eq!(format_zoom_percentage(0.1), "10%");
}

#[test]
fn test_format_zoom_percentage_2000() {
    assert_eq!(format_zoom_percentage(20.0), "2000%");
}

#[test]
fn test_format_zoom_percentage_fractional() {
    // Should round to nearest integer
    assert_eq!(format_zoom_percentage(1.234), "123%");
    assert_eq!(format_zoom_percentage(0.567), "57%");
}

#[test]
fn test_multiple_zoom_in_steps() {
    let mut zoom = 1.0;
    
    // 5 zoom in steps
    for _ in 0..5 {
        zoom = zoom_in(zoom, ZOOM_STEP);
    }
    
    // 1.0 * 1.2^5 ≈ 2.488
    assert!((zoom - 2.488).abs() < 0.01);
}

#[test]
fn test_multiple_zoom_out_steps() {
    let mut zoom = 2.0;
    
    // 5 zoom out steps
    for _ in 0..5 {
        zoom = zoom_out(zoom, ZOOM_STEP);
    }
    
    // 2.0 / 1.2^5 ≈ 0.804
    assert!((zoom - 0.804).abs() < 0.01);
}

#[test]
fn test_zoom_at_boundaries() {
    // At MIN_ZOOM, zooming out should stay at MIN_ZOOM
    let zoom = zoom_out(MIN_ZOOM, ZOOM_STEP);
    assert_eq!(zoom, MIN_ZOOM);
    
    // At MAX_ZOOM, zooming in should stay at MAX_ZOOM
    let zoom = zoom_in(MAX_ZOOM, ZOOM_STEP);
    assert_eq!(zoom, MAX_ZOOM);
}

#[test]
fn test_wheel_zoom_smaller_steps() {
    let current = 1.0;
    
    let wheel_zoomed = zoom_in(current, ZOOM_STEP_WHEEL);
    let keyboard_zoomed = zoom_in(current, ZOOM_STEP);
    
    // Wheel zoom should be smaller than keyboard zoom
    assert!(wheel_zoomed < keyboard_zoomed);
    assert_eq!(wheel_zoomed, 1.1);
}

#[test]
fn test_fit_to_window_portrait_image() {
    // Portrait image (taller than wide)
    let zoom = calculate_fit_to_window(600, 800, 1000.0, 1000.0);
    // Should be limited by height: 1000 / 800 = 1.25
    assert_eq!(zoom, 1.25);
}

#[test]
fn test_fit_to_window_landscape_image() {
    // Landscape image (wider than tall)
    let zoom = calculate_fit_to_window(800, 600, 1000.0, 1000.0);
    // Should be limited by width: 1000 / 800 = 1.25
    assert_eq!(zoom, 1.25);
}

#[test]
fn test_fit_to_window_square_image_portrait_viewport() {
    // Square image in portrait viewport
    let zoom = calculate_fit_to_window(500, 500, 600.0, 800.0);
    // Should be limited by width: 600 / 500 = 1.2
    assert_eq!(zoom, 1.2);
}

#[test]
fn test_fit_to_window_square_image_landscape_viewport() {
    // Square image in landscape viewport
    let zoom = calculate_fit_to_window(500, 500, 800.0, 600.0);
    // Should be limited by height: 600 / 500 = 1.2
    assert_eq!(zoom, 1.2);
}
