//! Unified OKLab pipeline.  All stages operate on a shared `Rgba16Float`
//! OKLab working buffer; sRGB↔OKLab conversions happen exactly once each
//! (decode at start, encode at end).  Stages with `params: None` are skipped
//! entirely — no GPU dispatch, no extra buffer write — so the cost of a
//! "Vibrance only" run is exactly one filter dispatch plus the unavoidable
//! decode/encode bookends.
//!
//! Pipeline order is fixed: **LC → BC → Vibrance(+Saturation) → Hue →
//! Equalize**.  The order matters a little (Saturation runs after Vibrance
//! so the chroma-weighted vibrance scaling sees pre-saturation chroma
//! magnitudes; Equalize runs last so the histogram is built on the final
//! perceptual L after every other adjustment, which is what the user sees);
//! tying it to a stable order keeps presets reproducible.
//!
//! Equalize is the only stage that needs a CPU readback mid-pipeline: pass 1
//! builds a 256-bin histogram of OKLab L into an atomic storage buffer, the
//! CPU normalizes it into a 256-element CDF, and pass 2 looks up the CDF
//! per pixel and blends with the original L by `Amount`.  Whenever Equalize
//! is enabled, `process_pipeline` submits the encoder twice (once before
//! the readback, once after) — every other path stays single-submit.

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
}

impl Default for LcParams {
    fn default() -> Self {
        Self {
            radius: 60.0,
            strength: 0.5,
        }
    }
}

impl LcParams {
    pub fn is_identity(&self) -> bool {
        self.strength.abs() < 0.0005
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqualizeParams {
    /// Flat blend between the original L and the histogram-equalized L, applied
    /// uniformly across all tones.  0 = no change, 1 = full equalization.
    pub amount: f32,
    /// Extra equalization blended in proportion to how *dark* the source pixel
    /// is (full weight at pure black, fading linearly to none at pure white).
    /// Magnifies subtle differences in the shadows.  0–1, default 0.  This is
    /// the GPU analogue of the CPU local-contrast "Alpha of pure Black".
    pub shadows: f32,
    /// Extra equalization blended in proportion to how *bright* the source
    /// pixel is (full weight at pure white, fading linearly to none at pure
    /// black).  Magnifies subtle differences in the highlights.  0–1,
    /// default 0.  GPU analogue of the CPU "Alpha of pure White".
    pub highlights: f32,
}

impl Default for EqualizeParams {
    fn default() -> Self {
        Self {
            amount: 0.5,
            shadows: 0.0,
            highlights: 0.0,
        }
    }
}

impl EqualizeParams {
    pub fn is_identity(&self) -> bool {
        self.amount.abs() < 0.0005
            && self.shadows.abs() < 0.0005
            && self.highlights.abs() < 0.0005
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
    pub equalize: Option<EqualizeParams>,
}

impl Default for UnifiedParams {
    fn default() -> Self {
        Self {
            resize_factor: 1.0,
            lc: None,
            bc: None,
            vibrance: None,
            hue: None,
            equalize: None,
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
            && self.equalize.is_none_or(|p| p.is_identity())
    }
}

// --- Uniform buffer layouts (16-byte aligned) -----------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LcUniforms {
    radius: f32,
    strength: f32,
    axis: f32,
    image_width: f32,
    image_height: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EqualizeUniforms {
    amount: f32,
    shadows: f32,
    highlights: f32,
    _pad: f32,
}

/// Uniforms for both Lanczos passes (H and V share the layout — only the
/// shader differs).  `dst_w`/`dst_h` are the destination dimensions for
/// bounds checking; `src_filter_dim` is the source size along the axis
/// being filtered (`src_w` for H, `src_h` for V); `filter_scale` is
/// `max(1, src_filter_dim / dst_filter_dim)` so the kernel widens for
/// downscaling.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LanczosUniforms {
    dst_w: f32,
    dst_h: f32,
    src_filter_dim: f32,
    filter_scale: f32,
}

// --- Lazy pipeline cache --------------------------------------------------

fn lc_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| {
        pipeline::build_stage_pipeline(ctx, "lc", include_str!("shaders/local_contrast.wgsl"))
    })
}
fn bc_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| pipeline::build_stage_pipeline(ctx, "bc", include_str!("shaders/bc.wgsl")))
}
fn vibrance_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| {
        pipeline::build_stage_pipeline(ctx, "vibrance", include_str!("shaders/vibrance.wgsl"))
    })
}
fn hue_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| pipeline::build_stage_pipeline(ctx, "hue", include_str!("shaders/hue.wgsl")))
}
fn lanczos_h_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| {
        pipeline::build_stage_pipeline(ctx, "lanczos h", include_str!("shaders/lanczos_h.wgsl"))
    })
}
fn lanczos_v_oklab_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| {
        pipeline::build_stage_pipeline_with_oklab(
            ctx,
            "lanczos v oklab",
            include_str!("shaders/lanczos_v_oklab.wgsl"),
        )
    })
}

fn histogram_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| {
        pipeline::build_pipeline_with_layout(
            ctx,
            "equalize histogram",
            include_str!("shaders/equalize_histogram.wgsl"),
            pipeline::histogram_layout(ctx),
        )
    })
}

fn equalize_apply_pipeline(ctx: &GpuContext) -> &'static wgpu::ComputePipeline {
    static P: OnceLock<wgpu::ComputePipeline> = OnceLock::new();
    P.get_or_init(|| {
        pipeline::build_pipeline_with_layout(
            ctx,
            "equalize apply",
            include_str!("shaders/equalize_apply.wgsl"),
            pipeline::equalize_apply_layout(ctx),
        )
    })
}

/// Image-size-independent buffers used by the equalize stage.  Three small
/// buffers (256 × 4 = 1 KB each) cached for the lifetime of the GPU
/// context — the equalize stage refills them per call via
/// `queue.write_buffer` and a `copy_buffer_to_buffer` for the readback.
struct EqualizeBuffers {
    histogram: wgpu::Buffer,
    histogram_readback: wgpu::Buffer,
    cdf: wgpu::Buffer,
}

fn equalize_buffers(ctx: &GpuContext) -> &'static EqualizeBuffers {
    static B: OnceLock<EqualizeBuffers> = OnceLock::new();
    B.get_or_init(|| EqualizeBuffers {
        histogram: pipeline::make_histogram_buffer(ctx),
        histogram_readback: pipeline::make_histogram_readback(ctx),
        cdf: pipeline::make_cdf_buffer(ctx),
    })
}

/// Read 256 × u32 from `readback` and turn it into a 256-element CDF in
/// [0, 1].  Empty histogram → identity CDF (`i / 255`), so an Equalize
/// dispatch on a transparent / empty buffer is a no-op rather than NaN.
fn read_cdf(ctx: &GpuContext, readback: &wgpu::Buffer) -> Result<[f32; 256], GpuError> {
    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    ctx.device
        .poll(wgpu::PollType::Wait)
        .map_err(|e| GpuError::DevicePoll(format!("{e:?}")))?;
    rx.recv()
        .map_err(|e| GpuError::BufferMap(format!("recv: {e}")))?
        .map_err(|e| GpuError::BufferMap(format!("map: {e:?}")))?;

    let mapped = slice.get_mapped_range();
    let mut hist = [0u32; 256];
    for (i, chunk) in mapped.chunks_exact(4).take(256).enumerate() {
        hist[i] = u32::from_le_bytes(chunk.try_into().expect("4-byte chunk"));
    }
    drop(mapped);
    readback.unmap();

    let total: u64 = hist.iter().map(|&v| u64::from(v)).sum();
    if total == 0 {
        return Ok(std::array::from_fn(|i| i as f32 / 255.0));
    }
    let mut cdf = [0.0f32; 256];
    let mut cumulative: u64 = 0;
    for i in 0..256 {
        cumulative += u64::from(hist[i]);
        cdf[i] = cumulative as f32 / total as f32;
    }
    // Hard-anchor the endpoints: rescale so cdf[0] → 0 and cdf[255] (always
    // 1.0) → 1.  Without this, a large mass of pure-black pixels makes cdf[0]
    // large and equalization lifts black toward grey — exactly what the user
    // wants to avoid.  With it, pure black stays black and pure white stays
    // white, while the *occupied* tonal range in between is stretched, so
    // subtle differences are magnified rather than the endpoints shifted.
    let cdf0 = cdf[0];
    let denom = 1.0 - cdf0;
    if denom < 1e-6 {
        // Essentially every pixel is at pure black — nothing to redistribute.
        return Ok(std::array::from_fn(|i| i as f32 / 255.0));
    }
    for v in cdf.iter_mut() {
        *v = ((*v - cdf0) / denom).clamp(0.0, 1.0);
    }
    Ok(cdf)
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

    // Resize is now done on the GPU (separable Lanczos-3, see lanczos_h.wgsl
    // and lanczos_v_oklab.wgsl).  When `resize` is true, the H pass writes
    // into the cached intermediate texture at `(out_w, src_h)` and the V
    // pass folds the linear→OKLab decode into its second sweep — so on the
    // resize path the standalone `decode_oklab` dispatch is skipped.  When
    // `resize` is false, source dims equal output dims and the original
    // `encode_decode` runs straight from source into `buf_a`.
    let (out_w, out_h) = if resize {
        let w2 = ((width as f32) * params.resize_factor).round().max(1.0) as u32;
        let h2 = ((height as f32) * params.resize_factor).round().max(1.0) as u32;
        (w2, h2)
    } else {
        (width, height)
    };

    let ctx = get_context().ok_or(GpuError::NoAdapter)?;

    // Defensive: refuse outputs that would exceed the device's texture-dim
    // ceiling.  Without this, `create_texture` panics on validation failure
    // deep inside the rayon worker (e.g. 4× upscale of a multi-MP image
    // pushes past 16384 even on Apple-M-series headroom).  Surfaces as
    // `Err` so the worker logs and skips the install — no crash.
    let max_dim = ctx.device.limits().max_texture_dimension_2d;
    let largest = width.max(height).max(out_w).max(out_h);
    if largest > max_dim {
        return Err(GpuError::OutputTooLarge {
            width: out_w,
            height: out_h,
            max: max_dim,
        });
    }

    // Three cached LRUs (source / intermediate / output), see `cache.rs`.
    let bgra = cache::with_textures(
        ctx,
        width,
        height,
        out_w,
        out_h,
        resize,
        |source, intermediate, outputs| -> Result<Vec<u8>, GpuError> {
            pipeline::write_source_srgb(ctx, source, rgba, width, height);

            let mut encoder = ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("rpview-gpu unified encoder"),
                });

            // Front-end: source → buf_a (in OKLab).  Resize path runs
            // separable Lanczos-3 with linear→OKLab folded into the V
            // pass; non-resize path uses the original sRGB→OKLab decode.
            if let Some(intermediate) = intermediate {
                let h_filter_scale = (width as f32 / out_w as f32).max(1.0);
                let h_uniforms = LanczosUniforms {
                    dst_w: out_w as f32,
                    dst_h: height as f32,
                    src_filter_dim: width as f32,
                    filter_scale: h_filter_scale,
                };
                let h_buf = pipeline::make_uniform_buffer(ctx, bytemuck::bytes_of(&h_uniforms));
                pipeline::encode_stage(
                    ctx,
                    &mut encoder,
                    lanczos_h_pipeline(ctx),
                    source,
                    intermediate,
                    &h_buf,
                    out_w,
                    height,
                    "lanczos h",
                );

                let v_filter_scale = (height as f32 / out_h as f32).max(1.0);
                let v_uniforms = LanczosUniforms {
                    dst_w: out_w as f32,
                    dst_h: out_h as f32,
                    src_filter_dim: height as f32,
                    filter_scale: v_filter_scale,
                };
                let v_buf = pipeline::make_uniform_buffer(ctx, bytemuck::bytes_of(&v_uniforms));
                pipeline::encode_stage(
                    ctx,
                    &mut encoder,
                    lanczos_v_oklab_pipeline(ctx),
                    intermediate,
                    &outputs.buf_a,
                    &v_buf,
                    out_w,
                    out_h,
                    "lanczos v oklab",
                );
            } else {
                pipeline::encode_decode(ctx, &mut encoder, source, &outputs.buf_a, out_w, out_h);
            }
            let mut current_in_a = true;

            // Each ping-pong helper picks the right (src, dst) pair given which
            // buffer currently holds the live OKLab data.
            let pingpong = |current_in_a: bool| -> (&wgpu::Texture, &wgpu::Texture) {
                if current_in_a {
                    (&outputs.buf_a, &outputs.buf_b)
                } else {
                    (&outputs.buf_b, &outputs.buf_a)
                }
            };

            // Stage: LC (two ping-pong dispatches).
            if let Some(lc) = params.lc
                && !lc.is_identity()
            {
                let make_u = |axis: f32| LcUniforms {
                    radius: lc.radius,
                    strength: lc.strength,
                    axis,
                    image_width: out_w as f32,
                    image_height: out_h as f32,
                    _pad0: 0.0,
                    _pad1: 0.0,
                    _pad2: 0.0,
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

            // Stage: Equalize (last — operates on the final perceptual L).
            // Splits the encoder around a CPU readback: pass 1 builds a
            // histogram, the CPU normalizes it into a CDF, pass 2 applies
            // it.  When equalize is disabled this whole block is skipped
            // and the single-encoder fast path remains.
            if let Some(eq) = params.equalize
                && !eq.is_identity()
            {
                let bufs = equalize_buffers(ctx);
                ctx.queue.write_buffer(&bufs.histogram, 0, &[0u8; 256 * 4]);

                let (src_for_hist, _) = pingpong(current_in_a);
                pipeline::encode_histogram(
                    ctx,
                    &mut encoder,
                    histogram_pipeline(ctx),
                    src_for_hist,
                    &bufs.histogram,
                    out_w,
                    out_h,
                );
                encoder.copy_buffer_to_buffer(
                    &bufs.histogram,
                    0,
                    &bufs.histogram_readback,
                    0,
                    256 * 4,
                );
                ctx.queue.submit(Some(encoder.finish()));

                let cdf = read_cdf(ctx, &bufs.histogram_readback)?;
                ctx.queue
                    .write_buffer(&bufs.cdf, 0, bytemuck::cast_slice(&cdf));

                encoder = ctx
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("rpview-gpu unified encoder (post-equalize)"),
                    });

                let u = EqualizeUniforms {
                    amount: eq.amount,
                    shadows: eq.shadows,
                    highlights: eq.highlights,
                    _pad: 0.0,
                };
                let uniform_buf = pipeline::make_uniform_buffer(ctx, bytemuck::bytes_of(&u));
                let (src, dst) = pingpong(current_in_a);
                pipeline::encode_equalize_apply(
                    ctx,
                    &mut encoder,
                    equalize_apply_pipeline(ctx),
                    src,
                    dst,
                    &bufs.cdf,
                    &uniform_buf,
                    out_w,
                    out_h,
                );
                current_in_a = !current_in_a;
            }

            // Encode: current OKLab buffer → BGRA output.
            let (current, _) = pingpong(current_in_a);
            pipeline::encode_encode(ctx, &mut encoder, current, &outputs.output, out_w, out_h);

            ctx.queue.submit(Some(encoder.finish()));
            readback::read_into(ctx, &outputs.output, &outputs.readback, out_w, out_h)
        },
    )?;
    Ok((bgra, out_w, out_h))
}
