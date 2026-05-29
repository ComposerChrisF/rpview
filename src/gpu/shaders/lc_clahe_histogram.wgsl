// Local Contrast — CLAHE pass 1: per-tile histogram of OKLab L.
//
// The image is divided into an `nx × ny` grid of contextual tiles (tile size
// derived from the LC Radius).  Each pixel atomically increments the L-bin of
// the tile it falls in.  Output is a flat `nx*ny*256` u32 buffer, tile-major:
// bin `b` of tile `t` lives at `t*256 + b`.  The caller zeroes the used region
// before dispatch (atomicAdd accumulates otherwise).
//
// This is the local (per-tile) analogue of `equalize_histogram.wgsl`, which
// builds a single global histogram.

struct Uniforms {
  nx:         u32,
  ny:         u32,
  image_w:    u32,
  image_h:    u32,
  clip_limit: f32,  // unused here; shared uniform across the 3 CLAHE passes
  shadows:    f32,  // unused here
  highlights: f32,  // unused here
  _pad:       f32,
}

@group(0) @binding(0) var Input: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> hist: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> u: Uniforms;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(Input));
  let p = vec2<i32>(gid.xy);
  if (p.x >= dims.x || p.y >= dims.y) { return; }

  let L = clamp(textureLoad(Input, p, 0).r, 0.0, 1.0);
  let bin = u32(min(L * 255.0, 255.0));

  let tx = min(u32(f32(p.x) / f32(u.image_w) * f32(u.nx)), u.nx - 1u);
  let ty = min(u32(f32(p.y) / f32(u.image_h) * f32(u.ny)), u.ny - 1u);
  let tile = ty * u.nx + tx;

  atomicAdd(&hist[tile * 256u + bin], 1u);
}
