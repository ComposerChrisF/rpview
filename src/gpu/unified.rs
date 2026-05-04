//! Unified OKLab pipeline.  All stages operate on a shared `Rgba16Float`
//! OKLab working buffer; sRGB↔OKLab conversions happen exactly once each
//! (decode at start, encode at end).  Stages with `params: None` are skipped
//! entirely — no GPU dispatch, no extra buffer write — so the cost of a
//! "Vibrance only" run is exactly one filter dispatch plus the unavoidable
//! decode/encode bookends.
//!
//! Pipeline order is fixed: **LC → BC → Vibrance(+Saturation) → Hue**.  The
//! order matters a little (Saturation runs after Vibrance so the
//! chroma-weighted vibrance scaling sees pre-saturation chroma magnitudes);
//! tying it to a stable order keeps presets reproducible.
//!
//! Equalize is reserved as a future stage; it isn't part of this struct yet
//! because its histogram-build pass hasn't shipped.

use std::sync::OnceLock;

use bytemuck::{Pod, Zeroable};

use crate::gpu::cache;
use crate::gpu::device::{GpuContext, GpuError, get_context};
use crate::gpu::{pipeline, readback};

// --- Per-stage param types ------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LcParams {
    /// Spatial scale of the local neighborhood, in pixels.  4–200, default 60.
    pub radius: f32,
    /// Overall intensity. 0–2, default 0.5.
    pub strength: f32,
    /// Brighten dark regions toward midpoint.  0–1.
    pub shadow_lift: f32,
    /// Darken bright regions toward midpoint.  0–1.
    pub highlight_darken: f32,
    /// Gray-point pivot.  0.1–0.9, default 0.5.
    pub midpoint: f32,
}

impl Default for LcParams {
    fn default() -> Self {
        Self {
            radius: 60.0,
            strength: 0.5,
            shadow_lift: 0.0,
            highlight_darken: 0.0,
            midpoint: 0.5,
        }
    }
}

impl LcParams {
    pub fn is_identity(&self) -> bool {
        self.strength.abs() < 0.0005
            && self.shadow_lift.abs() < 0.0005
            && self.highlight_darken.abs() < 0.0005
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BcParams {
    /// Brightness: additive shift on OKLab L.  −1 … +1.
    pub brightness: f32,
    /// Contrast: scale L around `midpoint`.  −1 … +2.
    pub contrast: f32,
    /// Pivot for Contrast scaling.  Conceptually the same value as the LC
    /// stage's Midpoint — the controls layer wires both to one slider in
    /// the LC section so the user has a single knob.  Defaults to 0.5
    /// (perceptual middle-grey) when constructed standalone.
    pub midpoint: f32,
}

impl Default for BcParams {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 0.0,
            midpoint: 0.5,
        }
    }
}

impl BcParams {
    pub fn is_identity(&self) -> bool {
        self.brightness.abs() < 0.0005 && self.contrast.abs() < 0.0005
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VibranceParams {
    /// Asymmetric saturation booster.  −1 (cut bright colors) … +1 (lift
    /// muted ones).
    pub amount: f32,
    /// Uniform saturation scale, applied AFTER vibrance.  Scales OKLab chroma
    /// magnitude.  −1 = greyscale, 0 = no change, +1 = double.
    pub saturation: f32,
}

impl Default for VibranceParams {
    fn default() -> Self {
        Self {
            amount: 0.5,
            saturation: 0.0,
        }
    }
}

impl VibranceParams {
    pub fn is_identity(&self) -> bool {
        self.amount.abs() < 0.0005 && self.saturation.abs() < 0.0005
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HueParams {
    /// Hue rotation, 0.0–1.0 (wraps).  0.5 = 180°.
    pub hue: f32,
}

impl Default for HueParams {
    fn default() -> Self {
        Self { hue: 0.0 }
    }
}

impl HueParams {
    pub fn is_identity(&self) -> bool {
        self.hue.abs() < 0.0005 || (self.hue - 1.0).abs() < 0.0005
    }
}

// --- The unified-pipeline struct ------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct UnifiedParams {
    /// Pre-process resize factor.  `1.0` = full resolution.  `0.5` halves
    /// each dimension before the GPU pipeline (Lanczos/bilinear CPU resize),
    /// and the returned BGRA buffer stays at the smaller dimensions so the
    /// caller's display layer scales it.
    pub resize_factor: f32,
    /// `Some(...)` enables the stage; `None` skips it entirely.
    pub lc: Option<LcParams>,
    pub bc: Option<BcParams>,
    pub vibrance: Option<VibranceParams>,
    pub hue: Option<HueParams>,
}

impl Default for UnifiedParams {
    fn default() -> Self {
        Self {
            resize_factor: 1.0,
            lc: None,
            bc: None,
            vibrance: None,
            hue: None,
        }
    }
}

impl UnifiedParams {
    /// Returns `true` when every enabled stage is at its identity value
    /// (and no stage is enabled with non-trivial params).  The caller can
    /// short-circuit and avoid GPU work in this case.
    pub fn is_identity(&self) -> bool {
        self.lc.is_none_or(|p| p.is_identity())
            && self.bc.is_none_or(|p| p.is_identity())
            && self.vibrance.is_none_or(|p| p.is_identity())
            && self.hue.is_none_or(|p| p.is_identity())
    }
}

// --- Uniform buffer layouts (16-byte aligned) -----------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LcUniforms {
    radius: f32,
    strength: f32,
    shadow_lift: f32,
    highlight_darken: f32,
    midpoint: f32,
    axis: f32,
    image_width: f32,
    image_height: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BcUniforms {
    brightness: f32,
    contrast: f32,
    midpoint: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VibranceUniforms {
    amount: f32,
    saturation: f32,
    _pad0: f32,
    _pad1: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct HueUniforms {
    hue: f32,
    _pad: [f32; 3],
}

// --- Lazy pipeline cache --------------------------------------------------

fn lc_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| pipeline::build_stage_pipeline(ctx, "lc", include_str!("shaders/local_contrast.wgsl")))
}
fn bc_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| pipeline::build_stage_pipeline(ctx, "bc", include_str!("shaders/bc.wgsl")))
}
fn vibrance_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| pipeline::build_stage_pipeline(ctx, "vibrance", include_str!("shaders/vibrance.wgsl")))
}
fn hue_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| pipeline::build_stage_pipeline(ctx, "hue", include_str!("shaders/hue.wgsl")))
}

// --- CPU bilinear pre-resize ----------------------------------------------

fn resize_rgba8_bilinear(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    let x_ratio = src_w as f32 / dst_w as f32;
    let y_ratio = src_h as f32 / dst_h as f32;
    for y in 0..dst_h {
        let sy = (y as f32 + 0.5) * y_ratio - 0.5;
        let sy0 = sy.floor().max(0.0) as u32;
        let sy1 = (sy0 + 1).min(src_h - 1);
        let fy = (sy - sy0 as f32).clamp(0.0, 1.0);
        for x in 0..dst_w {
            let sx = (x as f32 + 0.5) * x_ratio - 0.5;
            let sx0 = sx.floor().max(0.0) as u32;
            let sx1 = (sx0 + 1).min(src_w - 1);
            let fx = (sx - sx0 as f32).clamp(0.0, 1.0);
            let mut px = [0u8; 4];
            for c in 0..4usize {
                let i00 = ((sy0 * src_w + sx0) as usize) * 4 + c;
                let i01 = ((sy0 * src_w + sx1) as usize) * 4 + c;
                let i10 = ((sy1 * src_w + sx0) as usize) * 4 + c;
                let i11 = ((sy1 * src_w + sx1) as usize) * 4 + c;
                let p00 = src[i00] as f32;
                let p01 = src[i01] as f32;
                let p10 = src[i10] as f32;
                let p11 = src[i11] as f32;
                let top = p00 + fx * (p01 - p00);
                let bot = p10 + fx * (p11 - p10);
                px[c] = (top + fy * (bot - top)).round().clamp(0.0, 255.0) as u8;
            }
            let i = ((y * dst_w + x) * 4) as usize;
            out[i..i + 4].copy_from_slice(&px);
        }
    }
    out
}

fn rgba_to_bgra_passthrough(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|p| [p[2], p[1], p[0], p[3]])
        .collect()
}

// --- The orchestrator -----------------------------------------------------

/// Run the unified OKLab pipeline.  `rgba` is RGBA8 source bytes.  Returns
/// BGRA8 output bytes plus the actual output dimensions (which may differ
/// from input when `resize_factor != 1.0`).
///
/// If every stage is disabled (or each enabled stage is at identity values)
/// **and** no resize is requested, this short-circuits to a CPU
/// channel-swap and never touches the GPU.
pub fn process_pipeline(
    rgba: &[u8],
    width: u32,
    height: u32,
    params: &UnifiedParams,
) -> Result<(Vec<u8>, u32, u32), GpuError> {
    let resize = (params.resize_factor - 1.0).abs() >= 0.001;
    if !resize && params.is_identity() {
        return Ok((rgba_to_bgra_passthrough(rgba), width, height));
    }

    // Optional CPU pre-resize.
    let owned;
    let (rgba_resized, out_w, out_h) = if resize {
        let w2 = ((width as f32) * params.resize_factor).round().max(1.0) as u32;
        let h2 = ((height as f32) * params.resize_factor).round().max(1.0) as u32;
        owned = resize_rgba8_bilinear(rgba, width, height, w2, h2);
        (owned.as_slice(), w2, h2)
    } else {
        (rgba, width, height)
    };

    let ctx = get_context().ok_or(GpuError::NoAdapter)?;

    // The four textures + readback buffer come from a `(w, h)`-keyed LRU,
    // so the typical workflow (one preview-resize image at a time) hits the
    // cache after the first call and skips the alloc entirely.
    let bgra = cache::with_textures(
        ctx,
        out_w,
        out_h,
        |textures| -> Result<Vec<u8>, GpuError> {
            pipeline::write_source_srgb(ctx, &textures.source, rgba_resized, out_w, out_h);

            let mut encoder = ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("rpview-gpu unified encoder"),
                });

            // Decode: source → buf_a.  After this, "current" is buf_a.
            pipeline::encode_decode(
                ctx,
                &mut encoder,
                &textures.source,
                &textures.buf_a,
                out_w,
                out_h,
            );
            let mut current_in_a = true;

            // Each ping-pong helper picks the right (src, dst) pair given which
            // buffer currently holds the live OKLab data.
            let pingpong = |current_in_a: bool| -> (&wgpu::Texture, &wgpu::Texture) {
                if current_in_a {
                    (&textures.buf_a, &textures.buf_b)
                } else {
                    (&textures.buf_b, &textures.buf_a)
                }
            };

            // Stage: LC (two ping-pong dispatches).
            if let Some(lc) = params.lc
                && !lc.is_identity()
            {
                let make_u = |axis: f32| LcUniforms {
                    radius: lc.radius,
                    strength: lc.strength,
                    shadow_lift: lc.shadow_lift,
                    highlight_darken: lc.highlight_darken,
                    midpoint: lc.midpoint,
                    axis,
                    image_width: out_w as f32,
                    image_height: out_h as f32,
                };
                let u0 = pipeline::make_uniform_buffer(ctx, bytemuck::bytes_of(&make_u(0.0)));
                let u1 = pipeline::make_uniform_buffer(ctx, bytemuck::bytes_of(&make_u(1.0)));
                let pl = lc_pipeline(ctx);

                let (src, dst) = pingpong(current_in_a);
                pipeline::encode_stage(
                    ctx,
                    &mut encoder,
                    pl,
                    src,
                    dst,
                    &u0,
                    out_w,
                    out_h,
                    "lc pass1",
                );
                current_in_a = !current_in_a;

                let (src, dst) = pingpong(current_in_a);
                pipeline::encode_stage(
                    ctx,
                    &mut encoder,
                    pl,
                    src,
                    dst,
                    &u1,
                    out_w,
                    out_h,
                    "lc pass2",
                );
                current_in_a = !current_in_a;
            }

            // Stage: BC (Brightness/Contrast).
            if let Some(bc) = params.bc
                && !bc.is_identity()
            {
                let u = BcUniforms {
                    brightness: bc.brightness,
                    contrast: bc.contrast,
                    midpoint: bc.midpoint,
                    _pad: 0.0,
                };
                let buf = pipeline::make_uniform_buffer(ctx, bytemuck::bytes_of(&u));
                let (src, dst) = pingpong(current_in_a);
                pipeline::encode_stage(
                    ctx,
                    &mut encoder,
                    bc_pipeline(ctx),
                    src,
                    dst,
                    &buf,
                    out_w,
                    out_h,
                    "bc",
                );
                current_in_a = !current_in_a;
            }

            // Stage: Vibrance (with merged Saturation).
            if let Some(v) = params.vibrance
                && !v.is_identity()
            {
                let u = VibranceUniforms {
                    amount: v.amount,
                    saturation: v.saturation,
                    _pad0: 0.0,
                    _pad1: 0.0,
                };
                let buf = pipeline::make_uniform_buffer(ctx, bytemuck::bytes_of(&u));
                let (src, dst) = pingpong(current_in_a);
                pipeline::encode_stage(
                    ctx,
                    &mut encoder,
                    vibrance_pipeline(ctx),
                    src,
                    dst,
                    &buf,
                    out_w,
                    out_h,
                    "vibrance",
                );
                current_in_a = !current_in_a;
            }

            // Stage: Hue.
            if let Some(h) = params.hue
                && !h.is_identity()
            {
                let u = HueUniforms {
                    hue: h.hue,
                    _pad: [0.0; 3],
                };
                let buf = pipeline::make_uniform_buffer(ctx, bytemuck::bytes_of(&u));
                let (src, dst) = pingpong(current_in_a);
                pipeline::encode_stage(
                    ctx,
                    &mut encoder,
                    hue_pipeline(ctx),
                    src,
                    dst,
                    &buf,
                    out_w,
                    out_h,
                    "hue",
                );
                current_in_a = !current_in_a;
            }

            // Encode: current OKLab buffer → BGRA output.
            let (current, _) = pingpong(current_in_a);
            pipeline::encode_encode(ctx, &mut encoder, current, &textures.output, out_w, out_h);

            ctx.queue.submit(Some(encoder.finish()));
            readback::read_into(ctx, &textures.output, &textures.readback, out_w, out_h)
        },
    )?;
    Ok((bgra, out_w, out_h))
}
