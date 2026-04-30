//! Port of `FraleyMusic.ImageDsp.AdaptiveContrastDsp.LocallyNormalizeLuminance`.
//!
//! OkLCh-based (swap for HSL from the C# original). Parallelized across
//! histogram-block rows via rayon. See `docs/local-contrast-spec.md` §3 for
//! the algorithm narrative.
//!
//! Intentional deviations from the C# source:
//! - Color space is OkLCh, not HSL. The scalar L is Oklab's perceptual L.
//! - The §3.4 step 11 "desaturate near extremes" heuristic is omitted — it's
//!   an HSL workaround; Oklab chroma degrades gracefully at the endpoints.
//! - Cumulative-sum per histogram is precomputed once (O(256) per site),
//!   turning each per-pixel histogram lookup from O(256) to O(1).
//!
//! All other behavior — including the known `Contrast_Std` quirk on the
//! white-side branch (mixed 0..1 / 0..255 scale) — is preserved verbatim
//! for parity with C# reference outputs.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::utils::color;
use crate::utils::float_map::FloatMap;

// ---------------------------------------------------------------------------
// Feedback / cancellation
// ---------------------------------------------------------------------------

/// Progress + cancellation callback. Called from worker threads, so it must
/// be `Send + Sync`. Return `true` to request cancellation; the algorithm
/// stops at the next safe point and returns `None`.
pub type FeedbackFn = dyn Fn(f32, &str) -> bool + Send + Sync;

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

/// What image to produce — `Dsp` is the normal processed output; the other
/// variants expose intermediate visualizations useful for tuning defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// Pre-process resize factor (Lanczos3). `1.0` = no resize. Values like
    /// `0.5` halve each dimension before running LC and upscale the result
    /// back to the original size on the way out — useful for previewing
    /// expensive parameter combinations at a fraction of the cost.
    pub resize_factor: f32,
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
            resize_factor: 1.0,
        }
    }
}

impl Parameters {
    /// Returns `true` when every parameter that influences the output is at
    /// its neutral value. Used to short-circuit processing when the user has
    /// dialed everything to zero.
    pub fn is_identity(&self) -> bool {
        self.contrast.abs() < 0.001
            && self.lighten_shadows.abs() < 0.001
            && self.darken_highlights.abs() < 0.001
            && self.alpha_black.abs() < 0.001
            && self.alpha_white.abs() < 0.001
            && !self.use_document_contrast
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
///
/// `bins` and `cumsum` are fixed-size 256-element arrays so allocating a
/// `Histogram` (~16k of them per LC pass on a 4K image) is allocator-free.
#[derive(Clone)]
struct Histogram {
    bins: [f32; NUM_BINS],
    cumsum: [f32; NUM_BINS],
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
            bins: [0.0; NUM_BINS],
            cumsum: [0.0; NUM_BINS],
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

/// Compute a single histogram site. Extracted so both the serial and
/// parallel grid builders can share the inner math.
#[allow(clippy::too_many_arguments)]
fn compute_histogram_at(
    x_block: usize,
    y_block: usize,
    l_src: &[f32],
    bin_map: &[u8],
    width: u32,
    height: u32,
    cxy_window: u32,
    cxy_block: u32,
    kernel: &[f32],
) -> Option<Histogram> {
    let width_usize = width as usize;
    let kernel_side = (cxy_window as usize) * 2 + 1;
    let cx = x_block as i64 * cxy_block as i64;
    let cy = y_block as i64 * cxy_block as i64;
    let k = cxy_window as i64;

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
        return None;
    }
    h.mean = weighted_l_sum; // finalize() rescales
    h.finalize();
    Some(h)
}

/// Build the grid of sparsely-spaced weighted histograms over the luminance
/// plane. Parallelized over histogram-block rows.
#[allow(clippy::too_many_arguments)]
fn build_histogram_grid(
    l_src: &[f32],
    bin_map: &[u8],
    width: u32,
    height: u32,
    cxy_window: u32,
    cxy_block: u32,
    kernel: &[f32],
    progress: &ParallelProgress<'_>,
) -> Option<HistogramGrid> {
    let cx_histograms = width.div_ceil(cxy_block) as usize;
    let cy_histograms = height.div_ceil(cxy_block) as usize;
    let row_len = cx_histograms + 1;
    let total_rows = cy_histograms + 1; // +1 for overshoot guard row (stays empty)
    let mut histograms: Vec<Option<Histogram>> = (0..row_len * total_rows).map(|_| None).collect();

    histograms
        .par_chunks_mut(row_len)
        .take(cy_histograms)
        .enumerate()
        .for_each(|(y_block, row)| {
            if progress.is_cancelled() {
                return;
            }
            for (x_block, slot) in row.iter_mut().enumerate().take(cx_histograms) {
                *slot = compute_histogram_at(
                    x_block, y_block, l_src, bin_map, width, height, cxy_window, cxy_block, kernel,
                );
            }
            progress.report_row(cy_histograms, 0.05, 0.30, "Building histograms...");
        });

    if progress.is_cancelled() {
        return None;
    }
    Some(HistogramGrid {
        cx_histograms,
        cy_histograms,
        histograms,
    })
}

// ---------------------------------------------------------------------------
// Progress tracking helper
// ---------------------------------------------------------------------------

/// Thread-safe progress + cancellation tracker. Serializes calls to the
/// user-provided feedback callback by gating on a `Mutex<usize>` threshold.
struct ParallelProgress<'a> {
    completed: AtomicUsize,
    next_report: Mutex<usize>,
    report_every: usize,
    cancel: AtomicBool,
    feedback: Option<&'a FeedbackFn>,
}

impl<'a> ParallelProgress<'a> {
    fn new(total_ticks: usize, feedback: Option<&'a FeedbackFn>) -> Self {
        let report_every = ((total_ticks / 100).max(1)).min(total_ticks.max(1));
        Self {
            completed: AtomicUsize::new(0),
            next_report: Mutex::new(report_every),
            report_every,
            cancel: AtomicBool::new(false),
            feedback,
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Report completion of one unit of work. `progress_min` / `progress_max`
    /// give the algorithmic phase's slice of the 0..1 progress bar.
    fn report_row(&self, total: usize, progress_min: f32, progress_max: f32, message: &str) {
        let done = self.completed.fetch_add(1, Ordering::Relaxed) + 1;
        let Some(fb) = self.feedback else {
            return;
        };
        // Serialize callback invocations: only the thread that crosses the
        // current threshold fires the callback and advances it.
        let mut threshold = self.next_report.lock().unwrap();
        if done < *threshold {
            return;
        }
        *threshold = done + self.report_every;
        drop(threshold);
        let frac = (done as f32 / total as f32).min(1.0);
        let progress = progress_min + frac * (progress_max - progress_min);
        if fb(progress, message) {
            self.cancel();
        }
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
// Fast-path helpers (Phase E)
// ---------------------------------------------------------------------------

/// Apply the contrast / document-contrast / shadow / highlight tone curve to a
/// single pixel's luminance, given the local gray-point reference. Shared
/// between the faithful and fast paths so both do exactly the same tone
/// shaping — they only differ in how `bin_ref` is computed.
///
/// `bin_ref` is in the 0..255 scale (compatible with the 127 / 127.5
/// thresholds the C# original compares against), `l_cur` is in `[0, 1]`.
#[inline]
fn apply_tone_curve(l_cur: f32, bin_ref: f32, params: &Parameters) -> f32 {
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
    l_contrast
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Apply local-contrast luminance normalization to `source`.
///
/// Returns `None` if `feedback` requested cancellation at any checkpoint.
/// Otherwise returns the processed image. See `docs/local-contrast-spec.md`
/// §3 for algorithmic details. Parallelized across histogram-block rows via
/// rayon. Progress is reported to `feedback` at ~1% granularity; returning
/// `true` from the callback stops processing at the next safe checkpoint.
pub fn locally_normalize_luminance(
    source: &FloatMap,
    params: &Parameters,
    feedback: Option<&FeedbackFn>,
) -> Option<FloatMap> {
    // Optional pre-process resize. We resize the input and run the
    // algorithm on it; we deliberately do **not** resize the result back
    // up to the original dimensions — the renderer already scales the
    // LC `RenderImage` to fit the displayed image bounds on the GPU, so
    // an extra Lanczos pass on the way out would just do the same scaling
    // more slowly and worse. (At 0.25× or 0.5×, GPU upscales our small
    // result for display; at 2× or 4×, GPU downscales — extra detail is
    // visible only when the user zooms in.)
    if (params.resize_factor - 1.0).abs() > 0.001 {
        if let Some(fb) = feedback
            && fb(0.02, "Resizing…")
        {
            return None;
        }
        let new_w = ((source.width as f32) * params.resize_factor)
            .round()
            .max(1.0) as u32;
        let new_h = ((source.height as f32) * params.resize_factor)
            .round()
            .max(1.0) as u32;
        let resized = source.resize_lanczos3(new_w, new_h);
        let mut inner = params.clone();
        inner.resize_factor = 1.0;
        return locally_normalize_luminance(&resized, &inner, feedback);
    }

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

    if let Some(fb) = feedback
        && fb(0.01, "Setting up...")
    {
        return None;
    }

    // --- Decompose to OkLCh (parallel per-pixel) ---------------------------
    let mut l_src = vec![0.0f32; n];
    let mut c_src = vec![0.0f32; n];
    let mut h_src = vec![0.0f32; n];
    (
        l_src.par_iter_mut(),
        c_src.par_iter_mut(),
        h_src.par_iter_mut(),
        source.r.par_iter(),
        source.g.par_iter(),
        source.b.par_iter(),
    )
        .into_par_iter()
        .for_each(|(l, c, h, &r, &g, &b)| {
            let (ll, cc, hh) = color::srgb_to_oklch(r, g, b);
            *l = ll;
            *c = cc;
            *h = hh;
        });

    // --- Per-pixel bin map (parallel) --------------------------------------
    let bin_map: Vec<u8> = l_src
        .par_iter()
        .map(|&l| (l * 255.0).round().clamp(0.0, 255.0) as u8)
        .collect();

    // --- Kernel + histogram grid -------------------------------------------
    let kernel = build_windowing_kernel(cxy_window);
    let cy_histograms = height.div_ceil(cxy_block) as usize;
    let progress = ParallelProgress::new(cy_histograms, feedback);

    let grid = build_histogram_grid(
        &l_src, &bin_map, width, height, cxy_window, cxy_block, &kernel, &progress,
    )?;

    // --- Per-pixel normalization (parallel across block rows) --------------
    let mut l_dest = vec![0.0f32; n];
    let mut c_dest = c_src.clone();
    let cxy_block_f = cxy_block as f32;
    let width_usize = width as usize;
    let block_row_pixels = (cxy_block as usize) * width_usize;
    let return_image = params.return_image;
    let use_median = params.use_median_for_contrast;
    let alpha_black = params.alpha_black;
    let alpha_white = params.alpha_white;
    let cx_histograms = grid.cx_histograms;
    let empty = Histogram::empty();

    // Phase 2 progress: 0.30 .. 0.90
    let progress2 = ParallelProgress::new(grid.cy_histograms, feedback);

    l_dest
        .par_chunks_mut(block_row_pixels)
        .zip(c_dest.par_chunks_mut(block_row_pixels))
        .take(grid.cy_histograms)
        .enumerate()
        .for_each(|(y_block, (l_row, c_row))| {
            if progress2.is_cancelled() {
                return;
            }
            let y_pixel_start = y_block * (cxy_block as usize);
            // This row covers y ∈ [y_pixel_start, y_pixel_start + cxy_block),
            // clipped to the image height at the very last row.
            let rows_in_block = (l_row.len() / width_usize).min(cxy_block as usize);

            for x_block in 0..cx_histograms {
                let h1 = grid.get(x_block, y_block).unwrap_or(&empty);
                let h2 = grid.get(x_block + 1, y_block).unwrap_or(h1);
                let h3 = grid.get(x_block, y_block + 1).unwrap_or(h1);
                let h4 = grid.get(x_block + 1, y_block + 1).unwrap_or(h3);

                let x_pixel_start = x_block * (cxy_block as usize);
                for y_within in 0..rows_in_block {
                    let y = y_pixel_start + y_within;
                    if y >= height as usize {
                        break;
                    }
                    let y_weight = 1.0 - (y_within as f32) / cxy_block_f;
                    let row_offset_in_chunk = y_within * width_usize;
                    for x_within in 0..(cxy_block as usize) {
                        let x = x_pixel_start + x_within;
                        if x >= width_usize {
                            break;
                        }
                        let x_weight = 1.0 - (x_within as f32) / cxy_block_f;
                        let px_global = y * width_usize + x;
                        let px_local = row_offset_in_chunk + x;
                        let l_cur = l_src[px_global];
                        let ibin_cur = bin_map[px_global] as usize;

                        let frac_sum = normalize_value_via_4_histograms(
                            ibin_cur, h1, h2, h3, h4, x_weight, y_weight,
                        );

                        let bin_mean_x1 = h1.mean * x_weight + h2.mean * (1.0 - x_weight);
                        let bin_mean_x2 = h3.mean * x_weight + h4.mean * (1.0 - x_weight);
                        let bin_mean = bin_mean_x1 * y_weight + bin_mean_x2 * (1.0 - y_weight);

                        let bin_median_x1 = h1.median * x_weight + h2.median * (1.0 - x_weight);
                        let bin_median_x2 = h3.median * x_weight + h4.median * (1.0 - x_weight);
                        let bin_median =
                            bin_median_x1 * y_weight + bin_median_x2 * (1.0 - y_weight);

                        let bin_ref = if use_median { bin_median } else { bin_mean };

                        let l_contrast = apply_tone_curve(l_cur, bin_ref, params);
                        let frac_alpha =
                            alpha_white + (alpha_black - alpha_white) * (1.0 - l_contrast);
                        let mut l_final = frac_sum * frac_alpha + l_contrast * (1.0 - frac_alpha);

                        match return_image {
                            ReturnImage::Dsp => {}
                            ReturnImage::MedianGrayPointColored => l_final = bin_median / 255.0,
                            ReturnImage::MedianGrayPointLuminance => {
                                l_final = bin_median / 255.0;
                                c_row[px_local] = 0.0;
                            }
                            ReturnImage::MeanGrayPointColored => l_final = bin_mean / 255.0,
                            ReturnImage::MeanGrayPointLuminance => {
                                l_final = bin_mean / 255.0;
                                c_row[px_local] = 0.0;
                            }
                            ReturnImage::NormalizedValueColored => l_final = frac_sum,
                            ReturnImage::NormalizedValueLuminance => {
                                l_final = frac_sum;
                                c_row[px_local] = 0.0;
                            }
                        }

                        l_row[px_local] = l_final;
                    }
                }
            }
            progress2.report_row(grid.cy_histograms, 0.30, 0.90, "Processing image...");
        });

    if progress2.is_cancelled() {
        return None;
    }

    if let Some(fb) = feedback
        && fb(0.92, "Converting to RGB...")
    {
        return None;
    }

    // --- Reconstitute sRGB (parallel per-pixel) ----------------------------
    let mut dest_r = vec![0.0f32; n];
    let mut dest_g = vec![0.0f32; n];
    let mut dest_b = vec![0.0f32; n];
    (
        dest_r.par_iter_mut(),
        dest_g.par_iter_mut(),
        dest_b.par_iter_mut(),
        l_dest.par_iter(),
        c_dest.par_iter(),
        h_src.par_iter(),
    )
        .into_par_iter()
        .for_each(|(dr, dg, db, &l, &c, &h)| {
            let (r, g, b) = color::oklch_to_srgb(l, c, h);
            *dr = clamp01(r);
            *dg = clamp01(g);
            *db = clamp01(b);
        });

    Some(FloatMap {
        width,
        height,
        r: dest_r,
        g: dest_g,
        b: dest_b,
        a: source.a.clone(),
    })
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
        let out = locally_normalize_luminance(&src, &params, None).expect("not cancelled");
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
        let out = locally_normalize_luminance(&src, &params, None).expect("not cancelled");
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
        let out =
            locally_normalize_luminance(&src, &Parameters::default(), None).expect("not cancelled");
        assert_eq!(out.a, saved);
    }

    // --- Parallel / cancellation behaviors ---------------------------------

    /// Build a small image with spatial variation so parallel and serial
    /// paths have non-trivial work to do.
    fn gradient_map(width: u32, height: u32) -> FloatMap {
        let n = (width as usize) * (height as usize);
        let mut m = FloatMap {
            width,
            height,
            r: vec![0.0; n],
            g: vec![0.0; n],
            b: vec![0.0; n],
            a: Some(vec![1.0; n]),
        };
        for y in 0..height {
            for x in 0..width {
                let i = m.idx(x, y);
                m.r[i] = x as f32 / (width - 1) as f32;
                m.g[i] = y as f32 / (height - 1) as f32;
                m.b[i] = 0.5;
            }
        }
        m
    }

    #[test]
    fn parallel_outputs_match_repeated_runs() {
        // Rayon parallelism with float accumulation can reorder sums across
        // threads. Since each histogram site is computed entirely by one
        // worker and each output pixel only reads the (now-final) histograms,
        // results should be bit-identical run-to-run. Verify.
        let src = gradient_map(48, 32);
        let params = Parameters::default();
        let a = locally_normalize_luminance(&src, &params, None).unwrap();
        let b = locally_normalize_luminance(&src, &params, None).unwrap();
        assert_eq!(a.r, b.r);
        assert_eq!(a.g, b.g);
        assert_eq!(a.b, b.b);
    }

    #[test]
    fn cancellation_returns_none() {
        // Callback returns true immediately — algorithm should bail out.
        let src = gradient_map(48, 32);
        let fb: Box<FeedbackFn> = Box::new(|_, _| true);
        let out = locally_normalize_luminance(&src, &Parameters::default(), Some(&*fb));
        assert!(out.is_none());
    }

    #[test]
    fn progress_callback_is_called_with_increasing_values() {
        use std::sync::{Arc, Mutex};
        let src = gradient_map(48, 32);
        let seen: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let seen_cb = Arc::clone(&seen);
        let fb: Box<FeedbackFn> = Box::new(move |p, _msg| {
            seen_cb.lock().unwrap().push(p);
            false
        });
        let out = locally_normalize_luminance(&src, &Parameters::default(), Some(&*fb));
        assert!(out.is_some());
        let values = seen.lock().unwrap().clone();
        assert!(!values.is_empty(), "feedback never called");
        // Progress starts low and ends high.
        assert!(values[0] <= 0.1);
        assert!(values.iter().cloned().fold(0.0f32, f32::max) >= 0.90);
    }
}
