//! GPU pixel-shader filter pipeline.
//!
//! Single unified compute pipeline ported from PixelShaderPaint3
//! (`/Users/chris/Chris/Proj/PixelShaderPaint3`).  Public entry point is
//! [`process_pipeline`], which takes RGBA8 source bytes plus a
//! [`UnifiedParams`] (per-stage params, each `Option`-wrapped to enable/
//! disable) and returns BGRA8 bytes ready to wrap in `gpui::RenderImage`.
//!
//! Color flow:
//!   sRGB RGBA8 → linear (auto, via Rgba8UnormSrgb sample)
//!              → OKLab rgba16float (decode pass, once)
//!              → enabled stages, in fixed order, all in OKLab
//!              → linear sRGB → sRGB BGRA rgba8unorm (encode pass, once)
//!              → readback to Vec<u8>
//!
//! Pipeline order: **LC → SBC → Vibrance → Hue**.
//!
//! On systems without a compatible wgpu adapter, [`process_pipeline`] returns
//! `Err(GpuError::NoAdapter)`.  Callers should grey out the corresponding
//! UI / fall back to CPU paths where they exist.

mod cache;
pub mod device;
mod pipeline;
mod readback;
pub mod unified;

#[allow(unused_imports)] // Public surface for the not-yet-wired UI layer.
pub use device::{GpuContext, GpuError, get_context};
#[allow(unused_imports)] // Public surface for the not-yet-wired UI layer.
pub use unified::{
    BcParams, EqualizeParams, HueParams, LcParams, UnifiedParams, VibranceParams, process_pipeline,
};

/// Shared OKLab + sRGB↔linear helpers, prepended to every shader that needs
/// them (no `#include` in WGSL).  Source: PSP3 `shader-includes/oklab.wgsl`.
pub(crate) const OKLAB_WGSL: &str = include_str!("shaders/oklab.wgsl");

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Vec<u8>, u32, u32) {
        let w = 4u32;
        let h = 4u32;
        let mut bytes = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                bytes.push((40 + x * 30) as u8);
                bytes.push((60 + y * 25) as u8);
                bytes.push((80 + (x + y) * 15) as u8);
                bytes.push(255);
            }
        }
        (bytes, w, h)
    }

    fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
        rgba.chunks_exact(4)
            .flat_map(|p| [p[2], p[1], p[0], p[3]])
            .collect()
    }

    fn assert_close(actual: &[u8], expected: &[u8], tol: u8, label: &str) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "{label}: length mismatch ({} vs {})",
            actual.len(),
            expected.len()
        );
        let mut max_diff = 0u8;
        let mut worst_idx = 0usize;
        for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
            let d = a.abs_diff(*e);
            if d > max_diff {
                max_diff = d;
                worst_idx = i;
            }
        }
        assert!(
            max_diff <= tol,
            "{label}: max byte diff {max_diff} exceeds tolerance {tol} (idx {worst_idx}, \
             actual={}, expected={})",
            actual[worst_idx],
            expected[worst_idx]
        );
    }

    fn require_gpu() -> bool {
        if get_context().is_some() {
            true
        } else {
            eprintln!("[skip] no GPU adapter available");
            false
        }
    }

    /// All-disabled, no-resize: short-circuits to CPU passthrough — no GPU needed.
    #[test]
    fn empty_pipeline_passthrough_no_gpu() {
        let (rgba, w, h) = fixture();
        let (out, ow, oh) = process_pipeline(&rgba, w, h, &UnifiedParams::default()).unwrap();
        assert_eq!((ow, oh), (w, h));
        assert_eq!(out, rgba_to_bgra(&rgba));
    }

    /// Vibrance enabled at amount=0 still runs the full GPU path
    /// (decode → vibrance → encode).  Round-trip should match input within
    /// sRGB-quantize + OKLab-roundtrip noise.
    #[test]
    fn vibrance_zero_runs_full_pipeline() {
        if !require_gpu() {
            return;
        }
        let (rgba, w, h) = fixture();
        let params = UnifiedParams {
            vibrance: Some(VibranceParams {
                amount: 0.0,
                saturation: 0.0,
            }),
            ..Default::default()
        };
        let (out, ow, oh) = process_pipeline(&rgba, w, h, &params).unwrap();
        assert_eq!((ow, oh), (w, h));
        assert_close(&out, &rgba_to_bgra(&rgba), 2, "vibrance amount=0");
    }

    #[test]
    fn bc_zero_runs_full_pipeline() {
        if !require_gpu() {
            return;
        }
        let (rgba, w, h) = fixture();
        let params = UnifiedParams {
            bc: Some(BcParams::default()),
            ..Default::default()
        };
        let (out, _, _) = process_pipeline(&rgba, w, h, &params).unwrap();
        assert_close(&out, &rgba_to_bgra(&rgba), 2, "bc all-zero");
    }

    #[test]
    fn hue_zero_runs_full_pipeline() {
        if !require_gpu() {
            return;
        }
        let (rgba, w, h) = fixture();
        let params = UnifiedParams {
            hue: Some(HueParams { hue: 0.0 }),
            ..Default::default()
        };
        let (out, _, _) = process_pipeline(&rgba, w, h, &params).unwrap();
        assert_close(&out, &rgba_to_bgra(&rgba), 2, "hue=0");
    }

    #[test]
    fn lc_zero_strength_runs_full_pipeline() {
        if !require_gpu() {
            return;
        }
        let w = 32u32;
        let h = 32u32;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                rgba.push((30 + x * 5) as u8);
                rgba.push((50 + y * 4) as u8);
                rgba.push((90 + (x ^ y) * 3) as u8);
                rgba.push(255);
            }
        }
        let params = UnifiedParams {
            lc: Some(LcParams {
                strength: 0.0,
                ..Default::default()
            }),
            ..Default::default()
        };
        let (out, ow, oh) = process_pipeline(&rgba, w, h, &params).unwrap();
        assert_eq!((ow, oh), (w, h));
        assert_close(&out, &rgba_to_bgra(&rgba), 3, "lc strength=0");
    }

    #[test]
    fn resize_factor_changes_dimensions() {
        if !require_gpu() {
            return;
        }
        let w = 64u32;
        let h = 48u32;
        let rgba = vec![128u8; (w * h * 4) as usize];
        let params = UnifiedParams {
            resize_factor: 0.5,
            // Need at least one stage enabled or the empty-pipeline branch
            // would skip GPU and return at original dims.  Use vibrance at
            // identity — pipeline runs but math is no-op.
            vibrance: Some(VibranceParams {
                amount: 0.0,
                saturation: 0.0,
            }),
            ..Default::default()
        };
        let (_, ow, oh) = process_pipeline(&rgba, w, h, &params).unwrap();
        assert_eq!((ow, oh), (32, 24));
    }

    /// All four stages chained together.  Output should be deterministic and
    /// the right size; we don't pin the pixel values, just verify it runs.
    #[test]
    fn all_stages_chained() {
        if !require_gpu() {
            return;
        }
        let (rgba, w, h) = fixture();
        let params = UnifiedParams {
            lc: Some(LcParams::default()),
            bc: Some(BcParams {
                brightness: 0.05,
                contrast: 0.1,
                midpoint: 0.5,
            }),
            vibrance: Some(VibranceParams {
                amount: 0.3,
                saturation: 0.2,
            }),
            hue: Some(HueParams { hue: 0.05 }),
            equalize: Some(EqualizeParams {
                amount: 0.4,
                ..Default::default()
            }),
            ..Default::default()
        };
        let (out, ow, oh) = process_pipeline(&rgba, w, h, &params).unwrap();
        assert_eq!((ow, oh), (w, h));
        assert_eq!(out.len(), (w * h * 4) as usize);
    }

    /// Equalize at amount=0 is a luminance no-op — the histogram pass still
    /// runs (so the encoder split is exercised) but the apply pass blends
    /// 100% of the original L back in.  Round-trip should match input
    /// within sRGB-quantize + OKLab-roundtrip noise.
    #[test]
    fn equalize_zero_runs_full_pipeline() {
        if !require_gpu() {
            return;
        }
        let (rgba, w, h) = fixture();
        let params = UnifiedParams {
            equalize: Some(EqualizeParams {
                amount: 0.0,
                ..Default::default()
            }),
            ..Default::default()
        };
        let (out, ow, oh) = process_pipeline(&rgba, w, h, &params).unwrap();
        assert_eq!((ow, oh), (w, h));
        // amount=0 hits the is_identity short-circuit and never dispatches
        // — but the rest of the pipeline still runs and the OKLab round-trip
        // alone introduces a few units of noise.
        assert_close(&out, &rgba_to_bgra(&rgba), 2, "equalize amount=0");
    }

    /// Equalize at amount=1 on a 32×32 ramp produces output of the right
    /// shape and size.  We don't pin pixel values (the CDF depends on
    /// histogram details) but the GPU path must complete without error.
    #[test]
    fn equalize_full_amount_produces_output() {
        if !require_gpu() {
            return;
        }
        let w = 32u32;
        let h = 32u32;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let v = (x * 8 + y) as u8;
                rgba.push(v);
                rgba.push(v);
                rgba.push(v);
                rgba.push(255);
            }
        }
        let params = UnifiedParams {
            equalize: Some(EqualizeParams {
                amount: 1.0,
                ..Default::default()
            }),
            ..Default::default()
        };
        let (out, ow, oh) = process_pipeline(&rgba, w, h, &params).unwrap();
        assert_eq!((ow, oh), (w, h));
        assert_eq!(out.len(), (w * h * 4) as usize);
    }

    /// LC CLAHE (shadows/highlights) drives the three-pass local-histogram
    /// path: tile histogram → contrast-limited CDF → bilinear apply.  On a
    /// gradient the path must complete and return the right shape.  Strength
    /// is 0 so only the CLAHE sub-stage runs.
    #[test]
    fn lc_clahe_shadows_produces_output() {
        if !require_gpu() {
            return;
        }
        let w = 96u32;
        let h = 96u32;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let v = ((x + y) * 255 / (w + h)) as u8;
                rgba.push(v);
                rgba.push(v);
                rgba.push(v);
                rgba.push(255);
            }
        }
        let params = UnifiedParams {
            lc: Some(LcParams {
                radius: 32.0,
                strength: 0.0,
                shadows: 1.0,
                highlights: 0.0,
                ..Default::default()
            }),
            ..Default::default()
        };
        let (out, ow, oh) = process_pipeline(&rgba, w, h, &params).unwrap();
        assert_eq!((ow, oh), (w, h));
        assert_eq!(out.len(), (w * h * 4) as usize);
    }

    /// LC Document-Style Contrast drives its own three-pass local-histogram
    /// path: tile histogram → per-tile mean/median stats → bilinear apply of
    /// contrast_std + contrast_doc.  On a gradient the path must complete and
    /// return the right shape.  Strength and CLAHE are off so only the doc
    /// sub-stage runs.
    #[test]
    fn lc_doc_contrast_produces_output() {
        if !require_gpu() {
            return;
        }
        let w = 96u32;
        let h = 96u32;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let v = ((x + y) * 255 / (w + h)) as u8;
                rgba.push(v);
                rgba.push(v);
                rgba.push(v);
                rgba.push(255);
            }
        }
        let params = UnifiedParams {
            lc: Some(LcParams {
                radius: 32.0,
                strength: 0.0,
                shadows: 0.0,
                highlights: 0.0,
                doc_enabled: true,
                doc_mix: 1.0,
                ..Default::default()
            }),
            ..Default::default()
        };
        let (out, ow, oh) = process_pipeline(&rgba, w, h, &params).unwrap();
        assert_eq!((ow, oh), (w, h));
        assert_eq!(out.len(), (w * h * 4) as usize);
    }
}
