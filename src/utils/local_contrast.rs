#![allow(dead_code)] // Consumed by future UI wiring (Phase D).

//! Phase B port of `FraleyMusic.ImageDsp.AdaptiveContrastDsp.LocallyNormalizeLuminance`.
//!
//! Single-threaded, OkLCh-based (swap for HSL from the C# original).
//! See `docs/local-contrast-spec.md` §3 for the algorithm narrative.
//!
//! Intentional deviations from the C# source at this phase:
//! - Color space is OkLCh, not HSL. The scalar L is Oklab's perceptual L.
//! - The §3.4 step 11 "desaturate near extremes" heuristic is omitted — it's
//!   an HSL workaround; Oklab chroma degrades gracefully at the endpoints.
//! - Cumulative-sum per histogram is precomputed once (O(256) per site),
//!   turning each per-pixel histogram lookup from O(256) to O(1). This
//!   changes no outputs; it's a straightforward implementation choice the
//!   C# original happens not to make.
//!
//! All other behavior — including the known `Contrast_Std` quirk on the
//! white-side branch (mixed 0..1 / 0..255 scale) — is preserved verbatim
//! for parity with C# reference outputs. See §6 of the spec for the
//! efficiency passes planned for Phase E.

use crate::utils::color;
use crate::utils::float_map::FloatMap;

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

/// What image to produce — `Dsp` is the normal processed output; the other
/// variants expose intermediate visualizations useful for tuning defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnImage {
    Dsp,
    MedianGrayPointColored,
    MedianGrayPointLuminance,
    MeanGrayPointColored,
    MeanGrayPointLuminance,
    NormalizedValueColored,
    NormalizedValueLuminance,
}

/// Full parameter set for `locally_normalize_luminance`. Defaults match the
/// C# `AdaptiveContrastDsp.Parameters` constructor defaults. See
/// `docs/local-contrast-spec.md` §3.1 for semantics.
#[derive(Debug, Clone)]
pub struct Parameters {
    /// Radius (in pixels) of the local window. `0` = auto (`width / 32`).
    pub cxy_window: u32,
    /// Grid stride between histogram sample sites. `0` = auto
    /// (`cxy_window / 4`), floored at `2`.
    pub cxy_block: u32,
    pub alpha_black: f32,
    pub alpha_white: f32,
    pub use_median_for_contrast: bool,
    pub use_document_contrast: bool,
    pub tilt_black_doc_contrast: f32,
    pub tilt_white_doc_contrast: f32,
    pub apply_contrast_to_bw: bool,
    pub apply_contrast_to_xition: bool,
    pub mix_document_contrast: f32,
    pub contrast: f32,
    pub lighten_shadows: f32,
    pub darken_highlights: f32,
    pub return_image: ReturnImage,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            cxy_window: 0,
            cxy_block: 0,
            alpha_black: 0.04,
            alpha_white: 0.04,
            use_median_for_contrast: false,
            use_document_contrast: false,
            tilt_black_doc_contrast: -0.20,
            tilt_white_doc_contrast: -0.05,
            apply_contrast_to_bw: true,
            apply_contrast_to_xition: true,
            mix_document_contrast: 1.0,
            contrast: 0.04,
            lighten_shadows: 0.50,
            darken_highlights: 0.0,
            return_image: ReturnImage::Dsp,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: scalar math
// ---------------------------------------------------------------------------

#[inline]
fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

#[inline]
fn lerp(v0: f32, v1: f32, frac: f32) -> f32 {
    v0 + frac * (v1 - v0)
}

#[inline]
fn lerp_remap(v_out0: f32, v_out1: f32, v_in: f32, v_in0: f32, v_in1: f32) -> f32 {
    lerp(v_out0, v_out1, (v_in - v_in0) / (v_in1 - v_in0))
}

/// Standard contrast around `l_gray`. C# `Contrast_Std`, faithful port.
///
/// Quirk preserved: the white-side branch uses `255.0 - l_*` arithmetic even
/// though `l_cur` / `l_gray` are in `[0, 1]` — this yields an asymmetric
/// behavior vs the black branch. See spec §3.4 step 6.
fn contrast_std(l_gray: f32, l_cur: f32, contrast: f32) -> f32 {
    if l_cur < l_gray {
        let frac = l_cur / l_gray;
        let frac = frac * frac;
        let l_new = frac * l_gray;
        contrast * l_new + (1.0 - contrast) * l_cur
    } else {
        let frac = (255.0 - l_cur) / (255.0 - l_gray);
        let frac = frac * frac;
        let l_new = 255.0 - frac * (255.0 - l_gray);
        contrast * l_new + (1.0 - contrast) * l_cur
    }
}

/// Tilt the gray-point threshold toward black (tilt < 0) or white (tilt > 0).
fn apply_tilt(l_gray: f32, tilt: f32) -> f32 {
    if tilt <= 0.0 {
        l_gray * (1.0 + tilt)
    } else {
        1.0 - (1.0 - l_gray) * (1.0 - tilt)
    }
}

/// Document-mode contrast. C# `Contrast_Doc`.
fn contrast_doc(
    l_gray: f32,
    l_cur: f32,
    contrast: f32,
    tilt_black: f32,
    tilt_white: f32,
    apply_contrast_to_xition: bool,
    apply_contrast_to_bw: bool,
) -> f32 {
    let l_black = apply_tilt(l_gray, tilt_black);
    if l_cur < l_black {
        return if apply_contrast_to_bw {
            clamp01(lerp_remap(l_cur, 0.0, contrast, 0.0, 1.0))
        } else {
            0.0
        };
    }
    let l_white = apply_tilt(l_gray, tilt_white);
    if l_cur > l_white {
        return if apply_contrast_to_bw {
            clamp01(lerp(l_cur, 1.0, contrast))
        } else {
            1.0
        };
    }
    let l_new = lerp_remap(0.0, 1.0, l_cur, l_black, l_white);
    if !apply_contrast_to_xition {
        return clamp01(l_new);
    }
    clamp01(lerp_remap(l_cur, l_new, contrast, 0.0, 1.0))
}

// ---------------------------------------------------------------------------
// Windowing kernel
// ---------------------------------------------------------------------------

/// Precomputed squared-linear disc kernel with sparsity threshold.
/// Returned as a `(2k+1)²` row-major `Vec<f32>`.
fn build_windowing_kernel(cxy_window: u32) -> Vec<f32> {
    let side = (cxy_window as usize) * 2 + 1;
    let mut kernel = vec![0.0f32; side * side];
    let center = cxy_window as f32;
    for y in 0..side {
        for x in 0..side {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let mut w = (center - dist) / center;
            if w < 0.0 {
                continue;
            }
            w *= w;
            if w < 0.005 {
                continue;
            }
            kernel[y * side + x] = w;
        }
    }
    kernel
}

// ---------------------------------------------------------------------------
// Histograms
// ---------------------------------------------------------------------------

const NUM_BINS: usize = 256;

/// Per-site local histogram. `bins` is the weighted luminance histogram;
/// `cumsum[i] = sum(bins[0..=i])` precomputed once so per-pixel lookups
/// collapse from O(256) to O(1).
#[derive(Clone)]
struct Histogram {
    bins: Vec<f32>,   // NUM_BINS
    cumsum: Vec<f32>, // NUM_BINS
    /// Total weight (sum of all bin entries).
    total: f32,
    /// Mean luminance × 255.
    mean: f32,
    /// Fractional median bin index (0..=255).
    median: f32,
}

impl Histogram {
    fn empty() -> Self {
        Self {
            bins: vec![0.0; NUM_BINS],
            cumsum: vec![0.0; NUM_BINS],
            total: 0.0,
            mean: 0.0,
            median: 0.0,
        }
    }

    /// After bin accumulation, fill in derived stats (cumsum, mean, median).
    fn finalize(&mut self) {
        // Cumulative sum.
        let mut running = 0.0f32;
        for i in 0..NUM_BINS {
            running += self.bins[i];
            self.cumsum[i] = running;
        }
        // Median via cumulative walk to half-weight.
        let half = self.total * 0.5;
        let mut ibin = 0usize;
        let mut sum = 0.0f32;
        while ibin < NUM_BINS && sum + self.bins[ibin] < half {
            sum += self.bins[ibin];
            ibin += 1;
        }
        // Sub-bin fractional position, mirroring the C# formula.
        let after = sum + self.bins[ibin.min(NUM_BINS - 1)];
        if ibin > 0 && self.bins[ibin - 1] != 0.0 {
            self.median = (ibin as f32) - (after - half) / self.bins[ibin - 1];
        } else {
            self.median = (ibin as f32) - 0.5;
        }
        // `mean` was initially the weighted sum of L, divide and rescale.
        self.mean = if self.total > 0.0 {
            (self.mean / self.total) * 255.0
        } else {
            0.0
        };
    }
}

/// Grid of `(cx_histograms+1) × (cy_histograms+1)` histograms. The `+1`s are
/// an overshoot guard so the 4-histogram bilinear interpolation can read
/// `[xBlock+1, yBlock+1]` near the right/bottom edge without a bounds check.
struct HistogramGrid {
    cx_histograms: usize,
    cy_histograms: usize,
    histograms: Vec<Option<Histogram>>,
}

impl HistogramGrid {
    #[inline]
    fn get(&self, x_block: usize, y_block: usize) -> Option<&Histogram> {
        let idx = y_block * (self.cx_histograms + 1) + x_block;
        self.histograms[idx].as_ref()
    }
}

/// Build the grid of sparsely-spaced weighted histograms over the luminance
/// plane.
fn build_histogram_grid(
    l_src: &[f32],
    bin_map: &[u8],
    width: u32,
    height: u32,
    cxy_window: u32,
    cxy_block: u32,
    kernel: &[f32],
) -> HistogramGrid {
    let width_usize = width as usize;
    let kernel_side = (cxy_window as usize) * 2 + 1;
    let cx_histograms = width.div_ceil(cxy_block) as usize;
    let cy_histograms = height.div_ceil(cxy_block) as usize;
    let slots = (cx_histograms + 1) * (cy_histograms + 1);
    let mut histograms: Vec<Option<Histogram>> = (0..slots).map(|_| None).collect();

    for y_block in 0..cy_histograms {
        for x_block in 0..cx_histograms {
            let cx = x_block as i64 * cxy_block as i64;
            let cy = y_block as i64 * cxy_block as i64;
            let k = cxy_window as i64;

            // Window rect clipped to image bounds.
            let x_left = (cx - k).max(0);
            let y_top = (cy - k).max(0);
            let x_right = (cx + k).min(width as i64 - 1);
            let y_bottom = (cy + k).min(height as i64 - 1);

            let mut h = Histogram::empty();
            let mut weighted_l_sum = 0.0f32;

            let y_weight_start = (y_top - cy + k) as usize;
            for (y_rel, y_window) in (y_top..=y_bottom).enumerate() {
                let y_weight = y_weight_start + y_rel;
                for (x_rel, x_window) in (x_left..=x_right).enumerate() {
                    let x_weight = ((x_left - cx + k) as usize) + x_rel;
                    let w = kernel[y_weight * kernel_side + x_weight];
                    if w == 0.0 {
                        continue;
                    }
                    let pixel_idx = (y_window as usize) * width_usize + (x_window as usize);
                    let bin = bin_map[pixel_idx] as usize;
                    h.bins[bin] += w;
                    h.total += w;
                    weighted_l_sum += w * l_src[pixel_idx];
                }
            }
            if h.total <= 0.0 {
                continue;
            }
            // Stash the weighted sum in `mean` for now; `finalize()` rescales.
            h.mean = weighted_l_sum;
            h.finalize();

            let idx = y_block * (cx_histograms + 1) + x_block;
            histograms[idx] = Some(h);
        }
    }

    HistogramGrid {
        cx_histograms,
        cy_histograms,
        histograms,
    }
}

// ---------------------------------------------------------------------------
// Per-pixel normalization via 4-histogram bilinear interpolation
// ---------------------------------------------------------------------------

/// Fraction-from-black if we fully equalized each of the four neighboring
/// histograms, then bilinearly blended. Sub-bin precision adjusts the
/// contribution of the current bin by `1 - ibin/255`, mirroring the C#
/// `NormalizeValueVia4Histograms`.
#[allow(clippy::too_many_arguments)]
fn normalize_value_via_4_histograms(
    ibin_cur: usize,
    h1: &Histogram,
    h2: &Histogram,
    h3: &Histogram,
    h4: &Histogram,
    x_weight: f32,
    y_weight: f32,
) -> f32 {
    // O(1) per histogram thanks to the precomputed cumsum.
    let sub = 1.0 - (ibin_cur as f32) / 255.0;
    let s1 = h1.cumsum[ibin_cur] - h1.bins[ibin_cur] * sub;
    let s2 = h2.cumsum[ibin_cur] - h2.bins[ibin_cur] * sub;
    let s3 = h3.cumsum[ibin_cur] - h3.bins[ibin_cur] * sub;
    let s4 = h4.cumsum[ibin_cur] - h4.bins[ibin_cur] * sub;

    let f1 = if h1.total > 0.0 { s1 / h1.total } else { 0.0 };
    let f2 = if h2.total > 0.0 { s2 / h2.total } else { 0.0 };
    let f3 = if h3.total > 0.0 { s3 / h3.total } else { 0.0 };
    let f4 = if h4.total > 0.0 { s4 / h4.total } else { 0.0 };

    let fx1 = x_weight * f1 + (1.0 - x_weight) * f2;
    let fx2 = x_weight * f3 + (1.0 - x_weight) * f4;
    y_weight * fx1 + (1.0 - y_weight) * fx2
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Apply local-contrast luminance normalization to `source`, returning a new
/// FloatMap. See `docs/local-contrast-spec.md` §3 for the algorithm details.
///
/// **Phase B**: single-threaded, full-resolution, faithful to the C# original
/// modulo the two intentional deviations documented at the top of this file.
pub fn locally_normalize_luminance(source: &FloatMap, params: &Parameters) -> FloatMap {
    let width = source.width;
    let height = source.height;
    let n = (width as usize) * (height as usize);

    // --- Resolve auto parameters -------------------------------------------
    let mut cxy_window = params.cxy_window;
    if cxy_window == 0 {
        cxy_window = (width / 32).max(1);
    }
    let mut cxy_block = params.cxy_block;
    if cxy_block == 0 {
        cxy_block = cxy_window / 4;
    }
    if cxy_block < 2 {
        cxy_block = 2;
    }

    // --- Decompose to OkLCh -------------------------------------------------
    let mut l_src = vec![0.0f32; n];
    let mut c_src = vec![0.0f32; n];
    let mut h_src = vec![0.0f32; n];
    for i in 0..n {
        let (l, c, h) = color::srgb_to_oklch(source.r[i], source.g[i], source.b[i]);
        l_src[i] = l;
        c_src[i] = c;
        h_src[i] = h;
    }

    // --- Per-pixel bin map --------------------------------------------------
    let mut bin_map = vec![0u8; n];
    for i in 0..n {
        let b = (l_src[i] * 255.0).round();
        bin_map[i] = b.clamp(0.0, 255.0) as u8;
    }

    // --- Kernel + histogram grid -------------------------------------------
    let kernel = build_windowing_kernel(cxy_window);
    let grid = build_histogram_grid(
        &l_src, &bin_map, width, height, cxy_window, cxy_block, &kernel,
    );

    // --- Per-pixel normalization -------------------------------------------
    let mut l_dest = vec![0.0f32; n];
    let mut c_dest = c_src.clone();
    let cxy_block_f = cxy_block as f32;
    let width_usize = width as usize;
    let empty = Histogram::empty();

    for y_block in 0..grid.cy_histograms {
        for x_block in 0..grid.cx_histograms {
            let h1 = grid.get(x_block, y_block).unwrap_or(&empty);
            // Edge fallback: if a neighbor slot is empty, reuse the nearest
            // populated one, matching the C# null-fallback chain.
            let h2 = grid.get(x_block + 1, y_block).unwrap_or(h1);
            let h3 = grid.get(x_block, y_block + 1).unwrap_or(h1);
            let h4 = grid.get(x_block + 1, y_block + 1).unwrap_or(h3);

            let x_pixel_start = x_block * (cxy_block as usize);
            let y_pixel_start = y_block * (cxy_block as usize);
            for y_within in 0..(cxy_block as usize) {
                let y = y_pixel_start + y_within;
                if y >= height as usize {
                    break;
                }
                let y_weight = 1.0 - (y_within as f32) / cxy_block_f;
                for x_within in 0..(cxy_block as usize) {
                    let x = x_pixel_start + x_within;
                    if x >= width_usize {
                        break;
                    }
                    let x_weight = 1.0 - (x_within as f32) / cxy_block_f;
                    let px = y * width_usize + x;
                    let l_cur = l_src[px];
                    let ibin_cur = bin_map[px] as usize;

                    let frac_sum = normalize_value_via_4_histograms(
                        ibin_cur, h1, h2, h3, h4, x_weight, y_weight,
                    );

                    let bin_mean_x1 = h1.mean * x_weight + h2.mean * (1.0 - x_weight);
                    let bin_mean_x2 = h3.mean * x_weight + h4.mean * (1.0 - x_weight);
                    let bin_mean = bin_mean_x1 * y_weight + bin_mean_x2 * (1.0 - y_weight);

                    let bin_median_x1 = h1.median * x_weight + h2.median * (1.0 - x_weight);
                    let bin_median_x2 = h3.median * x_weight + h4.median * (1.0 - x_weight);
                    let bin_median = bin_median_x1 * y_weight + bin_median_x2 * (1.0 - y_weight);

                    let bin_ref = if params.use_median_for_contrast {
                        bin_median
                    } else {
                        bin_mean
                    };

                    // Standard contrast.
                    let mut l_contrast = contrast_std(bin_ref / 255.0, l_cur, params.contrast);
                    if params.use_document_contrast {
                        l_contrast = contrast_doc(
                            bin_ref / 255.0,
                            l_contrast,
                            params.mix_document_contrast,
                            params.tilt_black_doc_contrast,
                            params.tilt_white_doc_contrast,
                            params.apply_contrast_to_xition,
                            params.apply_contrast_to_bw,
                        );
                    }
                    if params.lighten_shadows != 0.0 && bin_ref <= 127.0 {
                        let frac_dark = 128.0 / (1.0 + bin_ref);
                        l_contrast = l_contrast * frac_dark * params.lighten_shadows
                            + l_contrast * (1.0 - params.lighten_shadows);
                    }
                    if params.darken_highlights != 0.0 && bin_ref >= 127.5 {
                        let frac_light = 127.5 / bin_ref;
                        l_contrast = l_contrast * frac_light * params.darken_highlights
                            + l_contrast * (1.0 - params.darken_highlights);
                    }

                    // Alpha-blend raw equalization with contrasted value.
                    let frac_alpha = params.alpha_white
                        + (params.alpha_black - params.alpha_white) * (1.0 - l_contrast);
                    let mut l_final = frac_sum * frac_alpha + l_contrast * (1.0 - frac_alpha);

                    // Debug visualizations (ReturnImage variants).
                    match params.return_image {
                        ReturnImage::Dsp => {}
                        ReturnImage::MedianGrayPointColored => l_final = bin_median / 255.0,
                        ReturnImage::MedianGrayPointLuminance => {
                            l_final = bin_median / 255.0;
                            c_dest[px] = 0.0;
                        }
                        ReturnImage::MeanGrayPointColored => l_final = bin_mean / 255.0,
                        ReturnImage::MeanGrayPointLuminance => {
                            l_final = bin_mean / 255.0;
                            c_dest[px] = 0.0;
                        }
                        ReturnImage::NormalizedValueColored => l_final = frac_sum,
                        ReturnImage::NormalizedValueLuminance => {
                            l_final = frac_sum;
                            c_dest[px] = 0.0;
                        }
                    }

                    l_dest[px] = l_final;
                    // (HSL desaturation heuristic from C# lines 494-500 omitted —
                    // Oklab chroma is stable near the luminance endpoints.)
                }
            }
        }
    }

    // --- Reconstitute sRGB -------------------------------------------------
    let mut dest = FloatMap {
        width,
        height,
        r: vec![0.0; n],
        g: vec![0.0; n],
        b: vec![0.0; n],
        a: source.a.clone(),
    };
    for i in 0..n {
        let (r, g, b) = color::oklch_to_srgb(l_dest[i], c_dest[i], h_src[i]);
        dest.r[i] = clamp01(r);
        dest.g[i] = clamp01(g);
        dest.b[i] = clamp01(b);
    }
    dest
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn filled_map(width: u32, height: u32, r: f32, g: f32, b: f32) -> FloatMap {
        let n = (width as usize) * (height as usize);
        FloatMap {
            width,
            height,
            r: vec![r; n],
            g: vec![g; n],
            b: vec![b; n],
            a: Some(vec![1.0; n]),
        }
    }

    fn max_abs_diff(a: &FloatMap, b: &FloatMap) -> f32 {
        let mut max: f32 = 0.0;
        for i in 0..a.pixel_count() {
            max = max.max((a.r[i] - b.r[i]).abs());
            max = max.max((a.g[i] - b.g[i]).abs());
            max = max.max((a.b[i] - b.b[i]).abs());
        }
        max
    }

    // --- Helper math --------------------------------------------------------

    #[test]
    fn apply_tilt_endpoints() {
        assert!((apply_tilt(0.5, 0.0) - 0.5).abs() < 1e-6);
        // Negative tilt pulls toward black.
        assert!(apply_tilt(0.5, -0.2) < 0.5);
        // Positive tilt pulls toward white.
        assert!(apply_tilt(0.5, 0.2) > 0.5);
    }

    #[test]
    fn contrast_std_at_gray_no_change_dark_side() {
        // When l_cur == l_gray, both branches produce l_cur.
        // Take the dark branch by using l_cur just below l_gray.
        let l = contrast_std(0.5, 0.5 - 1e-6, 1.0);
        assert!(l < 0.5);
    }

    #[test]
    fn contrast_doc_black_and_white_clamps() {
        // l_cur well below the (tilted) black threshold → returns 0 unless
        // apply_contrast_to_bw is on (then lerps toward 0).
        let v = contrast_doc(0.5, 0.01, 1.0, -0.2, -0.05, true, false);
        assert!(v.abs() < 1e-6);
        // Symmetrically, well above the (tilted) white threshold → 1.
        let v = contrast_doc(0.5, 0.99, 1.0, -0.2, -0.05, true, false);
        assert!((v - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_via_4_histograms_matches_naive() {
        // Build a small histogram, compare cumsum-based impl against the
        // naive summation the C# does.
        let mut h = Histogram::empty();
        for (i, w) in [
            (10usize, 1.0f32),
            (50, 2.0),
            (100, 3.0),
            (150, 2.0),
            (200, 1.0),
        ] {
            h.bins[i] = w;
            h.total += w;
        }
        h.mean = 100.0 * h.total; // arbitrary — only used downstream
        h.finalize();

        // Naive version for a couple of bins.
        for ibin_cur in [10usize, 75, 150, 255] {
            let mut naive_sum = 0.0;
            for i in 0..=ibin_cur {
                naive_sum += h.bins[i];
            }
            let sub = 1.0 - (ibin_cur as f32) / 255.0;
            naive_sum -= h.bins[ibin_cur] * sub;
            let naive_frac = naive_sum / h.total;

            // Our impl (via normalize_value_via_4_histograms with identical
            // histograms for all four corners → should return `naive_frac`).
            let got = normalize_value_via_4_histograms(ibin_cur, &h, &h, &h, &h, 0.5, 0.5);
            assert!(
                (got - naive_frac).abs() < 1e-5,
                "bin={} got={} expected={}",
                ibin_cur,
                got,
                naive_frac
            );
        }
    }

    // --- Algorithm end-to-end ----------------------------------------------

    #[test]
    fn uniform_input_stays_near_uniform() {
        // A perfectly uniform mid-gray has no local structure; the algorithm
        // should produce an output that is also roughly uniform at the same
        // shade. (It won't be exactly identical because the alpha-blended
        // `fracSum` term is 0.5 for all pixels when the histogram has only
        // one bin, shifting the output slightly.)
        let src = filled_map(64, 64, 0.5, 0.5, 0.5);
        let params = Parameters {
            cxy_window: 8,
            cxy_block: 2,
            ..Default::default()
        };
        let out = locally_normalize_luminance(&src, &params);
        // All output pixels should be equal (no spatial variation).
        let r0 = out.r[0];
        for i in 0..out.pixel_count() {
            assert!(
                (out.r[i] - r0).abs() < 1e-4,
                "pixel {} drifts: {} vs {}",
                i,
                out.r[i],
                r0
            );
            assert!((out.g[i] - r0).abs() < 1e-4);
            assert!((out.b[i] - r0).abs() < 1e-4);
        }
        // Output still near the original shade (within ~15% of L=0.5).
        assert!((r0 - 0.5).abs() < 0.15, "uniform drifted too far: {}", r0);
    }

    #[test]
    fn contrast_zero_produces_identity_like_output() {
        // With contrast = 0, lighten_shadows = 0, darken_highlights = 0, and
        // alpha_black = alpha_white = 0, the processed value should exactly
        // equal the input L (i.e. the full pipeline degenerates to identity
        // in Oklab space). The sRGB round-trip has its own < 1e-4 noise.
        let mut src = filled_map(32, 32, 0.0, 0.0, 0.0);
        for y in 0..32u32 {
            for x in 0..32u32 {
                let i = src.idx(x, y);
                let v = x as f32 / 31.0;
                src.r[i] = v;
                src.g[i] = v * 0.5;
                src.b[i] = 1.0 - v;
            }
        }
        let params = Parameters {
            cxy_window: 4,
            cxy_block: 2,
            contrast: 0.0,
            lighten_shadows: 0.0,
            darken_highlights: 0.0,
            alpha_black: 0.0,
            alpha_white: 0.0,
            ..Default::default()
        };
        let out = locally_normalize_luminance(&src, &params);
        let diff = max_abs_diff(&src, &out);
        assert!(diff < 1e-3, "identity pipeline drifted by {}", diff);
    }

    #[test]
    fn alpha_channel_preserved() {
        let mut src = filled_map(16, 16, 0.3, 0.3, 0.3);
        let alpha = src.a.as_mut().unwrap();
        for (i, v) in alpha.iter_mut().enumerate() {
            *v = (i % 4) as f32 / 4.0;
        }
        let saved = src.a.clone();
        let out = locally_normalize_luminance(&src, &Parameters::default());
        assert_eq!(out.a, saved);
    }
}
