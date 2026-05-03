use image::{DynamicImage, ImageBuffer};

/// Apply all filters to an image in a single pass using a combined LUT.
/// This is more efficient than applying filters sequentially, as it:
/// 1. Only iterates through pixels once instead of up to 3 times
/// 2. Only allocates one output buffer instead of up to 3
/// 3. Pre-computes all transformations into a single 256-entry lookup table
///
/// Currently exercised only by the test suite — the production save path
/// extracts BGRA bytes from the cached `filtered_render` instead of
/// re-applying filters on disk.  Kept around for symmetry with
/// `apply_filters_to_bgra` and as a `DynamicImage`-friendly entry point.
#[allow(dead_code)]
pub fn apply_filters(
    img: &DynamicImage,
    brightness: f32,
    contrast: f32,
    gamma: f32,
) -> DynamicImage {
    let Some(lut) = build_filter_lut(brightness, contrast, gamma) else {
        return img.clone();
    };

    // Apply combined LUT in a single pass using direct slice access
    let owned;
    let rgba_img: &image::RgbaImage = match img.as_rgba8() {
        Some(buf) => buf,
        None => {
            owned = img.to_rgba8();
            &owned
        }
    };
    let (width, height) = rgba_img.dimensions();
    let mut output = ImageBuffer::new(width, height);

    let input_bytes = rgba_img.as_raw();
    let output_bytes: &mut [u8] = output.as_mut();
    for (src, dst) in input_bytes
        .chunks_exact(4)
        .zip(output_bytes.chunks_exact_mut(4))
    {
        dst[0] = lut[src[0] as usize];
        dst[1] = lut[src[1] as usize];
        dst[2] = lut[src[2] as usize];
        dst[3] = src[3];
    }

    DynamicImage::ImageRgba8(output)
}

/// Build the combined brightness/contrast/gamma LUT (256 entries).
/// Returns `None` if all three values are no-ops, signalling "pass-through."
fn build_filter_lut(brightness: f32, contrast: f32, gamma: f32) -> Option<[u8; 256]> {
    let has_brightness = brightness.abs() >= 0.001;
    let has_contrast = contrast.abs() >= 0.001;
    let has_gamma = (gamma - 1.0).abs() >= 0.001;
    if !has_brightness && !has_contrast && !has_gamma {
        return None;
    }

    let brightness = brightness.clamp(-100.0, 100.0);
    let contrast = contrast.clamp(-100.0, 100.0);
    let gamma = gamma.clamp(0.1, 10.0);

    let contrast_factor = if contrast > 0.0 {
        1.0 + (contrast / 100.0) * 2.0
    } else {
        1.0 + (contrast / 100.0) * 0.9
    };
    let brightness_adjustment = brightness * 2.55;
    let inv_gamma = 1.0 / gamma;

    let mut lut = [0u8; 256];
    for (i, entry) in lut.iter_mut().enumerate() {
        let mut v = i as f32;
        if has_brightness {
            v = (v + brightness_adjustment).clamp(0.0, 255.0);
        }
        if has_contrast {
            v = (((v / 255.0) - 0.5) * contrast_factor + 0.5) * 255.0;
            v = v.clamp(0.0, 255.0);
        }
        if has_gamma {
            v = (v / 255.0).powf(inv_gamma) * 255.0;
        }
        *entry = v.clamp(0.0, 255.0) as u8;
    }
    Some(lut)
}

/// Apply brightness / contrast / gamma to an RGBA source, writing the result into a
/// freshly-allocated **BGRA** buffer of identical dimensions. This is the layout GPUI
/// expects for `RenderImage`, so callers can feed the result directly to `Frame::new`
/// without a separate channel swap pass.
///
/// If all three values are no-ops, returns a plain RGBA→BGRA copy (no LUT applied).
pub fn apply_filters_to_bgra(
    src: &image::RgbaImage,
    brightness: f32,
    contrast: f32,
    gamma: f32,
) -> image::RgbaImage {
    let (width, height) = src.dimensions();
    let mut output = image::RgbaImage::new(width, height);

    let src_bytes = src.as_raw();
    let dst_bytes: &mut [u8] = &mut output;

    match build_filter_lut(brightness, contrast, gamma) {
        Some(lut) => {
            for (s, d) in src_bytes.chunks_exact(4).zip(dst_bytes.chunks_exact_mut(4)) {
                // RGBA source → BGRA dest, with LUT applied to RGB channels.
                d[0] = lut[s[2] as usize];
                d[1] = lut[s[1] as usize];
                d[2] = lut[s[0] as usize];
                d[3] = s[3];
            }
        }
        None => {
            // Filters are no-ops; just swap channels.
            for (s, d) in src_bytes.chunks_exact(4).zip(dst_bytes.chunks_exact_mut(4)) {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = s[3];
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    // Test constants
    const MID_GRAY: u8 = 128;
    const WHITE: u8 = 255;
    const DEFAULT_GAMMA: f32 = 1.0;

    /// Helper function to create a 1x1 test image with a specific color
    fn create_test_image(r: u8, g: u8, b: u8, a: u8) -> DynamicImage {
        DynamicImage::ImageRgba8(ImageBuffer::from_pixel(1, 1, Rgba([r, g, b, a])))
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
    fn test_apply_filters_to_bgra_noop_swaps_channels() {
        // RGBA (10, 20, 30, 200) → BGRA (30, 20, 10, 200)
        let img = ImageBuffer::from_pixel(2, 1, Rgba([10u8, 20, 30, 200]));
        let out = apply_filters_to_bgra(&img, 0.0, 0.0, 1.0);
        let p = out.get_pixel(0, 0);
        assert_eq!(p.0, [30, 20, 10, 200]);
    }

    #[test]
    fn test_apply_filters_to_bgra_applies_lut_and_swaps() {
        // With brightness +100 (full range +255 mapped), all RGB clamp to 255 regardless of input.
        let img = ImageBuffer::from_pixel(1, 1, Rgba([10u8, 20, 30, 77]));
        let out = apply_filters_to_bgra(&img, 100.0, 0.0, 1.0);
        let p = out.get_pixel(0, 0);
        assert_eq!(p.0, [255, 255, 255, 77]);
    }
}
