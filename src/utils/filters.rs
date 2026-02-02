use image::{DynamicImage, ImageBuffer, Rgba};

/// Apply brightness adjustment to an image
/// brightness: -100.0 to +100.0
#[allow(dead_code)]
pub fn apply_brightness(img: &DynamicImage, brightness: f32) -> DynamicImage {
    let brightness = brightness.clamp(-100.0, 100.0);
    if brightness.abs() < 0.001 {
        return img.clone();
    }

    let adjustment = (brightness * 2.55) as i32; // Map -100..100 to -255..255

    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();

    let mut output = ImageBuffer::new(width, height);

    for (x, y, pixel) in rgba_img.enumerate_pixels() {
        let r = (pixel[0] as i32 + adjustment).clamp(0, 255) as u8;
        let g = (pixel[1] as i32 + adjustment).clamp(0, 255) as u8;
        let b = (pixel[2] as i32 + adjustment).clamp(0, 255) as u8;
        let a = pixel[3];

        output.put_pixel(x, y, Rgba([r, g, b, a]));
    }

    DynamicImage::ImageRgba8(output)
}

/// Apply contrast adjustment to an image
/// contrast: -100.0 to +100.0
#[allow(dead_code)]
pub fn apply_contrast(img: &DynamicImage, contrast: f32) -> DynamicImage {
    let contrast = contrast.clamp(-100.0, 100.0);
    if contrast.abs() < 0.001 {
        return img.clone();
    }

    // Convert contrast value to a factor
    // Formula: factor = (259 * (contrast + 255)) / (255 * (259 - contrast))
    // Simplified for our range: factor ranges from ~0.1 (very low) to ~10.0 (very high)
    let factor = if contrast > 0.0 {
        1.0 + (contrast / 100.0) * 2.0 // 1.0 to 3.0
    } else {
        1.0 + (contrast / 100.0) * 0.9 // 0.1 to 1.0
    };

    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();

    let mut output = ImageBuffer::new(width, height);

    for (x, y, pixel) in rgba_img.enumerate_pixels() {
        let r = apply_contrast_to_channel(pixel[0], factor);
        let g = apply_contrast_to_channel(pixel[1], factor);
        let b = apply_contrast_to_channel(pixel[2], factor);
        let a = pixel[3];

        output.put_pixel(x, y, Rgba([r, g, b, a]));
    }

    DynamicImage::ImageRgba8(output)
}

/// Apply contrast to a single channel
#[allow(dead_code)]
fn apply_contrast_to_channel(value: u8, factor: f32) -> u8 {
    let normalized = (value as f32) / 255.0;
    let adjusted = ((normalized - 0.5) * factor + 0.5) * 255.0;
    adjusted.clamp(0.0, 255.0) as u8
}

/// Apply gamma correction to an image
/// gamma: 0.1 to 10.0 (1.0 = no change, <1.0 = darker, >1.0 = brighter)
#[allow(dead_code)]
pub fn apply_gamma(img: &DynamicImage, gamma: f32) -> DynamicImage {
    let gamma = gamma.clamp(0.1, 10.0);
    if (gamma - 1.0).abs() < 0.001 {
        return img.clone();
    }

    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();

    let mut output = ImageBuffer::new(width, height);

    // Pre-calculate gamma lookup table for performance
    let mut gamma_lut = [0u8; 256];
    for (i, lut_entry) in gamma_lut.iter_mut().enumerate() {
        let normalized = (i as f32) / 255.0;
        let corrected = normalized.powf(1.0 / gamma);
        *lut_entry = (corrected * 255.0).clamp(0.0, 255.0) as u8;
    }

    for (x, y, pixel) in rgba_img.enumerate_pixels() {
        let r = gamma_lut[pixel[0] as usize];
        let g = gamma_lut[pixel[1] as usize];
        let b = gamma_lut[pixel[2] as usize];
        let a = pixel[3];

        output.put_pixel(x, y, Rgba([r, g, b, a]));
    }

    DynamicImage::ImageRgba8(output)
}

/// Apply all filters to an image in a single pass using a combined LUT
/// This is more efficient than applying filters sequentially, as it:
/// 1. Only iterates through pixels once instead of up to 3 times
/// 2. Only allocates one output buffer instead of up to 3
/// 3. Pre-computes all transformations into a single 256-entry lookup table
pub fn apply_filters(
    img: &DynamicImage,
    brightness: f32,
    contrast: f32,
    gamma: f32,
) -> DynamicImage {
    // Short-circuit if no filters are applied
    let has_brightness = brightness.abs() >= 0.001;
    let has_contrast = contrast.abs() >= 0.001;
    let has_gamma = (gamma - 1.0).abs() >= 0.001;

    if !has_brightness && !has_contrast && !has_gamma {
        return img.clone();
    }

    // Clamp input values
    let brightness = brightness.clamp(-100.0, 100.0);
    let contrast = contrast.clamp(-100.0, 100.0);
    let gamma = gamma.clamp(0.1, 10.0);

    // Pre-calculate combined lookup table for all filters
    // This applies brightness -> contrast -> gamma in order
    let mut lut = [0u8; 256];

    // Pre-calculate contrast factor
    let contrast_factor = if contrast > 0.0 {
        1.0 + (contrast / 100.0) * 2.0 // 1.0 to 3.0
    } else {
        1.0 + (contrast / 100.0) * 0.9 // 0.1 to 1.0
    };

    // Brightness adjustment (maps -100..100 to -255..255)
    let brightness_adjustment = brightness * 2.55;

    for (i, lut_entry) in lut.iter_mut().enumerate() {
        let mut value = i as f32;

        // Step 1: Apply brightness
        if has_brightness {
            value = (value + brightness_adjustment).clamp(0.0, 255.0);
        }

        // Step 2: Apply contrast
        if has_contrast {
            let normalized = value / 255.0;
            value = ((normalized - 0.5) * contrast_factor + 0.5) * 255.0;
            value = value.clamp(0.0, 255.0);
        }

        // Step 3: Apply gamma
        if has_gamma {
            let normalized = value / 255.0;
            let corrected = normalized.powf(1.0 / gamma);
            value = corrected * 255.0;
        }

        *lut_entry = value.clamp(0.0, 255.0) as u8;
    }

    // Apply combined LUT in a single pass
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let mut output = ImageBuffer::new(width, height);

    for (x, y, pixel) in rgba_img.enumerate_pixels() {
        let r = lut[pixel[0] as usize];
        let g = lut[pixel[1] as usize];
        let b = lut[pixel[2] as usize];
        let a = pixel[3]; // Alpha preserved

        output.put_pixel(x, y, Rgba([r, g, b, a]));
    }

    DynamicImage::ImageRgba8(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    // Test constants
    const MID_GRAY: u8 = 128;
    const WHITE: u8 = 255;
    const BLACK: u8 = 0;
    const DEFAULT_GAMMA: f32 = 1.0;
    const MIN_GAMMA: f32 = 0.1;
    const MAX_GAMMA: f32 = 10.0;

    /// Helper function to create a 1x1 test image with a specific color
    fn create_test_image(r: u8, g: u8, b: u8, a: u8) -> DynamicImage {
        DynamicImage::ImageRgba8(ImageBuffer::from_pixel(1, 1, Rgba([r, g, b, a])))
    }

    #[test]
    fn test_apply_contrast_to_channel() {
        // Test with factor 1.0 (no change)
        assert_eq!(apply_contrast_to_channel(MID_GRAY, 1.0), MID_GRAY);

        // Test with factor > 1.0 (increase contrast)
        assert!(apply_contrast_to_channel(200, 2.0) > 200);
        assert!(apply_contrast_to_channel(50, 2.0) < 50);

        // Test with factor < 1.0 (decrease contrast)
        let result = apply_contrast_to_channel(200, 0.5);
        assert!(result < 200 && result > MID_GRAY);
    }

    #[test]
    fn test_apply_contrast_to_channel_edge_values() {
        // Arrange & Act & Assert - black and white should clamp correctly
        assert_eq!(apply_contrast_to_channel(BLACK, 2.0), BLACK);
        assert_eq!(apply_contrast_to_channel(WHITE, 2.0), WHITE);

        // Mid-gray (128) should stay close to mid-gray regardless of factor
        // Note: Due to floating point math, 128 may become 127 (128 is not exactly 0.5 * 255)
        let mid_gray_low = apply_contrast_to_channel(MID_GRAY, 0.5);
        let mid_gray_high = apply_contrast_to_channel(MID_GRAY, 2.0);
        // Allow 1 unit of tolerance due to rounding
        assert!((mid_gray_low as i32 - MID_GRAY as i32).abs() <= 1);
        assert!((mid_gray_high as i32 - MID_GRAY as i32).abs() <= 1);
    }

    #[test]
    fn test_brightness_clamps() {
        // Arrange
        let img = create_test_image(MID_GRAY, MID_GRAY, MID_GRAY, WHITE);

        // Act - test extreme brightness values are clamped
        let bright = apply_brightness(&img, 150.0); // Should clamp to 100
        let bright_rgba = bright.to_rgba8();
        let pixel = bright_rgba.get_pixel(0, 0);

        // Assert
        assert_eq!(pixel[0], WHITE); // 128 + 255 = 383, clamped to 255

        // Act
        let dark = apply_brightness(&img, -150.0); // Should clamp to -100
        let dark_rgba = dark.to_rgba8();
        let pixel = dark_rgba.get_pixel(0, 0);

        // Assert
        assert_eq!(pixel[0], BLACK); // 128 - 255 = -127, clamped to 0
    }

    #[test]
    fn test_brightness_zero_no_change() {
        // Arrange
        let img = create_test_image(100, 150, 200, WHITE);

        // Act
        let result = apply_brightness(&img, 0.0);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - image should be unchanged
        assert_eq!(pixel[0], 100);
        assert_eq!(pixel[1], 150);
        assert_eq!(pixel[2], 200);
        assert_eq!(pixel[3], WHITE);
    }

    #[test]
    fn test_brightness_preserves_alpha() {
        // Arrange
        let alpha_value = 128;
        let img = create_test_image(100, 100, 100, alpha_value);

        // Act
        let result = apply_brightness(&img, 50.0);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - alpha should be preserved
        assert_eq!(pixel[3], alpha_value);
    }

    #[test]
    fn test_contrast_zero_no_change() {
        // Arrange
        let img = create_test_image(100, 150, 200, WHITE);

        // Act
        let result = apply_contrast(&img, 0.0);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - image should be unchanged
        assert_eq!(pixel[0], 100);
        assert_eq!(pixel[1], 150);
        assert_eq!(pixel[2], 200);
    }

    #[test]
    fn test_contrast_preserves_alpha() {
        // Arrange
        let alpha_value = 64;
        let img = create_test_image(100, 100, 100, alpha_value);

        // Act
        let result = apply_contrast(&img, 50.0);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - alpha should be preserved
        assert_eq!(pixel[3], alpha_value);
    }

    #[test]
    fn test_gamma_one_no_change() {
        // Arrange
        let img = create_test_image(100, 150, 200, WHITE);

        // Act
        let result = apply_gamma(&img, DEFAULT_GAMMA);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - image should be unchanged
        assert_eq!(pixel[0], 100);
        assert_eq!(pixel[1], 150);
        assert_eq!(pixel[2], 200);
    }

    #[test]
    fn test_gamma_preserves_black_and_white() {
        // Arrange - image with black and white pixels
        let mut img_buffer = ImageBuffer::new(2, 1);
        img_buffer.put_pixel(0, 0, Rgba([BLACK, BLACK, BLACK, WHITE]));
        img_buffer.put_pixel(1, 0, Rgba([WHITE, WHITE, WHITE, WHITE]));
        let img = DynamicImage::ImageRgba8(img_buffer);

        // Act
        let result = apply_gamma(&img, 2.2);
        let result_rgba = result.to_rgba8();

        // Assert - black stays black, white stays white
        let black_pixel = result_rgba.get_pixel(0, 0);
        let white_pixel = result_rgba.get_pixel(1, 0);

        assert_eq!(black_pixel[0], BLACK);
        assert_eq!(white_pixel[0], WHITE);
    }

    #[test]
    fn test_gamma_greater_than_one_brightens() {
        // Arrange
        let img = create_test_image(MID_GRAY, MID_GRAY, MID_GRAY, WHITE);

        // Act - gamma > 1 should brighten midtones
        let result = apply_gamma(&img, 2.0);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - pixel should be brighter
        assert!(pixel[0] > MID_GRAY);
    }

    #[test]
    fn test_gamma_less_than_one_darkens() {
        // Arrange
        let img = create_test_image(MID_GRAY, MID_GRAY, MID_GRAY, WHITE);

        // Act - gamma < 1 should darken midtones
        let result = apply_gamma(&img, 0.5);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - pixel should be darker
        assert!(pixel[0] < MID_GRAY);
    }

    #[test]
    fn test_gamma_clamps_input() {
        // Arrange
        let img = create_test_image(MID_GRAY, MID_GRAY, MID_GRAY, WHITE);

        // Act - gamma below MIN should clamp
        let result_low = apply_gamma(&img, 0.01);
        let result_min = apply_gamma(&img, MIN_GAMMA);

        // Assert - both should produce same result (clamped to MIN)
        let low_rgba = result_low.to_rgba8();
        let min_rgba = result_min.to_rgba8();
        assert_eq!(low_rgba.get_pixel(0, 0), min_rgba.get_pixel(0, 0));

        // Act - gamma above MAX should clamp
        let result_high = apply_gamma(&img, 20.0);
        let result_max = apply_gamma(&img, MAX_GAMMA);

        // Assert - both should produce same result (clamped to MAX)
        let high_rgba = result_high.to_rgba8();
        let max_rgba = result_max.to_rgba8();
        assert_eq!(high_rgba.get_pixel(0, 0), max_rgba.get_pixel(0, 0));
    }

    #[test]
    fn test_gamma_preserves_alpha() {
        // Arrange
        let alpha_value = 100;
        let img = create_test_image(MID_GRAY, MID_GRAY, MID_GRAY, alpha_value);

        // Act
        let result = apply_gamma(&img, 2.0);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - alpha should be preserved
        assert_eq!(pixel[3], alpha_value);
    }

    #[test]
    fn test_gamma_lut_correctness() {
        // Arrange - test specific gamma values and expected outputs
        let img = create_test_image(MID_GRAY, MID_GRAY, MID_GRAY, WHITE);

        // Act - gamma = 2.2 (standard sRGB gamma)
        let result = apply_gamma(&img, 2.2);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - expected value for gamma 2.2 on mid-gray
        // Formula: 255 * ((128/255)^(1/2.2)) = 255 * 0.7297 = 186
        let expected = 186;
        assert!(
            (pixel[0] as i32 - expected).abs() <= 1,
            "Expected ~{}, got {}",
            expected,
            pixel[0]
        );
    }

    #[test]
    fn test_apply_filters_all_default_no_change() {
        // Arrange
        let img = create_test_image(100, 150, 200, WHITE);

        // Act - all default values should not modify image
        let result = apply_filters(&img, 0.0, 0.0, DEFAULT_GAMMA);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert
        assert_eq!(pixel[0], 100);
        assert_eq!(pixel[1], 150);
        assert_eq!(pixel[2], 200);
    }

    #[test]
    fn test_apply_filters_order_brightness_contrast_gamma() {
        // Arrange
        let img = create_test_image(MID_GRAY, MID_GRAY, MID_GRAY, WHITE);

        // Act - apply all filters
        let result = apply_filters(&img, 20.0, 30.0, 1.5);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - result should be different from original
        assert_ne!(pixel[0], MID_GRAY);
    }

    #[test]
    fn test_apply_filters_preserves_alpha() {
        // Arrange
        let alpha_value = 200;
        let img = create_test_image(MID_GRAY, MID_GRAY, MID_GRAY, alpha_value);

        // Act
        let result = apply_filters(&img, 25.0, 25.0, 1.5);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - alpha should be preserved through all filters
        assert_eq!(pixel[3], alpha_value);
    }

    #[test]
    fn test_apply_filters_near_zero_treated_as_zero() {
        // Arrange
        let img = create_test_image(100, 150, 200, WHITE);

        // Act - values very close to default should be treated as no-op
        let result = apply_filters(&img, 0.0005, 0.0005, 1.0005);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - image should be unchanged (values below threshold)
        assert_eq!(pixel[0], 100);
        assert_eq!(pixel[1], 150);
        assert_eq!(pixel[2], 200);
    }

    #[test]
    fn test_contrast_positive_increases_range() {
        // Arrange - image with mid-gray
        let img = create_test_image(192, 64, MID_GRAY, WHITE);

        // Act - positive contrast increases difference from mid-gray
        let result = apply_contrast(&img, 50.0);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - 192 should move further from 128 (toward white)
        assert!(pixel[0] > 192);
        // 64 should move further from 128 (toward black)
        assert!(pixel[1] < 64);
    }

    #[test]
    fn test_contrast_negative_decreases_range() {
        // Arrange - image with values far from mid-gray
        let img = create_test_image(220, 30, MID_GRAY, WHITE);

        // Act - negative contrast decreases difference from mid-gray
        let result = apply_contrast(&img, -50.0);
        let result_rgba = result.to_rgba8();
        let pixel = result_rgba.get_pixel(0, 0);

        // Assert - 220 should move toward 128
        assert!(pixel[0] < 220 && pixel[0] > MID_GRAY);
        // 30 should move toward 128
        assert!(pixel[1] > 30 && pixel[1] < MID_GRAY);
    }
}
