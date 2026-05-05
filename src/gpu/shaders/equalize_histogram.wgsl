// Equalize — pass 1: build a 256-bin histogram of OKLab L.
//
// Each pixel atomically increments the bin matching its L value.  L is
// clamped to [0, 1] before binning (out-of-gamut math from the upstream
// stages can push L slightly outside that range).
//
// The buffer must be zeroed by the caller (queue.write_buffer with zeros)
// before each dispatch — atomicAdd accumulates over whatever was there.

@group(0) @binding(0) var Input: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> hist: array<atomic<u32>, 256>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(Input));
  let pixel = vec2<i32>(gid.xy);
  if (pixel.x >= dims.x || pixel.y >= dims.y) { return; }

  let lab = textureLoad(Input, pixel, 0);
  let L = clamp(lab.r, 0.0, 1.0);
  let bin = u32(min(L * 255.0, 255.0));
  atomicAdd(&hist[bin], 1u);
}
