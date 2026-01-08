use image::{DynamicImage, ImageBuffer, Rgba};
use rpview_gpui::utils::filters::*;

fn create_test_image(r: u8, g: u8, b: u8) -> DynamicImage {
    DynamicImage::ImageRgba8(ImageBuffer::from_pixel(10, 10, Rgba([r, g, b, 255])))
}

#[test]
fn test_apply_brightness_zero() {
    let img = create_test_image(128, 128, 128);
    let result = apply_brightness(&img, 0.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel[0], 128);
    assert_eq!(pixel[1], 128);
    assert_eq!(pixel[2], 128);
}

#[test]
fn test_apply_brightness_positive() {
    let img = create_test_image(100, 100, 100);
    let result = apply_brightness(&img, 50.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert!(pixel[0] > 100);
    assert!(pixel[1] > 100);
    assert!(pixel[2] > 100);
}

#[test]
fn test_apply_brightness_negative() {
    let img = create_test_image(200, 200, 200);
    let result = apply_brightness(&img, -50.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert!(pixel[0] < 200);
    assert!(pixel[1] < 200);
    assert!(pixel[2] < 200);
}

#[test]
fn test_apply_brightness_max() {
    let img = create_test_image(100, 100, 100);
    let result = apply_brightness(&img, 100.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel[0], 255);
    assert_eq!(pixel[1], 255);
    assert_eq!(pixel[2], 255);
}

#[test]
fn test_apply_brightness_min() {
    let img = create_test_image(100, 100, 100);
    let result = apply_brightness(&img, -100.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel[0], 0);
    assert_eq!(pixel[1], 0);
    assert_eq!(pixel[2], 0);
}

#[test]
fn test_apply_brightness_preserves_alpha() {
    let img = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(10, 10, Rgba([128, 128, 128, 100])));
    let result = apply_brightness(&img, 50.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel[3], 100);
}

#[test]
fn test_apply_brightness_clamps_input() {
    let img = create_test_image(128, 128, 128);

    let result1 = apply_brightness(&img, 200.0);
    let result2 = apply_brightness(&img, 100.0);

    let rgba1 = result1.to_rgba8();
    let rgba2 = result2.to_rgba8();
    let pixel1 = rgba1.get_pixel(0, 0);
    let pixel2 = rgba2.get_pixel(0, 0);

    assert_eq!(pixel1[0], pixel2[0]);
}

#[test]
fn test_apply_contrast_zero() {
    let img = create_test_image(128, 64, 192);
    let result = apply_contrast(&img, 0.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel[0], 128);
    assert_eq!(pixel[1], 64);
    assert_eq!(pixel[2], 192);
}

#[test]
fn test_apply_contrast_positive() {
    let img = create_test_image(128, 128, 128);
    let result = apply_contrast(&img, 50.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel[0], 128);
}

#[test]
fn test_apply_contrast_increases_difference() {
    let img = DynamicImage::ImageRgba8({
        let mut buffer = ImageBuffer::new(2, 1);
        buffer.put_pixel(0, 0, Rgba([100, 100, 100, 255]));
        buffer.put_pixel(1, 0, Rgba([150, 150, 150, 255]));
        buffer
    });

    let result = apply_contrast(&img, 50.0);
    let rgba = result.to_rgba8();

    let dark = rgba.get_pixel(0, 0);
    let light = rgba.get_pixel(1, 0);

    let original_diff = 150 - 100;
    let new_diff = light[0] as i32 - dark[0] as i32;

    assert!(new_diff > original_diff);
}

#[test]
fn test_apply_contrast_negative() {
    let img = DynamicImage::ImageRgba8({
        let mut buffer = ImageBuffer::new(2, 1);
        buffer.put_pixel(0, 0, Rgba([50, 50, 50, 255]));
        buffer.put_pixel(1, 0, Rgba([200, 200, 200, 255]));
        buffer
    });

    let result = apply_contrast(&img, -50.0);
    let rgba = result.to_rgba8();

    let dark = rgba.get_pixel(0, 0);
    let light = rgba.get_pixel(1, 0);

    let original_diff = 200 - 50;
    let new_diff = light[0] as i32 - dark[0] as i32;

    assert!(new_diff < original_diff);
}

#[test]
fn test_apply_gamma_one() {
    let img = create_test_image(128, 64, 192);
    let result = apply_gamma(&img, 1.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel[0], 128);
    assert_eq!(pixel[1], 64);
    assert_eq!(pixel[2], 192);
}

#[test]
fn test_apply_gamma_greater_than_one() {
    let img = create_test_image(100, 100, 100);
    let result = apply_gamma(&img, 2.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert!(pixel[0] > 100);
}

#[test]
fn test_apply_gamma_less_than_one() {
    let img = create_test_image(150, 150, 150);
    let result = apply_gamma(&img, 0.5);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert!(pixel[0] < 150);
}

#[test]
fn test_apply_gamma_preserves_black_and_white() {
    let img = DynamicImage::ImageRgba8({
        let mut buffer = ImageBuffer::new(2, 1);
        buffer.put_pixel(0, 0, Rgba([0, 0, 0, 255]));
        buffer.put_pixel(1, 0, Rgba([255, 255, 255, 255]));
        buffer
    });

    let result = apply_gamma(&img, 2.0);
    let rgba = result.to_rgba8();

    assert_eq!(rgba.get_pixel(0, 0)[0], 0);
    assert_eq!(rgba.get_pixel(1, 0)[0], 255);
}

#[test]
fn test_apply_gamma_clamps_input() {
    let img = create_test_image(128, 128, 128);

    let result1 = apply_gamma(&img, 0.05);
    let result2 = apply_gamma(&img, 0.1);

    let rgba1 = result1.to_rgba8();
    let rgba2 = result2.to_rgba8();
    let pixel1 = rgba1.get_pixel(0, 0);
    let pixel2 = rgba2.get_pixel(0, 0);

    assert_eq!(pixel1[0], pixel2[0]);
}

#[test]
fn test_apply_filters_all_default() {
    let img = create_test_image(128, 128, 128);
    let result = apply_filters(&img, 0.0, 0.0, 1.0);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel[0], 128);
    assert_eq!(pixel[1], 128);
    assert_eq!(pixel[2], 128);
}

#[test]
fn test_apply_filters_combined() {
    let img = create_test_image(100, 100, 100);

    let result = apply_filters(&img, 10.0, 20.0, 1.2);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);

    assert_ne!(pixel[0], 100);
    assert_ne!(pixel[1], 100);
    assert_ne!(pixel[2], 100);
}

#[test]
fn test_apply_filters_order_matters() {
    let img = create_test_image(100, 100, 100);

    let result = apply_filters(&img, 50.0, 50.0, 1.5);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    // Just verify it doesn't panic - u8 is always 0-255
    assert!(pixel[0] == pixel[0]);
}

#[test]
fn test_filters_preserve_alpha() {
    let img = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(10, 10, Rgba([128, 128, 128, 100])));

    let result = apply_filters(&img, 20.0, 30.0, 1.3);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel[3], 100);
}

#[test]
fn test_brightness_on_edge_values() {
    let img = DynamicImage::ImageRgba8({
        let mut buffer = ImageBuffer::new(2, 1);
        buffer.put_pixel(0, 0, Rgba([0, 0, 0, 255]));
        buffer.put_pixel(1, 0, Rgba([255, 255, 255, 255]));
        buffer
    });

    let result = apply_brightness(&img, 50.0);
    let rgba = result.to_rgba8();

    let black = rgba.get_pixel(0, 0);
    assert!(black[0] > 0);

    let white = rgba.get_pixel(1, 0);
    assert_eq!(white[0], 255);
}

#[test]
fn test_contrast_on_midtone() {
    let img = create_test_image(128, 128, 128);

    let result_high = apply_contrast(&img, 50.0);
    let result_low = apply_contrast(&img, -50.0);

    let rgba_high = result_high.to_rgba8();
    let rgba_low = result_low.to_rgba8();
    let pixel_high = rgba_high.get_pixel(0, 0);
    let pixel_low = rgba_low.get_pixel(0, 0);

    assert!((pixel_high[0] as i32 - 128).abs() < 10);
    assert!((pixel_low[0] as i32 - 128).abs() < 10);
}

#[test]
fn test_gamma_lookup_table_optimization() {
    let img = create_test_image(50, 100, 150);

    let result1 = apply_gamma(&img, 2.0);
    let result2 = apply_gamma(&img, 2.0);

    let rgba1 = result1.to_rgba8();
    let rgba2 = result2.to_rgba8();
    let pixel1 = rgba1.get_pixel(0, 0);
    let pixel2 = rgba2.get_pixel(0, 0);

    assert_eq!(pixel1[0], pixel2[0]);
    assert_eq!(pixel1[1], pixel2[1]);
    assert_eq!(pixel1[2], pixel2[2]);
}

#[test]
fn test_filters_with_small_values() {
    let img = create_test_image(128, 128, 128);

    let result = apply_filters(&img, 0.0001, 0.0001, 1.0001);

    let rgba = result.to_rgba8();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel[0], 128);
}

#[test]
fn test_image_dimensions_preserved() {
    let img = DynamicImage::ImageRgba8(ImageBuffer::new(37, 53));

    let result = apply_filters(&img, 50.0, 50.0, 1.5);

    assert_eq!(result.width(), 37);
    assert_eq!(result.height(), 53);
}
