//! Color-space conversions used by the local-contrast pipeline.
//!
//! Pipeline: `sRGB (u8 or f32 0..1)  →  linear sRGB  →  Oklab (L,a,b)  →  OkLCh (L,C,h)`
//! and back. All f32 throughout, no allocations in the per-pixel paths.
//!
//! Oklab matrices and the cube-root LMS form are from Björn Ottosson's
//! reference (<https://bottosson.github.io/posts/oklab/>). Forward matrix:
//!
//! ```text
//! l = 0.4122214708*r + 0.5363325363*g + 0.0514459929*b
//! m = 0.2119034982*r + 0.6806995451*g + 0.1073969566*b
//! s = 0.0883024619*r + 0.2817188376*g + 0.6299787005*b
//! L =  0.2104542553*l' + 0.7936177850*m' - 0.0040720468*s'
//! a =  1.9779984951*l' - 2.4285922050*m' + 0.4505937099*s'
//! b =  0.0259040371*l' + 0.7827717662*m' - 0.8086757660*s'
//! ```
//! where `l' = cbrt(l)`, etc.

use std::f32::consts::TAU;

// --- sRGB gamma encoding ------------------------------------------------------

/// sRGB ([0, 1]) → linear sRGB ([0, 1]). Component-wise.
#[inline]
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear sRGB ([0, 1]) → sRGB ([0, 1]). Component-wise.
#[inline]
pub fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

// --- Oklab --------------------------------------------------------------------

/// Linear sRGB → Oklab. Inputs should be in [0, 1]; outputs: L ≈ [0, 1],
/// a/b ≈ [-0.5, 0.5] for in-gamut colors.
#[inline]
pub fn linear_srgb_to_oklab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let l = 0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    let ok_l = 0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_;
    let ok_a = 1.977_998_5 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_;
    let ok_b = 0.025_904_037 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_;

    (ok_l, ok_a, ok_b)
}

/// Oklab → linear sRGB. Out-of-gamut results may fall outside [0, 1].
#[inline]
pub fn oklab_to_linear_srgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;

    let l_cubed = l_ * l_ * l_;
    let m_cubed = m_ * m_ * m_;
    let s_cubed = s_ * s_ * s_;

    let r = 4.076_741_7 * l_cubed - 3.307_711_6 * m_cubed + 0.230_969_94 * s_cubed;
    let g = -1.268_438 * l_cubed + 2.609_757_4 * m_cubed - 0.341_319_38 * s_cubed;
    let b = -0.004_196_086_3 * l_cubed - 0.703_418_6 * m_cubed + 1.707_614_7 * s_cubed;

    (r, g, b)
}

// --- OkLCh (polar form of Oklab) ---------------------------------------------

/// Oklab → OkLCh. Chroma C = sqrt(a² + b²); hue h in [0, τ) radians.
#[inline]
pub fn oklab_to_oklch(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let c = (a * a + b * b).sqrt();
    let mut h = b.atan2(a);
    if h < 0.0 {
        h += TAU;
    }
    (l, c, h)
}

/// OkLCh → Oklab.
#[inline]
pub fn oklch_to_oklab(l: f32, c: f32, h: f32) -> (f32, f32, f32) {
    (l, c * h.cos(), c * h.sin())
}

// --- Convenience: sRGB ↔ OkLCh ------------------------------------------------

/// sRGB (0-1 per channel) → OkLCh. This is the typical entry point for the
/// local-contrast pipeline.
#[inline]
pub fn srgb_to_oklch(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let lr = srgb_to_linear(r);
    let lg = srgb_to_linear(g);
    let lb = srgb_to_linear(b);
    let (l, a, b_) = linear_srgb_to_oklab(lr, lg, lb);
    oklab_to_oklch(l, a, b_)
}

/// OkLCh → sRGB (0-1 per channel). Caller should clamp to [0, 1] for display.
#[inline]
pub fn oklch_to_srgb(l: f32, c: f32, h: f32) -> (f32, f32, f32) {
    let (ol, oa, ob) = oklch_to_oklab(l, c, h);
    let (lr, lg, lb) = oklab_to_linear_srgb(ol, oa, ob);
    (linear_to_srgb(lr), linear_to_srgb(lg), linear_to_srgb(lb))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    // --- Gamma round-trip -----------------------------------------------------

    #[test]
    fn gamma_roundtrip_endpoints() {
        assert!(approx(linear_to_srgb(srgb_to_linear(0.0)), 0.0, 1e-7));
        assert!(approx(linear_to_srgb(srgb_to_linear(1.0)), 1.0, 1e-6));
    }

    #[test]
    fn gamma_roundtrip_many() {
        for i in 0..=255 {
            let v = i as f32 / 255.0;
            let rt = linear_to_srgb(srgb_to_linear(v));
            assert!(approx(rt, v, 1e-5), "v={} rt={}", v, rt);
        }
    }

    // --- Oklab known values ---------------------------------------------------

    #[test]
    fn oklab_black_is_origin() {
        let (l, a, b) = linear_srgb_to_oklab(0.0, 0.0, 0.0);
        assert!(approx(l, 0.0, EPS));
        assert!(approx(a, 0.0, EPS));
        assert!(approx(b, 0.0, EPS));
    }

    #[test]
    fn oklab_white_has_l_one() {
        let (l, a, b) = linear_srgb_to_oklab(1.0, 1.0, 1.0);
        assert!(approx(l, 1.0, EPS));
        assert!(approx(a, 0.0, EPS));
        assert!(approx(b, 0.0, EPS));
    }

    #[test]
    fn oklab_primaries_have_expected_sign() {
        // Red: +a, +b roughly
        let (_, a_r, b_r) = linear_srgb_to_oklab(1.0, 0.0, 0.0);
        assert!(a_r > 0.0 && b_r > 0.0, "red a={} b={}", a_r, b_r);
        // Green: -a, +b
        let (_, a_g, b_g) = linear_srgb_to_oklab(0.0, 1.0, 0.0);
        assert!(a_g < 0.0 && b_g > 0.0, "green a={} b={}", a_g, b_g);
        // Blue: negative b
        let (_, _, b_b) = linear_srgb_to_oklab(0.0, 0.0, 1.0);
        assert!(b_b < 0.0, "blue b={}", b_b);
    }

    // --- Oklab round-trip -----------------------------------------------------

    #[test]
    fn oklab_roundtrip_grid() {
        // Coarse grid over linear sRGB; check round-trip error stays tight.
        let mut max_err: f32 = 0.0;
        for ri in 0..=8 {
            for gi in 0..=8 {
                for bi in 0..=8 {
                    let r = ri as f32 / 8.0;
                    let g = gi as f32 / 8.0;
                    let b = bi as f32 / 8.0;
                    let (ol, oa, ob) = linear_srgb_to_oklab(r, g, b);
                    let (r2, g2, b2) = oklab_to_linear_srgb(ol, oa, ob);
                    max_err = max_err.max((r - r2).abs());
                    max_err = max_err.max((g - g2).abs());
                    max_err = max_err.max((b - b2).abs());
                }
            }
        }
        assert!(max_err < 1e-4, "oklab roundtrip max error = {}", max_err);
    }

    // --- OkLCh round-trip -----------------------------------------------------

    #[test]
    fn oklch_roundtrip_polar() {
        for deg in (0..360).step_by(10) {
            let h_rad = (deg as f32).to_radians();
            let (l2, a2, b2) = oklch_to_oklab(0.5, 0.12, h_rad);
            let (l3, c3, h3) = oklab_to_oklch(l2, a2, b2);
            assert!(approx(l2, l3, EPS));
            assert!(approx(0.12, c3, EPS));
            // Hue wraps — compare modulo TAU.
            let hd = ((h3 - h_rad).rem_euclid(TAU) - 0.0).abs();
            assert!(
                hd < 1e-3 || (TAU - hd) < 1e-3,
                "deg={} expected h={} got h={}",
                deg,
                h_rad,
                h3
            );
        }
    }

    #[test]
    fn srgb_to_oklch_and_back_grid() {
        // Full pipeline over a 9³ sRGB grid. Expect < 1e-4 round-trip error.
        let mut max_err: f32 = 0.0;
        for ri in 0..=8 {
            for gi in 0..=8 {
                for bi in 0..=8 {
                    let r = ri as f32 / 8.0;
                    let g = gi as f32 / 8.0;
                    let b = bi as f32 / 8.0;
                    let (l, c, h) = srgb_to_oklch(r, g, b);
                    let (r2, g2, b2) = oklch_to_srgb(l, c, h);
                    max_err = max_err.max((r - r2).abs());
                    max_err = max_err.max((g - g2).abs());
                    max_err = max_err.max((b - b2).abs());
                }
            }
        }
        assert!(
            max_err < 1e-4,
            "srgb↔oklch roundtrip max error = {}",
            max_err
        );
    }

    #[test]
    fn achromatic_has_zero_chroma() {
        for i in 0..=10 {
            let v = i as f32 / 10.0;
            let (_, c, _) = srgb_to_oklch(v, v, v);
            assert!(c < 1e-4, "gray v={} got chroma={}", v, c);
        }
    }
}
