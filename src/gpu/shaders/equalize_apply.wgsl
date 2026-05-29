// Equalize — pass 2: apply the CPU-computed CDF lookup to OKLab L.
//
// `cdf` is a 256-element array of `f32` in [0, 1] produced on the CPU from
// the histogram pass: `cdf[i] = cumulative_count[i] / total_pixels`, then
// hard-anchored so cdf[0] = 0 and cdf[255] = 1 (pure black/white are fixed
// points — see `read_cdf`).  We linearly interpolate between adjacent CDF
// entries so the result doesn't show 256 discrete plateaus, then blend with
// the original L.  L only — chroma (a, b) and alpha pass through untouched,
// so equalization is a luminance redistribution and doesn't shift hue.
//
// The blend weight is per-pixel: a flat `Amount` everywhere, plus extra
// equalization ramped by the *source* luminance — `Shadows` weighted toward
// the dark end and `Highlights` toward the bright end.  This mirrors the CPU
// local-contrast path, where "Alpha of pure Black" / "Alpha of pure White"
// fade the fully-equalized frame in by source pixel luminance.

struct Uniforms {
  Amount:     f32,  // flat equalization across all tones
  Shadows:    f32,  // extra equalization weighted toward dark pixels
  Highlights: f32,  // extra equalization weighted toward bright pixels
  _pad:       f32,
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

  // Source-luminance-weighted blend: full Shadows weight at L = 0 fading
  // linearly to 0 at L = 1, and vice-versa for Highlights.  Clamped so the
  // sliders can't push the blend past full equalization.
  let amount = clamp(u.Amount + u.Shadows * (1.0 - L) + u.Highlights * L, 0.0, 1.0);

  let new_L = mix(lab.r, eq_L, amount);
  textureStore(Output, pixel, vec4<f32>(new_L, lab.g, lab.b, lab.a));
}
