//! Planar f32 bitmap used by the local-contrast pipeline. Each channel is its
//! own `Vec<f32>` in row-major order (index = `y * width + x`), values in
//! `[0.0, 1.0]`. See `docs/local-contrast-spec.md` §1 for context.
//!
//! This is the Rust analogue of the C# `FloatMap`. We intentionally keep it
//! lean — just the planes we actually need plus cheap conversions to and
//! from `image::RgbaImage`. Higher-level data like Oklab channels are held
//! in separate `FloatMap` instances rather than extra fields here.

#![allow(dead_code)] // `new`/`pixel_count`/`idx`/`to_rgba8` round out the API and are exercised by tests.

use image::{ImageBuffer, Rgba, RgbaImage};

/// Byte-to-float conversion factor.
const BYTE_TO_F32: f32 = 1.0 / 255.0;

/// Planar f32 RGB image with an optional alpha channel.
#[derive(Debug, Clone)]
pub struct FloatMap {
    pub width: u32,
    pub height: u32,
    pub r: Vec<f32>,
    pub g: Vec<f32>,
    pub b: Vec<f32>,
    pub a: Option<Vec<f32>>,
}

impl FloatMap {
    /// Create a new, zero-initialized map. `with_alpha` fills the alpha plane
    /// to 1.0 (fully opaque).
    pub fn new(width: u32, height: u32, with_alpha: bool) -> Self {
        let n = (width as usize) * (height as usize);
        Self {
            width,
            height,
            r: vec![0.0; n],
            g: vec![0.0; n],
            b: vec![0.0; n],
            a: if with_alpha { Some(vec![1.0; n]) } else { None },
        }
    }

    /// Total pixel count.
    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Row-major index for `(x, y)`. Debug-assertion bounds-checked.
    #[inline]
    pub fn idx(&self, x: u32, y: u32) -> usize {
        debug_assert!(x < self.width && y < self.height);
        (y as usize) * (self.width as usize) + (x as usize)
    }

    /// Copy pixels from an 8-bit RGBA image into float planes.
    pub fn from_rgba8(img: &RgbaImage) -> Self {
        let (width, height) = img.dimensions();
        let n = (width as usize) * (height as usize);
        let mut r = Vec::with_capacity(n);
        let mut g = Vec::with_capacity(n);
        let mut b = Vec::with_capacity(n);
        let mut a = Vec::with_capacity(n);
        for px in img.pixels() {
            r.push(px[0] as f32 * BYTE_TO_F32);
            g.push(px[1] as f32 * BYTE_TO_F32);
            b.push(px[2] as f32 * BYTE_TO_F32);
            a.push(px[3] as f32 * BYTE_TO_F32);
        }
        Self {
            width,
            height,
            r,
            g,
            b,
            a: Some(a),
        }
    }

    /// Pack the planes back into an 8-bit RGBA image. Out-of-range floats are
    /// clamped and rounded.
    pub fn to_rgba8(&self) -> RgbaImage {
        let mut out = ImageBuffer::new(self.width, self.height);
        match &self.a {
            Some(a) => {
                for (i, px) in out.pixels_mut().enumerate() {
                    *px = Rgba([
                        float_to_byte(self.r[i]),
                        float_to_byte(self.g[i]),
                        float_to_byte(self.b[i]),
                        float_to_byte(a[i]),
                    ]);
                }
            }
            None => {
                for (i, px) in out.pixels_mut().enumerate() {
                    *px = Rgba([
                        float_to_byte(self.r[i]),
                        float_to_byte(self.g[i]),
                        float_to_byte(self.b[i]),
                        255,
                    ]);
                }
            }
        }
        out
    }

    /// Return a Lanczos3-resampled copy at `new_width` × `new_height`. Goes
    /// through `image::Rgba32FImage` so no precision is lost in the
    /// round-trip.
    pub fn resize_lanczos3(&self, new_width: u32, new_height: u32) -> FloatMap {
        use image::{Rgba, Rgba32FImage, imageops::FilterType, imageops::resize};
        let mut src = Rgba32FImage::new(self.width, self.height);
        for (i, px) in src.pixels_mut().enumerate() {
            let a = self.a.as_ref().map(|v| v[i]).unwrap_or(1.0);
            *px = Rgba([self.r[i], self.g[i], self.b[i], a]);
        }
        let resized = resize(&src, new_width, new_height, FilterType::Lanczos3);
        let new_n = (new_width as usize) * (new_height as usize);
        let mut r = Vec::with_capacity(new_n);
        let mut g = Vec::with_capacity(new_n);
        let mut b = Vec::with_capacity(new_n);
        let mut alpha: Option<Vec<f32>> = self.a.as_ref().map(|_| Vec::with_capacity(new_n));
        for px in resized.pixels() {
            r.push(px[0]);
            g.push(px[1]);
            b.push(px[2]);
            if let Some(ref mut a_vec) = alpha {
                a_vec.push(px[3]);
            }
        }
        FloatMap {
            width: new_width,
            height: new_height,
            r,
            g,
            b,
            a: alpha,
        }
    }

    /// Pack the planes into GPUI's expected pixel layout (BGRA) for use with
    /// `gpui::RenderImage`. The returned buffer is typed as `RgbaImage`
    /// because that's what `image::Frame::new` consumes, but the byte order
    /// is `B, G, R, A`.
    pub fn to_bgra_image(&self) -> RgbaImage {
        let mut out = ImageBuffer::new(self.width, self.height);
        match &self.a {
            Some(a) => {
                for (i, px) in out.pixels_mut().enumerate() {
                    *px = Rgba([
                        float_to_byte(self.b[i]),
                        float_to_byte(self.g[i]),
                        float_to_byte(self.r[i]),
                        float_to_byte(a[i]),
                    ]);
                }
            }
            None => {
                for (i, px) in out.pixels_mut().enumerate() {
                    *px = Rgba([
                        float_to_byte(self.b[i]),
                        float_to_byte(self.g[i]),
                        float_to_byte(self.r[i]),
                        255,
                    ]);
                }
            }
        }
        out
    }
}

/// Round and clamp a normalized float to an 8-bit component.
#[inline]
pub fn float_to_byte(f: f32) -> u8 {
    let scaled = (f * 255.0).round();
    if scaled < 0.0 {
        0
    } else if scaled > 255.0 {
        255
    } else {
        scaled as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_fills_defaults() {
        let m = FloatMap::new(3, 2, true);
        assert_eq!(m.pixel_count(), 6);
        assert!(m.r.iter().all(|&v| v == 0.0));
        assert!(m.a.as_ref().unwrap().iter().all(|&v| v == 1.0));
    }

    #[test]
    fn idx_row_major() {
        let m = FloatMap::new(4, 3, false);
        assert_eq!(m.idx(0, 0), 0);
        assert_eq!(m.idx(3, 0), 3);
        assert_eq!(m.idx(0, 1), 4);
        assert_eq!(m.idx(3, 2), 11);
    }

    #[test]
    fn rgba8_roundtrip_exact() {
        // Every 8-bit channel value should round-trip exactly.
        let mut img = RgbaImage::new(16, 16);
        for (i, px) in img.pixels_mut().enumerate() {
            let v = (i % 256) as u8;
            *px = Rgba([v, 255 - v, (v / 2) * 2, v.saturating_add(1)]);
        }
        let map = FloatMap::from_rgba8(&img);
        let back = map.to_rgba8();
        for (orig, got) in img.pixels().zip(back.pixels()) {
            assert_eq!(orig, got);
        }
    }

    #[test]
    fn float_to_byte_clamps() {
        assert_eq!(float_to_byte(-0.5), 0);
        assert_eq!(float_to_byte(0.0), 0);
        assert_eq!(float_to_byte(0.5), 128);
        assert_eq!(float_to_byte(1.0), 255);
        assert_eq!(float_to_byte(1.5), 255);
        assert!(float_to_byte(f32::NAN) == 0); // NaN comparison is false for both branches
    }

    // -- No-alpha initialization ----------------------------------------------

    #[test]
    fn new_without_alpha() {
        let m = FloatMap::new(2, 3, false);
        assert_eq!(m.pixel_count(), 6);
        assert!(m.a.is_none());
        assert!(m.r.iter().all(|&v| v == 0.0));
    }

    // -- pixel_count ----------------------------------------------------------

    #[test]
    fn pixel_count_1x1() {
        assert_eq!(FloatMap::new(1, 1, false).pixel_count(), 1);
    }

    #[test]
    fn pixel_count_large() {
        assert_eq!(FloatMap::new(100, 200, false).pixel_count(), 20_000);
    }

    // -- to_bgra_image channel swap -------------------------------------------

    #[test]
    fn bgra_swaps_red_and_blue() {
        let mut m = FloatMap::new(1, 1, false);
        m.r[0] = 1.0; // red = 255
        m.g[0] = 0.5; // green = 128
        m.b[0] = 0.0; // blue = 0
        let bgra = m.to_bgra_image();
        let px = bgra.get_pixel(0, 0);
        // BGRA layout: [B, G, R, A]
        assert_eq!(px[0], 0);   // B channel gets blue (0.0)
        assert_eq!(px[1], 128); // G channel gets green (0.5)
        assert_eq!(px[2], 255); // R channel gets red (1.0)
        assert_eq!(px[3], 255); // A defaults to 1.0 (no alpha plane)
    }

    #[test]
    fn bgra_with_alpha() {
        let mut m = FloatMap::new(1, 1, true);
        m.r[0] = 0.0;
        m.g[0] = 0.0;
        m.b[0] = 0.0;
        m.a.as_mut().unwrap()[0] = 0.5;
        let bgra = m.to_bgra_image();
        let px = bgra.get_pixel(0, 0);
        assert_eq!(px[3], 128); // alpha 0.5 → 128
    }

    // -- to_rgba8 alpha fallback ----------------------------------------------

    #[test]
    fn to_rgba8_no_alpha_defaults_opaque() {
        let m = FloatMap::new(2, 2, false);
        let img = m.to_rgba8();
        for px in img.pixels() {
            assert_eq!(px[3], 255, "expected fully opaque when no alpha plane");
        }
    }

    // -- resize ---------------------------------------------------------------

    #[test]
    fn resize_preserves_alpha_presence() {
        let with_alpha = FloatMap::new(4, 4, true);
        let resized = with_alpha.resize_lanczos3(2, 2);
        assert!(resized.a.is_some());
        assert_eq!(resized.pixel_count(), 4);

        let without_alpha = FloatMap::new(4, 4, false);
        let resized = without_alpha.resize_lanczos3(2, 2);
        assert!(resized.a.is_none());
        assert_eq!(resized.pixel_count(), 4);
    }

    #[test]
    fn resize_to_same_size() {
        let mut m = FloatMap::new(3, 3, false);
        for i in 0..9 {
            m.r[i] = i as f32 / 8.0;
        }
        let resized = m.resize_lanczos3(3, 3);
        assert_eq!(resized.width, 3);
        assert_eq!(resized.height, 3);
        // Values should be very close to originals (some Lanczos ringing allowed)
        for i in 0..9 {
            let diff = (m.r[i] - resized.r[i]).abs();
            assert!(diff < 0.15, "pixel {i}: orig={} resized={}", m.r[i], resized.r[i]);
        }
    }

    // -- from_rgba8 / to_rgba8 with specific values ---------------------------

    #[test]
    fn from_rgba8_converts_channels_correctly() {
        let mut img = RgbaImage::new(1, 1);
        *img.get_pixel_mut(0, 0) = Rgba([100, 150, 200, 255]);
        let m = FloatMap::from_rgba8(&img);
        let tol = 0.005;
        assert!((m.r[0] - 100.0 / 255.0).abs() < tol);
        assert!((m.g[0] - 150.0 / 255.0).abs() < tol);
        assert!((m.b[0] - 200.0 / 255.0).abs() < tol);
        assert!((m.a.as_ref().unwrap()[0] - 1.0).abs() < tol);
    }

    // -- float_to_byte boundary -----------------------------------------------

    #[test]
    fn float_to_byte_exact_boundaries() {
        // Verify each integer step maps correctly
        for i in 0u8..=255 {
            let f = i as f32 / 255.0;
            assert_eq!(float_to_byte(f), i, "byte value {i}");
        }
    }

    #[test]
    fn float_to_byte_infinity() {
        assert_eq!(float_to_byte(f32::INFINITY), 255);
        assert_eq!(float_to_byte(f32::NEG_INFINITY), 0);
    }
}
