//! Texture allocation cache for the unified pipeline.
//!
//! Every `process_pipeline` call needs four textures sized to the output
//! `(width, height)`: the sRGB source upload target, two `Rgba16Float` OKLab
//! ping-pong buffers, and the BGRA output target.  Plus a same-sized readback
//! buffer.  At preview resize factors a single set runs ~30–60 MB; at full-res
//! 24 MP it's ~600 MB.  Allocating that on every slider tick and dropping it
//! on the way out shows up as a real cost once async dispatch (v0.22.3) takes
//! the GPU work off the main thread and the alloc dominates what's left.
//!
//! Strategy: a tiny LRU keyed on `(width, height)`.  All five entries share
//! that key — formats and usages are constant in this module's universe —
//! so one lookup suffices for the whole set.  Capacity 4 covers the common
//! workloads (animation playback at one resize factor; user toggling between
//! 1×, ½×, ¼× preview; navigating between 2–3 differently-sized images at
//! one resize factor).  At capacity the LRU entry gets dropped, which
//! releases its GPU memory.
//!
//! Concurrency: the v0.22.3 GPU worker pattern guarantees at most one
//! `process_pipeline` call in flight, so the `Mutex` is never contended in
//! practice — but it lets the door stay open for concurrent callers later.
//! The lock is held for the duration of `with_textures`, which spans the
//! GPU dispatch and readback (tens of ms); fine while the single-worker
//! invariant holds.

use std::sync::Mutex;

use crate::gpu::device::GpuContext;
use crate::gpu::pipeline;
use crate::gpu::readback;

/// LRU capacity in distinct `(width, height)` keys.  See module docs for
/// rationale.
const CACHE_CAPACITY: usize = 4;

/// All resources a single `process_pipeline` call needs, sized for one
/// `(width, height)`.  Formats and usages are fixed:
///
/// - `source`:   `Rgba8UnormSrgb`, `TEXTURE_BINDING | COPY_DST`
/// - `buf_a/b`:  `Rgba16Float`,    `TEXTURE_BINDING | STORAGE_BINDING`
/// - `output`:   `Rgba8Unorm`,     `STORAGE_BINDING | COPY_SRC`
/// - `readback`: row-aligned `width × height × 4`, `COPY_DST | MAP_READ`
///
/// All entries are write-before-read within a single pipeline run — `source`
/// gets a fresh upload, `buf_a/b` are written by the decode/stage passes
/// before any read, `output` is written by the encode pass, `readback` is a
/// COPY_DST sink — so reuse is safe without any clear step.
pub struct TextureSet {
    pub source: wgpu::Texture,
    pub buf_a: wgpu::Texture,
    pub buf_b: wgpu::Texture,
    pub output: wgpu::Texture,
    pub readback: wgpu::Buffer,
}

impl TextureSet {
    fn new(ctx: &GpuContext, width: u32, height: u32) -> Self {
        Self {
            source: pipeline::make_source_srgb(ctx, width, height),
            buf_a: pipeline::make_oklab_buffer(ctx, width, height),
            buf_b: pipeline::make_oklab_buffer(ctx, width, height),
            output: pipeline::make_bgra_output(ctx, width, height),
            readback: readback::make_readback_buffer(ctx, width, height),
        }
    }
}

struct TextureCache {
    /// `(key, set)` ordered LRU-first → MRU-last.  Vec is fine at capacity 4
    /// — `position`, `remove`, and `push` are all O(n) over a 4-element span.
    entries: Vec<((u32, u32), TextureSet)>,
}

impl TextureCache {
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn get_or_make(&mut self, ctx: &GpuContext, w: u32, h: u32) -> &TextureSet {
        let key = (w, h);
        if let Some(pos) = self.entries.iter().position(|(k, _)| *k == key) {
            // Hit: move to MRU end so LRU eviction picks the right entry next.
            if pos + 1 != self.entries.len() {
                let entry = self.entries.remove(pos);
                self.entries.push(entry);
            }
        } else {
            if self.entries.len() >= CACHE_CAPACITY {
                self.entries.remove(0);
            }
            self.entries.push((key, TextureSet::new(ctx, w, h)));
        }
        &self
            .entries
            .last()
            .expect("entries non-empty after touch/insert")
            .1
    }
}

static TEXTURE_CACHE: Mutex<TextureCache> = Mutex::new(TextureCache::new());

/// Run `f` with a `TextureSet` sized to `(width, height)`.  Reuses the same
/// allocations across calls at the same size; LRU-evicts at capacity.  The
/// cache mutex is held for the whole call, which is fine under the single-
/// in-flight-worker invariant.
pub fn with_textures<R>(
    ctx: &GpuContext,
    width: u32,
    height: u32,
    f: impl FnOnce(&TextureSet) -> R,
) -> R {
    let mut cache = TEXTURE_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let set = cache.get_or_make(ctx, width, height);
    f(set)
}
