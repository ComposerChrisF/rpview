//! Texture allocation cache for the unified pipeline.
//!
//! Three independent LRUs, all keyed on `(u32, u32)`:
//!
//! - **`sources`** keyed on `(src_w, src_h)` — the sRGB upload target.
//!   Survives resize-factor scrubbing on a single image (source dims stay
//!   constant, only the output side changes).
//! - **`intermediates`** keyed on `(dst_w, src_h)` — the H-Lanczos pass
//!   output / V-Lanczos pass input.  Sized differently from both source
//!   and final output so it gets its own slot.  Skipped entirely when
//!   `resize_factor == 1.0` (the no-resize path runs `decode_oklab.wgsl`
//!   straight from source to `buf_a`).
//! - **`outputs`** keyed on `(out_w, out_h)` — the OKLab ping-pong pair,
//!   the final BGRA output, and the readback buffer.  These all share
//!   the post-resize dimensions.
//!
//! Splitting the cache by purpose means resize-factor scrubbing (same
//! source, different outputs) doesn't duplicate the source texture, and
//! image switches (different sources) don't blow away the output set if
//! the new image happens to land at the same `(out_w, out_h)`.
//!
//! Capacity 4 per LRU.  Realistic working set: animation playback at one
//! resize factor (1 of each), or resize-factor scrubbing across 1×/½×/¼×
//! on one image (1 source, 3 intermediates, 3 outputs).
//!
//! Concurrency: the v0.22.3 GPU worker pattern guarantees at most one
//! `process_pipeline` call in flight, so the `Mutex` is never contended in
//! practice.  Single lock, three disjoint field borrows inside.  Held for
//! the duration of `with_textures`, which spans the GPU dispatch and
//! readback.  Fine while the single-worker invariant holds.
//!
//! Reuse safety: every entry is write-before-read within a single pipeline
//! run.  `source` gets a fresh `queue.write_texture`; `intermediate` is
//! filled by lanczos_h before lanczos_v reads it; `buf_a/b` are written by
//! decode/lanczos_v_oklab/stage passes before any read; `output` is
//! written by encode; `readback` is `COPY_DST` only.  No clear step needed.

use std::sync::Mutex;

use crate::gpu::device::GpuContext;
use crate::gpu::pipeline;
use crate::gpu::readback;

const CACHE_CAPACITY: usize = 4;

pub struct OutputSet {
    pub buf_a: wgpu::Texture,
    pub buf_b: wgpu::Texture,
    pub output: wgpu::Texture,
    pub readback: wgpu::Buffer,
}

impl OutputSet {
    fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
        Self {
            buf_a: pipeline::make_oklab_buffer(ctx, width, height),
            buf_b: pipeline::make_oklab_buffer(ctx, width, height),
            output: pipeline::make_bgra_output(ctx, width, height),
            readback: readback::make_readback_buffer(ctx, width, height),
        }
    }
}

struct TextureCache {
    sources: Vec<((u32, u32), wgpu::Texture)>,
    intermediates: Vec<((u32, u32), wgpu::Texture)>,
    outputs: Vec<((u32, u32), OutputSet)>,
}

impl TextureCache {
    const fn new() -> Self {
        Self {
            sources: Vec::new(),
            intermediates: Vec::new(),
            outputs: Vec::new(),
        }
    }
}

/// Generic LRU touch-or-insert.  Same shape as the previous
/// `get_or_make` — entries Vec ordered LRU-first → MRU-last; on hit move
/// the entry to the MRU end, on miss evict the LRU front when full and
/// push new.  Returns `&V` borrowed from the just-touched/inserted entry.
fn touch_or_insert<V>(
    entries: &mut Vec<((u32, u32), V)>,
    key: (u32, u32),
    make: impl FnOnce() -> V,
) -> &V {
    if let Some(pos) = entries.iter().position(|(k, _)| *k == key) {
        if pos + 1 != entries.len() {
            let entry = entries.remove(pos);
            entries.push(entry);
        }
    } else {
        if entries.len() >= CACHE_CAPACITY {
            entries.remove(0);
        }
        entries.push((key, make()));
    }
    &entries.last().expect("entries non-empty after touch/insert").1
}

static TEXTURE_CACHE: Mutex<TextureCache> = Mutex::new(TextureCache::new());

/// Run `f` with cached textures sized for one `process_pipeline` call.
///
/// `resize` controls whether an intermediate texture for the H-Lanczos
/// pass is requested:
/// - `false` → `intermediate` argument to `f` is `None`; the caller runs
///   the standalone `decode_oklab` pass straight from `source` to `buf_a`.
/// - `true` → `intermediate` is `Some(_)` sized at `(out_w, src_h)`.
pub fn with_textures<R>(
    ctx: &GpuContext,
    src_w: u32,
    src_h: u32,
    out_w: u32,
    out_h: u32,
    resize: bool,
    f: impl FnOnce(&wgpu::Texture, Option<&wgpu::Texture>, &OutputSet) -> R,
) -> R {
    let mut guard = TEXTURE_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // Reborrow so the three field accesses below are split-borrows of
    // disjoint fields rather than three reborrows of `*guard`.
    let cache = &mut *guard;

    let source = touch_or_insert(&mut cache.sources, (src_w, src_h), || {
        pipeline::make_source_srgb(ctx, src_w, src_h)
    });
    let intermediate = if resize {
        Some(touch_or_insert(
            &mut cache.intermediates,
            (out_w, src_h),
            || pipeline::make_oklab_buffer(ctx, out_w, src_h),
        ))
    } else {
        None
    };
    let output = touch_or_insert(&mut cache.outputs, (out_w, out_h), || {
        OutputSet::new(ctx, out_w, out_h)
    });

    f(source, intermediate, output)
}
