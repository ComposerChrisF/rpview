// Equalize — pass 2: apply the CPU-computed CDF lookup to OKLab L.
//
// `cdf` is a 256-element array of `f32` in [0, 1] produced on the CPU from
// the histogram pass: `cdf[i] = cumulative_count[i] / total_pixels`.
// We do a linear interpolation between adjacent CDF entries so the result
// doesn't show 256 discrete plateaus, then blend with the original L by
// `Amount`.  L only — chroma (a, b) and alpha pass through untouched, so
// equalization is a luminance redistribution and doesn't shift hue.

struct Uniforms {
  Amount: f32,
  _pad0:  f32,
  _pad1:  f32,
  _pad2:  f32,
}

@group(0) @binding(0) var Input: texture_2d<f32>;
@group(0) @binding(1) var Output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<storage, read> cdf: array<f32, 256>;
@group(0) @binding(3) var<uniform> u: Uniforms;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<i32>(textureDimensions(Output));
  let pixel = vec2<i32>(gid.xy);
  if (pixel.x >= dims.x || pixel.y >= dims.y) { return; }

  let lab = textureLoad(Input, pixel, 0);
  let L = clamp(lab.r, 0.0, 1.0);
  let f = L * 255.0;
  let i0 = u32(min(f, 254.0));
  let i1 = i0 + 1u;
  let t = f - f32(i0);
  let eq_L = mix(cdf[i0], cdf[i1], t);
  let new_L = mix(lab.r, eq_L, u.Amount);
  textureStore(Output, pixel, vec4<f32>(new_L, lab.g, lab.b, lab.a));
}
