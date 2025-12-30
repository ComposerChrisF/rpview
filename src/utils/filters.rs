use image::{DynamicImage, ImageBuffer, Rgba};

/// Apply brightness adjustment to an image
/// brightness: -100.0 to +100.0
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
fn apply_contrast_to_channel(value: u8, factor: f32) -> u8 {
    let normalized = (value as f32) / 255.0;
    let adjusted = ((normalized - 0.5) * factor + 0.5) * 255.0;
    adjusted.clamp(0.0, 255.0) as u8
}

/// Apply gamma correction to an image
/// gamma: 0.1 to 10.0 (1.0 = no change, <1.0 = darker, >1.0 = brighter)
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
    for i in 0..256 {
        let normalized = (i as f32) / 255.0;
        let corrected = normalized.powf(1.0 / gamma);
        gamma_lut[i] = (corrected * 255.0).clamp(0.0, 255.0) as u8;
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

/// Apply all filters to an image
pub fn apply_filters(img: &DynamicImage, brightness: f32, contrast: f32, gamma: f32) -> DynamicImage {
    // Short-circuit if no filters are applied
    if brightness.abs() < 0.001 && contrast.abs() < 0.001 && (gamma - 1.0).abs() < 0.001 {
        return img.clone();
    }
    
    let mut result = img.clone();
    
    // Apply filters in order: brightness -> contrast -> gamma
    if brightness.abs() >= 0.001 {
        result = apply_brightness(&result, brightness);
    }
    
    if contrast.abs() >= 0.001 {
        result = apply_contrast(&result, contrast);
    }
    
    if (gamma - 1.0).abs() >= 0.001 {
        result = apply_gamma(&result, gamma);
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    
    #[test]
    fn test_apply_contrast_to_channel() {
        // Test with factor 1.0 (no change)
        assert_eq!(apply_contrast_to_channel(128, 1.0), 128);
        
        // Test with factor > 1.0 (increase contrast)
        assert!(apply_contrast_to_channel(200, 2.0) > 200);
        assert!(apply_contrast_to_channel(50, 2.0) < 50);
        
        // Test with factor < 1.0 (decrease contrast)
        let result = apply_contrast_to_channel(200, 0.5);
        assert!(result < 200 && result > 128);
    }
    
    #[test]
    fn test_brightness_clamps() {
        // Create a simple 1x1 image
        let img = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(1, 1, Rgba([128, 128, 128, 255])));
        
        // Test extreme brightness values are clamped
        let bright = apply_brightness(&img, 150.0); // Should clamp to 100
        let bright_rgba = bright.to_rgba8();
        let pixel = bright_rgba.get_pixel(0, 0);
        assert_eq!(pixel[0], 255); // 128 + 255 = 383, clamped to 255
        
        let dark = apply_brightness(&img, -150.0); // Should clamp to -100
        let dark_rgba = dark.to_rgba8();
        let pixel = dark_rgba.get_pixel(0, 0);
        assert_eq!(pixel[0], 0); // 128 - 255 = -127, clamped to 0
    }
}
